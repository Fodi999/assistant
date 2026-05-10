sed -i '' '/baseMeshDim:    \[2.0, 2.0, 2.0\],/d' src/web/home/scripts/webgpu/js/state.rs
sed -i '' 's/objectScale:    \[1.0, 1.0, 1.0\],/objectScale:    \[1.0, 1.0, 1.0\], \/\/ Массив scale X Y Z\n        baseMeshDim:    \[2.0, 2.0, 2.0\],/g' src/web/home/scripts/webgpu/js/state.rs
