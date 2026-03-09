# 🗓️ Seasonal Calendar — Frontend API Guide

Production base URL: `https://ministerial-yetta-fodi999-c58d8823.koyeb.app`

---

## 1. Рыбный календарь (полная таблица)

### `GET /public/tools/fish-season-table`

**Назначение:** Главная таблица — все рыбы по месяцам с атрибутами.

| Параметр | Тип | Default | Значения |
|---|---|---|---|
| `lang` | string | `en` | `ru` / `pl` / `uk` / `en` |
| `region` | string | `PL` | `PL` / `EU` / `ES` / `UA` / `GLOBAL` |

**Запрос:**
```
GET /public/tools/fish-season-table?lang=ru&region=PL
```

**Ответ:**
```json
{
  "fish": [
    {
      "slug": "salmon",
      "name": "Лосось",
      "name_en": "Salmon",
      "name_ru": "Лосось",
      "name_pl": "Łosoś",
      "name_uk": "Лосось",
      "image_url": "https://cdn.dima-fomin.pl/ingredients/salmon.webp",
      "status": "good",           // текущий месяц: "peak" | "good" | "limited" | "off"
      "water_type": "sea",        // "sea" | "freshwater" | "both"
      "wild_farmed": "both",      // "wild" | "farmed" | "both"
      "sushi_grade": true,
      "season": [
        { "month": 1, "month_name": "Январь", "available": true },
        { "month": 2, "month_name": "Февраль", "available": true },
        // ... 12 месяцев
      ]
    }
  ],
  "all_year": [
    { "slug": "canned-tuna", "name": "Консервированный тунец", "image_url": null }
  ],
  "lang": "ru",
  "region": "PL",
  "note_all_year": "Доступны круглый год — не привязаны к сезону"
}
```

**Фронтенд-код (React):**
```tsx
const BASE = "https://ministerial-yetta-fodi999-c58d8823.koyeb.app";

interface FishSeasonEntry {
  month: number;
  month_name: string;
  available: boolean;
}

interface FishItem {
  slug: string;
  name: string;
  name_en: string;
  image_url: string | null;
  status: "peak" | "good" | "limited" | "off";
  water_type: "sea" | "freshwater" | "both" | null;
  wild_farmed: "wild" | "farmed" | "both" | null;
  sushi_grade: boolean | null;
  season: FishSeasonEntry[];
}

interface FishSeasonTableResponse {
  fish: FishItem[];
  all_year: { slug: string; name: string; image_url: string | null }[];
  lang: string;
  region: string;
  note_all_year: string;
}

async function fetchFishTable(lang = "ru", region = "PL"): Promise<FishSeasonTableResponse> {
  const res = await fetch(`${BASE}/public/tools/fish-season-table?lang=${lang}&region=${region}`);
  return res.json();
}
```

**Цвета статусов для UI:**
```ts
const STATUS_COLORS = {
  peak:    "#22c55e",  // зелёный
  good:    "#84cc16",  // жёлто-зелёный
  limited: "#f59e0b",  // оранжевый
  off:     "#e5e7eb",  // серый
} as const;
```

---

## 2. "Лучшее прямо сейчас" (SEO-блок на главной)

### `GET /public/tools/best-right-now`

**Назначение:** Блок на главной / SEO-страница. Возвращает текущий месяц,
заголовок на нужном языке, peak[] и also_good[].

| Параметр | Тип | Default | Значения |
|---|---|---|---|
| `type` | string | `seafood` | `seafood` / `vegetable` / `fruit` / `meat` |
| `lang` | string | `en` | `ru` / `pl` / `uk` / `en` |
| `region` | string | `PL` | `PL` / `EU` / `ES` / `UA` |
| `water_type` | string | — | `sea` / `freshwater` |
| `wild_farmed` | string | — | `wild` / `farmed` |
| `sushi` | bool | — | `true` |

**Запрос:**
```
GET /public/tools/best-right-now?type=seafood&lang=ru&region=PL
```

**Ответ:**
```json
{
  "headline": "🔥 Лучшие рыба и морепродукты в Март",
  "month": 3,
  "month_name": "Март",
  "product_type": "seafood",
  "region": "PL",
  "lang": "ru",
  "peak": [
    {
      "slug": "pike",
      "name": "Щука",
      "image_url": "https://cdn.dima-fomin.pl/ingredients/pike.webp",
      "status": "peak",
      "water_type": "freshwater",
      "wild_farmed": "wild",
      "sushi_grade": false
    }
  ],
  "also_good": [
    { "slug": "cod",      "name": "Треска",       "status": "good", "sushi_grade": false },
    { "slug": "salmon",   "name": "Лосось",        "status": "good", "sushi_grade": true  },
    { "slug": "sea-bass", "name": "Морской окунь", "status": "good", "sushi_grade": true  },
    { "slug": "trout",    "name": "Форель",        "status": "good", "sushi_grade": false }
  ]
}
```

**Фронтенд-код:**
```tsx
interface BestRightNowItem {
  slug: string;
  name: string;
  image_url: string | null;
  status: string;
  water_type: string | null;
  wild_farmed: string | null;
  sushi_grade: boolean | null;
}

interface BestRightNowResponse {
  headline: string;
  month: number;
  month_name: string;
  product_type: string;
  region: string;
  lang: string;
  peak: BestRightNowItem[];
  also_good: BestRightNowItem[];
}

async function fetchBestRightNow(
  type = "seafood",
  lang = "ru",
  region = "PL"
): Promise<BestRightNowResponse> {
  const res = await fetch(
    `${BASE}/public/tools/best-right-now?type=${type}&lang=${lang}&region=${region}`
  );
  return res.json();
}
```

**React-компонент:**
```tsx
export function BestRightNow({ lang = "ru", region = "PL" }) {
  const [data, setData] = useState<BestRightNowResponse | null>(null);

  useEffect(() => {
    fetchBestRightNow("seafood", lang, region).then(setData);
  }, [lang, region]);

  if (!data) return <div>Загрузка...</div>;

  return (
    <section>
      <h2>{data.headline}</h2>

      <div className="grid grid-cols-2 gap-4">
        <div>
          <h3>🔴 В самом соку</h3>
          {data.peak.map(item => (
            <FishCard key={item.slug} item={item} />
          ))}
        </div>
        <div>
          <h3>🟡 Тоже хороши</h3>
          {data.also_good.map(item => (
            <FishCard key={item.slug} item={item} />
          ))}
        </div>
      </div>
    </section>
  );
}
```

---

## 3. Лучшие продукты за месяц (с фильтрами)

### `GET /public/tools/best-in-season`

**Назначение:** SEO-страница "Лучшая рыба в августе", фильтрованные списки.

| Параметр | Тип | Default | Описание |
|---|---|---|---|
| `type` | string | `seafood` | тип продукта |
| `month` | number | текущий | 1–12 |
| `lang` | string | `en` | язык |
| `region` | string | `PL` | регион |
| `peak_only` | bool | `false` | `true` → только peak, `false` → peak + good |
| `water_type` | string | — | `sea` / `freshwater` |
| `wild_farmed` | string | — | `wild` / `farmed` |
| `sushi` | bool | — | только sushi-grade |

**Примеры запросов:**
```
# Только peak в текущем месяце
GET /public/tools/best-in-season?type=seafood&region=PL&lang=ru&peak_only=true

# Морская рыба в августе
GET /public/tools/best-in-season?type=seafood&region=PL&lang=ru&month=8&water_type=sea

# Только sushi-рыба
GET /public/tools/best-in-season?type=seafood&region=PL&lang=ru&sushi=true

# Пресноводная рыба
GET /public/tools/best-in-season?type=seafood&region=PL&lang=ru&water_type=freshwater
```

**Ответ:**
```json
{
  "product_type": "seafood",
  "month": 3,
  "lang": "ru",
  "region": "PL",
  "items": [
    {
      "slug": "pike",
      "name": "Щука",
      "image_url": "...",
      "status": "peak",
      "water_type": "freshwater",
      "wild_farmed": "wild",
      "sushi_grade": false
    }
  ]
}
```

**Хук с фильтрами:**
```tsx
interface SeasonFilters {
  type?: string;
  month?: number;
  lang?: string;
  region?: string;
  peak_only?: boolean;
  water_type?: "sea" | "freshwater";
  wild_farmed?: "wild" | "farmed";
  sushi?: boolean;
}

function useSeasonalFish(filters: SeasonFilters) {
  const [items, setItems] = useState<BestRightNowItem[]>([]);

  useEffect(() => {
    const params = new URLSearchParams();
    if (filters.type)        params.set("type",        filters.type);
    if (filters.month)       params.set("month",       String(filters.month));
    if (filters.lang)        params.set("lang",        filters.lang);
    if (filters.region)      params.set("region",      filters.region);
    if (filters.peak_only)   params.set("peak_only",   "true");
    if (filters.water_type)  params.set("water_type",  filters.water_type);
    if (filters.wild_farmed) params.set("wild_farmed", filters.wild_farmed);
    if (filters.sushi)       params.set("sushi",       "true");

    fetch(`${BASE}/public/tools/best-in-season?${params}`)
      .then(r => r.json())
      .then(d => setItems(d.items));
  }, [JSON.stringify(filters)]);

  return items;
}

// Использование:
const sushiFish = useSeasonalFish({ type: "seafood", region: "PL", lang: "ru", sushi: true });
const seaFish   = useSeasonalFish({ type: "seafood", region: "PL", lang: "ru", water_type: "sea" });
```

---

## 4. Что сейчас в сезоне (универсальный)

### `GET /public/tools/in-season-now`

**Назначение:** Любой тип продукта — что сейчас хорошо. Сортировка: peak → good → limited.

| Параметр | Тип | Default |
|---|---|---|
| `type` | string | `seafood` |
| `lang` | string | `en` |
| `region` | string | `PL` |

```
GET /public/tools/in-season-now?type=vegetable&lang=ru&region=PL
GET /public/tools/in-season-now?type=fruit&lang=ru&region=PL
GET /public/tools/in-season-now?type=seafood&lang=ru&region=PL
```

**Ответ:**
```json
{
  "product_type": "vegetable",
  "month": 3,
  "lang": "ru",
  "region": "PL",
  "items": [
    { "slug": "spinach", "name": "Шпинат", "image_url": "...", "status": "good" }
  ]
}
```

---

## 5. Сезонность конкретного продукта

### `GET /public/tools/product-seasonality`

**Назначение:** Страница продукта — 12 месяцев с peak/good/limited/off.

| Параметр | Тип | Обязательный |
|---|---|---|
| `slug` | string | ✅ |
| `lang` | string | — |
| `region` | string | — |

```
GET /public/tools/product-seasonality?slug=salmon&lang=ru&region=PL
GET /public/tools/product-seasonality?slug=tuna&lang=ru&region=ES
```

**Ответ:**
```json
{
  "slug": "salmon",
  "name": "Лосось",
  "product_type": "seafood",
  "image_url": "...",
  "region": "PL",
  "lang": "ru",
  "season": [
    { "month": 1, "month_name": "Январь", "status": "good",    "available": true,  "note": null },
    { "month": 4, "month_name": "Апрель", "status": "limited", "available": true,  "note": null },
    { "month": 5, "month_name": "Май",    "status": "off",     "available": false, "note": null },
    { "month": 7, "month_name": "Июль",   "status": "peak",    "available": true,  "note": null }
  ]
}
```

---

## 6. Все продукты за месяц

### `GET /public/tools/products-by-month`

**Назначение:** SEO "Что в сезоне в июле", таблица по месяцу.

```
GET /public/tools/products-by-month?month=7&type=seafood&lang=ru&region=PL
GET /public/tools/products-by-month?month=8&type=vegetable&lang=ru&region=PL
```

---

## 7. Список регионов

### `GET /public/tools/regions`

```json
{
  "regions": [
    { "code": "PL", "name_en": "Poland",         "description": "Central European seasonality (Poland)" },
    { "code": "EU", "name_en": "European Union", "description": "Average Western/Central Europe" },
    { "code": "ES", "name_en": "Spain",           "description": "Mediterranean / Southern Europe" },
    { "code": "UA", "name_en": "Ukraine",         "description": "Eastern European seasonality" },
    { "code": "GLOBAL", "name_en": "Global",      "description": "Generic global average seasonality" }
  ]
}
```

---

## 8. Полный пример — страница календаря

```tsx
// app/fish-calendar/page.tsx (Next.js)
"use client";

import { useState, useEffect } from "react";

const BASE = "https://ministerial-yetta-fodi999-c58d8823.koyeb.app";

const STATUS_COLOR: Record<string, string> = {
  peak:    "bg-green-500",
  good:    "bg-lime-400",
  limited: "bg-amber-400",
  off:     "bg-gray-200",
};

const STATUS_EMOJI: Record<string, string> = {
  peak: "🔴", good: "🟡", limited: "🟠", off: "⚪",
};

export default function FishCalendarPage() {
  const [lang, setLang]       = useState("ru");
  const [region, setRegion]   = useState("PL");
  const [waterType, setWaterType] = useState<string>("");
  const [sushiOnly, setSushiOnly] = useState(false);
  const [data, setData]       = useState<any>(null);

  // Загружаем таблицу
  useEffect(() => {
    fetch(`${BASE}/public/tools/fish-season-table?lang=${lang}&region=${region}`)
      .then(r => r.json())
      .then(setData);
  }, [lang, region]);

  if (!data) return <div>Loading...</div>;

  // Фильтруем на клиенте
  const filtered = data.fish.filter((f: any) => {
    if (waterType && f.water_type !== waterType) return false;
    if (sushiOnly && !f.sushi_grade) return false;
    return true;
  });

  const months = Array.from({ length: 12 }, (_, i) => i + 1);

  return (
    <div className="p-4">
      <h1 className="text-2xl font-bold mb-4">🐟 Рыбный календарь</h1>

      {/* Контролы */}
      <div className="flex gap-4 mb-6 flex-wrap">
        <select value={lang} onChange={e => setLang(e.target.value)}>
          <option value="ru">Русский</option>
          <option value="pl">Polski</option>
          <option value="uk">Українська</option>
          <option value="en">English</option>
        </select>

        <select value={region} onChange={e => setRegion(e.target.value)}>
          <option value="PL">Польша</option>
          <option value="EU">Европа</option>
          <option value="ES">Испания</option>
          <option value="UA">Украина</option>
        </select>

        <select value={waterType} onChange={e => setWaterType(e.target.value)}>
          <option value="">Вся рыба</option>
          <option value="sea">Морская</option>
          <option value="freshwater">Речная/озёрная</option>
        </select>

        <label className="flex items-center gap-2 cursor-pointer">
          <input
            type="checkbox"
            checked={sushiOnly}
            onChange={e => setSushiOnly(e.target.checked)}
          />
          🍣 Только суши
        </label>
      </div>

      {/* Легенда */}
      <div className="flex gap-4 mb-4 text-sm">
        {Object.entries(STATUS_EMOJI).map(([s, e]) => (
          <span key={s}>{e} {s}</span>
        ))}
      </div>

      {/* Таблица */}
      <div className="overflow-x-auto">
        <table className="border-collapse w-full text-sm">
          <thead>
            <tr>
              <th className="text-left p-2 min-w-[140px]">Рыба</th>
              {months.map(m => (
                <th key={m} className="p-1 w-8 text-center text-xs">
                  {data.fish[0]?.season[m - 1]?.month_name?.slice(0, 3) ?? m}
                </th>
              ))}
              <th className="p-2 text-xs">Тип</th>
            </tr>
          </thead>
          <tbody>
            {filtered.map((fish: any) => (
              <tr key={fish.slug} className="border-t">
                <td className="p-2 flex items-center gap-2">
                  {fish.image_url && (
                    <img src={fish.image_url} alt={fish.name} className="w-8 h-8 rounded-full object-cover" />
                  )}
                  <span>{fish.name}</span>
                  {fish.sushi_grade && <span title="Sushi grade">🍣</span>}
                </td>

                {/* 12 ячеек месяцев — цвет по статусу из БД */}
                {months.map(m => {
                  // Для полного статуса нужен products-by-month или product-seasonality
                  // fish-season-table возвращает available:bool — используем status только для текущего
                  const entry = fish.season[m - 1];
                  const isCurrent = new Date().getMonth() + 1 === m;
                  return (
                    <td
                      key={m}
                      className={`w-8 h-8 text-center border ${
                        entry?.available
                          ? isCurrent
                            ? STATUS_COLOR[fish.status]
                            : "bg-green-100"
                          : "bg-gray-100"
                      } ${isCurrent ? "ring-2 ring-blue-400" : ""}`}
                      title={entry?.available ? "в сезоне" : "не в сезоне"}
                    />
                  );
                })}

                <td className="p-2 text-xs text-gray-500">
                  {fish.water_type === "sea" ? "🌊" : "🏞️"}{" "}
                  {fish.wild_farmed === "wild" ? "дикая" : fish.wild_farmed === "farmed" ? "ферма" : "оба"}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* All-year секция */}
      {data.all_year.length > 0 && (
        <div className="mt-6">
          <h3 className="font-semibold text-gray-600">♾️ {data.note_all_year}</h3>
          <div className="flex gap-3 mt-2">
            {data.all_year.map((f: any) => (
              <span key={f.slug} className="px-3 py-1 bg-blue-50 rounded-full text-sm">
                {f.name}
              </span>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
```

---

## 9. Компонент "Блок сейчас" (виджет на главную)

```tsx
// components/BestRightNow.tsx
export function BestRightNow({ lang = "ru", region = "PL", type = "seafood" }) {
  const [data, setData] = useState<any>(null);

  useEffect(() => {
    fetch(`${BASE}/public/tools/best-right-now?type=${type}&lang=${lang}&region=${region}`)
      .then(r => r.json()).then(setData);
  }, [lang, region, type]);

  if (!data) return null;

  return (
    <div className="rounded-2xl bg-gradient-to-br from-blue-50 to-teal-50 p-6">
      <h2 className="text-xl font-bold mb-4">{data.headline}</h2>

      {data.peak.length > 0 && (
        <>
          <p className="text-sm font-semibold text-green-700 mb-2">🔴 Пик сезона</p>
          <div className="flex gap-3 flex-wrap mb-4">
            {data.peak.map((item: any) => (
              <div key={item.slug} className="flex flex-col items-center gap-1">
                {item.image_url && (
                  <img src={item.image_url} alt={item.name}
                    className="w-16 h-16 rounded-full object-cover border-2 border-green-400" />
                )}
                <span className="text-xs font-medium">{item.name}</span>
                {item.sushi_grade && <span className="text-xs">🍣</span>}
              </div>
            ))}
          </div>
        </>
      )}

      {data.also_good.length > 0 && (
        <>
          <p className="text-sm font-semibold text-amber-600 mb-2">🟡 Тоже хороши</p>
          <div className="flex gap-2 flex-wrap">
            {data.also_good.map((item: any) => (
              <span key={item.slug}
                className="px-3 py-1 bg-white rounded-full text-sm border border-amber-200">
                {item.image_url && (
                  <img src={item.image_url} alt="" className="inline w-4 h-4 rounded-full mr-1" />
                )}
                {item.name}
              </span>
            ))}
          </div>
        </>
      )}
    </div>
  );
}
```

---

## 10. Полная таблица с peak/good/limited по месяцам

Если нужен **полный статус** (peak/good/limited/off) для каждой ячейки таблицы —
используй `product-seasonality` для каждой рыбы или `products-by-month` агрегировано:

```tsx
// Загружаем полные статусы для всех месяцев через product-seasonality
async function fetchFullCalendar(slugs: string[], lang: string, region: string) {
  const results = await Promise.all(
    slugs.map(slug =>
      fetch(`${BASE}/public/tools/product-seasonality?slug=${slug}&lang=${lang}&region=${region}`)
        .then(r => r.json())
    )
  );
  // results[i].season — массив из 12 объектов { month, status, available }
  return results;
}
```

---

## Быстрый старт (копировать-вставить)

```ts
// lib/seasonal-api.ts

const BASE = "https://ministerial-yetta-fodi999-c58d8823.koyeb.app";

export const SeasonalAPI = {
  // Полная таблица рыб
  fishTable: (lang = "ru", region = "PL") =>
    fetch(`${BASE}/public/tools/fish-season-table?lang=${lang}&region=${region}`).then(r => r.json()),

  // Лучшее прямо сейчас
  bestRightNow: (type = "seafood", lang = "ru", region = "PL") =>
    fetch(`${BASE}/public/tools/best-right-now?type=${type}&lang=${lang}&region=${region}`).then(r => r.json()),

  // Лучшие за месяц с фильтрами
  bestInSeason: (params: {
    type?: string; month?: number; lang?: string; region?: string;
    peak_only?: boolean; water_type?: string; wild_farmed?: string; sushi?: boolean;
  }) => {
    const p = new URLSearchParams();
    Object.entries(params).forEach(([k, v]) => { if (v !== undefined) p.set(k, String(v)); });
    return fetch(`${BASE}/public/tools/best-in-season?${p}`).then(r => r.json());
  },

  // Сезонность продукта
  productSeasonality: (slug: string, lang = "ru", region = "PL") =>
    fetch(`${BASE}/public/tools/product-seasonality?slug=${slug}&lang=${lang}&region=${region}`).then(r => r.json()),

  // Что сейчас в сезоне
  inSeasonNow: (type = "seafood", lang = "ru", region = "PL") =>
    fetch(`${BASE}/public/tools/in-season-now?type=${type}&lang=${lang}&region=${region}`).then(r => r.json()),

  // Регионы
  regions: () =>
    fetch(`${BASE}/public/tools/regions`).then(r => r.json()),
};
```
