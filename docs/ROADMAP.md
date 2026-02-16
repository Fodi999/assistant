# Restaurant Backend - Roadmap

## ✅ Completed (Phase 1)

### Core Foundation
- [x] User authentication & multi-tenancy
- [x] Catalog ingredients (99 items, 15 categories)
- [x] Inventory management with expiration tracking
- [x] Recipe creation with cost calculation
- [x] **Dish/Menu domain with financial analysis** ⭐
- [x] **Guided Assistant with FSM** (Start → Inventory → Recipes → Dishes → Report)
- [x] **"Момент вау" - Instant financial insights** 💰

### Financial Analysis (MVP)
- [x] Real-time profit calculation
- [x] Profit margin percentage (target: ≥60%)
- [x] Food cost percentage (target: ≤35%)
- [x] Multi-language financial warnings (PL/EN/UK/RU)
- [x] Integration test (565 lines, full E2E coverage)

---

## 🚀 Next Steps (Phase 2)

### 🥇 Priority 1: Menu Engineering Analysis
**Goal:** Automatically classify dishes into 4 categories based on profitability and popularity

#### Implementation
```rust
pub enum MenuCategory {
    Star,       // High margin + High sales → Keep & promote
    Plowhorse,  // Low margin + High sales → Optimize pricing
    Puzzle,     // High margin + Low sales → Marketing opportunity
    Dog,        // Low margin + Low sales → Remove from menu
}

pub struct DishPerformance {
    dish_id: DishId,
    category: MenuCategory,
    profit_margin_percent: f64,
    sales_volume: u32,          // NEW: track sales
    contribution_margin: i64,    // profit * volume
    recommendation: String,
}
```

#### Database Changes
```sql
-- Track dish sales
CREATE TABLE dish_sales (
    id UUID PRIMARY KEY,
    dish_id UUID REFERENCES dishes(id),
    tenant_id UUID REFERENCES tenants(id),
    sale_date DATE NOT NULL,
    quantity INTEGER NOT NULL,
    total_price_cents BIGINT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT now()
);

CREATE INDEX idx_dish_sales_dish ON dish_sales(dish_id, sale_date);
CREATE INDEX idx_dish_sales_tenant ON dish_sales(tenant_id, sale_date);
```

#### API Endpoints
- `GET /api/menu-engineering/analysis` - Full menu engineering matrix
- `GET /api/dishes/:id/performance` - Individual dish performance
- `GET /api/menu-engineering/recommendations` - AI-powered suggestions

**Estimated time:** 2-3 days  
**Business value:** High - helps owners optimize menu profitability

---

### 🥈 Priority 2: P&L (Profit & Loss) Reports
**Goal:** Financial reporting for day/week/month periods

#### Features
- Total revenue by period
- Total costs (COGS - Cost of Goods Sold)
- Gross profit & margin
- Waste tracking (expired inventory value)
- Top performing dishes
- Food cost % trending

#### Implementation
```rust
pub struct PLReport {
    period: DateRange,
    revenue_cents: i64,
    cogs_cents: i64,              // Cost of Goods Sold
    gross_profit_cents: i64,      // revenue - cogs
    gross_margin_percent: f64,    // (gross_profit / revenue) * 100
    waste_cents: i64,             // expired inventory value
    top_dishes: Vec<DishSummary>,
}
```

#### Database Changes
```sql
-- Aggregate sales data for reporting
CREATE MATERIALIZED VIEW daily_sales_summary AS
SELECT 
    tenant_id,
    sale_date,
    SUM(total_price_cents) as revenue_cents,
    SUM(quantity) as items_sold,
    COUNT(DISTINCT dish_id) as unique_dishes
FROM dish_sales
GROUP BY tenant_id, sale_date;

-- Refresh daily via cron job
CREATE INDEX idx_daily_sales_tenant_date ON daily_sales_summary(tenant_id, sale_date);
```

**Estimated time:** 3-4 days  
**Business value:** Critical - owners need visibility into profitability

---

### 🥉 Priority 3: Smart Pricing Assistant
**Goal:** AI-powered pricing recommendations

#### Features
- "If you increase price by X PLN → margin becomes Y%"
- Market positioning analysis
- Competitor pricing suggestions (future: external data integration)
- Dynamic pricing based on demand

#### Implementation
```rust
pub struct PricingSuggestion {
    dish_id: DishId,
    current_price_cents: i32,
    current_margin_percent: f64,
    suggested_price_cents: i32,
    target_margin_percent: f64,
    impact: PricingImpact,
}

pub struct PricingImpact {
    margin_increase: f64,
    estimated_demand_change: f64,  // elasticity
    projected_revenue_change: i64,
}
```

#### API Endpoints
- `POST /api/pricing/simulate` - Simulate price change impact
- `GET /api/pricing/suggestions/:dish_id` - Get pricing recommendations
- `POST /api/pricing/optimize-menu` - Optimize all dish prices

**Estimated time:** 4-5 days  
**Business value:** Medium-High - helps maximize revenue

---

### 🧱 Priority 4: Performance Optimization
**Goal:** Fast financial calculations for real-time UI

#### Techniques
1. **Materialized Views** for pre-calculated costs
2. **Redis Cache** for frequently accessed data
3. **Recursive recipe cost caching** (for semi-finished products)
4. **Database indexes** on critical queries

#### Implementation
```sql
-- Cache recipe costs (update on ingredient price change)
CREATE MATERIALIZED VIEW recipe_costs_cache AS
SELECT 
    r.id as recipe_id,
    r.tenant_id,
    SUM(ri.quantity * ip.price_per_unit_cents) as total_cost_cents,
    SUM(ri.quantity * ip.price_per_unit_cents) / r.servings as cost_per_serving_cents,
    MAX(ip.updated_at) as last_ingredient_update
FROM recipes r
JOIN recipe_ingredients ri ON r.id = ri.recipe_id
JOIN inventory_products ip ON ri.catalog_ingredient_id = ip.catalog_ingredient_id
GROUP BY r.id, r.tenant_id, r.servings;

CREATE UNIQUE INDEX idx_recipe_costs_cache ON recipe_costs_cache(recipe_id);
```

#### Redis Strategy
```rust
// Cache hot data (TTL: 5 minutes)
cache_key = format!("dish:{}:financials", dish_id);
redis.setex(cache_key, 300, financials.to_json());
```

**Estimated time:** 3-4 days  
**Business value:** Medium - improves UX, enables real-time features

---

### 🤖 Priority 5: AI Insights (Optional)
**Goal:** Explain financial metrics in natural language

#### Features
- "Why is this dish unprofitable?" → AI analysis
- "Which ingredients drive high food cost?" → Top 3 expensive items
- "How to improve margin?" → Actionable suggestions
- "Compare this dish to competitors" → Market analysis

#### Implementation
```rust
pub struct AIInsight {
    question: String,
    answer: String,
    confidence: f64,
    recommendations: Vec<String>,
    data_sources: Vec<String>,
}

// Integration with LLM (OpenAI, Anthropic, or local model)
pub async fn generate_insight(
    dish_id: DishId,
    question: String,
    context: DishFinancialContext,
) -> AppResult<AIInsight> {
    let prompt = format!(
        "Dish: {}\nCost: {} PLN\nSelling: {} PLN\nMargin: {:.1}%\n\nQuestion: {}",
        context.name, context.cost, context.selling_price, 
        context.margin, question
    );
    
    let response = llm_client.complete(prompt).await?;
    Ok(AIInsight::parse(response))
}
```

**Estimated time:** 5-7 days (depends on LLM integration complexity)  
**Business value:** Low-Medium - nice-to-have, differentiator

---

## 🎯 Success Metrics

### Phase 1 (Completed) ✅
- [x] Recipe cost calculation accuracy: 100%
- [x] Dish financial analysis: Profit, Margin, Food Cost
- [x] End-to-end test coverage: 565 lines, 9 test scenarios
- [x] Financial warnings working (low margin, high food cost)

### Phase 2 (Target)
- [ ] Menu Engineering: 4-category classification
- [ ] P&L Reports: Daily/Weekly/Monthly
- [ ] Smart Pricing: 3+ suggestion types
- [ ] Performance: <100ms response time for financial queries
- [ ] AI Insights: 80%+ user satisfaction

---

## 📦 Technical Debt & Improvements

### High Priority
- [ ] Add catalog ingredient seeding for tests (currently hardcoded UUIDs)
- [ ] Implement recursive cost calculation for semi-finished products (recipe_components)
- [ ] Add dish HTTP API endpoints (currently only via Assistant)
- [ ] Database connection pooling optimization
- [ ] Add logging/tracing for all financial calculations

### Medium Priority
- [ ] WebSocket for real-time inventory updates
- [ ] Batch operations for bulk dish creation
- [ ] Export P&L reports (PDF, Excel)
- [ ] Multi-currency support
- [ ] Tax calculation integration

### Low Priority
- [ ] GraphQL API alternative
- [ ] Mobile app (React Native)
- [ ] Barcode scanner for inventory
- [ ] Integration with POS systems

---

## 🚢 Deployment Strategy

### Current Setup
- **Local Development:** PostgreSQL on localhost
- **Production:** Neon (serverless PostgreSQL)
- **Config:** `.env` file with DATABASE_URL switching

### Recommended Next Steps
1. **CI/CD Pipeline**
   - GitHub Actions for automated testing
   - Run `assistant_dish_test.sh` on every PR
   - Automated deployment to staging

2. **Infrastructure**
   - Docker containerization
   - Kubernetes for scaling (optional)
   - Redis for caching
   - Monitoring (Grafana, Prometheus)

3. **Database**
   - Automated backups (daily)
   - Read replicas for reporting queries
   - Connection pooling (PgBouncer)

---

## 📚 Documentation Needed

- [ ] API documentation (OpenAPI/Swagger)
- [ ] Database schema diagram
- [ ] FSM state machine documentation
- [ ] Financial formulas reference
- [ ] Deployment guide
- [ ] Contributing guide

---

## 🎉 Conclusion

**Current State:** MVP complete with working financial analysis  
**Next Milestone:** Menu Engineering + P&L Reports (2-3 weeks)  
**Long-term Vision:** AI-powered restaurant management platform

**Contact:** [Your contact info]  
**Last Updated:** February 7, 2026
