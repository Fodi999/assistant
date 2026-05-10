#!/bin/bash
# Move Close button to top right
sed -i '' -e '/\.close-engine-btn {/,/bottom: 50px; left: 16px;/ s/bottom: 50px; left: 16px;/top: 16px; right: 16px;/' src/web/home/matter_lab_styles.rs

# Move gizmo to top right (below close or near it?) Let's put gizmo exactly at top right corner
sed -i '' -e '/#axis-gizmo {/,/bottom: 50px;/ s/bottom: 50px;/top: 60px;/' src/web/home/matter_lab_styles.rs
