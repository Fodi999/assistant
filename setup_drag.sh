sed -i '' '/function bindInput(id, onChange) {/a\
\
          const makeDraggable = (input) => {\
            input.style.cursor = "col-resize";\
            let isDragging = false;\
            let startX = 0;\
            let startVal = 0;\
            let hasDragged = false;\
            let baseStep = parseFloat(input.getAttribute("step")) || 0.1;\
            \
            input.addEventListener("mousedown", (e) => {\
              startX = e.clientX;\
              startVal = parseFloat(input.value) || 0;\
              isDragging = true;\
              hasDragged = false;\
              \
              const onMove = (me) => {\
                if (!isDragging) return;\
                const dx = me.clientX - startX;\
                if (Math.abs(dx) > 2) hasDragged = true;\
                if (hasDragged) {\
                  let speed = baseStep * 0.25;\
                  if (me.shiftKey) speed *= 0.1;\
                  if (me.altKey) speed *= 10.0;\
                  \
                  const newVal = startVal + dx * speed;\
                  input.value = newVal.toFixed(3);\
                  input.dispatchEvent(new Event("input"));\
                  \
                  if (document.activeElement !== input) {\
                    input.focus();\
                  }\
                }\
              };\
              \
              const onUp = (ue) => {\
                isDragging = false;\
                window.removeEventListener("mousemove", onMove);\
                window.removeEventListener("mouseup", onUp);\
                if (hasDragged) {\
                  ue.preventDefault();\
                  input.blur();\
                }\
              };\
              \
              window.addEventListener("mousemove", onMove);\
              window.addEventListener("mouseup", onUp);\
            });\
          };' src/web/home/scripts/webgpu/js/matter_ui.rs
