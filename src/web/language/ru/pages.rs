use crate::web::language::pl::pages::{MenuCategory, MenuItem, Recipe};

pub const HOME_HTML: &str = r##"
<div class="container">
  <section class="hero">
    <div class="hero-content">
      <div class="hero-badge"><div class="hero-badge-dot">✦</div>Сегодня готовим до 22:30</div>
      <h1>Авторская кухня<br>от <span class="orange">шефа</span></h1>
      <p class="hero-sub">Свежая рыба, горячие блюда, сезонные салаты и десерты. Закажите доставку домой или забронируйте столик для спокойного ужина.</p>
      <div class="hero-cta">
        <a href="/menu" class="btn btn-primary btn-lg"><i class="bi bi-bag"></i> Заказать еду</a>
        <a href="/booking" class="btn btn-ghost btn-lg"><i class="bi bi-calendar2-check"></i> Забронировать столик</a>
      </div>
      <p class="hero-trust"><span class="hero-trust-dot"></span>Доставка 45-60 мин &nbsp;&middot;&nbsp; Самовывоз &nbsp;&middot;&nbsp; Онлайн-бронирование</p>
    </div>
    <div class="hero-visual">
      <div class="dashboard-card restaurant-card">
        <div class="dash-header"><span class="dash-title">Сет шефа на сегодня</span><span class="dash-period">Хит</span></div>
        <div class="chef-dish-plate"><div class="plate-core"><span class="plate-fish"></span><span class="plate-leaf"></span><span class="plate-sauce"></span></div></div>
        <div class="menu-card-header"><span class="menu-card-title">Лаврак на гриле с лимонным маслом</span><span class="menu-card-price">89 zł</span></div>
        <p class="menu-card-desc">Средиземноморские травы, каперсы, свежий лимон и деликатная композиция от шефа.</p>
        <div class="hero-cta"><a href="/menu?cat=hot" class="btn btn-primary btn-full"><i class="bi bi-cart-plus"></i> Посмотреть меню</a></div>
      </div>
    </div>
  </section>
  <section class="features-section reveal"><div class="features-strip">
    <div class="feature-item"><div class="feature-icon-wrap"><i data-lucide="fish"></i></div><h3>Рыба и морепродукты</h3><p>Профессиональная обработка, свежий продукт и чистый вкус без лишнего шума.</p><a href="/menu" class="feature-arrow">В меню →</a></div>
    <div class="feature-item"><div class="feature-icon-wrap"><i data-lucide="flame"></i></div><h3>Горячие блюда в доставке</h3><p>Упаковываем так, чтобы блюда приехали аккуратно и сохранили температуру.</p><a href="/delivery" class="feature-arrow">Условия →</a></div>
    <div class="feature-item"><div class="feature-icon-wrap"><i data-lucide="utensils"></i></div><h3>Столики на вечер</h3><p>Камерный зал, спокойный сервис и подготовленный столик к вашему приходу.</p><a href="/booking" class="feature-arrow">Забронировать →</a></div>
    <div class="feature-item"><div class="feature-icon-wrap"><i data-lucide="chef-hat"></i></div><h3>Кухня шефа</h3><p>20+ лет опыта в sushi, seafood, технологии продукта и ресторанной работе.</p><a href="/about" class="feature-arrow">О шефе →</a></div>
  </div></section>
  <div class="stats-section reveal"><div class="stats-strip"><div class="stat-item"><span class="stat-value counter" data-target="20" data-suffix="+">0</span><span class="stat-label">лет опыта</span></div><div class="stat-item"><span class="stat-value counter" data-target="45" data-suffix="">0</span><span class="stat-label">мин от кухни</span></div><div class="stat-item"><span class="stat-value">HACCP</span><span class="stat-label">стандарт качества</span></div><div class="stat-item"><span class="stat-value">Daily</span><span class="stat-label">свежий mise en place</span></div></div></div>
  <div class="bottom-cards reveal"><div class="bottom-card"><div class="bottom-card-icon"><i data-lucide="shopping-bag"></i></div><h3>Заказ в несколько кликов</h3><p>Выберите блюда, добавьте комментарий и оформите доставку или самовывоз.</p><div class="mt-md"><a href="/menu" class="btn btn-ghost">Открыть меню →</a></div></div><div class="bottom-card"><div class="bottom-card-icon"><i data-lucide="calendar-check"></i></div><h3>Вечер в ресторане</h3><p>Забронируйте столик заранее: дата, время, гости и пожелания для сервиса.</p><div class="mt-md"><a href="/booking" class="btn btn-ghost">Забронировать →</a></div></div><div class="bottom-card"><div class="bottom-card-icon"><i data-lucide="star"></i></div><h3>Сет шефа</h3><p>Композиция блюд для первого знакомства с нашей кухней.</p><div class="price-big">219 zł</div><div class="price-period">для 2 гостей</div><ul class="price-features"><li class="price-feature"><span class="price-check">✓</span> Закуска от шефа</li><li class="price-feature"><span class="price-check">✓</span> Горячий seafood</li><li class="price-feature"><span class="price-check">✓</span> Десерт дня</li></ul><a href="/menu" class="btn btn-primary btn-full">Выбрать блюда</a></div></div>
</div>
"##;

pub const DELIVERY_HTML: &str = r#"<div class="container"><section class="page-header"><h1>Доставка и самовывоз</h1><p class="page-header-sub">Мы готовим после подтверждения заказа и упаковываем блюда так, чтобы они приехали в правильном состоянии.</p></section><section class="content-grid-3 reveal"><div class="bottom-card"><div class="bottom-card-icon"><i class="bi bi-scooter"></i></div><h3>Доставка 45-60 мин</h3><p>Время зависит от адреса и загрузки кухни. После заказа подтвердим детали.</p></div><div class="bottom-card"><div class="bottom-card-icon"><i class="bi bi-bag-check"></i></div><h3>Самовывоз</h3><p>Оформите заказ заранее и заберите блюда без ожидания к выбранному времени.</p></div><div class="bottom-card"><div class="bottom-card-icon"><i class="bi bi-credit-card"></i></div><h3>Оплата</h3><p>Наличными, картой при получении или переводом после подтверждения заказа.</p></div></section><div class="section-sep"></div><section class="booking-panel reveal"><div><span class="section-eyebrow"><i class="bi bi-geo-alt"></i> Зона доставки</span><h2 class="section-title">Проверим адрес перед готовкой</h2><p class="section-desc">На этом этапе заказ можно подготовить из меню. Следующий шаг — корзина, адрес доставки и уведомления для сервиса.</p></div><div class="delivery-list"><div><strong>Центр</strong><span>от 45 мин</span></div><div><strong>Близкие районы</strong><span>45-60 мин</span></div><div><strong>За городом</strong><span>по договоренности</span></div></div><a href="/menu" class="btn btn-primary btn-lg"><i class="bi bi-bag"></i> Перейти в меню</a></section></div>"#;

pub const BOOKING_HTML: &str = r#"<div class="container"><section class="page-header"><h1>Бронирование столика</h1><p class="page-header-sub">Выберите дату, время и количество гостей. Мы подтвердим бронь после проверки доступности.</p></section><section class="booking-layout reveal"><form class="booking-form" action="/booking" method="get"><label><span>Имя</span><input type="text" name="name" placeholder="Ваше имя" autocomplete="name" required></label><label><span>Телефон</span><input type="tel" name="phone" placeholder="+48 000 000 000" autocomplete="tel" required></label><label><span>Дата</span><input type="date" name="date" required></label><label><span>Время</span><input type="time" name="time" required></label><label><span>Гости</span><input type="number" name="guests" min="1" max="12" value="2" required></label><label><span>Комментарий</span><textarea name="comment" rows="4" placeholder="Детский стул, аллергии, повод ужина"></textarea></label><button class="btn btn-primary btn-lg btn-full" type="submit"><i class="bi bi-calendar2-check"></i> Отправить запрос</button><p class="form-note">Пока форма работает как черновик запроса. Далее подключим сохранение брони и уведомления.</p></form><aside class="booking-aside"><div class="about-side-card"><p class="side-section-title">Часы работы</p><ul class="side-list"><li>Пн-Чт: 12:00-22:00</li><li>Пт-Сб: 12:00-23:00</li><li>Вс: 12:00-21:00</li></ul><p class="side-section-title">Для гостей</p><ul class="side-list"><li>Держим бронь 15 минут</li><li>До 12 гостей онлайн</li><li>Особые события по запросу</li></ul><p class="side-section-title">Контакт</p><ul class="side-list"><li><i class="bi bi-telephone" style="color:var(--accent)"></i> &nbsp;+48 000 000 000</li><li><i class="bi bi-geo-alt" style="color:var(--accent)"></i> &nbsp;Польша / Украина</li></ul></div></aside></section></div>"#;

pub const ABOUT_HTML: &str = r#"<div class="container"><section class="about-hero"><span class="about-eyebrow"><i class="bi bi-person-badge"></i> Профиль шефа</span><h1 class="chef-name">Dima Fomin</h1><p class="chef-title">Sushi chef &bull; Технолог пищевого производства</p><p class="chef-intro">Я строю кухню вокруг продукта, сезонности, точной техники и спокойного профессионального сервиса.</p></section><div class="stats-section reveal"><div class="stats-strip"><div class="stat-item"><span class="stat-value counter" data-target="20" data-suffix="+">0</span><span class="stat-label">лет опыта</span></div><div class="stat-item"><span class="stat-value counter" data-target="6" data-suffix="">0</span><span class="stat-label">стран работы</span></div><div class="stat-item"><span class="stat-value">Sushi</span><span class="stat-label">&amp; Seafood</span></div><div class="stat-item"><span class="stat-value">HACCP</span><span class="stat-label">стандарт качества</span></div></div></div><div class="about-body"><div class="about-main"><div class="about-mission reveal"><span class="section-eyebrow"><i class="bi bi-compass"></i> Моя миссия</span><div class="mission-quote"><p>Сильная кухня начинается с понимания продукта. Я соединяю технику, процесс и вкус: от качества рыбы до mise en place и готовой тарелки.</p></div><ul class="mission-values"><li><i class="bi bi-gem"></i> <strong>Качество</strong> — работа с продуктом</li><li><i class="bi bi-gear"></i> <strong>Процесс</strong> — стабильность кухни</li><li><i class="bi bi-bullseye"></i> <strong>Техника</strong> — японская точность</li></ul></div><div class="reveal"><span class="section-eyebrow"><i class="bi bi-stars"></i> Специализация</span><h2 class="section-title">Главные направления работы</h2><div class="expertise-grid"><div class="expertise-item"><i class="expertise-icon bi bi-reception-4"></i><h4>Sushi</h4><p>Рис, рыба и работа ножом.</p></div><div class="expertise-item"><i class="expertise-icon bi bi-water"></i><h4>Рыба и морепродукты</h4><p>Выбор, обработка и контроль качества.</p></div><div class="expertise-item"><i class="expertise-icon bi bi-lightbulb"></i><h4>Разработка блюд</h4><p>Рецептуры, тесты и настройка вкуса.</p></div><div class="expertise-item"><i class="expertise-icon bi bi-shield-check"></i><h4>HACCP</h4><p>Безопасность еды в ежедневной работе.</p></div></div></div><section class="ingredient-catalog-cta reveal"><div><span class="section-eyebrow"><i class="bi bi-journal-richtext"></i> Каталог ингредиентов</span><h2 class="section-title">Продукты с пищевой ценностью и фото</h2><p class="section-desc">База ингредиентов кухни: фото, макро, энергия, сезонность и короткие технологические заметки.</p></div><a href="/ingredient-catalog" class="btn btn-primary btn-lg"><i class="bi bi-arrow-right-circle"></i> Открыть каталог</a></section></div><aside class="about-side reveal"><div class="about-side-card"><p class="side-section-title">Специализация</p><ul class="side-list"><li>Sushi и японская кухня</li><li>Рыба и морепродукты</li><li>Технология питания</li><li>Контроль качества</li></ul><p class="side-section-title">Контакт</p><ul class="side-list"><li><i class="bi bi-envelope" style="color:var(--accent)"></i> &nbsp;По запросу</li><li><i class="bi bi-building" style="color:var(--accent)"></i> &nbsp;FISH in HOUSE</li></ul></div></aside></div></div>"#;

pub const PRIVACY_HTML: &str = r#"<div class="container legal-page"><section class="page-header"><span class="section-eyebrow"><i class="bi bi-shield-lock"></i> Приватность</span><h1>Политика конфиденциальности</h1><p class="page-header-sub">Последнее обновление: 13 марта 2026</p></section><section class="legal-content solo"><p>Мы уважаем вашу приватность. Данные из контактных, броневых или заказных форм используются только для обработки запроса, брони или заказа.</p><p>Мы не продаём персональные данные и не используем рекламные пиксели отслеживания. Подробности о cookie доступны на странице <a href="/cookie">Политика cookie</a>.</p></section></div>"#;

pub const TERMS_HTML: &str = r#"<div class="container legal-page"><section class="page-header"><span class="section-eyebrow"><i class="bi bi-file-earmark-text"></i> Правила</span><h1>Правила</h1><p class="page-header-sub">Последнее обновление: 13 марта 2026</p></section><section class="legal-content solo"><p>Сайт dima-fomin.pl показывает меню, доставку, бронирование столиков и деятельность шефа. Цены и доступность блюд могут меняться из-за сезона и наличия продукта.</p><p>Бронирования и заказы требуют подтверждения сервисом. По вопросам приватности и cookie используйте страницы <a href="/privacy">Политика конфиденциальности</a> и <a href="/cookie">Политика cookie</a>.</p></section></div>"#;

pub const COOKIE_HTML: &str = r##"<div class="container legal-page"><section class="page-header"><span class="section-eyebrow"><i class="bi bi-shield-check"></i> Политика cookie</span><h1>Политика cookie</h1><p class="page-header-sub">Последнее обновление: 13 марта 2026</p></section><section class="legal-layout"><aside class="legal-index"><a href="#cookies-01"><span>01</span> Что такое cookie</a><a href="#cookies-02"><span>02</span> Типы cookie</a><a href="#cookies-03"><span>03</span> Необходимые cookie</a><a href="#cookies-04"><span>04</span> Аналитические cookie</a><a href="#cookies-05"><span>05</span> Управление cookie</a><a href="#cookies-06"><span>06</span> Cookie третьих сторон</a><a href="#cookies-07"><span>07</span> Обновления политики</a></aside><article class="legal-content"><div class="legal-related"><span>Связанные страницы:</span><a href="/privacy">Политика конфиденциальности</a><a href="/terms">Правила</a><a href="/cookie">Политика cookie</a></div><p>Эта Политика cookie объясняет, что такое cookie, как мы используем их на dima-fomin.pl и как можно управлять предпочтениями.</p><section id="cookies-01" class="legal-section"><span class="legal-num">01</span><h2>Что такое cookie</h2><p>Cookie — это небольшие текстовые файлы, которые сохраняются на устройстве при посещении сайта. Они помогают запоминать настройки и улучшать просмотр.</p></section><section id="cookies-02" class="legal-section"><span class="legal-num">02</span><h2>Типы cookie</h2><p>Мы используем необходимые, аналитические cookie и cookie предпочтений.</p></section><section id="cookies-03" class="legal-section"><span class="legal-num">03</span><h2>Необходимые cookie</h2><p>Эти cookie нужны для работы сайта, включая согласие на cookie, тему и язык.</p></section><section id="cookies-04" class="legal-section"><span class="legal-num">04</span><h2>Аналитические cookie</h2><p>Аналитика загружается только после согласия и помогает понимать анонимный трафик.</p></section><section id="cookies-05" class="legal-section"><span class="legal-num">05</span><h2>Управление cookie</h2><p>Можно использовать баннер согласия, очистить cookie в браузере или отключить их в настройках браузера.</p><button class="btn btn-ghost cookie-manage" type="button">Управлять cookie</button></section><section id="cookies-06" class="legal-section"><span class="legal-num">06</span><h2>Cookie третьих сторон</h2><p>Некоторые сторонние сервисы могут устанавливать собственные cookie. Мы не используем рекламные cookie или пиксели отслеживания.</p></section><section id="cookies-07" class="legal-section"><span class="legal-num">07</span><h2>Обновления политики</h2><p>Мы можем обновлять эту политику. Изменения будут опубликованы на этой странице.</p></section></article></section></div>"##;

pub const NOT_FOUND_HTML: &str = r#"<div class="page-header"><h1>404</h1><p>Страница не найдена</p><a href="/" class="btn btn-ghost">На главную</a></div>"#;

pub const MENU_TITLE: &str = "Наше меню";
pub const MENU_SUBTITLE: &str =
    "Авторская кухня из сезонных продуктов, доступная для доставки и самовывоза";
pub const MENU_ALL: &str = "Всё меню";
pub const MENU_ORDER: &str = "В заказ";
pub const MENU_CART_TITLE: &str = "Корзина";
pub const MENU_CART_EMPTY: &str = "Корзина пуста. Добавьте блюда из меню, и они сразу появятся здесь.";
pub const MENU_CART_SUBTOTAL: &str = "Сумма";
pub const MENU_CART_TOTAL: &str = "Итого";
pub const MENU_CART_CLEAR: &str = "Очистить";
pub const MENU_CART_CHECKOUT: &str = "Перейти к оформлению";
pub const MENU_CART_HINT: &str = "Корзина сохраняется локально в браузере.";

pub const MENU_CATEGORIES: &[MenuCategory] = &[
    MenuCategory {
        id: "hot",
        icon: "bi-fire",
        label: "Горячие блюда",
    },
    MenuCategory {
        id: "sushi",
        icon: "bi-water",
        label: "Sushi",
    },
    MenuCategory {
        id: "salads",
        icon: "bi-leaf",
        label: "Салаты",
    },
    MenuCategory {
        id: "soups",
        icon: "bi-cup-hot",
        label: "Супы",
    },
    MenuCategory {
        id: "desserts",
        icon: "bi-cake2",
        label: "Десерты",
    },
    MenuCategory {
        id: "drinks",
        icon: "bi-cup-straw",
        label: "Напитки",
    },
];

pub const MENU_ITEMS: &[MenuItem] = &[
    MenuItem {
        icon: "bi-fire",
        name: "Стейк ribeye",
        desc: "Мраморная говядина, соус из красного вина, картофель gratin",
        price: "139 zł",
        weight: "320 г",
        category: "hot",
        badge: "хит",
    },
    MenuItem {
        icon: "bi-fire",
        name: "Утиная грудка",
        desc: "Апельсиновый соус, пюре из пастернака, вишнёвый jus",
        price: "109 zł",
        weight: "280 г",
        category: "hot",
        badge: "шеф",
    },
    MenuItem {
        icon: "bi-fire",
        name: "Лаврак на гриле",
        desc: "Средиземноморские травы, каперсы, лимонное масло",
        price: "89 zł",
        weight: "300 г",
        category: "hot",
        badge: "seafood",
    },
    MenuItem {
        icon: "bi-fire",
        name: "Ризотто с грибами",
        desc: "Carnaroli, mascarpone, белые грибы, трюфельный крем",
        price: "69 zł",
        weight: "260 г",
        category: "hot",
        badge: "veg",
    },
    MenuItem {
        icon: "bi-water",
        name: "Нигири с лососем",
        desc: "Тёплый рис, свежий лосось, wasabi, соевый соус",
        price: "39 zł",
        weight: "2 шт.",
        category: "sushi",
        badge: "new",
    },
    MenuItem {
        icon: "bi-water",
        name: "Ролл тунец-авокадо",
        desc: "Тунец, авокадо, кунжут, nori, соус ponzu",
        price: "59 zł",
        weight: "240 г",
        category: "sushi",
        badge: "хит",
    },
    MenuItem {
        icon: "bi-leaf",
        name: "Цезарь с курицей",
        desc: "Romaine, пармезан, анчоусы, гренки, классический соус",
        price: "49 zł",
        weight: "230 г",
        category: "salads",
        badge: "",
    },
    MenuItem {
        icon: "bi-leaf",
        name: "Тартар из тунца",
        desc: "Авокадо, кунжутный dressing, рисовые чипсы",
        price: "69 zł",
        weight: "190 г",
        category: "salads",
        badge: "seafood",
    },
    MenuItem {
        icon: "bi-cup-hot",
        name: "Крем-суп из тыквы",
        desc: "Имбирь, кокосовое молоко, жареные тыквенные семечки",
        price: "39 zł",
        weight: "300 мл",
        category: "soups",
        badge: "veg",
    },
    MenuItem {
        icon: "bi-cup-hot",
        name: "Французский луковый суп",
        desc: "Gruyere, бриошь, насыщенный говяжий бульон 8 часов",
        price: "43 zł",
        weight: "320 мл",
        category: "soups",
        badge: "",
    },
    MenuItem {
        icon: "bi-cake2",
        name: "Creme brulee",
        desc: "Ваниль Bourbon, хрустящая карамельная корочка",
        price: "32 zł",
        weight: "140 г",
        category: "desserts",
        badge: "",
    },
    MenuItem {
        icon: "bi-cake2",
        name: "Шоколадный fondant",
        desc: "70% какао, горячая тёмная середина, ванильное мороженое",
        price: "36 zł",
        weight: "160 г",
        category: "desserts",
        badge: "хит",
    },
    MenuItem {
        icon: "bi-cup-straw",
        name: "Домашний лимонад",
        desc: "Базилик, огурец, свежий лимон, тростниковый сахар",
        price: "21 zł",
        weight: "350 мл",
        category: "drinks",
        badge: "",
    },
];

pub const RECIPES_TITLE: &str = "Блог шефа";
pub const RECIPES_SUBTITLE: &str = "Проверенные техники, рецептуры и практичные заметки с кухни";
pub const RECIPE_NOT_FOUND: &str = "Рецепт не найден";
pub const RECIPE_BACK: &str = "Назад";
pub const RECIPE_ALL: &str = "Все записи";
pub const RECIPE_INGREDIENTS: &str = "Ингредиенты";
pub const RECIPE_PREPARATION: &str = "Приготовление";

pub const RECIPES: &[Recipe] = &[
    Recipe {
        id: "borsch",
        name: "Киевский борщ",
        icon: "bi-cup-hot",
        time: "90 мин",
        difficulty: "Средний",
        ingredients: &[
            ("Говяжья грудинка", "500 г"),
            ("Свёкла", "2 шт."),
            ("Белая капуста", "300 г"),
            ("Картофель", "3 шт."),
            ("Морковь", "1 шт."),
            ("Томатная паста", "2 ст. л."),
        ],
        steps: &[
            "Сварите бульон из грудинки на слабом огне около 1 часа.",
            "Натрите свёклу и тушите с каплей уксуса 10 минут.",
            "Добавьте в бульон картофель и капусту.",
            "Добавьте зажарку из лука, моркови, томата и свёклу; варите 15 минут.",
            "Дайте настояться 20 минут, подавайте со сметаной и зеленью.",
        ],
    },
    Recipe {
        id: "tiramisu",
        name: "Классический тирамису",
        icon: "bi-cake2",
        time: "30 мин + 4 ч охлаждения",
        difficulty: "Лёгкий",
        ingredients: &[
            ("Mascarpone", "500 г"),
            ("Яйца", "4 шт."),
            ("Сахар", "100 г"),
            ("Savoiardi", "300 г"),
            ("Крепкий espresso", "200 мл"),
            ("Какао", "2 ст. л."),
        ],
        steps: &[
            "Взбейте желтки с сахаром до светлой пышной массы.",
            "Добавьте mascarpone и аккуратно перемешайте.",
            "Взбейте белки и соедините с кремом.",
            "Окунайте печенье в остывший espresso.",
            "Соберите слои: печенье, крем, печенье, крем.",
            "Охладите минимум 4 часа и посыпьте какао.",
        ],
    },
    Recipe {
        id: "risotto",
        name: "Ризотто с белыми грибами",
        icon: "bi-fire",
        time: "35 мин",
        difficulty: "Средний",
        ingredients: &[
            ("Рис Carnaroli", "300 г"),
            ("Белые грибы", "200 г"),
            ("Mascarpone", "100 г"),
            ("Пармезан", "80 г"),
            ("Белое сухое вино", "150 мл"),
            ("Горячий куриный бульон", "1 л"),
        ],
        steps: &[
            "Обжарьте грибы на масле и отложите.",
            "Пассеруйте шалот, добавьте рис и мешайте 2 минуты.",
            "Влейте вино и выпарите.",
            "Добавляйте бульон половником, постоянно мешая.",
            "В конце добавьте грибы, mascarpone и пармезан.",
            "Снимите с огня, накройте на 2 минуты и подавайте сразу.",
        ],
    },
];
