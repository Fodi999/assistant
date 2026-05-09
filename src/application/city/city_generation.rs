/// City generator — pure, deterministic, no DB, no async.
///
/// Input:  EconomySnapshot (from DB loader)
/// Output: CityMap (sent as JSON to frontend renderer)
///
/// Geometry model (real-city):
///   - Districts  → polygon XZ boundary (perturbed rectangle)
///   - Roads      → polyline centerline + width (not a box)
///   - Buildings  → footprint polygon (rect for now, upgradable to L/T/U shapes)
///                  + height extrude + floors count
///   - Lots       → polygon ground tiles
///
/// RNG: LCG seeded from tenant_id — same tenant always gets same layout.
use crate::application::city::economy_snapshot::EconomySnapshot;
use crate::application::city::terrain::build_city_terrain;
use crate::domain::city::*;
use crate::infrastructure::geometry::kernel::extrude::{
    extrude_polygon, ExtrudeOptions, Point2 as KernelPoint2,
};
use crate::shared::TenantId;

// ─────────────────────────────────────────────────────────────────────────────
// Constants — REAL-WORLD METERS
// ─────────────────────────────────────────────────────────────────────────────
//
// 1 backend unit = 1 metre. Frontend wraps the city in a `<group scale={...}>`
// to fit the camera (see `CityUnits.render_scale_hint`).

/// District (city block) size in metres.
pub const DISTRICT_W_M: f32 = 240.0;
pub const DISTRICT_D_M: f32 = 180.0;

/// Road widths in metres.
pub const ROAD_W_PRIMARY: f32 = 16.0;
pub const ROAD_W_SECONDARY: f32 = 8.0;

/// Floor height for building height calculations.
pub const FLOOR_H_M: f32 = 3.2;

/// Sidewalk width (reserved for future flatten-pad work).
pub const SIDEWALK_W_M: f32 = 2.5;

/// Terrain padding around city bounds and grid cell size in metres.
pub const TERRAIN_MARGIN_M: f32 = 120.0;
pub const TERRAIN_CELL_M: f32 = 4.0;

/// Suggested `<group scale={...}>` for the frontend (1 m → 0.05 world units).
pub const RENDER_SCALE_HINT: f32 = 0.05;

const STRIDE_X: f32 = DISTRICT_W_M + ROAD_W_PRIMARY; // 256
const STRIDE_Z: f32 = DISTRICT_D_M + ROAD_W_PRIMARY; // 196

/// How much district polygon corners are randomly perturbed (organic look) — metres.
const POLYGON_JITTER: f32 = 12.0;

// Backwards-compat aliases (used inside this module).
const CELL_W: f32 = DISTRICT_W_M;
const CELL_D: f32 = DISTRICT_D_M;

// ─────────────────────────────────────────────────────────────────────────────
// Public entry point
// ─────────────────────────────────────────────────────────────────────────────

pub struct CityGenerator;

impl CityGenerator {
    pub fn build(econ: &EconomySnapshot, tenant_id: TenantId) -> CityMap {
        let seed = tenant_seed(tenant_id);
        let mut gen = Gen {
            rng: Lcg::new(seed),
        };
        gen.generate(econ, seed)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal generator
// ─────────────────────────────────────────────────────────────────────────────

struct Gen {
    rng: Lcg,
}

impl Gen {
    fn generate(&mut self, econ: &EconomySnapshot, seed: u64) -> CityMap {
        // ── 1. Choose which districts exist ──────────────────────────────
        let specs = self.plan_districts(econ);

        // ── 2. Build district polygons ───────────────────────────────────
        let districts: Vec<CityDistrict> = specs
            .iter()
            .map(|spec| {
                let cx = spec.col as f32 * STRIDE_X;
                let cz = spec.row as f32 * STRIDE_Z;
                let mut local = Lcg::new(self.rng.next_u64().wrapping_add(
                    (spec.col.unsigned_abs() * 7 + spec.row.unsigned_abs() * 13 + 1) as u64 * 1000,
                ));
                self.build_district(spec, cx, cz, &mut local, econ)
            })
            .collect();

        // ── 3. Road network — polylines ──────────────────────────────────
        let roads = build_road_network(&specs);

        // ── 4. Bounds ─────────────────────────────────────────────────────
        let max_col = specs.iter().map(|d| d.col.abs()).max().unwrap_or(1) as f32;
        let max_row = specs.iter().map(|d| d.row.abs()).max().unwrap_or(1) as f32;

        let bounds_w = (max_col * 2.0 + 1.0) * STRIDE_X + CELL_W;
        let bounds_d = (max_row * 2.0 + 1.0) * STRIDE_Z + CELL_D;

        // ── 5. Terrain mesh ───────────────────────────────────────────────
        // Pad the terrain `TERRAIN_MARGIN_M` beyond the city bounds so map
        // edges sit comfortably inside the landscape (no visible cliffs).
        let terrain_width = bounds_w + TERRAIN_MARGIN_M * 2.0;
        let terrain_depth = bounds_d + TERRAIN_MARGIN_M * 2.0;
        let terrain = build_city_terrain(terrain_width, terrain_depth, TERRAIN_CELL_M, seed);

        CityMap {
            seed,
            bounds: CityBounds {
                width: bounds_w,
                depth: bounds_d,
            },
            economy: CityEconomy {
                inventory_value_cents: econ.inventory_value_cents,
                avg_profit_margin: econ.avg_profit_margin,
                assistant_progress: econ.assistant_progress,
                dish_count: econ.dish_count,
                inventory_count: econ.inventory_count,
                expiring_soon: econ.expiring_soon,
                revenue_cents: econ.revenue_cents,
                restaurant_name: econ.restaurant_name.clone(),
            },
            roads,
            districts,
            ground: CityGround {
                color: "#5a6048".into(),
                size: 1500.0, // real metres — frontend scales by render_scale_hint
                fog_color: "#7ab0e8".into(),
                fog_near: 400.0, // real metres
                fog_far: 1500.0, // real metres
            },
            terrain: Some(terrain),
            units: CityUnits {
                length_unit: "meter".into(),
                meters_per_unit: 1.0,
                render_scale_hint: RENDER_SCALE_HINT,
            },
        }
    }

    // ── District plan (economy-driven) ────────────────────────────────────

    fn plan_districts(&mut self, econ: &EconomySnapshot) -> Vec<DistrictSpec> {
        let mut specs = Vec::new();

        // Player HQ — always center
        specs.push(DistrictSpec::new(
            0,
            0,
            DistrictKind::Player,
            econ.restaurant_name.clone(),
        ));

        // Residential — always both sides
        specs.push(DistrictSpec::new(
            -1,
            0,
            DistrictKind::Residential,
            "Жилой район А".into(),
        ));
        specs.push(DistrictSpec::new(
            1,
            0,
            DistrictKind::Residential,
            "Жилой район Б".into(),
        ));

        // Park — always
        specs.push(DistrictSpec::new(
            0,
            1,
            DistrictKind::Park,
            "Городской парк".into(),
        ));

        // Competitor — always
        specs.push(DistrictSpec::new(
            -1,
            1,
            DistrictKind::Competitor,
            "Rival Kitchen".into(),
        ));

        // Office — unlocked when margins are good OR menu has dishes
        if econ.avg_profit_margin > 20.0 || econ.dish_count >= 2 {
            specs.push(DistrictSpec::new(
                0,
                -1,
                DistrictKind::Office,
                "Деловой центр".into(),
            ));
        }

        // Market — unlocked when inventory exists
        if econ.inventory_count > 0 {
            specs.push(DistrictSpec::new(
                -1,
                -1,
                DistrictKind::Market,
                "Продовольственный рынок".into(),
            ));
        }

        // Shops — unlocked when dishes exist
        if econ.dish_count > 0 {
            specs.push(DistrictSpec::new(
                1,
                -1,
                DistrictKind::Shops,
                "Торговая улица".into(),
            ));
        }

        // Industrial — warning district when products are expiring
        if econ.expiring_soon > 0 {
            specs.push(DistrictSpec::new(
                1,
                1,
                DistrictKind::Industrial,
                format!("⚠️ Склад ({} просрочка)", econ.expiring_soon),
            ));
        }

        specs
    }

    // ── Build one district ────────────────────────────────────────────────

    fn build_district(
        &mut self,
        spec: &DistrictSpec,
        cx: f32,
        cz: f32,
        rng: &mut Lcg,
        econ: &EconomySnapshot,
    ) -> CityDistrict {
        let hw = CELL_W * 0.5;
        let hd = CELL_D * 0.5;

        // Slightly perturbed rectangle polygon → organic district boundary
        let polygon = jitter_rect(cx, cz, hw, hd, POLYGON_JITTER, rng);
        let centroid = [cx, cz];

        let (ground_color, accent_color, badge) = district_theme(&spec.kind, econ);
        let unlocked =
            !matches!(spec.kind, DistrictKind::Industrial) || econ.assistant_progress >= 30;

        let buildings = self.fill_buildings(spec, rng, cx, cz, hw, hd);
        let lots = fill_lots(&spec.kind, rng, cx, cz, hw, hd);

        CityDistrict {
            id: format!("{}_{}_{}", spec.kind.as_str(), spec.col, spec.row),
            name: spec.name.clone(),
            kind: spec.kind.as_str().to_string(),
            polygon,
            centroid,
            ground_color: ground_color.into(),
            accent_color: accent_color.into(),
            buildings,
            lots,
            unlocked,
            badge,
        }
    }

    // ── Fill buildings within a district ─────────────────────────────────

    fn fill_buildings(
        &mut self,
        spec: &DistrictSpec,
        rng: &mut Lcg,
        cx: f32,
        cz: f32,
        hw: f32,
        hd: f32,
    ) -> Vec<CityBuilding> {
        // Reserve room near the road edge for sidewalks + half of the largest
        // expected building footprint.
        let margin = 25.0_f32;
        let usable_w = (hw - margin).max(10.0);
        let usable_d = (hd - margin).max(10.0);

        let count: usize = match spec.kind {
            DistrictKind::Player => 1,
            DistrictKind::Office => 5,
            DistrictKind::Market => 6,
            DistrictKind::Shops => 8,
            DistrictKind::Residential => 7,
            DistrictKind::Competitor => 2,
            DistrictKind::Park => 0,
            DistrictKind::Industrial => 4,
        };

        (0..count)
            .map(|i| {
                let bx = cx + (rng.next_f32() * 2.0 - 1.0) * usable_w;
                let bz = cz + (rng.next_f32() * 2.0 - 1.0) * usable_d;
                self.make_building(spec, rng, i, bx, bz)
            })
            .collect()
    }

    fn make_building(
        &mut self,
        spec: &DistrictSpec,
        rng: &mut Lcg,
        idx: usize,
        cx: f32,
        cz: f32,
    ) -> CityBuilding {
        // Unique per-district prefix so building ids never collide between districts
        let kind = &spec.kind;
        let p = format!("{}_{}_{}", spec.col, spec.row, idx);
        match kind {
            // ── Player HQ: prominent golden building, centred ────────────
            DistrictKind::Player => {
                let w = 18.0_f32;
                let d = 24.0_f32;
                let floors = 2u32;
                let h = floors as f32 * FLOOR_H_M;
                with_mesh(CityBuilding {
                    id: format!("player_{}", p),
                    footprint: rect_footprint(cx, cz, w, d),
                    base_y: 0.0,
                    height: h,
                    floors,
                    kind: "player".into(),
                    color: "#f5c842".into(),
                    roof_color: Some("#e8a020".into()),
                    emissive: Some("#f5c842".into()),
                    emissive_intensity: 0.35,
                    metalness: 0.1,
                    roughness: 0.6,
                    windows: true,
                    window_color: Some("#fff8d0".into()),
                    cast_shadow: true,
                    mesh: None,
                })
            }

            // ── Office: tall towers, metallic ────────────────────────────
            DistrictKind::Office => {
                // 10–30 floors, footprint 28–45 m
                let floors = 10 + (rng.next_f32() * 20.0).round() as u32;
                let h = floors as f32 * FLOOR_H_M;
                let w = 28.0 + rng.next_f32() * 17.0;
                let d = 28.0 + rng.next_f32() * 17.0;
                // Occasionally L-shaped for variety
                let footprint = if rng.next_f32() > 0.65 {
                    l_shape_footprint(cx, cz, w, d, rng)
                } else {
                    rect_footprint(cx, cz, w, d)
                };
                with_mesh(CityBuilding {
                    id: format!("office_{}", p),
                    footprint,
                    base_y: 0.0,
                    height: h,
                    floors,
                    kind: "office".into(),
                    color: pick(rng, &["#6a7888", "#7a8898", "#8899aa", "#5a6878"]).into(),
                    roof_color: Some("#3a4858".into()),
                    emissive: Some("#4060c0".into()),
                    emissive_intensity: 0.12,
                    metalness: 0.5,
                    roughness: 0.35,
                    windows: true,
                    window_color: Some("#b8d0f0".into()),
                    cast_shadow: true,
                    mesh: None,
                })
            }

            // ── Market: wide low sheds ────────────────────────────────────
            DistrictKind::Market => {
                let w = 30.0 + rng.next_f32() * 30.0; // 30–60 m
                let d = 25.0 + rng.next_f32() * 15.0; // 25–40 m
                let h = 8.0 + rng.next_f32() * 4.0; // 8–12 m
                with_mesh(CityBuilding {
                    id: format!("market_{}", p),
                    footprint: rect_footprint(cx, cz, w, d),
                    base_y: 0.0,
                    height: h,
                    floors: 1,
                    kind: "market".into(),
                    color: pick(rng, &["#c8b89a", "#d0c4a8", "#bca890", "#d4bc96"]).into(),
                    roof_color: Some("#e87030".into()),
                    emissive: None,
                    emissive_intensity: 0.0,
                    metalness: 0.05,
                    roughness: 0.9,
                    windows: false,
                    window_color: None,
                    cast_shadow: true,
                    mesh: None,
                })
            }

            // ── Shops: small low units, coloured roofs ────────────────────
            DistrictKind::Shops => {
                let w = 8.0 + rng.next_f32() * 8.0; // 8–16 m
                let d = 8.0 + rng.next_f32() * 4.0; // 8–12 m
                let floors = if rng.next_f32() > 0.5 { 2 } else { 1 };
                let h = floors as f32 * FLOOR_H_M;
                with_mesh(CityBuilding {
                    id: format!("shop_{}", p),
                    footprint: rect_footprint(cx, cz, w, d),
                    base_y: 0.0,
                    height: h,
                    floors,
                    kind: "shop".into(),
                    color: pick(rng, &["#c8b09a", "#d4b890", "#bc9880", "#e0c8a0"]).into(),
                    roof_color: Some(
                        pick(rng, &["#d06030", "#e04040", "#30a060", "#4060e0"]).to_string(),
                    ),
                    emissive: None,
                    emissive_intensity: 0.0,
                    metalness: 0.05,
                    roughness: 0.88,
                    windows: false,
                    window_color: None,
                    cast_shadow: true,
                    mesh: None,
                })
            }

            // ── Residential: medium grey blocks ───────────────────────────
            DistrictKind::Residential => {
                let gray = 130u8 + (rng.next_f32() * 60.0) as u8;
                let w = 18.0 + rng.next_f32() * 17.0; // 18–35 m
                let d = 16.0 + rng.next_f32() * 14.0; // 16–30 m
                let floors = 4 + (rng.next_f32() * 5.0).round() as u32; // 4–9
                let h = floors as f32 * FLOOR_H_M;
                with_mesh(CityBuilding {
                    id: format!("res_{}", p),
                    footprint: rect_footprint(cx, cz, w, d),
                    base_y: 0.0,
                    height: h,
                    floors,
                    kind: "residential".into(),
                    color: format!("#{:02x}{:02x}{:02x}", gray, gray, gray.saturating_sub(6)),
                    roof_color: Some("#889070".into()),
                    emissive: None,
                    emissive_intensity: 0.0,
                    metalness: 0.05,
                    roughness: 0.85,
                    windows: true,
                    window_color: Some("#d0c8a0".into()),
                    cast_shadow: true,
                    mesh: None,
                })
            }

            // ── Competitor: red-tinted building ───────────────────────────
            DistrictKind::Competitor => {
                let w = 22.0 + rng.next_f32() * 10.0; // 22–32 m
                let d = 18.0 + rng.next_f32() * 8.0; // 18–26 m
                let floors = 3 + (rng.next_f32() * 3.0).round() as u32; // 3–6
                let h = floors as f32 * FLOOR_H_M;
                with_mesh(CityBuilding {
                    id: format!("comp_{}", p),
                    footprint: rect_footprint(cx, cz, w, d),
                    base_y: 0.0,
                    height: h,
                    floors,
                    kind: "competitor".into(),
                    color: "#8a3030".into(),
                    roof_color: Some("#c04040".into()),
                    emissive: Some("#c03030".into()),
                    emissive_intensity: 0.2,
                    metalness: 0.1,
                    roughness: 0.7,
                    windows: true,
                    window_color: Some("#ff9090".into()),
                    cast_shadow: true,
                    mesh: None,
                })
            }

            // ── Industrial: orange warning glow ───────────────────────────
            DistrictKind::Industrial => {
                let w = 40.0 + rng.next_f32() * 40.0; // 40–80 m
                let d = 30.0 + rng.next_f32() * 30.0; // 30–60 m
                let h = 6.0 + rng.next_f32() * 4.0; // 6–10 m
                with_mesh(CityBuilding {
                    id: format!("ind_{}", p),
                    footprint: rect_footprint(cx, cz, w, d),
                    base_y: 0.0,
                    height: h,
                    floors: 1,
                    kind: "industrial".into(),
                    color: "#787060".into(),
                    roof_color: Some("#f59020".into()),
                    emissive: Some("#f08020".into()),
                    emissive_intensity: 0.25,
                    metalness: 0.3,
                    roughness: 0.7,
                    windows: false,
                    window_color: None,
                    cast_shadow: true,
                    mesh: None,
                })
            }

            // Park has no buildings (trees are lots)
            DistrictKind::Park => CityBuilding {
                id: format!("park_placeholder_{}", p),
                footprint: vec![],
                base_y: 0.0,
                height: 0.0,
                floors: 0,
                kind: "none".into(),
                color: "#000000".into(),
                roof_color: None,
                emissive: None,
                emissive_intensity: 0.0,
                metalness: 0.0,
                roughness: 1.0,
                windows: false,
                window_color: None,
                cast_shadow: false,
                mesh: None,
            },
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Road network builder
// ─────────────────────────────────────────────────────────────────────────────

fn build_road_network(specs: &[DistrictSpec]) -> Vec<CityRoad> {
    let mut roads = Vec::new();

    let cols: std::collections::BTreeSet<i32> = specs.iter().map(|d| d.col).collect();
    let rows: std::collections::BTreeSet<i32> = specs.iter().map(|d| d.row).collect();

    let min_col = *cols.iter().min().unwrap_or(&0);
    let max_col = *cols.iter().max().unwrap_or(&0);
    let min_row = *rows.iter().min().unwrap_or(&0);
    let max_row = *rows.iter().max().unwrap_or(&0);

    let x_start = min_col as f32 * STRIDE_X - STRIDE_X * 0.5;
    let x_end = max_col as f32 * STRIDE_X + STRIDE_X * 0.5;
    let z_start = min_row as f32 * STRIDE_Z - STRIDE_Z * 0.5;
    let z_end = max_row as f32 * STRIDE_Z + STRIDE_Z * 0.5;

    // ── Horizontal primary roads (one per row boundary) ─────────────────
    for row in min_row..=(max_row + 1) {
        let z = row as f32 * STRIDE_Z - STRIDE_Z * 0.5;
        let length = x_end - x_start;
        roads.push(CityRoad {
            id: format!("road_h_{}", row),
            polyline: vec![[x_start, z], [x_end, z]],
            width: ROAD_W_PRIMARY,
            lanes: 2,
            road_type: "primary".into(),
            color: "#3a3a42".into(),
            markings: polyline_markings(length, ROAD_W_PRIMARY),
        });
    }

    // ── Vertical primary roads (one per col boundary) ───────────────────
    for col in min_col..=(max_col + 1) {
        let x = col as f32 * STRIDE_X - STRIDE_X * 0.5;
        let length = z_end - z_start;
        roads.push(CityRoad {
            id: format!("road_v_{}", col),
            polyline: vec![[x, z_start], [x, z_end]],
            width: ROAD_W_PRIMARY,
            lanes: 2,
            road_type: "primary".into(),
            color: "#3a3a42".into(),
            markings: polyline_markings(length, ROAD_W_PRIMARY),
        });
    }

    // ── Secondary (inner) alley roads inside each district ──────────────
    for spec in specs {
        let cx = spec.col as f32 * STRIDE_X;
        let cz = spec.row as f32 * STRIDE_Z;
        let hw = CELL_W * 0.5;
        let hd = CELL_D * 0.5;

        // Horizontal alley
        roads.push(CityRoad {
            id: format!("alley_h_{}_{}", spec.col, spec.row),
            polyline: vec![[cx - hw + 8.0, cz], [cx + hw - 8.0, cz]],
            width: ROAD_W_SECONDARY,
            lanes: 1,
            road_type: "secondary".into(),
            color: "#2a2a30".into(),
            markings: vec![],
        });

        // Vertical alley
        roads.push(CityRoad {
            id: format!("alley_v_{}_{}", spec.col, spec.row),
            polyline: vec![[cx, cz - hd + 8.0], [cx, cz + hd - 8.0]],
            width: ROAD_W_SECONDARY,
            lanes: 1,
            road_type: "secondary".into(),
            color: "#2a2a30".into(),
            markings: vec![],
        });
    }

    roads
}

// ─────────────────────────────────────────────────────────────────────────────
// Lot builder
// ─────────────────────────────────────────────────────────────────────────────

fn fill_lots(
    kind: &DistrictKind,
    rng: &mut Lcg,
    cx: f32,
    cz: f32,
    hw: f32,
    hd: f32,
) -> Vec<CityLot> {
    let mut lots = Vec::new();

    match kind {
        DistrictKind::Park => {
            // Grass patches (real-world metres)
            for i in 0..4u32 {
                let lx = cx + (rng.next_f32() * 2.0 - 1.0) * hw * 0.6;
                let lz = cz + (rng.next_f32() * 2.0 - 1.0) * hd * 0.6;
                let w = 30.0 + rng.next_f32() * 30.0;
                let d = 25.0 + rng.next_f32() * 20.0;
                lots.push(CityLot {
                    id: format!("grass_{}", i),
                    polygon: rect_footprint(lx, lz, w, d),
                    kind: "grass".into(),
                    color: "#4a7830".into(),
                });
            }
            // Pond
            lots.push(CityLot {
                id: "pond_0".into(),
                polygon: rect_footprint(cx, cz + hd * 0.3, 20.0, 20.0),
                kind: "water".into(),
                color: "#2060a0".into(),
            });
        }
        DistrictKind::Market => {
            // Pavement plaza
            lots.push(CityLot {
                id: "plaza_0".into(),
                polygon: rect_footprint(cx, cz, 40.0, 30.0),
                kind: "plaza".into(),
                color: "#b0a080".into(),
            });
        }
        DistrictKind::Office => {
            // Paved plaza in front
            lots.push(CityLot {
                id: "pavement_0".into(),
                polygon: rect_footprint(cx, cz + hd * 0.6, CELL_W - 20.0, 25.0),
                kind: "pavement".into(),
                color: "#606070".into(),
            });
        }
        _ => {}
    }

    lots
}

// ─────────────────────────────────────────────────────────────────────────────
// Geometry helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Axis-aligned rectangle footprint: 4 XZ corners
fn rect_footprint(cx: f32, cz: f32, w: f32, d: f32) -> Vec<[f32; 2]> {
    let hw = w * 0.5;
    let hd = d * 0.5;
    vec![
        [cx - hw, cz - hd],
        [cx + hw, cz - hd],
        [cx + hw, cz + hd],
        [cx - hw, cz + hd],
    ]
}

/// L-shaped footprint (6 points) — used for office towers
fn l_shape_footprint(cx: f32, cz: f32, w: f32, d: f32, rng: &mut Lcg) -> Vec<[f32; 2]> {
    let hw = w * 0.5;
    let hd = d * 0.5;
    // Cut out one quadrant
    let cut_x = cx + hw * (0.3 + rng.next_f32() * 0.3);
    let cut_z = cz + hd * (0.3 + rng.next_f32() * 0.3);
    vec![
        [cx - hw, cz - hd],
        [cx + hw, cz - hd],
        [cx + hw, cut_z],
        [cut_x, cut_z],
        [cut_x, cz + hd],
        [cx - hw, cz + hd],
    ]
}

/// Perturbed rectangle for district polygon (organic boundary)
fn jitter_rect(cx: f32, cz: f32, hw: f32, hd: f32, jitter: f32, rng: &mut Lcg) -> Vec<[f32; 2]> {
    let j = |r: &mut Lcg| (r.next_f32() * 2.0 - 1.0) * jitter;
    vec![
        [cx - hw + j(rng), cz - hd + j(rng)],
        [cx + hw + j(rng), cz - hd + j(rng)],
        [cx + hw + j(rng), cz + hd + j(rng)],
        [cx - hw + j(rng), cz + hd + j(rng)],
    ]
}

/// Parametric dashed markings along a polyline (real-world metres).
fn polyline_markings(length: f32, _width: f32) -> Vec<RoadMarking> {
    let dash_every = 12.0_f32;
    let count = (length / dash_every) as usize;
    (0..count)
        .map(|i| RoadMarking {
            t: i as f32 * dash_every + dash_every * 0.5,
            length: 3.0,
            width: 0.4,
        })
        .collect()
}

fn pick<'a>(rng: &mut Lcg, choices: &[&'a str]) -> &'a str {
    let i = (rng.next_f32() * choices.len() as f32) as usize;
    choices[i.min(choices.len() - 1)]
}

// ─────────────────────────────────────────────────────────────────────────────
// District spec
// ─────────────────────────────────────────────────────────────────────────────

struct DistrictSpec {
    col: i32,
    row: i32,
    kind: DistrictKind,
    name: String,
}

impl DistrictSpec {
    fn new(col: i32, row: i32, kind: DistrictKind, name: String) -> Self {
        Self {
            col,
            row,
            kind,
            name,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Theme helper
// ─────────────────────────────────────────────────────────────────────────────

fn district_theme(
    kind: &DistrictKind,
    econ: &EconomySnapshot,
) -> (&'static str, &'static str, Option<String>) {
    let badge = match kind {
        DistrictKind::Industrial if econ.expiring_soon > 0 => {
            Some(format!("⚠️ {} просрочка", econ.expiring_soon))
        }
        DistrictKind::Office if econ.avg_profit_margin > 40.0 => Some("⭐ Высокая маржа".into()),
        DistrictKind::Player => Some("🏠 Ваш ресторан".into()),
        DistrictKind::Competitor => Some("⚔️ Конкурент".into()),
        _ => None,
    };

    let colors = match kind {
        DistrictKind::Player => ("#2a2010", "#f5c842"),
        DistrictKind::Office => ("#181c22", "#4a80f0"),
        DistrictKind::Market => ("#1a1812", "#e87030"),
        DistrictKind::Shops => ("#1e1a14", "#d04040"),
        DistrictKind::Residential => ("#1a1e18", "#70a050"),
        DistrictKind::Competitor => ("#1e1010", "#c03030"),
        DistrictKind::Park => ("#1a2818", "#40a840"),
        DistrictKind::Industrial => ("#1c1a14", "#f08020"),
    };

    (colors.0, colors.1, badge)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tenant seed
// ─────────────────────────────────────────────────────────────────────────────

fn tenant_seed(tenant_id: TenantId) -> u64 {
    let bytes = tenant_id.0.as_bytes();
    bytes.iter().enumerate().fold(0u64, |acc, (i, &b)| {
        acc.wrapping_add((b as u64).wrapping_mul(6364136223846793005u64.wrapping_pow(i as u32 + 1)))
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// LCG pseudo-random (deterministic, no std RNG dependency needed)
// ─────────────────────────────────────────────────────────────────────────────

struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_add(1),
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state
    }

    /// f32 in [0, 1)
    fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 11) as f32 / (1u64 << 53) as f32
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Geometry kernel integration
// ─────────────────────────────────────────────────────────────────────────────

/// Compute a pre-baked 3D mesh for a building footprint using the geometry kernel.
///
/// ## Coordinate space mapping
///
/// `extrude_polygon` produces a centred mesh along its Z axis:
///   - front cap at  z = +depth/2
///   - back  cap at  z = -depth/2
///   - side walls span z ∈ [-depth/2, +depth/2]
///
/// City space is Y-up, footprint in XZ plane, building extruded upward:
///   city_y = 0  → street level
///   city_y = height → roof
///
/// Kernel → city transform:
///   city_x = k_x
///   city_y = k_z + base_y + height/2   (re-centres: −h/2→base_y, +h/2→base_y+h)
///   city_z = k_y
///   normals: [nx, nz, ny]
///
/// Result: mesh Y ∈ [base_y, base_y + height]  ✓
fn extrude_footprint_to_mesh(footprint: &[[f32; 2]], base_y: f32, height: f32) -> Option<CityMesh> {
    if footprint.len() < 3 {
        return None;
    }

    let points: Vec<KernelPoint2> = footprint
        .iter()
        .map(|p| KernelPoint2::new(p[0], p[1]))
        .collect();

    let opts = ExtrudeOptions {
        depth: height,
        bevel: 0.0,
    };

    let parts = match extrude_polygon(&points, &opts) {
        Ok(p) => p,
        Err(_) => return None,
    };

    // k_z ∈ [-height/2, +height/2]  →  city_y ∈ [base_y, base_y + height]
    let half_h = height * 0.5;

    let mut positions: Vec<f32> = Vec::new();
    let mut normals: Vec<f32> = Vec::new();
    let mut uvs: Vec<f32> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut base_index: u32 = 0;

    for part in &parts {
        for &[kx, ky, kz] in &part.vertices {
            positions.push(kx);
            positions.push(kz + base_y + half_h); // k_z(-h/2..+h/2) → base_y..base_y+h
            positions.push(ky);
        }
        for &[nx, ny, nz] in &part.normals {
            normals.push(nx);
            normals.push(nz);
            normals.push(ny);
        }
        for &[u, v] in &part.uvs {
            uvs.push(u);
            uvs.push(v);
        }
        for &[a, b, c] in &part.faces {
            indices.push(base_index + a as u32);
            indices.push(base_index + b as u32);
            indices.push(base_index + c as u32);
        }
        base_index += part.vertices.len() as u32;
    }

    Some(CityMesh {
        positions,
        normals,
        uvs,
        indices,
        colors: vec![],
    })
}

/// Wrap a CityBuilding: compute its pre-baked mesh from footprint and attach it.
fn with_mesh(mut b: CityBuilding) -> CityBuilding {
    b.mesh = extrude_footprint_to_mesh(&b.footprint, b.base_y, b.height);
    b
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that extrude_footprint_to_mesh produces Y values in [base_y, base_y + height].
    ///
    /// This guards against the off-by-half bug:
    ///   extrude_polygon() centres on Z (range -h/2..+h/2).
    ///   Our transform applies kz + base_y + h/2, so:
    ///     min: -h/2 + base_y + h/2 = base_y        ✓
    ///     max: +h/2 + base_y + h/2 = base_y + h    ✓
    #[test]
    fn building_mesh_y_range() {
        let footprint = vec![[-1.0_f32, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]];
        let base_y = 0.0_f32;
        let height = 5.0_f32;

        let mesh = extrude_footprint_to_mesh(&footprint, base_y, height)
            .expect("mesh should be generated for a valid footprint");

        // Y values are at positions[1], [4], [7], … (stride 3, offset 1)
        let ys: Vec<f32> = mesh.positions.chunks(3).map(|v| v[1]).collect();

        let min_y = ys.iter().cloned().fold(f32::INFINITY, f32::min);
        let max_y = ys.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

        assert!(
            (min_y - base_y).abs() < 1e-4,
            "min Y should be base_y={base_y}, got {min_y}"
        );
        assert!(
            (max_y - (base_y + height)).abs() < 1e-4,
            "max Y should be base_y+height={}, got {max_y}",
            base_y + height
        );
    }

    /// Same test with non-zero base_y (elevated building, e.g. on a platform).
    #[test]
    fn building_mesh_y_range_elevated() {
        let footprint = vec![[0.0_f32, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0]];
        let base_y = 1.5_f32;
        let height = 3.0_f32;

        let mesh = extrude_footprint_to_mesh(&footprint, base_y, height).unwrap();

        let ys: Vec<f32> = mesh.positions.chunks(3).map(|v| v[1]).collect();
        let min_y = ys.iter().cloned().fold(f32::INFINITY, f32::min);
        let max_y = ys.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

        assert!(
            (min_y - base_y).abs() < 1e-4,
            "min Y={min_y}, expected {base_y}"
        );
        assert!(
            (max_y - (base_y + height)).abs() < 1e-4,
            "max Y={max_y}, expected {}",
            base_y + height
        );
    }
}
