//! Axum handlers render pages as HTML responses.

use axum::{
    extract::Query,
    http::{header, HeaderMap},
    response::Html,
};
use serde::Deserialize;

use crate::web::{language, pages, shell, Seo};

fn resolve_lang(headers: &HeaderMap, query_lang: Option<&str>) -> language::Lang {
    let cookie_header = headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok());
    language::Lang::resolve(query_lang, cookie_header)
}

#[derive(Deserialize)]
pub struct LangQuery {
    pub lang: Option<String>,
}

#[derive(Deserialize)]
pub struct MenuQuery {
    pub cat: Option<String>,
    pub lang: Option<String>,
}

fn description(lang: language::Lang, page: &str) -> &'static str {
    match (lang, page) {
        (language::Lang::Pl, "home") => {
            "Autorska kuchnia Dimy Fomina: menu, dostawa i rezerwacja stolika."
        }
        (language::Lang::Ru, "home") => {
            "Авторская кухня Димы Фомина: меню, доставка и бронирование столика."
        }
        (language::Lang::En, "home") => {
            "Dima Fomin's signature cuisine: menu, delivery and table booking."
        }
        (language::Lang::Pl, "menu") => {
            "Poznaj menu autorskiej kuchni Dimy Fomina i zamów ulubione dania."
        }
        (language::Lang::Ru, "menu") => {
            "Познакомьтесь с меню авторской кухни Димы Фомина и закажите любимые блюда."
        }
        (language::Lang::En, "menu") => {
            "Explore Dima Fomin's signature menu and order your favorite dishes."
        }
        (language::Lang::Pl, "delivery") => {
            "Dostawa autorskich dań Dimy Fomina prosto pod Twoje drzwi."
        }
        (language::Lang::Ru, "delivery") => {
            "Доставка авторских блюд Димы Фомина прямо к вашей двери."
        }
        (language::Lang::En, "delivery") => "Dima Fomin's signature dishes delivered to your door.",
        (language::Lang::Pl, "booking") => {
            "Zarezerwuj stolik i spędź spokojny wieczór przy autorskiej kuchni."
        }
        (language::Lang::Ru, "booking") => {
            "Забронируйте столик для спокойного вечера с авторской кухней."
        }
        (language::Lang::En, "booking") => {
            "Book a table for a relaxed evening with signature cuisine."
        }
        (language::Lang::Pl, "recipes") => {
            "Przepisy, techniki i inspiracje kulinarne od szefa Dimy Fomina."
        }
        (language::Lang::Ru, "recipes") => {
            "Рецепты, техники и кулинарные идеи от шефа Димы Фомина."
        }
        (language::Lang::En, "recipes") => {
            "Recipes, techniques and culinary inspiration from chef Dima Fomin."
        }
        (language::Lang::Pl, "about") => {
            "Poznaj szefa Dimę Fomina, jego doświadczenie i podejście do produktu."
        }
        (language::Lang::Ru, "about") => {
            "Познакомьтесь с шефом Димой Фоминым, его опытом и подходом к продуктам."
        }
        (language::Lang::En, "about") => {
            "Meet chef Dima Fomin, his experience and approach to ingredients."
        }
        (language::Lang::Pl, "blog") => {
            "Praktyczne artykuły szefa Dimy Fomina o produktach, technice i recepturach."
        }
        (language::Lang::Ru, "blog") => {
            "Практические статьи шефа Димы Фомина о продуктах, технике и рецептах."
        }
        (language::Lang::En, "blog") => {
            "Practical articles from chef Dima Fomin about ingredients, technique and recipes."
        }
        (language::Lang::Pl, "ingredients") => {
            "Katalog składników ze zdjęciami, wartościami odżywczymi i zastosowaniem kulinarnym."
        }
        (language::Lang::Ru, "ingredients") => {
            "Каталог ингредиентов с фотографиями, пищевой ценностью и кулинарным применением."
        }
        (language::Lang::En, "ingredients") => {
            "Ingredient catalog with photos, nutrition facts and culinary uses."
        }
        (language::Lang::Pl, "legal") => "Informacje prawne serwisu Dima Fomin Chef.",
        (language::Lang::Ru, "legal") => "Правовая информация сайта Dima Fomin Chef.",
        (language::Lang::En, "legal") => "Legal information for the Dima Fomin Chef website.",
        (language::Lang::Pl, _) => "Autorska kuchnia, wiedza i inspiracje kulinarne Dimy Fomina.",
        (language::Lang::Ru, _) => "Авторская кухня, знания и кулинарные идеи Димы Фомина.",
        (language::Lang::En, _) => {
            "Signature cuisine, knowledge and culinary inspiration from Dima Fomin."
        }
    }
}

fn seo<'a>(lang: language::Lang, title: &'a str, page: &str, path: &'a str) -> Seo<'a> {
    Seo {
        title,
        description: description(lang, page),
        path,
        index: true,
    }
}

fn slug_title(slug: &str) -> String {
    let text = slug.replace(['-', '_'], " ");
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => text,
    }
}

fn state_title(lang: language::Lang, state: &str) -> &'static str {
    match (lang, state) {
        (language::Lang::Pl, "raw") => "surowy",
        (language::Lang::Pl, "boiled") => "gotowany",
        (language::Lang::Pl, "steamed") => "na parze",
        (language::Lang::Pl, "baked") => "pieczony",
        (language::Lang::Pl, "grilled") => "grillowany",
        (language::Lang::Pl, "fried") => "smażony",
        (language::Lang::Pl, "smoked") => "wędzony",
        (language::Lang::Pl, "frozen") => "mrożony",
        (language::Lang::Pl, "dried") => "suszony",
        (language::Lang::Pl, "pickled") => "marynowany",
        (language::Lang::Ru, "raw") => "сырой",
        (language::Lang::Ru, "boiled") => "варёный",
        (language::Lang::Ru, "steamed") => "на пару",
        (language::Lang::Ru, "baked") => "запечённый",
        (language::Lang::Ru, "grilled") => "на гриле",
        (language::Lang::Ru, "fried") => "жареный",
        (language::Lang::Ru, "smoked") => "копчёный",
        (language::Lang::Ru, "frozen") => "замороженный",
        (language::Lang::Ru, "dried") => "сушёный",
        (language::Lang::Ru, "pickled") => "маринованный",
        (_, "raw") => "raw",
        (_, "boiled") => "boiled",
        (_, "steamed") => "steamed",
        (_, "baked") => "baked",
        (_, "grilled") => "grilled",
        (_, "fried") => "fried",
        (_, "smoked") => "smoked",
        (_, "frozen") => "frozen",
        (_, "dried") => "dried",
        (_, "pickled") => "pickled",
        _ => "processing state",
    }
}

pub async fn home(headers: HeaderMap, Query(q): Query<LangQuery>) -> Html<String> {
    let lang = resolve_lang(&headers, q.lang.as_deref());
    let titles = lang.pack().titles;
    Html(shell(
        lang,
        seo(lang, titles.start, "home", "/"),
        &pages::home::render(lang),
    ))
}

pub async fn menu(headers: HeaderMap, Query(q): Query<MenuQuery>) -> Html<String> {
    let lang = resolve_lang(&headers, q.lang.as_deref());
    let titles = lang.pack().titles;
    let cat = q.cat.as_deref();
    let title = cat.unwrap_or(titles.menu);
    Html(shell(
        lang,
        seo(lang, title, "menu", "/menu"),
        &pages::menu::render(lang, cat),
    ))
}

pub async fn recipes_list(headers: HeaderMap, Query(q): Query<LangQuery>) -> Html<String> {
    let lang = resolve_lang(&headers, q.lang.as_deref());
    let titles = lang.pack().titles;
    Html(shell(
        lang,
        seo(lang, titles.chef_blog, "recipes", "/recipes"),
        &pages::recipes::list(lang),
    ))
}

pub async fn delivery(headers: HeaderMap, Query(q): Query<LangQuery>) -> Html<String> {
    let lang = resolve_lang(&headers, q.lang.as_deref());
    let titles = lang.pack().titles;
    Html(shell(
        lang,
        seo(lang, titles.delivery, "delivery", "/delivery"),
        &pages::delivery::render(lang),
    ))
}

pub async fn booking(headers: HeaderMap, Query(q): Query<LangQuery>) -> Html<String> {
    let lang = resolve_lang(&headers, q.lang.as_deref());
    let titles = lang.pack().titles;
    Html(shell(
        lang,
        seo(lang, titles.booking, "booking", "/booking"),
        &pages::booking::render(lang),
    ))
}

pub async fn recipe_detail(
    axum::extract::Path(id): axum::extract::Path<String>,
    headers: HeaderMap,
    Query(q): Query<LangQuery>,
) -> Html<String> {
    let lang = resolve_lang(&headers, q.lang.as_deref());
    let title = slug_title(&id);
    let path = format!("/recipes/{id}");
    Html(shell(
        lang,
        seo(lang, &title, "recipes", &path),
        &pages::recipes::detail(lang, &id),
    ))
}

pub async fn about(headers: HeaderMap, Query(q): Query<LangQuery>) -> Html<String> {
    let lang = resolve_lang(&headers, q.lang.as_deref());
    let titles = lang.pack().titles;
    Html(shell(
        lang,
        seo(lang, titles.about, "about", "/about"),
        &pages::about::render(lang),
    ))
}

pub async fn blog_list(headers: HeaderMap, Query(q): Query<LangQuery>) -> Html<String> {
    let lang = resolve_lang(&headers, q.lang.as_deref());
    let title = match lang {
        language::Lang::Pl => "Blog szefa",
        language::Lang::Ru => "Блог шефа",
        language::Lang::En => "Chef journal",
    };
    Html(shell(
        lang,
        seo(lang, title, "blog", "/blog"),
        &pages::blog::list(lang),
    ))
}

pub async fn blog_detail(
    axum::extract::Path(slug): axum::extract::Path<String>,
    headers: HeaderMap,
    Query(q): Query<LangQuery>,
) -> Html<String> {
    let lang = resolve_lang(&headers, q.lang.as_deref());
    let title = pages::blog::title(lang, &slug).unwrap_or("Blog");
    let path = format!("/blog/{slug}");
    let body = pages::blog::detail(lang, &slug).unwrap_or_else(|| pages::blog::list(lang));
    Html(shell(lang, seo(lang, title, "blog", &path), &body))
}

pub async fn ingredients(headers: HeaderMap, Query(q): Query<LangQuery>) -> Html<String> {
    let lang = resolve_lang(&headers, q.lang.as_deref());
    let titles = lang.pack().titles;
    Html(shell(
        lang,
        seo(
            lang,
            titles.ingredients,
            "ingredients",
            "/ingredient-catalog",
        ),
        &pages::ingredients::render(lang),
    ))
}

pub async fn ingredient_detail(
    axum::extract::Path(slug): axum::extract::Path<String>,
    headers: HeaderMap,
    Query(q): Query<LangQuery>,
) -> Html<String> {
    let lang = resolve_lang(&headers, q.lang.as_deref());
    let title = slug_title(&slug);
    let path = format!("/ingredient-catalog/{slug}");
    Html(shell(
        lang,
        seo(lang, &title, "ingredients", &path),
        &pages::ingredient_detail::render(lang, &slug, None),
    ))
}

pub async fn ingredient_state_detail(
    axum::extract::Path((slug, state)): axum::extract::Path<(String, String)>,
    headers: HeaderMap,
    Query(q): Query<LangQuery>,
) -> Html<String> {
    let lang = resolve_lang(&headers, q.lang.as_deref());
    let title = format!("{} - {}", slug_title(&slug), state_title(lang, &state));
    let path = format!("/ingredient-catalog/{slug}/{state}");
    Html(shell(
        lang,
        seo(lang, &title, "ingredients", &path),
        &pages::ingredient_detail::render(lang, &slug, Some(&state)),
    ))
}

pub async fn cookie(headers: HeaderMap, Query(q): Query<LangQuery>) -> Html<String> {
    let lang = resolve_lang(&headers, q.lang.as_deref());
    let titles = lang.pack().titles;
    Html(shell(
        lang,
        seo(lang, titles.cookie, "legal", "/cookie"),
        &pages::cookie::render(lang),
    ))
}

pub async fn privacy(headers: HeaderMap, Query(q): Query<LangQuery>) -> Html<String> {
    let lang = resolve_lang(&headers, q.lang.as_deref());
    let titles = lang.pack().titles;
    Html(shell(
        lang,
        seo(lang, titles.privacy, "legal", "/privacy"),
        &pages::legal::privacy(lang),
    ))
}

pub async fn terms(headers: HeaderMap, Query(q): Query<LangQuery>) -> Html<String> {
    let lang = resolve_lang(&headers, q.lang.as_deref());
    let titles = lang.pack().titles;
    Html(shell(
        lang,
        seo(lang, titles.terms, "legal", "/terms"),
        &pages::legal::terms(lang),
    ))
}

pub async fn not_found(headers: HeaderMap) -> Html<String> {
    let lang = resolve_lang(&headers, None);
    let titles = lang.pack().titles;
    let page_text = pages::i18n::pack(lang);
    Html(shell(
        lang,
        Seo {
            title: titles.not_found,
            description: description(lang, "not_found"),
            path: "/404",
            index: false,
        },
        page_text.not_found_html,
    ))
}
