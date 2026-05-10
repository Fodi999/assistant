#!/bin/bash
sed -i '' -e 's/sceneState.objectScale.toFixed(3)/sceneState.objectScale[0].toFixed(3)/g' src/web/home/scripts/webgpu/js/matter_ui.rs
