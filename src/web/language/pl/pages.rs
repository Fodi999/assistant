pub const HOME_HTML: &str = r##"
<div class="container">

  <section class="hero">
    <div class="hero-content">
      <div class="hero-badge">
        <div class="hero-badge-dot">✦</div>
        Dziś gotujemy do 22:30
      </div>
      <h1>Autorska kuchnia<br>od <span class="orange">szefa</span></h1>
      <p class="hero-sub">Świeże ryby, gorące dania, sezonowe sałatki i desery. Zamów dostawę do domu albo zarezerwuj stolik na spokojną kolację.</p>
      <div class="hero-cta">
        <a href="/menu" class="btn btn-primary btn-lg"><i class="bi bi-bag"></i> Zamów jedzenie</a>
        <a href="/booking" class="btn btn-ghost btn-lg"><i class="bi bi-calendar2-check"></i> Zarezerwuj stolik</a>
      </div>
      <p class="hero-trust">
        <span class="hero-trust-dot"></span>
        Dostawa 45-60 min &nbsp;&middot;&nbsp; Odbiór osobisty &nbsp;&middot;&nbsp; Rezerwacja online
      </p>
    </div>

    <div class="hero-visual">
      <div class="dashboard-card restaurant-card">
        <div class="dash-header">
          <span class="dash-title">Set szefa na dziś</span>
          <span class="dash-period">Hit</span>
        </div>
        <div class="chef-dish-plate">
          <div class="plate-core">
            <span class="plate-fish"></span>
            <span class="plate-leaf"></span>
            <span class="plate-sauce"></span>
          </div>
        </div>
        <div class="menu-card-header">
          <span class="menu-card-title">Grillowany labraks z masłem cytrynowym</span>
          <span class="menu-card-price">89 zł</span>
        </div>
        <p class="menu-card-desc">Śródziemnomorskie zioła, kapary, świeża cytryna i delikatna kompozycja od szefa.</p>
        <div class="hero-cta">
          <a href="/menu?cat=hot" class="btn btn-primary btn-full"><i class="bi bi-cart-plus"></i> Zobacz menu</a>
        </div>
      </div>
    </div>
  </section>

  <section class="features-section reveal">
    <div class="features-strip">
      <div class="feature-item">
        <div class="feature-icon-wrap"><i data-lucide="fish"></i></div>
        <h3>Ryby i owoce morza</h3>
        <p>Profesjonalna obróbka, świeży produkt i czysty smak bez zbędnego szumu.</p>
        <a href="/menu" class="feature-arrow">Do menu →</a>
      </div>
      <div class="feature-item">
        <div class="feature-icon-wrap"><i data-lucide="flame"></i></div>
        <h3>Gorące dania w dostawie</h3>
        <p>Pakujemy tak, aby potrawy dotarły estetycznie i zachowały temperaturę.</p>
        <a href="/delivery" class="feature-arrow">Warunki →</a>
      </div>
      <div class="feature-item">
        <div class="feature-icon-wrap"><i data-lucide="utensils"></i></div>
        <h3>Stoliki na wieczór</h3>
        <p>Kameralna sala, spokojna obsługa i stolik przygotowany przed Twoim przyjściem.</p>
        <a href="/booking" class="feature-arrow">Zarezerwuj →</a>
      </div>
      <div class="feature-item">
        <div class="feature-icon-wrap"><i data-lucide="chef-hat"></i></div>
        <h3>Kuchnia szefa</h3>
        <p>20+ lat doświadczenia w sushi, seafood, technologii produktu i pracy restauracyjnej.</p>
        <a href="/about" class="feature-arrow">O szefie →</a>
      </div>
    </div>
  </section>

  <div class="stats-section reveal">
    <div class="stats-strip">
      <div class="stat-item">
        <span class="stat-value counter" data-target="20" data-suffix="+">0</span>
        <span class="stat-label">lat doświadczenia</span>
      </div>
      <div class="stat-item">
        <span class="stat-value counter" data-target="45" data-suffix="">0</span>
        <span class="stat-label">min od kuchni</span>
      </div>
      <div class="stat-item">
        <span class="stat-value">HACCP</span>
        <span class="stat-label">standard jakości</span>
      </div>
      <div class="stat-item">
        <span class="stat-value">Daily</span>
        <span class="stat-label">świeże mise en place</span>
      </div>
    </div>
  </div>

  <div class="bottom-cards reveal">
    <div class="bottom-card">
      <div class="bottom-card-icon"><i data-lucide="shopping-bag"></i></div>
      <h3>Zamówienie w kilku kliknięciach</h3>
      <p>Wybierz dania z menu, dodaj komentarz i zamów dostawę albo odbiór osobisty.</p>
      <div class="mt-md">
        <a href="/menu" class="btn btn-ghost">Otwórz menu →</a>
      </div>
    </div>
    <div class="bottom-card">
      <div class="bottom-card-icon"><i data-lucide="calendar-check"></i></div>
      <h3>Wieczór w restauracji</h3>
      <p>Zarezerwuj stolik wcześniej: data, godzina, liczba gości i uwagi do obsługi.</p>
      <div class="mt-md">
        <a href="/booking" class="btn btn-ghost">Zarezerwuj →</a>
      </div>
    </div>
    <div class="bottom-card">
      <div class="bottom-card-icon"><i data-lucide="star"></i></div>
      <h3>Set szefa</h3>
      <p>Kompozycja dań na pierwsze spotkanie z naszą kuchnią.</p>
      <div class="price-big">219 zł</div>
      <div class="price-period">dla 2 osób</div>
      <ul class="price-features">
        <li class="price-feature"><span class="price-check">✓</span> Przystawka od szefa</li>
        <li class="price-feature"><span class="price-check">✓</span> Gorące seafood</li>
        <li class="price-feature"><span class="price-check">✓</span> Deser dnia</li>
      </ul>
      <a href="/menu" class="btn btn-primary btn-full">Wybierz dania</a>
    </div>
  </div>

</div>
"##;

pub const DELIVERY_HTML: &str = r#"
<div class="container">
  <section class="page-header">
    <h1>Dostawa i odbiór osobisty</h1>
    <p class="page-header-sub">Gotujemy po potwierdzeniu zamówienia i pakujemy dania tak, aby dotarły w odpowiednim stanie.</p>
  </section>

  <section class="content-grid-3 reveal">
    <div class="bottom-card">
      <div class="bottom-card-icon"><i class="bi bi-scooter"></i></div>
      <h3>Dostawa 45-60 min</h3>
      <p>Czas zależy od adresu i obłożenia kuchni. Po zamówieniu potwierdzimy szczegóły.</p>
    </div>
    <div class="bottom-card">
      <div class="bottom-card-icon"><i class="bi bi-bag-check"></i></div>
      <h3>Odbiór osobisty</h3>
      <p>Złóż zamówienie wcześniej i odbierz dania bez czekania o wybranej godzinie.</p>
    </div>
    <div class="bottom-card">
      <div class="bottom-card-icon"><i class="bi bi-credit-card"></i></div>
      <h3>Płatność</h3>
      <p>Gotówką, kartą przy odbiorze albo przelewem po potwierdzeniu zamówienia.</p>
    </div>
  </section>

  <div class="section-sep"></div>

  <section class="booking-panel reveal">
    <div>
      <span class="section-eyebrow"><i class="bi bi-geo-alt"></i> Strefa dostawy</span>
      <h2 class="section-title">Sprawdzimy adres przed gotowaniem</h2>
      <p class="section-desc">Na tym etapie zamówienie można przygotować z poziomu menu. Kolejny krok to koszyk, adres dostawy i powiadomienia dla obsługi.</p>
    </div>
    <div class="delivery-list">
      <div><strong>Centrum</strong><span>od 45 min</span></div>
      <div><strong>Bliskie dzielnice</strong><span>45-60 min</span></div>
      <div><strong>Poza miastem</strong><span>do ustalenia</span></div>
    </div>
    <a href="/menu" class="btn btn-primary btn-lg"><i class="bi bi-bag"></i> Przejdź do menu</a>
  </section>
</div>
"#;

pub const BOOKING_HTML: &str = r#"
<div class="container">
  <section class="page-header">
    <h1>Rezerwacja stolika</h1>
    <p class="page-header-sub">Wybierz datę, godzinę i liczbę gości. Potwierdzimy rezerwację po sprawdzeniu dostępności.</p>
  </section>

  <section class="booking-layout reveal">
    <form class="booking-form" action="/booking" method="get">
      <label>
        <span>Imię</span>
        <input type="text" name="name" placeholder="Twoje imię" autocomplete="name" required>
      </label>
      <label>
        <span>Telefon</span>
        <input type="tel" name="phone" placeholder="+48 000 000 000" autocomplete="tel" required>
      </label>
      <label>
        <span>Data</span>
        <input type="date" name="date" required>
      </label>
      <label>
        <span>Godzina</span>
        <input type="time" name="time" required>
      </label>
      <label>
        <span>Goście</span>
        <input type="number" name="guests" min="1" max="12" value="2" required>
      </label>
      <label>
        <span>Komentarz</span>
        <textarea name="comment" rows="4" placeholder="Krzesełko dziecięce, alergie, okazja kolacji"></textarea>
      </label>
      <button class="btn btn-primary btn-lg btn-full" type="submit"><i class="bi bi-calendar2-check"></i> Wyślij zapytanie</button>
      <p class="form-note">Na tym etapie formularz działa jako szkic zapytania. Następnie podłączymy zapisywanie rezerwacji i powiadomienia.</p>
    </form>

    <aside class="booking-aside">
      <div class="about-side-card">
        <p class="side-section-title">Godziny otwarcia</p>
        <ul class="side-list">
          <li>Pn-Czw: 12:00-22:00</li>
          <li>Pt-Sb: 12:00-23:00</li>
          <li>Nd: 12:00-21:00</li>
        </ul>

        <p class="side-section-title">Dla gości</p>
        <ul class="side-list">
          <li>Rezerwację trzymamy 15 minut</li>
          <li>Do 12 gości online</li>
          <li>Wydarzenia specjalne na zapytanie</li>
        </ul>

        <p class="side-section-title">Kontakt</p>
        <ul class="side-list">
          <li><i class="bi bi-telephone" style="color:var(--accent)"></i> &nbsp;+48 000 000 000</li>
          <li><i class="bi bi-geo-alt" style="color:var(--accent)"></i> &nbsp;Polska / Ukraina</li>
        </ul>
      </div>
    </aside>
  </section>
</div>
"#;

pub const ABOUT_HTML: &str = r#"
<div class="container">

  <section class="about-hero">
    <span class="about-eyebrow"><i class="bi bi-person-badge"></i> Profil szefa</span>
    <h1 class="chef-name">Dima Fomin</h1>
    <p class="chef-title">Sushi chef &bull; Technolog żywności</p>
    <p class="chef-intro">
      Jestem sushi chefem i technologiem żywności. Tworzę kuchnię opartą na produkcie,
      sezonowości, precyzji techniki i spokojnym, profesjonalnym serwisie.
    </p>
  </section>

  <div class="stats-section reveal">
    <div class="stats-strip">
      <div class="stat-item">
        <span class="stat-value counter" data-target="20" data-suffix="+">0</span>
        <span class="stat-label">lat doświadczenia</span>
      </div>
      <div class="stat-item">
        <span class="stat-value counter" data-target="6" data-suffix="">0</span>
        <span class="stat-label">krajów pracy</span>
      </div>
      <div class="stat-item">
        <span class="stat-value">Sushi</span>
        <span class="stat-label">&amp; Seafood</span>
      </div>
      <div class="stat-item">
        <span class="stat-value">HACCP</span>
        <span class="stat-label">standard jakości</span>
      </div>
    </div>
  </div>

  <div class="about-body">
    <div class="about-main">
      <div class="about-mission reveal">
        <span class="section-eyebrow"><i class="bi bi-compass"></i> Moja misja</span>
        <div class="mission-quote">
          <p>Silna kuchnia zaczyna się od zrozumienia produktu. Dlatego łączę technikę,
          proces i smak: od jakości ryby, przez mise en place, aż po danie, które trafia
          do gościa w restauracji albo w dostawie.</p>
        </div>
        <ul class="mission-values">
          <li><i class="bi bi-gem"></i> <strong>Jakość</strong> — praca z produktem</li>
          <li><i class="bi bi-gear"></i> <strong>Proces</strong> — powtarzalność kuchni</li>
          <li><i class="bi bi-bullseye"></i> <strong>Technika</strong> — japońska precyzja</li>
        </ul>
      </div>

      <div class="reveal" style="margin-bottom:3.5rem">
        <span class="section-eyebrow"><i class="bi bi-stars"></i> Specjalizacja</span>
        <h2 class="section-title">Najważniejsze obszary pracy</h2>
        <div class="expertise-grid">
          <div class="expertise-item">
            <i class="expertise-icon bi bi-reception-4"></i>
            <h4>Sushi</h4>
            <p>Tradycyjne techniki, właściwa praca z ryżem, rybą i japońskimi nożami.</p>
          </div>
          <div class="expertise-item">
            <i class="expertise-icon bi bi-water"></i>
            <h4>Ryby i owoce morza</h4>
            <p>Dobór, obróbka i kontrola jakości produktów seafood.</p>
          </div>
          <div class="expertise-item">
            <i class="expertise-icon bi bi-lightbulb"></i>
            <h4>Rozwój dań</h4>
            <p>Autorskie receptury, testy, wdrożenia i dopracowanie smaku.</p>
          </div>
          <div class="expertise-item">
            <i class="expertise-icon bi bi-gear-wide-connected"></i>
            <h4>Procesy kuchenne</h4>
            <p>Organizacja pracy, sprzęt, bezpieczeństwo i stabilna jakość.</p>
          </div>
          <div class="expertise-item">
            <i class="expertise-icon bi bi-people"></i>
            <h4>Szkolenie zespołu</h4>
            <p>Mentoring kucharzy, standardy pracy i praktyczne wdrożenia.</p>
          </div>
          <div class="expertise-item">
            <i class="expertise-icon bi bi-shield-check"></i>
            <h4>HACCP</h4>
            <p>Bezpieczeństwo żywności i kontrola jakości w codziennej pracy.</p>
          </div>
        </div>
      </div>

      <div class="reveal">
        <span class="section-eyebrow"><i class="bi bi-briefcase"></i> Kariera</span>
        <h2 class="section-title">Doświadczenie</h2>
        <div class="timeline" style="margin-top:2rem">
          <div class="timeline-featured">
            <p class="timeline-year"><i class="bi bi-star-fill"></i> 2002 — 2026 (obecnie)</p>
            <p class="timeline-role">FISH in HOUSE</p>
            <p class="timeline-place">Head Chef / Food Technologist</p>
            <p class="timeline-desc">Kluczowa rola przez ponad 20 lat.</p>
            <ul class="timeline-list">
              <li>Rozwój nowych produktów</li>
              <li>Kontrola jakości i trwałości</li>
              <li>Organizacja procesów produkcyjnych</li>
              <li>Dobór sprzętu</li>
              <li>Szkolenie personelu</li>
              <li>Praca z wolumenem sprzedaży</li>
            </ul>
          </div>

          <div class="timeline-item">
            <p class="timeline-year">2017 — 2018</p>
            <p class="timeline-role">Restauracja Autorska &ldquo;Mi&oacute;d Malina&rdquo;</p>
            <p class="timeline-place">Cook</p>
          </div>

          <div class="timeline-item">
            <p class="timeline-year">2017 — 2018</p>
            <p class="timeline-role">Restaurant Charlemagne</p>
            <p class="timeline-place">Cook, Seafood</p>
          </div>

          <div class="timeline-item">
            <p class="timeline-year">2022 — 2023</p>
            <p class="timeline-role">Boulangerie P&acirc;tisserie WAWEL</p>
            <p class="timeline-place">Cook</p>
          </div>
        </div>
      </div>

      <section class="ingredient-catalog-cta reveal">
        <div>
          <span class="section-eyebrow"><i class="bi bi-journal-richtext"></i> Katalog składników</span>
          <h2 class="section-title">Produkty z wartością odżywczą i zdjęciami</h2>
          <p class="section-desc">Baza składników używanych w kuchni: zdjęcia, makro, energia, sezonowość i krótkie notatki technologiczne do pracy z produktem.</p>
        </div>
        <a href="/ingredient-catalog" class="btn btn-primary btn-lg"><i class="bi bi-arrow-right-circle"></i> Otwórz katalog</a>
      </section>
    </div>

    <aside class="about-side reveal">
      <div class="about-side-card">
        <p class="side-section-title">Edukacja</p>
        <ul class="side-list">
          <li>Szkoła zawodowa nr 53, Dnipro</li>
          <li>Dyplomowany kucharz</li>
          <li>2002-2003 &bull; dyplom z wyróżnieniem</li>
          <li>Staż w restauracji Charlie&rsquo;s</li>
        </ul>

        <p class="side-section-title">Specjalizacja</p>
        <ul class="side-list">
          <li>Sushi i kuchnia japońska</li>
          <li>Ryby i owoce morza</li>
          <li>Technologia żywności</li>
          <li>Kontrola jakości (HACCP)</li>
          <li>Rozwój produktów</li>
          <li>Szkolenie personelu</li>
        </ul>

        <p class="side-section-title">Kontakt</p>
        <ul class="side-list">
          <li><i class="bi bi-envelope" style="color:var(--accent)"></i> &nbsp;Na zapytanie</li>
          <li><i class="bi bi-geo-alt" style="color:var(--accent)"></i> &nbsp;Polska / Ukraina</li>
          <li><i class="bi bi-building" style="color:var(--accent)"></i> &nbsp;FISH in HOUSE</li>
        </ul>
      </div>
    </aside>
  </div>
</div>
"#;

pub const PRIVACY_HTML: &str = r#"
<div class="container legal-page">
  <section class="page-header">
    <span class="section-eyebrow"><i class="bi bi-shield-lock"></i> Prywatność</span>
    <h1>Polityka prywatności</h1>
    <p class="page-header-sub">Ostatnia aktualizacja: 13 marca 2026</p>
  </section>
  <section class="legal-content solo">
    <p>Szanujemy Twoją prywatność. Dane przekazane przez formularze kontaktowe, rezerwacyjne lub zamówieniowe wykorzystujemy wyłącznie do obsługi zapytania, rezerwacji albo zamówienia.</p>
    <p>Nie sprzedajemy danych osobowych i nie używamy reklamowych pikseli śledzących. Szczegóły dotyczące plików cookie znajdziesz na stronie <a href="/cookie">Polityka cookie</a>.</p>
  </section>
</div>
"#;

pub const TERMS_HTML: &str = r#"
<div class="container legal-page">
  <section class="page-header">
    <span class="section-eyebrow"><i class="bi bi-file-earmark-text"></i> Regulamin</span>
    <h1>Regulamin</h1>
    <p class="page-header-sub">Ostatnia aktualizacja: 13 marca 2026</p>
  </section>
  <section class="legal-content solo">
    <p>Strona dima-fomin.pl prezentuje menu, informacje o dostawie, rezerwacji stolików oraz działalności szefa kuchni. Ceny i dostępność dań mogą zmieniać się w zależności od sezonu i dostępności produktu.</p>
    <p>Rezerwacje i zamówienia wymagają potwierdzenia przez obsługę. W sprawach prywatności i plików cookie skorzystaj ze stron <a href="/privacy">Polityka prywatności</a> oraz <a href="/cookie">Polityka cookie</a>.</p>
  </section>
</div>
"#;

pub const COOKIE_HTML: &str = r##"
<div class="container legal-page">
  <section class="page-header">
    <span class="section-eyebrow"><i class="bi bi-shield-check"></i> Polityka cookie</span>
    <h1>Polityka cookie</h1>
    <p class="page-header-sub">Ostatnia aktualizacja: 13 marca 2026</p>
  </section>

  <section class="legal-layout">
    <aside class="legal-index">
      <a href="#cookies-01"><span>01</span> Czym są pliki cookie</a>
      <a href="#cookies-02"><span>02</span> Rodzaje używanych plików cookie</a>
      <a href="#cookies-03"><span>03</span> Niezbędne pliki cookie</a>
      <a href="#cookies-04"><span>04</span> Analityczne pliki cookie</a>
      <a href="#cookies-05"><span>05</span> Zarządzanie plikami cookie</a>
      <a href="#cookies-06"><span>06</span> Pliki cookie stron trzecich</a>
      <a href="#cookies-07"><span>07</span> Aktualizacje polityki</a>
    </aside>

    <article class="legal-content">
      <div class="legal-related">
        <span>Powiązane strony:</span>
        <a href="/privacy">Polityka prywatności</a>
        <a href="/terms">Regulamin</a>
        <a href="/cookie">Polityka cookie</a>
      </div>

      <p>Niniejsza Polityka cookie wyjaśnia, czym są pliki cookie, jak ich używamy na dima-fomin.pl i jak możesz zarządzać swoimi preferencjami.</p>

      <section id="cookies-01" class="legal-section">
        <span class="legal-num">01</span>
        <h2>Czym są pliki cookie</h2>
        <p>Pliki cookie to małe pliki tekstowe przechowywane na Twoim urządzeniu podczas odwiedzania strony. Pomagają stronie zapamiętać Twoje preferencje i poprawiają komfort przeglądania.</p>
      </section>

      <section id="cookies-02" class="legal-section">
        <span class="legal-num">02</span>
        <h2>Rodzaje używanych plików cookie</h2>
        <p>Używamy następujących rodzajów plików cookie:</p>
        <ul>
          <li>Niezbędne pliki cookie — wymagane do podstawowego działania strony</li>
          <li>Analityczne pliki cookie — pomagają zrozumieć, jak odwiedzający korzystają ze strony</li>
          <li>Pliki cookie preferencji — zapamiętują Twoje ustawienia i wybory</li>
        </ul>
      </section>

      <section id="cookies-03" class="legal-section">
        <span class="legal-num">03</span>
        <h2>Niezbędne pliki cookie</h2>
        <p>Te pliki cookie są konieczne do prawidłowego działania strony. Obejmują:</p>
        <ul>
          <li>Preferencja zgody na pliki cookie (cookie-consent)</li>
          <li>Preferencja motywu (jasny/ciemny)</li>
          <li>Preferencja języka</li>
        </ul>
      </section>

      <section id="cookies-04" class="legal-section">
        <span class="legal-num">04</span>
        <h2>Analityczne pliki cookie</h2>
        <p>Korzystamy z Vercel Analytics do analizy ruchu na stronie. Te pliki cookie zbierają anonimowe dane, takie jak odsłony i czas sesji. Analityczne pliki cookie są ładowane tylko po zaakceptowaniu ich przez banner zgody.</p>
      </section>

      <section id="cookies-05" class="legal-section">
        <span class="legal-num">05</span>
        <h2>Zarządzanie plikami cookie</h2>
        <p>Możesz zarządzać swoimi preferencjami dotyczącymi plików cookie na kilka sposobów:</p>
        <ul>
          <li>Skorzystaj z bannera zgody przy pierwszej wizycie na stronie</li>
          <li>Wyczyść pliki cookie w ustawieniach przeglądarki</li>
          <li>Wyłącz pliki cookie w preferencjach przeglądarki</li>
          <li>Użyj rozszerzeń przeglądarki blokujących pliki cookie</li>
        </ul>
        <button class="btn btn-ghost cookie-manage" type="button">Zarządzaj cookie</button>
      </section>

      <section id="cookies-06" class="legal-section">
        <span class="legal-num">06</span>
        <h2>Pliki cookie stron trzecich</h2>
        <p>Niektóre usługi stron trzecich mogą ustawiać własne pliki cookie. Korzystamy z minimalnej liczby usług stron trzecich i nie używamy reklamowych plików cookie ani pikseli śledzących.</p>
      </section>

      <section id="cookies-07" class="legal-section">
        <span class="legal-num">07</span>
        <h2>Aktualizacje polityki</h2>
        <p>Możemy aktualizować niniejszą Politykę cookie. Wszelkie zmiany będą publikowane na tej stronie z zaktualizowaną datą.</p>
      </section>
    </article>
  </section>
</div>
"##;

pub const NOT_FOUND_HTML: &str = r#"<div class="page-header"><h1>404</h1><p>Nie znaleziono strony</p><a href="/" class="btn btn-ghost">Wróć na start</a></div>"#;

pub struct MenuCategory {
    pub id: &'static str,
    pub icon: &'static str,
    pub label: &'static str,
}

pub struct MenuItem {
    pub icon: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub price: &'static str,
    pub weight: &'static str,
    pub category: &'static str,
    pub badge: &'static str,
}

pub const MENU_TITLE: &str = "Nasze menu";
pub const MENU_SUBTITLE: &str =
    "Autorska kuchnia z sezonowych produktów, dostępna w dostawie i odbiorze osobistym";
pub const MENU_ALL: &str = "Całe menu";
pub const MENU_ORDER: &str = "Do zamówienia";
pub const MENU_CART_TITLE: &str = "Koszyk";
pub const MENU_CART_EMPTY: &str =
    "Koszyk jest pusty. Dodaj dania z menu, a pokażą się tutaj od razu.";
pub const MENU_CART_SUBTOTAL: &str = "Suma";
pub const MENU_CART_TOTAL: &str = "Razem";
pub const MENU_CART_CLEAR: &str = "Wyczyść";
pub const MENU_CART_CHECKOUT: &str = "Przejdź do finalizacji";
pub const MENU_CART_HINT: &str = "Koszyk zapisuje się lokalnie w przeglądarce.";

pub const MENU_CATEGORIES: &[MenuCategory] = &[
    MenuCategory {
        id: "hot",
        icon: "bi-fire",
        label: "Dania gorące",
    },
    MenuCategory {
        id: "sushi",
        icon: "bi-water",
        label: "Sushi",
    },
    MenuCategory {
        id: "salads",
        icon: "bi-leaf",
        label: "Sałatki",
    },
    MenuCategory {
        id: "soups",
        icon: "bi-cup-hot",
        label: "Zupy",
    },
    MenuCategory {
        id: "desserts",
        icon: "bi-cake2",
        label: "Desery",
    },
    MenuCategory {
        id: "drinks",
        icon: "bi-cup-straw",
        label: "Napoje",
    },
];

pub const MENU_ITEMS: &[MenuItem] = &[
    MenuItem {
        icon: "bi-fire",
        name: "Stek ribeye",
        desc: "Marmurkowana wołowina, sos z czerwonego wina, ziemniaki gratin",
        price: "139 zł",
        weight: "320 g",
        category: "hot",
        badge: "hit",
    },
    MenuItem {
        icon: "bi-fire",
        name: "Pierś z kaczki",
        desc: "Sos pomarańczowy, purée z pasternaku, wiśniowy jus",
        price: "109 zł",
        weight: "280 g",
        category: "hot",
        badge: "szef",
    },
    MenuItem {
        icon: "bi-fire",
        name: "Labraks z grilla",
        desc: "Śródziemnomorskie zioła, kapary, masło cytrynowe",
        price: "89 zł",
        weight: "300 g",
        category: "hot",
        badge: "seafood",
    },
    MenuItem {
        icon: "bi-fire",
        name: "Risotto z grzybami",
        desc: "Carnaroli, mascarpone, borowiki, krem truflowy",
        price: "69 zł",
        weight: "260 g",
        category: "hot",
        badge: "veg",
    },
    MenuItem {
        icon: "bi-water",
        name: "Nigiri z łososiem",
        desc: "Ciepły ryż, świeży łosoś, wasabi, sos sojowy",
        price: "39 zł",
        weight: "2 szt.",
        category: "sushi",
        badge: "new",
    },
    MenuItem {
        icon: "bi-water",
        name: "Rolka tuńczyk-awokado",
        desc: "Tuńczyk, awokado, sezam, nori, sos ponzu",
        price: "59 zł",
        weight: "240 g",
        category: "sushi",
        badge: "hit",
    },
    MenuItem {
        icon: "bi-leaf",
        name: "Cezar z kurczakiem",
        desc: "Romaine, parmezan, anchois, grzanki, klasyczny sos",
        price: "49 zł",
        weight: "230 g",
        category: "salads",
        badge: "",
    },
    MenuItem {
        icon: "bi-leaf",
        name: "Tatar z tuńczyka",
        desc: "Awokado, dressing sezamowy, chipsy ryżowe",
        price: "69 zł",
        weight: "190 g",
        category: "salads",
        badge: "seafood",
    },
    MenuItem {
        icon: "bi-cup-hot",
        name: "Krem z dyni",
        desc: "Imbir, mleczko kokosowe, prażone pestki dyni",
        price: "39 zł",
        weight: "300 ml",
        category: "soups",
        badge: "veg",
    },
    MenuItem {
        icon: "bi-cup-hot",
        name: "Francuska zupa cebulowa",
        desc: "Gruyère, brioszka, intensywny bulion wołowy gotowany 8 godzin",
        price: "43 zł",
        weight: "320 ml",
        category: "soups",
        badge: "",
    },
    MenuItem {
        icon: "bi-cake2",
        name: "Crème brûlée",
        desc: "Wanilia Bourbon, chrupiąca karmelowa skorupka",
        price: "32 zł",
        weight: "140 g",
        category: "desserts",
        badge: "",
    },
    MenuItem {
        icon: "bi-cake2",
        name: "Fondant czekoladowy",
        desc: "70% kakao, gorące ciemne wnętrze, lody waniliowe",
        price: "36 zł",
        weight: "160 g",
        category: "desserts",
        badge: "hit",
    },
    MenuItem {
        icon: "bi-cup-straw",
        name: "Domowa lemoniada",
        desc: "Bazylia, ogórek, świeżo wyciskana cytryna, cukier trzcinowy",
        price: "21 zł",
        weight: "350 ml",
        category: "drinks",
        badge: "",
    },
];

pub struct Recipe {
    pub id: &'static str,
    pub name: &'static str,
    pub icon: &'static str,
    pub time: &'static str,
    pub difficulty: &'static str,
    pub ingredients: &'static [(&'static str, &'static str)],
    pub steps: &'static [&'static str],
}

pub const RECIPES_TITLE: &str = "Blog szefa";
pub const RECIPES_SUBTITLE: &str = "Sprawdzone techniki, receptury i praktyczne notatki z kuchni";
pub const RECIPE_NOT_FOUND: &str = "Nie znaleziono przepisu";
pub const RECIPE_BACK: &str = "Wróć";
pub const RECIPE_ALL: &str = "Wszystkie wpisy";
pub const RECIPE_INGREDIENTS: &str = "Składniki";
pub const RECIPE_PREPARATION: &str = "Przygotowanie";

pub const RECIPES: &[Recipe] = &[
    Recipe {
        id: "borsch",
        name: "Barszcz kijowski",
        icon: "bi-cup-hot",
        time: "90 min",
        difficulty: "Średni",
        ingredients: &[
            ("Mostek wołowy", "500 g"),
            ("Buraki", "2 szt."),
            ("Biała kapusta", "300 g"),
            ("Ziemniaki", "3 szt."),
            ("Marchew", "1 szt."),
            ("Koncentrat pomidorowy", "2 łyżki"),
        ],
        steps: &[
            "Ugotuj bulion z mostku na małym ogniu przez około 1 godzinę.",
            "Zetrzyj buraki i duś je z odrobiną octu przez 10 minut.",
            "Dodaj do bulionu pokrojone ziemniaki i kapustę.",
            "Dodaj zasmażkę z cebuli, marchewki i koncentratu oraz buraki; gotuj 15 minut.",
            "Odstaw na 20 minut, podawaj ze śmietaną i ziołami.",
        ],
    },
    Recipe {
        id: "tiramisu",
        name: "Klasyczne tiramisu",
        icon: "bi-cake2",
        time: "30 min + 4 h chłodzenia",
        difficulty: "Łatwy",
        ingredients: &[
            ("Mascarpone", "500 g"),
            ("Jajka", "4 szt."),
            ("Cukier", "100 g"),
            ("Biszkopty savoiardi", "300 g"),
            ("Mocne espresso", "200 ml"),
            ("Kakao do posypania", "2 łyżki"),
        ],
        steps: &[
            "Ubij żółtka z cukrem na jasną, puszystą masę.",
            "Dodaj mascarpone i delikatnie wymieszaj szpatułką.",
            "Ubij białka na sztywno i ostrożnie połącz z kremem.",
            "Zanurzaj biszkopty w wystudzonym espresso przez około 2 sekundy.",
            "Układaj warstwy: biszkopty, krem, biszkopty, krem.",
            "Schłódź minimum 4 godziny, przed podaniem posyp kakao.",
        ],
    },
    Recipe {
        id: "risotto",
        name: "Risotto z borowikami",
        icon: "bi-fire",
        time: "35 min",
        difficulty: "Średni",
        ingredients: &[
            ("Ryż Carnaroli", "300 g"),
            ("Borowiki", "200 g"),
            ("Mascarpone", "100 g"),
            ("Tarty parmezan", "80 g"),
            ("Białe wytrawne wino", "150 ml"),
            ("Gorący bulion drobiowy", "1 l"),
        ],
        steps: &[
            "Podsmaż grzyby na maśle do złotego koloru i odłóż.",
            "Zeszklij szalotkę, dodaj ryż i mieszaj przez 2 minuty.",
            "Wlej wino i odparuj niemal całkowicie.",
            "Dodawaj bulion po chochelce, stale mieszając.",
            "Na końcu dodaj grzyby, mascarpone i parmezan.",
            "Zdejmij z ognia, przykryj na 2 minuty i podawaj od razu.",
        ],
    },
];
