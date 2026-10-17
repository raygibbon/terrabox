use crate::buffer::Buffer;
use crate::{Cell, Position, Size, Style};
use unicode_segmentation::UnicodeSegmentation;

/// Display columns in a string, using the engine's Unicode width rules.
pub fn text_width(text: &str) -> usize {
    unicode_width::UnicodeWidthStr::width(text)
}

/// A half-open rectangle in terminal cell coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub const fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn right(self) -> u16 {
        self.x.saturating_add(self.width)
    }

    pub fn bottom(self) -> u16 {
        self.y.saturating_add(self.height)
    }

    pub fn intersection(self, other: Self) -> Self {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        Self::new(
            x,
            y,
            self.right().min(other.right()).saturating_sub(x),
            self.bottom().min(other.bottom()).saturating_sub(y),
        )
    }

    pub fn inset(self, amount: u16) -> Self {
        let horizontal = amount.min(self.width / 2);
        let vertical = amount.min(self.height / 2);
        Self::new(
            self.x.saturating_add(horizontal),
            self.y.saturating_add(vertical),
            self.width.saturating_sub(amount.saturating_mul(2)),
            self.height.saturating_sub(amount.saturating_mul(2)),
        )
    }

    pub fn contains(self, position: Position) -> bool {
        position.x >= self.x
            && position.x < self.right()
            && position.y >= self.y
            && position.y < self.bottom()
    }
}

/// An opaque off-screen buffer sharing Terminal's grapheme and cell rules.
#[derive(Clone)]
pub struct Surface {
    buffer: Buffer,
}

impl Surface {
    pub fn new(size: Size) -> Self {
        Self {
            buffer: Buffer::new(size),
        }
    }

    pub fn size(&self) -> Size {
        self.buffer.size
    }

    pub fn bounds(&self) -> Rect {
        Rect::new(0, 0, self.size().width, self.size().height)
    }

    /// Resize clears contents, matching Terminal's resize behaviour.
    pub fn resize(&mut self, size: Size) {
        self.buffer.resize(size);
    }

    pub fn clear(&mut self, style: Style) {
        self.buffer.clear_with_style(style);
    }

    /// Continuations and positions outside the surface return None.
    pub fn cell(&self, x: u16, y: u16) -> Option<&Cell> {
        self.buffer.cell(x, y)
    }

    /// Coordinates in the canvas are relative to the requested rectangle.
    pub fn canvas(&mut self, rect: Rect) -> Canvas<'_> {
        let clip = self.bounds().intersection(rect);
        Canvas {
            buffer: &mut self.buffer,
            origin: Position {
                x: rect.x,
                y: rect.y,
            },
            clip,
        }
    }

    /// Copy a source rectangle into this surface, clipping both sides atomically.
    pub fn blit(&mut self, source: &Surface, source_rect: Rect, position: Position) {
        let clip = self.bounds();
        copy_region(
            &source.buffer,
            source_rect,
            &mut self.buffer,
            position,
            clip,
        );
    }

    /// Scroll logical rows upward for positive values, downward for negative.
    /// Exposed cells use fill. Glyphs crossing the region boundary are preserved.
    pub fn scroll(&mut self, rect: Rect, rows: i32, fill: Style) {
        if rows == 0 {
            return;
        }
        let rect = self.bounds().intersection(rect);
        let previous = self.buffer.clone();
        let mut canvas = self.canvas(rect);
        canvas.fill(fill);
        for target_y in rect.y..rect.bottom() {
            let source_y = i64::from(target_y) + i64::from(rows);
            if source_y < i64::from(rect.y) || source_y >= i64::from(rect.bottom()) {
                continue;
            }
            copy_region(
                &previous,
                Rect::new(rect.x, source_y as u16, rect.width, 1),
                canvas.buffer,
                Position {
                    x: rect.x,
                    y: target_y,
                },
                rect,
            );
        }
    }

    pub(crate) fn copy_to(&self, target: &mut Buffer, position: Position) {
        let clip = Rect::new(0, 0, target.size.width, target.size.height);
        copy_region(&self.buffer, self.bounds(), target, position, clip);
    }
}

fn copy_region(source: &Buffer, rect: Rect, target: &mut Buffer, position: Position, clip: Rect) {
    let bounds = Rect::new(0, 0, source.size.width, source.size.height);
    let visible = rect.intersection(bounds);
    for source_y in visible.y..visible.bottom() {
        for source_x in visible.x..visible.right() {
            let Some(cell) = source.cell(source_x, source_y) else {
                continue;
            };
            if usize::from(source_x) + cell.width() > usize::from(visible.right()) {
                continue;
            }
            let target_x = u32::from(position.x) + u32::from(source_x - rect.x);
            let target_y = u32::from(position.y) + u32::from(source_y - rect.y);
            if target_x > u32::from(u16::MAX) || target_y > u32::from(u16::MAX) {
                continue;
            }
            put_clipped(target, clip, target_x as u16, target_y as u16, cell.clone());
        }
    }
}

fn put_clipped(buffer: &mut Buffer, clip: Rect, x: u16, y: u16, cell: Cell) {
    let width = cell.width();
    if !clip.contains(Position { x, y })
        || width == 0
        || width > 2
        || usize::from(x) + width > usize::from(clip.right())
    {
        return;
    }
    let index = usize::from(y) * usize::from(buffer.size.width) + usize::from(x);
    if buffer.cells[index].continuation && x == clip.x {
        return;
    }
    let last = index + width - 1;
    if x as usize + width == usize::from(clip.right())
        && last + 1 < buffer.cells.len()
        && buffer.cells[last + 1].continuation
    {
        return;
    }
    buffer.put_cell(x, y, cell);
}

/// A clipped drawing view. Wide glyphs that would cross its boundary are skipped.
pub struct Canvas<'a> {
    buffer: &'a mut Buffer,
    origin: Position,
    clip: Rect,
}

impl Canvas<'_> {
    /// Create a nested local view, intersecting the parent clip.
    pub fn clip(&mut self, rect: Rect) -> Canvas<'_> {
        let origin = Position {
            x: self.origin.x.saturating_add(rect.x),
            y: self.origin.y.saturating_add(rect.y),
        };
        let clip = self
            .clip
            .intersection(Rect::new(origin.x, origin.y, rect.width, rect.height));
        Canvas {
            buffer: self.buffer,
            origin,
            clip,
        }
    }

    pub fn size(&self) -> Size {
        Size {
            width: self.clip.right().saturating_sub(self.origin.x),
            height: self.clip.bottom().saturating_sub(self.origin.y),
        }
    }

    pub fn put_cell(&mut self, x: u16, y: u16, cell: Cell) {
        let Some(x) = self.origin.x.checked_add(x) else {
            return;
        };
        let Some(y) = self.origin.y.checked_add(y) else {
            return;
        };
        put_clipped(self.buffer, self.clip, x, y, cell);
    }

    pub fn put_char(&mut self, x: u16, y: u16, character: char, style: Style) {
        self.put_cell(x, y, Cell::new(character, style));
    }

    pub fn put_str(&mut self, x: u16, y: u16, text: &str, style: Style) {
        let start = u32::from(x);
        let mut column = start;
        let mut row = u32::from(y);
        for grapheme in text.graphemes(true) {
            if grapheme == "\n" {
                row += 1;
                column = start;
                continue;
            }
            let Some(cell) = Cell::grapheme(grapheme, style) else {
                continue;
            };
            let width = cell.width() as u32;
            if column <= u32::from(u16::MAX) && row <= u32::from(u16::MAX) {
                self.put_cell(column as u16, row as u16, cell);
            }
            column = column.saturating_add(width);
        }
    }

    pub fn fill(&mut self, style: Style) {
        for y in self.clip.y..self.clip.bottom() {
            for x in self.clip.x..self.clip.right() {
                put_clipped(self.buffer, self.clip, x, y, Cell::new(' ', style));
            }
        }
    }

    pub fn border(&mut self, style: Style) {
        let size = self.size();
        if size.width == 0 || size.height == 0 {
            return;
        }
        for x in 0..size.width {
            self.put_char(x, 0, '─', style);
            self.put_char(x, size.height - 1, '─', style);
        }
        for y in 0..size.height {
            self.put_char(0, y, '│', style);
            self.put_char(size.width - 1, y, '│', style);
        }
        for (x, y, character) in [
            (0, 0, '┌'),
            (size.width - 1, 0, '┐'),
            (0, size.height - 1, '└'),
            (size.width - 1, size.height - 1, '┘'),
        ] {
            self.put_char(x, y, character, style);
        }
    }
}
