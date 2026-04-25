use crossterm::{
    cursor,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    QueueableCommand,
};
use std::{io::Write, sync::{Arc, atomic::{AtomicBool, Ordering}}};
use ringbuf::consumer::Consumer;
use super::{AudioConsumer, TAU};

pub fn run(mut cons: AudioConsumer, running: Arc<AtomicBool>) {
    let mut stdout = std::io::stdout();
    let mut read_buf = vec![0.0f32; 512];
    let mut smoothed_rms = 0.0f32;
    let mut avg_rms = 0.01f32;
    let mut band_smoothed = [0.0f32; 64];
    let mut waveform = vec![0.0f32; 512];
    let mut time = 0.0f32;
    let mut beat_flash = 0.0f32;

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
        time += 0.04;

        if total > 0 {
            let tail = &read_buf[total.saturating_sub(512)..total];
            let rms = (tail.iter().map(|s| s * s).sum::<f32>() / tail.len() as f32).sqrt();
            smoothed_rms = if rms > smoothed_rms { rms } else { rms * 0.20 + smoothed_rms * 0.80 };
            avg_rms = avg_rms * 0.995 + smoothed_rms * 0.005;

            if smoothed_rms > avg_rms * 2.2 && smoothed_rms > 0.015 {
                beat_flash = 1.0;
            }
            beat_flash = (beat_flash - 0.06).max(0.0);

            let bsz = (tail.len() / 64).max(1);
            for b in 0..64 {
                let s = b * bsz;
                let e = ((b + 1) * bsz).min(tail.len());
                if s < tail.len() {
                    let level = (tail[s..e].iter().map(|x| x * x).sum::<f32>() / (e - s) as f32).sqrt();
                    band_smoothed[b] = if level > band_smoothed[b] { level }
                                       else { level * 0.12 + band_smoothed[b] * 0.88 };
                }
            }

            let wstep = (tail.len() / waveform.len()).max(1);
            for i in 0..waveform.len() {
                waveform[i] = tail[(i * wstep).min(tail.len() - 1)];
            }
        } else {
            for b in &mut band_smoothed { *b *= 0.95; }
            smoothed_rms *= 0.96;
            beat_flash = (beat_flash - 0.06).max(0.0);
        }

        let (cols, rows) = terminal::size().unwrap_or((80, 24));
        let cx = cols as f32 / 2.0;
        let cy = rows as f32 / 2.0;

        stdout.queue(terminal::Clear(terminal::ClearType::All)).unwrap();

        draw_plasma(&mut stdout, time, smoothed_rms, &band_smoothed, beat_flash, cols, rows);
        draw_waveforms(&mut stdout, &waveform, cx, cy, smoothed_rms, time, cols, rows);

        let title = "  x-visual · plasma  ";
        let tx = (cx as u16).saturating_sub(title.len() as u16 / 2);
        stdout.queue(cursor::MoveTo(tx, 0)).unwrap()
              .queue(SetForegroundColor(Color::Rgb { r: 60, g: 50, b: 85 })).unwrap()
              .queue(Print(title)).unwrap();

        stdout.queue(cursor::MoveTo(cols.saturating_sub(8), 0)).unwrap()
              .queue(SetForegroundColor(Color::Rgb { r: 50, g: 45, b: 70 })).unwrap()
              .queue(Print("ctrl+c")).unwrap();

        stdout.queue(ResetColor).unwrap();
        stdout.flush().unwrap();

        let elapsed = frame_start.elapsed();
        let budget = std::time::Duration::from_millis(16);
        if elapsed < budget { std::thread::sleep(budget - elapsed); }
    }

    stdout.queue(LeaveAlternateScreen).unwrap();
    stdout.queue(cursor::Show).unwrap();
    terminal::disable_raw_mode().unwrap();
    stdout.flush().unwrap();
}

fn draw_plasma(
    stdout: &mut std::io::Stdout,
    time: f32, rms: f32,
    bands: &[f32; 64],
    beat: f32,
    cols: u16, rows: u16,
) {
    for row in (1..rows.saturating_sub(1)).step_by(2) {
        for col in (0..cols).step_by(3) {
            let nx = col as f32 / cols as f32;
            let ny = row as f32 / rows as f32;
            let band_idx = (col as usize * 64 / cols as usize).min(63);
            let band_e = bands[band_idx];

            let v = (nx * TAU * 3.0 + time).sin() * 0.28
                  + (ny * TAU * 2.0 + time * 1.3).sin() * 0.28
                  + ((nx + ny) * TAU * 2.5 + time * 0.85).sin() * 0.20
                  + band_e * 0.80;

            let hue = ((v * 0.5 + 0.5) + time * 0.10 + beat * 0.20).fract().abs();
            let lightness = (0.12 + v * 0.08 + rms * 0.22 + beat * 0.08).clamp(0.0, 0.40);
            let (r, g, b) = hsl_to_rgb(hue, 0.90, lightness);

            let ch = if band_e > 0.15 { "▓" } else if band_e > 0.06 { "▒" } else { "░" };
            let _ = stdout
                .queue(cursor::MoveTo(col, row))
                .and_then(|s| s.queue(SetForegroundColor(Color::Rgb { r, g, b })))
                .and_then(|s| s.queue(Print(ch)));
        }
    }
}

fn draw_waveforms(
    stdout: &mut std::io::Stdout,
    waveform: &[f32],
    _cx: f32, cy: f32,
    rms: f32, time: f32,
    cols: u16, rows: u16,
) {
    if cols == 0 || waveform.is_empty() { return; }
    let width = cols as usize;
    let max_amp = (rows as f32 * 0.38).min(18.0 + rms * rows as f32 * 0.28);

    for layer in 0..3u32 {
        let phase = layer as f32 * 0.65;
        let amp = max_amp * (1.0 - layer as f32 * 0.22);
        let y_base = cy + (layer as f32 - 1.0) * (rows as f32 * 0.18);
        let hue_base = (time * 0.14 + layer as f32 * 0.33) % 1.0;

        for xi in 0..width {
            let widx = (xi * waveform.len() / width).min(waveform.len() - 1);
            let harmonic_wave = if layer > 0 {
                (xi as f32 / width as f32 * TAU * (layer as f32 + 1.0) + time * 2.0 + phase).sin()
                    * rms * 0.35
            } else {
                0.0
            };
            let sample = waveform[widx] + harmonic_wave;
            let y = (y_base + sample * amp).clamp(1.0, (rows - 2) as f32) as u16;

            let t = xi as f32 / width as f32;
            let hue = (hue_base + t * 0.25) % 1.0;
            let bright = (0.50 + rms * 0.30 - layer as f32 * 0.10).clamp(0.25, 0.72);
            let (r, g, b) = hsl_to_rgb(hue, 1.0, bright);

            let ch = match layer {
                0 => if sample.abs() > 0.03 { "█" } else { "▪" },
                1 => if sample.abs() > 0.03 { "▓" } else { "·" },
                _ => if sample.abs() > 0.05 { "▒" } else { "·" },
            };
            let _ = stdout
                .queue(cursor::MoveTo(xi as u16, y))
                .and_then(|s| s.queue(SetForegroundColor(Color::Rgb { r, g, b })))
                .and_then(|s| s.queue(Print(ch)));
        }
    }
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let h6 = h * 6.0;
    let x = c * (1.0 - (h6 % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r1, g1, b1) = match h6 as u32 {
        0 => (c, x, 0.0_f32),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    (
        ((r1 + m) * 255.0).clamp(0.0, 255.0) as u8,
        ((g1 + m) * 255.0).clamp(0.0, 255.0) as u8,
        ((b1 + m) * 255.0).clamp(0.0, 255.0) as u8,
    )
}
