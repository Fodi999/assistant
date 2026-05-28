pub mod pages;
pub mod handlers;

const CSS: &str = include_str!("../../static/chef/style.css");
const JS:  &str = include_str!("../../static/chef/app.js");

pub fn shell(title: &str, body: &str) -> String {
    format!(r#"<!DOCTYPE html>
<html lang="ru">
<head>
  <meta charset="UTF-8"/>
  <meta name="viewport" content="width=device-width,initial-scale=1.0"/>
  <title>{title} &middot; Северов</title>
  <link rel="preconnect" href="https://fonts.googleapis.com"/>
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin/>
  <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/bootstrap-icons@1.11.3/font/bootstrap-icons.min.css"/>
  <style>{CSS}</style>
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
</html>"#, title=title, body=body, CSS=CSS, JS=JS)
}
