//! Blob assembly helpers: SHA-256 hashing, MIME sniffing, EXIF stripping.
//!
//! All three operate on `&[u8]`/`Bytes` (in-memory) for testability. The
//! upload path reads the assembled temp file into memory once before
//! hashing — fine for a demo Vault. Production would stream-hash with a
//! `Sha256` running across chunks. The trade-off is documented in LESSON.

use bytes::Bytes;
use sha2::{Digest, Sha256};

/// Lower-hex SHA-256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

/// Detect the MIME type by sniffing magic numbers. We never trust the
/// client-supplied `Content-Type`. `infer` reads the first ~16 bytes
/// and matches a database of magic-number signatures.
///
/// Falls back to `application/octet-stream` for unknown files.
pub fn sniff_mime(bytes: &[u8]) -> String {
    infer::get(bytes)
        .map(|t| t.mime_type().to_string())
        .unwrap_or_else(|| "application/octet-stream".to_string())
}

/// Returns true iff the MIME is one we'll re-encode to strip EXIF.
pub fn is_strippable_image(mime: &str) -> bool {
    matches!(mime, "image/jpeg" | "image/png")
}

/// Decode-then-re-encode roundtrip via the `image` crate. The output
/// has no EXIF, XMP, ICC, or other ancillary metadata — only pixel
/// data and the format header.
///
/// Returns the stripped bytes on success. On decode failure we return
/// the input unchanged: better to store the raw file than to fail an
/// upload because of a single odd image.
pub fn strip_exif(input: &Bytes, mime: &str) -> Bytes {
    use image::ImageFormat;
    let fmt = match mime {
        "image/jpeg" => ImageFormat::Jpeg,
        "image/png" => ImageFormat::Png,
        _ => return input.clone(),
    };
    let img = match image::load_from_memory_with_format(input, fmt) {
        Ok(i) => i,
        Err(e) => {
            tracing::warn!(error = %e, mime, "exif strip: decode failed, storing original");
            return input.clone();
        }
    };
    let mut out = std::io::Cursor::new(Vec::with_capacity(input.len()));
    if let Err(e) = img.write_to(&mut out, fmt) {
        tracing::warn!(error = %e, mime, "exif strip: encode failed, storing original");
        return input.clone();
    }
    Bytes::from(out.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_known_vector() {
        // Per the NIST FIPS 180-4 test vector for "abc".
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn sniff_unknown_is_octet_stream() {
        assert_eq!(
            sniff_mime(b"definitely not a known file format"),
            "application/octet-stream"
        );
    }

    #[test]
    fn sniff_png_magic() {
        // PNG signature: 0x89 'P' 'N' 'G' 0x0D 0x0A 0x1A 0x0A
        let png = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(sniff_mime(&png), "image/png");
    }

    #[test]
    fn is_strippable() {
        assert!(is_strippable_image("image/jpeg"));
        assert!(is_strippable_image("image/png"));
        assert!(!is_strippable_image("application/pdf"));
        assert!(!is_strippable_image("image/gif"));
    }

    #[test]
    fn strip_exif_passthrough_for_non_image() {
        let b = Bytes::from_static(b"hello world");
        let out = strip_exif(&b, "text/plain");
        assert_eq!(out, b);
    }
}
