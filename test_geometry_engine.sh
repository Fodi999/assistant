#!/usr/bin/env bash
# test_geometry_engine.sh
# Smoke + точность geometry_engine: extrude, boolean, координаты вершин, bbox, объём
#
# Использование:
#   chmod +x test_geometry_engine.sh && ./test_geometry_engine.sh
#
# Требования: cargo run --bin restaurant-backend (порт 8000)

BASE="http://localhost:8000"
PASS=0; FAIL=0

check_eq() {
  local label="$1" got="$2" want="$3"
  if [ "$got" = "$want" ]; then
    echo "  ✅  $label = $got"
    PASS=$((PASS+1))
  else
    echo "  ❌  $label = '$got'  (ожидалось '$want')"
    FAIL=$((FAIL+1))
  fi
}

check_ge() {
  local label="$1" got="$2" want="$3"
  if python3 -c "exit(0 if float('$got') >= float('$want') else 1)" 2>/dev/null; then
    echo "  ✅  $label = $got  (>= $want)"
    PASS=$((PASS+1))
  else
    echo "  ❌  $label = '$got'  (ожидалось >= $want)"
    FAIL=$((FAIL+1))
  fi
}

check_near() {
  local label="$1" got="$2" want="$3" eps="${4:-1e-4}"
  if python3 -c "import math; exit(0 if abs(float('$got')-float('$want')) <= float('$eps') else 1)" 2>/dev/null; then
    printf "  ✅  %-44s = %s  (эталон %s, ε=%s)\n" "$label" "$got" "$want" "$eps"
    PASS=$((PASS+1))
  else
    printf "  ❌  %-44s = '%s'  (эталон %s, ε=%s)\n" "$label" "$got" "$want" "$eps"
    FAIL=$((FAIL+1))
  fi
}

bbox_from_positions() {
  python3 - "$1" <<'PY'
import sys, json
d = json.loads(sys.argv[1])
p = d['positions']
xs=p[0::3]; ys=p[1::3]; zs=p[2::3]
print(min(xs), min(ys), min(zs), max(xs), max(ys), max(zs))
PY
}

volume_bbox() {
  python3 -c "print((float('$4')-float('$1'))*(float('$5')-float('$2'))*(float('$6')-float('$3')))"
}

unique_verts() {
  python3 - "$1" <<'PY'
import sys, json
d = json.loads(sys.argv[1])
p = d['positions']
s = set()
for i in range(0, len(p), 3):
    s.add((round(p[i],6), round(p[i+1],6), round(p[i+2],6)))
print(len(s))
PY
}

check_normals_unit() {
  local label="$1" json="$2"
  local bad
  bad=$(python3 - "$json" <<'PY'
import sys, json, math
d = json.loads(sys.argv[1])
n = d.get('normals', [])
bad = sum(1 for i in range(0, len(n), 3)
          if abs(math.sqrt(n[i]**2+n[i+1]**2+n[i+2]**2)-1.0) > 1e-3)
print(bad)
PY
)
  if [ "$bad" = "0" ]; then
    echo "  ✅  $label — все нормали единичные"
    PASS=$((PASS+1))
  else
    echo "  ❌  $label — $bad нормалей с |n| ≠ 1"
    FAIL=$((FAIL+1))
  fi
}

check_indices_valid() {
  local label="$1" json="$2"
  local bad
  bad=$(python3 - "$json" <<'PY'
import sys, json
d = json.loads(sys.argv[1])
vc = d['vertex_count']
bad = sum(1 for i in d['indices'] if i >= vc)
print(bad)
PY
)
  if [ "$bad" = "0" ]; then
    echo "  ✅  $label — все индексы валидны"
    PASS=$((PASS+1))
  else
    echo "  ❌  $label — $bad индексов out-of-range"
    FAIL=$((FAIL+1))
  fi
}

# ── Topology checker (Python) ─────────────────────────────────────────────────
# Принимает JSON ответа и возвращает одну строку:
#   manifold_ok  boundary_edges  degenerate_tris  volume  outward_ok  isolated_verts
run_topology() {
  python3 - "$1" <<'PY'
import sys, json, math
from collections import defaultdict

d = json.loads(sys.argv[1])
pos = d['positions']       # flat f32 array
idx = d['indices']         # flat u32 array
vc  = d['vertex_count']
tc  = d['triangle_count']

def v(i):
    b = i*3
    return (pos[b], pos[b+1], pos[b+2])

def sub(a,b):  return (a[0]-b[0], a[1]-b[1], a[2]-b[2])
def cross(a,b):
    return (a[1]*b[2]-a[2]*b[1],
            a[2]*b[0]-a[0]*b[2],
            a[0]*b[1]-a[1]*b[0])
def dot(a,b):  return a[0]*b[0]+a[1]*b[1]+a[2]*b[2]
def norm(a):   return math.sqrt(dot(a,a))

# 1. Edge usage: each undirected edge should appear exactly 2 times
edge_count = defaultdict(int)
for t in range(tc):
    a,b,c = idx[t*3], idx[t*3+1], idx[t*3+2]
    for e in ((min(a,b),max(a,b)), (min(b,c),max(b,c)), (min(a,c),max(a,c))):
        edge_count[e] += 1

boundary_edges = sum(1 for cnt in edge_count.values() if cnt != 2)
manifold_ok = (boundary_edges == 0)

# 2. Degenerate triangles (area < ε)
degen = 0
for t in range(tc):
    p0,p1,p2 = v(idx[t*3]), v(idx[t*3+1]), v(idx[t*3+2])
    area = norm(cross(sub(p1,p0), sub(p2,p0))) * 0.5
    if area < 1e-10:
        degen += 1

# 3. Signed volume via divergence theorem  V = (1/6)|Σ v0·(v1×v2)|
vol = 0.0
for t in range(tc):
    p0,p1,p2 = v(idx[t*3]), v(idx[t*3+1]), v(idx[t*3+2])
    vol += dot(p0, cross(p1, p2))
volume = abs(vol) / 6.0

# 4. Outward normals: centroid-to-face-center dot face-normal > 0
#    (uses mesh centroid as interior reference)
cx = sum(pos[0::3]) / vc
cy = sum(pos[1::3]) / vc
cz = sum(pos[2::3]) / vc
inward = 0
for t in range(tc):
    p0,p1,p2 = v(idx[t*3]), v(idx[t*3+1]), v(idx[t*3+2])
    fc = ((p0[0]+p1[0]+p2[0])/3, (p0[1]+p1[1]+p2[1])/3, (p0[2]+p1[2]+p2[2])/3)
    n = cross(sub(p1,p0), sub(p2,p0))
    outward_dir = (fc[0]-cx, fc[1]-cy, fc[2]-cz)
    if dot(n, outward_dir) < 0:
        inward += 1
outward_ok = (inward == 0)

# 5. Isolated vertices (not referenced by any triangle)
used = set(idx)
isolated = vc - len(used)

print(f"{int(manifold_ok)} {boundary_edges} {degen} {volume:.8f} {int(outward_ok)} {isolated}")
PY
}

check_topology() {
  local label="$1" json="$2" exp_vol="$3" vol_eps="${4:-1e-4}"
  local result
  result=$(run_topology "$json")
  local manifold boundary degen vol outward isolated
  read manifold boundary degen vol outward isolated <<< "$result"

  echo "  ── topology: $label"

  if [ "$manifold" = "1" ]; then
    echo "    ✅  closed manifold  (0 boundary edges)"
    PASS=$((PASS+1))
  else
    echo "    ❌  НЕ manifold — $boundary граничных рёбер (edge_usage ≠ 2)"
    FAIL=$((FAIL+1))
  fi

  if [ "$degen" = "0" ]; then
    echo "    ✅  нет вырожденных треугольников"
    PASS=$((PASS+1))
  else
    echo "    ❌  $degen вырожденных треугольников (area ≈ 0)"
    FAIL=$((FAIL+1))
  fi

  if [ "$outward" = "1" ]; then
    echo "    ✅  все нормали наружу (нет внутренних граней)"
    PASS=$((PASS+1))
  else
    echo "    ⚠   часть нормалей внутрь — возможна проблема winding"
    FAIL=$((FAIL+1))
  fi

  if [ "$isolated" = "0" ]; then
    echo "    ✅  нет изолированных вершин"
    PASS=$((PASS+1))
  else
    echo "    ❌  $isolated изолированных вершин"
    FAIL=$((FAIL+1))
  fi

  if [ -n "$exp_vol" ]; then
    check_near "    объём (дивергенция)" "$vol" "$exp_vol" "$vol_eps"
  else
    echo "    ℹ   объём (дивергенция) = $vol  м³"
  fi
}

# ─────────────────────────────────────────────────────────────────────────────
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  geometry_engine  точность & корректность"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# ── 1. Куб 20×20×20 см ───────────────────────────────────────────
echo ""
echo "▶  1. Extrude куба 20×20×20 см  (0.2 m × 0.2 m × 0.2 m)"
R=$(curl -sf -X POST "$BASE/api/matter/sketch/extrude" \
  -H "Content-Type: application/json" \
  -d '{"plane":"XZ","depth":0.2,"profile":[{"x":0,"y":0,"z":0},{"x":0.2,"y":0,"z":0},{"x":0.2,"y":0,"z":0.2},{"x":0,"y":0,"z":0.2}]}')
if [ -z "$R" ]; then echo "  ❌  нет ответа (сервер запущен?)"; FAIL=$((FAIL+1))
else
  TC=$(echo "$R" | python3 -c "import sys,json; print(json.load(sys.stdin)['triangle_count'])")
  check_eq "triangle_count (куб = 12)" "$TC" 12
  read mn_x mn_y mn_z mx_x mx_y mx_z <<< $(bbox_from_positions "$R")
  check_near "bbox min_x"  "$mn_x"  "0.0"  "1e-5"
  check_near "bbox min_z"  "$mn_z"  "0.0"  "1e-5"
  check_near "bbox max_x"  "$mx_x"  "0.2"  "1e-5"
  check_near "bbox max_z"  "$mx_z"  "0.2"  "1e-5"
  DY=$(python3 -c "print(round(float('$mx_y')-float('$mn_y'),6))")
  check_near "depth Y (= 0.2 m)"        "$DY"  "0.2"   "1e-5"
  VOL=$(volume_bbox $mn_x $mn_y $mn_z $mx_x $mx_y $mx_z)
  check_near "volume bbox (0.008 m³)"   "$VOL" "0.008" "1e-4"
  check_normals_unit  "нормали единичные" "$R"
  check_indices_valid "индексы валидны"   "$R"
fi

# ── 2. Прямоугольник 30×10×5 см ─────────────────────────────────
echo ""
echo "▶  2. Extrude прямоугольника 30×10 см, depth=5 см"
R=$(curl -sf -X POST "$BASE/api/matter/sketch/extrude" \
  -H "Content-Type: application/json" \
  -d '{"plane":"XZ","depth":0.05,"profile":[{"x":0,"y":0,"z":0},{"x":0.3,"y":0,"z":0},{"x":0.3,"y":0,"z":0.1},{"x":0,"y":0,"z":0.1}]}')
if [ -z "$R" ]; then echo "  ❌  нет ответа"; FAIL=$((FAIL+1))
else
  read mn_x mn_y mn_z mx_x mx_y mx_z <<< $(bbox_from_positions "$R")
  check_near "width X  (= 0.30 m)" "$(python3 -c "print(round(float('$mx_x')-float('$mn_x'),6))")" "0.3"  "1e-5"
  check_near "height Z (= 0.10 m)" "$(python3 -c "print(round(float('$mx_z')-float('$mn_z'),6))")" "0.1"  "1e-5"
  check_near "depth Y  (= 0.05 m)" "$(python3 -c "print(round(float('$mx_y')-float('$mn_y'),6))")" "0.05" "1e-5"
  VOL=$(volume_bbox $mn_x $mn_y $mn_z $mx_x $mx_y $mx_z)
  check_near "volume (0.0015 m³)"  "$VOL" "0.0015" "1e-5"
  check_normals_unit "нормали единичные" "$R"
fi

# ── 3. Треугольная призма ────────────────────────────────────────
echo ""
echo "▶  3. Extrude треугольного профиля  (прямоугольный треугольник 10×10 см)"
R=$(curl -sf -X POST "$BASE/api/matter/sketch/extrude" \
  -H "Content-Type: application/json" \
  -d '{"plane":"XZ","depth":0.05,"profile":[{"x":0,"y":0,"z":0},{"x":0.1,"y":0,"z":0},{"x":0,"y":0,"z":0.1}]}')
if [ -z "$R" ]; then echo "  ❌  нет ответа"; FAIL=$((FAIL+1))
else
  TC=$(echo "$R" | python3 -c "import sys,json; print(json.load(sys.stdin)['triangle_count'])")
  check_eq "triangle_count (треуг. призма = 8)" "$TC" 8
  check_normals_unit  "нормали единичные" "$R"
  check_indices_valid "индексы валидны"   "$R"
fi

# ── 4. Точность координат вершин ────────────────────────────────
echo ""
echo "▶  4. Точность координат вершин  (куб 10×10×10 см)"
R=$(curl -sf -X POST "$BASE/api/matter/sketch/extrude" \
  -H "Content-Type: application/json" \
  -d '{"plane":"XZ","depth":0.1,"profile":[{"x":0,"y":0,"z":0},{"x":0.1,"y":0,"z":0},{"x":0.1,"y":0,"z":0.1},{"x":0,"y":0,"z":0.1}]}')
if [ -z "$R" ]; then echo "  ❌  нет ответа"; FAIL=$((FAIL+1))
else
  RESULT=$(python3 - "$R" <<'PY'
import sys, json, math
d = json.loads(sys.argv[1])
p = d['positions']
xs=p[0::3]; zs=p[2::3]
bad = 0
devs = []
for v in xs+zs:
    d2 = min(abs(v-0.0), abs(v-0.1))
    devs.append(d2)
    if d2 > 1e-5: bad += 1
print(bad, f"{max(devs):.2e}", f"{sum(devs)/len(devs):.2e}")
PY
)
  BAD=$(echo $RESULT | cut -d' ' -f1)
  MAX_DEV=$(echo $RESULT | cut -d' ' -f2)
  AVG_DEV=$(echo $RESULT | cut -d' ' -f3)
  if [ "$BAD" = "0" ]; then
    echo "  ✅  все X,Z ∈ {0.0, 0.1}  |  max_err=$MAX_DEV  avg_err=$AVG_DEV"
    PASS=$((PASS+1))
  else
    echo "  ❌  $BAD координат вне {0.0, 0.1}  |  max_err=$MAX_DEV  avg_err=$AVG_DEV"
    FAIL=$((FAIL+1))
  fi

  UV=$(unique_verts "$R")
  check_eq "уникальных геометрических вершин = 8" "$UV" "8"
fi

# ── 5. Boolean subtract ──────────────────────────────────────────
echo ""
echo "▶  5. Boolean subtract: куб 20 см  −  куб 10 см (центрированный)"
R=$(curl -sf -X POST "$BASE/api/matter/geometry/boolean" \
  -H "Content-Type: application/json" \
  -d '{"op":"subtract","a":{"plane":"XZ","depth":0.2,"profile":[{"x":0,"y":0,"z":0},{"x":0.2,"y":0,"z":0},{"x":0.2,"y":0,"z":0.2},{"x":0,"y":0,"z":0.2}]},"b":{"plane":"XZ","depth":0.1,"profile":[{"x":0.05,"y":0,"z":0.05},{"x":0.15,"y":0,"z":0.05},{"x":0.15,"y":0,"z":0.15},{"x":0.05,"y":0,"z":0.15}]}}')
if [ -z "$R" ]; then echo "  ❌  нет ответа"; FAIL=$((FAIL+1))
else
  check_eq "ok" "$(echo "$R" | python3 -c "import sys,json; print(json.load(sys.stdin)['ok'])")" "True"
  VC=$(echo "$R" | python3 -c "import sys,json; print(json.load(sys.stdin)['vertex_count'])")
  check_ge "vertex_count > 0" "$VC" "1"
  read mn_x mn_y mn_z mx_x mx_y mx_z <<< $(bbox_from_positions "$R")
  check_near "bbox охватывает X=0..0.2" \
    "$(python3 -c "print(round(float('$mx_x')-float('$mn_x'),5))")" "0.2" "1e-4"
  check_indices_valid "индексы валидны" "$R"
fi

# ── 6. Boolean intersect ─────────────────────────────────────────
echo ""
echo "▶  6. Boolean intersect: два куба по 20 см, сдвиг 10 см → куб 10 см"
R=$(curl -sf -X POST "$BASE/api/matter/geometry/boolean" \
  -H "Content-Type: application/json" \
  -d '{"op":"intersect","a":{"plane":"XZ","depth":0.2,"profile":[{"x":0,"y":0,"z":0},{"x":0.2,"y":0,"z":0},{"x":0.2,"y":0,"z":0.2},{"x":0,"y":0,"z":0.2}]},"b":{"plane":"XZ","depth":0.2,"profile":[{"x":0.1,"y":0,"z":0.1},{"x":0.3,"y":0,"z":0.1},{"x":0.3,"y":0,"z":0.3},{"x":0.1,"y":0,"z":0.3}]}}')
if [ -z "$R" ]; then echo "  ❌  нет ответа"; FAIL=$((FAIL+1))
else
  VC=$(echo "$R" | python3 -c "import sys,json; print(json.load(sys.stdin)['vertex_count'])")
  check_ge "vertex_count > 0" "$VC" "1"
  if [ "$VC" -gt 0 ] 2>/dev/null; then
    read mn_x mn_y mn_z mx_x mx_y mx_z <<< $(bbox_from_positions "$R")
    check_near "intersection X_min ≈ 0.1" "$mn_x" "0.1" "1e-4"
    check_near "intersection X_max ≈ 0.2" "$mx_x" "0.2" "1e-4"
    check_near "intersection Z_min ≈ 0.1" "$mn_z" "0.1" "1e-4"
    check_near "intersection Z_max ≈ 0.2" "$mx_z" "0.2" "1e-4"
    VOL=$(volume_bbox $mn_x $mn_y $mn_z $mx_x $mx_y $mx_z)
    check_near "volume пересечения (0.002 m³)" "$VOL" "0.002" "1e-3"
  fi
  check_indices_valid "индексы валидны" "$R"
fi

# ── 7. Восьмиугольник ────────────────────────────────────────────
echo ""
echo "▶  7. Stress: восьмиугольная призма, depth=0.3 м"
R=$(curl -sf -X POST "$BASE/api/matter/sketch/extrude" \
  -H "Content-Type: application/json" \
  -d '{"plane":"XZ","depth":0.3,"profile":[{"x":0.1,"y":0,"z":0.0},{"x":0.2,"y":0,"z":0.0},{"x":0.3,"y":0,"z":0.1},{"x":0.3,"y":0,"z":0.2},{"x":0.2,"y":0,"z":0.3},{"x":0.1,"y":0,"z":0.3},{"x":0.0,"y":0,"z":0.2},{"x":0.0,"y":0,"z":0.1}]}')
if [ -z "$R" ]; then echo "  ❌  нет ответа"; FAIL=$((FAIL+1))
else
  VC=$(echo "$R" | python3 -c "import sys,json; print(json.load(sys.stdin)['vertex_count'])")
  check_ge "vertex_count >= 16" "$VC" "16"
  DY=$(python3 - "$R" <<'PY'
import sys, json
d = json.loads(sys.argv[1]); p = d['positions']; ys=p[1::3]
print(round(max(ys)-min(ys), 6))
PY
)
  check_near "depth Y восьмиугольника (= 0.3 m)" "$DY" "0.3" "1e-5"
  check_normals_unit  "нормали единичные" "$R"
  check_indices_valid "индексы валидны"   "$R"
fi

# ── 8. Ошибки валидации ──────────────────────────────────────────
echo ""
echo "▶  8. Ошибки валидации"
HTTP=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE/api/matter/sketch/extrude" \
  -H "Content-Type: application/json" \
  -d '{"plane":"XZ","depth":0.1,"profile":[{"x":0,"y":0,"z":0},{"x":1,"y":0,"z":0}]}')
check_eq "< 3 точек → 400"  "$HTTP" "400"

HTTP=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE/api/matter/sketch/extrude" \
  -H "Content-Type: application/json" \
  -d '{"plane":"XZ","depth":-1,"profile":[{"x":0,"y":0,"z":0},{"x":1,"y":0,"z":0},{"x":0.5,"y":0,"z":1}]}')
check_eq "depth <= 0 → 400"  "$HTTP" "400"

HTTP=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE/api/matter/geometry/boolean" \
  -H "Content-Type: application/json" \
  -d '{"op":"explode","a":{"plane":"XZ","depth":0.1,"profile":[{"x":0,"y":0,"z":0},{"x":1,"y":0,"z":0},{"x":0.5,"y":0,"z":1}]},"b":{"plane":"XZ","depth":0.1,"profile":[{"x":0,"y":0,"z":0},{"x":1,"y":0,"z":0},{"x":0.5,"y":0,"z":1}]}}')
check_eq "неизвестный op → 400"  "$HTTP" "400"

# ── 9. Топология: closed manifold, edge_usage=2, volume, winding ─
echo ""
echo "▶  9. Топология mesh (manifold · edge_usage · winding · volume)"

echo ""
echo "  — 9a. Куб 10×10×10 см  (ожид. объём = 0.001 м³)"
R=$(curl -sf -X POST "$BASE/api/matter/sketch/extrude" \
  -H "Content-Type: application/json" \
  -d '{"plane":"XZ","depth":0.1,"profile":[{"x":0,"y":0,"z":0},{"x":0.1,"y":0,"z":0},{"x":0.1,"y":0,"z":0.1},{"x":0,"y":0,"z":0.1}]}')
[ -n "$R" ] && check_topology "куб 10³" "$R" "0.001" "1e-5"

echo ""
echo "  — 9b. Прямоугольник 30×10×5 см  (ожид. объём = 0.0015 м³)"
R=$(curl -sf -X POST "$BASE/api/matter/sketch/extrude" \
  -H "Content-Type: application/json" \
  -d '{"plane":"XZ","depth":0.05,"profile":[{"x":0,"y":0,"z":0},{"x":0.3,"y":0,"z":0},{"x":0.3,"y":0,"z":0.1},{"x":0,"y":0,"z":0.1}]}')
[ -n "$R" ] && check_topology "прямоугольник 30×10×5" "$R" "0.0015" "1e-6"

echo ""
echo "  — 9c. Треугольная призма  (ожид. объём = 0.5×0.1×0.1×0.05 = 0.00025 м³)"
R=$(curl -sf -X POST "$BASE/api/matter/sketch/extrude" \
  -H "Content-Type: application/json" \
  -d '{"plane":"XZ","depth":0.05,"profile":[{"x":0,"y":0,"z":0},{"x":0.1,"y":0,"z":0},{"x":0,"y":0,"z":0.1}]}')
[ -n "$R" ] && check_topology "треугольная призма" "$R" "0.00025" "1e-6"

echo ""
echo "  — 9d. Восьмиугольная призма  (объём ≈ 0.006 м³)"
R=$(curl -sf -X POST "$BASE/api/matter/sketch/extrude" \
  -H "Content-Type: application/json" \
  -d '{"plane":"XZ","depth":0.3,"profile":[{"x":0.1,"y":0,"z":0.0},{"x":0.2,"y":0,"z":0.0},{"x":0.3,"y":0,"z":0.1},{"x":0.3,"y":0,"z":0.2},{"x":0.2,"y":0,"z":0.3},{"x":0.1,"y":0,"z":0.3},{"x":0.0,"y":0,"z":0.2},{"x":0.0,"y":0,"z":0.1}]}')
[ -n "$R" ] && check_topology "восьмиугольная призма" "$R" "" ""

echo ""
echo "  — 9e. Boolean subtract: куб 20 см − куб 10 см  (topology)"
R=$(curl -sf -X POST "$BASE/api/matter/geometry/boolean" \
  -H "Content-Type: application/json" \
  -d '{"op":"subtract","a":{"plane":"XZ","depth":0.2,"profile":[{"x":0,"y":0,"z":0},{"x":0.2,"y":0,"z":0},{"x":0.2,"y":0,"z":0.2},{"x":0,"y":0,"z":0.2}]},"b":{"plane":"XZ","depth":0.1,"profile":[{"x":0.05,"y":0,"z":0.05},{"x":0.15,"y":0,"z":0.05},{"x":0.15,"y":0,"z":0.15},{"x":0.05,"y":0,"z":0.15}]}}')
[ -n "$R" ] && check_topology "subtract куб−куб" "$R" "" ""

echo ""
echo "  — 9f. Boolean intersect: пересечение двух кубов  (topology)"
R=$(curl -sf -X POST "$BASE/api/matter/geometry/boolean" \
  -H "Content-Type: application/json" \
  -d '{"op":"intersect","a":{"plane":"XZ","depth":0.2,"profile":[{"x":0,"y":0,"z":0},{"x":0.2,"y":0,"z":0},{"x":0.2,"y":0,"z":0.2},{"x":0,"y":0,"z":0.2}]},"b":{"plane":"XZ","depth":0.2,"profile":[{"x":0.1,"y":0,"z":0.1},{"x":0.3,"y":0,"z":0.1},{"x":0.3,"y":0,"z":0.3},{"x":0.1,"y":0,"z":0.3}]}}')
[ -n "$R" ] && check_topology "intersect кубы" "$R" "0.002" "5e-4"

# ── Итог ─────────────────────────────────────────────────────────
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
TOTAL=$((PASS+FAIL))
if [ $FAIL -eq 0 ]; then
  echo "  🎉  Результат: $PASS/$TOTAL  ✅  — всё точно!"
else
  echo "  ⚠   Результат: $PASS/$TOTAL  ✅  |  $FAIL ❌ провалено"
fi
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
[ $FAIL -eq 0 ] && exit 0 || exit 1
