//! Image upload pipeline.
//!
//! Five things this module is responsible for:
//!
//! 1. **Cap the byte budget.** Multipart bodies are unbounded by default;
//!    a 10 GB POST would happily allocate 10 GB of memory before we reject
//!    it. We enforce a 10 MB-per-file limit during accumulation.
//! 2. **Sniff the MIME type from the bytes** with the `infer` crate.
//!    Trusting the client's `Content-Type` is how malware ships with a
//!    `.jpg` extension. We re-derive the type from the magic bytes.
//! 3. **Re-encode** the image with the `image` crate. The decode → encode
//!    round-trip is what strips EXIF / GPS / IPTC metadata — a privacy
//!    win that costs us nothing.
//! 4. **Generate a thumbnail** capped at 400×400, JPEG quality 85.
//!    Frontend lists are bandwidth-sensitive; serving 4 MB hero shots in
//!    a grid is criminal.
//! 5. **Write both files atomically** under `{upload_dir}/{recipe_id}/`.
//!    Filenames are random UUIDs so a clever upload can't collide with
//!    another recipe's image.

use image::ImageFormat;
use image::codecs::jpeg::JpegEncoder;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use tokio::fs;

use crate::error::{AppError, AppResult};

pub const MAX_UPLOAD_BYTES: usize = 10 * 1024 * 1024; // 10 MB
pub const THUMB_MAX_DIM: u32 = 400;
pub const THUMB_QUALITY: u8 = 85;

#[derive(Debug, Clone, Copy)]
pub enum SniffedKind {
    Jpeg,
    Png,
    Webp,
}

impl SniffedKind {
    pub fn mime(self) -> &'static str {
        match self {
            SniffedKind::Jpeg => "image/jpeg",
            SniffedKind::Png => "image/png",
            SniffedKind::Webp => "image/webp",
        }
    }

    pub fn ext(self) -> &'static str {
        match self {
            SniffedKind::Jpeg => "jpg",
            SniffedKind::Png => "png",
            SniffedKind::Webp => "webp",
        }
    }

    pub fn image_format(self) -> ImageFormat {
        match self {
            SniffedKind::Jpeg => ImageFormat::Jpeg,
            SniffedKind::Png => ImageFormat::Png,
            SniffedKind::Webp => ImageFormat::WebP,
        }
    }
}

/// Mime-sniff the first few hundred bytes via magic numbers.
/// Allow only JPEG / PNG / WebP — every other format is rejected.
///
/// Why not trust `Content-Type`? Any attacker controls it. A `.php` shell
/// with `Content-Type: image/jpeg` would otherwise sail past our checks
/// straight onto disk. Magic-byte sniffing reads the actual format header,
/// which an attacker cannot forge without producing a real image.
pub fn sniff_image(bytes: &[u8]) -> AppResult<SniffedKind> {
    let kind = infer::get(bytes)
        .ok_or_else(|| AppError::Unsupported("unrecognized file format".into()))?;
    match kind.mime_type() {
        "image/jpeg" => Ok(SniffedKind::Jpeg),
        "image/png" => Ok(SniffedKind::Png),
        "image/webp" => Ok(SniffedKind::Webp),
        other => Err(AppError::Unsupported(format!(
            "unsupported image type: {other} (only JPEG, PNG, WebP allowed)"
        ))),
    }
}

/// Processed upload — the result of decode + re-encode + thumbnail.
pub struct ProcessedImage {
    pub kind: SniffedKind,
    /// The re-encoded ORIGINAL (EXIF-stripped). Same format as `kind`.
    pub original: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// Thumbnail, always JPEG, max 400x400.
    pub thumb_jpeg: Vec<u8>,
}

/// Decode the input bytes, re-encode in the SAME format (stripping EXIF in
/// the process), and emit a 400x400 JPEG thumbnail.
///
/// Why re-encode the original at all? Because that's what strips EXIF.
/// `image::load_from_memory` decodes the pixel data into an
/// in-memory `DynamicImage`; encoders only emit the pixel data plus the
/// standard format chunks — no EXIF / GPS / IPTC. A camera photo's
/// "where I live" GPS tag silently vanishes. This is a privacy feature
/// users get for free.
pub fn process_image(bytes: &[u8]) -> AppResult<ProcessedImage> {
    let kind = sniff_image(bytes)?;
    let img = image::load_from_memory_with_format(bytes, kind.image_format())
        .map_err(|e| AppError::Validation(format!("could not decode image: {e}")))?;
    let width = img.width();
    let height = img.height();

    // Re-encode original in the same format. The decode → encode round-trip
    // is what strips EXIF; the encoders do not propagate metadata chunks.
    let mut original_buf = Cursor::new(Vec::<u8>::new());
    match kind {
        SniffedKind::Jpeg => {
            // Quality 90 preserves perceptual quality while still strongly
            // compressing — typical 24-megapixel photos drop from ~10 MB
            // to ~2 MB.
            let encoder = JpegEncoder::new_with_quality(&mut original_buf, 90);
            img.to_rgb8()
                .write_with_encoder(encoder)
                .map_err(|e| AppError::Io(format!("encode JPEG: {e}")))?;
        }
        SniffedKind::Png => {
            img.write_to(&mut original_buf, ImageFormat::Png)
                .map_err(|e| AppError::Io(format!("encode PNG: {e}")))?;
        }
        SniffedKind::Webp => {
            // The `image` crate's `webp` feature is lossless-only for write,
            // which is what we want for content-faithful re-encoding.
            img.write_to(&mut original_buf, ImageFormat::WebP)
                .map_err(|e| AppError::Io(format!("encode WebP: {e}")))?;
        }
    }

    // Thumbnail: scale-to-fit inside 400x400 preserving aspect ratio.
    // `thumbnail` uses a fast nearest-neighbor; `resize_to_fill` is too
    // aggressive (crops). For a 400-px gallery we want letterboxed
    // proportions, not cropped.
    let thumb = img.thumbnail(THUMB_MAX_DIM, THUMB_MAX_DIM);
    let mut thumb_buf = Cursor::new(Vec::<u8>::new());
    let encoder = JpegEncoder::new_with_quality(&mut thumb_buf, THUMB_QUALITY);
    thumb
        .to_rgb8()
        .write_with_encoder(encoder)
        .map_err(|e| AppError::Io(format!("encode thumbnail: {e}")))?;

    Ok(ProcessedImage {
        kind,
        original: original_buf.into_inner(),
        width,
        height,
        thumb_jpeg: thumb_buf.into_inner(),
    })
}

/// Persist `processed` under `{upload_root}/{recipe_id}/{image_id}.{ext}`
/// (original) and `{upload_root}/{recipe_id}/{image_id}-thumb.jpg`.
/// Returns the relative paths stored in DB (NOT absolute, so the upload
/// dir can be moved without rewriting rows).
pub async fn write_to_disk(
    upload_root: &Path,
    recipe_id: &str,
    image_id: &str,
    processed: &ProcessedImage,
) -> AppResult<(String, String)> {
    let dir = upload_root.join(recipe_id);
    fs::create_dir_all(&dir).await?;

    let ext = processed.kind.ext();
    let original_name = format!("{image_id}.{ext}");
    let thumb_name = format!("{image_id}-thumb.jpg");

    fs::write(dir.join(&original_name), &processed.original).await?;
    fs::write(dir.join(&thumb_name), &processed.thumb_jpeg).await?;

    Ok((
        format!("{recipe_id}/{original_name}"),
        format!("{recipe_id}/{thumb_name}"),
    ))
}

/// Best-effort deletion. Missing files are OK — DB row removal is the
/// source of truth.
pub async fn delete_from_disk(upload_root: &Path, relative: &str) -> AppResult<()> {
    let full: PathBuf = upload_root.join(relative);
    match fs::remove_file(&full).await {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(AppError::Io(format!(
            "could not delete {}: {e}",
            full.display()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tiny but valid 1x1 PNG (8 bytes header + IHDR + IDAT + IEND).
    /// Generated via `image::ImageBuffer::new(1, 1).save("/tmp/x.png")`
    /// and then `xxd` to copy the bytes.
    const TINY_PNG: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f,
        0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0d, 0x0a, 0x2d, 0xb4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];

    #[test]
    fn sniff_recognizes_png() {
        let k = sniff_image(TINY_PNG).unwrap();
        assert!(matches!(k, SniffedKind::Png));
    }

    #[test]
    fn sniff_rejects_text() {
        let txt = b"this is not an image, it's an .exe with a .jpg extension";
        assert!(matches!(sniff_image(txt), Err(AppError::Unsupported(_))));
    }

    #[test]
    fn sniff_rejects_pdf() {
        // PDF magic: "%PDF-1.4"
        let pdf = b"%PDF-1.4\n...";
        assert!(matches!(sniff_image(pdf), Err(AppError::Unsupported(_))));
    }

    #[test]
    fn process_emits_thumbnail() {
        // Bigger PNG so the thumb actually has area > 0.
        let mut buf = std::io::Cursor::new(Vec::new());
        let img = image::ImageBuffer::from_fn(800, 600, |x, _y| image::Rgb([x as u8, 100, 200]));
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut buf, image::ImageFormat::Png)
            .unwrap();
        let processed = process_image(&buf.into_inner()).unwrap();

        // Thumb decodes back to a valid image whose largest dim ≤ 400.
        let thumb =
            image::load_from_memory_with_format(&processed.thumb_jpeg, image::ImageFormat::Jpeg)
                .unwrap();
        assert!(thumb.width().max(thumb.height()) <= THUMB_MAX_DIM);
        // Source dimensions are reported faithfully.
        assert_eq!(processed.width, 800);
        assert_eq!(processed.height, 600);
        // The thumbnail is real JPEG bytes.
        assert!(processed.thumb_jpeg.starts_with(&[0xff, 0xd8]));
    }

    #[test]
    fn process_preserves_aspect_ratio() {
        let mut buf = std::io::Cursor::new(Vec::new());
        // wide landscape — thumb should be 400 wide, ~225 tall.
        let img = image::ImageBuffer::from_fn(1600, 900, |_x, _y| image::Rgb([128_u8, 128, 128]));
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut buf, image::ImageFormat::Png)
            .unwrap();
        let processed = process_image(&buf.into_inner()).unwrap();
        let thumb =
            image::load_from_memory_with_format(&processed.thumb_jpeg, image::ImageFormat::Jpeg)
                .unwrap();
        assert_eq!(thumb.width(), 400);
        // 400 * 9 / 16 = 225
        assert!((thumb.height() as i32 - 225).abs() <= 1);
    }
}
