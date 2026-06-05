use crate::web::language::pl::pages::{MenuCategory, MenuItem, Recipe};

pub const HOME_HTML: &str = r##"
<div class="container">
  <section class="hero">
    <div class="hero-content">
      <div class="hero-badge"><div class="hero-badge-dot">✦</div>Cooking today until 22:30</div>
      <h1>Signature cuisine<br>by the <span class="orange">chef</span></h1>
      <p class="hero-sub">Fresh fish, hot dishes, seasonal salads and desserts. Order delivery at home or book a table for a calm dinner.</p>
      <div class="hero-cta">
        <a href="/menu" class="btn btn-primary btn-lg"><i class="bi bi-bag"></i> Order food</a>
        <a href="/booking" class="btn btn-ghost btn-lg"><i class="bi bi-calendar2-check"></i> Book a table</a>
      </div>
      <p class="hero-trust"><span class="hero-trust-dot"></span>Delivery 45-60 min &nbsp;&middot;&nbsp; Pickup &nbsp;&middot;&nbsp; Online booking</p>
    </div>
    <div class="hero-visual">
      <div class="dashboard-card restaurant-card">
        <div class="dash-header"><span class="dash-title">Chef set today</span><span class="dash-period">Hit</span></div>
        <div class="chef-dish-plate"><div class="plate-core"><span class="plate-fish"></span><span class="plate-leaf"></span><span class="plate-sauce"></span></div></div>
        <div class="menu-card-header"><span class="menu-card-title">Grilled sea bass with lemon butter</span><span class="menu-card-price">89 zł</span></div>
        <p class="menu-card-desc">Mediterranean herbs, capers, fresh lemon and a delicate chef composition.</p>
        <div class="hero-cta"><a href="/menu?cat=hot" class="btn btn-primary btn-full"><i class="bi bi-cart-plus"></i> View menu</a></div>
      </div>
    </div>
  </section>
  <section class="features-section reveal">
    <div class="features-strip">
      <div class="feature-item"><div class="feature-icon-wrap"><i data-lucide="fish"></i></div><h3>Fish and seafood</h3><p>Professional handling, fresh product and a clean taste without noise.</p><a href="/menu" class="feature-arrow">Menu →</a></div>
      <div class="feature-item"><div class="feature-icon-wrap"><i data-lucide="flame"></i></div><h3>Hot dishes delivered</h3><p>We pack dishes so they arrive neat and warm.</p><a href="/delivery" class="feature-arrow">Terms →</a></div>
      <div class="feature-item"><div class="feature-icon-wrap"><i data-lucide="utensils"></i></div><h3>Evening tables</h3><p>A calm room, attentive service and a table prepared before you arrive.</p><a href="/booking" class="feature-arrow">Book →</a></div>
      <div class="feature-item"><div class="feature-icon-wrap"><i data-lucide="chef-hat"></i></div><h3>Chef cuisine</h3><p>20+ years in sushi, seafood, product technology and restaurant work.</p><a href="/about" class="feature-arrow">About →</a></div>
    </div>
  </section>
  <div class="stats-section reveal"><div class="stats-strip">
    <div class="stat-item"><span class="stat-value counter" data-target="20" data-suffix="+">0</span><span class="stat-label">years of experience</span></div>
    <div class="stat-item"><span class="stat-value counter" data-target="45" data-suffix="">0</span><span class="stat-label">min from kitchen</span></div>
    <div class="stat-item"><span class="stat-value">HACCP</span><span class="stat-label">quality standard</span></div>
    <div class="stat-item"><span class="stat-value">Daily</span><span class="stat-label">fresh mise en place</span></div>
  </div></div>
  <div class="bottom-cards reveal">
    <div class="bottom-card"><div class="bottom-card-icon"><i data-lucide="shopping-bag"></i></div><h3>Order in a few clicks</h3><p>Choose dishes, add a note and order delivery or pickup.</p><div class="mt-md"><a href="/menu" class="btn btn-ghost">Open menu →</a></div></div>
    <div class="bottom-card"><div class="bottom-card-icon"><i data-lucide="calendar-check"></i></div><h3>Dinner at the restaurant</h3><p>Book a table with date, time, number of guests and notes for service.</p><div class="mt-md"><a href="/booking" class="btn btn-ghost">Book →</a></div></div>
    <div class="bottom-card"><div class="bottom-card-icon"><i data-lucide="star"></i></div><h3>Chef set</h3><p>A tasting composition for the first meeting with our kitchen.</p><div class="price-big">219 zł</div><div class="price-period">for 2 people</div><ul class="price-features"><li class="price-feature"><span class="price-check">✓</span> Chef starter</li><li class="price-feature"><span class="price-check">✓</span> Hot seafood</li><li class="price-feature"><span class="price-check">✓</span> Dessert of the day</li></ul><a href="/menu" class="btn btn-primary btn-full">Choose dishes</a></div>
  </div>
</div>
"##;

pub const DELIVERY_HTML: &str = r#"
<div class="container">
  <section class="page-header"><h1>Delivery and pickup</h1><p class="page-header-sub">We cook after confirming the order and pack dishes so they arrive in the right condition.</p></section>
  <section class="content-grid-3 reveal">
    <div class="bottom-card"><div class="bottom-card-icon"><i class="bi bi-scooter"></i></div><h3>Delivery 45-60 min</h3><p>Timing depends on address and kitchen load. We confirm details after ordering.</p></div>
    <div class="bottom-card"><div class="bottom-card-icon"><i class="bi bi-bag-check"></i></div><h3>Pickup</h3><p>Order in advance and collect your food without waiting.</p></div>
    <div class="bottom-card"><div class="bottom-card-icon"><i class="bi bi-credit-card"></i></div><h3>Payment</h3><p>Cash, card on pickup or bank transfer after order confirmation.</p></div>
  </section>
  <div class="section-sep"></div>
  <section class="booking-panel reveal"><div><span class="section-eyebrow"><i class="bi bi-geo-alt"></i> Delivery area</span><h2 class="section-title">We check the address before cooking</h2><p class="section-desc">At this stage, the order can be prepared from the menu. The next step is cart, delivery address and service notifications.</p></div><div class="delivery-list"><div><strong>Center</strong><span>from 45 min</span></div><div><strong>Nearby districts</strong><span>45-60 min</span></div><div><strong>Outside the city</strong><span>to be agreed</span></div></div><a href="/menu" class="btn btn-primary btn-lg"><i class="bi bi-bag"></i> Go to menu</a></section>
</div>
"#;

pub const BOOKING_HTML: &str = r#"
<div class="container">
  <section class="page-header"><h1>Table booking</h1><p class="page-header-sub">Choose date, time and number of guests. We confirm your booking after checking availability.</p></section>
  <section class="booking-layout reveal">
    <form class="booking-form" action="/booking" method="get">
      <label><span>Name</span><input type="text" name="name" placeholder="Your name" autocomplete="name" required></label>
      <label><span>Phone</span><input type="tel" name="phone" placeholder="+48 000 000 000" autocomplete="tel" required></label>
      <label><span>Date</span><input type="date" name="date" required></label>
      <label><span>Time</span><input type="time" name="time" required></label>
      <label><span>Guests</span><input type="number" name="guests" min="1" max="12" value="2" required></label>
      <label><span>Comment</span><textarea name="comment" rows="4" placeholder="Child seat, allergies, dinner occasion"></textarea></label>
      <button class="btn btn-primary btn-lg btn-full" type="submit"><i class="bi bi-calendar2-check"></i> Send request</button>
      <p class="form-note">For now the form works as a booking draft. Next we will connect saving and notifications.</p>
    </form>
    <aside class="booking-aside"><div class="about-side-card"><p class="side-section-title">Opening hours</p><ul class="side-list"><li>Mon-Thu: 12:00-22:00</li><li>Fri-Sat: 12:00-23:00</li><li>Sun: 12:00-21:00</li></ul><p class="side-section-title">For guests</p><ul class="side-list"><li>We hold reservations for 15 minutes</li><li>Up to 12 guests online</li><li>Special events on request</li></ul><p class="side-section-title">Contact</p><ul class="side-list"><li><i class="bi bi-telephone" style="color:var(--accent)"></i> &nbsp;+48 000 000 000</li><li><i class="bi bi-geo-alt" style="color:var(--accent)"></i> &nbsp;Poland / Ukraine</li></ul></div></aside>
  </section>
</div>
"#;

pub const ABOUT_HTML: &str = r#"
<div class="container">
  <section class="about-hero"><span class="about-eyebrow"><i class="bi bi-person-badge"></i> Chef profile</span><h1 class="chef-name">Dima Fomin</h1><p class="chef-title">Sushi chef &bull; Food technologist</p><p class="chef-intro">I build cuisine around product, seasonality, precise technique and calm professional service.</p></section>
  <div class="stats-section reveal"><div class="stats-strip"><div class="stat-item"><span class="stat-value counter" data-target="20" data-suffix="+">0</span><span class="stat-label">years of experience</span></div><div class="stat-item"><span class="stat-value counter" data-target="6" data-suffix="">0</span><span class="stat-label">countries</span></div><div class="stat-item"><span class="stat-value">Sushi</span><span class="stat-label">&amp; Seafood</span></div><div class="stat-item"><span class="stat-value">HACCP</span><span class="stat-label">quality standard</span></div></div></div>
  <div class="about-body"><div class="about-main"><div class="about-mission reveal"><span class="section-eyebrow"><i class="bi bi-compass"></i> My mission</span><div class="mission-quote"><p>A strong kitchen starts with understanding the product. I combine technique, process and taste from fish quality to mise en place and the final plate.</p></div><ul class="mission-values"><li><i class="bi bi-gem"></i> <strong>Quality</strong> — product work</li><li><i class="bi bi-gear"></i> <strong>Process</strong> — repeatable kitchen</li><li><i class="bi bi-bullseye"></i> <strong>Technique</strong> — Japanese precision</li></ul></div><div class="reveal"><span class="section-eyebrow"><i class="bi bi-stars"></i> Focus</span><h2 class="section-title">Main areas of work</h2><div class="expertise-grid"><div class="expertise-item"><i class="expertise-icon bi bi-reception-4"></i><h4>Sushi</h4><p>Rice, fish and knife work.</p></div><div class="expertise-item"><i class="expertise-icon bi bi-water"></i><h4>Fish and seafood</h4><p>Selection, handling and quality control.</p></div><div class="expertise-item"><i class="expertise-icon bi bi-lightbulb"></i><h4>Dish development</h4><p>Recipes, testing and taste refinement.</p></div><div class="expertise-item"><i class="expertise-icon bi bi-shield-check"></i><h4>HACCP</h4><p>Food safety in daily work.</p></div></div></div><section class="ingredient-catalog-cta reveal"><div><span class="section-eyebrow"><i class="bi bi-journal-richtext"></i> Ingredient catalog</span><h2 class="section-title">Products with nutrition values and photos</h2><p class="section-desc">A kitchen ingredient base: photos, macros, energy, seasonality and short technology notes.</p></div><a href="/ingredient-catalog" class="btn btn-primary btn-lg"><i class="bi bi-arrow-right-circle"></i> Open catalog</a></section></div><aside class="about-side reveal"><div class="about-side-card"><p class="side-section-title">Specialization</p><ul class="side-list"><li>Sushi and Japanese cuisine</li><li>Fish and seafood</li><li>Food technology</li><li>Quality control</li></ul><p class="side-section-title">Contact</p><ul class="side-list"><li><i class="bi bi-envelope" style="color:var(--accent)"></i> &nbsp;On request</li><li><i class="bi bi-building" style="color:var(--accent)"></i> &nbsp;FISH in HOUSE</li></ul></div></aside></div>
</div>
"#;

pub const PRIVACY_HTML: &str = r#"<div class="container legal-page"><section class="page-header"><span class="section-eyebrow"><i class="bi bi-shield-lock"></i> Privacy</span><h1>Privacy policy</h1><p class="page-header-sub">Last updated: March 13, 2026</p></section><section class="legal-content solo"><p>We respect your privacy. Data submitted through contact, booking or order forms is used only to handle the request, reservation or order.</p><p>We do not sell personal data and do not use advertising tracking pixels. Cookie details are available on the <a href="/cookie">Cookie policy</a> page.</p></section></div>"#;

pub const TERMS_HTML: &str = r#"<div class="container legal-page"><section class="page-header"><span class="section-eyebrow"><i class="bi bi-file-earmark-text"></i> Terms</span><h1>Terms</h1><p class="page-header-sub">Last updated: March 13, 2026</p></section><section class="legal-content solo"><p>The website dima-fomin.pl presents the menu, delivery, table booking and chef activity. Prices and dish availability may change depending on season and product availability.</p><p>Bookings and orders require confirmation by service. For privacy and cookies, use the <a href="/privacy">Privacy policy</a> and <a href="/cookie">Cookie policy</a> pages.</p></section></div>"#;

pub const COOKIE_HTML: &str = r##"
<div class="container legal-page"><section class="page-header"><span class="section-eyebrow"><i class="bi bi-shield-check"></i> Cookie policy</span><h1>Cookie policy</h1><p class="page-header-sub">Last updated: March 13, 2026</p></section><section class="legal-layout"><aside class="legal-index"><a href="#cookies-01"><span>01</span> What cookies are</a><a href="#cookies-02"><span>02</span> Types of cookies</a><a href="#cookies-03"><span>03</span> Essential cookies</a><a href="#cookies-04"><span>04</span> Analytics cookies</a><a href="#cookies-05"><span>05</span> Managing cookies</a><a href="#cookies-06"><span>06</span> Third-party cookies</a><a href="#cookies-07"><span>07</span> Policy updates</a></aside><article class="legal-content"><div class="legal-related"><span>Related pages:</span><a href="/privacy">Privacy policy</a><a href="/terms">Terms</a><a href="/cookie">Cookie policy</a></div><p>This Cookie Policy explains what cookies are, how we use them on dima-fomin.pl and how you can manage your preferences.</p><section id="cookies-01" class="legal-section"><span class="legal-num">01</span><h2>What cookies are</h2><p>Cookies are small text files stored on your device when you visit a website. They help remember preferences and improve browsing.</p></section><section id="cookies-02" class="legal-section"><span class="legal-num">02</span><h2>Types of cookies</h2><p>We use essential, analytics and preference cookies.</p></section><section id="cookies-03" class="legal-section"><span class="legal-num">03</span><h2>Essential cookies</h2><p>These cookies are required for the site to work, including cookie consent, theme and language preferences.</p></section><section id="cookies-04" class="legal-section"><span class="legal-num">04</span><h2>Analytics cookies</h2><p>Analytics cookies are loaded only after consent and help us understand anonymous traffic.</p></section><section id="cookies-05" class="legal-section"><span class="legal-num">05</span><h2>Managing cookies</h2><p>You can use the consent banner, clear browser cookies or disable cookies in browser settings.</p><button class="btn btn-ghost cookie-manage" type="button">Manage cookies</button></section><section id="cookies-06" class="legal-section"><span class="legal-num">06</span><h2>Third-party cookies</h2><p>Some third-party services may set their own cookies. We do not use advertising cookies or tracking pixels.</p></section><section id="cookies-07" class="legal-section"><span class="legal-num">07</span><h2>Policy updates</h2><p>We may update this policy. Changes will be published on this page.</p></section></article></section></div>
"##;

pub const NOT_FOUND_HTML: &str = r#"<div class="page-header"><h1>404</h1><p>Page not found</p><a href="/" class="btn btn-ghost">Back home</a></div>"#;

pub const MENU_TITLE: &str = "Our menu";
pub const MENU_SUBTITLE: &str =
    "Signature cuisine from seasonal products, available for delivery and pickup";
pub const MENU_ALL: &str = "Full menu";
pub const MENU_ORDER: &str = "Add to order";
pub const MENU_CART_TITLE: &str = "Cart";
pub const MENU_CART_EMPTY: &str = "Your cart is empty. Add dishes from the menu and they will appear here instantly.";
pub const MENU_CART_SUBTOTAL: &str = "Subtotal";
pub const MENU_CART_TOTAL: &str = "Total";
pub const MENU_CART_CLEAR: &str = "Clear";
pub const MENU_CART_CHECKOUT: &str = "Continue to checkout";
pub const MENU_CART_HINT: &str = "The cart is saved locally in your browser.";

pub const MENU_CATEGORIES: &[MenuCategory] = &[
    MenuCategory {
        id: "hot",
        icon: "bi-fire",
        label: "Hot dishes",
    },
    MenuCategory {
        id: "sushi",
        icon: "bi-water",
        label: "Sushi",
    },
    MenuCategory {
        id: "salads",
        icon: "bi-leaf",
        label: "Salads",
    },
    MenuCategory {
        id: "soups",
        icon: "bi-cup-hot",
        label: "Soups",
    },
    MenuCategory {
        id: "desserts",
        icon: "bi-cake2",
        label: "Desserts",
    },
    MenuCategory {
        id: "drinks",
        icon: "bi-cup-straw",
        label: "Drinks",
    },
];

pub const MENU_ITEMS: &[MenuItem] = &[
    MenuItem {
        icon: "bi-fire",
        name: "Ribeye steak",
        desc: "Marbled beef, red wine sauce, potato gratin",
        price: "139 zł",
        weight: "320 g",
        category: "hot",
        badge: "hit",
    },
    MenuItem {
        icon: "bi-fire",
        name: "Duck breast",
        desc: "Orange sauce, parsnip puree, cherry jus",
        price: "109 zł",
        weight: "280 g",
        category: "hot",
        badge: "chef",
    },
    MenuItem {
        icon: "bi-fire",
        name: "Grilled sea bass",
        desc: "Mediterranean herbs, capers, lemon butter",
        price: "89 zł",
        weight: "300 g",
        category: "hot",
        badge: "seafood",
    },
    MenuItem {
        icon: "bi-fire",
        name: "Mushroom risotto",
        desc: "Carnaroli, mascarpone, porcini, truffle cream",
        price: "69 zł",
        weight: "260 g",
        category: "hot",
        badge: "veg",
    },
    MenuItem {
        icon: "bi-water",
        name: "Salmon nigiri",
        desc: "Warm rice, fresh salmon, wasabi, soy sauce",
        price: "39 zł",
        weight: "2 pcs",
        category: "sushi",
        badge: "new",
    },
    MenuItem {
        icon: "bi-water",
        name: "Tuna-avocado roll",
        desc: "Tuna, avocado, sesame, nori, ponzu sauce",
        price: "59 zł",
        weight: "240 g",
        category: "sushi",
        badge: "hit",
    },
    MenuItem {
        icon: "bi-leaf",
        name: "Chicken Caesar",
        desc: "Romaine, parmesan, anchovy, croutons, classic sauce",
        price: "49 zł",
        weight: "230 g",
        category: "salads",
        badge: "",
    },
    MenuItem {
        icon: "bi-leaf",
        name: "Tuna tartare",
        desc: "Avocado, sesame dressing, rice chips",
        price: "69 zł",
        weight: "190 g",
        category: "salads",
        badge: "seafood",
    },
    MenuItem {
        icon: "bi-cup-hot",
        name: "Pumpkin cream soup",
        desc: "Ginger, coconut milk, roasted pumpkin seeds",
        price: "39 zł",
        weight: "300 ml",
        category: "soups",
        badge: "veg",
    },
    MenuItem {
        icon: "bi-cup-hot",
        name: "French onion soup",
        desc: "Gruyere, brioche, intense beef broth cooked for 8 hours",
        price: "43 zł",
        weight: "320 ml",
        category: "soups",
        badge: "",
    },
    MenuItem {
        icon: "bi-cake2",
        name: "Creme brulee",
        desc: "Bourbon vanilla, crisp caramel crust",
        price: "32 zł",
        weight: "140 g",
        category: "desserts",
        badge: "",
    },
    MenuItem {
        icon: "bi-cake2",
        name: "Chocolate fondant",
        desc: "70% cocoa, hot dark center, vanilla ice cream",
        price: "36 zł",
        weight: "160 g",
        category: "desserts",
        badge: "hit",
    },
    MenuItem {
        icon: "bi-cup-straw",
        name: "Homemade lemonade",
        desc: "Basil, cucumber, fresh lemon, cane sugar",
        price: "21 zł",
        weight: "350 ml",
        category: "drinks",
        badge: "",
    },
];

pub const RECIPES_TITLE: &str = "Chef blog";
pub const RECIPES_SUBTITLE: &str = "Tested techniques, recipes and practical kitchen notes";
pub const RECIPE_NOT_FOUND: &str = "Recipe not found";
pub const RECIPE_BACK: &str = "Back";
pub const RECIPE_ALL: &str = "All posts";
pub const RECIPE_INGREDIENTS: &str = "Ingredients";
pub const RECIPE_PREPARATION: &str = "Preparation";

pub const RECIPES: &[Recipe] = &[
    Recipe {
        id: "borsch",
        name: "Kyiv borscht",
        icon: "bi-cup-hot",
        time: "90 min",
        difficulty: "Medium",
        ingredients: &[
            ("Beef brisket", "500 g"),
            ("Beetroot", "2 pcs"),
            ("White cabbage", "300 g"),
            ("Potatoes", "3 pcs"),
            ("Carrot", "1 pc"),
            ("Tomato paste", "2 tbsp"),
        ],
        steps: &[
            "Cook the brisket broth gently for about 1 hour.",
            "Grate beets and stew with a little vinegar for 10 minutes.",
            "Add potatoes and cabbage to the broth.",
            "Add onion, carrot, tomato paste and beets; cook 15 minutes.",
            "Rest for 20 minutes and serve with sour cream and herbs.",
        ],
    },
    Recipe {
        id: "tiramisu",
        name: "Classic tiramisu",
        icon: "bi-cake2",
        time: "30 min + 4 h chill",
        difficulty: "Easy",
        ingredients: &[
            ("Mascarpone", "500 g"),
            ("Eggs", "4 pcs"),
            ("Sugar", "100 g"),
            ("Savoiardi biscuits", "300 g"),
            ("Strong espresso", "200 ml"),
            ("Cocoa", "2 tbsp"),
        ],
        steps: &[
            "Whisk yolks with sugar until pale and fluffy.",
            "Add mascarpone and fold gently.",
            "Whisk whites and fold into the cream.",
            "Dip biscuits in cooled espresso.",
            "Layer biscuits and cream.",
            "Chill at least 4 hours and dust with cocoa.",
        ],
    },
    Recipe {
        id: "risotto",
        name: "Porcini risotto",
        icon: "bi-fire",
        time: "35 min",
        difficulty: "Medium",
        ingredients: &[
            ("Carnaroli rice", "300 g"),
            ("Porcini", "200 g"),
            ("Mascarpone", "100 g"),
            ("Parmesan", "80 g"),
            ("Dry white wine", "150 ml"),
            ("Hot chicken stock", "1 l"),
        ],
        steps: &[
            "Saute mushrooms in butter and set aside.",
            "Sweat shallot, add rice and stir for 2 minutes.",
            "Pour wine and reduce.",
            "Add stock ladle by ladle, stirring.",
            "Finish with mushrooms, mascarpone and parmesan.",
            "Rest covered for 2 minutes and serve immediately.",
        ],
    },
];
