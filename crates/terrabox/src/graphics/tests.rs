use super::*;
#[test]
fn base64_rfc4648_vectors() {
    for (plain, encoded) in [
        ("", ""),
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg=="),
        ("fooba", "Zm9vYmE="),
        ("foobar", "Zm9vYmFy"),
    ] {
        assert_eq!(kitty::base64(plain.as_bytes()), encoded.as_bytes());
    }
}

fn image(bytes: usize) -> RgbaImage {
    RgbaImage::new((bytes / 4) as u32, 1, vec![123; bytes]).unwrap()
}
fn size() -> Size {
    Size {
        width: 80,
        height: 24,
    }
}
#[test]
fn rgba_validation_and_chunk_boundaries_preserve_payload() {
    assert!(RgbaImage::new(0, 1, vec![]).is_err());
    assert!(RgbaImage::new(1, 1, vec![0; 3]).is_err());
    assert!(RgbaImage::new(u32::MAX, u32::MAX, vec![]).is_err());
    for length in [4, 3072, 3076, 6144, 6148] {
        let image = image(length);
        let mut chunks = Vec::new();
        kitty::transmit(&image, ImageId(17), |chunk| {
            chunks.push(chunk.to_vec());
            Ok(())
        })
        .unwrap();
        assert_eq!(chunks.len(), length.div_ceil(3072));
        let mut payload = Vec::new();
        for (i, chunk) in chunks.iter().enumerate() {
            let at = chunk.iter().position(|b| *b == b';').unwrap();
            let part = &chunk[at + 1..chunk.len() - 2];
            assert!(part.len() <= 4096 && part.len() % 4 == 0);
            assert!(chunk.ends_with(b"\x1b\\"));
            assert!(
                std::str::from_utf8(&chunk[..at])
                    .unwrap()
                    .ends_with(if i + 1 < chunks.len() { "m=1" } else { "m=0" })
            );
            payload.extend_from_slice(part);
        }
        assert_eq!(payload, kitty::base64(image.pixels()));
        assert!(chunks[0].starts_with(b"\x1b_Ga=t,t=d,f=32,"));
        if chunks.len() > 1 {
            assert!(chunks[1].starts_with(b"\x1b_Gm="));
        }
    }
}
#[test]
fn interrupted_upload_registers_then_aborts_and_deletes_its_own_id() {
    let mut state = GraphicsState::default();
    let mut writes = Vec::new();
    let mut calls = 0;
    let result = state.upload(&image(6144), |bytes| {
        calls += 1;
        writes.push(bytes.to_vec());
        if calls == 2 {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "simulated partial write",
            ))
        } else {
            Ok(())
        }
    });
    assert!(result.is_err());
    assert!(state.images.is_empty());
    assert_eq!(state.retired.len(), 1);
    assert!(
        writes
            .last()
            .unwrap()
            .starts_with(b"\x1b\\\x1b_Ga=d,d=I,i=")
    );
    assert!(!String::from_utf8_lossy(writes.last().unwrap()).contains("d=A"));
}
#[test]
fn ids_are_unique_and_foreign_resources_are_rejected() {
    let mut first = GraphicsState::default();
    let mut second = GraphicsState::default();
    let a = first.upload(&image(4), |_| Ok(())).unwrap();
    let b = second.upload(&image(4), |_| Ok(())).unwrap();
    assert_ne!(a, b);
    assert_ne!(a.get(), 0);
    assert!(second.place(a, Rect::new(0, 0, 1, 1), size()).is_err());
    assert!(second.delete(a).is_err());
    assert!(second.images.contains_key(&b));
}
#[test]
fn placements_clip_safely_resize_and_free_in_order() {
    let mut state = GraphicsState::default();
    let id = state.upload(&image(4), |_| Ok(())).unwrap();
    for rect in [
        Rect::new(79, 0, 2, 1),
        Rect::new(u16::MAX, 0, 2, 1),
        Rect::new(0, 0, 0, 1),
    ] {
        assert!(state.place(id, rect, size()).is_err());
    }
    let p = state.place(id, Rect::new(2, 3, 10, 4), size()).unwrap();
    let suffix = state
        .suffix(false, |x, y| Ok(format!("cursor:{x},{y}").into_bytes()))
        .unwrap();
    assert!(suffix.starts_with(b"cursor:2,3\x1b_Ga=p"));
    assert!(
        String::from_utf8(suffix)
            .unwrap()
            .contains("c=10,r=4,C=1,z=-1")
    );
    state.commit();
    assert!(state.suffix(false, |_, _| Ok(vec![])).unwrap().is_empty());
    state
        .move_placement(p, Rect::new(4, 5, 10, 4), size())
        .unwrap();
    assert!(state.prefix().windows(5).any(|b| b == b"a=d,d"));
    state.resize();
    assert!(state.placements.is_empty());
    assert!(state.images.contains_key(&id));
    assert!(
        state
            .move_placement(p, Rect::new(0, 0, 1, 1), size())
            .is_err()
    );
    state.delete(id).unwrap();
    let prefix = String::from_utf8(state.prefix()).unwrap();
    assert!(prefix.find("d=i").unwrap() < prefix.find("d=I").unwrap());
    assert!(!state.cleanup().is_empty());
    state.commit();
    assert!(state.cleanup().is_empty());
}
#[test]
fn retained_scenes_reuse_pixels_and_release_hidden_slots() {
    let mut state = GraphicsState::default();
    let slot = ImageSlot::new();
    let data = Arc::new(image(16));
    let request = ImageRequest {
        slot,
        image: data.clone(),
        area: Rect::new(1, 1, 5, 5),
    };
    let mut uploads = 0;
    state
        .sync(std::slice::from_ref(&request), size(), |_| {
            uploads += 1;
            Ok(())
        })
        .unwrap();
    assert_eq!(uploads, 1);
    state.commit();
    state
        .sync(std::slice::from_ref(&request), size(), |_| {
            uploads += 1;
            Ok(())
        })
        .unwrap();
    assert_eq!(uploads, 1);
    let moved = ImageRequest {
        area: Rect::new(2, 2, 5, 5),
        ..request.clone()
    };
    state
        .sync(&[moved], size(), |_| {
            uploads += 1;
            Ok(())
        })
        .unwrap();
    assert_eq!(uploads, 1);
    state.resize();
    state
        .sync(std::slice::from_ref(&request), size(), |_| {
            uploads += 1;
            Ok(())
        })
        .unwrap();
    assert_eq!(uploads, 1);
    let changed = ImageRequest {
        image: Arc::new(image(20)),
        ..request.clone()
    };
    state
        .sync(&[changed], size(), |_| {
            uploads += 1;
            Ok(())
        })
        .unwrap();
    assert_eq!(uploads, 2);
    state.sync(&[], size(), |_| Ok(())).unwrap();
    assert!(state.images.is_empty());
    assert_eq!(state.retired.len(), 1);
    state.commit();
    assert!(state.cleanup().is_empty());
    assert!(
        state
            .sync(&[request.clone(), request], size(), |_| Ok(()))
            .is_err()
    );
}
#[test]
fn pending_deletions_and_placements_are_bounded_until_present_commits() {
    let mut state = GraphicsState::default();
    for _ in 0..MAX_IMAGES {
        let id = state.upload(&image(4), |_| Ok(())).unwrap();
        state.delete(id).unwrap();
    }
    assert!(state.upload(&image(4), |_| Ok(())).is_err());
    let before = state.prefix();
    assert!(!before.is_empty());
    assert_eq!(state.prefix(), before); // failed presentation can retry.
    state.commit();
    let id = state.upload(&image(4), |_| Ok(())).unwrap();
    for _ in 0..256 {
        let placement = state.place(id, Rect::new(0, 0, 1, 1), size()).unwrap();
        state.remove_placement(placement).unwrap();
    }
    assert!(state.place(id, Rect::new(0, 0, 1, 1), size()).is_err());
    state.commit();
    assert!(state.place(id, Rect::new(0, 0, 1, 1), size()).is_ok());
}
#[test]
fn asynchronous_upload_errors_retire_the_resource_without_touching_others() {
    let mut state = GraphicsState::default();
    let a = state.upload(&image(4), |_| Ok(())).unwrap();
    let b = state.upload(&image(4), |_| Ok(())).unwrap();
    state.reject(format!("\x1b_Gi={};ENOSPC:quota\x1b\\", a.get()).as_bytes());
    assert!(!state.images.contains_key(&a));
    assert!(state.images.contains_key(&b));
    assert!(state.cleanup().windows(4).any(|b| b == b"d=I,"));
}
