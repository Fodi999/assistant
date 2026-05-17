// ── JS: unified Matter Lab UI state object ────────────────────────────────────
// Domain: Application — single source of truth that the Matter Lab panels

pub const JS: &str = r##"
      const engineState = {
        screen: 'matter-lab',
        tool:   'shape',
        matter: {
          density:       1.4,                  // k particles per unit³
          surface:       0,
          interior:      0,
        },
        action: {
          selected: 'configure-formation',
          last:     null,
        },
        performance: { fps: 0, frameMs: 0 },
      };

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
"##;
