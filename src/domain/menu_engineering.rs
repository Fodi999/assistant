use serde::{Deserialize, Serialize};

use crate::domain::DishId;

/// ABC Classification (Pareto Principle - 80/20 rule)
/// 
/// Classification based on revenue contribution:
/// - A: Top 80% of revenue (critical items)
/// - B: Next 15% of revenue (important items)
/// - C: Last 5% of revenue (marginal items)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum AbcClass {
    /// Top performers: 80% of revenue
    /// Critical items that drive business
    A,
    
    /// Middle performers: 15% of revenue
    /// Important but not critical
    B,
    
    /// Low performers: 5% of revenue
    /// Consider removing or redesigning
    C,
}

impl AbcClass {
    /// Classify based on cumulative revenue share
    pub fn classify(cumulative_share: f64) -> Self {
        if cumulative_share <= 0.80 {
            AbcClass::A
        } else if cumulative_share <= 0.95 {
            AbcClass::B
        } else {
            AbcClass::C
        }
    }
    
    /// Get emoji representation
    pub fn emoji(&self) -> &'static str {
        match self {
            AbcClass::A => "🥇",
            AbcClass::B => "🥈",
            AbcClass::C => "🥉",
        }
    }
}

/// Menu Engineering Category (Boston Consulting Group Matrix)
/// 
/// Classification based on:
/// - Profitability (profit margin %)
/// - Popularity (sales volume/frequency)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MenuCategory {
    /// High margin + High sales
    /// Strategy: Keep, promote, maintain quality
    Star,
    
    /// Low margin + High sales
    /// Strategy: Increase price or reduce costs
    Plowhorse,
    
    /// High margin + Low sales
    /// Strategy: Marketing push, repositioning, bundle deals
    Puzzle,
    
    /// Low margin + Low sales
    /// Strategy: Remove from menu or redesign completely
    Dog,
}

impl MenuCategory {
    /// Classify dish based on profitability and popularity
    pub fn classify(profit_margin_percent: f64, popularity_score: f64) -> Self {
        // Thresholds (industry standard)
        const HIGH_MARGIN_THRESHOLD: f64 = 60.0;  // 60%+
        const HIGH_POPULARITY_THRESHOLD: f64 = 0.7; // 70th percentile
        
        let is_profitable = profit_margin_percent >= HIGH_MARGIN_THRESHOLD;
        let is_popular = popularity_score >= HIGH_POPULARITY_THRESHOLD;
        
        match (is_profitable, is_popular) {
            (true, true) => MenuCategory::Star,
            (false, true) => MenuCategory::Plowhorse,
            (true, false) => MenuCategory::Puzzle,
            (false, false) => MenuCategory::Dog,
        }
    }
    
    /// Get emoji representation
    pub fn emoji(&self) -> &'static str {
        match self {
            MenuCategory::Star => "⭐",
            MenuCategory::Plowhorse => "🐴",
            MenuCategory::Puzzle => "❓",
            MenuCategory::Dog => "🐶",
        }
    }
    
    /// Get strategic recommendation
    pub fn recommendation(&self, language: crate::shared::Language) -> String {
        use crate::shared::Language;
        
        match (self, language) {
            (MenuCategory::Star, Language::En) => 
                "Excellent! Keep this dish, promote it, and maintain quality.".to_string(),
            (MenuCategory::Star, Language::Pl) => 
                "Doskonale! Zachowaj to danie, promuj je i utrzymuj jakość.".to_string(),
            (MenuCategory::Star, Language::Uk) => 
                "Чудово! Залиште цю страву, просувайте її та підтримуйте якість.".to_string(),
            (MenuCategory::Star, Language::Ru) => 
                "Отлично! Сохраните это блюдо, продвигайте его и поддерживайте качество.".to_string(),
                
            (MenuCategory::Plowhorse, Language::En) => 
                "Popular but low profit. Consider increasing price or reducing costs.".to_string(),
            (MenuCategory::Plowhorse, Language::Pl) => 
                "Popularne, ale niski zysk. Rozważ podniesienie ceny lub obniżenie kosztów.".to_string(),
            (MenuCategory::Plowhorse, Language::Uk) => 
                "Популярна, але низький прибуток. Розгляньте підвищення ціни або зниження витрат.".to_string(),
            (MenuCategory::Plowhorse, Language::Ru) => 
                "Популярно, но низкая прибыль. Рассмотрите повышение цены или снижение затрат.".to_string(),
                
            (MenuCategory::Puzzle, Language::En) => 
                "High margin but low sales. Improve marketing or reposition this dish.".to_string(),
            (MenuCategory::Puzzle, Language::Pl) => 
                "Wysoka marża, ale niskie sprzedaże. Popraw marketing lub zmień pozycjonowanie.".to_string(),
            (MenuCategory::Puzzle, Language::Uk) => 
                "Висока маржа, але низькі продажі. Покращте маркетинг або репозиціонуйте страву.".to_string(),
            (MenuCategory::Puzzle, Language::Ru) => 
                "Высокая маржа, но низкие продажи. Улучшите маркетинг или репозиционируйте блюдо.".to_string(),
                
            (MenuCategory::Dog, Language::En) => 
                "Low profit and low sales. Consider removing from menu or complete redesign.".to_string(),
            (MenuCategory::Dog, Language::Pl) => 
                "Niski zysk i niskie sprzedaże. Rozważ usunięcie z menu lub całkowite przeprojektowanie.".to_string(),
            (MenuCategory::Dog, Language::Uk) => 
                "Низький прибуток та низькі продажі. Розгляньте видалення з меню або повний редизайн.".to_string(),
            (MenuCategory::Dog, Language::Ru) => 
                "Низкая прибыль и низкие продажи. Рассмотрите удаление из меню или полный редизайн.".to_string(),
        }
    }
    
    /// Get combined strategic recommendation (BCG × ABC)
    /// Provides actionable insights based on both profitability/popularity AND revenue contribution
    pub fn combined_strategy(
        bcg_category: MenuCategory,
        abc_class: AbcClass,
        language: crate::shared::Language,
    ) -> String {
        use crate::shared::Language;
        
        match (bcg_category, abc_class, language) {
            // ⭐🥇 Star + A: Protect at all costs
            (MenuCategory::Star, AbcClass::A, Language::En) => 
                "🎯 Core menu item! Protect quality, don't change price, ensure consistent availability.".to_string(),
            (MenuCategory::Star, AbcClass::A, Language::Ru) => 
                "🎯 Основа меню! Защищайте качество, не меняйте цену, обеспечьте постоянную доступность.".to_string(),
                
            // ⭐🥈 Star + B: Slight price increase opportunity
            (MenuCategory::Star, AbcClass::B, Language::En) => 
                "💰 Strong performer. Consider slight price increase (+5-10%) to maximize profit.".to_string(),
            (MenuCategory::Star, AbcClass::B, Language::Ru) => 
                "💰 Сильная позиция. Рассмотрите небольшое повышение цены (+5-10%) для максимизации прибыли.".to_string(),
                
            // ⭐🥉 Star + C: Impossible (Stars are popular, C is low revenue)
            (MenuCategory::Star, AbcClass::C, Language::En) => 
                "⚠️ Anomaly detected. High sales but low revenue - check portion size or pricing.".to_string(),
            (MenuCategory::Star, AbcClass::C, Language::Ru) => 
                "⚠️ Аномалия. Высокие продажи, но низкая выручка - проверьте размер порции или цену.".to_string(),
                
            // 🐴🥇 Plowhorse + A: Reduce portion or increase price
            (MenuCategory::Plowhorse, AbcClass::A, Language::En) => 
                "⚖️ High volume, low margin. Reduce portion size by 10-15% OR increase price by 15-20%.".to_string(),
            (MenuCategory::Plowhorse, AbcClass::A, Language::Ru) => 
                "⚖️ Большой объём, низкая маржа. Уменьшите порцию на 10-15% ИЛИ поднимите цену на 15-20%.".to_string(),
                
            // 🐴🥈 Plowhorse + B: Optimize costs
            (MenuCategory::Plowhorse, AbcClass::B, Language::En) => 
                "🔧 Popular but unprofitable. Optimize ingredient costs or find cheaper suppliers.".to_string(),
            (MenuCategory::Plowhorse, AbcClass::B, Language::Ru) => 
                "🔧 Популярно, но неприбыльно. Оптимизируйте стоимость ингредиентов или найдите дешевле поставщиков.".to_string(),
                
            // 🐴🥉 Plowhorse + C: Consider removal
            (MenuCategory::Plowhorse, AbcClass::C, Language::En) => 
                "🚫 Low margin, low revenue. Strong candidate for menu removal.".to_string(),
            (MenuCategory::Plowhorse, AbcClass::C, Language::Ru) => 
                "🚫 Низкая маржа, низкая выручка. Сильный кандидат на удаление из меню.".to_string(),
                
            // ❓🥇 Puzzle + A: Aggressive promotion
            (MenuCategory::Puzzle, AbcClass::A, Language::En) => 
                "📣 High margin, needs visibility! Move to top of menu, add photo, create combo deals.".to_string(),
            (MenuCategory::Puzzle, AbcClass::A, Language::Ru) => 
                "📣 Высокая маржа, нужна видимость! Переместите в топ меню, добавьте фото, создайте комбо.".to_string(),
                
            // ❓🥈 Puzzle + B: Marketing push
            (MenuCategory::Puzzle, AbcClass::B, Language::En) => 
                "📢 Profitable but underselling. Improve presentation, staff training, menu positioning.".to_string(),
            (MenuCategory::Puzzle, AbcClass::B, Language::Ru) => 
                "📢 Прибыльно, но недопродаётся. Улучшите подачу, обучите персонал, измените позицию в меню.".to_string(),
                
            // ❓🥉 Puzzle + C: Promotion or removal
            (MenuCategory::Puzzle, AbcClass::C, Language::En) => 
                "🎲 High margin but very low sales. Run 2-week promotion, then remove if no improvement.".to_string(),
            (MenuCategory::Puzzle, AbcClass::C, Language::Ru) => 
                "🎲 Высокая маржа, но очень низкие продажи. Проведите 2-недельную акцию, затем удалите при отсутствии роста.".to_string(),
                
            // 🐶🥇 Dog + A: Impossible (Dogs have low sales, A is high revenue)
            (MenuCategory::Dog, AbcClass::A, Language::En) => 
                "⚠️ Data anomaly. Low profit + low sales cannot generate high revenue.".to_string(),
            (MenuCategory::Dog, AbcClass::A, Language::Ru) => 
                "⚠️ Аномалия данных. Низкая прибыль + низкие продажи не могут давать высокую выручку.".to_string(),
                
            // 🐶🥈 Dog + B: Remove immediately
            (MenuCategory::Dog, AbcClass::B, Language::En) => 
                "❌ Unprofitable and unpopular. Remove from menu this week.".to_string(),
            (MenuCategory::Dog, AbcClass::B, Language::Ru) => 
                "❌ Неприбыльно и непопулярно. Удалите из меню на этой неделе.".to_string(),
                
            // 🐶🥉 Dog + C: Remove now
            (MenuCategory::Dog, AbcClass::C, Language::En) => 
                "🗑️ Complete failure. Remove from menu immediately and analyze why it failed.".to_string(),
            (MenuCategory::Dog, AbcClass::C, Language::Ru) => 
                "🗑️ Полный провал. Удалите из меню немедленно и проанализируйте причины неудачи.".to_string(),
                
            // Fallback for other language combinations (using English template)
            _ => MenuCategory::combined_strategy(bcg_category, abc_class, Language::En),
        }
    }
}

/// Dish performance metrics for Menu Engineering analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DishPerformance {
    /// Dish identifier
    pub dish_id: DishId,
    
    /// Dish name
    pub dish_name: String,
    
    /// Menu Engineering category (BCG Matrix)
    pub category: MenuCategory,
    
    /// ABC classification (revenue contribution)
    pub abc_class: AbcClass,
    
    /// Profit margin percentage
    pub profit_margin_percent: f64,
    
    /// Popularity score (0.0 to 1.0, normalized against all dishes)
    pub popularity_score: f64,
    
    /// Total sales volume (number of orders)
    pub sales_volume: u32,
    
    /// Total revenue generated (in cents)
    pub total_revenue_cents: i64,
    
    /// Total profit generated (in cents)
    pub total_profit_cents: i64,
    
    /// Contribution margin (profit × volume)
    pub contribution_margin_cents: i64,
    
    /// Cumulative revenue share (for ABC analysis)
    pub cumulative_revenue_share: f64,
    
    /// Strategic recommendation (BCG only)
    pub recommendation: String,
    
    /// Combined strategy (BCG × ABC)
    pub strategy: String,
}

impl DishPerformance {
    pub fn new(
        dish_id: DishId,
        dish_name: String,
        profit_margin_percent: f64,
        popularity_score: f64,
        sales_volume: u32,
        total_revenue_cents: i64,
        total_profit_cents: i64,
        cumulative_revenue_share: f64,
        language: crate::shared::Language,
    ) -> Self {
        let category = MenuCategory::classify(profit_margin_percent, popularity_score);
        let abc_class = AbcClass::classify(cumulative_revenue_share);
        let contribution_margin_cents = total_profit_cents;
        let recommendation = category.recommendation(language);
        let strategy = MenuCategory::combined_strategy(category, abc_class, language);
        
        Self {
            dish_id,
            dish_name,
            category,
            abc_class,
            profit_margin_percent,
            popularity_score,
            sales_volume,
            total_revenue_cents,
            total_profit_cents,
            contribution_margin_cents,
            cumulative_revenue_share,
            recommendation,
            strategy,
        }
    }
    
    /// Check if dish is a Star
    pub fn is_star(&self) -> bool {
        matches!(self.category, MenuCategory::Star)
    }
    
    /// Check if dish needs attention (Puzzle or Dog)
    pub fn needs_attention(&self) -> bool {
        matches!(self.category, MenuCategory::Puzzle | MenuCategory::Dog)
    }
}

/// Menu Engineering Matrix - aggregated analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuEngineeringMatrix {
    /// Total number of dishes analyzed
    pub total_dishes: usize,
    
    /// Count by category
    pub stars: usize,
    pub plowhorses: usize,
    pub puzzles: usize,
    pub dogs: usize,
    
    /// Performance by dish
    pub dishes: Vec<DishPerformance>,
    
    /// Summary statistics
    pub avg_profit_margin: f64,
    pub total_revenue_cents: i64,
    pub total_profit_cents: i64,
}

impl MenuEngineeringMatrix {
    pub fn analyze(dishes: Vec<DishPerformance>) -> Self {
        let total_dishes = dishes.len();
        
        let stars = dishes.iter().filter(|d| d.category == MenuCategory::Star).count();
        let plowhorses = dishes.iter().filter(|d| d.category == MenuCategory::Plowhorse).count();
        let puzzles = dishes.iter().filter(|d| d.category == MenuCategory::Puzzle).count();
        let dogs = dishes.iter().filter(|d| d.category == MenuCategory::Dog).count();
        
        let avg_profit_margin = if total_dishes > 0 {
            dishes.iter().map(|d| d.profit_margin_percent).sum::<f64>() / total_dishes as f64
        } else {
            0.0
        };
        
        let total_revenue_cents = dishes.iter().map(|d| d.total_revenue_cents).sum();
        let total_profit_cents = dishes.iter().map(|d| d.total_profit_cents).sum();
        
        Self {
            total_dishes,
            stars,
            plowhorses,
            puzzles,
            dogs,
            dishes,
            avg_profit_margin,
            total_revenue_cents,
            total_profit_cents,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_menu_category_classification() {
        // Star: high margin + high popularity
        assert_eq!(
            MenuCategory::classify(70.0, 0.8),
            MenuCategory::Star
        );
        
        // Plowhorse: low margin + high popularity
        assert_eq!(
            MenuCategory::classify(40.0, 0.8),
            MenuCategory::Plowhorse
        );
        
        // Puzzle: high margin + low popularity
        assert_eq!(
            MenuCategory::classify(70.0, 0.3),
            MenuCategory::Puzzle
        );
        
        // Dog: low margin + low popularity
        assert_eq!(
            MenuCategory::classify(40.0, 0.3),
            MenuCategory::Dog
        );
    }
    
    #[test]
    fn test_dish_performance_creation() {
        let perf = DishPerformance::new(
            DishId::from_uuid(uuid::Uuid::new_v4()),
            "Test Dish".to_string(),
            75.0,   // high margin
            0.85,   // high popularity
            100,    // sales
            10000,  // revenue
            7500,   // profit
            0.5,    // cumulative share (ABC class B)
            crate::shared::Language::En,
        );
        
        assert!(perf.is_star());
        assert!(!perf.needs_attention());
        assert_eq!(perf.category, MenuCategory::Star);
        assert_eq!(perf.abc_class, AbcClass::A);
    }
}
