//! Independent Kitty command encoder. Raw RGBA only; no image decoder.
use super::{ImageId, PlacementId, RgbaImage};
use crate::Rect;
use std::io;
const RAW_CHUNK: usize = 3072; // 4096 base64 bytes, divisible by three.
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
pub(super) fn base64(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let a = u32::from(chunk[0]);
        let b = u32::from(*chunk.get(1).unwrap_or(&0));
        let c = u32::from(*chunk.get(2).unwrap_or(&0));
        let word = (a << 16) | (b << 8) | c;
        out.push(ALPHABET[((word >> 18) & 63) as usize]);
        out.push(ALPHABET[((word >> 12) & 63) as usize]);
        out.push(if chunk.len() > 1 {
            ALPHABET[((word >> 6) & 63) as usize]
        } else {
            b'='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[(word & 63) as usize]
        } else {
            b'='
        });
    }
    out
}
pub(super) fn transmit(
    image: &RgbaImage,
    id: ImageId,
    mut write: impl FnMut(&[u8]) -> io::Result<()>,
) -> io::Result<()> {
    let chunks = image.pixels.chunks(RAW_CHUNK);
    let count = chunks.len();
    for (index, chunk) in chunks.enumerate() {
        let more = u8::from(index + 1 < count);
        let mut command = if index == 0 {
            format!(
                "\x1b_Ga=t,t=d,f=32,s={},v={},i={},q=1,m={more};",
                image.width, image.height, id.0
            )
            .into_bytes()
        } else {
            format!("\x1b_Gm={more};").into_bytes()
        };
        command.extend(base64(chunk));
        command.extend_from_slice(b"\x1b\\");
        write(&command)?;
    }
    Ok(())
}
pub(super) fn delete_image(id: ImageId) -> Vec<u8> {
    format!("\x1b_Ga=d,d=I,i={},q=2\x1b\\", id.0).into_bytes()
}
pub(super) fn delete_placement(image: ImageId, placement: PlacementId) -> Vec<u8> {
    format!("\x1b_Ga=d,d=i,i={},p={},q=2\x1b\\", image.0, placement.0).into_bytes()
}
pub(super) fn place(image: ImageId, placement: PlacementId, rect: Rect) -> Vec<u8> {
    // Text is above the image; the image remains above cell backgrounds.
    format!(
        "\x1b_Ga=p,i={},p={},c={},r={},C=1,z=-1,q=1\x1b\\",
        image.0, placement.0, rect.width, rect.height
    )
    .into_bytes()
}
