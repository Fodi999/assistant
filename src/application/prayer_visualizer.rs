//! Backend preprocessing for the prayer-mode WebGL particle visualizer.
//!
//! Previously the browser downloaded the raw source photo on every page load
//! and rebuilt the entire particle field client-side (luminance/edge sampling
//! over tens of thousands of candidate pixels, every time, on every device).
//! This module does that work exactly once per source image and produces
//! ready-to-use binary particle maps for three device tiers, uploaded to R2.
//!
//! "Face/eyes/hands/halo/silhouette/garment contour" prioritization is
//! implemented as generic Sobel-gradient contour emphasis + local-contrast
//! (texture) detection + a soft upper-center compositional prior (most icon
//! portraits frame the face there) — not semantic face/hand landmark
//! detection. There is no ML model in this pipeline. In practice this reads
//! as "detail and the subject region are preserved, flat background is
//! culled" without needing real recognition.
//!
//! Every particle also carries baked alpha, size, a coarse z-depth band, and
//! a reveal-order value (silhouette → face-region → fine detail → color
//! fill → highlights) — all computed once here from the same per-pixel
//! signals, so the frontend never re-derives brightness, position, or
//! timing; it only renders exactly what this module produced.
//!
//! Nothing about the output is admin-configured anymore: particle count per
//! tier, the color tint, exposure, and the shadow lift are all derived from
//! the source image itself (see `ImageStats`) — a flat, low-detail icon gets
//! fewer, larger particles and a gentler grade than a highly detailed one,
//! automatically. `particle_size`/`particle_color_mode`/`particle_count_*`
//! on `church_prayers` are legacy columns the pipeline no longer reads.

use std::cmp::Ordering;
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
pub const PROCESSING_VERSION: i32 = 4;

// Particle-count bounds per tier — the actual target within each range is
// chosen automatically from the image's own detail score (see
// `ImageStats::detail_score`), not from an admin setting.
const DESKTOP_MIN_COUNT: usize = 42_000;
const DESKTOP_MAX_COUNT: usize = 64_000;
const MOBILE_MIN_COUNT: usize = 11_000;
const MOBILE_MAX_COUNT: usize = 18_000;
const LOW_POWER_MIN_COUNT: usize = 5_000;
const LOW_POWER_MAX_COUNT: usize = 8_000;

const BINARY_MAGIC: &[u8; 4] = b"PVM1";
/// v3 adds two baked-once-per-map header fields (auto base point size, auto
/// exposure) after height; see `encode_binary` for the exact layout.
const BINARY_FORMAT_VERSION: u16 = 3;

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

    match process(&r2, prayer_id, &source_image_url).await {
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
    r: f32,
    g: f32,
    b: f32,
    /// Combined sampling priority: how strongly this pixel should be kept
    /// and how densely packed its neighborhood should be.
    importance: f32,
    /// 0..1 — when this particle should finish assembling, relative to the
    /// others: silhouette/contours first, then the face-prior region, then
    /// fine detail, then color fill, then bright/saturated highlights last.
    /// Also identifies the highlight band (reveal > HIGHLIGHT_REVEAL_FLOOR
    /// is only ever produced by the highlight branch below) — no other
    /// branch reaches that range, so it doubles as a highlight flag without
    /// needing a separate stored field.
    reveal: f32,
}

/// Only the highlight reveal-order branch in `extract_candidates` produces
/// values at or above this floor; every other branch caps below it.
const HIGHLIGHT_REVEAL_FLOOR: f32 = 0.85;

async fn process(
    r2: &R2Client,
    prayer_id: Uuid,
    source_image_url: &str,
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

    // Everything below is derived from the image itself — no admin input.
    let stats = ImageStats::compute(&candidates, sample_w, sample_h);
    let color_mode = stats.auto_color_mode();
    let shadow_lift = stats.auto_shadow_lift();
    let auto_exposure = stats.auto_exposure();
    let detail_score = stats.detail_score;
    tracing::info!(
        %prayer_id,
        detail_score,
        mean_luminance = stats.mean_luminance,
        color_mode,
        shadow_lift,
        auto_exposure,
        "prayer visualizer: auto-derived parameters"
    );

    let desktop_target = scale_count(DESKTOP_MIN_COUNT, DESKTOP_MAX_COUNT, detail_score);
    let mobile_target = scale_count(MOBILE_MIN_COUNT, MOBILE_MAX_COUNT, detail_score);
    let low_power_target = scale_count(LOW_POWER_MIN_COUNT, LOW_POWER_MAX_COUNT, detail_score);

    let prefix = format!("church/prayer-visualizer/{prayer_id}/v{PROCESSING_VERSION}-{hash}");

    let (desktop_particle_count, desktop_map_url) = process_tier(
        r2, &candidates, sample_w, sample_h, aspect, color_mode, shadow_lift, auto_exposure, desktop_target, &prefix, "desktop",
    )
    .await?;
    let (mobile_particle_count, mobile_map_url) = process_tier(
        r2, &candidates, sample_w, sample_h, aspect, color_mode, shadow_lift, auto_exposure, mobile_target, &prefix, "mobile",
    )
    .await?;
    let (low_power_particle_count, low_power_map_url) = process_tier(
        r2, &candidates, sample_w, sample_h, aspect, color_mode, shadow_lift, auto_exposure, low_power_target, &prefix, "low-power",
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
    shadow_lift: f32,
    auto_exposure: f32,
    target_count: usize,
    prefix: &str,
    tier_name: &str,
) -> Result<(i32, String), String> {
    let picked = pick_tier(candidates, target_count, sample_w, sample_h);
    let map = build_particle_map(&picked, sample_w, sample_h, aspect, color_mode, shadow_lift, auto_exposure);
    let raw = encode_binary(&map);
    let compressed = gzip(&raw)?;
    let key = format!("{prefix}-{tier_name}.bin");
    let url = r2
        .upload_object(&key, Bytes::from(compressed), "application/octet-stream", Some("gzip"))
        .await
        .map_err(|e| format!("R2 upload failed: {e:?}"))?;
    Ok((map.count as i32, url))
}

/// Samples real color + luminance + Sobel edge + local contrast per pixel,
/// derives a compositional face-region prior and a highlight score from
/// those, combines them into one `importance` sampling weight and a
/// `reveal` order, and keeps only pixels that read as a contour, textured
/// detail, or a mid-tone saturated "figure" pixel — everything else (near-
/// transparent, near-black, flat low-contrast background) is dropped.
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
            let edge_norm = (edge / 2.5).min(1.0);

            // Local contrast: stddev of luminance in the 3x3 neighborhood —
            // catches fine texture (eyes, hands, decorative detail) that a
            // pure gradient can miss when the transition is soft.
            let mut sum = 0f32;
            let mut sum_sq = 0f32;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let l = lum_at(xi + dx, yi + dy);
                    sum += l;
                    sum_sq += l * l;
                }
            }
            let mean = sum / 9.0;
            let variance = (sum_sq / 9.0 - mean * mean).max(0.0);
            let contrast_norm = (variance.sqrt() / 0.28).min(1.0);

            let max_c = r.max(g).max(b);
            let min_c = r.min(g).min(b);
            let saturation = if max_c > 0.0 { (max_c - min_c) / max_c } else { 0.0 };

            let is_detail = edge > 0.16;
            let is_figure_tone = lum < 0.62 && saturation > 0.08;
            if !is_detail && !is_figure_tone {
                continue;
            }

            // Soft compositional prior: most icon portraits frame the face
            // in the upper-middle third. This is a prior over *composition*,
            // not a detected landmark — it nudges importance/reveal-order,
            // it never gates inclusion.
            let nx = x as f32 / w as f32;
            let ny = y as f32 / h as f32;
            let fdx = nx - 0.5;
            let fdy = ny - 0.32;
            let face_prior = (-(fdx * fdx + fdy * fdy) / (2.0 * 0.12 * 0.12)).exp();

            // Bright + moderately-to-highly saturated pixels read as
            // catchlights/gilding/jewelry — the "highlights" band.
            let highlight_score = ((lum - 0.6).max(0.0) / 0.4) * (0.35 + saturation.min(0.65));

            let importance = (0.42 * edge_norm
                + 0.22 * contrast_norm
                + 0.18 * face_prior.min(1.0)
                + 0.18 * highlight_score.min(1.0))
            .clamp(0.05, 1.0);

            let reveal = if highlight_score > 0.35 {
                0.86 + 0.14 * highlight_score.min(1.0)
            } else if edge_norm > 0.45 {
                0.04 + 0.16 * edge_norm
            } else if face_prior > 0.5 {
                0.26 + 0.14 * face_prior.min(1.0)
            } else if contrast_norm > 0.35 {
                0.46 + 0.14 * contrast_norm
            } else {
                0.66 + 0.14 * (1.0 - lum).clamp(0.0, 1.0)
            };

            out.push(Candidate {
                x,
                y,
                r,
                g,
                b,
                importance,
                reveal: reveal.clamp(0.0, 1.0),
            });
        }
    }
    out
}

/// Depth banding is computed against a specific *tier's already-picked*
/// particles (not the full candidate pool) — the blue-noise selection in
/// `pick_tier` inherently favors higher-importance points (they pack denser
/// and so survive selection more often), which skews the survivors' own
/// importance distribution well above the full pool's. Using this tier-local
/// percentile is what actually gets ~20% of its non-highlight particles into
/// the background band, rather than ~0%.
fn depth_band_for(picked: &[Candidate]) -> Vec<i8> {
    let mut non_highlight_importance: Vec<f32> = picked
        .iter()
        .filter(|c| c.reveal < HIGHLIGHT_REVEAL_FLOOR)
        .map(|c| c.importance)
        .collect();
    non_highlight_importance.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let bg_threshold = if non_highlight_importance.is_empty() {
        0.0
    } else {
        let idx = (((non_highlight_importance.len() as f32) * 0.20) as usize).min(non_highlight_importance.len() - 1);
        non_highlight_importance[idx]
    };

    picked
        .iter()
        .map(|c| {
            if c.reveal >= HIGHLIGHT_REVEAL_FLOOR {
                1
            } else if c.importance < bg_threshold {
                -1
            } else {
                0
            }
        })
        .collect()
}

/// Importance-weighted blue-noise (dart-throwing) selection: candidates are
/// processed in descending importance order and accepted only if they land
/// farther than a minimum radius from every already-accepted neighbor, where
/// that radius shrinks with importance (dense packing on contours/face/
/// highlights) and grows on low-importance background fill. This avoids the
/// row/grid artifacts a uniform stride or raster-order pick produces, and
/// naturally thins background fill without a second, separate "background"
/// pass. Falls back to returning the whole pool if it's already <= target.
fn pick_tier(candidates: &[Candidate], target: usize, sample_w: u32, sample_h: u32) -> Vec<Candidate> {
    let total = candidates.len();
    if total <= target {
        return candidates.to_vec();
    }

    let mut order: Vec<usize> = (0..total).collect();
    order.sort_unstable_by(|&a, &b| {
        candidates[b]
            .importance
            .partial_cmp(&candidates[a].importance)
            .unwrap_or(Ordering::Equal)
    });

    let area = (sample_w as f32) * (sample_h as f32);
    // Average disk radius for `target` points at ~70% hex-packing efficiency.
    let base_radius = (area / (target as f32) / 1.4).sqrt().max(0.6);

    let mut best: Vec<usize> = Vec::new();
    let mut scale = 1.0f32;
    for _attempt in 0..5 {
        // Narrower spread than an earlier version (was 1.85..0.55): a wide
        // spread over-thinned merely-flat *subject* areas (a robe's mid-fold
        // fill has little edge/contrast but is still the figure, not photo
        // background) into a near-black void. Contours/face/highlights
        // still pack visibly denser than flat fill, just not as extremely.
        let radius_for = |importance: f32| -> f32 {
            (base_radius * scale) * (1.4 - 0.85 * importance.clamp(0.0, 1.0))
        };
        let cell_size = (base_radius * scale * 0.6).max(1.0);
        let grid_w = ((sample_w as f32 / cell_size).ceil() as i32 + 1).max(1);
        let grid_h = ((sample_h as f32 / cell_size).ceil() as i32 + 1).max(1);
        let mut grid: Vec<Vec<usize>> = vec![Vec::new(); (grid_w * grid_h) as usize];
        let cell_of = |x: f32, y: f32| -> (i32, i32) { ((x / cell_size) as i32, (y / cell_size) as i32) };

        let mut accepted: Vec<usize> = Vec::with_capacity(target.min(total));
        for &idx in &order {
            let c = &candidates[idx];
            let r = radius_for(c.importance);
            let (cxg, cyg) = cell_of(c.x as f32, c.y as f32);
            let mut ok = true;
            'search: for gy in (cyg - 1).max(0)..=(cyg + 1).min(grid_h - 1) {
                for gx in (cxg - 1).max(0)..=(cxg + 1).min(grid_w - 1) {
                    for &other_idx in &grid[(gy * grid_w + gx) as usize] {
                        let o = &candidates[other_idx];
                        let dx = o.x as f32 - c.x as f32;
                        let dy = o.y as f32 - c.y as f32;
                        let min_r = r.min(radius_for(o.importance));
                        if dx * dx + dy * dy < min_r * min_r {
                            ok = false;
                            break 'search;
                        }
                    }
                }
            }
            if ok {
                grid[(cyg * grid_w + cxg) as usize].push(idx);
                accepted.push(idx);
                if accepted.len() >= target {
                    break;
                }
            }
        }

        if accepted.len() > best.len() {
            best = accepted;
        }
        if best.len() >= target {
            break;
        }
        // Too few accepted — shrink the radius (denser packing) and retry.
        scale *= 0.7;
    }

    best.truncate(target);
    best.sort_unstable();
    best.into_iter().map(|i| candidates[i]).collect()
}

struct ParticleMapBuffers {
    count: usize,
    width: f32,
    height: f32,
    /// Auto-derived from this tier's own particle count (denser tiers get
    /// smaller points, sparser tiers get larger ones) — replaces the old
    /// admin `particle_size` field as the renderer's base point size.
    base_point_size: f32,
    /// Auto-derived from the image's overall luminance — replaces the old
    /// hardcoded frontend exposure constant.
    auto_exposure: f32,
    targets: Vec<f32>,
    starts: Vec<f32>,
    colors: Vec<u8>,
    alphas: Vec<u8>,
    sizes: Vec<u8>,
    reveal: Vec<u8>,
    randoms: Vec<u8>,
}

fn build_particle_map(
    picked: &[Candidate],
    sample_w: u32,
    sample_h: u32,
    aspect: f32,
    color_mode: &str,
    shadow_lift: f32,
    auto_exposure: f32,
) -> ParticleMapBuffers {
    let count = picked.len();
    let target_height = 1.7f32;
    let target_width = target_height * aspect;

    let mut targets = vec![0f32; count * 3];
    let mut starts = vec![0f32; count * 3];
    let mut colors = vec![0u8; count * 3];
    let mut alphas = vec![0u8; count];
    let mut sizes = vec![0u8; count];
    let mut reveal = vec![0u8; count];
    let mut randoms = vec![0u8; count];

    let depth_bands = depth_band_for(picked);

    for (i, c) in picked.iter().enumerate() {
        let nx = (c.x as f32 / sample_w as f32 - 0.5) * target_width;
        let ny = -(c.y as f32 / sample_h as f32 - 0.5) * target_height;

        // Structured, subtle z-depth: background sits slightly behind the
        // main plane, the subject sits on it, highlights sit slightly in
        // front. Kept small on purpose — with an orthographic camera this
        // reads as gentle volumetric relief via depth-sorted blending and
        // (below) depth-correlated size/alpha, not as perspective parallax.
        let jitter = (rand::random::<f32>() - 0.5) * 0.02;
        let tz = match depth_bands[i] {
            1 => 0.05 + 0.05 * jitter.abs() * 4.0,
            -1 => -0.11 - 0.05 * jitter.abs() * 4.0,
            _ => jitter,
        };
        targets[i * 3] = nx;
        targets[i * 3 + 1] = ny;
        targets[i * 3 + 2] = tz;

        // Organic scattered dust cloud instead of a square grid: uniform
        // over an ellipse matching the image's aspect, so the pre-assembly
        // state never reads as rows/columns.
        let theta = rand::random::<f32>() * std::f32::consts::TAU;
        let radius_frac = rand::random::<f32>().sqrt();
        let sx = radius_frac * target_width * 0.62 * theta.cos();
        let sy = radius_frac * target_height * 0.62 * theta.sin();
        starts[i * 3] = sx;
        starts[i * 3 + 1] = sy;
        starts[i * 3 + 2] = (rand::random::<f32>() - 0.5) * 0.22;

        let (cr, cg, cb) = grade_color(color_mode, shadow_lift, c.r, c.g, c.b);
        colors[i * 3] = (cr.clamp(0.0, 1.0) * 255.0).round() as u8;
        colors[i * 3 + 1] = (cg.clamp(0.0, 1.0) * 255.0).round() as u8;
        colors[i * 3 + 2] = (cb.clamp(0.0, 1.0) * 255.0).round() as u8;

        // Baked per-particle alpha/size: higher importance (contours, face
        // region, highlights) reads as more opaque and larger; flat-fill
        // background dust stays smaller and dimmer. This is the "readable
        // alpha" + "adaptive size" the client no longer computes itself.
        let alpha_base = (0.70 + 0.30 * c.importance).clamp(0.0, 1.0);
        let size_mult = (0.78 + 0.65 * c.importance).clamp(0.0, 1.5);
        alphas[i] = (alpha_base * 255.0).round() as u8;
        sizes[i] = ((size_mult / 1.5) * 255.0).round().clamp(0.0, 255.0) as u8;
        reveal[i] = (c.reveal.clamp(0.0, 1.0) * 255.0).round() as u8;
        randoms[i] = (rand::random::<f32>().clamp(0.0, 1.0) * 255.0).round() as u8;
    }

    ParticleMapBuffers {
        count,
        width: target_width,
        height: target_height,
        base_point_size: auto_base_point_size(count),
        auto_exposure,
        targets,
        starts,
        colors,
        alphas,
        sizes,
        reveal,
        randoms,
    }
}

/// Grades the *real* sampled pixel color for the auto-detected aesthetic
/// mode (see `ImageStats::auto_color_mode`) — the source photo's actual
/// hue/detail is preserved; the mode only applies a gentle tint, and `lift`
/// (also auto-derived, from `ImageStats::auto_shadow_lift`) brightens
/// shadows so real dark pixels — hair, deep robe folds — don't vanish once
/// alpha-blended against the black canvas.
fn grade_color(mode: &str, lift: f32, r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let (tr, tg, tb): (f32, f32, f32) = match mode {
        "gold" => (1.15, 1.0, 0.80),
        "silver" => (0.98, 1.02, 1.08),
        "warm_white" => (1.05, 1.02, 0.95),
        _ => (1.06, 1.0, 0.88),
    };
    let lr = r * (1.0 - lift) + lift;
    let lg = g * (1.0 - lift) + lift;
    let lb = b * (1.0 - lift) + lift;
    (lr * tr, lg * tg, lb * tb)
}

/// Aggregate signals over a candidate pool, used to auto-derive everything
/// that used to be an admin setting: color tint, shadow lift, exposure, and
/// (via `detail_score`) the particle-count target for each tier.
struct ImageStats {
    /// Mean luminance across candidates, 0..1.
    mean_luminance: f32,
    /// How "busy" the image is: blends candidate density (fraction of the
    /// sample that qualified as a particle) with mean per-candidate
    /// importance. Drives particle-count scaling — a flat, low-detail icon
    /// gets fewer (larger, more opaque) particles automatically.
    detail_score: f32,
    /// Importance-weighted average color, used to pick the tint.
    avg_r: f32,
    avg_g: f32,
    avg_b: f32,
}

impl ImageStats {
    fn compute(candidates: &[Candidate], sample_w: u32, sample_h: u32) -> Self {
        if candidates.is_empty() {
            return Self { mean_luminance: 0.5, detail_score: 0.3, avg_r: 0.5, avg_g: 0.5, avg_b: 0.5 };
        }

        let mut lum_sum = 0f32;
        let mut importance_sum = 0f32;
        let (mut wr, mut wg, mut wb, mut wsum) = (0f32, 0f32, 0f32, 0f32);
        for c in candidates {
            let lum = 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b;
            lum_sum += lum;
            importance_sum += c.importance;
            let w = c.importance.max(0.05);
            wr += c.r * w;
            wg += c.g * w;
            wb += c.b * w;
            wsum += w;
        }
        let n = candidates.len() as f32;
        let mean_luminance = (lum_sum / n).clamp(0.0, 1.0);
        let mean_importance = (importance_sum / n).clamp(0.0, 1.0);

        let total_pixels = (sample_w as f32 * sample_h as f32).max(1.0);
        let density = (n / total_pixels).clamp(0.0, 1.0);
        // 60% very detailed/dense image already saturates the score — an
        // icon with a large, busy figure rarely exceeds that in practice.
        let density_norm = (density / 0.6).min(1.0);
        let detail_score = (0.55 * density_norm + 0.45 * mean_importance).clamp(0.0, 1.0);

        let (avg_r, avg_g, avg_b) = if wsum > 0.0 { (wr / wsum, wg / wsum, wb / wsum) } else { (0.5, 0.5, 0.5) };

        Self { mean_luminance, detail_score, avg_r, avg_g, avg_b }
    }

    /// Warm/cool/neutral tint choice from the image's own importance-
    /// weighted average color — no admin dropdown.
    fn auto_color_mode(&self) -> &'static str {
        let max_c = self.avg_r.max(self.avg_g).max(self.avg_b);
        let min_c = self.avg_r.min(self.avg_g).min(self.avg_b);
        let saturation = if max_c > 0.0 { (max_c - min_c) / max_c } else { 0.0 };
        let warmth = self.avg_r - self.avg_b;

        if saturation < 0.12 {
            "silver"
        } else if warmth > 0.08 {
            "gold"
        } else if warmth < -0.05 {
            "silver"
        } else {
            "warm_white"
        }
    }

    /// Darker source photos get a stronger shadow lift so real dark pixels
    /// stay visible once additively blended against the black canvas.
    fn auto_shadow_lift(&self) -> f32 {
        (0.34 - self.mean_luminance * 0.32).clamp(0.14, 0.30)
    }

    /// Darker source photos get a higher exposure multiplier at render time.
    fn auto_exposure(&self) -> f32 {
        (2.05 - self.mean_luminance * 1.05).clamp(1.35, 2.1)
    }
}

/// Linearly scales a 0..1 `score` into a `[min, max]` particle-count target.
fn scale_count(min: usize, max: usize, score: f32) -> usize {
    let s = score.clamp(0.0, 1.0);
    (min as f32 + (max - min) as f32 * s).round() as usize
}

/// Smaller points for denser tiers (they don't need per-particle bulk to
/// look filled in), larger points for sparser tiers (fewer particles need to
/// cover the same area) — replaces the old admin `particle_size` field.
fn auto_base_point_size(count: usize) -> f32 {
    let c = (count.max(1) as f32).sqrt();
    (2.65 - c / 300.0).clamp(1.5, 2.6)
}

/// Compact versioned binary layout — no JSON for tens of thousands of points.
/// All multi-byte fields little-endian. f32 blocks come first so each one
/// starts at a 4-byte-aligned offset (required to later hand the browser a
/// zero-copy `Float32Array` view straight over the decompressed buffer); the
/// byte-per-element blocks (no alignment constraint) come last.
///
/// ```text
/// offset  0: magic "PVM1"            (4 bytes)
/// offset  4: format version (u16 LE) (2 bytes) — 3
/// offset  6: reserved                (2 bytes)
/// offset  8: particle count (u32 LE) (4 bytes)
/// offset 12: width  (f32 LE)         (4 bytes)
/// offset 16: height (f32 LE)         (4 bytes)
/// offset 20: basePointSize (f32 LE)  (4 bytes) — auto-derived, replaces admin particle_size
/// offset 24: autoExposure (f32 LE)   (4 bytes) — auto-derived, replaces the old fixed frontend constant
/// offset 28: targets  f32[count * 3] (count * 12 bytes) — xyz, z = baked depth
/// offset  +: starts   f32[count * 3] (count * 12 bytes)
/// offset  +: colors   u8[count * 3]  (count * 3 bytes,  0..255 per channel, real graded RGB)
/// offset  +: alphas   u8[count]      (count bytes,      0..255, baked per-particle alpha)
/// offset  +: sizes    u8[count]      (count bytes,      0..255, baked per-particle size)
/// offset  +: reveal   u8[count]      (count bytes,      0..255, reveal-order 0=first..255=last)
/// offset  +: randoms  u8[count]      (count bytes,      0..255)
/// ```
fn encode_binary(map: &ParticleMapBuffers) -> Vec<u8> {
    let mut buf = Vec::with_capacity(
        28 + map.targets.len() * 4
            + map.starts.len() * 4
            + map.colors.len()
            + map.alphas.len()
            + map.sizes.len()
            + map.reveal.len()
            + map.randoms.len(),
    );
    buf.extend_from_slice(BINARY_MAGIC);
    buf.extend_from_slice(&BINARY_FORMAT_VERSION.to_le_bytes());
    buf.extend_from_slice(&0u16.to_le_bytes());
    buf.extend_from_slice(&(map.count as u32).to_le_bytes());
    buf.extend_from_slice(&map.width.to_le_bytes());
    buf.extend_from_slice(&map.height.to_le_bytes());
    buf.extend_from_slice(&map.base_point_size.to_le_bytes());
    buf.extend_from_slice(&map.auto_exposure.to_le_bytes());
    for v in &map.targets {
        buf.extend_from_slice(&v.to_le_bytes());
    }
    for v in &map.starts {
        buf.extend_from_slice(&v.to_le_bytes());
    }
    buf.extend_from_slice(&map.colors);
    buf.extend_from_slice(&map.alphas);
    buf.extend_from_slice(&map.sizes);
    buf.extend_from_slice(&map.reveal);
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

    fn test_candidate(x: u32, y: u32, importance: f32, reveal: f32) -> Candidate {
        Candidate {
            x,
            y,
            r: 0.6,
            g: 0.5,
            b: 0.4,
            importance,
            reveal,
        }
    }

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
    fn extract_candidates_produces_reveal_bands_and_real_color() {
        let img = synthetic_image(160, 160).to_rgba8();
        let candidates = extract_candidates(&img, 160, 160);
        // Real sampled color should be preserved (not replaced by a
        // synthetic gold/silver constant) — the disc's fill color was
        // (220, 190, 150), i.e. roughly r > g > b.
        let inside = candidates
            .iter()
            .find(|c| c.r > 0.5)
            .expect("at least one bright candidate from the disc");
        assert!(inside.r > inside.b);
        // Reveal order must stay within the documented 0..1 range.
        assert!(candidates.iter().all(|c| c.reveal >= 0.0 && c.reveal <= 1.0));
    }

    fn stats_candidate(r: f32, g: f32, b: f32, importance: f32) -> Candidate {
        Candidate { x: 0, y: 0, r, g, b, importance, reveal: 0.5 }
    }

    #[test]
    fn image_stats_auto_color_mode_reads_the_image_not_a_setting() {
        // A warm golden-toned image (r > g > b, dominant warmth) should pick
        // "gold" with no admin input at all.
        let warm = vec![stats_candidate(0.85, 0.65, 0.30, 0.7); 20];
        assert_eq!(ImageStats::compute(&warm, 100, 100).auto_color_mode(), "gold");

        // A desaturated (near-grayscale) image should pick "silver" instead.
        let desaturated = vec![stats_candidate(0.55, 0.54, 0.53, 0.5); 20];
        assert_eq!(ImageStats::compute(&desaturated, 100, 100).auto_color_mode(), "silver");
    }

    #[test]
    fn image_stats_darker_image_gets_more_lift_and_more_exposure() {
        let dark = ImageStats::compute(&vec![stats_candidate(0.15, 0.12, 0.10, 0.5); 20], 100, 100);
        let bright = ImageStats::compute(&vec![stats_candidate(0.85, 0.82, 0.80, 0.5); 20], 100, 100);
        assert!(dark.auto_shadow_lift() > bright.auto_shadow_lift());
        assert!(dark.auto_exposure() > bright.auto_exposure());
    }

    #[test]
    fn detail_score_scales_particle_count_within_bounds() {
        // A sparse, low-importance candidate pool over a large sample area
        // should land near the low end of the range; a dense, high-
        // importance pool should land near the high end — with no manual
        // particle-count setting involved anywhere in this computation.
        let sparse: Vec<Candidate> = (0..50).map(|_| stats_candidate(0.5, 0.5, 0.5, 0.1)).collect();
        let dense: Vec<Candidate> = (0..5000).map(|_| stats_candidate(0.5, 0.5, 0.5, 0.9)).collect();

        let sparse_score = ImageStats::compute(&sparse, 1000, 1000).detail_score;
        let dense_score = ImageStats::compute(&dense, 1000, 1000).detail_score;
        assert!(dense_score > sparse_score);

        let sparse_count = scale_count(DESKTOP_MIN_COUNT, DESKTOP_MAX_COUNT, sparse_score);
        let dense_count = scale_count(DESKTOP_MIN_COUNT, DESKTOP_MAX_COUNT, dense_score);
        assert!(sparse_count >= DESKTOP_MIN_COUNT && sparse_count <= DESKTOP_MAX_COUNT);
        assert!(dense_count >= DESKTOP_MIN_COUNT && dense_count <= DESKTOP_MAX_COUNT);
        assert!(dense_count > sparse_count);
    }

    #[test]
    fn auto_base_point_size_shrinks_as_particle_count_grows() {
        let sparse_size = auto_base_point_size(LOW_POWER_MIN_COUNT);
        let dense_size = auto_base_point_size(DESKTOP_MAX_COUNT);
        assert!(sparse_size > dense_size);
        assert!(sparse_size <= 2.6 && dense_size >= 1.5);
    }

    #[test]
    fn pick_tier_respects_target_and_pool_size() {
        let candidates: Vec<Candidate> = (0..1000)
            .map(|i| test_candidate(i % 40, i / 40, (i % 10) as f32 / 10.0, (i % 5) as f32 / 5.0))
            .collect();

        let picked = pick_tier(&candidates, 200, 40, 25);
        assert_eq!(picked.len(), 200);

        // Requesting more than the pool has just returns the whole pool.
        let picked_all = pick_tier(&candidates, 5000, 40, 25);
        assert_eq!(picked_all.len(), candidates.len());
    }

    #[test]
    fn pick_tier_blue_noise_avoids_exact_duplicate_positions() {
        // A dense grid of candidates all sharing the same importance: naive
        // stride-picking would produce a very regular pattern; blue-noise
        // dart-throwing should still enforce a minimum spacing rather than
        // just taking every Nth raster-order point.
        let mut candidates = Vec::new();
        for y in 0..60u32 {
            for x in 0..60u32 {
                candidates.push(test_candidate(x, y, 0.5, 0.5));
            }
        }
        let picked = pick_tier(&candidates, 400, 60, 60);
        assert!(picked.len() <= 400 && !picked.is_empty());
        // No two accepted points should sit at the exact same pixel (sanity
        // check that the selection isn't just returning input order verbatim).
        let mut seen = std::collections::HashSet::new();
        for c in &picked {
            assert!(seen.insert((c.x, c.y)), "duplicate position in blue-noise pick");
        }
    }

    #[test]
    fn binary_roundtrip_header_and_payload_layout() {
        let picked = vec![
            test_candidate(10, 20, 0.9, 0.1),
            test_candidate(30, 5, 0.3, 0.7),
            test_candidate(0, 0, 0.05, 0.9),
        ];
        let map = build_particle_map(&picked, 100, 100, 1.0, "gold", 0.2, 1.6);
        assert_eq!(map.count, 3);
        let raw = encode_binary(&map);

        let expected_len = 28
            + map.count * 3 * 4 // targets
            + map.count * 3 * 4 // starts
            + map.count * 3 // colors
            + map.count // alphas
            + map.count // sizes
            + map.count // reveal
            + map.count; // randoms
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
        let base_point_size = f32::from_le_bytes([raw[20], raw[21], raw[22], raw[23]]);
        assert_eq!(base_point_size, map.base_point_size);
        let auto_exposure = f32::from_le_bytes([raw[24], raw[25], raw[26], raw[27]]);
        assert_eq!(auto_exposure, map.auto_exposure);

        // First target.x (offset 28) matches what build_particle_map computed.
        let first_target_x = f32::from_le_bytes([raw[28], raw[29], raw[30], raw[31]]);
        assert_eq!(first_target_x, map.targets[0]);

        // Colors block starts right after both f32 blocks (4-byte aligned by
        // construction — count*12 is always a multiple of 4).
        let colors_offset = 28 + map.count * 3 * 4 + map.count * 3 * 4;
        assert_eq!(raw[colors_offset], map.colors[0]);

        let alphas_offset = colors_offset + map.count * 3;
        assert_eq!(raw[alphas_offset], map.alphas[0]);

        let sizes_offset = alphas_offset + map.count;
        assert_eq!(raw[sizes_offset], map.sizes[0]);

        let reveal_offset = sizes_offset + map.count;
        assert_eq!(raw[reveal_offset], map.reveal[0]);

        let randoms_offset = reveal_offset + map.count;
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
            test_candidate(10, 20, 0.9, 0.1),
            test_candidate(30, 5, 0.3, 0.7),
            test_candidate(50, 50, 0.5, 0.5),
            test_candidate(90, 80, 0.1, 0.95),
        ];
        let map = build_particle_map(&picked, 100, 100, 1.4, "silver", 0.2, 1.6);
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
