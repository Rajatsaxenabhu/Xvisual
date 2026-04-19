// Symmetric rainbow EQ bars — up and down from a center line
use crossterm::{
    cursor,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    QueueableCommand,
};
use std::{io::Write, sync::{Arc, atomic::{AtomicBool, Ordering}}};
use ringbuf::consumer::Consumer;
use super::AudioConsumer;

pub fn run(mut cons: AudioConsumer, running: Arc<AtomicBool>) {
    let mut stdout = std::io::stdout();
    let mut read_buf = vec![0.0f32; 512];
    let mut band_smoothed = [0.0f32; 64];
    let mut peak_level  = [0.0f32; 64];
    let mut peak_timer  = [0u32;   64];
    let mut _frame: u64 = 0;

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
        _frame += 1;

        if total > 0 {
            let tail = &read_buf[total.saturating_sub(512)..total];
            let bsz = (tail.len() / 64).max(1);
            for b in 0..64 {
                let s = b * bsz;
                let e = ((b + 1) * bsz).min(tail.len());
                if s < tail.len() {
                    let energy = tail[s..e].iter().map(|x| x * x).sum::<f32>() / (e - s) as f32;
                    let level = (energy.sqrt() * 5.5).min(1.0);
                    band_smoothed[b] = if level > band_smoothed[b] { level }
                        else { level * 0.12 + band_smoothed[b] * 0.88 };
                    if band_smoothed[b] > peak_level[b] {
                        peak_level[b] = band_smoothed[b];
                        peak_timer[b] = 35;
                    } else if peak_timer[b] > 0 {
                        peak_timer[b] -= 1;
                    } else {
                        peak_level[b] = (peak_level[b] - 0.015).max(0.0);
                    }
                }
            }
        } else {
            for b in 0..64 {
                band_smoothed[b] *= 0.90;
                if peak_timer[b] > 0 { peak_timer[b] -= 1; }
                else { peak_level[b] = (peak_level[b] - 0.015).max(0.0); }
            }
        }

        let (cols, rows) = terminal::size().unwrap_or((80, 24));
        let bar_count = ((cols as usize).saturating_sub(4) / 2).clamp(8, 64);
        let margin    = (cols as usize).saturating_sub(bar_count * 2) / 2;
        let cy        = rows / 2;
        let max_h     = cy.saturating_sub(2) as usize;

        stdout.queue(terminal::Clear(terminal::ClearType::All)).unwrap();

        // center line
        stdout.queue(cursor::MoveTo(0, cy)).unwrap()
              .queue(SetForegroundColor(Color::Rgb { r: 35, g: 35, b: 55 })).unwrap()
              .queue(Print("─".repeat(cols as usize))).unwrap();

        for i in 0..bar_count {
            let band_idx = (i * 64 / bar_count).min(63);
            let level    = band_smoothed[band_idx];
            let ph       = peak_level[band_idx];
            let bar_h    = (level * max_h as f32) as usize;
            let peak_h   = (ph * max_h as f32) as usize;
            let x        = (margin + i * 2) as u16;

            // upward bars
            for h in 0..bar_h {
                let row_i = cy as isize - 1 - h as isize;
                if row_i < 1 { break; }
                let v = (55.0 + (h as f32 / max_h as f32) * 110.0) as u8;
                let ch = if h == bar_h - 1 { "▀" } else { "█" };
                let _ = stdout.queue(cursor::MoveTo(x, row_i as u16))
                    .and_then(|s| s.queue(SetForegroundColor(Color::Rgb { r: (v as f32 * 0.6) as u8, g: (v as f32 * 0.65) as u8, b: v })))
                    .and_then(|s| s.queue(Print(ch)));
            }

            // mirror bars downward (dimmer)
            for h in 0..bar_h {
                let row = cy + 1 + h as u16;
                if row >= rows - 1 { break; }
                let v = (20.0 + (h as f32 / max_h as f32) * 35.0) as u8;
                let ch = if h == bar_h - 1 { "▄" } else { "█" };
                let _ = stdout.queue(cursor::MoveTo(x, row))
                    .and_then(|s| s.queue(SetForegroundColor(Color::Rgb { r: (v as f32 * 0.6) as u8, g: (v as f32 * 0.65) as u8, b: v })))
                    .and_then(|s| s.queue(Print(ch)));
            }

            // peak hold dot
            if peak_h > bar_h && peak_h > 0 {
                let row_i = cy as isize - 1 - peak_h as isize;
                if row_i >= 1 {
                    let _ = stdout.queue(cursor::MoveTo(x, row_i as u16))
                        .and_then(|s| s.queue(SetForegroundColor(Color::Rgb { r: 140, g: 145, b: 165 })))
                        .and_then(|s| s.queue(Print("▪")));
                }
            }
        }

        let title = "  eq bars  ";
        let tx = (cols / 2).saturating_sub(title.len() as u16 / 2);
        stdout.queue(cursor::MoveTo(tx, 0)).unwrap()
              .queue(SetForegroundColor(Color::Rgb { r: 60, g: 60, b: 75 })).unwrap()
              .queue(Print(title)).unwrap();

        // quit hint
        stdout.queue(cursor::MoveTo(cols.saturating_sub(8), 0)).unwrap()
              .queue(SetForegroundColor(Color::Rgb { r: 45, g: 45, b: 65 })).unwrap()
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
