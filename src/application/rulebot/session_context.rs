//! Session Context — lightweight per-session memory for ChefOS chat.
//!
//! Stored client-side (JSON in `context` field of ChatRequest).
//! Server is stateless — no Redis, no DB, no memory leak.
//!
//! Flow:
//!   Client sends:   { "input": "...", "context": { ... } }
//!   Server returns: { "text": "...", "context": { ... } }  ← updated context
//!   Client stores context and sends it back next turn.
//!
//! What context stores:
//!   - last_intent     — previous turn's intent
//!   - last_product    — last product slug mentioned (for follow-up "а сколько в нём калорий?")
//!   - last_lang       — last detected language
//!   - turn_count      — how many turns this session
//!   - modifier        — last explicit goal ("на массу", "похудение")

use serde::{Deserialize, Serialize};
use super::intent_router::{ChatLang, HealthModifier, Intent};
use super::category_filter::ProductCategory;

// ── Session Context ───────────────────────────────────────────────────────────

/// Lightweight per-session memory — sent and received as JSON.
/// Client is responsible for storage between turns.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionContext {
    /// Last detected intent (for follow-up resolution).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_intent: Option<Intent>,

    /// All intents from the last turn (multi-intent context).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub last_intents: Vec<Intent>,

    /// Last product slug mentioned (enables "а сколько в нём калорий?").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_product_slug: Option<String>,

    /// Last product name (for display in follow-ups).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_product_name: Option<String>,

    /// Last detected language (persists across turns).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_lang: Option<ChatLang>,

    /// Last explicit health modifier ("на массу", "похудеть", etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_modifier: Option<HealthModifier>,

    /// Last active goal modifier — remembered for "а что ещё?" follow-ups.
    /// More specific than last_modifier: set even if last turn had no explicit modifier
    /// (inferred from context). Maps to HealthModifier enum.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_goal: Option<HealthModifier>,

    /// Slugs of cards shown in the last turn — used to exclude them from
    /// "а что ещё?" / "show me something else" follow-up responses.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub last_cards: Vec<String>,

    /// Cumulative slugs shown across ALL turns this session — prevents repeats
    /// when user keeps asking "ещё" / "more".  Capped at 30 to limit JSON size.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shown_slugs: Vec<String>,

    /// How many turns this session has had (for personalization and 80/20 exploration).
    #[serde(default)]
    pub turn_count: u32,

    // ── Step 3 (stateful v1) — operational state ────────────────────────
    // Filled by the CLIENT after the user actually executes an action
    // (AddToPlan, AddToShopping). Server uses these to:
    //   • exclude already-chosen items from new suggestions
    //   • drop AddToPlan / AddToShopping actions from cards already added
    //   • route follow-up suggestions toward complementary categories
    //
    // Strict contract: only operational state, no full message history.

    /// Recipe identifiers (display_name) the user has added to their plan
    /// in this session. Capped at 20 to keep JSON small.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub added_recipes: Vec<String>,

    /// Product slugs the user has added to their shopping list this session.
    /// Capped at 30.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub added_products: Vec<String>,

    /// The category of the last shown card set (vegetable / fish / meat …).
    /// Used by the suggestion layer to recommend complementary categories
    /// ("you got protein → suggest a vegetable side").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_category: Option<ProductCategory>,

    // ── Step 4 Day 2 (personalization) — server-populated, ephemeral ────
    // These fields are derived from `user_preferences` on every turn and
    // intentionally `#[serde(skip)]` — we never trust client-provided values
    // for allergy/diet filtering, and there's no point serializing them back
    // (client re-populates via stored profile next turn).

    /// Product slugs to HARD-exclude from every recommendation:
    /// allergies + intolerances + explicit dislikes, resolved against the
    /// ingredient cache by stem/name matching.
    #[serde(skip)]
    pub preference_excludes: Vec<String>,

    /// Product slugs the user explicitly likes — used for soft-boost scoring
    /// in future iterations (not a hard include).
    #[serde(skip)]
    pub preference_likes: Vec<String>,
}

impl SessionContext {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update context after a turn.
    pub fn advance(
        &self,
        intent: Intent,
        intents: Vec<Intent>,
        product_slug: Option<String>,
        product_name: Option<String>,
        lang: ChatLang,
        modifier: HealthModifier,
        card_slugs: Vec<String>,
        last_category: Option<ProductCategory>,
    ) -> Self {
        let effective_goal = if modifier != HealthModifier::None {
            Some(modifier)
        } else {
            self.last_goal.or(self.last_modifier)
        };
        Self {
            last_intent: Some(intent),
            last_intents: intents,
            last_product_slug: product_slug.or_else(|| self.last_product_slug.clone()),
            last_product_name: product_name.or_else(|| self.last_product_name.clone()),
            last_lang: Some(lang),
            last_modifier: if modifier != HealthModifier::None {
                Some(modifier)
            } else {
                self.last_modifier
            },
            last_goal: effective_goal,
            last_cards: card_slugs.clone(),
            shown_slugs: {
                let mut all = self.shown_slugs.clone();
                for s in card_slugs {
                    if !all.contains(&s) {
                        all.push(s);
                    }
                }
                // Cap at 30 to keep JSON small
                if all.len() > 30 { all.drain(..all.len() - 30); }
                all
            },
            turn_count: self.turn_count + 1,
            // ── Step 3: operational state passes through unchanged ─────
            // Server never mutates these — the client owns them and updates
            // them when the user actually executes an action. last_category
            // is overwritten when this turn produced category-typed cards.
            added_recipes: self.added_recipes.clone(),
            added_products: self.added_products.clone(),
            last_category: last_category.or(self.last_category),
            // Personalization is server-derived per turn; pass through.
            preference_excludes: self.preference_excludes.clone(),
            preference_likes: self.preference_likes.clone(),
        }
    }
    /// Slugs to exclude in "а что ещё?" follow-ups.
    /// Returns cumulative shown_slugs (all products ever shown this session)
    /// PLUS added_products (already in shopping list — no point re-suggesting)
    /// PLUS preference_excludes (allergies / intolerances / dislikes — HARD filter).
    /// Falls back to last_cards on first turn.
    pub fn excluded_slugs(&self) -> Vec<String> {
        let mut out: Vec<String> = if self.shown_slugs.is_empty() {
            self.last_cards.clone()
        } else {
            self.shown_slugs.clone()
        };
        for p in &self.added_products {
            if !out.contains(p) {
                out.push(p.clone());
            }
        }
        for p in &self.preference_excludes {
            if !out.contains(p) {
                out.push(p.clone());
            }
        }
        out
    }

    /// Resolve follow-up: "а сколько в нём калорий?" — refers to last_product.
    /// Returns true if input is a pronoun follow-up referring to last product.
    pub fn is_followup(&self, input: &str) -> bool {
        if self.last_product_slug.is_none() {
            return false;
        }
        let t = input.to_lowercase();
        // Russian pronouns for "in it", "about it"
        let followup_patterns = [
            "в нём", "в ней", "о нём", "о ней",
            "его", "её", "про него", "про неё",
            "in it", "about it", "its", "this",
            "w nim", "o nim", "tego",
        ];
        followup_patterns.iter().any(|p| t.contains(p))
    }

    /// Effective lang: current turn's lang OR last turn's (fallback).
    pub fn effective_lang(&self, current: ChatLang) -> ChatLang {
        // If current turn has no meaningful signal (bare short input), use last
        current
    }

    /// Effective modifier: current OR remembered from last turn.
    pub fn effective_modifier(&self, current: HealthModifier) -> HealthModifier {
        if current != HealthModifier::None {
            current
        } else {
            self.last_modifier.unwrap_or(HealthModifier::None)
        }
    }

    /// Return the best modifier from context as Option (for DialogContext).
    /// Tries last_goal first (persistent), then last_modifier.
    pub fn effective_modifier_opt(&self) -> Option<HealthModifier> {
        self.last_goal
            .or(self.last_modifier)
            .filter(|m| *m != HealthModifier::None)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_followup_detection() {
        let mut ctx = SessionContext::new();
        ctx.last_product_slug = Some("salmon".to_string());
        ctx.last_product_name = Some("Лосось".to_string());

        assert!(ctx.is_followup("а сколько в нём калорий?"));
        assert!(ctx.is_followup("расскажи о нём подробнее"));
        assert!(!ctx.is_followup("что полезного поесть"));
    }

    #[test]
    fn test_advance_preserves_product() {
        let ctx = SessionContext::new();
        let next = ctx.advance(
            Intent::NutritionInfo,
            vec![Intent::NutritionInfo],
            Some("spinach".to_string()),
            Some("Шпинат".to_string()),
            ChatLang::Ru,
            HealthModifier::None,
            vec!["spinach".to_string()],
            Some(ProductCategory::Vegetable),
        );
        assert_eq!(next.last_product_slug, Some("spinach".to_string()));
        assert_eq!(next.turn_count, 1);
        assert_eq!(next.last_cards, vec!["spinach".to_string()]);
        assert_eq!(next.last_category, Some(ProductCategory::Vegetable));

        // Second turn without product OR category — should KEEP both
        let next2 = next.advance(Intent::Greeting, vec![], None, None, ChatLang::Ru, HealthModifier::None, vec![], None);
        assert_eq!(next2.last_product_slug, Some("spinach".to_string()));
        assert_eq!(next2.turn_count, 2);
        assert_eq!(next2.last_category, Some(ProductCategory::Vegetable));
        // last_cards cleared (no cards this turn)
        assert!(next2.last_cards.is_empty());
    }

    #[test]
    fn test_modifier_persistence() {
        let ctx = SessionContext::new();
        let next = ctx.advance(
            Intent::HealthyProduct,
            vec![Intent::HealthyProduct],
            None,
            None,
            ChatLang::Ru,
            HealthModifier::HighProtein,
            vec![],
            None,
        );
        // Next turn no modifier — should carry forward HighProtein
        let effective = next.effective_modifier(HealthModifier::None);
        assert_eq!(effective, HealthModifier::HighProtein);
        // last_goal should also be set
        assert_eq!(next.last_goal, Some(HealthModifier::HighProtein));
    }

    #[test]
    fn test_added_state_passes_through() {
        let mut ctx = SessionContext::new();
        ctx.added_recipes.push("Овсянка с ягодами".to_string());
        ctx.added_products.push("oats".to_string());

        let next = ctx.advance(
            Intent::HealthyProduct,
            vec![Intent::HealthyProduct],
            None, None,
            ChatLang::Ru,
            HealthModifier::None,
            vec![],
            None,
        );
        assert_eq!(next.added_recipes, vec!["Овсянка с ягодами".to_string()]);
        assert_eq!(next.added_products, vec!["oats".to_string()]);
    }
}
