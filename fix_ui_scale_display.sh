#!/bin/bash
sed -i '' -e 's/sy.value = sceneState.objectScale\[0\].toFixed(3);/sy.value = sceneState.objectScale\[1\].toFixed(3);/g' src/web/home/scripts/webgpu/js/matter_ui.rs
sed -i '' -e 's/sz.value = sceneState.objectScale\[0\].toFixed(3);/sz.value = sceneState.objectScale\[2\].toFixed(3);/g' src/web/home/scripts/webgpu/js/matter_ui.rs
