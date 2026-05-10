#!/bin/bash
sed -i '' -e 's/<button class="mode-btn active" data-mode="PARTICLES" title="Particle \/ Morph Mode">/<button class="mode-btn" data-mode="PARTICLES" title="Render Mode">/g' src/web/home/matter_lab.rs
sed -i '' -e 's/<span class="mode-label">PARTICLE<\/span>/<span class="mode-label">RENDER<\/span>/g' src/web/home/matter_lab.rs
sed -i '' -e 's/<button class="mode-btn" data-mode="CAD" title="Solid \/ CAD Mode">/<button class="mode-btn active" data-mode="CAD" title="Solid Mode">/g' src/web/home/matter_lab.rs
