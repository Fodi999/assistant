use leptos::prelude::*;

pub fn footer_html() -> impl IntoView {
    view! {
        <footer class="footer">
            <span>"© 2026 Алексей Северов"</span>
            <span>"·"</span>
            <span>"Авторская кухня"</span>
            <span>"·"</span>
            <a href="mailto:chef@severov.ru">"chef@severov.ru"</a>
        </footer>
    }
}
