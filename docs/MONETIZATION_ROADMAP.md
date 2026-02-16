# 💰 Restaurant SaaS - Monetization Roadmap

**STATUS:** Main = Stable Production | Recipe V2 = Feature Branch  
**PRIORITY:** Subscription Revenue FIRST, then growth features

---

## 🎯 PHASE 1: SUBSCRIPTION FOUNDATION (Week 1-2)

### Database Schema

```sql
-- migrations/20260216000000_add_subscriptions.sql

CREATE TYPE subscription_plan AS ENUM ('starter', 'pro', 'enterprise');
CREATE TYPE subscription_status AS ENUM ('trialing', 'active', 'past_due', 'canceled', 'incomplete');

CREATE TABLE subscriptions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL UNIQUE REFERENCES tenants(id) ON DELETE CASCADE,
    
    -- Stripe data
    stripe_customer_id TEXT NOT NULL UNIQUE,
    stripe_subscription_id TEXT UNIQUE,
    
    -- Plan details
    plan subscription_plan NOT NULL DEFAULT 'starter',
    status subscription_status NOT NULL DEFAULT 'trialing',
    
    -- Trial
    trial_ends_at TIMESTAMPTZ,
    trial_days INTEGER DEFAULT 14,
    
    -- Billing period
    current_period_start TIMESTAMPTZ,
    current_period_end TIMESTAMPTZ,
    
    -- Metadata
    cancel_at_period_end BOOLEAN DEFAULT FALSE,
    canceled_at TIMESTAMPTZ,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_subscriptions_tenant_id ON subscriptions(tenant_id);
CREATE INDEX idx_subscriptions_stripe_customer ON subscriptions(stripe_customer_id);
CREATE INDEX idx_subscriptions_status ON subscriptions(status);

-- Add plan limits table
CREATE TABLE plan_limits (
    plan subscription_plan PRIMARY KEY,
    max_products INTEGER,  -- NULL = unlimited
    max_users INTEGER,
    ai_enabled BOOLEAN DEFAULT FALSE,
    reports_enabled BOOLEAN DEFAULT FALSE,
    priority_support BOOLEAN DEFAULT FALSE
);

INSERT INTO plan_limits VALUES
    ('starter', 200, 3, FALSE, FALSE, FALSE),
    ('pro', NULL, 10, TRUE, TRUE, FALSE),
    ('enterprise', NULL, NULL, TRUE, TRUE, TRUE);
```

### Domain Layer

**File:** `src/domain/subscription.rs`

```rust
use uuid::Uuid;
use time::OffsetDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionPlan {
    Starter,
    Pro,
    Enterprise,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionStatus {
    Trialing,
    Active,
    PastDue,
    Canceled,
    Incomplete,
}

pub struct Subscription {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub stripe_customer_id: String,
    pub stripe_subscription_id: Option<String>,
    pub plan: SubscriptionPlan,
    pub status: SubscriptionStatus,
    pub trial_ends_at: Option<OffsetDateTime>,
    pub current_period_end: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
}

impl Subscription {
    pub fn is_active(&self) -> bool {
        matches!(self.status, SubscriptionStatus::Active | SubscriptionStatus::Trialing)
    }
    
    pub fn is_trial_expired(&self) -> bool {
        if let Some(trial_end) = self.trial_ends_at {
            trial_end < OffsetDateTime::now_utc()
        } else {
            false
        }
    }
    
    pub fn can_access_feature(&self, feature: Feature) -> bool {
        if !self.is_active() || self.is_trial_expired() {
            return false;
        }
        
        match feature {
            Feature::BasicInventory => true, // All plans
            Feature::AIAssistant => matches!(self.plan, SubscriptionPlan::Pro | SubscriptionPlan::Enterprise),
            Feature::Reports => matches!(self.plan, SubscriptionPlan::Pro | SubscriptionPlan::Enterprise),
            Feature::PrioritySupport => matches!(self.plan, SubscriptionPlan::Enterprise),
        }
    }
}

pub enum Feature {
    BasicInventory,
    AIAssistant,
    Reports,
    PrioritySupport,
}
```

---

## 💳 PHASE 2: STRIPE INTEGRATION (Week 2-3)

### Dependencies

Add to `Cargo.toml`:
```toml
stripe = "0.27"
```

### Configuration

Add to `.env`:
```bash
STRIPE_SECRET_KEY=sk_test_...
STRIPE_PUBLISHABLE_KEY=pk_test_...
STRIPE_WEBHOOK_SECRET=whsec_...
STRIPE_PRICE_STARTER=price_...
STRIPE_PRICE_PRO=price_...
STRIPE_PRICE_ENTERPRISE=price_...
```

### BillingService

**File:** `src/application/billing_service.rs`

```rust
use stripe::{
    Client, CreateCustomer, CreateCheckoutSession, CheckoutSession,
    Customer, Subscription as StripeSubscription, Event, EventType,
};
use crate::domain::{Subscription, SubscriptionPlan, SubscriptionStatus};
use crate::shared::{AppResult, AppError, TenantId};

pub struct BillingService {
    stripe_client: Client,
    subscription_repo: Arc<dyn SubscriptionRepositoryTrait>,
}

impl BillingService {
    pub async fn create_checkout_session(
        &self,
        tenant_id: TenantId,
        plan: SubscriptionPlan,
        success_url: &str,
        cancel_url: &str,
    ) -> AppResult<CheckoutSession> {
        // Create or get Stripe customer
        let customer = self.get_or_create_customer(tenant_id).await?;
        
        // Create checkout session
        let price_id = self.get_price_id_for_plan(plan);
        
        let session = CreateCheckoutSession::new()
            .customer(&customer.id)
            .mode(stripe::CheckoutSessionMode::Subscription)
            .line_items(vec![
                stripe::CreateCheckoutSessionLineItems {
                    price: Some(price_id.to_string()),
                    quantity: Some(1),
                    ..Default::default()
                }
            ])
            .success_url(success_url)
            .cancel_url(cancel_url)
            .send_async(&self.stripe_client)
            .await?;
        
        Ok(session)
    }
    
    pub async fn handle_webhook(&self, payload: &[u8], signature: &str) -> AppResult<()> {
        let event = stripe::Webhook::construct_event(payload, signature, &self.webhook_secret)?;
        
        match event.type_ {
            EventType::CheckoutSessionCompleted => {
                self.handle_checkout_completed(event).await?;
            }
            EventType::CustomerSubscriptionUpdated => {
                self.handle_subscription_updated(event).await?;
            }
            EventType::CustomerSubscriptionDeleted => {
                self.handle_subscription_deleted(event).await?;
            }
            EventType::InvoicePaymentSucceeded => {
                self.handle_payment_succeeded(event).await?;
            }
            EventType::InvoicePaymentFailed => {
                self.handle_payment_failed(event).await?;
            }
            _ => {} // Ignore other events
        }
        
        Ok(())
    }
}
```

---

## 🔐 PHASE 3: MIDDLEWARE PROTECTION (Week 3)

### SubscriptionGuard Middleware

**File:** `src/interfaces/http/middleware/subscription_guard.rs`

```rust
use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use crate::application::BillingService;
use crate::interfaces::http::middleware::AuthUser;

pub async fn subscription_guard<B>(
    State(billing_service): State<Arc<BillingService>>,
    auth_user: AuthUser,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    // Get subscription status
    let subscription = billing_service
        .get_subscription(auth_user.tenant_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Check if active and not expired
    if !subscription.is_active() || subscription.is_trial_expired() {
        return Err(StatusCode::PAYMENT_REQUIRED);
    }
    
    Ok(next.run(request).await)
}
```

### Apply to Protected Routes

```rust
// In routes.rs
let protected_routes = Router::new()
    .route("/inventory/products", post(add_product))
    .route("/recipes/v2", post(create_recipe))
    .layer(middleware::from_fn_with_state(
        billing_service.clone(),
        subscription_guard
    ));
```

---

## 📊 PHASE 4: PRICING PLANS (Week 4)

### Pricing Structure

| Feature | Starter ($29/mo) | Pro ($99/mo) | Enterprise ($299/mo) |
|---------|------------------|--------------|---------------------|
| Products | 200 | Unlimited | Unlimited |
| Users | 3 | 10 | Unlimited |
| AI Assistant | ❌ | ✅ | ✅ |
| Reports | ❌ | ✅ | ✅ |
| Support | Email | Priority | Dedicated |
| Trial | 14 days | 14 days | 30 days |

### Frontend Integration

```typescript
// Next.js checkout page
const handleSubscribe = async (plan: 'starter' | 'pro' | 'enterprise') => {
  const response = await fetch('/api/billing/create-checkout', {
    method: 'POST',
    body: JSON.stringify({ plan }),
    headers: { 'Authorization': `Bearer ${token}` }
  });
  
  const { url } = await response.json();
  window.location.href = url; // Redirect to Stripe Checkout
};
```

---

## 🎯 SUCCESS METRICS

### Week 1-2: Foundation
- ✅ Subscriptions table created
- ✅ Domain models implemented
- ✅ Repository layer done

### Week 2-3: Stripe
- ✅ Stripe SDK integrated
- ✅ Checkout flow working
- ✅ Webhook handling verified

### Week 3-4: Protection
- ✅ Middleware guards routes
- ✅ Trial expiration enforced
- ✅ Plan limits respected

### Week 4+: Growth
- ✅ First paying customer 💰
- ✅ $MRR > $0
- ✅ Churn < 5%

---

## 🚀 DEPLOYMENT CHECKLIST

### Environment Variables
```bash
# Production .env
STRIPE_SECRET_KEY=sk_live_...
STRIPE_PUBLISHABLE_KEY=pk_live_...
STRIPE_WEBHOOK_SECRET=whsec_...
```

### Stripe Dashboard Setup
1. Create products (Starter, Pro, Enterprise)
2. Create prices (monthly recurring)
3. Configure webhook endpoint: `https://your-domain.com/api/billing/webhook`
4. Enable events:
   - `checkout.session.completed`
   - `customer.subscription.updated`
   - `customer.subscription.deleted`
   - `invoice.payment_succeeded`
   - `invoice.payment_failed`

---

## 📝 NOTES

**MVP = Monetization + Core Value**

Current MVP features that SELL:
- ✅ Multi-tenant isolation
- ✅ Inventory tracking
- ✅ Cost calculation
- ✅ AI analysis (Pro+)
- ✅ Admin panel

**NOT MVP:**
- ❌ Recipe V2 (nice-to-have)
- ❌ Public recipe feed (growth)
- ❌ Social features (future)

**Rule:** If it doesn't contribute to revenue → feature branch only.

---

**Last Updated:** 2026-02-15  
**Next Review:** After first paying customer
