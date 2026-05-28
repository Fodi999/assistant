pub fn render() -> String {
    r#"
<section class="hero reveal visible">
  <h1>Авторская кухня<br>без компромиссов</h1>
  <p>Сезонные продукты, французские техники и душа русской кухни — в каждом блюде</p>
  <div style="display:flex;gap:.75rem;justify-content:center;flex-wrap:wrap;margin-top:2rem">
    <a href="/menu"    class="btn"><i class="bi bi-journal-richtext"></i> Смотреть меню</a>
    <a href="/about"   class="btn btn-ghost"><i class="bi bi-person-circle"></i> О шефе</a>
  </div>
</section>

<section class="features" style="margin-top:3.5rem">
  <div class="feature reveal">
    <div class="feature-icon"><i class="bi bi-flower2"></i></div>
    <h3>Сезонные продукты</h3>
    <p>Только свежее и локальное — меню меняется вместе с природой</p>
  </div>
  <div class="feature reveal">
    <div class="feature-icon"><i class="bi bi-book-half"></i></div>
    <h3>Авторские рецепты</h3>
    <p>Более 200 блюд собственной разработки, проверенных временем</p>
  </div>
  <div class="feature reveal">
    <div class="feature-icon"><i class="bi bi-globe2"></i></div>
    <h3>15 лет опыта</h3>
    <p>Флоренция, Париж, Москва — три школы, один почерк</p>
  </div>
  <div class="feature reveal">
    <div class="feature-icon"><i class="bi bi-award"></i></div>
    <h3>Честный вкус</h3>
    <p>Минимум лишнего, максимум вкуса — никакой химии и полуфабрикатов</p>
  </div>
</section>

<hr class="divider">

<section style="text-align:center;padding:2rem 0 1rem">
  <h2 style="font-family:'Playfair Display',serif;color:var(--accent);font-size:1.6rem;margin-bottom:2rem">
    <i class="bi bi-graph-up-arrow"></i>&nbsp; В цифрах
  </h2>
  <div class="stats">
    <div class="stat">
      <span class="stat-num counter" data-target="15" data-suffix="">0</span>
      <span class="stat-label">лет опыта</span>
    </div>
    <div class="stat">
      <span class="stat-num counter" data-target="3" data-suffix="">0</span>
      <span class="stat-label">страны работы</span>
    </div>
    <div class="stat">
      <span class="stat-num counter" data-target="200" data-suffix="+">0</span>
      <span class="stat-label">авторских блюд</span>
    </div>
    <div class="stat">
      <span class="stat-num counter" data-target="8" data-suffix="">0</span>
      <span class="stat-label">лет мишленовских ресторанов</span>
    </div>
  </div>
</section>
"#.to_string()
}
