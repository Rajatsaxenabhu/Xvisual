use crossterm::{
    cursor,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    QueueableCommand,
};
use std::{
    io::Write,
    sync::{Arc, atomic::{AtomicBool, Ordering}},
};
use ringbuf::consumer::Consumer;
use super::{AudioConsumer, TAU};

struct Ripple {
    radius: f32,
    max_radius: f32,
    life: f32,
}

pub fn run(mut cons: AudioConsumer, running: Arc<AtomicBool>) {
    let mut stdout = std::io::stdout();
    let mut read_buf = vec![0.0f32; 512];
    let mut smoothed_rms = 0.0f32;
    let mut peak_rms = 0.0f32;
    let mut peak_hold = 0u32;
    let mut frame: u64 = 0;
    let mut ripples: Vec<Ripple> = Vec::new();
    let mut avg_rms = 0.01f32;
    let mut beat_flash = 0.0f32;
    let mut band_smoothed = [0.0f32; 32];

    terminal::enable_raw_mode().unwrap();
    stdout.queue(EnterAlternateScreen).unwrap();
    stdout.queue(cursor::Hide).unwrap();
    stdout.flush().unwrap();

    while running.load(Ordering::SeqCst) {
        let frame_start = std::time::Instant::now();
        let mut total = 0usize;
        loop {
            let n = cons.pop_slice(&mut read_buf[total..]);
            if n == 0 { break; }
            total += n;
            if total >= read_buf.len() { break; }
        }
        let count = total;
        frame += 1;

        if count > 0 {
            let tail = &read_buf[count.saturating_sub(512)..count];
            let rms = (tail.iter().map(|s| s * s).sum::<f32>() / tail.len() as f32).sqrt();

            smoothed_rms = if rms > smoothed_rms { rms } else { rms * 0.20 + smoothed_rms * 0.80 };

            avg_rms = avg_rms * 0.995 + smoothed_rms * 0.005;
            if smoothed_rms > avg_rms * 2.2 && smoothed_rms > 0.015 {
                beat_flash = 1.0;
                let (_, rows) = terminal::size().unwrap_or((80, 24));
                if ripples.len() < 6 {
                    ripples.push(Ripple {
                        radius: 3.0,
                        max_radius: rows as f32 / 2.0 * 1.15,
                        life: 1.0,
                    });
                }
            }
            beat_flash = (beat_flash - 0.1).max(0.0);

            let band_size = (tail.len() / 32).max(1);
            for b in 0..32 {
                let start = b * band_size;
                let end = ((b + 1) * band_size).min(tail.len());
                if start < tail.len() {
                    let e = tail[start..end].iter().map(|s| s * s).sum::<f32>()
                        / (end - start) as f32;
                    let level = e.sqrt();
                    band_smoothed[b] = if level > band_smoothed[b] {
                        level
                    } else {
                        level * 0.1 + band_smoothed[b] * 0.9
                    };
                }
            }

            if smoothed_rms > peak_rms {
                peak_rms = smoothed_rms;
                peak_hold = 20;
            } else if peak_hold > 0 {
                peak_hold -= 1;
            } else {
                peak_rms *= 0.97;
            }
        } else {
            for b in &mut band_smoothed { *b *= 0.95; }
            beat_flash = (beat_flash - 0.1).max(0.0);
        }

        let mut i = 0;
        while i < ripples.len() {
            ripples[i].radius += 1.2;
            ripples[i].life = (ripples[i].life - 0.02).max(0.0);
            if ripples[i].life <= 0.0 || ripples[i].radius >= ripples[i].max_radius {
                ripples.swap_remove(i);
            } else {
                i += 1;
            }
        }

        let (cols, rows) = terminal::size().unwrap_or((80, 24));
        let cx = cols as f32 / 2.0;
        let cy = rows as f32 / 2.0;

        let max_r_from_cols = (cols as f32 / 2.0) / 2.1;
        let max_r_from_rows = rows as f32 / 2.0;
        let max_radius = max_r_from_cols.min(max_r_from_rows) * 1.22;
        let min_radius = max_radius * 0.01;
        let target_radius = min_radius + (max_radius - min_radius) * (smoothed_rms * 4.0).min(1.0);

        stdout.queue(terminal::Clear(terminal::ClearType::All)).unwrap();

        if beat_flash > 0.05 {
            draw_beat_flash(&mut stdout, cx, cy, max_radius * 0.55, beat_flash, cols, rows);
        }

        let spin1 = (frame as f32 * 0.003) % 1.0;
        let spin2 = 1.0 - (frame as f32 * 0.002) % 1.0;
        draw_halo(&mut stdout, cx, cy, max_radius * 0.40, Color::Rgb { r: 40, g: 40, b: 55 }, cols, rows, spin1);
        draw_halo(&mut stdout, cx, cy, max_radius * 0.75, Color::Rgb { r: 35, g: 35, b: 50 }, cols, rows, spin2);

        for ripple in &ripples {
            let v = (ripple.life * 60.0) as u8;
            draw_ring(&mut stdout, cx, cy, ripple.radius, "·",
                Color::Rgb { r: v, g: v, b: v }, cols, rows);
        }

        draw_spikes(&mut stdout, cx, cy, target_radius, &band_smoothed, cols, rows);

        draw_filled_circle(&mut stdout, cx, cy, target_radius * 0.80, smoothed_rms, cols, rows);

        draw_ring(&mut stdout, cx, cy, target_radius, "█", rms_color(smoothed_rms, 1.0), cols, rows);
        draw_ring(&mut stdout, cx, cy, target_radius * 0.70, "▓", rms_color(smoothed_rms, 0.75), cols, rows);
        draw_ring(&mut stdout, cx, cy, target_radius * 0.50, "▒", rms_color(smoothed_rms, 0.5), cols, rows);

        if peak_rms > min_radius / max_radius {
            let peak_r = (min_radius + (max_radius - min_radius)
                * (peak_rms * 4.0).min(1.0) * 1.08).min(max_radius);
            draw_ring(&mut stdout, cx, cy, peak_r, "○",
                Color::Rgb { r: 160, g: 160, b: 175 }, cols, rows);
        }

        stdout
            .queue(cursor::MoveTo(cx as u16, cy as u16)).unwrap()
            .queue(SetForegroundColor(rms_color(smoothed_rms, 1.0))).unwrap()
            .queue(Print(center_char(smoothed_rms))).unwrap();

        let bar_w = (cols as usize).saturating_sub(20);
        let filled = ((smoothed_rms * 4.0).min(1.0) * bar_w as f32) as usize;
        let bar: String = (0..bar_w)
            .map(|i| if i < filled { block_char(i, bar_w) } else { ' ' })
            .collect();
        stdout
            .queue(cursor::MoveTo(0, rows - 1)).unwrap()
            .queue(SetForegroundColor(Color::Rgb { r: 55, g: 55, b: 70 })).unwrap()
            .queue(Print("vol ")).unwrap()
            .queue(SetForegroundColor(rms_color(smoothed_rms, 0.9))).unwrap()
            .queue(Print(&bar)).unwrap()
            .queue(SetForegroundColor(Color::Rgb { r: 50, g: 50, b: 65 })).unwrap()
            .queue(Print(format!(" {:.3}", smoothed_rms))).unwrap();

        let title = "  x-visual  ";
        let title_x = (cx as u16).saturating_sub(title.len() as u16 / 2);
        stdout
            .queue(cursor::MoveTo(title_x, 0)).unwrap()
            .queue(SetForegroundColor(Color::Rgb { r: 60, g: 60, b: 75 })).unwrap()
            .queue(Print(title)).unwrap();

        stdout
            .queue(cursor::MoveTo(cols.saturating_sub(8), 0)).unwrap()
            .queue(SetForegroundColor(Color::Rgb { r: 45, g: 45, b: 55 })).unwrap()
            .queue(Print("ctrl+c")).unwrap();

        stdout.queue(ResetColor).unwrap();
        stdout.flush().unwrap();

        let elapsed = frame_start.elapsed();
        let budget = std::time::Duration::from_millis(16);
        if elapsed < budget {
            std::thread::sleep(budget - elapsed);
        }
    }

    stdout.queue(LeaveAlternateScreen).unwrap();
    stdout.queue(cursor::Show).unwrap();
    terminal::disable_raw_mode().unwrap();
    stdout.flush().unwrap();
}

// ── color ──────────────────────────────────────────────────────────

fn rms_color(rms: f32, scale: f32) -> Color {
    let v = (rms * 4.0).clamp(0.0, 1.0) * scale;
    Color::Rgb {
        r: (60.0 + v * 80.0) as u8,
        g: (65.0 + v * 85.0) as u8,
        b: (85.0 + v * 100.0) as u8,
    }
}

// ── drawing helpers ────────────────────────────────────────────────

fn draw_ring(
    stdout: &mut std::io::Stdout,
    cx: f32, cy: f32, radius: f32,
    ch: &str, color: Color,
    cols: u16, rows: u16,
) {
    if radius < 1.0 { return; }
    let steps = (radius * 7.0).max(48.0) as u32;
    for i in 0..steps {
        let angle = (i as f32 / steps as f32) * TAU;
        let x = cx + radius * angle.cos() * 2.1;
        let y = cy + radius * angle.sin();
        if x >= 0.0 && x < cols as f32 && y >= 0.0 && y < (rows - 1) as f32 {
            let _ = stdout
                .queue(cursor::MoveTo(x as u16, y as u16))
                .and_then(|s| s.queue(SetForegroundColor(color)))
                .and_then(|s| s.queue(Print(ch)));
        }
    }
}

fn draw_halo(
    stdout: &mut std::io::Stdout,
    cx: f32, cy: f32, radius: f32,
    color: Color,
    cols: u16, rows: u16,
    spin: f32,
) {
    if radius < 1.0 { return; }
    let steps = (radius * 7.0).max(48.0) as u32;
    for i in 0..steps {
        if i % 3 == 0 { continue; }
        let angle = (i as f32 / steps as f32 + spin) * TAU;
        let x = cx + radius * angle.cos() * 2.1;
        let y = cy + radius * angle.sin();
        if x >= 0.0 && x < cols as f32 && y >= 0.0 && y < (rows - 1) as f32 {
            let _ = stdout
                .queue(cursor::MoveTo(x as u16, y as u16))
                .and_then(|s| s.queue(SetForegroundColor(color)))
                .and_then(|s| s.queue(Print("·")));
        }
    }
}

fn draw_spikes(
    stdout: &mut std::io::Stdout,
    cx: f32, cy: f32, base_radius: f32,
    bands: &[f32; 32],
    cols: u16, rows: u16,
) {
    let num_spikes = 64usize;
    for i in 0..num_spikes {
        let t = i as f32 / num_spikes as f32;
        let angle = t * TAU;
        let band_idx = (i * 32 / num_spikes).min(31);
        let level = bands[band_idx];
        if level < 0.004 { continue; }
        let spike_len = level * 14.0;
        let steps = (spike_len * 2.0) as u32 + 2;
        for s in 0..=steps {
            let r = base_radius + (s as f32 / steps as f32) * spike_len;
            let x = cx + r * angle.cos() * 2.1;
            let y = cy + r * angle.sin();
            if x >= 0.0 && x < cols as f32 && y >= 0.0 && y < (rows - 1) as f32 {
                let tip_t = s as f32 / steps as f32;
                let brightness = ((1.0 - tip_t) * level * 4.0).clamp(0.0, 1.0);
                let v = (brightness * 120.0 + 30.0) as u8;
                let ch = if tip_t < 0.35 { "█" } else if tip_t < 0.65 { "▓" } else { "▒" };
                let _ = stdout
                    .queue(cursor::MoveTo(x as u16, y as u16))
                    .and_then(|s| s.queue(SetForegroundColor(Color::Rgb { r: (v as f32 * 0.6) as u8, g: (v as f32 * 0.65) as u8, b: v })))
                    .and_then(|s| s.queue(Print(ch)));
            }
        }
    }
}

fn draw_beat_flash(
    stdout: &mut std::io::Stdout,
    cx: f32, cy: f32, radius: f32,
    intensity: f32,
    cols: u16, rows: u16,
) {
    for row in (1..rows.saturating_sub(1)).step_by(2) {
        for col in (0..cols).step_by(3) {
            let dx = (col as f32 - cx) / 2.1;
            let dy = row as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < radius {
                let falloff = (1.0 - dist / radius) * intensity;
                let v = (falloff * 55.0) as u8;
                let _ = stdout
                    .queue(cursor::MoveTo(col, row))
                    .and_then(|s| s.queue(SetForegroundColor(Color::Rgb { r: v, g: v, b: v })))
                    .and_then(|s| s.queue(Print("·")));
            }
        }
    }
}

fn draw_filled_circle(
    stdout: &mut std::io::Stdout,
    cx: f32, cy: f32, radius: f32,
    rms: f32, cols: u16, rows: u16,
) {
    if radius < 2.0 { return; }
    let x_min = ((cx - radius * 2.1) as i32).max(0);
    let x_max = ((cx + radius * 2.1) as i32).min(cols as i32 - 1);
    let y_min = ((cy - radius) as i32).max(0);
    let y_max = ((cy + radius) as i32).min(rows as i32 - 2);

    for y in (y_min..=y_max).step_by(2) {
        for x in (x_min..=x_max).step_by(3) {
            let dx = (x as f32 - cx) / 2.1;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < radius * 0.78 {
                let t = dist / (radius * 0.78);
                let brightness = ((1.0 - t) * 0.4 + rms * 0.35).min(1.0);
                let v = (brightness * 70.0) as u8;
                let ch = fill_char(t, rms);
                let _ = stdout
                    .queue(cursor::MoveTo(x as u16, y as u16))
                    .and_then(|s| s.queue(SetForegroundColor(Color::Rgb { r: (v as f32 * 0.6) as u8, g: (v as f32 * 0.65) as u8, b: v })))
                    .and_then(|s| s.queue(Print(ch)));
            }
        }
    }
}

fn fill_char(t: f32, rms: f32) -> &'static str {
    let energy = ((1.0 - t) * rms * 6.0).clamp(0.0, 1.0);
    match (energy * 4.0) as u8 {
        3..=4 => "█",
        2     => "▓",
        1     => "▒",
        _     => "░",
    }
}

fn center_char(rms: f32) -> &'static str {
    match (rms * 20.0) as u8 {
        0     => "·",
        1     => "○",
        2     => "◎",
        3..=4 => "◉",
        5..=7 => "✦",
        _     => "✸",
    }
}

fn block_char(i: usize, total: usize) -> char {
    let t = i as f32 / total as f32;
    match (t * 4.0) as u8 {
        0 => '█',
        1 => '▓',
        2 => '▒',
        _ => '░',
    }
}
