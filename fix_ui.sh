sed -i '' '/function syncMatterUi() {/ a\
        if (sceneState) {\
          const x = document.getElementById("tf-loc-x"); if (x !== document.activeElement) x.value = sceneState.objectPosition[0].toFixed(3);\
          const y = document.getElementById("tf-loc-y"); if (y !== document.activeElement) y.value = sceneState.objectPosition[1].toFixed(3);\
          const z = document.getElementById("tf-loc-z"); if (z !== document.activeElement) z.value = sceneState.objectPosition[2].toFixed(3);\
\
          const sx = document.getElementById("tf-scale-x"); if (sx !== document.activeElement) sx.value = sceneState.objectScale.toFixed(3);\
          const sy = document.getElementById("tf-scale-y"); if (sy !== document.activeElement) sy.value = sceneState.objectScale.toFixed(3);\
          const sz = document.getElementById("tf-scale-z"); if (sz !== document.activeElement) sz.value = sceneState.objectScale.toFixed(3);\
\
          // dimensions are approx = scale * base size (e.g. 2.0)\
          const baseSize = 2.0;\
          const dx = document.getElementById("tf-dim-x"); if (dx !== document.activeElement) dx.value = (sceneState.objectScale * baseSize).toFixed(3);\
          const dy = document.getElementById("tf-dim-y"); if (dy !== document.activeElement) dy.value = (sceneState.objectScale * baseSize).toFixed(3);\
          const dz = document.getElementById("tf-dim-z"); if (dz !== document.activeElement) dz.value = (sceneState.objectScale * baseSize).toFixed(3);\
        }\
' src/web/home/scripts/webgpu/js/matter_ui.rs

sed -i '' '/function bindMatterUi() {/ a\
        const bindInput = (id, cb) => {\
          const el = document.getElementById(id);\
          if (!el) return;\
          el.addEventListener("input", (e) => cb(parseFloat(e.target.value) || 0.0));\
        };\
        bindInput("tf-loc-x", v => sceneState.objectPosition[0] = v);\
        bindInput("tf-loc-y", v => sceneState.objectPosition[1] = v);\
        bindInput("tf-loc-z", v => sceneState.objectPosition[2] = v);\
\
        const setScale = v => sceneState.objectScale = v;\
        bindInput("tf-scale-x", setScale);\
        bindInput("tf-scale-y", setScale);\
        bindInput("tf-scale-z", setScale);\
' src/web/home/scripts/webgpu/js/matter_ui.rs
