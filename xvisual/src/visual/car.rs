// Car Dashboard — animated tachometer arc with needle, RPM readout, gear
use crossterm::{
    cursor,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    QueueableCommand,
};
use std::{io::Write, sync::{Arc, atomic::{AtomicBool, Ordering}}};
use ringbuf::consumer::Consumer;
use super::AudioConsumer;

const PI: f32 = std::f32::consts::PI;

// Arc spans 240° clockwise: 150° → 390° (= 30°) in terminal coords
// At 150°: lower-left  (7 o'clock)
// At 270°: top-center  (12 o'clock)
// At 390°: lower-right (5 o'clock)
const ARC_START: f32 = 150.0;
const ARC_SPAN:  f32 = 240.0;

pub fn run(mut cons: AudioConsumer, running: Arc<AtomicBool>) {
    let mut stdout = std::io::stdout();
    let mut read_buf = vec![0.0f32; 512];
    let mut smoothed_rms = 0.0f32;
    let mut peak_rms = 0.0f32;
    let mut avg_rms = 0.01f32;
    let mut boost_timer = 0u32;

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

        if total > 0 {
            let tail = &read_buf[total.saturating_sub(512)..total];
            let rms = (tail.iter().map(|s| s * s).sum::<f32>() / tail.len() as f32).sqrt();
            smoothed_rms = if rms > smoothed_rms { rms } else { rms * 0.15 + smoothed_rms * 0.85 };
            if smoothed_rms > peak_rms { peak_rms = smoothed_rms; }
            peak_rms *= 0.998;
            avg_rms = avg_rms * 0.996 + smoothed_rms * 0.004;
            if smoothed_rms > avg_rms * 2.3 && smoothed_rms > 0.02 { boost_timer = 22; }
        } else {
            smoothed_rms *= 0.92;
            peak_rms *= 0.998;
        }
        if boost_timer > 0 { boost_timer -= 1; }

        let rpm_frac = (smoothed_rms * 6.5).min(1.0);
        let rpm_val  = (rpm_frac * 9000.0) as u32;
        let gear = match rpm_val {
            0..=1400    => 1,
            1401..=2900 => 2,
            2901..=4300 => 3,
            4301..=5800 => 4,
            5801..=7500 => 5,
            _           => 6,
        };

        let (cols, rows) = terminal::size().unwrap_or((80, 24));
        let cx = cols as f32 / 2.0;
        // shift gauge center downward so arc curves up into the screen
        let cy_gauge = rows as f32 / 2.0 + rows as f32 * 0.12;
        let radius = (cols as f32 / 2.0 / 2.1).min(rows as f32 / 2.0) * 0.80;

        stdout.queue(terminal::Clear(terminal::ClearType::All)).unwrap();

        // ── tachometer arc ────────────────────────────────────────────
        let arc_steps = 300u32;
        for i in 0..arc_steps {
            let t = i as f32 / arc_steps as f32;
            let deg = ARC_START + t * ARC_SPAN;
            let a = deg * PI / 180.0;
            let x = cx + radius * a.cos() * 2.1;
            let y = cy_gauge + radius * a.sin();
            if x < 0.0 || x >= cols as f32 || y < 0.0 || y >= (rows - 1) as f32 { continue; }

            // redline zone (last 15% of arc = ~8000+ RPM)
            let color = if t > 0.85 {
                Color::Rgb { r: 140, g: 30, b: 30 }
            } else if t > 0.65 {
                Color::Rgb { r: 110, g: 70, b: 25 }
            } else {
                Color::Rgb { r: 50, g: 85, b: 50 }
            };
            let ch = if (i % 10 == 0) || (i % 10 == 1) { "▪" } else { "·" };
            let _ = stdout.queue(cursor::MoveTo(x as u16, y as u16))
                .and_then(|s| s.queue(SetForegroundColor(color)))
                .and_then(|s| s.queue(Print(ch)));
        }

        // ── RPM tick marks & labels ────────────────────────────────────
        for step in 0..=9u32 {
            let t = step as f32 / 9.0;
            let deg = ARC_START + t * ARC_SPAN;
            let a = deg * PI / 180.0;
            // inner tick
            for r_off in 0..3u32 {
                let r = radius - r_off as f32;
                let x = cx + r * a.cos() * 2.1;
                let y = cy_gauge + r * a.sin();
                if x >= 0.0 && x < cols as f32 && y >= 0.0 && y < (rows - 1) as f32 {
                    let _ = stdout.queue(cursor::MoveTo(x as u16, y as u16))
                        .and_then(|s| s.queue(SetForegroundColor(Color::Rgb { r: 200, g: 200, b: 200 })))
                        .and_then(|s| s.queue(Print("█")));
                }
            }
            // label slightly outside arc
            let lr = radius + 2.5;
            let lx = cx + lr * a.cos() * 2.1;
            let ly = cy_gauge + lr * a.sin();
            if lx >= 0.0 && lx < cols as f32 - 1.0 && ly >= 0.0 && ly < (rows - 1) as f32 {
                let label_color = if step >= 8 { Color::Rgb { r: 255, g: 50, b: 50 } }
                    else { Color::Rgb { r: 160, g: 160, b: 180 } };
                let _ = stdout.queue(cursor::MoveTo(lx as u16, ly as u16))
                    .and_then(|s| s.queue(SetForegroundColor(label_color)))
                    .and_then(|s| s.queue(Print(step)));
            }
        }

        // ── needle ────────────────────────────────────────────────────
        let needle_deg = ARC_START + rpm_frac * ARC_SPAN;
        let needle_a = needle_deg * PI / 180.0;
        let needle_color = if rpm_frac > 0.85 {
            Color::Rgb { r: 200, g: 50, b: 50 }
        } else {
            Color::Rgb { r: 160, g: 165, b: 180 }
        };
        for r_step in 0..(radius as u32) {
            let r = r_step as f32;
            let x = cx + r * needle_a.cos() * 2.1;
            let y = cy_gauge + r * needle_a.sin();
            if x >= 0.0 && x < cols as f32 && y >= 0.0 && y < (rows - 1) as f32 {
                let ch = if r_step < 2 { "●" } else { "▪" };
                let _ = stdout.queue(cursor::MoveTo(x as u16, y as u16))
                    .and_then(|s| s.queue(SetForegroundColor(needle_color)))
                    .and_then(|s| s.queue(Print(ch)));
            }
        }

        // ── hub dot ───────────────────────────────────────────────────
        stdout.queue(cursor::MoveTo(cx as u16, cy_gauge as u16)).unwrap()
              .queue(SetForegroundColor(Color::Rgb { r: 255, g: 255, b: 255 })).unwrap()
              .queue(Print("◉")).unwrap();

        // ── center RPM readout ────────────────────────────────────────
        let rpm_str  = format!("{:5}", rpm_val);
        let unit_str = "RPM";
        let center_row = (cy_gauge - radius * 0.38) as u16;
        let rx = (cx as u16).saturating_sub(rpm_str.len() as u16 / 2);
        stdout.queue(cursor::MoveTo(rx, center_row)).unwrap()
              .queue(SetForegroundColor(needle_color)).unwrap()
              .queue(Print(&rpm_str)).unwrap();
        stdout.queue(cursor::MoveTo((cx as u16).saturating_sub(unit_str.len() as u16 / 2), center_row + 1)).unwrap()
              .queue(SetForegroundColor(Color::Rgb { r: 100, g: 100, b: 120 })).unwrap()
              .queue(Print(unit_str)).unwrap();

        // ── gear indicator ────────────────────────────────────────────
        let gear_row = center_row + 3;
        let gear_str = format!("GEAR  {}", gear);
        stdout.queue(cursor::MoveTo((cx as u16).saturating_sub(gear_str.len() as u16 / 2), gear_row)).unwrap()
              .queue(SetForegroundColor(Color::Rgb { r: 80, g: 80, b: 100 })).unwrap()
              .queue(Print(&gear_str)).unwrap();

        // ── BOOST flash ───────────────────────────────────────────────
        if boost_timer > 0 {
            let b_bright = boost_timer as f32 / 22.0;
            let bv = (255.0 * b_bright) as u8;
            let boost_str = "◀◀  BOOST  ▶▶";
            let bx = (cx as u16).saturating_sub(boost_str.len() as u16 / 2);
            stdout.queue(cursor::MoveTo(bx, gear_row + 2)).unwrap()
                  .queue(SetForegroundColor(Color::Rgb { r: bv, g: (bv as f32 * 0.6) as u8, b: 0 })).unwrap()
                  .queue(Print(boost_str)).unwrap();
        }

        let title = "  car dashboard  ";
        let tx = (cx as u16).saturating_sub(title.len() as u16 / 2);
        stdout.queue(cursor::MoveTo(tx, 0)).unwrap()
              .queue(SetForegroundColor(Color::Rgb { r: 60, g: 60, b: 75 })).unwrap()
              .queue(Print(title)).unwrap();
        stdout.queue(cursor::MoveTo(cols.saturating_sub(8), 0)).unwrap()
              .queue(SetForegroundColor(Color::Rgb { r: 45, g: 45, b: 60 })).unwrap()
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
