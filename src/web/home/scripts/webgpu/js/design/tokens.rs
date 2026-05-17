// ── Modal Theme — единый дизайн для всех CAD floating popups ─────────────────
//
// Меняй ТОЛЬКО этот файл, чтобы изменить внешний вид ВСЕХ модальных окон:
//   • COLORS  — цветовая палитра
//   • LAYOUT  — геометрия / шрифт / тени
//
// window.__modalTheme предоставляет:
//   .COLORS                      — цветовая палитра
//   .LAYOUT                      — токены геометрии
//   .applyPopupStyle(el, opts?)  — применить базовые стили к элементу
//   .injectCSS(id, css)          — вставить <style> тег один раз по id
//   .injectBaseCSS()             — вставить общие .cad-popup-* CSS классы
//   .makeDraggable(el, handle)   — сделать попап перетаскиваемым
//   .positionNear(el, px, py)    — позиционировать у курсора, внутри viewport
//   .blockCanvasEvents(el)       — остановить орбит/pick события

pub const JS: &str = r##"
  // ═══════════════════════════════════════════════════════════════════════════
  //  MODAL THEME — меняй здесь, все попапы обновятся автоматически
  // ═══════════════════════════════════════════════════════════════════════════
  window.__modalTheme = (function() {

    // ──────────────────────────────────────────────────────────────────────
    // ЦВЕТА — единая палитра для всех модальных окон
    // ──────────────────────────────────────────────────────────────────────
    const COLORS = {
      // ── Base ─────────────────────────────────────────────────────
      bg:          'rgba(15,23,42,0.97)',       // фон попапа
      bgHud:       'rgba(2,6,23,0.82)',         // фон HUD (чуть темнее)
      panel:       'rgba(30,41,59,0.9)',        // фон инпутов и кнопок
      panelSolid:  '#1e293b',                   // сплошной фон (разделители HUD)
      panelActive: '#0e7490',                   // активная кнопка формации
      panelBorder: '#334155',                   // неактивный бордер кнопок HUD
      border:      'rgba(148,163,184,0.35)',    // бордер кнопок/инпутов
      frame:       'rgba(226,232,240,0.25)',    // бордер попапа и разделители
      frameHud:    'rgba(103,232,249,0.35)',    // бордер HUD (cyan tint)
      // ── Text ─────────────────────────────────────────────────────
      fg:      '#e2e8f0',   // основной текст
      mute:    '#94a3b8',   // второстепенный текст / лейблы
      dim:     '#64748b',   // подсказки
      dimDark: '#475569',   // совсем тусклый (interior count)
      input:   '#f1f5f9',   // текст значений в инпутах
      // ── Accent ───────────────────────────────────────────────────
      accent:  'rgba(99,102,241,0.85)',  // акцентная кнопка (фон)
      accent2: 'rgba(99,102,241,1.0)',   // акцентная кнопка (бордер) / focus
      info:    '#67e8f9',   // cyan — заголовки HUD, активная формация border
      violet:  '#a78bfa',   // количество частиц, surface count
      pink:    '#f0abfc',   // debug hide-low
      warn2:   '#fcd34d',   // dist / cam coordinates
      // ── Semantic ─────────────────────────────────────────────────
      danger:  '#f87171',   // ошибка
      warn:    '#fbbf24',   // предупреждение / shape label
      ok:      '#34d399',   // успех
    };

    // ──────────────────────────────────────────────────────────────────────
    // ГЕОМЕТРИЯ — радиусы, тени, шрифт
    // ──────────────────────────────────────────────────────────────────────
    const LAYOUT = {
      borderRadius: '8px',
      shadow:       '0 8px 32px rgba(0,0,0,0.7)',
      font:         '"JetBrains Mono", system-ui, monospace',
      fontSize:     '12px',
      padding:      '12px 14px',
    };

    // ──────────────────────────────────────────────────────────────────────
    // Применить базовые стили к элементу-попапу
    //   opts.zIndex   — z-index (default '9998')
    //   opts.minWidth — минимальная ширина (default '240px')
    //   opts.maxWidth — максимальная ширина (default '320px')
    // ──────────────────────────────────────────────────────────────────────
    function applyPopupStyle(el, opts) {
      opts = opts || {};
      Object.assign(el.style, {
        position:      'fixed',
        zIndex:        opts.zIndex    || '9998',
        display:       'none',
        background:    COLORS.bg,
        border:        '1px solid ' + COLORS.frame,
        borderRadius:  LAYOUT.borderRadius,
        padding:       LAYOUT.padding,
        boxShadow:     LAYOUT.shadow,
        fontFamily:    LAYOUT.font,
        fontSize:      LAYOUT.fontSize,
        color:         COLORS.fg,
        minWidth:      opts.minWidth  || '240px',
        maxWidth:      opts.maxWidth  || '320px',
        pointerEvents: 'all',
        userSelect:    'none',
      });
    }

    // ──────────────────────────────────────────────────────────────────────
    // Вставить <style> тег один раз (по уникальному id)
    // ──────────────────────────────────────────────────────────────────────
    function injectCSS(id, css) {
      if (!document.getElementById(id)) {
        const s = document.createElement('style');
        s.id = id;
        s.textContent = css;
        document.head.appendChild(s);
      }
    }

    // ──────────────────────────────────────────────────────────────────────
    // Общие CSS классы — используются во всех попапах
    // Изменяй здесь → все попапы обновятся
    // ──────────────────────────────────────────────────────────────────────
    function injectBaseCSS() {
      injectCSS('__cad-popup-base-style', `
        /* Сетка label:value */
        .cad-popup-grid {
          display:grid; grid-template-columns:60px 1fr;
          gap:2px 8px; margin:0 0 6px; font-size:11px;
        }
        .cad-popup-grid dt { color:${COLORS.mute}; margin:0; }
        .cad-popup-grid dd { color:${COLORS.fg}; margin:0;
          text-align:right; font-weight:600; }

        /* Горизонтальный разделитель */
        .cad-popup-sep { height:1px; background:${COLORS.frame}; margin:6px 0; }

        /* Обычная кнопка */
        .cad-popup-btn {
          background:${COLORS.panel};
          border:1px solid ${COLORS.border};
          border-radius:4px;
          color:${COLORS.fg};
          font-family:${LAYOUT.font}; font-size:11px;
          padding:5px 8px; cursor:pointer;
        }
        .cad-popup-btn:hover { filter:brightness(1.2); }

        /* Акцентная (primary action) кнопка */
        .cad-popup-btn-accent {
          background:${COLORS.accent};
          border-color:${COLORS.accent2};
          color:#fff;
        }

        /* Заголовок — drag handle */
        .cad-popup-titlebar {
          display:flex; justify-content:space-between; align-items:center;
          margin-bottom:8px; cursor:grab; user-select:none;
        }
        .cad-popup-titlebar:active { cursor:grabbing; }
        .cad-popup-title {
          font-size:11px; color:${COLORS.mute};
          text-transform:uppercase; letter-spacing:0.5px; font-weight:700;
          pointer-events:none;
        }

        /* Кнопка закрытия */
        .cad-popup-close {
          background:none; border:none; color:${COLORS.mute};
          font-size:14px; cursor:pointer; padding:0 2px; line-height:1;
          pointer-events:all;
        }
        .cad-popup-close:hover { color:${COLORS.fg}; }

        /* Подсказка внизу */
        .cad-popup-hint { margin-top:4px; font-size:9px; color:${COLORS.dim}; }

        /* Строка статуса */
        .cad-popup-msg { min-height:14px; margin-top:6px; font-size:11px; color:${COLORS.mute}; }
      `);
    }

    // ──────────────────────────────────────────────────────────────────────
    // Сделать попап перетаскиваемым
    //   el     — сам попап (двигается)
    //   handle — элемент за который тащат (обычно titlebar)
    //            если handle === el, пропускаем клики по inputs/buttons
    // ──────────────────────────────────────────────────────────────────────
    function makeDraggable(el, handle) {
      let active = false, ox = 0, oy = 0;
      const wholePopup = (handle === el);

      handle.addEventListener('pointerdown', e => {
        if (e.button !== 0) return;
        // Всегда пропускаем кликабельные элементы — и когда handle=el, и когда handle=titlebar
        if (e.target.closest('input, button, label, select, a')) return;
        active = true;
        const r = el.getBoundingClientRect();
        ox = e.clientX - r.left;
        oy = e.clientY - r.top;
        handle.setPointerCapture(e.pointerId);
        el.style.cursor = 'grabbing';
        e.preventDefault();
        e.stopPropagation();
      }, true);

      handle.addEventListener('pointermove', e => {
        if (!active) return;
        const vw = window.innerWidth, vh = window.innerHeight;
        const left = Math.max(0, Math.min(vw - el.offsetWidth,  e.clientX - ox));
        const top  = Math.max(0, Math.min(vh - el.offsetHeight, e.clientY - oy));
        el.style.left = left + 'px';
        el.style.top  = top  + 'px';
        e.stopPropagation();
      }, true);

      handle.addEventListener('pointerup', e => {
        if (!active) return;
        active = false;
        el.style.cursor = '';
        e.stopPropagation();
      }, true);

      handle.addEventListener('pointercancel', () => { active = false; el.style.cursor = ''; });
    }

    // ──────────────────────────────────────────────────────────────────────
    // Позиционировать попап рядом с курсором, не выходя за viewport
    // ──────────────────────────────────────────────────────────────────────
    function positionNear(el, px, py) {
      el.style.display = 'block';
      const vw = window.innerWidth, vh = window.innerHeight;
      const w = el.offsetWidth || 260, h = el.offsetHeight || 320;
      let left = px + 14, top = py - 14;
      if (left + w > vw - 8) left = px - w - 14;
      if (top  + h > vh - 8) top  = vh - h - 8;
      if (left < 8) left = 8;
      if (top  < 8) top  = 8;
      el.style.left = left + 'px';
      el.style.top  = top  + 'px';
    }

    // ──────────────────────────────────────────────────────────────────────
    // Блокировать orbit/pick события от канваса
    // ──────────────────────────────────────────────────────────────────────
    function blockCanvasEvents(el) {
      ['pointerdown','mousedown'].forEach(ev =>
        el.addEventListener(ev, e => e.stopPropagation(), true));
      ['click','dblclick','contextmenu'].forEach(ev =>
        el.addEventListener(ev, e => e.stopPropagation(), false));
    }

    // ──────────────────────────────────────────────────────────────────────
    // Применить стиль HUD-панели (info/perf overlay, не popup)
    //   opts.top / right / bottom / left — позиция (default top:72px right:356px)
    //   opts.zIndex — default '9999'
    // ──────────────────────────────────────────────────────────────────────
    function applyHudStyle(el, opts) {
      opts = opts || {};
      Object.assign(el.style, {
        position:         'fixed',
        top:              opts.top    || '72px',
        right:            opts.right  || '356px',
        zIndex:           opts.zIndex || '9999',
        padding:          '10px 14px',
        borderRadius:     LAYOUT.borderRadius,
        background:       COLORS.bgHud,
        backdropFilter:   'blur(10px)',
        webkitBackdropFilter: 'blur(10px)',
        border:           '1px solid ' + COLORS.frameHud,
        font:             '500 ' + LAYOUT.fontSize + '/1.5 ' + LAYOUT.font,
        color:            COLORS.fg,
        letterSpacing:    '.02em',
        pointerEvents:    'auto',
        userSelect:       'none',
        boxShadow:        LAYOUT.shadow,
        display:          opts.display || 'none',
      });
    }

    // Вставить базовые классы сразу при регистрации темы
    injectBaseCSS();

    return {
      COLORS, LAYOUT,
      applyPopupStyle, applyHudStyle,
      injectCSS, injectBaseCSS,
      makeDraggable, positionNear, blockCanvasEvents,
    };
  })();
"##;
