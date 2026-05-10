#!/bin/bash
# Remove SOLID button completely
sed -i '' -e '/<button class="mode-btn active" data-mode="CAD" title="Solid Mode">/,/<\/button>/d' src/web/home/matter_lab.rs

# Let's fix the css of the mode switcher, keep it at bottom left, but also check axis and close button
sed -i '' -e 's/top: 16px;//g' src/web/home/matter_lab_styles.rs
sed -i '' -e 's/bottom: 50px;/bottom: 50px;/g' src/web/home/matter_lab_styles.rs
