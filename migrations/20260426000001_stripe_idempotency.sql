-- Stripe webhook idempotency.
--
-- A single Stripe `checkout.session.completed` event can arrive multiple
-- times (Stripe retries on non-2xx, network blips, replay). To make the
-- webhook safe to call repeatedly we enforce uniqueness on receipt_id
-- when present. Backfilled rows where receipt_id IS NULL are unaffected.

CREATE UNIQUE INDEX IF NOT EXISTS idx_usage_purchases_receipt_unique
    ON usage_purchases (receipt_id)
    WHERE receipt_id IS NOT NULL;
