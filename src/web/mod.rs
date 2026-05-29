pub mod pages;
pub mod handlers;

// ── Base ──────────────────────────────────────
const CSS_TOKENS:     &str = include_str!("../../static/chef/base/tokens.css");
const CSS_RESET:      &str = include_str!("../../static/chef/base/reset.css");
const CSS_TYPOGRAPHY: &str = include_str!("../../static/chef/base/typography.css");
const CSS_ANIMATIONS: &str = include_str!("../../static/chef/base/animations.css");

// ── Layout ────────────────────────────────────
const CSS_LAYOUT:     &str = include_str!("../../static/chef/layout.css");

// ── Components ────────────────────────────────
const CSS_NAVBAR:     &str = include_str!("../../static/chef/components/navbar.css");
const CSS_FOOTER:     &str = include_str!("../../static/chef/components/footer.css");
const CSS_BUTTONS:    &str = include_str!("../../static/chef/components/buttons.css");
const CSS_HERO:       &str = include_str!("../../static/chef/components/hero.css");
const CSS_FEATURES:   &str = include_str!("../../static/chef/components/features.css");
const CSS_STATS:      &str = include_str!("../../static/chef/components/stats.css");
const CSS_TABS:       &str = include_str!("../../static/chef/components/tabs.css");
const CSS_MENU:       &str = include_str!("../../static/chef/components/menu.css");
const CSS_RECIPES:    &str = include_str!("../../static/chef/components/recipes.css");
const CSS_ABOUT:      &str = include_str!("../../static/chef/components/about.css");
const CSS_MISC:       &str = include_str!("../../static/chef/components/misc.css");

const JS: &str = include_str!("../../static/chef/app.js");

/// Собираем все CSS-файлы в правильном порядке:
/// tokens → reset → typography → animations → layout → компоненты
fn bundle_css() -> String {
    [
        CSS_TOKENS,
        CSS_RESET,
        CSS_TYPOGRAPHY,
        CSS_ANIMATIONS,
        CSS_LAYOUT,
        CSS_NAVBAR,
        CSS_FOOTER,
        CSS_BUTTONS,
        CSS_HERO,
        CSS_FEATURES,
        CSS_STATS,
        CSS_TABS,
        CSS_MENU,
        CSS_RECIPES,
        CSS_ABOUT,
        CSS_MISC,
    ].join("\n")
}

pub fn shell(title: &str, body: &str) -> String {
    let css = bundle_css();
    format!(r#"<!DOCTYPE html>
<html lang="ru">
<head>
  <meta charset="UTF-8"/>
  <meta name="viewport" content="width=device-width,initial-scale=1.0"/>
  <title>{title} &middot; Северов</title>
  <link rel="preconnect" href="https://fonts.googleapis.com"/>
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin/>
  <link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Playfair+Display:ital,wght@0,500;0,700;1,500&family=Inter:wght@300;400;500;600&display=swap"/>
  <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/bootstrap-icons@1.11.3/font/bootstrap-icons.min.css"/>
  <style>{css}</style>
</head>
<body>
<nav class="navbar" id="navbar">
  <a href="/" class="nav-logo"><i class="bi bi-stars"></i> Северов</a>
  <div class="nav-links" id="navLinks">
    <a href="/"        class="nav-link"><i class="bi bi-house"></i> Главная</a>
    <a href="/menu"    class="nav-link"><i class="bi bi-journal-richtext"></i> Меню</a>
    <a href="/recipes" class="nav-link"><i class="bi bi-book"></i> Рецепты</a>
    <a href="/about"   class="nav-link"><i class="bi bi-person-circle"></i> О шефе</a>
  </div>
  <button class="nav-burger" id="navBurger" aria-label="Меню">
    <i class="bi bi-list" id="burgerIcon"></i>
  </button>
</nav>
<main class="main-content">
{body}
</main>
<footer class="footer">
  <span><i class="bi bi-stars" style="color:var(--accent)"></i> Алексей Северов</span>
  <span>&middot;</span>
  <span>Авторская кухня</span>
  <span>&middot;</span>
  <a href="mailto:chef@severov.ru"><i class="bi bi-envelope"></i> chef@severov.ru</a>
</footer>
<script>{JS}</script>
</body>
</html>"#, title=title, body=body, css=css, JS=JS)
}
