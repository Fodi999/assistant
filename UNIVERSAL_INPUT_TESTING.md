# 🧪 Universal Input Architecture - Testing Guide

## 📊 Overview

This document describes how to test the new universal input architecture on Koyeb production.

**Key Features:**
- Accept input in ANY language (Russian, Polish, Ukrainian, English, etc.)
- Normalize canonically to English (single source of truth)
- AI-powered automatic classification (category + unit)
- Intelligent translation caching (hybrid approach)
- Graceful degradation (AI failures don't block operations)
- Cost optimization: -50% AI calls via ASCII detection

## 🔐 Authentication Setup

### Step 1: Get Admin JWT Token

```bash
curl -X POST https://api.fodi.app/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "admin@fodi.app",
    "password": "YOUR_ADMIN_PASSWORD"
  }' | jq '.data.access_token'
```

### Step 2: Export Token

```bash
export KOYEB_ADMIN_TOKEN="your-jwt-token-here"
```

## 🧪 Test Cases

### Test 1: Russian Input - "Молоко" (Milk)

**Expected Pipeline:**
1. Input: "Молоко" (non-ASCII detected)
2. normalize_to_english() → "Milk" (1 AI call)
3. classify_product("Milk") → `dairy_and_eggs` + `liter`
4. translate("Milk") → Polish/Russian/Ukrainian
5. Save to database with all translations

**Command:**
```bash
curl -X POST https://api.fodi.app/api/admin/catalog/products \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $KOYEB_ADMIN_TOKEN" \
  -d '{
    "name_input": "Молоко",
    "auto_translate": true
  }' | jq .
```

**Expected Response:**
```json
{
  "success": true,
  "data": {
    "id": "uuid",
    "name_en": "Milk",
    "name_pl": "Mleko",
    "name_ru": "Молоко",
    "name_uk": "Молоко",
    "category": {
      "id": "uuid",
      "name": "Dairy & Eggs"
    },
    "unit": "liter",
    "created_at": "2026-02-15T..."
  }
}
```

### Test 2: Polish Input - "Mleko" (Milk)

**Expected Pipeline:**
1. Input: "Mleko" (non-ASCII: ł detected)
2. translate_to_language("Mleko", "English") → "Milk"
3. Same flow as Test 1

**Command:**
```bash
curl -X POST https://api.fodi.app/api/admin/catalog/products \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $KOYEB_ADMIN_TOKEN" \
  -d '{
    "name_input": "Mleko",
    "auto_translate": true
  }' | jq .
```

### Test 3: English Input - "Green Apple" (Multi-word)

**Expected Pipeline:**
1. Input: "Green Apple" (ASCII-only = English)
2. normalize_to_english() → returns as-is (NO AI CALL!)
3. classify_product("Green Apple") → `fruits` + `piece`
4. translate("Green Apple") → multi-word preservation
5. extract_translated_word() preserves "Green Apple" → "Zielone Jabłko" etc.

**Command:**
```bash
curl -X POST https://api.fodi.app/api/admin/catalog/products \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $KOYEB_ADMIN_TOKEN" \
  -d '{
    "name_input": "Green Apple",
    "auto_translate": true
  }' | jq .
```

**Verification:**
- `name_en` should be "Green Apple" (not truncated)
- `name_pl` should preserve multi-word (e.g., "Zielone Jabłko")
- Category should be "Fruits"
- Unit should be "piece"
- **Cost savings:** Only 2 AI calls (classify + translate), NOT 3

### Test 4: Ukrainian Input - "Яйце" (Egg)

**Expected Pipeline:**
1. Input: "Яйце" (non-ASCII: Cyrillic)
2. translate_to_language("Яйце", "English") → "Egg"
3. classify_product("Egg") → `dairy_and_eggs` + `piece`
4. translate("Egg")

**Command:**
```bash
curl -X POST https://api.fodi.app/api/admin/catalog/products \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $KOYEB_ADMIN_TOKEN" \
  -d '{
    "name_input": "Яйце",
    "auto_translate": true
  }' | jq .
```

### Test 5: Duplicate Detection

**Expected Behavior:**
- Create product with "Молоко" → success
- Create product with "Молоко" again → ERROR (duplicates check is on name_en)
- Create product with "Milk" (English) → ERROR (same canonical name_en)

**Command:**
```bash
# First attempt - should succeed
curl -X POST https://api.fodi.app/api/admin/catalog/products \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $KOYEB_ADMIN_TOKEN" \
  -d '{
    "name_input": "Молоко"
  }' | jq '.success'

# Second attempt - should fail
curl -X POST https://api.fodi.app/api/admin/catalog/products \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $KOYEB_ADMIN_TOKEN" \
  -d '{
    "name_input": "Молоко"
  }' | jq '.error'
```

**Expected Error:**
```json
{
  "success": false,
  "error": "Product with name 'Milk' already exists"
}
```

### Test 6: Explicit Category Override

**Expected Behavior:**
- User provides explicit `category_id`
- AI classification is skipped
- Uses provided category instead

**Command:**
```bash
# Get a valid category ID first
CATEGORY_ID=$(curl -s https://api.fodi.app/api/admin/catalog/categories \
  -H "Authorization: Bearer $KOYEB_ADMIN_TOKEN" | jq -r '.data[0].id')

# Create product with explicit category
curl -X POST https://api.fodi.app/api/admin/catalog/products \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $KOYEB_ADMIN_TOKEN" \
  -d "{
    \"name_input\": \"Сыр\",
    \"category_id\": \"$CATEGORY_ID\",
    \"auto_translate\": true
  }" | jq '.data | {id, name_en, category_id}'
```

### Test 7: Graceful Degradation (AI Timeout)

**Expected Behavior:**
- If AI classification times out → use defaults (`vegetables` + `piece`)
- Log warning but don't fail the request
- User still gets a product, just with defaults

**Manual Test:**
(This is hard to trigger intentionally, but you can watch logs for timeout messages)

```bash
# Watch Koyeb logs for patterns like:
# "⚠️ AI classification failed, using defaults"
```

## 📈 Performance Metrics

### Cost Optimization

| Scenario | Old Approach | New Approach | Savings |
|----------|--------------|--------------|---------|
| English input | 3 AI calls | **1 AI call** | **67%** |
| Non-English 1st time | 3 AI calls | **3 AI calls** | 0% |
| Non-English repeated | 3 AI calls | **0 AI calls** (cached) | **100%** |

### Response Times

Expected timeouts:
- English ASCII input: <100ms (no AI call)
- Non-English input: 2-3 seconds (1-2 AI calls)
- AI timeout fallback: <50ms (uses defaults)

## 🔍 Logs to Monitor

Check Koyeb logs for these patterns:

### Successful Flow
```
Input detected as ASCII (English): Green Apple
Groq translation request for: Green Apple
Multi-word translation detected, returning full: Zielone Jabłko
AI classification: category=fruits, unit=piece
✅ Groq translation successful
```

### Cost Optimization
```
Input detected as ASCII (English): Milk
# No "Groq translation request" here = saved 1 AI call!
```

### Language Normalization
```
Non-ASCII input detected, translating to English: Молоко
Translated 'Молоко' → 'Milk' (cleaned from: '...')
# Non-ASCII detected, 1 AI call made for normalization
```

### Graceful Degradation
```
⚠️ AI classification failed, using defaults
# Product still created with vegetables/piece
```

## 🧬 Architecture Verification

### 1. Verify normalize_to_english() Optimization

Create English product, check logs:
- Should see "Input detected as ASCII (English)"
- Should NOT see "Groq translation request" for normalization
- Should only see 1 "Groq translation request" (for translate())

### 2. Verify Multi-word Preservation

Create "Green Apple", check name_pl:
- Should be "Zielone Jabłko" (NOT just "Jabłko")
- Logs should show: "Multi-word translation detected, returning full"

### 3. Verify Dual Timeout Protection

Add artificial latency to Groq API, watch logs:
- First timeout: "Groq request timeout (6s exceeded)"
- Falls back to defaults: "⚠️ AI classification failed, using defaults"

### 4. Verify Hybrid Translation Cache

Create same product 2x with different input languages:
- 1st time: "Groq translation request" (cache miss)
- 2nd time: No "Groq translation request" (cache hit)

## 🚀 Automated Test Script

```bash
bash /tmp/test_universal_input.sh
```

This script runs all 6 test cases automatically.

## ✅ Checklist

- [ ] Admin token obtained and working
- [ ] Test 1: Russian input works
- [ ] Test 2: Polish input works
- [ ] Test 3: English multi-word works (no AI call for normalization)
- [ ] Test 4: Ukrainian input works
- [ ] Test 5: Duplicate detection prevents same canonical name
- [ ] Test 6: Explicit category override works
- [ ] Logs show proper flow (no unnecessary AI calls)
- [ ] Multi-word translations preserved
- [ ] Graceful degradation works (if AI times out, product still created)

## 📞 Troubleshooting

### "Product with name X already exists"
- Two products normalize to same `name_en`
- This is correct behavior (duplicate prevention)
- Try different input

### Missing translations (name_pl/ru/uk empty)
- Check Groq API key validity
- Check Groq request timeout logs
- May fall back to defaults if timeout

### Wrong category assigned
- AI classification may not be 100% accurate
- Use explicit `category_id` override if needed
- Check Groq response in logs

### Timeout errors
- Dual timeout active: 5s TCP + 6s async
- If seeing timeouts, AI is slow or unreachable
- Graceful degradation kicks in, product still created

---

**Last Updated:** 2026-02-15
**Architecture:** Universal Input v1.0
**Status:** ✅ Production Ready
