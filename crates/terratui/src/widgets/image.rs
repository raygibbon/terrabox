use crate::{Theme, Widget};
use std::sync::Arc;
use terrabox::{
    Canvas, Rect,
    graphics::{ImageRequest, ImageSlot, RgbaImage},
};

/// Optional static image view. Decoding/assets belong to the application.
/// The engine frees its slot when hidden, replaced, or covered by a modal/menu.
pub struct ImageView {
    pub title: String,
    pub image: Option<Arc<RgbaImage>>,
    pub disabled: bool,
    slot: ImageSlot,
    available: bool,
}
impl ImageView {
    pub fn new(title: impl Into<String>, image: Option<Arc<RgbaImage>>) -> Self {
        Self {
            title: title.into(),
            image,
            disabled: false,
            slot: ImageSlot::new(),
            available: false,
        }
    }
}
impl Widget for ImageView {
    fn prepare_graphics(&mut self, available: bool) {
        self.available = available && !self.disabled;
    }
    fn collect_graphics(&self, area: Rect, requests: &mut Vec<ImageRequest>) {
        if self.available
            && area.width > 2
            && area.height > 2
            && let Some(image) = &self.image
        {
            requests.push(ImageRequest {
                slot: self.slot,
                image: image.clone(),
                area: Rect::new(area.x + 1, area.y + 1, area.width - 2, area.height - 2),
            });
        }
    }
    fn draw(&mut self, canvas: &mut Canvas<'_>, focused: bool, theme: &Theme) {
        canvas.fill(theme.surface);
        canvas.border(if focused { theme.focused } else { theme.border });
        let size = canvas.size();
        if size.width < 3 || size.height < 3 {
            return;
        }
        canvas
            .clip(Rect::new(1, 0, size.width - 2, 1))
            .put_str(0, 0, &self.title, theme.border);
        if !self.available || self.image.is_none() {
            let mut content = canvas.clip(Rect::new(1, 1, size.width - 2, size.height - 2));
            content.put_str(
                1,
                1,
                "Text preview · graphics unconfirmed/hidden",
                theme.surface,
            );
            if let Some(image) = &self.image {
                content.put_str(
                    1,
                    3,
                    &format!("RGBA {} × {} pixels", image.width(), image.height()),
                    theme.surface,
                );
                content.put_str(
                    1,
                    5,
                    "Image data stays available.\nUse a Kitty-compatible terminal and probe.\nGraphics are hidden while a menu/dialog opens.",
                    theme.surface,
                );
            } else {
                content.put_str(1, 3, "No image selected", theme.surface);
            }
        }
    }
}
