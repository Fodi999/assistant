# Remove the duplicate if block
sed -i '' '26,40d' src/web/home/scripts/webgpu/js/matter_ui.rs

# Now replace the remaining block
sed -i '' '19,23c\
          // dimensions = scale * baseMeshDim\
          const dx = document.getElementById("tf-dim-x"); if (dx !== document.activeElement) dx.value = (sceneState.objectScale[0] * sceneState.baseMeshDim[0]).toFixed(3);\
          const dy = document.getElementById("tf-dim-y"); if (dy !== document.activeElement) dy.value = (sceneState.objectScale[1] * sceneState.baseMeshDim[1]).toFixed(3);\
          const dz = document.getElementById("tf-dim-z"); if (dz !== document.activeElement) dz.value = (sceneState.objectScale[2] * sceneState.baseMeshDim[2]).toFixed(3);\
' src/web/home/scripts/webgpu/js/matter_ui.rs
