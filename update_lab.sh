#!/bin/bash
sed -i '' -e "s/engineMode:     'PARTICLES',/engineMode:     'CAD',/g" src/web/home/scripts/webgpu/js/state.rs
