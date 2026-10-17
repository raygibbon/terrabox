//! Optional static Kitty graphics, separate from character cells.
//!
//! Images belong to a Terminal. Upload transmits immediately; placement changes
//! commit with `present`. Full rectangles must fit: partial pixel clipping is
//! intentionally rejected. Static support does not establish native animation.
use crate::{Rect, Size};
use std::{
    collections::{BTreeMap, BTreeSet},
    io,
    sync::{
        Arc,
        atomic::{AtomicU32, AtomicU64, Ordering},
    },
};
mod kitty;

const MAX_BYTES: usize = 64 * 1024 * 1024;
const MAX_IMAGES: usize = 64;
static NEXT_ID: AtomicU32 = AtomicU32::new(1);
static NEXT_SLOT: AtomicU64 = AtomicU64::new(1);
pub(crate) fn allocate_id() -> io::Result<u32> {
    NEXT_ID
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .map_err(|_| io::Error::other("graphics IDs exhausted"))
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImageId(u32);
impl ImageId {
    pub fn get(self) -> u32 {
        self.0
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PlacementId(u32);
impl PlacementId {
    pub fn get(self) -> u32 {
        self.0
    }
}
/// Stable slot identity for optional retained image scenes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImageSlot(u64);
impl Default for ImageSlot {
    fn default() -> Self {
        Self::new()
    }
}
impl ImageSlot {
    pub fn new() -> Self {
        Self(
            NEXT_SLOT
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
                .expect("image slots exhausted"),
        )
    }
}
/// Straight (not premultiplied) RGBA8 pixels. Decoding is application-owned.
pub struct RgbaImage {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}
impl RgbaImage {
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> io::Result<Self> {
        let expected = usize::try_from(width)
            .ok()
            .and_then(|w| usize::try_from(height).ok().and_then(|h| w.checked_mul(h)))
            .and_then(|n| n.checked_mul(4));
        if width == 0 || height == 0 || expected != Some(pixels.len()) || pixels.len() > MAX_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "RGBA dimensions/length invalid or image exceeds 64 MiB",
            ));
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }
    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
}
/// A retained frame request. Reuse the same Arc to avoid retransmission.
#[derive(Clone)]
pub struct ImageRequest {
    pub slot: ImageSlot,
    pub image: Arc<RgbaImage>,
    pub area: Rect,
}
struct Placement {
    image: ImageId,
    area: Rect,
    dirty: bool,
}
struct SceneItem {
    image: Arc<RgbaImage>,
    id: ImageId,
    placement: PlacementId,
}
#[derive(Default)]
pub(crate) struct GraphicsState {
    images: BTreeMap<ImageId, usize>,
    placements: BTreeMap<PlacementId, Placement>,
    retired: BTreeMap<ImageId, usize>,
    removed: BTreeSet<(ImageId, PlacementId)>,
    scene: BTreeMap<ImageSlot, SceneItem>,
}
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
fn missing() -> io::Error {
    io::Error::new(
        io::ErrorKind::NotFound,
        "graphics resource is not owned by this terminal",
    )
}
fn validate_area(area: Rect, size: Size) -> io::Result<()> {
    if area.width == 0
        || area.height == 0
        || u32::from(area.x) + u32::from(area.width) > u32::from(size.width)
        || u32::from(area.y) + u32::from(area.height) > u32::from(size.height)
    {
        Err(invalid(
            "image placement must fit completely inside the terminal",
        ))
    } else {
        Ok(())
    }
}
impl GraphicsState {
    pub fn upload(
        &mut self,
        image: &RgbaImage,
        mut write: impl FnMut(&[u8]) -> io::Result<()>,
    ) -> io::Result<ImageId> {
        if self.images.len() + self.retired.len() >= MAX_IMAGES
            || self
                .images
                .values()
                .chain(self.retired.values())
                .sum::<usize>()
                .saturating_add(image.pixels.len())
                > MAX_BYTES
        {
            return Err(invalid("terminal graphics resource budget exceeded"));
        }
        let id = ImageId(allocate_id()?);
        // Register before I/O: an interrupted upload still needs deletion.
        self.images.insert(id, image.pixels.len());
        if let Err(error) = kitty::transmit(image, id, &mut write) {
            self.delete(id)?;
            let mut abort = b"\x1b\\".to_vec();
            abort.extend(kitty::delete_image(id));
            let _ = write(&abort);
            return Err(error);
        }
        Ok(id)
    }
    pub fn replace(
        &mut self,
        id: ImageId,
        image: &RgbaImage,
        mut write: impl FnMut(&[u8]) -> io::Result<()>,
    ) -> io::Result<()> {
        let old = *self.images.get(&id).ok_or_else(missing)?;
        if self
            .images
            .values()
            .chain(self.retired.values())
            .sum::<usize>()
            - old
            + image.pixels.len()
            > MAX_BYTES
        {
            return Err(invalid("terminal graphics resource budget exceeded"));
        }
        if let Err(error) = kitty::transmit(image, id, &mut write) {
            self.delete(id)?;
            let mut abort = b"\x1b\\".to_vec();
            abort.extend(kitty::delete_image(id));
            let _ = write(&abort);
            return Err(error);
        }
        self.images.insert(id, image.pixels.len());
        for placement in self.placements.values_mut().filter(|p| p.image == id) {
            placement.dirty = true;
        }
        Ok(())
    }
    pub fn place(&mut self, image: ImageId, area: Rect, size: Size) -> io::Result<PlacementId> {
        if !self.images.contains_key(&image) {
            return Err(missing());
        }
        validate_area(area, size)?;
        if self.placements.len() + self.removed.len() >= 256 {
            return Err(invalid("placement budget exceeded"));
        }
        let id = PlacementId(allocate_id()?);
        self.placements.insert(
            id,
            Placement {
                image,
                area,
                dirty: true,
            },
        );
        Ok(id)
    }
    pub fn move_placement(&mut self, id: PlacementId, area: Rect, size: Size) -> io::Result<()> {
        validate_area(area, size)?;
        let p = self.placements.get_mut(&id).ok_or_else(missing)?;
        if p.area != area {
            self.removed.insert((p.image, id));
            p.area = area;
            p.dirty = true;
        }
        Ok(())
    }
    pub fn remove_placement(&mut self, id: PlacementId) -> io::Result<()> {
        let p = self.placements.remove(&id).ok_or_else(missing)?;
        self.removed.insert((p.image, id));
        Ok(())
    }
    pub fn delete(&mut self, id: ImageId) -> io::Result<()> {
        let bytes = self.images.remove(&id).ok_or_else(missing)?;
        let placements: Vec<_> = self
            .placements
            .iter()
            .filter(|(_, p)| p.image == id)
            .map(|(id, _)| *id)
            .collect();
        for id in placements {
            self.remove_placement(id)?;
        }
        self.retired.insert(id, bytes);
        Ok(())
    }
    pub fn sync(
        &mut self,
        requests: &[ImageRequest],
        size: Size,
        mut write: impl FnMut(&[u8]) -> io::Result<()>,
    ) -> io::Result<()> {
        let mut slots = BTreeSet::new();
        for r in requests {
            validate_area(r.area, size)?;
            if !slots.insert(r.slot) {
                return Err(invalid("duplicate image slot"));
            }
        }
        let obsolete: Vec<_> = self
            .scene
            .keys()
            .filter(|s| !slots.contains(s))
            .copied()
            .collect();
        for slot in obsolete {
            let item = self.scene.remove(&slot).unwrap();
            if self.images.contains_key(&item.id) {
                self.delete(item.id)?;
            }
        }
        for r in requests {
            if let Some(mut item) = self.scene.remove(&r.slot)
                && self.images.contains_key(&item.id)
            {
                if !Arc::ptr_eq(&item.image, &r.image) {
                    if let Err(error) = self.replace(item.id, &r.image, &mut write) {
                        if self.images.contains_key(&item.id) {
                            self.scene.insert(r.slot, item);
                        }
                        return Err(error);
                    }
                    item.image = r.image.clone();
                }
                let placement = if self.placements.contains_key(&item.placement) {
                    self.move_placement(item.placement, r.area, size)
                        .map(|()| item.placement)
                } else {
                    self.place(item.id, r.area, size)
                };
                match placement {
                    Ok(id) => item.placement = id,
                    Err(error) => {
                        self.scene.insert(r.slot, item);
                        return Err(error);
                    }
                }
                self.scene.insert(r.slot, item);
                continue;
            }
            let id = self.upload(&r.image, &mut write)?;
            match self.place(id, r.area, size) {
                Ok(placement) => {
                    self.scene.insert(
                        r.slot,
                        SceneItem {
                            image: r.image.clone(),
                            id,
                            placement,
                        },
                    );
                }
                Err(e) => {
                    self.delete(id)?;
                    return Err(e);
                }
            }
        }
        Ok(())
    }
    pub fn prefix(&self) -> Vec<u8> {
        let mut out = Vec::new();
        if !self.retired.is_empty() || !self.removed.is_empty() {
            out.extend_from_slice(b"\x1b\\");
        }
        // Placement deletion precedes freeing image data.
        for (image, id) in &self.removed {
            out.extend(kitty::delete_placement(*image, *id));
        }
        for id in self.retired.keys() {
            out.extend(kitty::delete_image(*id));
        }
        out
    }
    pub fn suffix(
        &self,
        full_redraw: bool,
        mut cursor: impl FnMut(u16, u16) -> io::Result<Vec<u8>>,
    ) -> io::Result<Vec<u8>> {
        let mut out = Vec::new();
        for (id, p) in &self.placements {
            if p.dirty || full_redraw {
                out.extend(cursor(p.area.x, p.area.y)?);
                out.extend(kitty::place(p.image, *id, p.area));
            }
        }
        Ok(out)
    }
    pub fn commit(&mut self) {
        self.retired.clear();
        self.removed.clear();
        for p in self.placements.values_mut() {
            p.dirty = false;
        }
    }
    pub fn resize(&mut self) {
        for (id, p) in &self.placements {
            self.removed.insert((p.image, *id));
        }
        self.placements.clear();
    }
    pub fn cleanup(&self) -> Vec<u8> {
        let mut out = Vec::new();
        if !self.images.is_empty() || !self.retired.is_empty() {
            out.extend_from_slice(b"\x1b\\");
        }
        for id in self.images.keys().chain(self.retired.keys()) {
            out.extend(kitty::delete_image(*id));
        }
        out
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    pub fn reject(&mut self, bytes: &[u8]) {
        let Ok(text) = std::str::from_utf8(bytes) else {
            return;
        };
        let Some(body) = text
            .strip_prefix("\x1b_G")
            .and_then(|s| s.strip_suffix("\x1b\\"))
        else {
            return;
        };
        let Some((header, message)) = body.split_once(';') else {
            return;
        };
        if message == "OK" {
            return;
        }
        if let Some(id) = header
            .split(',')
            .find_map(|s| s.strip_prefix("i=")?.parse().ok())
        {
            let id = ImageId(id);
            if self.images.contains_key(&id) {
                let _ = self.delete(id);
            }
        }
    }
}
#[cfg(test)]
mod tests;
