// ── JS: unified Matter Lab UI state object ────────────────────────────────────
// Domain: Application — single source of truth that the Matter Lab panels
// read from / write to. Mirrors the existing `cam`, `shape`, `formation`,
// `cellSdf`, `NUM_SPHERES` runtime objects so the new UI can stay declarative.

pub const JS: &str = r##"
      // ── 4b. Matter Lab unified state ────────────────────────────
      const engineState = {
        screen: 'matter-lab',
        tool:   'shape',
        matter: {
          particlesM:    NUM_SPHERES / 1_000_000, // current count (starts as 1 particle)
          maxParticlesM: Math.max(1, Math.floor(MAX_PARTICLES / 1_000_000)),
          formation:     'cube',               // cloud | cube | wall
          shape:         'super-cube',         // super-cube | octa | super-sphere
          density:       1.4,                  // k particles per unit³
          noise:         22.0,
          cohesion:      0.75,
          surface:       58800,
          interior:      941200,
          frozen:        false,
        },
        action: {
          selected: 'configure-formation',
          last:     null,
        },
        performance: { fps: 0, frameMs: 0 },
      };

      // shared between render_loop and matter_ui
      const __matterPerf = { fps: 0, frameMs: 0 };

      // helpers
      function fmtMillions(n) {
        return n >= 1 ? n.toFixed(0) + 'M' : (n * 1000).toFixed(0) + 'K';
      }
      function fmtThousands(n) {
        return n >= 1000 ? (n / 1000).toFixed(1) + 'K' : n.toFixed(0);
      }
      function capitalize(s) {
        if (!s) return '';
        return s.charAt(0).toUpperCase() + s.slice(1);
      }
      function capitalizeWords(s) {
        return (s || '').split(/[\s-]+/).map(capitalize).join(' ');
      }

      // recompute interior/surface counts from formation + count
      function recomputeMatterCounts() {
        const N = NUM_SPHERES;
        const m = engineState.matter;
        if (m.formation === 'cube') {
          // side³ solid, surface = 6s² − 12s + 8
          const side = N <= 1 ? 1 : Math.max(2, Math.floor(Math.cbrt(N)));
          const surf = N <= 1 ? 1 : 6*side*side - 12*side + 8;
          m.surface  = Math.min(surf, N);
          m.interior = Math.max(0, N - m.surface);
        } else if (m.formation === 'wall') {
          // a single layer = everything is "surface"
          m.surface  = N;
          m.interior = 0;
        } else {
          // cloud: ~6% surface (thin shell heuristic)
          m.surface  = Math.round(N * 0.06);
          m.interior = N - m.surface;
        }
      }

"##;
