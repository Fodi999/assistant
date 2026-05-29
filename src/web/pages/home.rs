pub fn render() -> String {
    r#"
<section class="hero">
  <div class="hero-eyebrow"><i class="bi bi-stars"></i> Авторская кухня</div>
  <h1>Без компромиссов,<br><em>с душой</em></h1>
  <p class="hero-sub">Сезонные продукты, французские техники и душа русской кухни — в каждом блюде</p>
  <div class="hero-cta">
    <a href="/menu"  class="btn"><i class="bi bi-journal-richtext"></i> Смотреть меню</a>
    <a href="/about" class="btn btn-ghost"><i class="bi bi-person-circle"></i> О шефе</a>
  </div>
</section>

<section class="features-section reveal">
  <div class="section-title">
    <span class="section-eyebrow">Почему мы</span>
    <h2>Принципы кухни</h2>
    <div class="accent-line"></div>
  </div>
  <div class="features">
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
  </div>
</section>

<div class="stats-section reveal">
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
</div>
"#.to_string()
}
