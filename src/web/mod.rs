pub mod handlers;
pub mod language;
pub mod pages;

// ── Base ──────────────────────────────────────
const CSS_TOKENS: &str = include_str!("../../static/chef/base/tokens.css");
const CSS_RESET: &str = include_str!("../../static/chef/base/reset.css");
const CSS_TYPOGRAPHY: &str = include_str!("../../static/chef/base/typography.css");
const CSS_ANIMATIONS: &str = include_str!("../../static/chef/base/animations.css");

// ── Layout ────────────────────────────────────
const CSS_LAYOUT: &str = include_str!("../../static/chef/layout.css");

// ── Components ────────────────────────────────
const CSS_NAVBAR: &str = include_str!("../../static/chef/components/navbar.css");
const CSS_FOOTER: &str = include_str!("../../static/chef/components/footer.css");
const CSS_BUTTONS: &str = include_str!("../../static/chef/components/buttons.css");
const CSS_HERO: &str = include_str!("../../static/chef/components/hero.css");
const CSS_FEATURES: &str = include_str!("../../static/chef/components/features.css");
const CSS_STATS: &str = include_str!("../../static/chef/components/stats.css");
const CSS_TABS: &str = include_str!("../../static/chef/components/tabs.css");
const CSS_MENU: &str = include_str!("../../static/chef/components/menu.css");
const CSS_RECIPES: &str = include_str!("../../static/chef/components/recipes.css");
const CSS_ABOUT: &str = include_str!("../../static/chef/components/about.css");
const CSS_MISC: &str = include_str!("../../static/chef/components/misc.css");

const JS: &str = include_str!("../../static/chef/app.js");

/// Bundle CSS files in the required order.
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
    ]
    .join("\n")
}

pub fn shell(lang: language::Lang, title: &str, body: &str) -> String {
    let css = bundle_css();
    let pack = lang.pack();
    let t = pack.shell;
    let language_options: String = language::LANG_OPTIONS
        .iter()
        .map(|(code, label)| {
            format!(
                r#"<a href="?lang={code}" class="lang-option{active}" data-lang-option="{code}" aria-label="{nav_language}: {label}">{label}</a>"#,
                code = code,
                label = label,
                nav_language = t.nav_language,
                active = if *code == pack.code { " active" } else { "" }
            )
        })
        .collect();
    format!(
        r##"<!DOCTYPE html>
<html lang="{html_lang}">
<head>
  <meta charset="UTF-8"/>
  <meta name="viewport" content="width=device-width,initial-scale=1.0"/>
  <title>{title} &middot; Dima Fomin Chef</title>
  <link rel="preconnect" href="https://fonts.googleapis.com"/>
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin/>
  <link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Manrope:wght@400;500;600;700;800&family=Onest:wght@400;500;600;700;800&display=swap"/>
  <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/bootstrap-icons@1.11.3/font/bootstrap-icons.min.css"/>
  <script src="https://unpkg.com/lucide@latest/dist/umd/lucide.min.js"></script>
  <style>{css}</style>
</head>
<body>

<!-- ── AMORPHOUS AURORA BACKGROUND ─────────── -->
<div class="aurora" aria-hidden="true">
  <span class="aurora-blob blob-1"></span>
  <span class="aurora-blob blob-2"></span>
  <span class="aurora-blob blob-3"></span>
  <span class="aurora-blob blob-4"></span>
</div>

<!-- ── NAVBAR ──────────────────────────────── -->
<nav class="site-nav" id="navbar">
  <div class="nav-inner">
    <a href="/" class="nav-logo">
      <div class="nav-logo-icon"><i data-lucide="chef-hat"></i></div>
      {brand_plain}<span>{brand_accent}</span>
    </a>
    <div class="nav-links" id="navLinks">
      <a href="/"         class="nav-link active">{nav_start}</a>
      <a href="/menu"     class="nav-link">{nav_menu}</a>
      <a href="/delivery" class="nav-link">{nav_delivery}</a>
      <a href="/booking"  class="nav-link">{nav_booking}</a>
      <a href="/about"    class="nav-link">{nav_about}</a>
    </div>
    <div class="nav-actions">
      <div class="lang-switcher" role="group" aria-label="{nav_language}" data-current-lang="{current_lang}">
        {language_options}
      </div>
      <a href="/booking" class="btn btn-ghost btn-sm">{nav_table}</a>
      <a href="/menu"    class="btn btn-primary btn-sm">{nav_order}</a>
    </div>
    <button class="nav-toggle" id="navToggle" aria-label="{aria_menu}">
      <i class="bi bi-list" id="navToggleIcon"></i>
    </button>
  </div>
</nav>

<main class="main-content">
{body}
</main>

<div class="cookie-banner" id="cookieBanner" role="dialog" aria-live="polite" aria-label="{cookie_aria}">
  <div>
    <strong>{cookie_title}</strong>
    <p>{cookie_intro}</p>
  </div>
  <div class="cookie-actions">
    <button class="btn btn-ghost btn-sm" type="button" data-cookie-choice="necessary">{cookie_necessary}</button>
    <button class="btn btn-primary btn-sm" type="button" data-cookie-choice="all">{cookie_accept}</button>
  </div>
</div>

<!-- ── FOOTER ──────────────────────────────── -->
<footer class="site-footer">
  <div class="container">

    <div class="security-bar">
      <span class="security-item"><i class="bi bi-clock-fill"></i> {trust_delivery}</span>
      <span class="security-sep"></span>
      <span class="security-item"><i class="bi bi-bag-check-fill"></i> {trust_pickup}</span>
      <span class="security-sep"></span>
      <span class="security-item"><i class="bi bi-calendar2-check"></i> {trust_booking}</span>
      <span class="security-sep"></span>
      <span class="security-item"><i class="bi bi-stars"></i> {trust_author}</span>
    </div>

    <!-- Main footer -->
    <div class="footer-inner">
      <div>
        <a href="/" class="footer-logo">
          <div class="footer-logo-icon"><i data-lucide="chef-hat"></i></div>
          {brand_plain}<span>{brand_accent}</span>
        </a>
        <p class="footer-tagline">{footer_tagline}</p>
      </div>
      <div>
        <p class="footer-heading">{footer_guests}</p>
        <div class="footer-links">
          <a href="/menu">{footer_menu}</a>
          <a href="/delivery">{footer_delivery}</a>
          <a href="/booking">{footer_booking}</a>
          <a href="/recipes">{footer_blog}</a>
        </div>
      </div>
      <div>
        <p class="footer-heading">{footer_restaurant}</p>
        <div class="footer-links">
          <a href="/about">{footer_about}</a>
          <a href="#">FISH in HOUSE</a>
          <a href="#">{footer_haccp}</a>
          <a href="/cookie">{footer_cookie}</a>
        </div>
      </div>
      <div>
        <p class="footer-heading">{footer_contact}</p>
        <div class="footer-links">
          <a href="tel:+48000000000">+48 000 000 000</a>
          <a href="mailto:chef@example.com">chef@example.com</a>
          <a href="/privacy">{footer_privacy}</a>
          <a href="/terms">{footer_terms}</a>
        </div>
      </div>
    </div>

    <div class="footer-bottom">
      <span class="footer-copy">{footer_copy}</span>
      <button class="footer-cookie-link cookie-manage" type="button">{footer_manage_cookie}</button>
    </div>

  </div>
</footer>

<script>
window.CHEF_I18N = {{
  lang: "{current_lang}",
  langCookie: "{lang_cookie}",
  orderAdded: "{order_added}"
}};
</script>
<script>{JS}</script>
</body>
</html>"##,
        title = title,
        body = body,
        css = css,
        JS = JS,
        html_lang = t.html_lang,
        current_lang = pack.code,
        lang_cookie = language::LANG_COOKIE,
        brand_plain = t.brand_plain,
        brand_accent = t.brand_accent,
        nav_start = t.nav_start,
        nav_menu = t.nav_menu,
        nav_delivery = t.nav_delivery,
        nav_booking = t.nav_booking,
        nav_about = t.nav_about,
        nav_table = t.nav_table,
        nav_order = t.nav_order,
        nav_language = t.nav_language,
        aria_menu = t.aria_menu,
        cookie_aria = t.cookie_aria,
        cookie_title = t.cookie_title,
        cookie_intro = t.cookie_intro,
        cookie_necessary = t.cookie_necessary,
        cookie_accept = t.cookie_accept,
        trust_delivery = t.trust_delivery,
        trust_pickup = t.trust_pickup,
        trust_booking = t.trust_booking,
        trust_author = t.trust_author,
        footer_tagline = t.footer_tagline,
        footer_guests = t.footer_guests,
        footer_restaurant = t.footer_restaurant,
        footer_contact = t.footer_contact,
        footer_menu = t.footer_menu,
        footer_delivery = t.footer_delivery,
        footer_booking = t.footer_booking,
        footer_blog = t.footer_blog,
        footer_about = t.footer_about,
        footer_haccp = t.footer_haccp,
        footer_privacy = t.footer_privacy,
        footer_terms = t.footer_terms,
        footer_cookie = t.footer_cookie,
        footer_manage_cookie = t.footer_manage_cookie,
        footer_copy = t.footer_copy,
        language_options = language_options,
        order_added = pack.js.order_added
    )
}
