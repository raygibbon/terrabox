use std::time::{Duration, Instant};
use terrabox::{Color, Deadline, Position, Rect, Size, Style, Surface};

fn surface() -> Surface {
    Surface::new(Size {
        width: 8,
        height: 4,
    })
}

#[test]
fn nested_clips_preserve_graphemes_and_neighbours() {
    let mut surface = surface();
    surface
        .canvas(surface.bounds())
        .put_str(0, 0, "01234567", Style::default());
    let mut canvas = surface.canvas(Rect::new(1, 0, 5, 3));
    canvas
        .clip(Rect::new(1, 0, 3, 1))
        .put_str(0, 0, "e\u{301}界x", Style::default());
    assert_eq!(surface.cell(1, 0).unwrap().text(), "1");
    assert_eq!(surface.cell(2, 0).unwrap().text(), "e\u{301}");
    assert_eq!(surface.cell(3, 0).unwrap().text(), "界");
    assert!(surface.cell(4, 0).is_none());
    assert_eq!(surface.cell(5, 0).unwrap().text(), "5");
}

#[test]
fn clip_does_not_erase_a_wide_glyph_outside_its_bounds() {
    let mut surface = surface();
    surface
        .canvas(surface.bounds())
        .put_str(0, 0, "界x界", Style::default());
    surface.canvas(Rect::new(1, 0, 3, 1)).fill(Style::default());
    assert_eq!(surface.cell(0, 0).unwrap().text(), "界");
    assert!(surface.cell(1, 0).is_none());
    assert_eq!(surface.cell(2, 0).unwrap().text(), " ");
    assert_eq!(surface.cell(3, 0).unwrap().text(), "界");
    assert!(surface.cell(4, 0).is_none());
}

#[test]
fn blit_skips_partial_source_and_destination_glyphs() {
    let mut source = surface();
    source
        .canvas(source.bounds())
        .put_str(0, 0, "界e\u{301}👩‍💻", Style::default());
    let mut target = surface();
    target.blit(&source, Rect::new(1, 0, 3, 1), Position { x: 0, y: 0 });
    assert_eq!(target.cell(0, 0).unwrap().text(), " ");
    assert_eq!(target.cell(1, 0).unwrap().text(), "e\u{301}");
    assert_eq!(target.cell(2, 0).unwrap().text(), " ");
    target.blit(&source, source.bounds(), Position { x: 7, y: 0 });
    assert_eq!(target.cell(7, 0).unwrap().text(), " ");
}

#[test]
fn scrolling_moves_only_the_region_and_fills_exposed_rows() {
    let mut surface = surface();
    for (row, text) in ["outside", "界ab", "last", "bottom"].iter().enumerate() {
        surface
            .canvas(surface.bounds())
            .put_str(0, row as u16, text, Style::default());
    }
    let fill = Style::new(Color::Indexed(1), Color::Indexed(2));
    let rect = Rect::new(0, 1, 6, 2);
    surface.scroll(rect, 1, fill);
    assert_eq!(surface.cell(0, 0).unwrap().text(), "o");
    assert_eq!(surface.cell(0, 1).unwrap().text(), "l");
    assert_eq!(surface.cell(0, 2).unwrap().style, fill);
    assert_eq!(surface.cell(0, 3).unwrap().text(), "b");
    surface.scroll(rect, -1, fill);
    assert_eq!(surface.cell(0, 1).unwrap().text(), " ");
    assert_eq!(surface.cell(0, 2).unwrap().text(), "l");
    surface.scroll(rect, i32::MIN, fill);
    assert_eq!(surface.cell(0, 2).unwrap().text(), " ");
}

#[test]
fn empty_and_overflowing_rectangles_do_not_draw_or_panic() {
    let mut surface = surface();
    surface
        .canvas(Rect::new(u16::MAX, u16::MAX, u16::MAX, u16::MAX))
        .put_str(u16::MAX, u16::MAX, "界\nx", Style::default());
    surface.scroll(Rect::new(100, 100, 3, 3), 1, Style::default());
    surface.resize(Size::default());
    surface
        .canvas(Rect::new(0, 0, 10, 10))
        .border(Style::default());
    assert!(surface.cell(0, 0).is_none());
}

#[test]
fn deadline_remaining_is_monotonic_and_saturates_at_zero() {
    let now = Instant::now();
    let deadline = Deadline(now + Duration::from_millis(10));
    assert!(!deadline.is_due(now));
    assert_eq!(deadline.remaining(now), Duration::from_millis(10));
    assert!(deadline.is_due(now + Duration::from_millis(20)));
    assert_eq!(
        deadline.remaining(now + Duration::from_millis(20)),
        Duration::ZERO
    );
}

#[test]
fn region_scroll_matches_snapshot_reference_across_unicode_boundaries() {
    let mut original = surface();
    for (row, text) in ["界abc界x", "a界e\u{301}界!", "👩‍💻abc界x", "abcdef界"]
        .iter()
        .enumerate()
    {
        original
            .canvas(original.bounds())
            .put_str(0, row as u16, text, Style::default());
    }
    for x in 0..9 {
        for width in 0..10 {
            for y in 0..5 {
                for height in 0..6 {
                    for rows in [-8, -1, 0, 1, 8, i32::MIN, i32::MAX] {
                        let rect = Rect::new(x, y, width, height).intersection(original.bounds());
                        let fill = Style::new(Color::Indexed(1), Color::Indexed(2));
                        let mut expected = original.clone();
                        if rows != 0 {
                            expected.canvas(rect).fill(fill);
                            for target in rect.y..rect.bottom() {
                                let source = i64::from(target) + i64::from(rows);
                                if source >= i64::from(rect.y) && source < i64::from(rect.bottom())
                                {
                                    for sx in rect.x..rect.right() {
                                        if let Some(cell) = original.cell(sx, source as u16)
                                            && usize::from(sx) + terrabox::text_width(&cell.text())
                                                <= usize::from(rect.right())
                                        {
                                            expected.canvas(rect).put_cell(
                                                sx - rect.x,
                                                target - rect.y,
                                                cell.clone(),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        let mut actual = original.clone();
                        actual.scroll(rect, rows, fill);
                        for cy in 0..4 {
                            for cx in 0..8 {
                                assert_eq!(
                                    actual.cell(cx, cy),
                                    expected.cell(cx, cy),
                                    "rect={rect:?}, rows={rows}, cell={cx},{cy}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
