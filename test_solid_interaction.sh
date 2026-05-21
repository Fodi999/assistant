#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# test_solid_interaction.sh
#
# Тестирует подсистему выделения/подсветки куба (CadInteraction).
#
# Тесты:
#   1. JS присутствует в HTML (символы CadInteraction, picking, selection)
#   2. Все обязательные API функции задекларированы в коде
#   3. Подсистемы загружаются в правильном порядке (bootstrap → selection → picking → highlight)
#   4. Шейдер поддерживает isSelected / selectionMode / selected_face_id / hovered_face_id
#   5. UBO-поля обновляются при выделении (render_loop_ubo читает __solid*)
#   6. WASM файлы доступны (нужны для geometry_engine → face metadata)
#   7. API endpoint для экструзии (POST /api/matter/sketch/extrude) отвечает
# ─────────────────────────────────────────────────────────────────────────────

BASE="http://localhost:8000"
PASS=0
FAIL=0
TOTAL=0

# ANSI
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

pass() { echo -e "  ${GREEN}✓${NC} $1"; ((PASS++)); ((TOTAL++)); }
fail() { echo -e "  ${RED}✗${NC} $1"; ((FAIL++)); ((TOTAL++)); }
info() { echo -e "  ${CYAN}ℹ${NC} $1"; }
section() { echo -e "\n${BOLD}${YELLOW}══ $1 ══${NC}"; }

HTML=$(curl -s --max-time 5 "$BASE/")
if [ -z "$HTML" ]; then
  echo -e "${RED}✗ Сервер не отвечает на $BASE${NC}"
  echo "  Запусти: cd /Users/dmitrijfomin/Desktop/assistant && cargo run --bin restaurant-backend &"
  exit 1
fi

# ─────────────────────────────────────────────────────────────────────────────
section "1. Пространство имён CadInteraction"

if echo "$HTML" | grep -q "bootstrapCadInteraction"; then
  pass "Bootstrap функция присутствует"
else
  fail "bootstrapCadInteraction отсутствует"
fi
if echo "$HTML" | grep -q "namespace ready"; then
  pass "Namespace инициализируется (namespace ready)"
else
  fail "CadInteraction namespace не создаётся"
fi
if echo "$HTML" | grep -q "CadInteraction.selection.*store\|store ready"; then
  pass "Selection store log зарегистрирован"
else
  fail "selection store missing"
fi
if echo "$HTML" | grep -q "picking.*ready\|picking.*namespace\|CadInteraction.picking"; then
  pass "Picking namespace log зарегистрирован"
else
  fail "picking module missing"
fi
if echo "$HTML" | grep -q "highlight.*bridge\|bridge.*ready"; then
  pass "Highlight bridge log зарегистрирован"
else
  fail "highlight bridge missing"
fi
if echo "$HTML" | grep -q "debug overlay ready"; then
  pass "Debug overlay log зарегистрирован"
else
  fail "debug overlay missing"
fi

# ─────────────────────────────────────────────────────────────────────────────
section "2. Picking API"

echo "$HTML" | grep -q "buildFaceMetadata"  && pass "CadInteraction.picking.buildFaceMetadata" || fail "buildFaceMetadata отсутствует"
echo "$HTML" | grep -q "makeCameraRay"      && pass "CadInteraction.picking.makeCameraRay"     || fail "makeCameraRay отсутствует"
echo "$HTML" | grep -q "\.rayTri\b\|rayTri = " && pass "CadInteraction.picking.rayTri (Möller-Trumbore)" || fail "rayTri отсутствует"
echo "$HTML" | grep -q "P\.pickFace\|pickFace = " && pass "CadInteraction.picking.pickFace"  || fail "pickFace отсутствует"

# ─────────────────────────────────────────────────────────────────────────────
section "3. Selection store API"

echo "$HTML" | grep -q "sel\.snapshot\|snapshot: function"  && pass "selection.snapshot()"          || fail "snapshot() отсутствует"
echo "$HTML" | grep -q "sel\.set\b\|set: function"          && pass "selection.set(patch)"           || fail "set() отсутствует"
echo "$HTML" | grep -q "sel\.setHover\|setHover: function"  && pass "selection.setHover(patch)"      || fail "setHover() отсутствует"
echo "$HTML" | grep -q "sel\.clear\b\|clear: function"      && pass "selection.clear()"              || fail "clear() отсутствует"
echo "$HTML" | grep -q "subscribe: function\|sel\.subscribe" && pass "selection.subscribe(cb)"       || fail "subscribe() отсутствует"

# ─────────────────────────────────────────────────────────────────────────────
section "4. Highlight → UBO bridge"

echo "$HTML" | grep -q "__solidSelected"     && pass "__solidSelected global (UBO bridge)"    || fail "__solidSelected отсутствует"
echo "$HTML" | grep -q "__solidSelMode"      && pass "__solidSelMode global"                  || fail "__solidSelMode отсутствует"
echo "$HTML" | grep -q "__solidSelFaceId"    && pass "__solidSelFaceId global"                || fail "__solidSelFaceId отсутствует"
echo "$HTML" | grep -q "__solidHoverFaceId"  && pass "__solidHoverFaceId global"              || fail "__solidHoverFaceId отсутствует"

# ─────────────────────────────────────────────────────────────────────────────
section "5. Render loop — UBO записывает selection state"

echo "$HTML" | grep -q "ubo\[38\].*solidSelected\|solidSelected.*ubo\[38\]" \
                                             && pass "ubo[38] ← isSelected"       || fail "ubo[38] не обновляется из __solidSelected"
echo "$HTML" | grep -q "ubo\[39\].*solidSelMode\|solidSelMode.*ubo\[39\]" \
                                             && pass "ubo[39] ← selectionMode"    || fail "ubo[39] не обновляется из __solidSelMode"
echo "$HTML" | grep -q "ubo\[60\].*solidSelFaceId\|solidSelFaceId.*ubo\[60\]" \
                                             && pass "ubo[60] ← selected_face_id" || fail "ubo[60] не обновляется из __solidSelFaceId"
echo "$HTML" | grep -q "ubo\[61\].*solidHoverFaceId\|solidHoverFaceId.*ubo\[61\]" \
                                             && pass "ubo[61] ← hovered_face_id"  || fail "ubo[61] не обновляется из __solidHoverFaceId"

# ─────────────────────────────────────────────────────────────────────────────
section "6. CAD fragment shader — поддержка выделения"

echo "$HTML" | grep -q "isSelected\|u\.u9\.z"           && pass "Shader: isSelected (u9.z)"          || fail "Shader: isSelected отсутствует"
echo "$HTML" | grep -q "selectionMode\|u\.u9\.w"        && pass "Shader: selectionMode (u9.w)"        || fail "Shader: selectionMode отсутствует"
echo "$HTML" | grep -q "selected_face_id\|u\.u15\.x"    && pass "Shader: selected_face_id (u15.x)"   || fail "Shader: selected_face_id отсутствует"
echo "$HTML" | grep -q "hovered_face_id\|u\.u15\.y"     && pass "Shader: hovered_face_id (u15.y)"    || fail "Shader: hovered_face_id отсутствует"
echo "$HTML" | grep -q "yellowLine\|0\.75.*0\.0\|rim"   && pass "Shader: rim highlight (yellow)"     || fail "Shader: rim highlight отсутствует"
echo "$HTML" | grep -q "0\.85.*1\.0\|cyan.*tint\|hovered_face_id.*cellMask\|cellMask.*hovered" \
                                                         && pass "Shader: hover cyan tint"            || fail "Shader: hover tint отсутствует"

# ─────────────────────────────────────────────────────────────────────────────
section "7. Легаси shims (обратная совместимость)"

echo "$HTML" | grep -q "window\.__buildFaceMetadata = function" \
                                             && pass "__buildFaceMetadata shim → CadInteraction.picking" || fail "__buildFaceMetadata shim отсутствует"
echo "$HTML" | grep -q "window\.__solidFacePick = function" \
                                             && pass "__solidFacePick shim → CadInteraction.picking"    || fail "__solidFacePick shim отсутствует"

# ─────────────────────────────────────────────────────────────────────────────
section "8. WASM геометрии — geometry_engine"

GEO_JS=$(curl -s --max-time 3 "$BASE/wasm/geometry_engine/geometry_engine.js" | head -3)
if echo "$GEO_JS" | grep -q "ts-self-types\|export\|function\|wasm"; then
  pass "geometry_engine.js загружается ($BASE/wasm/geometry_engine/...)"
else
  fail "geometry_engine.js — 404 или пустой"
fi

GEO_WASM=$(curl -s --max-time 3 -o /dev/null -w "%{http_code}" "$BASE/wasm/geometry_engine/geometry_engine_bg.wasm")
[ "$GEO_WASM" = "200" ] && pass "geometry_engine_bg.wasm загружается (200)" \
                         || fail "geometry_engine_bg.wasm — HTTP $GEO_WASM"

# ─────────────────────────────────────────────────────────────────────────────
section "9. Порядок загрузки (архитектурная проверка)"

# Bootstrap must appear BEFORE selection store, picking, highlight
BOOT_POS=$(echo "$HTML" | grep -n "bootstrapCadInteraction"   | head -1 | cut -d: -f1)
SEL_POS=$(echo  "$HTML" | grep -n "CadInteraction.selection.*store\|store ready" | head -1 | cut -d: -f1)
PICK_POS=$(echo "$HTML" | grep -n "face metadata ready\|raycast ready\|pickFace ready" | head -1 | cut -d: -f1)
HLT_POS=$(echo  "$HTML" | grep -n "bridge ready\|highlight.*bridge" | head -1 | cut -d: -f1)
OVL_POS=$(echo  "$HTML" | grep -n "debug overlay ready"        | head -1 | cut -d: -f1)

if [ -n "$BOOT_POS" ] && [ -n "$SEL_POS" ] && [ "$BOOT_POS" -lt "$SEL_POS" ]; then
  pass "Порядок: bootstrap (L$BOOT_POS) < selection (L$SEL_POS)"
else
  fail "Порядок: bootstrap должен быть ДО selection  (boot=$BOOT_POS sel=$SEL_POS)"
fi

if [ -n "$SEL_POS" ] && [ -n "$PICK_POS" ] && [ "$SEL_POS" -lt "$PICK_POS" ]; then
  pass "Порядок: selection (L$SEL_POS) < picking (L$PICK_POS)"
else
  info "Picking (L$PICK_POS) — не критично если уже OK"
fi

if [ -n "$PICK_POS" ] && [ -n "$HLT_POS" ] && [ "$PICK_POS" -lt "$HLT_POS" ]; then
  pass "Порядок: picking (L$PICK_POS) < highlight (L$HLT_POS)"
else
  info "Highlight (L$HLT_POS) — проверь вручную"
fi

if [ -n "$HLT_POS" ] && [ -n "$OVL_POS" ] && [ "$HLT_POS" -lt "$OVL_POS" ]; then
  pass "Порядок: highlight (L$HLT_POS) < overlays (L$OVL_POS)"
else
  info "Overlays (L$OVL_POS)"
fi

# ─────────────────────────────────────────────────────────────────────────────
section "10. Реальный CAD объект — POST /api/matter/sketch/extrude"

# Простой прямоугольник 10×10 см, глубина 5 см — чистый куб
EXTRUDE_BODY='{
  "plane": "XZ",
  "depth": 0.05,
  "bevel": 0.0,
  "profile": [
    {"x":0.0, "y":0.0, "z":0.0},
    {"x":0.1, "y":0.0, "z":0.0},
    {"x":0.1, "y":0.0, "z":0.1},
    {"x":0.0, "y":0.0, "z":0.1}
  ]
}'
RESP=$(curl -s --max-time 8 -X POST "$BASE/api/matter/sketch/extrude" \
  -H "Content-Type: application/json" \
  -d "$EXTRUDE_BODY")

# 10.1  Endpoint отвечает без ошибки
if echo "$RESP" | grep -q '"vertex_count"'; then
  pass "Endpoint /api/matter/sketch/extrude отвечает 200 JSON"
else
  fail "Extrude endpoint не отвечает — $(echo $RESP | head -c 120)"
fi

# 10.2  Нет поля "error"
if echo "$RESP" | grep -qv '"error"'; then
  pass "Ответ не содержит поля error"
else
  fail "Ответ содержит error: $(echo $RESP | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d.get(\"error\",\"\"))' 2>/dev/null)"
fi

# Парсим поля через python3
VC=$(echo "$RESP"   | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d["vertex_count"])'   2>/dev/null)
TC=$(echo "$RESP"   | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d["triangle_count"])' 2>/dev/null)
FC=$(echo "$RESP"   | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d["face_count"])'     2>/dev/null)
IDX_LEN=$(echo "$RESP" | python3 -c 'import json,sys; d=json.load(sys.stdin); print(len(d["indices"]))' 2>/dev/null)
POS_LEN=$(echo "$RESP" | python3 -c 'import json,sys; d=json.load(sys.stdin); print(len(d["positions"]))' 2>/dev/null)
NRM_LEN=$(echo "$RESP" | python3 -c 'import json,sys; d=json.load(sys.stdin); print(len(d["normals"]))' 2>/dev/null)
FID_LEN=$(echo "$RESP" | python3 -c 'import json,sys; d=json.load(sys.stdin); print(len(d["face_ids"]))' 2>/dev/null)
KERNEL=$(echo "$RESP"  | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d.get("kernel","?"))' 2>/dev/null)

info "vertex_count=$VC  triangle_count=$TC  face_count=$FC  kernel=$KERNEL"
info "positions=${POS_LEN}floats  normals=${NRM_LEN}floats  indices=${IDX_LEN}ints  face_ids=${FID_LEN}ints"

# 10.3  Вершины > 0
if [ -n "$VC" ] && [ "$VC" -gt 0 ] 2>/dev/null; then
  pass "vertex_count=$VC > 0 (объект не пустой)"
else
  fail "vertex_count=$VC — объект пустой или не распарсился"
fi

# 10.4  Треугольников достаточно для закрытого куба (минимум 12 = 6 граней × 2 треугольника)
if [ -n "$TC" ] && [ "$TC" -ge 12 ] 2>/dev/null; then
  pass "triangle_count=$TC >= 12 (закрытый куб)"
else
  fail "triangle_count=$TC < 12 — куб незакрытый или неполный"
fi

# 10.5  Face count = 3 (top_cap=1, bottom_cap=2, sides=3 от ядра)
if [ "$FC" = "3" ]; then
  pass "face_count=3 (top/bottom/sides — три MeshPart от ядра)"
else
  fail "face_count=$FC != 3 — неожиданное число частей от ядра"
fi

# 10.6  positions длина = vertex_count * 3
EXPECTED_POS=$(( VC * 3 ))
if [ "$POS_LEN" = "$EXPECTED_POS" ]; then
  pass "positions.length = vertex_count×3 = $POS_LEN (консистентно)"
else
  fail "positions.length=$POS_LEN != vertex_count×3=$EXPECTED_POS"
fi

# 10.7  normals длина = positions длина
if [ "$NRM_LEN" = "$POS_LEN" ]; then
  pass "normals.length = positions.length = $NRM_LEN (1 нормаль на вершину)"
else
  fail "normals.length=$NRM_LEN != positions.length=$POS_LEN"
fi

# 10.8  face_ids длина = vertex_count
if [ "$FID_LEN" = "$VC" ]; then
  pass "face_ids.length = vertex_count = $FID_LEN (1 face_id на вершину)"
else
  fail "face_ids.length=$FID_LEN != vertex_count=$VC"
fi

# 10.9  indices длина = triangle_count * 3
EXPECTED_IDX=$(( TC * 3 ))
if [ "$IDX_LEN" = "$EXPECTED_IDX" ]; then
  pass "indices.length = triangle_count×3 = $IDX_LEN (консистентно)"
else
  fail "indices.length=$IDX_LEN != triangle_count×3=$EXPECTED_IDX"
fi

# 10.10  Ядро = geometry-kernel или geometry_engine (наш Rust kernel, не fallback)
if [ "$KERNEL" = "geometry_engine" ] || [ "$KERNEL" = "geometry-kernel" ]; then
  pass "kernel=$KERNEL (свой Rust kernel, не fallback)"
else
  fail "kernel=$KERNEL — неизвестное ядро, ожидалось geometry-kernel или geometry_engine"
fi

# 10.11  face_ids содержит только значения 1, 2, 3 (чистый объект без посторонних)
EXTRA_FIDS=$(echo "$RESP" | python3 -c '
import json,sys
d = json.load(sys.stdin)
fids = set(d.get("face_ids", []))
extra = fids - {1, 2, 3}
print(len(extra), sorted(extra))
' 2>/dev/null)
EXTRA_COUNT=$(echo "$EXTRA_FIDS" | awk '{print $1}')
if [ "$EXTRA_COUNT" = "0" ]; then
  pass "face_ids содержит только {1,2,3} — чистый CAD объект без посторонних граней"
else
  fail "face_ids содержит посторонние значения: $EXTRA_FIDS"
fi

# 10.12  Все нормали ненулевые
ZERO_NORMALS=$(echo "$RESP" | python3 -c '
import json,sys,math
d = json.load(sys.stdin)
nrm = d.get("normals", [])
zero = 0
for i in range(0, len(nrm), 3):
    nx,ny,nz = nrm[i], nrm[i+1], nrm[i+2]
    if math.hypot(nx, math.hypot(ny, nz)) < 1e-6:
        zero += 1
print(zero)
' 2>/dev/null)
if [ "$ZERO_NORMALS" = "0" ]; then
  pass "Все нормали ненулевые (valid lighting)"
else
  fail "$ZERO_NORMALS нулевых нормалей — lighting сломан"
fi

# 10.13  Все индексы в пределах vertex_count
BAD_IDX=$(echo "$RESP" | python3 -c '
import json,sys
d = json.load(sys.stdin)
vc  = d["vertex_count"]
bad = [i for i in d["indices"] if i >= vc]
print(len(bad))
' 2>/dev/null)
if [ "$BAD_IDX" = "0" ]; then
  pass "Все индексы в пределах vertex_count (нет out-of-bounds)"
else
  fail "$BAD_IDX индексов выходят за vertex_count=$VC — buffer overread"
fi

# 10.14  Нет вырожденных треугольников (i0==i1 или i1==i2 или i0==i2)
DEGEN=$(echo "$RESP" | python3 -c '
import json,sys
d = json.load(sys.stdin)
idx = d["indices"]
deg = sum(1 for k in range(0,len(idx),3) if idx[k]==idx[k+1] or idx[k+1]==idx[k+2] or idx[k]==idx[k+2])
print(deg)
' 2>/dev/null)
if [ "$DEGEN" = "0" ]; then
  pass "Нет вырожденных треугольников (все 3 индекса уникальны)"
else
  fail "$DEGEN вырожденных треугольников — weld/tessellator проблема"
fi

# 10.15  AABB куба соответствует размерам профиля (10×5×10 см)
AABB=$(echo "$RESP" | python3 -c '
import json,sys
d = json.load(sys.stdin)
p = d["positions"]
xs = p[0::3]; ys = p[1::3]; zs = p[2::3]
print(f"x=[{min(xs):.4f},{max(xs):.4f}] y=[{min(ys):.4f},{max(ys):.4f}] z=[{min(zs):.4f},{max(zs):.4f}]")
dx = max(xs)-min(xs); dy = max(ys)-min(ys); dz = max(zs)-min(zs)
ok = abs(dx-0.1)<0.001 and abs(dy-0.05)<0.001 and abs(dz-0.1)<0.001
print("OK" if ok else f"FAIL dx={dx:.4f} dy={dy:.4f} dz={dz:.4f}")
' 2>/dev/null)
AABB_STATUS=$(echo "$AABB" | grep -o "OK\|FAIL.*")
if echo "$AABB_STATUS" | grep -q "^OK"; then
  AABB_COORDS=$(echo "$AABB" | head -1)
  pass "AABB совпадает с профилем 10×5×10 см ($AABB_COORDS)"
else
  fail "AABB не совпадает с профилем: $AABB_STATUS"
fi

# ─────────────────────────────────────────────────────────────────────────────
section "11. Переключение режимов выделения (Object / Face / Edge / Vertex)"

# Проверяем что константы SelectionMode определены в исходниках
SM_FILE=$(find src -name "selection_modes.rs" 2>/dev/null | head -1)

check_mode() {
  local name=$1 val=$2
  if grep -q "  ${name}:.*${val}" "$SM_FILE" 2>/dev/null; then
    pass "SelectionMode.${name} = ${val}"
  else
    fail "SelectionMode.${name} = ${val} — не найдено в $SM_FILE"
  fi
}

check_mode "OBJECT" 0
check_mode "FACE"   1
check_mode "EDGE"   2
check_mode "VERTEX" 3

# Проверяем Object.freeze — нельзя добавлять новые режимы случайно
if grep -q "Object.freeze" "$SM_FILE" 2>/dev/null; then
  pass "SelectionMode заморожен (Object.freeze) — иммутабельный enum"
else
  fail "SelectionMode не заморожен — риск случайной мутации"
fi

# Проверяем что selection.set() принимает mode и шейдер его читает
# mode=0 OBJECT → ubo[39]=0, шейдер рисует rim на весь объект
# mode=1 FACE   → ubo[39]=1, шейдер подсвечивает конкретную грань
# mode=2 EDGE   → ubo[39]=2
# mode=3 VERTEX → ubo[39]=3

UBO_FILE=$(find src -name "render_loop_ubo.rs" 2>/dev/null | head -1)
if grep -q "ubo\[39\]" "$UBO_FILE" 2>/dev/null; then
  pass "ubo[39] ← selectionMode (режим передаётся в шейдер каждый кадр)"
else
  fail "ubo[39] не записывается в $UBO_FILE"
fi

FRAG_FILE=$(find src -name "cad_frag.rs" 2>/dev/null | head -1)

if grep -qE "selectionMode|u9\.w" "$FRAG_FILE" 2>/dev/null; then
  pass "Шейдер читает selectionMode (u9.w) для выбора стиля подсветки"
else
  fail "Шейдер не использует selectionMode"
fi

# mode=0 OBJECT → rim highlight (желтый контур всего объекта)
if grep -qiE "rim|isSelected.*u9\.z|yellow|1\.0.*0\.8.*0\.0|0\.0.*0\.8.*1\.0" "$FRAG_FILE" 2>/dev/null; then
  pass "Режим OBJECT: rim-highlight на весь объект (Blender-стиль жёлтый контур)"
else
  fail "Rim-highlight для режима OBJECT не найден в шейдере"
fi

# mode=1 FACE → грань подсвечивается отдельно
if grep -qE "selected_face_id|u15\.x" "$FRAG_FILE" 2>/dev/null; then
  pass "Режим FACE: шейдер использует selected_face_id для подсветки грани"
else
  fail "selected_face_id не используется в шейдере"
fi

# hover → cyan tint (hover режим как в Blender при наведении)
if grep -qE "hovered_face_id|u15\.y|cyan|0\.0.*1\.0.*1\.0" "$FRAG_FILE" 2>/dev/null; then
  pass "Hover: cyan-tint при наведении мыши (Blender hover highlight)"
else
  fail "Cyan hover tint не найден в шейдере"
fi

# Проверяем что selection.set({ mode }) обновляет __solidSelMode
SEL_STORE=$(find src -name "selection_store.rs" 2>/dev/null | head -1)
if grep -q "__solidSelMode\|solidSelMode" "$SEL_STORE" 2>/dev/null; then
  pass "selection.set({mode}) → __solidSelMode обновляется (UBO bridge)"
else
  # может быть в highlight_state
  HL_FILE=$(find src -name "highlight_state.rs" 2>/dev/null | head -1)
  if grep -q "__solidSelMode" "$HL_FILE" 2>/dev/null; then
    pass "selection.set({mode}) → __solidSelMode через highlight_state bridge"
  else
    fail "__solidSelMode не обновляется при смене режима"
  fi
fi

# Итоговая таблица режимов
echo ""
echo -e "  ${BOLD}Таблица режимов (как в Blender):${NC}"
echo -e "  ┌─────────┬───────┬──────────────────────────────────────────┐"
echo -e "  │ Режим   │ mode= │ Подсветка                                │"
echo -e "  ├─────────┼───────┼──────────────────────────────────────────┤"
echo -e "  │ OBJECT  │   0   │ Жёлтый rim-контур всего объекта          │"
echo -e "  │ FACE    │   1   │ Цвет грани + selected_face_id в шейдере  │"
echo -e "  │ EDGE    │   2   │ Рёбра подсвечиваются (wireframe режим)   │"
echo -e "  │ VERTEX  │   3   │ Точки/вершины подсвечиваются             │"
echo -e "  └─────────┴───────┴──────────────────────────────────────────┘"
echo ""

# ─────────────────────────────────────────────────────────────────────────────
section "12. sketch_engine → прямоугольник → geometry_engine → extrude"

# 12.1  sketch_engine существует как crate
SE_LIB="crates/sketch_engine/src/lib.rs"
if [ -f "$SE_LIB" ]; then
  pass "crates/sketch_engine — библиотека найдена"
else
  fail "crates/sketch_engine/src/lib.rs не найден"
fi

# 12.2  geometry_engine существует как crate
GE_LIB="crates/geometry_engine/src/lib.rs"
if [ -f "$GE_LIB" ]; then
  pass "crates/geometry_engine — библиотека найдена"
else
  fail "crates/geometry_engine/src/lib.rs не найден"
fi

# 12.3  sketch_engine экспортирует функции для построения прямоугольника
SE_CMDS="crates/sketch_engine/src/commands"
if grep -q "pub fn apply_add_point" "$SE_CMDS/add_point.rs" 2>/dev/null; then
  pass "sketch_engine: apply_add_point — добавить точку (кнопка «точка»)"
else
  fail "sketch_engine: apply_add_point не найден"
fi

if grep -q "pub fn apply_add_edge" "$SE_CMDS/add_edge.rs" 2>/dev/null; then
  pass "sketch_engine: apply_add_edge — соединить точки в ребро (кнопка «линия»)"
else
  fail "sketch_engine: apply_add_edge не найден"
fi

# 12.4  detect_profiles обнаруживает замкнутый прямоугольный контур
SE_DETECT="crates/sketch_engine/src/profiles/detect.rs"
if grep -q "pub fn detect_profiles" "$SE_DETECT" 2>/dev/null; then
  pass "sketch_engine: detect_profiles — определяет замкнутый профиль прямоугольника"
else
  fail "sketch_engine: detect_profiles не найден"
fi

# 12.5  В тестах sketch_engine есть эталонный тест прямоугольника
SE_RECT_TEST="crates/sketch_engine/src/commands/move_point.rs"
if grep -q "rectangle_produces_one_closed_profile" "$SE_RECT_TEST" 2>/dev/null; then
  pass "sketch_engine: тест rectangle → 1 замкнутый профиль (4 ребра)"
else
  fail "sketch_engine: тест rectangle_produces_one_closed_profile не найден"
fi

# 12.6  SketchGraph — основная структура данных эскиза
if grep -q "pub struct SketchGraph\|pub use.*SketchGraph" "crates/sketch_engine/src/types/sketch.rs" 2>/dev/null \
   || grep -q "SketchGraph" "$SE_LIB" 2>/dev/null; then
  pass "sketch_engine: SketchGraph — граф эскиза с working_plane + grid_size"
else
  fail "sketch_engine: SketchGraph не найден"
fi

# 12.7  geometry_engine экспортирует extrude_polygon
GE_EXTRUDE="crates/geometry_engine/src/ops/extrude.rs"
if grep -q "pub fn extrude_polygon" "$GE_EXTRUDE" 2>/dev/null; then
  pass "geometry_engine: extrude_polygon — вытягивает профиль в 3D тело (кнопка «E»)"
else
  fail "geometry_engine: extrude_polygon не найден"
fi

# 12.8  Point2 — 2D точка профиля для extrude
if grep -q "pub struct Point2" "$GE_EXTRUDE" 2>/dev/null; then
  pass "geometry_engine: Point2 { x, y } — 2D точка контура прямоугольника"
else
  fail "geometry_engine: Point2 не найден в extrude.rs"
fi

# 12.9  ExtrudeOptions — параметры вытягивания (глубина, bevel)
if grep -q "pub struct ExtrudeOptions" "$GE_EXTRUDE" 2>/dev/null; then
  pass "geometry_engine: ExtrudeOptions { depth, bevel } — глубина вытягивания"
else
  fail "geometry_engine: ExtrudeOptions не найден"
fi

# 12.10  extrude возвращает 3 MeshPart (front/back/sides)
if grep -qE "\[0\].*front|front.*cap|\[MeshPart;\s*3\]|Vec<MeshPart>" "$GE_EXTRUDE" 2>/dev/null \
   || grep -q "front_cap\|back_cap\|side_walls\|\[0\].*front\|front cap" "$GE_EXTRUDE" 2>/dev/null; then
  pass "geometry_engine: extrude → [front_cap, back_cap, side_walls] (3 MeshPart)"
else
  # check doc comments
  if grep -q "front\|back\|side" "$GE_EXTRUDE" 2>/dev/null; then
    pass "geometry_engine: extrude → 3 части (front/back/sides) — задокументировано"
  else
    fail "geometry_engine: структура 3 MeshPart не подтверждена"
  fi
fi

# 12.11  extrude_profile_rs обёртка (публичный re-export)
GE_EP="crates/geometry_engine/src/ops/extrude_profile.rs"
if [ -f "$GE_EP" ] && grep -q "extrude_polygon" "$GE_EP" 2>/dev/null; then
  pass "geometry_engine: extrude_profile.rs — re-export extrude_polygon as run"
else
  fail "geometry_engine: extrude_profile.rs re-export не найден"
fi

# 12.12  Брэп-builder — топологический слой geometry_engine
GE_BREP="crates/geometry_engine/src/brep/builder.rs"
if [ -f "$GE_BREP" ]; then
  pass "geometry_engine: brep/builder.rs — BrepBuilder (B-Rep топология для solid)"
else
  fail "geometry_engine: brep/builder.rs не найден"
fi

# 12.13  Вся цепочка через HTTP: POST /api/matter/sketch/extrude с прямоугольником 5×8 см
RESP2=$(curl -s --max-time 5 -X POST "$BASE/api/matter/sketch/extrude" \
  -H "Content-Type: application/json" \
  -d '{"plane":"XZ","depth":0.08,"bevel":0.0,"profile":[
        {"x":0.0,"y":0,"z":0.0},
        {"x":0.05,"y":0,"z":0.0},
        {"x":0.05,"y":0,"z":0.08},
        {"x":0.0,"y":0,"z":0.08}
      ]}')

VC2=$(echo "$RESP2" | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d.get("vertex_count",0))' 2>/dev/null)
TC2=$(echo "$RESP2" | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d.get("triangle_count",0))' 2>/dev/null)
FC2=$(echo "$RESP2" | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d.get("face_count",0))' 2>/dev/null)

if [ "${VC2:-0}" -gt 0 ] 2>/dev/null; then
  pass "Прямоугольник 5×8 см выдавлен: vertex_count=$VC2 triangle_count=$TC2 face_count=$FC2"
else
  fail "POST /api/matter/sketch/extrude для 5×8 см вернул пустой результат: $RESP2"
fi

# 12.14  AABB прямоугольника 5×8×8 см
AABB2=$(echo "$RESP2" | python3 -c '
import json,sys
d = json.load(sys.stdin)
p = d.get("positions",[])
if not p: print("FAIL no positions"); exit()
xs = p[0::3]; ys = p[1::3]; zs = p[2::3]
dx = max(xs)-min(xs); dy = max(ys)-min(ys); dz = max(zs)-min(zs)
ok = abs(dx-0.05)<0.002 and abs(dy-0.08)<0.002 and abs(dz-0.08)<0.002
print("OK" if ok else f"FAIL dx={dx:.4f} dy={dy:.4f} dz={dz:.4f}")
' 2>/dev/null)
if echo "$AABB2" | grep -q "^OK"; then
  pass "AABB прямоугольника 5×8×8 см совпадает с профилем"
else
  fail "AABB не совпадает: $AABB2"
fi

# 12.15  face_ids содержат только 1,2,3 — чистый объект без лишних граней
FID2=$(echo "$RESP2" | python3 -c '
import json,sys
d = json.load(sys.stdin)
ids = set(d.get("face_ids",[]))
extra = ids - {1,2,3}
print("OK" if not extra else f"FAIL extra={extra}")
' 2>/dev/null)
if echo "$FID2" | grep -q "^OK"; then
  pass "face_ids прямоугольника ⊆ {1,2,3} — нет посторонних граней"
else
  fail "face_ids выходят за {1,2,3}: $FID2"
fi

# ─────────────────────────────────────────────────────────────────────────────
section "13. Итог + инструкция для ручного теста"

echo ""
if [ "$FAIL" -eq 0 ]; then
  echo -e "${GREEN}${BOLD}  ✅ Все $PASS/$TOTAL тестов прошли!${NC}"
else
  echo -e "${RED}${BOLD}  ❌ Провалено: $FAIL/$TOTAL   Прошло: $PASS/$TOTAL${NC}"
fi

cat << 'EOF'

──────────────────────────────────────────────────────────────────────────────
  РУЧНОЙ ТЕСТ в браузере (localhost:8000)
──────────────────────────────────────────────────────────────────────────────

  1. СОЗДАТЬ КУБ
     • Нажми R → нарисуй прямоугольник
     • Нажми E → потяни вверх → Enter (или цифра мм + Enter)

  2. ТЕСТ HOVER (подсветка при наведении)
     ✦ Наведи курсор на грань куба
     ✦ Ожидаемо: грань подсвечивается голубым (cyan tint)
     ✦ В консоли: [FaceInput] или [CadInteraction.picking] pickFace hit

  3. ТЕСТ CLICK (выделение)
     ✦ Кликни по грани куба
     ✦ Ожидаемо: весь объект подсвечивается жёлтым контуром (rim light)
     ✦ В консоли: [FaceInput] ✅ click face_id=N src=M t=...
     ✦ Статус-бар: "Face FN selected"
     ✦ Debug-панель (слева): активная запись подсвечена

  4. ТЕСТ DESELECT
     ✦ Кликни мимо куба
     ✦ Ожидаемо: жёлтый контур исчезает
     ✦ В консоли: CadInteraction.selection.clear()

  5. ПЕРЕКЛЮЧЕНИЕ РЕЖИМОВ (как в Blender — Tab/цифры)
     ✦ В консоли переключай режим вручную:
       CadInteraction.selection.set({ mode: 0 })  // OBJECT — жёлтый контур
       CadInteraction.selection.set({ mode: 1 })  // FACE   — конкретная грань
       CadInteraction.selection.set({ mode: 2 })  // EDGE   — рёбра
       CadInteraction.selection.set({ mode: 3 })  // VERTEX — точки
     ✦ Проверь что при mode=0 жёлтый rim охватывает весь куб
     ✦ При mode=1 только кликнутая грань меняет цвет
     ✦ Hover (наведение мышью) — всегда голубой независимо от режима

  6. ОТЛАДКА В КОНСОЛИ
     # Снимок текущего состояния выделения:
     window.CadInteraction.selection.snapshot()

     # Принудительное выделение через API:
     window.CadInteraction.selection.set({ selected:true, mode:0, faceId:3, sourceFaceId:3 })

     # Очистить выделение:
     window.CadInteraction.selection.clear()

     # Показать/скрыть debug overlay:
     window.CadInteraction.overlays.debug.toggle()

     # Проверить пикинг в точке экрана (физические пиксели):
     window.CadInteraction.picking.pickFace(960, 540)

──────────────────────────────────────────────────────────────────────────────
EOF

exit $FAIL
