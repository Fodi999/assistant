// ── JS: Render loop — собирается из 6 подмодулей ─────────────────────────────
//
//  Структура:
//  ┌─ render_loop_ubo              — камера, UBO, WebGPU passes (bg + cad)
//  ├─ render_loop_overlay_grid     — w2s(), сетка, оси, орбит-кольцо
//  ├─ render_loop_overlay_sketch   — профили, стены, экструд, рёбра, точки
//  ├─ render_loop_overlay_cursor   — курсор точности / snap-маркер
//  ├─ render_loop_overlay_drafting — размеры, лейблы, центровые, линейка
//  └─ render_loop_raf              — статус-баннер, гизмо, copy-preview, RAF
//
//  Снаружи всё по-прежнему доступно как `core::render_loop::JS`.

use const_format::concatcp;

mod render_loop_ubo;
mod render_loop_overlay_grid;
mod render_loop_overlay_sketch;
mod render_loop_overlay_cursor;
mod render_loop_overlay_drafting;
mod render_loop_raf;

pub const JS: &str = concatcp!(
    render_loop_ubo::JS,
    render_loop_overlay_grid::JS,
    render_loop_overlay_sketch::JS,
    render_loop_overlay_cursor::JS,
    render_loop_overlay_drafting::JS,
    render_loop_raf::JS,
);
