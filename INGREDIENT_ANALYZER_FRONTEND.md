# 🔬 Smart Ingredient Analyzer — Frontend Guide

**Production API:** `https://ministerial-yetta-fodi999-c58d8823.koyeb.app`  
**Route suggestion:** `/chef-tools/ingredient-analyzer` or `/ingredients/[slug]`

---

## Что в базе (111 ингредиентов)

| Тип | Кол-во | Примеры |
|---|---|---|
| `seafood` | 11 | salmon, tuna, cod, herring, mackerel, pike, shrimp… |
| `vegetable` | 22 | spinach, broccoli, tomato, garlic, potato… |
| `fruit` | 15 | apple, banana, lemon, strawberry, avocado… |
| `meat` | 8 | chicken-breast, beef, pork, bacon, ham… |
| `grain` | 7 | rice, wheat-flour, oats, pasta, buckwheat… |
| `dairy` | 6 | butter, eggs, cottage-cheese, hard-cheese… |
| `legume` | 3 | lentils, chickpeas, beans |
| `nut` | 5 | almonds, walnuts, sesame-seeds… |
| `spice` | 10 | basil, garlic, dill, parsley, turmeric… |
| `other` | 24 | olive-oil, honey, sugar, soy-sauce, ketchup… |

Витамины (b12, d, iron, magnesium) заполнены для **31 продукта**:
все рыбы, beef, pork, chicken, egg, milk, cheese, spinach, broccoli, tomato, potato, carrot, onion, garlic, lemon, apple, banana, rice, wheat-flour, oats, butter, olive-oil.

---

## API — все endpoints для страницы

### 1. Поиск / автокомплит
```
GET /public/tools/ingredients?search=sal&lang=en&limit=10
```
→ используется для autocomplete input.

### 2. Карточка + нутриция + витамины + score
```
GET /public/tools/nutrition?name=salmon&amount=150&unit=g&lang=en
```
→ вся информация по одному продукту.

### 3. Сравнение двух продуктов
```
GET /public/tools/compare?food1=salmon&food2=tuna&lang=en
```
→ side-by-side с `winner` объектом.

### 4. Сезонность продукта
```
GET /public/tools/product-seasonality?slug=salmon&lang=en&region=PL
```
→ 12 месяцев peak/good/limited/off.

### 5. Полный список (для выпадающего меню compare)
```
GET /public/tools/ingredients?lang=en&limit=200
```
→ все 111 ингредиентов.

---

## TypeScript интерфейсы

```ts
// lib/types/ingredient.ts

export interface NutritionBreakdown {
  calories:   number;
  protein_g:  number;
  fat_g:      number;
  carbs_g:    number;
  fiber_g:    number;
  sugar_g:    number;
  salt_g:     number;
  sodium_mg:  number;
}

export interface MacrosRatio {
  protein_pct: number;
  fat_pct:     number;
  carbs_pct:   number;
}

export interface VitaminData {
  vitamin_b12_mcg: number | null;
  vitamin_d_mcg:   number | null;
  iron_mg:         number | null;
  magnesium_mg:    number | null;
}

export interface NutritionResponse {
  query:             string;
  slug:              string | null;
  name:              string;
  product_type:      string | null;
  image_url:         string | null;
  water_type:        string | null;
  wild_farmed:       string | null;
  sushi_grade:       boolean | null;
  amount_g:          number;
  unit:              string;
  unit_label:        string;
  per_100g:          NutritionBreakdown;
  for_amount:        NutritionBreakdown;
  macros_ratio:      MacrosRatio;
  nutrition_score:   number;
  vitamins:          VitaminData;
  typical_portion_g: number | null;
  found_in_db:       boolean;
  lang:              string;
}

export interface IngredientDbEntry {
  slug:              string | null;
  name:              string;
  name_en:           string;
  product_type:      string | null;
  image_url:         string | null;
  water_type:        string | null;
  wild_farmed:       string | null;
  sushi_grade:       boolean | null;
  typical_portion_g: number | null;
  per_100g:          NutritionBreakdown;
  macros_ratio:      MacrosRatio;
  nutrition_score:   number;
  vitamins:          VitaminData;
}

export interface CompareSide {
  query:            string;
  slug:             string | null;
  name:             string;
  product_type:     string | null;
  image_url:        string | null;
  water_type:       string | null;
  wild_farmed:      string | null;
  sushi_grade:      boolean | null;
  per_100g:         NutritionBreakdown;
  macros_ratio:     MacrosRatio;
  nutrition_score:  number;
  vitamins:         VitaminData;
  found_in_db:      boolean;
}

export interface CompareWinner {
  calories_lower:  "food1" | "food2" | "tie";
  protein_higher:  "food1" | "food2" | "tie";
  fat_lower:       "food1" | "food2" | "tie";
  carbs_lower:     "food1" | "food2" | "tie";
  fiber_higher:    "food1" | "food2" | "tie";
  nutrition_score: "food1" | "food2" | "tie";
}

export interface CompareResponse {
  food1:  CompareSide;
  food2:  CompareSide;
  winner: CompareWinner;
  lang:   string;
}

export interface SeasonMonth {
  month:        number;
  month_name:   string;
  status:       "peak" | "good" | "limited" | "off";
  available:    boolean;
  note:         string | null;
}

export interface SeasonalityResponse {
  slug:         string;
  name:         string;
  product_type: string | null;
  image_url:    string | null;
  region:       string;
  lang:         string;
  season:       SeasonMonth[];
}
```

---

## API helper

```ts
// lib/api/ingredient-analyzer.ts

const BASE = "https://ministerial-yetta-fodi999-c58d8823.koyeb.app";

export const IngredientAPI = {
  // Autocomplete search
  search: (query: string, lang = "en", limit = 10): Promise<{ total: number; items: IngredientDbEntry[] }> =>
    fetch(`${BASE}/public/tools/ingredients?search=${encodeURIComponent(query)}&lang=${lang}&limit=${limit}`)
      .then(r => r.json()),

  // All ingredients (for dropdown)
  all: (lang = "en"): Promise<{ total: number; items: IngredientDbEntry[] }> =>
    fetch(`${BASE}/public/tools/ingredients?lang=${lang}&limit=200`)
      .then(r => r.json()),

  // Full nutrition for one ingredient + portion
  nutrition: (name: string, amount = 100, unit = "g", lang = "en"): Promise<NutritionResponse> =>
    fetch(`${BASE}/public/tools/nutrition?name=${encodeURIComponent(name)}&amount=${amount}&unit=${unit}&lang=${lang}`)
      .then(r => r.json()),

  // Compare two foods
  compare: (food1: string, food2: string, lang = "en"): Promise<CompareResponse> =>
    fetch(`${BASE}/public/tools/compare?food1=${encodeURIComponent(food1)}&food2=${encodeURIComponent(food2)}&lang=${lang}`)
      .then(r => r.json()),

  // Seasonality for one slug
  seasonality: (slug: string, lang = "en", region = "PL"): Promise<SeasonalityResponse> =>
    fetch(`${BASE}/public/tools/product-seasonality?slug=${slug}&lang=${lang}&region=${region}`)
      .then(r => r.json()),
};
```

---

## Полная страница — Next.js (App Router)

```tsx
// app/chef-tools/ingredient-analyzer/page.tsx
"use client";

import { useState, useEffect, useRef } from "react";
import { IngredientAPI } from "@/lib/api/ingredient-analyzer";
import type { NutritionResponse, CompareResponse, SeasonalityResponse, IngredientDbEntry } from "@/lib/types/ingredient";

const BASE_URL = "https://ministerial-yetta-fodi999-c58d8823.koyeb.app";

// ── Score color ──────────────────────────────────────────────────────────────
function scoreColor(s: number) {
  if (s >= 70) return "text-green-600 bg-green-50";
  if (s >= 40) return "text-amber-600 bg-amber-50";
  return "text-red-500 bg-red-50";
}

// ── Season status colors ─────────────────────────────────────────────────────
const SEASON_BG: Record<string, string> = {
  peak:    "bg-green-500",
  good:    "bg-lime-400",
  limited: "bg-amber-400",
  off:     "bg-gray-200",
};

// ── MacroBar ─────────────────────────────────────────────────────────────────
function MacroBar({ label, pct, color }: { label: string; pct: number; color: string }) {
  return (
    <div className="flex items-center gap-2 text-sm">
      <span className="w-16 text-gray-600">{label}</span>
      <div className="flex-1 h-3 rounded-full bg-gray-100 overflow-hidden">
        <div className={`h-full rounded-full ${color}`} style={{ width: `${pct}%` }} />
      </div>
      <span className="w-10 text-right font-medium">{pct}%</span>
    </div>
  );
}

// ── NutritionRow ─────────────────────────────────────────────────────────────
function NutritionRow({ label, val, unit }: { label: string; val: number; unit: string }) {
  return (
    <div className="flex justify-between py-1 border-b border-gray-100 text-sm">
      <span className="text-gray-600">{label}</span>
      <span className="font-semibold">{val} {unit}</span>
    </div>
  );
}

// ── VitaminRow ───────────────────────────────────────────────────────────────
function VitaminRow({ label, val, unit }: { label: string; val: number | null; unit: string }) {
  if (val === null) return null;
  return (
    <div className="flex justify-between py-1 border-b border-gray-100 text-sm">
      <span className="text-gray-600">{label}</span>
      <span className="font-semibold">{val} {unit}</span>
    </div>
  );
}

// ── CompareBar ───────────────────────────────────────────────────────────────
function CompareRow({
  label, v1, v2, unit, winner, food1Name, food2Name
}: {
  label: string; v1: number; v2: number; unit: string;
  winner: string; food1Name: string; food2Name: string;
}) {
  const max = Math.max(v1, v2, 0.01);
  return (
    <div className="mb-3">
      <div className="flex justify-between text-xs text-gray-500 mb-1">
        <span>{label}</span>
        {winner !== "tie" && (
          <span className="text-green-600 font-medium">
            🥇 {winner === "food1" ? food1Name : food2Name}
          </span>
        )}
      </div>
      <div className="flex gap-2 items-center">
        <span className="text-xs w-8 text-right text-blue-600 font-medium">{v1}{unit}</span>
        <div className="flex-1 flex gap-1 h-4">
          <div className="flex-1 flex justify-end">
            <div className="h-full bg-blue-400 rounded-l" style={{ width: `${(v1 / max) * 100}%` }} />
          </div>
          <div className="flex-1">
            <div className="h-full bg-orange-400 rounded-r" style={{ width: `${(v2 / max) * 100}%` }} />
          </div>
        </div>
        <span className="text-xs w-8 text-orange-600 font-medium">{v2}{unit}</span>
      </div>
    </div>
  );
}

// ── Main Page ────────────────────────────────────────────────────────────────
export default function IngredientAnalyzerPage() {
  const [search, setSearch]           = useState("");
  const [suggestions, setSuggestions] = useState<IngredientDbEntry[]>([]);
  const [showSugg, setShowSugg]       = useState(false);

  const [selected, setSelected]   = useState("salmon");
  const [amount, setAmount]       = useState(100);
  const [unit, setUnit]           = useState("g");
  const [lang, setLang]           = useState("en");
  const [region, setRegion]       = useState("PL");

  const [nutrition, setNutrition]     = useState<NutritionResponse | null>(null);
  const [compare, setCompare]         = useState<CompareResponse | null>(null);
  const [seasonality, setSeasonality] = useState<SeasonalityResponse | null>(null);
  const [compareWith, setCompareWith] = useState("tuna");
  const [allIngredients, setAll]      = useState<IngredientDbEntry[]>([]);

  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Load all ingredients for compare dropdown
  useEffect(() => {
    IngredientAPI.all(lang).then(d => setAll(d.items));
  }, [lang]);

  // Autocomplete
  useEffect(() => {
    if (search.length < 2) { setSuggestions([]); return; }
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => {
      IngredientAPI.search(search, lang, 8).then(d => {
        setSuggestions(d.items);
        setShowSugg(true);
      });
    }, 250);
  }, [search, lang]);

  // Fetch nutrition when selected/amount/unit/lang changes
  useEffect(() => {
    if (!selected) return;
    IngredientAPI.nutrition(selected, amount, unit, lang).then(setNutrition);
  }, [selected, amount, unit, lang]);

  // Fetch seasonality
  useEffect(() => {
    if (!selected) return;
    IngredientAPI.seasonality(selected, lang, region).then(setSeasonality);
  }, [selected, lang, region]);

  // Fetch compare
  useEffect(() => {
    if (!selected || !compareWith || selected === compareWith) return;
    IngredientAPI.compare(selected, compareWith, lang).then(setCompare);
  }, [selected, compareWith, lang]);

  const pickSuggestion = (item: IngredientDbEntry) => {
    const slug = item.slug || item.name_en.toLowerCase();
    setSelected(slug);
    setSearch(item.name_en);
    setShowSugg(false);
  };

  const n = nutrition;

  return (
    <div className="max-w-4xl mx-auto p-4 space-y-6">

      {/* ── Header ── */}
      <div>
        <h1 className="text-3xl font-bold">🔬 Ingredient Analyzer</h1>
        <p className="text-gray-500 text-sm mt-1">
          Search any ingredient — get nutrition, vitamins, macros, season & food comparison.
        </p>
      </div>

      {/* ── Controls bar ── */}
      <div className="flex gap-3 flex-wrap items-center">
        <select
          value={lang}
          onChange={e => setLang(e.target.value)}
          className="border rounded px-2 py-1 text-sm"
        >
          <option value="en">English</option>
          <option value="ru">Русский</option>
          <option value="pl">Polski</option>
          <option value="uk">Українська</option>
        </select>
        <select
          value={region}
          onChange={e => setRegion(e.target.value)}
          className="border rounded px-2 py-1 text-sm"
        >
          <option value="PL">Poland</option>
          <option value="EU">Europe</option>
          <option value="ES">Spain</option>
          <option value="UA">Ukraine</option>
        </select>
      </div>

      {/* ── Search ── */}
      <div className="relative">
        <div className="flex items-center border-2 border-blue-300 rounded-xl overflow-hidden focus-within:border-blue-500">
          <span className="px-3 text-xl">🔍</span>
          <input
            className="flex-1 py-3 text-lg outline-none"
            placeholder="Search ingredient: salmon, broccoli, rice..."
            value={search}
            onChange={e => setSearch(e.target.value)}
            onFocus={() => suggestions.length > 0 && setShowSugg(true)}
          />
          {search && (
            <button
              onClick={() => { setSearch(""); setSuggestions([]); }}
              className="px-3 text-gray-400 hover:text-gray-700"
            >✕</button>
          )}
        </div>

        {/* Autocomplete dropdown */}
        {showSugg && suggestions.length > 0 && (
          <div className="absolute top-full left-0 right-0 bg-white border rounded-xl shadow-lg z-50 mt-1 overflow-hidden">
            {suggestions.map(item => (
              <button
                key={item.slug}
                onClick={() => pickSuggestion(item)}
                className="w-full flex items-center gap-3 px-4 py-2 hover:bg-blue-50 text-left"
              >
                {item.image_url ? (
                  <img src={item.image_url} alt="" className="w-8 h-8 rounded-full object-cover" />
                ) : (
                  <div className="w-8 h-8 rounded-full bg-gray-100 flex items-center justify-center text-lg">
                    {item.product_type === "seafood" ? "🐟"
                      : item.product_type === "vegetable" ? "🥦"
                      : item.product_type === "fruit" ? "🍎"
                      : item.product_type === "meat" ? "🥩"
                      : item.product_type === "grain" ? "🌾"
                      : item.product_type === "dairy" ? "🥛"
                      : "🍽️"}
                  </div>
                )}
                <div>
                  <div className="font-medium">{item.name}</div>
                  <div className="text-xs text-gray-400">
                    {item.product_type} · {item.per_100g.calories} kcal/100g · score {item.nutrition_score}
                  </div>
                </div>
              </button>
            ))}
          </div>
        )}
      </div>

      {/* ── Main card ── */}
      {n && (
        <div className="grid md:grid-cols-2 gap-6">

          {/* Left: ingredient card */}
          <div className="bg-white rounded-2xl border shadow-sm p-5 space-y-4">

            {/* Header */}
            <div className="flex gap-4 items-start">
              {n.image_url ? (
                <img src={n.image_url} alt={n.name} className="w-20 h-20 rounded-xl object-cover" />
              ) : (
                <div className="w-20 h-20 rounded-xl bg-gradient-to-br from-blue-100 to-teal-100 flex items-center justify-center text-4xl">
                  {n.product_type === "seafood" ? "🐟"
                    : n.product_type === "vegetable" ? "🥦"
                    : n.product_type === "fruit" ? "🍎"
                    : n.product_type === "meat" ? "🥩"
                    : "🍽️"}
                </div>
              )}
              <div className="flex-1">
                <h2 className="text-2xl font-bold">{n.name}</h2>
                <div className="flex gap-2 flex-wrap mt-1">
                  {n.product_type && (
                    <span className="px-2 py-0.5 bg-blue-100 text-blue-700 rounded-full text-xs capitalize">
                      {n.product_type}
                    </span>
                  )}
                  {n.water_type && (
                    <span className="px-2 py-0.5 bg-cyan-100 text-cyan-700 rounded-full text-xs">
                      {n.water_type === "sea" ? "🌊 sea" : "🏞️ freshwater"}
                    </span>
                  )}
                  {n.wild_farmed && (
                    <span className="px-2 py-0.5 bg-green-100 text-green-700 rounded-full text-xs">
                      {n.wild_farmed}
                    </span>
                  )}
                  {n.sushi_grade && (
                    <span className="px-2 py-0.5 bg-pink-100 text-pink-700 rounded-full text-xs">
                      🍣 sushi grade
                    </span>
                  )}
                </div>
              </div>
              {/* Score badge */}
              <div className={`px-3 py-2 rounded-xl text-center min-w-[56px] ${scoreColor(n.nutrition_score)}`}>
                <div className="text-2xl font-bold">{n.nutrition_score}</div>
                <div className="text-xs">score</div>
              </div>
            </div>

            {/* Portion calculator */}
            <div className="bg-gray-50 rounded-xl p-3">
              <p className="text-xs text-gray-500 font-medium mb-2 uppercase tracking-wide">Portion calculator</p>
              <div className="flex gap-2 items-center">
                <input
                  type="number"
                  min={1}
                  max={2000}
                  value={amount}
                  onChange={e => setAmount(Number(e.target.value))}
                  className="w-24 border rounded-lg px-2 py-1 text-lg font-bold text-center"
                />
                <select
                  value={unit}
                  onChange={e => setUnit(e.target.value)}
                  className="border rounded-lg px-2 py-1 text-sm"
                >
                  <option value="g">g</option>
                  <option value="kg">kg</option>
                  <option value="oz">oz</option>
                  <option value="lb">lb</option>
                  <option value="cup">cup</option>
                  <option value="tbsp">tbsp</option>
                  <option value="tsp">tsp</option>
                </select>
                {n.typical_portion_g && (
                  <button
                    onClick={() => { setAmount(n.typical_portion_g!); setUnit("g"); }}
                    className="text-xs text-blue-600 underline"
                  >
                    typical ({n.typical_portion_g}g)
                  </button>
                )}
              </div>
              <p className="text-xs text-gray-400 mt-1">{n.amount_g}g total</p>
            </div>

            {/* Nutrition breakdown */}
            <div>
              <p className="text-xs text-gray-500 font-medium mb-2 uppercase tracking-wide">
                Nutrition for {n.amount_g}g
              </p>
              <div className="text-2xl font-bold text-orange-500 mb-2">
                {n.for_amount.calories} kcal
              </div>
              <NutritionRow label="Protein"  val={n.for_amount.protein_g} unit="g" />
              <NutritionRow label="Fat"      val={n.for_amount.fat_g}     unit="g" />
              <NutritionRow label="Carbs"    val={n.for_amount.carbs_g}   unit="g" />
              <NutritionRow label="Fiber"    val={n.for_amount.fiber_g}   unit="g" />
              <NutritionRow label="Sugar"    val={n.for_amount.sugar_g}   unit="g" />
              <NutritionRow label="Salt"     val={n.for_amount.salt_g}    unit="g" />
              <NutritionRow label="Sodium"   val={n.for_amount.sodium_mg} unit="mg" />
            </div>
          </div>

          {/* Right: macros + vitamins + season */}
          <div className="space-y-4">

            {/* Macros ratio */}
            <div className="bg-white rounded-2xl border shadow-sm p-5">
              <p className="text-xs text-gray-500 font-medium mb-3 uppercase tracking-wide">Macros ratio (% of kcal)</p>
              <div className="space-y-2">
                <MacroBar label="Protein" pct={n.macros_ratio.protein_pct} color="bg-blue-400" />
                <MacroBar label="Fat"     pct={n.macros_ratio.fat_pct}     color="bg-orange-400" />
                <MacroBar label="Carbs"   pct={n.macros_ratio.carbs_pct}   color="bg-green-400" />
              </div>
              <p className="text-xs text-gray-400 mt-3">
                Per 100g: {n.per_100g.protein_g}g P · {n.per_100g.fat_g}g F · {n.per_100g.carbs_g}g C
              </p>
            </div>

            {/* Vitamins */}
            {(n.vitamins.vitamin_b12_mcg !== null
              || n.vitamins.vitamin_d_mcg !== null
              || n.vitamins.iron_mg !== null
              || n.vitamins.magnesium_mg !== null) && (
              <div className="bg-white rounded-2xl border shadow-sm p-5">
                <p className="text-xs text-gray-500 font-medium mb-3 uppercase tracking-wide">Vitamins & Minerals</p>
                <VitaminRow label="Vitamin B12"   val={n.vitamins.vitamin_b12_mcg} unit="µg" />
                <VitaminRow label="Vitamin D"     val={n.vitamins.vitamin_d_mcg}   unit="µg" />
                <VitaminRow label="Iron"          val={n.vitamins.iron_mg}         unit="mg" />
                <VitaminRow label="Magnesium"     val={n.vitamins.magnesium_mg}    unit="mg" />
                <p className="text-xs text-gray-400 mt-2">Values per 100g</p>
              </div>
            )}

            {/* Seasonality */}
            {seasonality && seasonality.season.length > 0 && (
              <div className="bg-white rounded-2xl border shadow-sm p-5">
                <p className="text-xs text-gray-500 font-medium mb-3 uppercase tracking-wide">
                  Seasonal Calendar ({seasonality.region})
                </p>
                <div className="grid grid-cols-12 gap-0.5">
                  {seasonality.season.map(m => (
                    <div key={m.month} className="text-center">
                      <div
                        className={`h-6 rounded-sm ${SEASON_BG[m.status]}`}
                        title={`${m.month_name}: ${m.status}`}
                      />
                      <div className="text-[9px] text-gray-400 mt-0.5">
                        {m.month_name.slice(0, 1)}
                      </div>
                    </div>
                  ))}
                </div>
                <div className="flex gap-3 mt-2 text-xs text-gray-500">
                  <span><span className="inline-block w-3 h-3 rounded bg-green-500 mr-1" />peak</span>
                  <span><span className="inline-block w-3 h-3 rounded bg-lime-400 mr-1" />good</span>
                  <span><span className="inline-block w-3 h-3 rounded bg-amber-400 mr-1" />limited</span>
                  <span><span className="inline-block w-3 h-3 rounded bg-gray-200 mr-1" />off</span>
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* ── Compare panel ── */}
      {n && (
        <div className="bg-white rounded-2xl border shadow-sm p-5">
          <p className="text-xs text-gray-500 font-medium mb-3 uppercase tracking-wide">Compare with</p>

          <div className="flex gap-3 items-center mb-4">
            {/* Food1 badge */}
            <div className="flex items-center gap-2 px-3 py-1.5 bg-blue-50 rounded-lg">
              {n.image_url && <img src={n.image_url} alt="" className="w-6 h-6 rounded-full object-cover" />}
              <span className="font-semibold text-blue-700">{n.name}</span>
            </div>
            <span className="text-gray-400 font-bold">VS</span>
            {/* Food2 select */}
            <select
              value={compareWith}
              onChange={e => setCompareWith(e.target.value)}
              className="border rounded-lg px-3 py-1.5 text-sm flex-1 max-w-[200px]"
            >
              {allIngredients
                .filter(i => i.slug !== selected && i.slug !== null)
                .map(i => (
                  <option key={i.slug} value={i.slug!}>{i.name_en}</option>
                ))}
            </select>
          </div>

          {compare && (
            <>
              {/* Winner summary */}
              <div className="flex gap-3 mb-4 flex-wrap">
                {Object.entries(compare.winner).map(([key, val]) => {
                  if (val === "tie") return null;
                  const winnerName = val === "food1" ? compare.food1.name : compare.food2.name;
                  const label = key.replace("_", " ");
                  return (
                    <span key={key} className="px-2 py-1 bg-green-50 text-green-700 rounded-lg text-xs">
                      🥇 {winnerName} — {label}
                    </span>
                  );
                })}
              </div>

              {/* Bars */}
              <div className="grid md:grid-cols-2 gap-x-8 gap-y-1">
                <div className="text-xs text-center text-blue-600 font-semibold mb-2">{compare.food1.name}</div>
                <div className="text-xs text-center text-orange-600 font-semibold mb-2">{compare.food2.name}</div>
              </div>
              <CompareRow label="Calories"  v1={compare.food1.per_100g.calories}  v2={compare.food2.per_100g.calories}  unit=" kcal" winner={compare.winner.calories_lower}  food1Name={compare.food1.name} food2Name={compare.food2.name} />
              <CompareRow label="Protein"   v1={compare.food1.per_100g.protein_g} v2={compare.food2.per_100g.protein_g} unit="g"     winner={compare.winner.protein_higher}  food1Name={compare.food1.name} food2Name={compare.food2.name} />
              <CompareRow label="Fat"       v1={compare.food1.per_100g.fat_g}     v2={compare.food2.per_100g.fat_g}     unit="g"     winner={compare.winner.fat_lower}       food1Name={compare.food1.name} food2Name={compare.food2.name} />
              <CompareRow label="Carbs"     v1={compare.food1.per_100g.carbs_g}   v2={compare.food2.per_100g.carbs_g}   unit="g"     winner={compare.winner.carbs_lower}     food1Name={compare.food1.name} food2Name={compare.food2.name} />
              <CompareRow label="Score"     v1={compare.food1.nutrition_score}    v2={compare.food2.nutrition_score}    unit=""      winner={compare.winner.nutrition_score} food1Name={compare.food1.name} food2Name={compare.food2.name} />

              {/* Vitamins compare */}
              {(compare.food1.vitamins.vitamin_b12_mcg !== null || compare.food2.vitamins.vitamin_b12_mcg !== null) && (
                <CompareRow label="B12"     v1={compare.food1.vitamins.vitamin_b12_mcg ?? 0} v2={compare.food2.vitamins.vitamin_b12_mcg ?? 0} unit="µg"  winner="tie" food1Name={compare.food1.name} food2Name={compare.food2.name} />
              )}
              {(compare.food1.vitamins.vitamin_d_mcg !== null || compare.food2.vitamins.vitamin_d_mcg !== null) && (
                <CompareRow label="Vit D"   v1={compare.food1.vitamins.vitamin_d_mcg ?? 0}   v2={compare.food2.vitamins.vitamin_d_mcg ?? 0}   unit="µg"  winner="tie" food1Name={compare.food1.name} food2Name={compare.food2.name} />
              )}
              {(compare.food1.vitamins.iron_mg !== null || compare.food2.vitamins.iron_mg !== null) && (
                <CompareRow label="Iron"    v1={compare.food1.vitamins.iron_mg ?? 0}          v2={compare.food2.vitamins.iron_mg ?? 0}          unit="mg"  winner="tie" food1Name={compare.food1.name} food2Name={compare.food2.name} />
              )}
            </>
          )}
        </div>
      )}

    </div>
  );
}
```

---

## SEO — статические страницы `/ingredients/[slug]`

```tsx
// app/ingredients/[slug]/page.tsx (Next.js static generation)

const BASE = "https://ministerial-yetta-fodi999-c58d8823.koyeb.app";

// Generate all 111 static pages at build time
export async function generateStaticParams() {
  const res = await fetch(`${BASE}/public/tools/ingredients?limit=200`);
  const data = await res.json();
  return data.items
    .filter((i: any) => i.slug)
    .map((i: any) => ({ slug: i.slug }));
}

// SEO metadata per page
export async function generateMetadata({ params }: { params: { slug: string } }) {
  const res = await fetch(`${BASE}/public/tools/nutrition?name=${params.slug}&lang=en`);
  const data = await res.json();
  return {
    title: `${data.name} Nutrition Facts — Calories, Protein, Vitamins`,
    description: `${data.name}: ${data.per_100g.calories} kcal, ${data.per_100g.protein_g}g protein per 100g. Nutrition score: ${data.nutrition_score}/100. Full macro breakdown, vitamins, seasonal availability.`,
    openGraph: {
      title: `${data.name} Nutrition`,
      description: `${data.per_100g.calories} kcal | Protein ${data.per_100g.protein_g}g | Score ${data.nutrition_score}`,
      images: data.image_url ? [{ url: data.image_url }] : [],
    },
  };
}

// Page reuses IngredientAnalyzerPage with pre-selected slug
export default function IngredientPage({ params }: { params: { slug: string } }) {
  return <IngredientAnalyzerPage preselected={params.slug} />;
}
```

**Это генерирует 111 SEO-страниц:**
```
/ingredients/salmon       → "Salmon Nutrition Facts"
/ingredients/tuna         → "Tuna Nutrition Facts"
/ingredients/cod          → "Cod Nutrition Facts"
/ingredients/chicken-breast → "Chicken breast Nutrition Facts"
... (111 pages)
```

---

## Быстрый старт

```bash
# 1. Создай файл API helper
touch lib/api/ingredient-analyzer.ts

# 2. Создай страницу
mkdir -p app/chef-tools/ingredient-analyzer
touch app/chef-tools/ingredient-analyzer/page.tsx

# 3. Для SEO статических страниц
mkdir -p app/ingredients/[slug]
touch app/ingredients/[slug]/page.tsx

# 4. Запусти
npm run dev
# открой http://localhost:3000/chef-tools/ingredient-analyzer
```

---

## API Coverage — что проверено в production

| Endpoint | Status | Данные |
|---|---|---|
| `GET /public/tools/ingredients?limit=200` | ✅ | 111 ингредиентов |
| `GET /public/tools/nutrition?name=salmon` | ✅ | nutrition_score, macros_ratio, vitamins |
| `GET /public/tools/compare?food1=salmon&food2=tuna` | ✅ | winner объект, side-by-side |
| `GET /public/tools/product-seasonality?slug=salmon` | ✅ | 12 месяцев peak/good/limited/off |
| `GET /public/tools/ingredients?search=sal&limit=8` | ✅ | autocomplete |

**Все данные берутся из `catalog_ingredients` в Neon PostgreSQL — один источник правды.**
