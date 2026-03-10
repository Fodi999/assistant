-- Seed real chef profile data for Dima Fomin
-- Date: 2026-03-10

-- ── 1. UPDATE about_page ──────────────────────────────────────────────────────
UPDATE about_page
SET
    title_en   = 'Dima Fomin — Sushi Chef & Food Technology Expert',
    title_pl   = 'Dima Fomin — Szef Sushi i Ekspert Technologii Żywności',
    title_ru   = 'Дима Фомин — Суши-шеф и технолог пищевого производства',
    title_uk   = 'Діма Фомін — Суші-шеф та технолог харчового виробництва',

    content_en = E'I am a sushi chef and food production technologist. I help restaurants build quality, processes, and menus based on product, seasonality, and technical precision.\n\nThrough this blog and my projects I share practical knowledge from professional kitchens. My goal is to help chefs, restaurants, and food enthusiasts understand the product, technique, and technology that underpin modern gastronomy. I believe a strong kitchen starts not with a recipe, but with an understanding of the process.\n\n20+ years of experience · 6 countries · HACCP certified · Sushi & Seafood specialist',

    content_pl = E'Jestem szefem sushi i technologiem produkcji żywności. Pomagam restauracjom budować jakość, procesy i menu oparte na produkcie, sezonowości i precyzji technicznej.\n\nPrzez ten blog i swoje projekty dzielę się praktyczną wiedzą z profesjonalnych kuchni. Moim celem jest pomaganie kucharzom, restauracjom i entuzjastom jedzenia w rozumieniu produktu, techniki i technologii, które stanowią podstawę nowoczesnej gastronomii. Uważam, że silna kuchnia zaczyna się nie od przepisu, ale od zrozumienia procesu.\n\nPonad 20 lat doświadczenia · 6 krajów · Certyfikat HACCP · Specjalista sushi i owoców morza',

    content_ru = E'Я суши-шеф и технолог пищевого производства. Помогаю ресторанам выстраивать качество, процессы и меню, основанные на продукте, сезонности и точности техники.\n\nЧерез этот блог и проекты я делюсь практическими знаниями профессиональной кухни. Моя цель — помогать поварам, ресторанам и энтузиастам еды понимать продукт, технику и технологии, на которых строится современная гастрономия. Я верю, что сильная кухня начинается не с рецепта, а с понимания процесса.\n\nКачество — понимание продукта · Процесс — оптимизация работы · Техника — японская точность\n\n20+ лет опыта · 6 стран · Сертификат HACCP · Специалист по суши и морепродуктам',

    content_uk = E'Я суші-шеф та технолог харчового виробництва. Допомагаю ресторанам вибудовувати якість, процеси та меню, засновані на продукті, сезонності та точності техніки.\n\nЧерез цей блог і проекти я ділюся практичними знаннями з професійної кухні. Моя мета — допомагати кухарям, ресторанам та ентузіастам їжі розуміти продукт, техніку та технології, на яких будується сучасна гастрономія. Я вірю, що сильна кухня починається не з рецепту, а з розуміння процесу.\n\nЯкість — розуміння продукту · Процес — оптимізація роботи · Техніка — японська точність\n\n20+ років досвіду · 6 країн · Сертифікат HACCP · Спеціаліст із суші та морепродуктів',

    updated_at = NOW()
WHERE id = '00000000-0000-0000-0000-000000000001';

-- ── 2. REPLACE expertise with real specializations ────────────────────────────
DELETE FROM expertise;

INSERT INTO expertise (icon, title_en, title_pl, title_ru, title_uk, order_index) VALUES
    ('🍣', 'Traditional sushi preparation and presentation',
            'Tradycyjne przygotowanie i podawanie sushi',
            'Традиционное приготовление и подача суши',
            'Традиційне приготування та подача суші', 1),
    ('🔪', 'Professional Japanese knife skills',
            'Profesjonalna praca z japońskimi nożami',
            'Профессиональная работа с японскими ножами',
            'Професійна робота з японськими ножами', 2),
    ('🐟', 'Fish & seafood selection, processing and quality control',
            'Wybór, obróbka i kontrola jakości ryb i owoców morza',
            'Выбор, обработка и контроль качества рыбы и морепродуктов',
            'Вибір, обробка та контроль якості риби та морепродуктів', 3),
    ('🧬', 'New product development and implementation',
            'Opracowywanie i wdrażanie nowych produktów',
            'Разработка и внедрение новых продуктов',
            'Розробка та впровадження нових продуктів', 4),
    ('⚙️', 'Kitchen process setup and optimization',
            'Konfiguracja i optymalizacja procesów kuchennych',
            'Настройка и оптимизация кухонных процессов',
            'Налаштування та оптимізація кухонних процесів', 5),
    ('🤖', 'Kitchen automation and culinary technology',
            'Automatyzacja kuchni i technologie gastronomiczne',
            'Автоматизация кухни и гастрономические технологии',
            'Автоматизація кухні та гастрономічні технології', 6),
    ('👨‍🏫', 'Staff training and culinary mentoring',
            'Szkolenie personelu i mentoring kulinarny',
            'Обучение персонала и кулинарное наставничество',
            'Навчання персоналу та кулінарне наставництво', 7),
    ('📋', 'HACCP food safety standards',
            'Standardy bezpieczeństwa żywności HACCP',
            'Работа по стандартам HACCP',
            'Робота за стандартами HACCP', 8);

-- ── 3. REPLACE experience with real career history ────────────────────────────
DELETE FROM experience;

INSERT INTO experience
    (restaurant, country, position, start_year, end_year,
     description_en, description_pl, description_ru, description_uk, order_index)
VALUES
    (
        'FISH in HOUSE',
        'Ukraine — Dnipro',
        'Head Chef / Food Technologist',
        2002, NULL,
        E'Flagship role spanning 20+ years.\n· New product development\n· Quality control and shelf-life extension\n· Production process optimization\n· Equipment selection\n· Staff training\n· Sales volume management',
        E'Główna rola obejmująca ponad 20 lat.\n· Opracowywanie nowych produktów\n· Kontrola jakości i wydłużanie trwałości\n· Optymalizacja procesów produkcyjnych\n· Dobór sprzętu\n· Szkolenie personelu\n· Zarządzanie wolumenem sprzedaży',
        E'Ключевая роль на протяжении 20+ лет.\n· Разработка новых продуктов\n· Контроль качества и увеличение сроков хранения\n· Настройка производственных процессов\n· Подбор оборудования\n· Обучение персонала\n· Работа с объёмами продаж',
        E'Ключова роль протягом 20+ років.\n· Розробка нових продуктів\n· Контроль якості та збільшення термінів зберігання\n· Налаштування виробничих процесів\n· Підбір обладнання\n· Навчання персоналу\n· Робота з обсягами продажів',
        1
    ),
    (
        'Restauracja Autorska "Miód Malina"',
        'Poland — Zgorzelec',
        'Cook',
        2017, 2018,
        'Signature restaurant. Worked with Polish and European cuisine.',
        'Restauracja autorska. Praca z kuchnią polską i europejską.',
        'Авторский ресторан. Работа с польской и европейской кухней.',
        'Авторський ресторан. Робота з польською та європейською кухнею.',
        2
    ),
    (
        'Restaurant Charlemagne',
        'France — Agde',
        'Cook, Seafood',
        2017, 2018,
        'French restaurant specializing in seafood and Mediterranean cuisine.',
        'Francuska restauracja specjalizująca się w owocach morza i kuchni śródziemnomorskiej.',
        'Французский ресторан с акцентом на морепродукты и средиземноморскую кухню.',
        'Французький ресторан із акцентом на морепродукти та середземноморську кухню.',
        3
    ),
    (
        'Boulangerie Pâtisserie WAWEL',
        'Canada — Montreal',
        'Cook',
        2022, 2023,
        'Polish bakery and pastry shop. Bread, pastries, and traditional Polish products.',
        'Polska piekarnia i cukiernia. Chleb, ciasta i tradycyjne polskie produkty.',
        'Польская пекарня и кондитерская. Хлеб, выпечка и традиционные польские изделия.',
        'Польська пекарня та кондитерська. Хліб, випічка та традиційні польські вироби.',
        4
    ),
    (
        'Vocational School No. 53 — Dnipro',
        'Ukraine — Dnipro',
        'Certified Cook — Diploma with Honours',
        2002, 2003,
        'Formal culinary education. Graduated with honours. Internship at Restaurant Charlie''s.',
        'Formalne wykształcenie kulinarne. Ukończenie z wyróżnieniem. Staż w restauracji Charlie''s.',
        'Профессиональное кулинарное образование. Диплом с отличием. Стажировка в ресторане Charlie''s.',
        'Професійна кулінарна освіта. Диплом з відзнакою. Стажування в ресторані Charlie''s.',
        5
    );
