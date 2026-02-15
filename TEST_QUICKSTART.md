# 🚀 Quick Start - Testing Universal Input Architecture

## Setup (5 minutes)

### 1. Get Admin JWT Token

```bash
curl -X POST https://api.fodi.app/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "admin@fodi.app",
    "password": "YOUR_PASSWORD"
  }' | jq '.data.access_token'
```

Replace `YOUR_PASSWORD` with your actual admin password.

**Output:**
```
"eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
```

### 2. Export Token

```bash
export KOYEB_ADMIN_TOKEN="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
```

Copy the token from step 1 and paste it here.

### 3. Run Tests

```bash
bash test_universal_input.sh
```

---

## ✨ What Gets Tested

### Test 1: Russian Input
```bash
Input: "Молоко" (Milk in Russian)
↓
normalize_to_english() → "Milk"
↓
classify_product("Milk") → dairy_and_eggs + liter
↓
translate("Milk") → Polish/Russian/Ukrainian
↓
✅ Product saved to database
```

### Test 2: Polish Input
```bash
Input: "Mleko" (Milk in Polish)
↓
Normalizes to "Milk"
↓
Duplicate detection: ERROR (Milk already exists)
↓
✅ Duplicate prevention working
```

### Test 3: English Multi-word
```bash
Input: "Green Apple"
↓
ASCII-only optimization: NO AI CALL (saves cost!)
↓
Classify as Fruits
↓
Multi-word preserved: "Zielone Jabłko" (not truncated)
↓
✅ Multi-word preservation working
```

### Test 4: Ukrainian Input
```bash
Input: "Яйце" (Egg in Ukrainian)
↓
Translates to English
↓
Classification applied
↓
✅ Cyrillic input handled
```

### Test 5: Duplicate Detection
```bash
Create "Milk" → ✅ Success
Create "Milk" again → ❌ Error (duplicate)
Create "Молоко" → ❌ Error (normalizes to Milk)
↓
✅ Prevents duplicate canonical names
```

### Test 6: Category Override
```bash
Input: "Cheese" with explicit category_id
↓
Ignores AI classification
↓
Uses provided category
↓
✅ Manual override working
```

### Test 7: Edge Cases
- Empty input → Rejected ✅
- Very long input (200+ chars) → Rejected ✅

---

## 📊 Expected Results

All tests should show:
```
✅ Test 1: Russian Input 'Молоко' - PASSED
✅ Test 2: Polish Input 'Mleko' - PASSED
✅ Test 3: English 'Green Apple' - PASSED
✅ Test 4: Ukrainian Input 'Яйце' - PASSED
✅ Test 5: Duplicate Detection - PASSED
✅ Test 6: Category Override - PASSED
✅ Test 7: Edge Cases - PASSED

📊 Test Summary
Total Tests:  7
Passed:       7
Failed:       0

🎉 All tests passed!
```

---

## 🔍 Manual Testing

### Test Russian Input Directly

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
    "id": "uuid-here",
    "name_en": "Milk",
    "name_pl": "Mleko",
    "name_ru": "Молоко",
    "name_uk": "Молоко",
    "category": {
      "name": "Dairy & Eggs"
    },
    "unit": "liter",
    "created_at": "2026-02-15T..."
  }
}
```

### Check Product was Created

```bash
curl -X GET "https://api.fodi.app/api/admin/catalog/products?search=Milk" \
  -H "Authorization: Bearer $KOYEB_ADMIN_TOKEN" | jq .
```

### Watch Logs on Koyeb

Go to: https://app.koyeb.com → Select your service → Logs

Watch for these patterns:
```
✅ Input detected as ASCII (English): Green Apple
   (No "Groq translation request" here = saved AI cost!)

✅ Non-ASCII input detected, translating to English: Молоко
   Translated 'Молоко' → 'Milk'

✅ AI classification: category=dairy_and_eggs, unit=liter

✅ Multi-word translation detected, returning full: Zielone Jabłko
```

---

## ⚡ Performance Expectations

| Scenario | Time | AI Calls |
|----------|------|----------|
| English input | <100ms | 0 (detection) + 1 (classify) = 1 |
| Non-English 1st time | 2-3s | 1 (normalize) + 1 (classify) = 2 |
| Non-English cached | <100ms | 0 (cache hit) |
| Classification timeout | <50ms | 0 (fallback) |

---

## 🛠️ Troubleshooting

### "Command not found: jq"

Install jq:
```bash
# macOS
brew install jq

# Ubuntu/Debian
sudo apt-get install jq

# Or use alternative without jq
curl ... | python3 -m json.tool
```

### "No ADMIN_TOKEN provided"

Set the token:
```bash
export KOYEB_ADMIN_TOKEN="your-token-here"
bash test_universal_input.sh
```

### "API not responding"

Check:
1. Koyeb service is running
2. Network connectivity
3. Token is valid (try login again)

### "Product with name X already exists"

This is correct! Duplicate detection is working.

Try a different product:
```bash
curl -X POST https://api.fodi.app/api/admin/catalog/products \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $KOYEB_ADMIN_TOKEN" \
  -d '{"name_input":"Cheese","auto_translate":true}' | jq .
```

### "AI classification failed, using defaults"

This is OK! Graceful degradation is working.
- Category: vegetables (default)
- Unit: piece (default)

Check Koyeb logs to see why AI failed (timeout, API issue, etc.)

---

## 📚 Documentation

- **UNIVERSAL_INPUT_TESTING.md** - Detailed test procedures
- **UNIVERSAL_INPUT_COMPLETE.md** - Full implementation report
- **test_universal_input.sh** - Automated test suite (this file)

---

## ✅ Success Checklist

- [ ] Token obtained and exported
- [ ] Tests run successfully
- [ ] Russian input test passed
- [ ] English optimization verified (no AI call for normalization)
- [ ] Multi-word preservation working
- [ ] Duplicate detection active
- [ ] All 7 tests passed
- [ ] Logs show expected patterns

---

## 🎯 What This Tests

✅ **Universal Input:** Any language accepted  
✅ **Normalization:** Non-ASCII → English  
✅ **Cost Optimization:** ASCII check saves AI calls  
✅ **Classification:** AI determines category + unit  
✅ **Translation:** Hybrid cache + Groq  
✅ **Multi-word:** "Green Apple" not truncated to "Apple"  
✅ **Duplicate Prevention:** Same canonical name blocked  
✅ **Graceful Degradation:** AI failure doesn't break create  
✅ **Timeout Protection:** Dual timeout (5s + 6s)  
✅ **Retry Logic:** 2 attempts on AI failure  

---

## 🚀 Production Ready?

If all tests pass → **YES! Ready for production** ✅

---

*Last Updated: 2026-02-15*
*Architecture: Universal Input v1.0*
*Status: Production Ready*
