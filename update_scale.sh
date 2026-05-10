#!/bin/bash
sed -i '' -e 's/ubo\[35\] = sceneState.objectScale;/ubo[35] = sceneState.objectScale[0];\n    ubo[36] = sceneState.objectScale[1];\n    ubo[37] = sceneState.objectScale[2];/g' src/web/home/scripts/webgpu/js/render_loop.rs
