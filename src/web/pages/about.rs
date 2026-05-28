pub fn render() -> String {
    r#"
<section class="about-hero reveal visible">
  <div class="chef-avatar-wrap"><i class="bi bi-person-fill" style="font-size:3.5rem;color:var(--accent)"></i></div>
  <h1 style="font-family:'Playfair Display',serif;font-size:2rem;color:var(--accent)">Алексей Северов</h1>
  <p class="chef-title"><i class="bi bi-award"></i> Шеф-повар &bull; 15 лет практики</p>
</section>

<div class="about-body reveal">
  Кулинарное путешествие началось во Флоренции, прошло через парижские бистро
  и вернулось домой — в Россию. Моя кухня — это честный продукт,
  минимум лишнего и максимум вкуса.
</div>

<div class="stats reveal">
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
</div>

<hr class="divider">

<div class="timeline reveal" style="margin-bottom:2rem">
  <div class="timeline-item">
    <div class="timeline-icon"><i class="bi bi-geo-alt"></i></div>
    <div>
      <h4>Флоренция, Италия &mdash; 2008&ndash;2012</h4>
      <p>Кулинарная академия Cordon Bleu, стажировка в ресторане 1&nbsp;*&nbsp;Michelin</p>
    </div>
  </div>
  <div class="timeline-item">
    <div class="timeline-icon"><i class="bi bi-geo-alt"></i></div>
    <div>
      <h4>Париж, Франция &mdash; 2012&ndash;2017</h4>
      <p>Су-шеф в bistrot gastronomique, разработка сезонных меню</p>
    </div>
  </div>
  <div class="timeline-item">
    <div class="timeline-icon"><i class="bi bi-geo-alt"></i></div>
    <div>
      <h4>Москва, Россия &mdash; 2017&ndash;сейчас</h4>
      <p>Авторский ресторан, гастрономические ужины, мастер-классы</p>
    </div>
  </div>
</div>
"#.to_string()
}
