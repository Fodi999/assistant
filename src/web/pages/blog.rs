use crate::web::language::{self, Lang};

struct Article {
    slug: &'static str,
    category: &'static str,
    title: &'static str,
    excerpt: &'static str,
    read_time: &'static str,
    date: &'static str,
    icon: &'static str,
    tone: &'static str,
    intro: &'static str,
    sections: &'static [(&'static str, &'static str)],
}

fn articles(lang: Lang) -> [Article; 3] {
    match lang {
        language::Lang::Ru => [
            Article {
                slug: "how-to-choose-fresh-fish",
                category: "Продукт",
                title: "Как выбрать свежую рыбу: профессиональный чек-лист",
                excerpt: "Запах, текстура, температура и признаки, которые действительно важны на закупке.",
                read_time: "6 мин",
                date: "7 июня 2026",
                icon: "bi-water",
                tone: "ocean",
                intro: "Качество рыбного блюда определяется задолго до ножа и разделочной доски. Этот чек-лист помогает быстро оценить продукт и принять спокойное профессиональное решение.",
                sections: &[
                    ("Начните с температуры", "Охлаждённая рыба должна храниться максимально близко к температуре тающего льда. Проверьте не только витрину, но и сам продукт: холод должен ощущаться равномерно."),
                    ("Смотрите на структуру", "Плотная упругая мякоть, прозрачные глаза и чистые жабры важнее красивой выкладки. После лёгкого нажатия поверхность должна быстро восстановиться."),
                    ("Оценивайте аромат", "Свежая рыба пахнет морем и чистой водой. Резкий аммиачный, кислый или тяжёлый запах — достаточная причина отказаться от продукта."),
                ],
            },
            Article {
                slug: "consistent-sushi-rice",
                category: "Техника",
                title: "Рис для суши: стабильный результат на каждой варке",
                excerpt: "Как управлять промывкой, водой, отдыхом и заправкой без случайностей.",
                read_time: "8 мин",
                date: "31 мая 2026",
                icon: "bi-reception-4",
                tone: "rice",
                intro: "Хороший рис для суши — не секретный рецепт, а управляемый процесс. Стабильность появляется, когда каждый этап измеряется и повторяется.",
                sections: &[
                    ("Промывка без спешки", "Промывайте рис небольшими порциями холодной водой, не повреждая зерно. Цель — убрать лишний поверхностный крахмал, сохранив структуру."),
                    ("Вода и отдых", "Соотношение воды зависит от сорта и оборудования. После варки обязательно дайте рису отдохнуть под закрытой крышкой, чтобы влага распределилась равномерно."),
                    ("Заправка и температура", "Вносите заправку аккуратно, режущими движениями лопатки. Для работы рис должен быть тёплым, блестящим и держать форму без липкой массы."),
                ],
            },
            Article {
                slug: "balancing-a-new-recipe",
                category: "Рецептура",
                title: "Баланс вкуса: как шеф настраивает новый рецепт",
                excerpt: "Практическая система проверки соли, кислоты, сладости, умами и текстуры.",
                read_time: "7 мин",
                date: "24 мая 2026",
                icon: "bi-sliders",
                tone: "citrus",
                intro: "Новый рецепт становится рабочим не тогда, когда он один раз получился вкусным, а когда команда может стабильно повторить задуманный баланс.",
                sections: &[
                    ("Определите главный вкус", "Сначала сформулируйте, что гость должен почувствовать первым. Остальные элементы должны поддерживать эту идею, а не спорить с ней."),
                    ("Меняйте по одному параметру", "Во время тестов корректируйте только один фактор за раз: соль, кислоту, сладость или текстуру. Так легче понять причину результата."),
                    ("Зафиксируйте стандарт", "Запишите вес, температуру, время и выход. Финальная дегустация должна проходить в тех же условиях, в которых блюдо получит гость."),
                ],
            },
        ],
        language::Lang::Pl => [
            Article {
                slug: "how-to-choose-fresh-fish",
                category: "Produkt",
                title: "Jak wybrać świeżą rybę: profesjonalna lista kontroli",
                excerpt: "Zapach, tekstura, temperatura i sygnały naprawdę ważne podczas zakupu.",
                read_time: "6 min",
                date: "7 czerwca 2026",
                icon: "bi-water",
                tone: "ocean",
                intro: "Jakość dania rybnego powstaje długo przed użyciem noża. Ta lista pomaga szybko ocenić produkt i podjąć profesjonalną decyzję.",
                sections: &[
                    ("Zacznij od temperatury", "Schłodzona ryba powinna być przechowywana blisko temperatury topniejącego lodu. Sprawdź nie tylko ladę, ale również sam produkt."),
                    ("Sprawdź strukturę", "Sprężyste mięso, przejrzyste oczy i czyste skrzela są ważniejsze niż efektowna ekspozycja. Powierzchnia powinna szybko wracać po naciśnięciu."),
                    ("Oceń aromat", "Świeża ryba pachnie morzem i czystą wodą. Ostry zapach amoniaku, kwaśny lub ciężki aromat oznacza, że należy zrezygnować z produktu."),
                ],
            },
            Article {
                slug: "consistent-sushi-rice",
                category: "Technika",
                title: "Ryż do sushi: stabilny rezultat przy każdym gotowaniu",
                excerpt: "Jak kontrolować płukanie, wodę, odpoczynek i zaprawę bez przypadku.",
                read_time: "8 min",
                date: "31 maja 2026",
                icon: "bi-reception-4",
                tone: "rice",
                intro: "Dobry ryż do sushi nie jest sekretnym przepisem, lecz kontrolowanym procesem. Stabilność pojawia się wtedy, gdy każdy etap jest mierzony i powtarzalny.",
                sections: &[
                    ("Płukanie bez pośpiechu", "Płucz ryż małymi porcjami zimnej wody, nie uszkadzając ziaren. Celem jest usunięcie nadmiaru skrobi z powierzchni."),
                    ("Woda i odpoczynek", "Proporcja wody zależy od odmiany ryżu i sprzętu. Po gotowaniu pozostaw ryż pod zamkniętą pokrywą, aby wilgoć rozłożyła się równomiernie."),
                    ("Zaprawa i temperatura", "Dodawaj zaprawę delikatnie, tnącymi ruchami łopatki. Ryż do pracy powinien być ciepły, błyszczący i zachowywać kształt."),
                ],
            },
            Article {
                slug: "balancing-a-new-recipe",
                category: "Receptura",
                title: "Balans smaku: jak szef dopracowuje nową recepturę",
                excerpt: "Praktyczny system kontroli soli, kwasowości, słodyczy, umami i tekstury.",
                read_time: "7 min",
                date: "24 maja 2026",
                icon: "bi-sliders",
                tone: "citrus",
                intro: "Nowa receptura staje się gotowa nie wtedy, gdy raz smakuje dobrze, ale wtedy, gdy zespół potrafi stabilnie odtworzyć zamierzony balans.",
                sections: &[
                    ("Określ główny smak", "Najpierw ustal, co gość powinien poczuć jako pierwsze. Pozostałe elementy mają wspierać tę ideę, a nie z nią konkurować."),
                    ("Zmieniaj jeden parametr", "Podczas testów koryguj tylko jeden czynnik naraz: sól, kwasowość, słodycz albo teksturę. Łatwiej wtedy zrozumieć rezultat."),
                    ("Zapisz standard", "Zanotuj wagę, temperaturę, czas i wydajność. Końcowa degustacja powinna odbyć się w warunkach zbliżonych do serwisu."),
                ],
            },
        ],
        language::Lang::En => [
            Article {
                slug: "how-to-choose-fresh-fish",
                category: "Ingredient",
                title: "How to choose fresh fish: a professional checklist",
                excerpt: "Aroma, texture, temperature and the signals that truly matter when buying.",
                read_time: "6 min",
                date: "June 7, 2026",
                icon: "bi-water",
                tone: "ocean",
                intro: "The quality of a fish dish is decided long before the knife touches the board. This checklist helps assess the product quickly and make a calm professional decision.",
                sections: &[
                    ("Start with temperature", "Chilled fish should be held close to the temperature of melting ice. Check the product itself, not only the display."),
                    ("Look at structure", "Firm flesh, clear eyes and clean gills matter more than an attractive display. The surface should recover quickly after gentle pressure."),
                    ("Evaluate aroma", "Fresh fish smells of the sea and clean water. A sharp ammonia, sour or heavy smell is enough reason to reject it."),
                ],
            },
            Article {
                slug: "consistent-sushi-rice",
                category: "Technique",
                title: "Sushi rice: consistent results with every batch",
                excerpt: "How to control washing, water, resting and seasoning without guesswork.",
                read_time: "8 min",
                date: "May 31, 2026",
                icon: "bi-reception-4",
                tone: "rice",
                intro: "Good sushi rice is not a secret recipe. It is a controlled process. Consistency appears when every stage is measured and repeated.",
                sections: &[
                    ("Wash without rushing", "Wash rice in small batches with cold water without damaging the grain. The goal is to remove excess surface starch while preserving structure."),
                    ("Water and rest", "The water ratio depends on the variety and equipment. After cooking, let the rice rest under a closed lid so moisture distributes evenly."),
                    ("Seasoning and temperature", "Fold in the seasoning carefully with cutting motions. Working rice should be warm, glossy and able to hold its shape."),
                ],
            },
            Article {
                slug: "balancing-a-new-recipe",
                category: "Recipe development",
                title: "Flavor balance: how a chef develops a new recipe",
                excerpt: "A practical system for checking salt, acidity, sweetness, umami and texture.",
                read_time: "7 min",
                date: "May 24, 2026",
                icon: "bi-sliders",
                tone: "citrus",
                intro: "A new recipe is ready not when it tastes good once, but when the team can consistently reproduce the intended balance.",
                sections: &[
                    ("Define the leading flavor", "First decide what the guest should notice first. Every other element should support that idea instead of competing with it."),
                    ("Change one variable", "During testing, adjust only one factor at a time: salt, acidity, sweetness or texture. This makes the result easier to understand."),
                    ("Document the standard", "Record weight, temperature, time and yield. Final tasting should happen under conditions close to the actual service."),
                ],
            },
        ],
    }
}

fn labels(
    lang: Lang,
) -> (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
) {
    match lang {
        Lang::Ru => ("Блог шефа", "Заметки о продуктах, технике и рецептах", "Практические материалы из ежедневной работы кухни: без лишней теории, с понятными решениями.", "Все статьи", "Читать статью"),
        Lang::Pl => ("Blog szefa", "Notatki o produktach, technice i recepturach", "Praktyczne materiały z codziennej pracy kuchni: bez zbędnej teorii, z konkretnymi rozwiązaniami.", "Wszystkie artykuły", "Czytaj artykuł"),
        Lang::En => ("Chef journal", "Notes on ingredients, technique and recipes", "Practical lessons from daily kitchen work: focused, clear and ready to use.", "All articles", "Read article"),
    }
}

fn card(article: &Article, featured: bool, read_label: &str) -> String {
    format!(
        r#"<article class="chef-blog-card{featured}">
  <a class="chef-blog-cover tone-{tone}" href="/blog/{slug}" aria-label="{title}">
    <i class="bi {icon}"></i><span>{category}</span>
  </a>
  <div class="chef-blog-card-body">
    <div class="chef-blog-meta"><span>{category}</span><time>{date}</time><span>{read_time}</span></div>
    <h3><a href="/blog/{slug}">{title}</a></h3>
    <p>{excerpt}</p>
    <a class="chef-blog-read" href="/blog/{slug}">{read_label}<i class="bi bi-arrow-up-right"></i></a>
  </div>
</article>"#,
        featured = if featured { " featured" } else { "" },
        tone = article.tone,
        slug = article.slug,
        title = article.title,
        icon = article.icon,
        category = article.category,
        date = article.date,
        read_time = article.read_time,
        excerpt = article.excerpt,
        read_label = read_label,
    )
}

pub fn about_section(lang: Lang) -> String {
    let (eyebrow, title, desc, all, read) = labels(lang);
    let items = articles(lang);
    format!(
        r#"<section class="chef-blog-section reveal">
  <div class="chef-blog-heading">
    <div><span class="section-eyebrow"><i class="bi bi-journal-richtext"></i> {eyebrow}</span><h2>{title}</h2><p>{desc}</p></div>
    <a href="/blog" class="btn btn-ghost">{all}<i class="bi bi-arrow-right"></i></a>
  </div>
  <div class="chef-blog-grid">{cards}</div>
</section>"#,
        cards = items
            .iter()
            .enumerate()
            .map(|(index, article)| card(article, index == 0, read))
            .collect::<String>()
    )
}

pub fn list(lang: Lang) -> String {
    let (eyebrow, title, desc, _, read) = labels(lang);
    let items = articles(lang);
    format!(
        r#"<div class="container blog-page"><header class="blog-page-header"><span class="section-eyebrow"><i class="bi bi-journal-richtext"></i> {eyebrow}</span><h1>{title}</h1><p>{desc}</p></header><div class="chef-blog-grid blog-archive">{cards}</div></div>"#,
        cards = items
            .iter()
            .map(|article| card(article, false, read))
            .collect::<String>()
    )
}

pub fn detail(lang: Lang, slug: &str) -> Option<String> {
    let article = articles(lang)
        .into_iter()
        .find(|article| article.slug == slug)?;
    let back = match lang {
        Lang::Ru => "Все статьи",
        Lang::Pl => "Wszystkie artykuły",
        Lang::En => "All articles",
    };
    let sections = article
        .sections
        .iter()
        .map(|(title, body)| format!("<section><h2>{title}</h2><p>{body}</p></section>"))
        .collect::<String>();
    Some(format!(
        r#"<div class="container blog-article-page">
  <a href="/blog" class="blog-back"><i class="bi bi-arrow-left"></i>{back}</a>
  <article class="blog-article">
    <header><div class="chef-blog-meta"><span>{category}</span><time>{date}</time><span>{read_time}</span></div><h1>{title}</h1><p>{excerpt}</p></header>
    <div class="blog-article-cover tone-{tone}"><i class="bi {icon}"></i><span>{category}</span></div>
    <div class="blog-article-content"><p class="blog-lead">{intro}</p>{sections}</div>
    <footer><span>Dima Fomin</span><small>Sushi chef &bull; Food technologist</small></footer>
  </article>
</div>"#,
        category = article.category,
        date = article.date,
        read_time = article.read_time,
        title = article.title,
        excerpt = article.excerpt,
        tone = article.tone,
        icon = article.icon,
        intro = article.intro,
    ))
}

pub fn title(lang: Lang, slug: &str) -> Option<&'static str> {
    articles(lang)
        .into_iter()
        .find(|article| article.slug == slug)
        .map(|article| article.title)
}
