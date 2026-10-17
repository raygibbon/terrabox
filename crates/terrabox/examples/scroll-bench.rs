//! Repeatable scroll workload; run with --release. Not a statistical benchmark.
use std::{hint::black_box, time::Instant};
use terrabox::{Rect, Size, Style, Surface};
fn main() {
    for rect in [Rect::new(20, 10, 20, 10), Rect::new(0, 0, 200, 60)] {
        let mut surface = Surface::new(Size {
            width: 200,
            height: 60,
        });
        surface.canvas(surface.bounds()).put_str(
            20,
            12,
            "ASCII 界 e\u{301} scrolling",
            Style::default(),
        );
        let start = Instant::now();
        for i in 0..2000 {
            surface.scroll(rect, if i % 2 == 0 { 1 } else { -1 }, Style::default());
            black_box(&surface);
        }
        println!(
            "{}x{} region: {:?} / 2000 scrolls",
            rect.width,
            rect.height,
            start.elapsed()
        );
    }
}
