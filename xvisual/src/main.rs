
mod audio_read;
use audio_read::capture_audio;

use ringbuf::{
    HeapRb,
    traits::Split,
};
mod visual;
use visual::visual;
fn main() {
    let rb = HeapRb::<f32>::new(1024);
    let (prod, cons) = rb.split();
    std::thread::spawn(move || {
        if let Err(e) = capture_audio(prod) {
            eprintln!("audio error: {e}");
        }
    });
    visual(cons);
}