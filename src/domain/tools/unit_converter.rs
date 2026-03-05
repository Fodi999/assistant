/// Unit Converter — domain layer
///
/// All conversions go through a base unit:
///   mass   → grams (g)
///   volume → milliliters (ml)

// ── Mass table (base: grams) ─────────────────────────────────────────────────

pub fn mass_to_grams(unit: &str) -> Option<f64> {
    match unit {
        "mg" => Some(0.001),
        "g"  => Some(1.0),
        "kg" => Some(1_000.0),
        "oz" => Some(28.3495),
        "lb" => Some(453.592),
        _    => None,
    }
}

// ── Volume table (base: ml) ──────────────────────────────────────────────────

pub fn volume_to_ml(unit: &str) -> Option<f64> {
    match unit {
        "ml"     => Some(1.0),
        "l"      => Some(1_000.0),
        "fl_oz"  => Some(29.5735),
        "tsp"    => Some(4.92892),
        "tbsp"   => Some(14.7868),
        "cup"    => Some(236.588),
        "pint"   => Some(473.176),
        "quart"  => Some(946.353),
        "gallon" => Some(3_785.41),
        // Micro-kitchen
        "dash"        => Some(0.616),    // ~1/8 tsp
        "pinch"       => Some(0.308),    // ~1/16 tsp
        "drop"        => Some(0.051),    // ~1 drop
        "stick_butter" => Some(118.294), // 1/2 cup (US stick)
        _             => None,
    }
}

// ── Core converters ──────────────────────────────────────────────────────────

pub fn convert_mass(value: f64, from: &str, to: &str) -> Option<f64> {
    let f = mass_to_grams(from)?;
    let t = mass_to_grams(to)?;
    Some(value * f / t)
}

pub fn convert_volume(value: f64, from: &str, to: &str) -> Option<f64> {
    let f = volume_to_ml(from)?;
    let t = volume_to_ml(to)?;
    Some(value * f / t)
}

/// Universal converter: tries mass first, then volume.
/// Returns `None` when units are from different groups (use density for that).
pub fn convert_units(value: f64, from: &str, to: &str) -> Option<f64> {
    if let Some(r) = convert_mass(value, from, to) {
        return Some(r);
    }
    if let Some(r) = convert_volume(value, from, to) {
        return Some(r);
    }
    None
}

/// Density-aware cross-group conversion:  volume ↔ mass via g/ml density.
pub fn convert_with_density(value: f64, from: &str, to: &str, density_g_per_ml: f64) -> Option<f64> {
    // Same group → direct
    if let Some(r) = convert_units(value, from, to) {
        return Some(r);
    }

    // volume → mass:   value(vol) → ml → g(density) → target mass
    if let Some(from_ml) = volume_to_ml(from) {
        if let Some(to_g) = mass_to_grams(to) {
            let ml = value * from_ml;
            let grams = ml * density_g_per_ml;
            return Some(grams / to_g);
        }
    }

    // mass → volume:   value(mass) → g → ml(density) → target volume
    if let Some(from_g) = mass_to_grams(from) {
        if let Some(to_ml) = volume_to_ml(to) {
            let grams = value * from_g;
            let ml = grams / density_g_per_ml;
            return Some(ml / to_ml);
        }
    }

    None
}

// ── Smart auto-unit ──────────────────────────────────────────────────────────

/// Picks the most human-friendly mass unit for a value in grams.
pub fn smart_mass_unit(grams: f64) -> (&'static str, f64) {
    if grams >= 1_000.0 {
        ("kg", grams / 1_000.0)
    } else if grams < 1.0 {
        ("mg", grams * 1_000.0)
    } else {
        ("g", grams)
    }
}

/// Picks the most human-friendly volume unit for a value in ml.
pub fn smart_volume_unit(ml: f64) -> (&'static str, f64) {
    if ml >= 1_000.0 {
        ("l", ml / 1_000.0)
    } else if ml < 1.0 {
        ("drop", ml / 0.051)
    } else {
        ("ml", ml)
    }
}

// ── Recipe scaler ────────────────────────────────────────────────────────────

pub fn scale(value: f64, from_portions: f64, to_portions: f64) -> f64 {
    if from_portions <= 0.0 {
        return 0.0;
    }
    value * to_portions / from_portions
}

// ── Yield calculator ─────────────────────────────────────────────────────────

pub fn yield_percent(raw: f64, usable: f64) -> f64 {
    if raw <= 0.0 {
        return 0.0;
    }
    (usable / raw) * 100.0
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Round to N decimal places
pub fn round_to(value: f64, decimals: u32) -> f64 {
    let factor = 10f64.powi(decimals as i32);
    (value * factor).round() / factor
}

/// Display-friendly rounding that snaps near-integers.
/// Eliminates artefacts like 4.000004 → 4.0, 15.999946 → 16.0, 0.99998 → 1.0
pub fn display_round(v: f64) -> f64 {
    // First: snap to nearest integer if within 0.0005
    let rounded_int = v.round();
    if (v - rounded_int).abs() < 0.0005 {
        return rounded_int;
    }
    // Then: adaptive precision
    let abs = v.abs();
    if abs >= 100.0 {
        round_to(v, 2)
    } else if abs >= 1.0 {
        round_to(v, 4)
    } else {
        round_to(v, 6)
    }
}

/// Smart rounding for auto-unit display: 2 dp for ≥ 1, 3 dp for < 1.
pub fn smart_round(v: f64) -> f64 {
    if v.abs() >= 1.0 {
        round_to(v, 2)
    } else {
        round_to(v, 3)
    }
}

/// Food cost helpers
pub fn food_cost(price_per_unit: f64, amount: f64) -> f64 {
    price_per_unit * amount
}

pub fn cost_per_portion(total_cost: f64, portions: f64) -> f64 {
    if portions <= 0.0 { return 0.0; }
    total_cost / portions
}

pub fn margin_percent(sell_price: f64, cost: f64) -> f64 {
    if sell_price <= 0.0 { return 0.0; }
    ((sell_price - cost) / sell_price) * 100.0
}

/// Check if unit is a mass unit
pub fn is_mass(unit: &str) -> bool {
    mass_to_grams(unit).is_some()
}

/// Check if unit is a volume unit
pub fn is_volume(unit: &str) -> bool {
    volume_to_ml(unit).is_some()
}

/// Returns all supported mass unit codes
pub fn mass_units() -> &'static [&'static str] {
    &["mg", "g", "kg", "oz", "lb"]
}

/// Returns all supported volume unit codes (includes kitchen)
pub fn volume_units() -> &'static [&'static str] {
    &["ml", "l", "fl_oz", "tsp", "tbsp", "cup", "pint", "quart", "gallon",
      "dash", "pinch", "drop", "stick_butter"]
}
