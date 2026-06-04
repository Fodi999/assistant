use leptos::prelude::*;

use crate::web::language::active::SHELL;

pub fn footer_html() -> impl IntoView {
    let t = &SHELL;
    view! {
        <footer class="footer">
            <span>{t.footer_copy}</span>
            <span>"·"</span>
            <span>{t.trust_author}</span>
            <span>"·"</span>
            <a href="mailto:chef@example.com">"chef@example.com"</a>
        </footer>
    }
}
