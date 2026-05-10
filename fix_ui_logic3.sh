sed -i '' '/bindInput("tf-scale-z", v => sceneState.objectScale\[2\] = Math.max(0.001, v));/a\
\
        // dimensions editing changes scale under the hood (Blender-like)\
        bindInput("tf-dim-x", v => sceneState.objectScale[0] = Math.max(0.001, v / sceneState.baseMeshDim[0]));\
        bindInput("tf-dim-y", v => sceneState.objectScale[1] = Math.max(0.001, v / sceneState.baseMeshDim[1]));\
        bindInput("tf-dim-z", v => sceneState.objectScale[2] = Math.max(0.001, v / sceneState.baseMeshDim[2]));\
\
        const btnApplyScale = document.getElementById("btn-apply-scale");\
        if (btnApplyScale) btnApplyScale.addEventListener("click", () => {\
          sceneState.baseMeshDim[0] *= sceneState.objectScale[0];\
          sceneState.baseMeshDim[1] *= sceneState.objectScale[1];\
          sceneState.baseMeshDim[2] *= sceneState.objectScale[2];\
          sceneState.objectScale[0] = 1.0;\
          sceneState.objectScale[1] = 1.0;\
          sceneState.objectScale[2] = 1.0;\
        });\
' src/web/home/scripts/webgpu/js/matter_ui.rs
