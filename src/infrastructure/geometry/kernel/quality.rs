//! Geometry quality presets (PR #23).
//!
//! Plasticity-style quality switch for procedural mesh generators. Controls:
//!   * **radial_segments** — how many slices around a lathed profile or
//!     angular fan. Drives circle smoothness on every revolve / disk / band.
//!   * **surface_rings**  — how many concentric rings on heightfield-style
//!     surfaces (sauce swirl, plate-food mound). Drives radial smoothness.
//!
//! Recommended:
//!   * `Draft`     — fast preview, large vertex budget hidden.
//!   * `Standard`  — old default (PR #11–#14 hard-coded values).
//!   * `High`      — Studio default — visibly smoother circles, ~4× verts.
//!   * `Ultra`     — final-render GLB, ~8–10× verts vs Draft.
//!
//! Render-side note: this is **not** the same axis as `RenderQuality` on the
//! frontend. Render quality changes instantly (DPR / shadow maps / AA);
//! geometry quality requires a model regeneration.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GeometryQuality {
    /// 32 segments / 8 rings — fast preview.
    Draft,
    /// 48 segments / 14 rings — legacy default (PR #11–#14).
    Standard,
    /// 144 segments / 48 rings — Studio default (PR #32 — smoother sauce/food).
    #[default]
    High,
    /// 192 segments / 64 rings — final-render GLB (PR #32).
    Ultra,
}

impl GeometryQuality {
    /// Slices around the Y axis on lathed profiles, disk fans, label bands.
    #[inline]
    pub fn radial_segments(self) -> usize {
        match self {
            GeometryQuality::Draft    => 32,
            GeometryQuality::Standard => 48,
            GeometryQuality::High     => 144,
            GeometryQuality::Ultra    => 192,
        }
    }

    /// Concentric rings on heightfield surfaces (sauce swirl, food mound).
    #[inline]
    pub fn surface_rings(self) -> usize {
        match self {
            GeometryQuality::Draft    => 8,
            GeometryQuality::Standard => 14,
            GeometryQuality::High     => 48,
            GeometryQuality::Ultra    => 64,
        }
    }

    /// Stable lowercase identifier used by the HTTP API and persisted specs.
    #[inline]
    pub fn as_str(self) -> &'static str {
        match self {
            GeometryQuality::Draft => "draft",
            GeometryQuality::Standard => "standard",
            GeometryQuality::High => "high",
            GeometryQuality::Ultra => "ultra",
        }
    }

    /// Parse a quality identifier (case-insensitive). Unknown → `None`.
    pub fn from_str_ci(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "draft" => Some(GeometryQuality::Draft),
            "standard" | "std" => Some(GeometryQuality::Standard),
            "high" => Some(GeometryQuality::High),
            "ultra" => Some(GeometryQuality::Ultra),
            _ => None,
        }
    }

    /// Same as [`from_str_ci`] but returns `Default` for unknown / `None`.
    pub fn from_opt(s: Option<&str>) -> Self {
        s.and_then(Self::from_str_ci).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_studio_spec() {
        assert_eq!(GeometryQuality::default(), GeometryQuality::High);
        assert_eq!(GeometryQuality::High.radial_segments(), 144);
        assert_eq!(GeometryQuality::High.surface_rings(), 48);
    }

    #[test]
    fn radial_and_rings_strictly_increase() {
        let order = [
            GeometryQuality::Draft,
            GeometryQuality::Standard,
            GeometryQuality::High,
            GeometryQuality::Ultra,
        ];
        for w in order.windows(2) {
            assert!(w[0].radial_segments() < w[1].radial_segments());
            assert!(w[0].surface_rings() < w[1].surface_rings());
        }
    }

    #[test]
    fn from_str_ci_is_case_insensitive() {
        assert_eq!(GeometryQuality::from_str_ci("HIGH"), Some(GeometryQuality::High));
        assert_eq!(GeometryQuality::from_str_ci(" Ultra "), Some(GeometryQuality::Ultra));
        assert_eq!(GeometryQuality::from_str_ci("std"), Some(GeometryQuality::Standard));
        assert_eq!(GeometryQuality::from_str_ci("nope"), None);
    }

    #[test]
    fn from_opt_falls_back_to_default() {
        assert_eq!(GeometryQuality::from_opt(None), GeometryQuality::High);
        assert_eq!(GeometryQuality::from_opt(Some("garbage")), GeometryQuality::High);
        assert_eq!(GeometryQuality::from_opt(Some("draft")), GeometryQuality::Draft);
    }

    #[test]
    fn as_str_roundtrips() {
        for q in [
            GeometryQuality::Draft,
            GeometryQuality::Standard,
            GeometryQuality::High,
            GeometryQuality::Ultra,
        ] {
            assert_eq!(GeometryQuality::from_str_ci(q.as_str()), Some(q));
        }
    }
}
