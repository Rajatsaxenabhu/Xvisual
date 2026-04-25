pub mod classic;
pub mod bars;
pub mod car;
pub mod space;
pub mod plasma;
pub mod hallucination;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    QueueableCommand,
};
use std::{
    io::Write,
    sync::{Arc, atomic::{AtomicBool, Ordering}},
};

pub(crate) type AudioConsumer =
    ringbuf::wrap::caching::Caching<
        Arc<ringbuf::SharedRb<ringbuf::storage::Heap<f32>>>,
        false, true,
    >;

pub(crate) const TAU: f32 = 2.0 * std::f32::consts::PI;

pub fn visual(cons: AudioConsumer) {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || r.store(false, Ordering::SeqCst)).unwrap();

    let choice = show_menu(running.clone());
    if !running.load(Ordering::SeqCst) { return; }

    match choice {
        1 => classic::run(cons, running),
        2 => car::run(cons, running),
        3 => bars::run(cons, running),
        4 => space::run(cons, running),
        5 => plasma::run(cons, running),
        6 => hallucination::run(cons, running),
        _ => {}
    }
}

fn show_menu(running: Arc<AtomicBool>) -> u8 {
    let mut stdout = std::io::stdout();
    terminal::enable_raw_mode().unwrap();
    stdout.queue(EnterAlternateScreen).unwrap();
    stdout.queue(cursor::Hide).unwrap();
    stdout.flush().unwrap();

    let choice = draw_menu(&mut stdout, &running);

    stdout.queue(LeaveAlternateScreen).unwrap();
    stdout.queue(cursor::Show).unwrap();
    terminal::disable_raw_mode().unwrap();
    stdout.flush().unwrap();
    choice
}

fn draw_menu(stdout: &mut std::io::Stdout, running: &Arc<AtomicBool>) -> u8 {
    let (cols, rows) = terminal::size().unwrap_or((80, 24));
    let cx = cols / 2;
    let cy = rows / 2;

    let options: &[(&str, &str)] = &[
        ("1", "Classic Orb"),
        ("2", "Car Dashboard"),
        ("3", "Colored EQ Bars"),
        ("4", "Space Starfield"),
        ("5", "Plasma Wave"),
        ("6", "Hallucination"),
    ];

    let inner_w: usize = 28;
    let box_w = inner_w + 2;
    let bx = cx.saturating_sub(box_w as u16 / 2);
    let by = cy.saturating_sub((options.len() as u16 + 4) / 2);

    let border = Color::Rgb { r: 60, g: 60, b: 75 };
    let dim    = Color::Rgb { r: 50, g: 50, b: 60 };
    let text   = Color::Rgb { r: 80, g: 80, b: 100 };
    let key_c  = Color::Rgb { r: 140, g: 140, b: 155 };

    stdout.queue(terminal::Clear(terminal::ClearType::All)).unwrap();

    stdout.queue(cursor::MoveTo(bx, by)).unwrap()
          .queue(SetForegroundColor(border)).unwrap()
          .queue(Print(format!("╔{:─<w$}╗", "", w = inner_w))).unwrap();

    let title = " x-visual ";
    let pad = inner_w.saturating_sub(title.len());
    stdout.queue(cursor::MoveTo(bx, by + 1)).unwrap()
          .queue(SetForegroundColor(dim)).unwrap()
          .queue(Print("║")).unwrap()
          .queue(SetForegroundColor(text)).unwrap()
          .queue(Print(format!("{}{:w$}", title, "", w = pad))).unwrap()
          .queue(SetForegroundColor(dim)).unwrap()
          .queue(Print("║")).unwrap();

    stdout.queue(cursor::MoveTo(bx, by + 2)).unwrap()
          .queue(SetForegroundColor(dim)).unwrap()
          .queue(Print(format!("╟{:─<w$}╢", "", w = inner_w))).unwrap();

    for (i, (key, name)) in options.iter().enumerate() {
        let row = by + 3 + i as u16;
        let name_pad = inner_w.saturating_sub(7 + name.len());
        stdout.queue(cursor::MoveTo(bx, row)).unwrap()
              .queue(SetForegroundColor(dim)).unwrap()
              .queue(Print("║  ")).unwrap()
              .queue(SetForegroundColor(key_c)).unwrap()
              .queue(Print(format!("[{}]", key))).unwrap()
              .queue(SetForegroundColor(text)).unwrap()
              .queue(Print(format!("  {}{:w$}", name, "", w = name_pad))).unwrap()
              .queue(SetForegroundColor(dim)).unwrap()
              .queue(Print("║")).unwrap();
    }

    let last = by + 3 + options.len() as u16;
    stdout.queue(cursor::MoveTo(bx, last)).unwrap()
          .queue(SetForegroundColor(border)).unwrap()
          .queue(Print(format!("╚{:─<w$}╝", "", w = inner_w))).unwrap();

    stdout.queue(cursor::MoveTo(bx + 2, last + 1)).unwrap()
          .queue(SetForegroundColor(Color::Rgb { r: 55, g: 55, b: 65 })).unwrap()
          .queue(Print("1-6 select  ·  ctrl+c quit")).unwrap();

    stdout.queue(ResetColor).unwrap();
    stdout.flush().unwrap();

    loop {
        if !running.load(Ordering::SeqCst) { return 0; }
        if event::poll(std::time::Duration::from_millis(80)).unwrap_or(false) {
            if let Ok(Event::Key(k)) = event::read() {
                match k.code {
                    KeyCode::Char('1') => return 1,
                    KeyCode::Char('2') => return 2,
                    KeyCode::Char('3') => return 3,
                    KeyCode::Char('4') => return 4,
                    KeyCode::Char('5') => return 5,
                    KeyCode::Char('6') => return 6,
                    _ => {}
                }
            }
        }
    }
}

