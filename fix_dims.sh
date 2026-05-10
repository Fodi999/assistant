#!/bin/bash
sed -i '' -e 's/const dx = document.getElementById("tf-dim-x"); if (dx !== document.activeElement) dx.value = (sceneState.objectScale \* baseSize).toFixed(3);/const dx = document.getElementById("tf-dim-x"); if (dx !== document.activeElement) dx.value = (sceneState.objectScale[0] \* baseSize).toFixed(3);/g' src/web/home/scripts/webgpu/js/matter_ui.rs

sed -i '' -e 's/const dy = document.getElementById("tf-dim-y"); if (dy !== document.activeElement) dy.value = (sceneState.objectScale \* baseSize).toFixed(3);/const dy = document.getElementById("tf-dim-y"); if (dy !== document.activeElement) dy.value = (sceneState.objectScale[1] \* baseSize).toFixed(3);/g' src/web/home/scripts/webgpu/js/matter_ui.rs

sed -i '' -e 's/const dz = document.getElementById("tf-dim-z"); if (dz !== document.activeElement) dz.value = (sceneState.objectScale \* baseSize).toFixed(3);/const dz = document.getElementById("tf-dim-z"); if (dz !== document.activeElement) dz.value = (sceneState.objectScale[2] \* baseSize).toFixed(3);/g' src/web/home/scripts/webgpu/js/matter_ui.rs
