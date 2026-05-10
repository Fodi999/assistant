#!/bin/bash
sed -i '' -e 's/const ubo = new Float32Array(44); \/\/ 11 × vec4/const ubo = new Float32Array(48); \/\/ 12 × vec4/g' src/web/home/scripts/webgpu/js/render_loop.rs
