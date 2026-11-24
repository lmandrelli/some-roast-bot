//! Downloads the images of a Discord context so they can be sent to the
//! model in the same prompt as the transcript.

use std::{collections::HashMap, io::Cursor, time::Duration};

use base64::Engine as _;
use futures_util::{StreamExt as _, future::join_all};
use rig::message::{DocumentSourceKind, Image, ImageDetail, ImageMediaType};

use crate::bot::context::{ChannelContext, Visual, canonical_image_key, prioritize_visuals};

const FETCH_TIMEOUT: Duration = Duration::from_secs(5);
const IMAGE_BYTE_LIMIT: usize = 10 * 1024 * 1024;
const TOTAL_BYTE_LIMIT: usize = 20 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DetectedFormat {
    Jpeg,
    Png,
    Gif,
    WebP,
    Bmp,
}

impl DetectedFormat {
    fn detect(bytes: &[u8]) -> Option<Self> {
        match image::guess_format(bytes).ok()? {
            image::ImageFormat::Jpeg => Some(Self::Jpeg),
            image::ImageFormat::Png => Some(Self::Png),
            image::ImageFormat::Gif => Some(Self::Gif),
            image::ImageFormat::WebP => Some(Self::WebP),
            image::ImageFormat::Bmp => Some(Self::Bmp),
            _ => None,
        }
    }

    fn image_format(self) -> image::ImageFormat {
        match self {
            Self::Jpeg => image::ImageFormat::Jpeg,
            Self::Png => image::ImageFormat::Png,
            Self::Gif => image::ImageFormat::Gif,
            Self::WebP => image::ImageFormat::WebP,
            Self::Bmp => image::ImageFormat::Bmp,
        }
    }

    fn media_type(self) -> Option<ImageMediaType> {
        match self {
            Self::Jpeg => Some(ImageMediaType::JPEG),
            Self::Png => Some(ImageMediaType::PNG),
            Self::Gif => Some(ImageMediaType::GIF),
            Self::WebP => Some(ImageMediaType::WEBP),
            // rig-core 0.32 has no BMP media-type variant. BMP is normalized only at
            // this final adapter boundary so it can still be submitted safely.
            Self::Bmp => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Jpeg => "jpeg",
            Self::Png => "png",
            Self::Gif => "gif",
            Self::WebP => "webp",
            Self::Bmp => "bmp",
        }
    }
}

#[derive(Debug)]
struct DownloadedImage {
    bytes: Vec<u8>,
    format: DetectedFormat,
}

#[derive(Debug)]
struct PreparedImage {
    source_host: String,
    canonical_key: String,
    format: DetectedFormat,
    byte_size: usize,
    image: Image,
}

fn source_host(source_url: &str) -> String {
    url::Url::parse(source_url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned))
        .unwrap_or_else(|| "invalid-host".to_owned())
}

async fn download_image(
    client: &reqwest::Client,
    source_url: &str,
) -> Result<DownloadedImage, &'static str> {
    let fetch = async {
        let response = client.get(source_url).send().await.map_err(|_| "request")?;
        let response = response.error_for_status().map_err(|_| "http-status")?;
        if response
            .content_length()
            .is_some_and(|length| length > IMAGE_BYTE_LIMIT as u64)
        {
            return Err("size");
        }

        let mut bytes = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|_| "body")?;
            if bytes.len().saturating_add(chunk.len()) > IMAGE_BYTE_LIMIT {
                return Err("size");
            }
            bytes.extend_from_slice(&chunk);
        }
        if bytes.is_empty() {
            return Err("empty");
        }

        let format = DetectedFormat::detect(&bytes).ok_or("format")?;
        let mut reader =
            image::ImageReader::with_format(Cursor::new(&bytes), format.image_format());
        let mut limits = image::Limits::default();
        limits.max_alloc = Some(128 * 1024 * 1024);
        reader.limits(limits);
        reader.decode().map_err(|_| "decode")?;
        Ok(DownloadedImage { bytes, format })
    };

    tokio::time::timeout(FETCH_TIMEOUT, fetch)
        .await
        .map_err(|_| "timeout")?
}

fn build_payload(
    download: DownloadedImage,
) -> Result<(Image, DetectedFormat, usize), &'static str> {
    let original_size = download.bytes.len();
    let (bytes, media_type) = match download.format.media_type() {
        Some(media_type) => (download.bytes, media_type),
        None => {
            let decoded = image::load_from_memory_with_format(
                &download.bytes,
                download.format.image_format(),
            )
            .map_err(|_| "decode")?;
            let mut png = Cursor::new(Vec::new());
            decoded
                .write_to(&mut png, image::ImageFormat::Png)
                .map_err(|_| "encode")?;
            (png.into_inner(), ImageMediaType::PNG)
        }
    };
    Ok((
        Image {
            data: DocumentSourceKind::Base64(
                base64::engine::general_purpose::STANDARD.encode(bytes),
            ),
            media_type: Some(media_type),
            detail: Some(ImageDetail::Auto),
            additional_params: None,
        },
        download.format,
        original_size,
    ))
}

async fn prepare(client: &reqwest::Client, visuals: Vec<Visual>) -> Vec<(usize, PreparedImage)> {
    let downloads = join_all(
        visuals
            .into_iter()
            .enumerate()
            .map(|(index, visual)| async move {
                let host = source_host(&visual.url);
                let result = download_image(client, &visual.url).await;
                (index + 1, canonical_image_key(&visual.url), host, result)
            }),
    )
    .await;

    let mut accepted_bytes = 0usize;
    let mut prepared = Vec::new();
    for (position, canonical_key, host, result) in downloads {
        let download = match result {
            Ok(download) => download,
            Err(stage) => {
                tracing::warn!(image_position = position, source_host = %host, failure_stage = stage, "context image skipped");
                continue;
            }
        };
        let format = download.format;
        let byte_size = download.bytes.len();
        if accepted_bytes.saturating_add(byte_size) > TOTAL_BYTE_LIMIT {
            tracing::warn!(image_position = position, source_host = %host, detected_format = format.label(), byte_size, failure_stage = "aggregate-size", "context image skipped");
            continue;
        }
        let (image, format, byte_size) = match build_payload(download) {
            Ok(payload) => payload,
            Err(stage) => {
                tracing::warn!(image_position = position, source_host = %host, detected_format = format.label(), byte_size, failure_stage = stage, "context image skipped");
                continue;
            }
        };
        accepted_bytes += byte_size;
        prepared.push((
            position,
            PreparedImage {
                source_host: host,
                canonical_key,
                format,
                byte_size,
                image,
            },
        ));
    }
    prepared
}

/// Download the context images in priority order (trigger, reply target,
/// then most recent, at most `VISUAL_LIMIT`), keyed by canonical URL.
/// An image that fails a check is logged and left out of the prompt.
pub async fn load_context_images(context: &ChannelContext) -> HashMap<String, Image> {
    let visuals = prioritize_visuals(&context.messages);
    if visuals.is_empty() {
        return HashMap::new();
    }
    let client = reqwest::Client::new();
    prepare(&client, visuals)
        .await
        .into_iter()
        .map(|(position, prepared)| {
            tracing::debug!(image_position = position, source_host = %prepared.source_host, detected_format = prepared.format.label(), byte_size = prepared.byte_size, "context image attached");
            (prepared.canonical_key, prepared.image)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_supported_formats_from_magic_bytes() {
        let cases = [
            (&b"\xff\xd8\xff\xe0"[..], DetectedFormat::Jpeg),
            (&b"\x89PNG\r\n\x1a\n"[..], DetectedFormat::Png),
            (&b"GIF89a"[..], DetectedFormat::Gif),
            (&b"RIFF\x04\0\0\0WEBP"[..], DetectedFormat::WebP),
            (&b"BM\x1a\0\0\0"[..], DetectedFormat::Bmp),
        ];
        for (bytes, expected) in cases {
            assert_eq!(DetectedFormat::detect(bytes), Some(expected));
        }
        assert_eq!(DetectedFormat::detect(b"not an image"), None);
    }

    #[test]
    fn payload_is_explicit_base64_with_auto_detail() {
        let cases = [
            (DetectedFormat::Jpeg, ImageMediaType::JPEG),
            (DetectedFormat::Png, ImageMediaType::PNG),
            (DetectedFormat::Gif, ImageMediaType::GIF),
            (DetectedFormat::WebP, ImageMediaType::WEBP),
        ];
        for (format, expected_media_type) in cases {
            let (image, actual_format, size) = build_payload(DownloadedImage {
                bytes: vec![1, 2, 3],
                format,
            })
            .unwrap();
            assert_eq!(actual_format, format);
            assert_eq!(size, 3);
            assert_eq!(image.media_type, Some(expected_media_type));
            assert_eq!(image.detail, Some(ImageDetail::Auto));
            assert!(matches!(image.data, DocumentSourceKind::Base64(_)));
        }
    }

    #[test]
    fn bmp_payload_is_normalized_to_supported_png_metadata() {
        let source = image::DynamicImage::new_rgb8(1, 1);
        let mut bmp = Cursor::new(Vec::new());
        source.write_to(&mut bmp, image::ImageFormat::Bmp).unwrap();
        let bmp = bmp.into_inner();
        let original_size = bmp.len();

        let (image, format, size) = build_payload(DownloadedImage {
            bytes: bmp,
            format: DetectedFormat::Bmp,
        })
        .unwrap();
        assert_eq!(format, DetectedFormat::Bmp);
        assert_eq!(size, original_size);
        assert_eq!(image.media_type, Some(ImageMediaType::PNG));
        assert_eq!(image.detail, Some(ImageDetail::Auto));
        assert!(matches!(image.data, DocumentSourceKind::Base64(_)));
    }
}
