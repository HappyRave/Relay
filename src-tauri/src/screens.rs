//! The screenshot taken when a recording starts, drawn under the preview.
//! It's machine-local, like triggers and run counts: `screens\<id>.jpg` in
//! the data folder, never part of a `.rly` export (it can show anything that
//! was on screen).

use std::io;
use std::path::{Path, PathBuf};

use relay_platform::Snapshot;
use uuid::Uuid;

use crate::storage::write_atomic_bytes;

/// Snapshots are at most this wide: a Full HD or 4K monitor stays sharp in
/// the preview, three monitors side by side are halved.
pub const MAX_WIDTH: u32 = 3200;
const QUALITY: u8 = 80;

pub fn path(dir: &Path, id: Uuid) -> PathBuf {
    dir.join("screens").join(format!("{id}.jpg"))
}

/// The snapshot as a JPEG, or `None` if it can't be encoded.
pub fn encode(s: &Snapshot) -> Option<Vec<u8>> {
    let (w, h) = (u16::try_from(s.w).ok()?, u16::try_from(s.h).ok()?);
    let mut out = Vec::new();
    let encoder = jpeg_encoder::Encoder::new(&mut out, QUALITY);
    encoder.encode(&s.rgb, w, h, jpeg_encoder::ColorType::Rgb).ok()?;
    Some(out)
}

pub fn save(dir: &Path, id: Uuid, jpeg: &[u8]) -> io::Result<()> {
    write_atomic_bytes(&path(dir, id), jpeg)
}

/// The macro's screenshot, if it has one.
pub fn read(dir: &Path, id: Uuid) -> Option<Vec<u8>> {
    std::fs::read(path(dir, id)).ok()
}

/// Gives a duplicated macro its original's screenshot. Having none isn't an error.
pub fn copy(dir: &Path, from: Uuid, to: Uuid) -> io::Result<()> {
    match std::fs::read(path(dir, from)) {
        Ok(jpeg) => save(dir, to, &jpeg),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(w: u32, h: u32) -> Snapshot {
        let rgb = (0..w * h).flat_map(|i| [(i % 256) as u8, 40, 200]).collect();
        Snapshot { w, h, rgb }
    }

    #[test]
    fn snapshots_are_saved_as_jpeg_and_read_back() {
        let dir = tempfile::tempdir().unwrap();
        let id = Uuid::new_v4();
        assert_eq!(read(dir.path(), id), None);
        let jpeg = encode(&snapshot(64, 36)).unwrap();
        assert_eq!(&jpeg[..3], &[0xFF, 0xD8, 0xFF], "a JPEG");
        save(dir.path(), id, &jpeg).unwrap();
        assert_eq!(read(dir.path(), id), Some(jpeg));
        assert!(dir.path().join("screens").join(format!("{id}.jpg")).exists());
    }

    #[test]
    fn too_big_to_encode_is_none() {
        let s = Snapshot { w: 70_000, h: 1, rgb: vec![0; 3 * 70_000] };
        assert_eq!(encode(&s), None);
    }

    #[test]
    fn a_copy_gets_the_screenshot_and_having_none_is_fine() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b, c) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        save(dir.path(), a, b"jpeg").unwrap();
        copy(dir.path(), a, b).unwrap();
        assert_eq!(read(dir.path(), b).as_deref(), Some(&b"jpeg"[..]));
        copy(dir.path(), c, Uuid::new_v4()).unwrap();
    }
}
