pub fn render(cat: Option<&str>) -> String {
    let cats: &[(&str, &str, &str)] = &[
        ("hot",      "bi-fire",            "Горячее"),
        ("salads",   "bi-leaf",            "Салаты"),
        ("soups",    "bi-cup-hot",         "Супы"),
        ("desserts", "bi-cake2",           "Десерты"),
        ("drinks",   "bi-cup-straw",       "Напитки"),
    ];
    let items: &[(&str, &str, &str, &str, &str)] = &[
        ("bi-fire",      "Стейк Рибай",          "Мраморная говядина, соус из красного вина, картофель гратен", "1 850 ₽", "hot"),
        ("bi-fire",      "Утиная грудка",         "Апельсиновый соус, пюре из пастернака, вишнёвый джус",       "1 450 ₽", "hot"),
        ("bi-fire",      "Сибас на гриле",        "Средиземноморские травы, каперсы, лимонное масло",           "1 200 ₽", "hot"),
        ("bi-fire",      "Ризотто с грибами",     "Карнероли, маскарпоне, белые грибы, трюфельный крем",         "890 ₽",  "hot"),
        ("bi-leaf",      "Цезарь с курицей",      "Романо, пармезан, анчоусы, крутоны, классический соус",       "680 ₽",  "salads"),
        ("bi-leaf",      "Тартар из тунца",       "Авокадо, кунжутная заправка, рисовые чипсы",                  "890 ₽",  "salads"),
        ("bi-cup-hot",   "Крем-суп из тыквы",     "Имбирь, кокосовые сливки, обжаренные тыквенные семечки",      "520 ₽",  "soups"),
        ("bi-cup-hot",   "Французский луковый",   "Грюйер, бриошь, насыщенный говяжий бульон 8 часов",           "580 ₽",  "soups"),
        ("bi-cake2",     "Крем-брюле",            "Ваниль Бурбон, хрустящая карамельная корочка",                "420 ₽",  "desserts"),
        ("bi-cake2",     "Шоколадный фондан",     "70% какао, горячее тёмное ядро, ванильное мороженое",         "480 ₽",  "desserts"),
        ("bi-cup-straw", "Лимонад ручной работы", "Базилик, огурец, свежевыжатый лимон, тростниковый сахар",     "280 ₽",  "drinks"),
    ];

    let current = cat.unwrap_or("all");
    let mut out = String::new();

    out.push_str(r#"<section class="page-header"><h1>Наше меню</h1><p>Авторская кухня с сезонными продуктами</p></section>"#);
    out.push_str(r#"<div class="tabs">"#);
    out.push_str(&format!(
        r#"<button class="tab-btn{}" data-cat="all"><i class="bi bi-grid"></i> Всё меню</button>"#,
        if current == "all" { " active" } else { "" }
    ));
    for (id, icon, label) in cats {
        out.push_str(&format!(
            r#"<button class="tab-btn{}" data-cat="{}"><i class="bi {}"></i> {}</button>"#,
            if current == *id { " active" } else { "" }, id, icon, label
        ));
    }
    out.push_str("</div>");
    out.push_str(r#"<section class="menu-grid">"#);
    for (icon, name, desc, price, c) in items {
        let hidden = if current == "all" || current == *c { "" } else { " hidden" };
        out.push_str(&format!(
            r#"<div class="menu-card{}" data-cat="{}"><div class="menu-card-icon"><i class="bi {}"></i></div><h3>{}</h3><p>{}</p><span class="price">{}</span></div>"#,
            hidden, c, icon, name, desc, price
        ));
    }
    out.push_str("</section>");
    out
}
