use crossterm::{
    cursor,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    QueueableCommand,
};
use std::{io::Write, sync::{Arc, atomic::{AtomicBool, Ordering}}};
use ringbuf::consumer::Consumer;
use super::AudioConsumer;

struct Neuron {
    x: f32,
    y: f32,
    activation: f32,
    hue: f32,
}

pub fn run(mut cons: AudioConsumer, running: Arc<AtomicBool>) {
    let mut stdout = std::io::stdout();
    let mut read_buf = vec![0.0f32; 512];
    let mut smoothed_rms = 0.0f32;
    let mut avg_rms = 0.01f32;
    let mut band_smoothed = [0.0f32; 16];
    let mut waveform = vec![0.0f32; 256];
    let mut time = 0.0f32;
    let mut beat_flash = 0.0f32;
    let mut rng: u64 = 0xdeadbeef_cafebabe;

    terminal::enable_raw_mode().unwrap();
    stdout.queue(EnterAlternateScreen).unwrap();
    stdout.queue(cursor::Hide).unwrap();
    stdout.flush().unwrap();

    let (mut last_cols, mut last_rows) = terminal::size().unwrap_or((80, 24));
    let mut neurons = init_neurons(90, last_cols, last_rows, &mut rng);
    let mut connections = build_connections(&neurons);
    let mut stars = init_stars(80, last_cols, last_rows, &mut rng);

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

        let (cols, rows) = terminal::size().unwrap_or((80, 24));
        if cols != last_cols || rows != last_rows {
            neurons = init_neurons(90, cols, rows, &mut rng);
            connections = build_connections(&neurons);
            stars = init_stars(80, cols, rows, &mut rng);
            last_cols = cols;
            last_rows = rows;
        }

        if total > 0 {
            let tail = &read_buf[total.saturating_sub(512)..total];
            let rms = (tail.iter().map(|s| s * s).sum::<f32>() / tail.len() as f32).sqrt();
            smoothed_rms = if rms > smoothed_rms { rms } else { rms * 0.20 + smoothed_rms * 0.80 };
            avg_rms = avg_rms * 0.995 + smoothed_rms * 0.005;

            if smoothed_rms > avg_rms * 2.2 && smoothed_rms > 0.015 {
                beat_flash = 1.0;
                let count = 4 + (smoothed_rms * 14.0) as usize;
                let nlen = neurons.len();
                for _ in 0..count {
                    let idx = (rand_f32(&mut rng) * nlen as f32) as usize;
                    neurons[idx.min(nlen - 1)].activation = 1.0;
                }
            }
            beat_flash = (beat_flash - 0.07).max(0.0);

            let bsz = (tail.len() / 16).max(1);
            for b in 0..16 {
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
            for b in &mut band_smoothed { *b *= 0.92; }
            smoothed_rms *= 0.96;
            beat_flash = (beat_flash - 0.07).max(0.0);
        }

        // Drive neurons from frequency bands mapped to x position
        for neuron in neurons.iter_mut() {
            let band_idx = ((neuron.x / cols as f32) * 16.0) as usize;
            let drive = band_smoothed[band_idx.min(15)] * 1.8;
            if drive > neuron.activation {
                neuron.activation = drive.min(1.0);
            }
        }

        // Spread activation along connections
        let acts: Vec<f32> = neurons.iter().map(|n| n.activation).collect();
        for &(i, j) in &connections {
            let ai = acts[i];
            let aj = acts[j];
            if ai > 0.20 && ai > aj + 0.06 {
                neurons[j].activation = (neurons[j].activation + (ai - aj) * 0.20).min(1.0);
            }
            if aj > 0.20 && aj > ai + 0.06 {
                neurons[i].activation = (neurons[i].activation + (aj - ai) * 0.20).min(1.0);
            }
        }

        // Decay
        for n in neurons.iter_mut() {
            n.activation *= 0.88;
            if n.activation < 0.01 { n.activation = 0.0; }
        }

        stdout.queue(terminal::Clear(terminal::ClearType::All)).unwrap();

        // Background star field
        for &(sx, sy, sh) in &stars {
            if sx < cols && sy >= 1 && sy < rows.saturating_sub(1) {
                let bright = 0.07 + beat_flash * 0.05;
                let (r, g, b) = hsl_to_rgb(sh, 0.50, bright);
                let _ = stdout
                    .queue(cursor::MoveTo(sx, sy))
                    .and_then(|s| s.queue(SetForegroundColor(Color::Rgb { r, g, b })))
                    .and_then(|s| s.queue(Print("·")));
            }
        }

        // Synaptic connections
        for &(i, j) in &connections {
            let act = (neurons[i].activation + neurons[j].activation) * 0.5;
            if act < 0.04 { continue; }
            let hue = (neurons[i].hue + neurons[j].hue) * 0.5;
            draw_connection(&mut stdout, &neurons[i], &neurons[j], act, hue, beat_flash, cols, rows);
        }

        // Neuron nodes
        for n in &neurons {
            if n.x < 1.0 || n.x >= cols as f32 || n.y < 1.0 || n.y >= (rows - 1) as f32 { continue; }
            let act = n.activation;
            let hue = if act > 0.60 { n.hue * 0.4 + 0.50 * 0.6 } else { n.hue };
            let bright = (0.10 + act * 0.75 + beat_flash * 0.08).clamp(0.0, 0.92);
            let sat = if act > 0.65 { 0.45 } else { 0.92 };
            let (r, g, b) = hsl_to_rgb(hue, sat, bright);
            let ch = if act > 0.72 { "◉" } else if act > 0.35 { "○" } else { "·" };
            let _ = stdout
                .queue(cursor::MoveTo(n.x as u16, n.y as u16))
                .and_then(|s| s.queue(SetForegroundColor(Color::Rgb { r, g, b })))
                .and_then(|s| s.queue(Print(ch)));
        }

        // EEG waveform at bottom
        draw_waveform(&mut stdout, &waveform, smoothed_rms, time, cols, rows);

        let title = "  x-visual · hallucination  ";
        let tx = (cols / 2).saturating_sub(title.len() as u16 / 2);
        stdout.queue(cursor::MoveTo(tx, 0)).unwrap()
              .queue(SetForegroundColor(Color::Rgb { r: 80, g: 30, b: 120 })).unwrap()
              .queue(Print(title)).unwrap();

        stdout.queue(cursor::MoveTo(cols.saturating_sub(8), 0)).unwrap()
              .queue(SetForegroundColor(Color::Rgb { r: 55, g: 22, b: 80 })).unwrap()
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

fn init_neurons(count: usize, cols: u16, rows: u16, rng: &mut u64) -> Vec<Neuron> {
    (0..count).map(|i| {
        let x = 2.0 + rand_f32(rng) * cols.saturating_sub(4) as f32;
        let y = 2.0 + rand_f32(rng) * rows.saturating_sub(6) as f32;
        let hue = 0.55 + (i as f32 / count as f32) * 0.35;
        Neuron { x, y, activation: 0.0, hue }
    }).collect()
}

fn build_connections(neurons: &[Neuron]) -> Vec<(usize, usize)> {
    let mut conns = Vec::new();
    let n = neurons.len();
    for i in 0..n {
        let mut nearby: Vec<(usize, f32)> = (i + 1..n).filter_map(|j| {
            let dx = neurons[i].x - neurons[j].x;
            let dy = (neurons[i].y - neurons[j].y) * 2.0; // terminal cells are ~2x taller
            let d = (dx * dx + dy * dy).sqrt();
            if d < 24.0 { Some((j, d)) } else { None }
        }).collect();
        nearby.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        for &(j, _) in nearby.iter().take(4) {
            conns.push((i, j));
        }
    }
    conns
}

fn init_stars(count: usize, cols: u16, rows: u16, rng: &mut u64) -> Vec<(u16, u16, f32)> {
    (0..count).map(|_| {
        let x = (rand_f32(rng) * cols as f32) as u16;
        let y = 1 + (rand_f32(rng) * rows.saturating_sub(3) as f32) as u16;
        let h = 0.55 + rand_f32(rng) * 0.35;
        (x, y, h)
    }).collect()
}

fn draw_connection(
    stdout: &mut std::io::Stdout,
    na: &Neuron, nb: &Neuron,
    activation: f32, hue: f32, beat: f32,
    cols: u16, rows: u16,
) {
    let dx = nb.x - na.x;
    let dy = nb.y - na.y;
    let steps = dx.abs().max(dy.abs()) as usize;
    if steps == 0 { return; }

    let ch = if dx.abs() > dy.abs() * 2.5 { "─" }
        else if dy.abs() > dx.abs() * 1.2 { "│" }
        else if (dx > 0.0) == (dy > 0.0) { "╲" }
        else { "╱" };

    let bright = (0.05 + activation * 0.50 + beat * 0.07).clamp(0.0, 0.68);
    let (r, g, b) = hsl_to_rgb(hue, 0.88, bright);

    for step in 1..steps {
        let t = step as f32 / steps as f32;
        let x = (na.x + dx * t) as u16;
        let y = (na.y + dy * t) as u16;
        if x < cols && y >= 1 && y < rows.saturating_sub(1) {
            let _ = stdout
                .queue(cursor::MoveTo(x, y))
                .and_then(|s| s.queue(SetForegroundColor(Color::Rgb { r, g, b })))
                .and_then(|s| s.queue(Print(ch)));
        }
    }
}

fn draw_waveform(
    stdout: &mut std::io::Stdout,
    waveform: &[f32],
    rms: f32, time: f32,
    cols: u16, rows: u16,
) {
    if cols < 4 || waveform.is_empty() { return; }
    let width = cols as usize - 4;
    let base_row = (rows - 3) as f32;
    let amp = (2.0 + rms * rows as f32 * 0.12).min(rows as f32 * 0.18);

    for xi in 0..width {
        let widx = (xi * waveform.len() / width).min(waveform.len() - 1);
        let sample = waveform[widx];
        let y = (base_row + sample * amp).clamp(1.0, (rows - 2) as f32) as u16;
        let t = xi as f32 / width as f32;
        let hue = (0.70 + time * 0.10 + t * 0.15) % 1.0;
        let (r, g, b) = hsl_to_rgb(hue, 0.95, 0.40 + rms * 0.28);
        let ch = if sample.abs() > 0.03 { "▪" } else { "·" };
        let _ = stdout
            .queue(cursor::MoveTo(2 + xi as u16, y))
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
