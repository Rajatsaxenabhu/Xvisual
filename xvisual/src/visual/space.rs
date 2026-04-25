use crossterm::{
    cursor,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    QueueableCommand,
};
use std::{io::Write, sync::{Arc, atomic::{AtomicBool, Ordering}}};
use ringbuf::consumer::Consumer;
use super::{AudioConsumer, TAU};

struct Star {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    age: u32,
    max_age: u32,
    hue: f32,
    speed: f32,
}

pub fn run(mut cons: AudioConsumer, running: Arc<AtomicBool>) {
    let mut stdout = std::io::stdout();
    let mut read_buf = vec![0.0f32; 512];
    let mut smoothed_rms = 0.0f32;
    let mut avg_rms = 0.01f32;
    let mut frame: u64 = 0;
    let mut stars: Vec<Star> = Vec::with_capacity(250);
    let mut waveform = vec![0.0f32; 256];
    let mut hue_offset = 0.0f32;
    let mut beat_flash = 0.0f32;
    let mut rng: u64 = 0xdeadbeef_cafebabe;

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
        frame += 1;
        rng = rng.wrapping_add(frame.wrapping_mul(6364136223846793005));

        let (cols, rows) = terminal::size().unwrap_or((80, 24));
        let cx = cols as f32 / 2.0;
        let cy = rows as f32 / 2.0;

        if total > 0 {
            let tail = &read_buf[total.saturating_sub(512)..total];
            let rms = (tail.iter().map(|s| s * s).sum::<f32>() / tail.len() as f32).sqrt();
            smoothed_rms = if rms > smoothed_rms { rms } else { rms * 0.20 + smoothed_rms * 0.80 };
            avg_rms = avg_rms * 0.995 + smoothed_rms * 0.005;

            if smoothed_rms > avg_rms * 2.2 && smoothed_rms > 0.015 {
                beat_flash = 1.0;
                let burst = (12 + (smoothed_rms * 25.0) as usize).min(30);
                for _ in 0..burst {
                    spawn_star(&mut stars, &mut rng, cx, cy, smoothed_rms * 1.8 + 0.5, cols, rows);
                }
            }
            beat_flash = (beat_flash - 0.07).max(0.0);

            let wstep = (tail.len() / waveform.len()).max(1);
            for i in 0..waveform.len() {
                waveform[i] = tail[(i * wstep).min(tail.len() - 1)];
            }
        } else {
            smoothed_rms *= 0.96;
            beat_flash = (beat_flash - 0.07).max(0.0);
        }

        if stars.len() < 200 {
            let ambient = (1 + (smoothed_rms * 6.0) as usize).min(5);
            for _ in 0..ambient {
                spawn_star(&mut stars, &mut rng, cx, cy, smoothed_rms * 0.8 + 0.2, cols, rows);
            }
        }

        hue_offset = (hue_offset + 0.003) % 1.0;

        let mut i = 0;
        while i < stars.len() {
            stars[i].x += stars[i].vx;
            stars[i].y += stars[i].vy;
            stars[i].age += 1;
            let gone = stars[i].x < 0.0 || stars[i].x >= cols as f32
                    || stars[i].y < 0.0 || stars[i].y >= (rows - 1) as f32
                    || stars[i].age >= stars[i].max_age;
            if gone { stars.swap_remove(i); } else { i += 1; }
        }

        stdout.queue(terminal::Clear(terminal::ClearType::All)).unwrap();

        draw_nebula(&mut stdout, cx, cy, smoothed_rms, hue_offset, cols, rows);

        for star in &stars {
            let life = 1.0 - star.age as f32 / star.max_age as f32;
            let hue = (star.hue + hue_offset) % 1.0;
            let (r, g, b) = hsl_to_rgb(hue, 0.85, 0.30 + life * 0.50);
            let ch = if star.speed > 1.5 { "✦" } else if star.speed > 0.9 { "*" } else { "·" };
            if star.x >= 0.0 && star.x < cols as f32 && star.y >= 0.0 && star.y < (rows - 1) as f32 {
                let _ = stdout
                    .queue(cursor::MoveTo(star.x as u16, star.y as u16))
                    .and_then(|s| s.queue(SetForegroundColor(Color::Rgb { r, g, b })))
                    .and_then(|s| s.queue(Print(ch)));
            }
        }

        draw_waveform(&mut stdout, &waveform, cx, cy, smoothed_rms, hue_offset, cols, rows);

        if beat_flash > 0.1 {
            let (r, g, b) = hsl_to_rgb(hue_offset, 0.9, 0.55 + beat_flash * 0.2);
            let flash_x = (cx as u16).saturating_sub(5);
            let _ = stdout
                .queue(cursor::MoveTo(flash_x, cy as u16))
                .and_then(|s| s.queue(SetForegroundColor(Color::Rgb { r, g, b })))
                .and_then(|s| s.queue(Print("✦ · ✦ · ✦ · ✦")));
        }

        let title = "  x-visual · space  ";
        let tx = (cx as u16).saturating_sub(title.len() as u16 / 2);
        stdout.queue(cursor::MoveTo(tx, 0)).unwrap()
              .queue(SetForegroundColor(Color::Rgb { r: 55, g: 55, b: 80 })).unwrap()
              .queue(Print(title)).unwrap();

        stdout.queue(cursor::MoveTo(cols.saturating_sub(8), 0)).unwrap()
              .queue(SetForegroundColor(Color::Rgb { r: 45, g: 45, b: 65 })).unwrap()
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

fn spawn_star(stars: &mut Vec<Star>, rng: &mut u64, cx: f32, cy: f32, energy: f32, _cols: u16, _rows: u16) {
    let angle = rand_f32(rng) * TAU;
    let speed = 0.25 + energy * 0.65 + rand_f32(rng) * 0.35;
    let max_age = 35 + (rand_f32(rng) * 85.0) as u32;
    stars.push(Star {
        x: cx,
        y: cy,
        vx: angle.cos() * speed * 2.1,
        vy: angle.sin() * speed,
        age: 0,
        max_age,
        hue: angle / TAU,
        speed,
    });
}

fn draw_nebula(
    stdout: &mut std::io::Stdout,
    _cx: f32, cy: f32, rms: f32, hue: f32,
    cols: u16, rows: u16,
) {
    let cx = cols as f32 / 2.0;
    let base_r = 1.5 + rms * 7.0;
    for ring in 0..6u32 {
        let radius = base_r + ring as f32 * 2.2;
        let steps = (radius * 5.5).max(24.0) as u32;
        let life = 1.0 - ring as f32 / 6.0;
        let ring_hue = (hue + ring as f32 * 0.07) % 1.0;
        let (r, g, b) = hsl_to_rgb(ring_hue, 0.80, life * 0.30 + rms * 0.18);
        for i in 0..steps {
            if ring > 1 && i % 2 == 0 { continue; }
            let angle = (i as f32 / steps as f32) * TAU;
            let x = cx + radius * angle.cos() * 2.1;
            let y = cy + radius * angle.sin();
            if x >= 0.0 && x < cols as f32 && y >= 0.0 && y < (rows - 1) as f32 {
                let _ = stdout
                    .queue(cursor::MoveTo(x as u16, y as u16))
                    .and_then(|s| s.queue(SetForegroundColor(Color::Rgb { r, g, b })))
                    .and_then(|s| s.queue(Print("·")));
            }
        }
    }
}

fn draw_waveform(
    stdout: &mut std::io::Stdout,
    waveform: &[f32],
    _cx: f32, cy: f32, rms: f32, hue: f32,
    cols: u16, rows: u16,
) {
    if cols < 4 || waveform.is_empty() { return; }
    let width = cols as usize - 4;
    let amp = (3.0 + rms * rows as f32 * 0.28).min(rows as f32 * 0.42);

    for xi in 0..width {
        let widx = (xi * waveform.len() / width).min(waveform.len() - 1);
        let sample = waveform[widx];
        let y = (cy + sample * amp).clamp(1.0, (rows - 2) as f32);
        let t = xi as f32 / width as f32;
        let wave_hue = (hue + t * 0.28) % 1.0;
        let (r, g, b) = hsl_to_rgb(wave_hue, 1.0, 0.50 + rms * 0.30);
        let ch = if sample.abs() > 0.04 { "▪" } else { "·" };
        let _ = stdout
            .queue(cursor::MoveTo(2 + xi as u16, y as u16))
            .and_then(|s| s.queue(SetForegroundColor(Color::Rgb { r, g, b })))
            .and_then(|s| s.queue(Print(ch)));
    }
}

fn rand_f32(state: &mut u64) -> f32 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    (*state >> 11) as f32 / (1u64 << 53) as f32
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
