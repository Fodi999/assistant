//! Backend preprocessing for the prayer-mode WebGL particle visualizer.
//!
//! Previously the browser downloaded the raw source photo on every page load
//! and rebuilt the entire particle field client-side (luminance/edge sampling
//! over tens of thousands of candidate pixels, every time, on every device).
//! This module does that work exactly once per source image and produces
//! ready-to-use binary particle maps for three device tiers, uploaded to R2.
//!
//! "Edge enhancement of face/eyes/halo/clothing" is implemented as generic
//! Sobel-gradient contour emphasis, not semantic face/landmark detection —
//! there is no ML model in this pipeline. In practice contours (face outline,
//! eye sockets, the halo ring, clothing folds) are exactly the regions that
//! score highest on a gradient magnitude map, so this reads as "detail is
//! preserved, flat background is culled" without needing real recognition.

use std::collections::HashSet;
use std::io::Write;

use bytes::Bytes;
use flate2::{write::GzEncoder, Compression};
use image::{imageops::FilterType, GenericImageView};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::infrastructure::R2Client;

/// Bump whenever the sampling/encoding algorithm changes materially — forces
/// a distinct R2 key (via the filename) and lets us tell "never processed"
/// apart from "processed with an older algorithm" in the DB.
pub const PROCESSING_VERSION: i32 = 1;

const DESKTOP_TARGET_COUNT: usize = 72_000;
const MOBILE_TARGET_COUNT: usize = 18_000;
const LOW_POWER_TARGET_COUNT: usize = 8_000;

const BINARY_MAGIC: &[u8; 4] = b"PVM1";
const BINARY_FORMAT_VERSION: u16 = 1;

const GOLD: (f32, f32, f32) = (0.87, 0.68, 0.32);
const SILVER: (f32, f32, f32) = (0.82, 0.86, 0.92);
const WARM_WHITE: (f32, f32, f32) = (0.96, 0.88, 0.74);

/// Marks (or re-marks) a prayer's visualizer asset row as freshly queued for
/// processing. Called synchronously from the HTTP handler, before the actual
/// image work is spawned in the background, so a UI refresh right after
/// saving already shows "pending" instead of stale/missing data.
pub async fn mark_pending(
    pool: &PgPool,
    prayer_id: Uuid,
    source_image_url: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"INSERT INTO church_prayer_visualizer_assets (prayer_id, source_image_url, processing_status, processing_version, processing_error)
           VALUES ($1, $2, 'pending', $3, '')
           ON CONFLICT (prayer_id) DO UPDATE SET
               source_image_url = EXCLUDED.source_image_url,
               processing_status = 'pending',
               processing_version = EXCLUDED.processing_version,
               processing_error = ''"#,
    )
    .bind(prayer_id)
    .bind(source_image_url)
    .bind(PROCESSING_VERSION)
    .execute(pool)
    .await?;
    Ok(())
}

/// Runs the full pipeline (download → sample → 3 particle maps → fallback
/// WebP → thumbnail → upload → persist) and writes the final `ready`/`failed`
/// state to the DB. Fire-and-forget: callers `tokio::spawn` this, no HTTP
/// response depends on it.
pub async fn run_processing_job(
    pool: PgPool,
    r2: R2Client,
    prayer_id: Uuid,
    source_image_url: String,
    color_mode: String,
) {
    if let Err(err) = sqlx::query(
        "UPDATE church_prayer_visualizer_assets SET processing_status = 'processing' WHERE prayer_id = $1",
    )
    .bind(prayer_id)
    .execute(&pool)
    .await
    {
        tracing::error!(%err, %prayer_id, "prayer visualizer: failed to mark processing");
    }

    match process(&r2, prayer_id, &source_image_url, &color_mode).await {
        Ok(outputs) => {
            if let Err(err) = save_ready(&pool, prayer_id, &source_image_url, &outputs).await {
                tracing::error!(%err, %prayer_id, "prayer visualizer: failed to persist ready state");
            } else {
                tracing::info!(
                    %prayer_id,
                    desktop = outputs.desktop_particle_count,
                    mobile = outputs.mobile_particle_count,
                    low_power = outputs.low_power_particle_count,
                    "prayer visualizer: processing complete"
                );
            }
        }
        Err(message) => {
            tracing::warn!(%prayer_id, %message, "prayer visualizer: processing failed");
            if let Err(err) = save_failed(&pool, prayer_id, &message).await {
                tracing::error!(%err, %prayer_id, "prayer visualizer: failed to persist failure state");
            }
        }
    }
}

struct ProcessedOutputs {
    desktop_map_url: String,
    mobile_map_url: String,
    low_power_map_url: String,
    fallback_image_url: String,
    thumbnail_url: String,
    desktop_particle_count: i32,
    mobile_particle_count: i32,
    low_power_particle_count: i32,
}

#[derive(Clone, Copy)]
struct Candidate {
    x: u32,
    y: u32,
    luminance: f32,
    edge: f32,
}

async fn process(
    r2: &R2Client,
    prayer_id: Uuid,
    source_image_url: &str,
    color_mode: &str,
) -> Result<ProcessedOutputs, String> {
    let bytes = download(source_image_url).await?;
    let hash = content_hash(&bytes);

    let img = image::load_from_memory(&bytes).map_err(|e| format!("decode failed: {e}"))?;
    let (orig_w, orig_h) = img.dimensions();
    if orig_w == 0 || orig_h == 0 {
        return Err("source image has zero dimensions".to_string());
    }
    let aspect = orig_w as f32 / orig_h as f32;

    // Sample generously enough to keep facial/robe detail for the desktop
    // tier; lower tiers stride down from the same candidate pool.
    let sample_h: u32 = 640;
    let sample_w: u32 = ((sample_h as f32) * aspect).round().max(1.0) as u32;
    let sampled = img
        .resize_exact(sample_w, sample_h, FilterType::Lanczos3)
        .to_rgba8();

    let candidates = extract_candidates(&sampled, sample_w, sample_h);
    if candidates.is_empty() {
        return Err("no particle candidates found (image is fully flat or transparent)".to_string());
    }

    let prefix = format!("church/prayer-visualizer/{prayer_id}/v{PROCESSING_VERSION}-{hash}");

    let (desktop_particle_count, desktop_map_url) = process_tier(
        r2, &candidates, sample_w, sample_h, aspect, color_mode, DESKTOP_TARGET_COUNT, &prefix, "desktop",
    )
    .await?;
    let (mobile_particle_count, mobile_map_url) = process_tier(
        r2, &candidates, sample_w, sample_h, aspect, color_mode, MOBILE_TARGET_COUNT, &prefix, "mobile",
    )
    .await?;
    let (low_power_particle_count, low_power_map_url) = process_tier(
        r2, &candidates, sample_w, sample_h, aspect, color_mode, LOW_POWER_TARGET_COUNT, &prefix, "low-power",
    )
    .await?;

    // The pure-Rust WebP encoder (image-webp, no native libwebp) is
    // lossless-only, so it doesn't compress detailed photographs nearly as
    // well as a lossy encoder would — keep this tier's dimensions modest so
    // the "fallback" stays meaningfully lighter than the full source image.
    let fallback_bytes = encode_webp(&img, 640)?;
    let fallback_image_url = r2
        .upload_object(
            &format!("{prefix}-fallback.webp"),
            Bytes::from(fallback_bytes),
            "image/webp",
            None,
        )
        .await
        .map_err(|e| format!("R2 upload failed: {e:?}"))?;

    let thumb_bytes = encode_webp(&img, 240)?;
    let thumbnail_url = r2
        .upload_object(&format!("{prefix}-thumb.webp"), Bytes::from(thumb_bytes), "image/webp", None)
        .await
        .map_err(|e| format!("R2 upload failed: {e:?}"))?;

    Ok(ProcessedOutputs {
        desktop_map_url,
        mobile_map_url,
        low_power_map_url,
        fallback_image_url,
        thumbnail_url,
        desktop_particle_count,
        mobile_particle_count,
        low_power_particle_count,
    })
}

#[allow(clippy::too_many_arguments)]
async fn process_tier(
    r2: &R2Client,
    candidates: &[Candidate],
    sample_w: u32,
    sample_h: u32,
    aspect: f32,
    color_mode: &str,
    target_count: usize,
    prefix: &str,
    tier_name: &str,
) -> Result<(i32, String), String> {
    let picked = pick_tier(candidates, target_count);
    let map = build_particle_map(&picked, sample_w, sample_h, aspect, color_mode);
    let raw = encode_binary(&map);
    let compressed = gzip(&raw)?;
    let key = format!("{prefix}-{tier_name}.bin");
    let url = r2
        .upload_object(&key, Bytes::from(compressed), "application/octet-stream", Some("gzip"))
        .await
        .map_err(|e| format!("R2 upload failed: {e:?}"))?;
    Ok((map.count as i32, url))
}

/// Samples luminance + Sobel edge magnitude per pixel and keeps only pixels
/// that read as either a strong edge (contour) or a mid-tone, saturated
/// "figure" pixel — the same two-part test the frontend sampler used, just
/// computed once here with a proper gradient instead of a cheap 4-neighbour
/// diff. Everything else (near-transparent, near-black, flat low-contrast
/// background) is dropped — this is the "remove most background points" step.
fn extract_candidates(img: &image::RgbaImage, w: u32, h: u32) -> Vec<Candidate> {
    let mut luminance = vec![0f32; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            let p = img.get_pixel(x, y);
            let r = p[0] as f32 / 255.0;
            let g = p[1] as f32 / 255.0;
            let b = p[2] as f32 / 255.0;
            luminance[(y * w + x) as usize] = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        }
    }
    let lum_at = |x: i32, y: i32| -> f32 {
        let cx = x.clamp(0, w as i32 - 1) as u32;
        let cy = y.clamp(0, h as i32 - 1) as u32;
        luminance[(cy * w + cx) as usize]
    };

    let mut out = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let p = img.get_pixel(x, y);
            let r = p[0] as f32 / 255.0;
            let g = p[1] as f32 / 255.0;
            let b = p[2] as f32 / 255.0;
            let a = p[3] as f32 / 255.0;
            let lum = luminance[(y * w + x) as usize];
            if a < 0.15 || lum < 0.06 {
                continue;
            }

            let xi = x as i32;
            let yi = y as i32;
            let gx = -1.0 * lum_at(xi - 1, yi - 1) + 1.0 * lum_at(xi + 1, yi - 1)
                - 2.0 * lum_at(xi - 1, yi)
                + 2.0 * lum_at(xi + 1, yi)
                - 1.0 * lum_at(xi - 1, yi + 1)
                + 1.0 * lum_at(xi + 1, yi + 1);
            let gy = -1.0 * lum_at(xi - 1, yi - 1) - 2.0 * lum_at(xi, yi - 1) - 1.0 * lum_at(xi + 1, yi - 1)
                + 1.0 * lum_at(xi - 1, yi + 1)
                + 2.0 * lum_at(xi, yi + 1)
                + 1.0 * lum_at(xi + 1, yi + 1);
            let edge = (gx * gx + gy * gy).sqrt();

            let max_c = r.max(g).max(b);
            let min_c = r.min(g).min(b);
            let saturation = if max_c > 0.0 { (max_c - min_c) / max_c } else { 0.0 };

            let is_detail = edge > 0.16;
            let is_figure_tone = lum < 0.62 && saturation > 0.08;
            if !is_detail && !is_figure_tone {
                continue;
            }

            out.push(Candidate { x, y, luminance: lum, edge });
        }
    }
    out
}

/// Picks `target` points out of the full candidate pool for one device tier:
/// ~55% are the highest-edge (contour) pixels — this is the "enhance
/// face/eye/halo/clothing contours" step — and the rest are stride-sampled
/// uniformly across the remaining pool so the silhouette stays filled in,
/// not just its edges.
fn pick_tier(candidates: &[Candidate], target: usize) -> Vec<Candidate> {
    let total = candidates.len();
    if total <= target {
        return candidates.to_vec();
    }

    let edge_quota = ((target as f64) * 0.55).round() as usize;
    let mut by_edge: Vec<usize> = (0..total).collect();
    by_edge.sort_unstable_by(|&a, &b| {
        candidates[b]
            .edge
            .partial_cmp(&candidates[a].edge)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut chosen: Vec<usize> = by_edge.into_iter().take(edge_quota).collect();
    let chosen_set: HashSet<usize> = chosen.iter().copied().collect();

    let remaining: Vec<usize> = (0..total).filter(|i| !chosen_set.contains(i)).collect();
    let fill_quota = target.saturating_sub(chosen.len());
    if fill_quota > 0 && !remaining.is_empty() {
        let stride = (remaining.len() as f64 / fill_quota as f64).max(1.0);
        let mut pos = 0f64;
        while chosen.len() < target && (pos as usize) < remaining.len() {
            chosen.push(remaining[pos as usize]);
            pos += stride;
        }
    }

    chosen.sort_unstable();
    chosen.into_iter().map(|i| candidates[i]).collect()
}

struct ParticleMapBuffers {
    count: usize,
    width: f32,
    height: f32,
    targets: Vec<f32>,
    starts: Vec<f32>,
    colors: Vec<u8>,
    randoms: Vec<u8>,
}

fn build_particle_map(
    picked: &[Candidate],
    sample_w: u32,
    sample_h: u32,
    aspect: f32,
    color_mode: &str,
) -> ParticleMapBuffers {
    let count = picked.len();
    let target_height = 1.7f32;
    let target_width = target_height * aspect;

    let mut targets = vec![0f32; count * 3];
    let mut starts = vec![0f32; count * 3];
    let mut colors = vec![0u8; count * 3];
    let mut randoms = vec![0u8; count];

    let grid_cols = ((count as f32 * aspect).sqrt().ceil() as usize).max(1);
    let grid_rows = ((count as f32 / grid_cols as f32).ceil() as usize).max(1);

    for (i, c) in picked.iter().enumerate() {
        let nx = (c.x as f32 / sample_w as f32 - 0.5) * target_width;
        let ny = -(c.y as f32 / sample_h as f32 - 0.5) * target_height;
        // Weak z-depth: edge (contour) points sit marginally more forward so
        // the assembled face reads with a faint sense of relief instead of
        // being perfectly flat.
        let tz = (rand::random::<f32>() - 0.5) * 0.05 - c.edge.min(1.0) * 0.015;
        targets[i * 3] = nx;
        targets[i * 3 + 1] = ny;
        targets[i * 3 + 2] = tz;

        let rnd = rand::random::<f32>();
        let gx = i % grid_cols;
        let gy = i / grid_cols;
        let square_x = ((gx as f32 + 0.5) / grid_cols as f32 - 0.5) * target_width * 1.08;
        let square_y = -(((gy as f32 + 0.5) / grid_rows as f32 - 0.5) * target_height * 1.08);
        starts[i * 3] = square_x + (rand::random::<f32>() - 0.5) * 0.012;
        starts[i * 3 + 1] = square_y + (rand::random::<f32>() - 0.5) * 0.012;
        starts[i * 3 + 2] = (rand::random::<f32>() - 0.5) * 0.18;

        let (cr, cg, cb) = color_for(color_mode, c.luminance, rnd);
        colors[i * 3] = (cr.clamp(0.0, 1.0) * 255.0).round() as u8;
        colors[i * 3 + 1] = (cg.clamp(0.0, 1.0) * 255.0).round() as u8;
        colors[i * 3 + 2] = (cb.clamp(0.0, 1.0) * 255.0).round() as u8;
        randoms[i] = (rnd.clamp(0.0, 1.0) * 255.0).round() as u8;
    }

    ParticleMapBuffers { count, width: target_width, height: target_height, targets, starts, colors, randoms }
}

fn color_for(mode: &str, luminance: f32, rand: f32) -> (f32, f32, f32) {
    let base = match mode {
        "gold" => GOLD,
        "silver" => SILVER,
        "warm_white" => WARM_WHITE,
        _ => {
            if rand > 0.45 {
                GOLD
            } else {
                SILVER
            }
        }
    };
    let boost = 0.28 + luminance.min(0.72) * 0.42;
    (base.0 * boost, base.1 * boost, base.2 * boost)
}

/// Compact versioned binary layout — no JSON for tens of thousands of points.
/// All multi-byte fields little-endian. f32 blocks come first so each one
/// starts at a 4-byte-aligned offset (required to later hand the browser a
/// zero-copy `Float32Array` view straight over the decompressed buffer); the
/// two byte-per-element blocks (no alignment constraint) come last.
///
/// ```text
/// offset  0: magic "PVM1"            (4 bytes)
/// offset  4: format version (u16 LE) (2 bytes)
/// offset  6: reserved                (2 bytes)
/// offset  8: particle count (u32 LE) (4 bytes)
/// offset 12: width  (f32 LE)         (4 bytes)
/// offset 16: height (f32 LE)         (4 bytes)
/// offset 20: targets  f32[count * 3] (count * 12 bytes)
/// offset  +: starts   f32[count * 3] (count * 12 bytes)
/// offset  +: colors   u8[count * 3]  (count * 3 bytes,  0..255 per channel)
/// offset  +: randoms  u8[count]      (count bytes,      0..255)
/// ```
fn encode_binary(map: &ParticleMapBuffers) -> Vec<u8> {
    let mut buf = Vec::with_capacity(20 + map.targets.len() * 4 + map.starts.len() * 4 + map.colors.len() + map.randoms.len());
    buf.extend_from_slice(BINARY_MAGIC);
    buf.extend_from_slice(&BINARY_FORMAT_VERSION.to_le_bytes());
    buf.extend_from_slice(&0u16.to_le_bytes());
    buf.extend_from_slice(&(map.count as u32).to_le_bytes());
    buf.extend_from_slice(&map.width.to_le_bytes());
    buf.extend_from_slice(&map.height.to_le_bytes());
    for v in &map.targets {
        buf.extend_from_slice(&v.to_le_bytes());
    }
    for v in &map.starts {
        buf.extend_from_slice(&v.to_le_bytes());
    }
    buf.extend_from_slice(&map.colors);
    buf.extend_from_slice(&map.randoms);
    buf
}

fn gzip(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(bytes).map_err(|e| format!("gzip write failed: {e}"))?;
    encoder.finish().map_err(|e| format!("gzip finish failed: {e}"))
}

fn encode_webp(img: &image::DynamicImage, max_width: u32) -> Result<Vec<u8>, String> {
    let (w, h) = img.dimensions();
    let scale = (max_width as f32 / w as f32).min(1.0);
    let out_w = ((w as f32) * scale).round().max(1.0) as u32;
    let out_h = ((h as f32) * scale).round().max(1.0) as u32;
    let resized = img.resize(out_w, out_h, FilterType::Lanczos3);
    let mut buf = std::io::Cursor::new(Vec::new());
    resized
        .write_to(&mut buf, image::ImageFormat::WebP)
        .map_err(|e| format!("webp encode failed: {e}"))?;
    Ok(buf.into_inner())
}

async fn download(url: &str) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("http client build failed: {e}"))?;
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("download failed: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("download failed: HTTP {}", response.status()));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("download body failed: {e}"))?;
    Ok(bytes.to_vec())
}

fn content_hash(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    hex::encode(&digest[..5])
}

async fn save_ready(
    pool: &PgPool,
    prayer_id: Uuid,
    source_image_url: &str,
    outputs: &ProcessedOutputs,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"UPDATE church_prayer_visualizer_assets SET
               source_image_url = $2,
               desktop_map_url = $3, mobile_map_url = $4, low_power_map_url = $5,
               fallback_image_url = $6, thumbnail_url = $7,
               desktop_particle_count = $8, mobile_particle_count = $9, low_power_particle_count = $10,
               processing_status = 'ready', processing_error = '', processing_version = $11
           WHERE prayer_id = $1"#,
    )
    .bind(prayer_id)
    .bind(source_image_url)
    .bind(&outputs.desktop_map_url)
    .bind(&outputs.mobile_map_url)
    .bind(&outputs.low_power_map_url)
    .bind(&outputs.fallback_image_url)
    .bind(&outputs.thumbnail_url)
    .bind(outputs.desktop_particle_count)
    .bind(outputs.mobile_particle_count)
    .bind(outputs.low_power_particle_count)
    .bind(PROCESSING_VERSION)
    .execute(pool)
    .await?;
    Ok(())
}

async fn save_failed(pool: &PgPool, prayer_id: Uuid, message: &str) -> Result<(), sqlx::Error> {
    let truncated: String = message.chars().take(500).collect();
    sqlx::query(
        "UPDATE church_prayer_visualizer_assets SET processing_status = 'failed', processing_error = $2 WHERE prayer_id = $1",
    )
    .bind(prayer_id)
    .bind(truncated)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_image(w: u32, h: u32) -> image::DynamicImage {
        // A bright disc (the "face") on a flat dark background, so both the
        // is_figure_tone and is_detail (edge) candidate paths get exercised
        // and most of the flat background gets culled.
        let mut img = image::RgbaImage::new(w, h);
        let cx = w as f32 / 2.0;
        let cy = h as f32 / 2.0;
        let r = (w.min(h) as f32) * 0.35;
        for y in 0..h {
            for x in 0..w {
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                let inside = (dx * dx + dy * dy).sqrt() < r;
                let px = if inside {
                    image::Rgba([220, 190, 150, 255])
                } else {
                    image::Rgba([8, 8, 10, 255])
                };
                img.put_pixel(x, y, px);
            }
        }
        image::DynamicImage::ImageRgba8(img)
    }

    #[test]
    fn extract_candidates_culls_flat_background() {
        let img = synthetic_image(160, 160).to_rgba8();
        let candidates = extract_candidates(&img, 160, 160);
        assert!(!candidates.is_empty());
        // A 160x160 image with a r=56 disc has a disc area of ~9852px out of
        // 25600 total; background is flat (no edge, no saturation) so it
        // should be almost entirely culled, leaving well under half the pixels.
        assert!(candidates.len() < (160 * 160) / 2);
    }

    #[test]
    fn pick_tier_respects_target_and_pool_size() {
        let candidates: Vec<Candidate> = (0..1000)
            .map(|i| Candidate { x: i % 40, y: i / 40, luminance: 0.5, edge: (i % 10) as f32 / 10.0 })
            .collect();

        let picked = pick_tier(&candidates, 200);
        assert_eq!(picked.len(), 200);

        // Requesting more than the pool has just returns the whole pool.
        let picked_all = pick_tier(&candidates, 5000);
        assert_eq!(picked_all.len(), candidates.len());
    }

    #[test]
    fn binary_roundtrip_header_and_payload_layout() {
        let picked = vec![
            Candidate { x: 10, y: 20, luminance: 0.4, edge: 0.9 },
            Candidate { x: 30, y: 5, luminance: 0.7, edge: 0.1 },
            Candidate { x: 0, y: 0, luminance: 0.05, edge: 0.0 },
        ];
        let map = build_particle_map(&picked, 100, 100, 1.0, "gold");
        assert_eq!(map.count, 3);
        let raw = encode_binary(&map);

        let expected_len = 20 + map.count * 3 * 4 + map.count * 3 * 4 + map.count * 3 + map.count;
        assert_eq!(raw.len(), expected_len);

        assert_eq!(&raw[0..4], b"PVM1");
        let version = u16::from_le_bytes([raw[4], raw[5]]);
        assert_eq!(version, BINARY_FORMAT_VERSION);
        let count = u32::from_le_bytes([raw[8], raw[9], raw[10], raw[11]]);
        assert_eq!(count, 3);
        let width = f32::from_le_bytes([raw[12], raw[13], raw[14], raw[15]]);
        let height = f32::from_le_bytes([raw[16], raw[17], raw[18], raw[19]]);
        assert_eq!(width, map.width);
        assert_eq!(height, map.height);

        // First target.x (offset 20) matches what build_particle_map computed.
        let first_target_x = f32::from_le_bytes([raw[20], raw[21], raw[22], raw[23]]);
        assert_eq!(first_target_x, map.targets[0]);

        // Colors block starts right after both f32 blocks (4-byte aligned by
        // construction — count*12 is always a multiple of 4).
        let colors_offset = 20 + map.count * 3 * 4 + map.count * 3 * 4;
        assert_eq!(raw[colors_offset], map.colors[0]);

        // Randoms block is the final `count` bytes.
        let randoms_offset = colors_offset + map.count * 3;
        assert_eq!(raw.len() - randoms_offset, map.count);
        assert_eq!(raw[randoms_offset], map.randoms[0]);

        // Gzip round-trips back to the exact same bytes.
        let compressed = gzip(&raw).expect("gzip should succeed");
        let mut decoder = flate2::read::GzDecoder::new(&compressed[..]);
        let mut decompressed = Vec::new();
        std::io::Read::read_to_end(&mut decoder, &mut decompressed).expect("gunzip should succeed");
        assert_eq!(decompressed, raw);
    }

    /// Writes a small, known-good gzip-compressed map to disk so the
    /// frontend's `parseParticleMap` can be tested against real bytes
    /// produced by this exact encoder (cross-language contract check).
    #[test]
    fn write_fixture_for_frontend_decoder_test() {
        let picked = vec![
            Candidate { x: 10, y: 20, luminance: 0.4, edge: 0.9 },
            Candidate { x: 30, y: 5, luminance: 0.7, edge: 0.1 },
            Candidate { x: 50, y: 50, luminance: 0.6, edge: 0.3 },
            Candidate { x: 90, y: 80, luminance: 0.2, edge: 0.05 },
        ];
        let map = build_particle_map(&picked, 100, 100, 1.4, "silver");
        let raw = encode_binary(&map);
        let compressed = gzip(&raw).expect("gzip should succeed");

        let dir = std::env::var("PRAYER_VISUALIZER_FIXTURE_DIR").unwrap_or_else(|_| "/tmp".to_string());
        let raw_path = format!("{dir}/prayer_visualizer_fixture.bin");
        let gz_path = format!("{dir}/prayer_visualizer_fixture.bin.gz");
        std::fs::write(&raw_path, &raw).expect("write raw fixture");
        std::fs::write(&gz_path, &compressed).expect("write gzip fixture");
        println!("wrote fixtures to {raw_path} and {gz_path} (count={})", map.count);
    }
}
