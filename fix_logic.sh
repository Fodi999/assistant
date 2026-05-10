#!/bin/bash
sed -i '' -e 's/const mode = btn.dataset.mode;/const isRender = sceneState.engineMode === "PARTICLES";\
            const mode = isRender ? "CAD" : "PARTICLES";\
            btn.querySelector(".mode-label").textContent = isRender ? "RENDER" : "SOLID";\
            btn.querySelector(".mode-icon").textContent = isRender ? "⬡" : "◈";\
            btn.dataset.mode = mode;\
            if (isRender) {\
                btn.classList.remove("active");\
            } else {\
                btn.classList.add("active");\
            }/g' src/web/home/scripts/webgpu/js/matter_ui.rs

# Let's remove the querySelectorAll handling since it's just one button now
sed -i '' -e '/modeSwitcher.querySelectorAll/d' src/web/home/scripts/webgpu/js/matter_ui.rs
sed -i '' -e '/btn.classList.add(.active.);/d' src/web/home/scripts/webgpu/js/matter_ui.rs

sed -i '' -e 's/if (!mode) return;//g' src/web/home/scripts/webgpu/js/matter_ui.rs
