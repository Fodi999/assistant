sed -i '' -e '/pub fn properties_panel() -> &'\'static' str {/,/    "#/!b' -e '/    "#/!d' -e '/    "#/r prop_panel.tmp' -e '/    "#/d' src/web/home/matter_panels.rs
