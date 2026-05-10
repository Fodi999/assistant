sed -i '' '/const makeDraggable = (input) => {/a\
\
          // Style wrapper to allow custom arrows\
          const parent = input.parentElement;\
          const btnDec = document.createElement("button");\
          btnDec.innerText = "◀";\
          btnDec.style.cssText = "background:transparent; border:none; color:#475569; font-size:8px; padding:0 4px; cursor:pointer;";\
          const btnInc = document.createElement("button");\
          btnInc.innerText = "▶";\
          btnInc.style.cssText = "background:transparent; border:none; color:#475569; font-size:8px; padding:0 8px 0 4px; cursor:pointer;";\
          \
          btnDec.addEventListener("click", () => {\
            let v = parseFloat(input.value) || 0;\
            let step = parseFloat(input.getAttribute("step")) || 0.1;\
            input.value = (v - step).toFixed(3);\
            input.dispatchEvent(new Event("input"));\
          });\
          btnInc.addEventListener("click", () => {\
            let v = parseFloat(input.value) || 0;\
            let step = parseFloat(input.getAttribute("step")) || 0.1;\
            input.value = (v + step).toFixed(3);\
            input.dispatchEvent(new Event("input"));\
          });\
          \
          // Insert arrows after input\
          parent.appendChild(btnDec);\
          parent.appendChild(btnInc);\
' src/web/home/scripts/webgpu/js/matter_ui.rs
