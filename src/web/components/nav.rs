use leptos::prelude::*;

use crate::web::language::active::SHELL;

pub fn nav_html() -> impl IntoView {
    let t = &SHELL;
    view! {
        <nav class="navbar">
            <a href="/" class="nav-logo"><i data-lucide="chef-hat"></i>{t.brand_plain}{t.brand_accent}</a>
            <div class="nav-links">
                <a href="/"        class="nav-link">{t.nav_start}</a>
                <a href="/menu"    class="nav-link">{t.nav_menu}</a>
                <a href="/recipes" class="nav-link">{t.footer_blog}</a>
                <a href="/about"   class="nav-link">{t.nav_about}</a>
            </div>
        </nav>
    }
}
