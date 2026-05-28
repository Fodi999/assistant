use leptos::prelude::*;

pub fn nav_html() -> impl IntoView {
    view! {
        <nav class="navbar">
            <a href="/" class="nav-logo">"👨‍🍳 Северов"</a>
            <div class="nav-links">
                <a href="/"        class="nav-link">"Главная"</a>
                <a href="/menu"    class="nav-link">"Меню"</a>
                <a href="/recipes" class="nav-link">"Рецепты"</a>
                <a href="/about"   class="nav-link">"О шефе"</a>
            </div>
        </nav>
    }
}
