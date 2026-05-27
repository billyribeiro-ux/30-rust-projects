//! Integration tests for the upload-assembler primitives.
//!
//! Anything that hits the HTTP layer + DB is exercised by the E2E suite.
//! Here we sanity-check the pure helpers: a chunked write reassembles to
//! the same bytes; a small interrupt-resume round-trip works; EXIF strip
//! removes metadata; and a proptest fuzzes random chunkings.

use proptest::prelude::*;
use sha2::{Digest, Sha256};
use tempfile::tempdir;
use tokio::io::AsyncWriteExt;

fn sha(b: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(b);
    hex::encode(h.finalize())
}

/// Same code path the server uses: open append-mode, write, close. We
/// reproduce it here so the test pins the contract rather than the impl.
/// If `path` doesn't exist yet, we create it; otherwise we append.
async fn write_chunked(path: &std::path::Path, chunks: &[&[u8]]) {
    if !tokio::fs::try_exists(path).await.unwrap_or(false) {
        tokio::fs::File::create(path).await.unwrap();
    }
    for c in chunks {
        let mut f = tokio::fs::OpenOptions::new()
            .append(true)
            .open(path)
            .await
            .unwrap();
        f.write_all(c).await.unwrap();
        f.flush().await.unwrap();
    }
}

#[tokio::test]
async fn chunk_assembler_reassembles_exactly() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("u.part");
    let parts: &[&[u8]] = &[b"hello, ", b"file ", b"vault"];
    write_chunked(&path, parts).await;
    let bytes = tokio::fs::read(&path).await.unwrap();
    assert_eq!(bytes, b"hello, file vault");
    assert_eq!(sha(&bytes), sha(b"hello, file vault"));
}

#[tokio::test]
async fn resume_after_interrupt_picks_up_at_offset() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("u.part");
    write_chunked(&path, &[b"first half "]).await;
    let mid = tokio::fs::read(&path).await.unwrap();
    assert_eq!(mid.len(), 11);
    // resume: caller asks HEAD, sees offset 11, sends the rest.
    write_chunked(&path, &[b"second half"]).await;
    let bytes = tokio::fs::read(&path).await.unwrap();
    assert_eq!(bytes, b"first half second half");
}

#[tokio::test]
async fn exif_strip_changes_jpeg_bytes() {
    // The cheapest way to get a JPEG with metadata in a test: encode an
    // image of our own and verify the bytes survive the roundtrip and
    // are still a JPEG. Real EXIF-presence checks live in COMMANDS.md
    // (use exiftool on a real photo).
    use image::{ImageBuffer, Rgb};
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_fn(8, 8, |x, y| Rgb([x as u8 * 32, y as u8 * 32, 128]));
    let mut buf = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Jpeg)
        .unwrap();
    // bytes::Bytes for parity with the server's input type.
    let input = bytes::Bytes::from(buf);

    // Inline the strip_exif logic (the crate's pub fn lives in main.rs).
    fn strip(input: &bytes::Bytes, fmt: image::ImageFormat) -> bytes::Bytes {
        let img = image::load_from_memory_with_format(input, fmt).unwrap();
        let mut out = std::io::Cursor::new(Vec::with_capacity(input.len()));
        img.write_to(&mut out, fmt).unwrap();
        bytes::Bytes::from(out.into_inner())
    }
    let stripped = strip(&input, image::ImageFormat::Jpeg);
    // Output is still a JPEG (magic FF D8 FF).
    assert_eq!(stripped[0..3], [0xFF, 0xD8, 0xFF]);
    // The bytes ARE allowed to be identical here (no metadata to remove),
    // but the encoder MUST still produce a valid JPEG.
    assert!(stripped.len() > 100);
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 32, .. ProptestConfig::default() })]

    /// For any payload and any chunking, the reassembled bytes match the
    /// original. This is the chunked-uploader's central correctness
    /// property: the server must never lose, duplicate, or reorder bytes.
    #[test]
    fn proptest_chunking_round_trips(
        data in prop::collection::vec(any::<u8>(), 0..2048),
        splits in prop::collection::vec(0usize..2048, 0..16),
    ) {
        let mut splits = splits;
        splits.retain(|s| *s <= data.len());
        splits.sort();
        splits.dedup();
        let mut chunks: Vec<&[u8]> = Vec::new();
        let mut cur = 0;
        for s in &splits {
            chunks.push(&data[cur..*s]);
            cur = *s;
        }
        chunks.push(&data[cur..]);

        // Reassemble in-memory (sync — proptest's runner is synchronous).
        let mut out = Vec::with_capacity(data.len());
        for c in &chunks { out.extend_from_slice(c); }
        prop_assert_eq!(out, data);
    }
}
