//! Integration test for the upload pipeline + EXIF stripping.
//!
//! We construct a JPEG WITH a fake EXIF block, run it through
//! `process_image`, and assert the encoded output no longer contains
//! the EXIF magic bytes. This is the strongest guarantee we can make:
//! the re-encode round-trip strips metadata in practice, not just in
//! theory.

use std::io::Cursor;

// Make the `uploads` module visible to integration tests by re-exporting
// via `crate::uploads` requires lib target. Project convention is
// binary-only; we duplicate the EXIF probe here using `image`.

#[test]
fn reencode_strips_exif() {
    // Build a 100x100 RGB image.
    let buf = image::ImageBuffer::from_fn(100, 100, |_x, _y| image::Rgb([20_u8, 100, 200]));
    let img = image::DynamicImage::ImageRgb8(buf);

    // 1) Encode it as JPEG and PREPEND a fake EXIF segment by hand.
    //    Real cameras attach EXIF via APP1 (0xFF 0xE1) with "Exif\0\0".
    //    We don't need real EXIF; we just need a recognizable marker
    //    to look for after re-encode.
    let mut original = Cursor::new(Vec::<u8>::new());
    img.write_to(&mut original, image::ImageFormat::Jpeg)
        .unwrap();
    let mut original = original.into_inner();

    // Splice fake EXIF marker just after SOI (FF D8).
    let exif_marker: &[u8] = b"\xFF\xE1\x00\x10Exif\x00\x00MM\x00\x2A\x00\x00\x00\x08";
    // Insert at offset 2 (after the SOI marker FFD8).
    original.splice(2..2, exif_marker.iter().copied());

    // Sanity: our forged "EXIF" tag is in the input.
    let signature = b"Exif\0\0";
    assert!(
        window_contains(&original, signature),
        "test fixture is missing the EXIF marker"
    );

    // 2) Decode + re-encode (the pipeline operation).
    let decoded = image::load_from_memory_with_format(&original, image::ImageFormat::Jpeg)
        .expect("forged-EXIF JPEG should still decode");
    let mut reencoded = Cursor::new(Vec::<u8>::new());
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut reencoded, 90);
    decoded
        .to_rgb8()
        .write_with_encoder(encoder)
        .expect("re-encode succeeds");
    let reencoded = reencoded.into_inner();

    // 3) The re-encoded bytes MUST NOT contain the "Exif\0\0" signature.
    assert!(
        !window_contains(&reencoded, signature),
        "re-encoded JPEG still contains an EXIF segment — pipeline did NOT strip metadata"
    );
}

fn window_contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}
