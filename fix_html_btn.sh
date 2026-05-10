#!/bin/bash
# Remove active class from HTML so it's not permanently active
sed -i '' -e 's/<button class="mode-btn active" data-mode="PARTICLES" title="Render Mode">/<button class="mode-btn" data-mode="PARTICLES" title="Mode Switch">/g' src/web/home/matter_lab.rs
sed -i '' -e 's/<button class="mode-btn" data-mode="PARTICLES" title="Render Mode">/<button class="mode-btn" data-mode="PARTICLES" title="Mode Switch">/g' src/web/home/matter_lab.rs
