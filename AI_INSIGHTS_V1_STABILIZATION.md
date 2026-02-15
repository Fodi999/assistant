# AI Insights V1 Stabilization Checklist

## ✅ DONE (Tested & Working)

- [x] Database schema created
- [x] Migrations applied to production
- [x] Services initialized
- [x] .env loading fixed (dotenvy::dotenv())
- [x] Recipe creation works
- [x] AI insights generation works (2.1s average)
- [x] JSONB storage works
- [x] Groq API integration works
- [x] Basic validation logic works
- [x] Suggestion generation works
- [x] Feasibility score calculation works

**Test Evidence**: Recipe `da49b9f0-6ad1-49d6-ab2e-715f2f815b60` has insights with 6 steps, 85/100 score

---

## 🔍 TO TEST (Critical)

### 1. Caching Behavior
```bash
# Test cache hit
RECIPE_ID="da49b9f0-6ad1-49d6-ab2e-715f2f815b60"
TOKEN="<your_token>"

# First call (should be cached already)
time curl -s "http://localhost:8000/api/recipes/v2/$RECIPE_ID/insights/ru" \
  -H "Authorization: Bearer $TOKEN" > /dev/null

# Expected: <100ms (cache hit)
```

**Success Criteria**: Response time <100ms

---

### 2. Refresh Endpoint
```bash
# Force regeneration
curl -X POST "http://localhost:8000/api/recipes/v2/$RECIPE_ID/insights/ru/refresh" \
  -H "Authorization: Bearer $TOKEN" | jq '.'

# Expected: 201 status, new insights with different timestamps
```

**Success Criteria**:
- Returns 201 Created
- Generation time ~2-3 seconds
- New `updated_at` timestamp
- Different `id` (or same id with new data)

---

### 3. Get All Languages
```bash
# Get all insights for recipe
curl "http://localhost:8000/api/recipes/v2/$RECIPE_ID/insights" \
  -H "Authorization: Bearer $TOKEN" | jq '.'

# Expected: Array of insights (currently only ["ru"])
```

**Success Criteria**: Returns array with 1 element (ru)

---

### 4. Error Handling

#### Invalid Recipe ID
```bash
curl "http://localhost:8000/api/recipes/v2/00000000-0000-0000-0000-000000000000/insights/ru" \
  -H "Authorization: Bearer $TOKEN"

# Expected: 404 Not Found
```

#### Invalid Language
```bash
curl "http://localhost:8000/api/recipes/v2/$RECIPE_ID/insights/zz" \
  -H "Authorization: Bearer $TOKEN"

# Expected: 400 Bad Request or 422 Unprocessable Entity
```

#### Unauthorized
```bash
curl "http://localhost:8000/api/recipes/v2/$RECIPE_ID/insights/ru"

# Expected: 401 Unauthorized
```

---

### 5. Edge Cases

#### Empty Instructions
```bash
curl -X POST "http://localhost:8000/api/recipes/v2" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Test Empty",
    "instructions": "",
    "language": "ru",
    "servings": 1,
    "ingredients": []
  }'

# Then try to generate insights
# Expected: AI should return validation error or minimal insights
```

#### Very Long Instructions (>5000 chars)
```bash
# Create recipe with 5000+ char instructions
# Expected: Should handle or return error about length
```

#### Wrong Language Instructions
```bash
# Create recipe with language="ru" but instructions in English
# Expected: AI should detect mismatch in validation.warnings
```

---

## 🐛 KNOWN ISSUES

### 1. Admin Catalog Endpoints Hang
**Status**: 🔴 Blocker for full testing
**Issue**: `/api/admin/catalog/ingredients` never returns
**Workaround**: Create recipes without ingredients
**Priority**: High - blocks full integration testing

**Debug Steps**:
1. Check middleware execution order
2. Check admin auth validation
3. Test direct DB query: `SELECT * FROM catalog_ingredients LIMIT 1;`
4. Add debug logs to admin_catalog handler
5. Check if pooler connection has timeout issues

---

### 2. Time Format Not Standard
**Status**: ⚠️ Warning - needs frontend handling
**Issue**: `created_at` returns `[2026, 46, 21, 38, 31, 798332000, 0, 0, 0]` instead of ISO 8601
**Impact**: Frontend needs custom parser
**Priority**: Medium - works but not ideal

**Fix Options**:
- Option A: Change domain to use ISO 8601 strings
- Option B: Document format for frontend
- Option C: Add DTO layer to convert

---

### 3. AI Detected Ingredients Not in Recipe
**Status**: ⚠️ Feature or Bug?
**Issue**: AI extracts ingredients from instructions text, even if `recipe.ingredients = []`
**Impact**: Might confuse cost calculations
**Priority**: Low - might be desired behavior

**Example**:
```json
{
  "recipe": {
    "ingredients": []
  },
  "insights": {
    "steps": [{
      "ingredients_used": ["свекла", "морковь", "капуста"]
    }]
  }
}
```

**Decision Needed**: Is this good or bad?
- ✅ Good: AI helps detect missing ingredients
- ❌ Bad: Inconsistent with recipe data

---

## 📊 Performance Testing Plan

### Load Test Script
```bash
#!/bin/bash
# Generate insights for 10 recipes and measure performance

for i in {1..10}; do
  # Create recipe
  RECIPE=$(curl -s -X POST "http://localhost:8000/api/recipes/v2" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "{
      \"name\": \"Test Recipe $i\",
      \"instructions\": \"Step 1: Do something. Step 2: Do something else.\",
      \"language\": \"ru\",
      \"servings\": 4,
      \"ingredients\": []
    }")
  
  RECIPE_ID=$(echo $RECIPE | jq -r '.id')
  
  # Generate insights and measure time
  START=$(gdate +%s%3N)
  curl -s "http://localhost:8000/api/recipes/v2/$RECIPE_ID/insights/ru" \
    -H "Authorization: Bearer $TOKEN" > /dev/null
  END=$(gdate +%s%3N)
  
  DURATION=$((END - START))
  echo "Recipe $i: ${DURATION}ms"
  
  sleep 1  # Rate limit
done
```

**Success Criteria**:
- Average generation time: <3000ms
- No timeouts
- No memory leaks
- Database connections released properly

---

## 🔄 Translation Pipeline Testing

### Test Multi-Language Generation
```bash
# 1. Create recipe in Russian
RECIPE_ID="..."

# 2. Generate insights in Russian (done)
curl "http://localhost:8000/api/recipes/v2/$RECIPE_ID/insights/ru" \
  -H "Authorization: Bearer $TOKEN"

# 3. Try to generate insights in English
curl "http://localhost:8000/api/recipes/v2/$RECIPE_ID/insights/en" \
  -H "Authorization: Bearer $TOKEN"

# Expected: Either auto-translate or return error "not implemented"

# 4. Get all languages
curl "http://localhost:8000/api/recipes/v2/$RECIPE_ID/insights" \
  -H "Authorization: Bearer $TOKEN"

# Expected: ["ru", "en"] if translation works
```

**Current Status**: Unknown - needs testing

---

## 🎯 Frontend Integration Checklist

### API Response Format
```typescript
interface RecipeAIInsights {
  id: string;
  recipe_id: string;
  language: string;
  steps: CookingStep[];
  validation: RecipeValidation;
  suggestions: RecipeSuggestion[];
  feasibility_score: number;  // 0-100
  model: string;
  created_at: number[];  // ⚠️ Non-standard format
  updated_at: number[];  // ⚠️ Non-standard format
}

interface CookingStep {
  step_number: number;
  action: string;
  description: string;
  duration_minutes: number | null;
  temperature: string | null;
  technique: string | null;
  ingredients_used: string[];
}

interface RecipeValidation {
  is_valid: boolean;
  warnings: ValidationIssue[];
  errors: ValidationIssue[];
  missing_ingredients: string[];
  safety_checks: string[];
}

interface ValidationIssue {
  severity: "warning" | "error";
  code: string;  // e.g. "TEMPERATURE_MISSING"
  message: string;
  field: string | null;
}

interface RecipeSuggestion {
  suggestion_type: "improvement" | "substitution" | "alternative";
  title: string;
  description: string;
  impact: string;  // e.g. "аромат", "вкус", "текстура"
  confidence: number;  // 0.0 - 1.0
}
```

### Time Format Converter
```typescript
// Convert [2026, 46, 21, 38, 31, 798332000, 0, 0, 0] to Date
function parseTimeArray(arr: number[]): Date {
  // arr[0] = year
  // arr[1] = month (0-indexed?)
  // arr[2] = day
  // arr[3] = hour
  // arr[4] = minute
  // arr[5] = nanoseconds
  
  return new Date(arr[0], arr[1], arr[2], arr[3], arr[4], Math.floor(arr[5] / 1000000));
}
```

### React Component Example
```tsx
function RecipeInsights({ recipeId }: { recipeId: string }) {
  const [insights, setInsights] = useState<RecipeAIInsights | null>(null);
  const [loading, setLoading] = useState(false);
  
  useEffect(() => {
    const fetchInsights = async () => {
      setLoading(true);
      try {
        const response = await fetch(
          `/api/recipes/v2/${recipeId}/insights/ru`,
          { headers: { Authorization: `Bearer ${token}` } }
        );
        const data = await response.json();
        setInsights(data.insights);  // ⚠️ Note: wrapped in { insights: {...} }
      } catch (error) {
        console.error('Failed to fetch insights:', error);
      } finally {
        setLoading(false);
      }
    };
    
    fetchInsights();
  }, [recipeId]);
  
  if (loading) return <div>🤖 Generating AI insights...</div>;
  if (!insights) return <div>No insights available</div>;
  
  return (
    <div>
      <h3>Feasibility Score: {insights.feasibility_score}/100</h3>
      
      <h4>Cooking Steps ({insights.steps.length})</h4>
      {insights.steps.map(step => (
        <div key={step.step_number}>
          <strong>Step {step.step_number}: {step.action}</strong>
          <p>{step.description}</p>
          {step.duration_minutes && <span>⏱️ {step.duration_minutes} min</span>}
          {step.temperature && <span>🌡️ {step.temperature}</span>}
        </div>
      ))}
      
      {insights.validation.warnings.length > 0 && (
        <div className="warnings">
          <h4>⚠️ Warnings</h4>
          {insights.validation.warnings.map((w, i) => (
            <div key={i}>{w.message}</div>
          ))}
        </div>
      )}
      
      {insights.suggestions.length > 0 && (
        <div className="suggestions">
          <h4>💡 Suggestions</h4>
          {insights.suggestions.map((s, i) => (
            <div key={i}>
              <strong>{s.title}</strong>
              <p>{s.description}</p>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
```

---

## 📝 Documentation Needed

- [ ] API documentation for insights endpoints
- [ ] Frontend integration guide
- [ ] Time format parsing guide
- [ ] Error code reference (TEMPERATURE_MISSING, etc.)
- [ ] Feasibility score interpretation guide
- [ ] Suggestion type taxonomy
- [ ] Validation code reference

---

## 🎉 Success Criteria for V1 "Stable"

V1 is considered **stable** when:

- [x] AI generation works consistently (✅ tested)
- [ ] Caching works and is fast (<100ms)
- [ ] Refresh endpoint works
- [ ] All error cases handled gracefully
- [ ] Performance is acceptable (<3s generation, <50ms cache)
- [ ] Documentation is complete
- [ ] Frontend can integrate successfully
- [ ] At least 10 recipes tested in production
- [ ] No crashes or memory leaks after 100 generations

**Current Status**: 60% stable

**Blocking Issues**: 
1. Admin catalog endpoint hang
2. Missing comprehensive testing

**Time Estimate**: 2-3 days to reach "stable"

---

## 🚀 When to Move to V2?

**Move to V2 when V1 is:**
- ✅ Stable (all tests passing)
- ✅ Deployed to production
- ✅ Used by at least 10 real users
- ✅ Feedback collected
- ✅ Pain points identified
- ✅ Performance baseline established

**Don't move to V2 if:**
- ❌ V1 still has critical bugs
- ❌ No production usage yet
- ❌ No user feedback
- ❌ Performance issues unresolved

**Estimated Timeline**: V1 stable in 2-3 days, V2 upgrade in 1-2 weeks after gathering feedback
