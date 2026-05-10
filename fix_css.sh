#!/bin/bash
sed -i '' -e 's/top: 16px;/bottom: 50px;/g' src/web/home/matter_lab_styles.rs
sed -i '' -e 's/left: 50%;/left: 16px;/g' src/web/home/matter_lab_styles.rs
sed -i '' -e 's/transform: translateX(-50%);/transform: none;/g' src/web/home/matter_lab_styles.rs
