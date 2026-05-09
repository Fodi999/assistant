// ── Scene: Ingredient — a kitchen ingredient as a scene entity ───────────────────
// Domain: Restaurant domain mapped to scene — an ingredient occupies a slot
// in the inventory and can be visualised as a particle cluster.

/// A named ingredient with a stock quantity, used to drive particle density
/// and color in the cloud formation.
#[derive(Debug, Clone)]
pub struct Ingredient {
    /// Display name (e.g. "Авокадо").
    pub name: String,
    /// Current stock in base units (grams, ml, …).
    pub quantity: f32,
    /// Unit label shown in HUD (e.g. "г", "мл", "шт").
    pub unit: &'static str,
    /// RGB color hint for particles representing this ingredient.
    pub color_hint: [f32; 3],
}

impl Ingredient {
    pub fn new(
        name: impl Into<String>,
        quantity: f32,
        unit: &'static str,
        color_hint: [f32; 3],
    ) -> Self {
        Self {
            name: name.into(),
            quantity,
            unit,
            color_hint,
        }
    }

    /// Normalised stock level 0..1 relative to a reference maximum.
    pub fn stock_level(&self, max_quantity: f32) -> f32 {
        if max_quantity <= 0.0 {
            return 0.0;
        }
        (self.quantity / max_quantity).clamp(0.0, 1.0)
    }
}
