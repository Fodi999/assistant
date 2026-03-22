-- Switch uniqueness from (intent_type, entity_a, entity_b, locale) to (slug, locale)
-- 
-- Reason: one product can have MULTIPLE sub-intent pages:
--   "is-salmon-healthy", "salmon-calories", "salmon-protein"
-- All are intent_type=question, entity_a=salmon — but different slugs.

-- Drop old unique index (entity combo)
DROP INDEX IF EXISTS uq_intent_pages_content;

-- Create new unique index on (slug, locale)
CREATE UNIQUE INDEX uq_intent_pages_slug_locale ON intent_pages (slug, locale);
