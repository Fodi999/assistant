//! Axum handlers render pages as HTML responses.

use axum::{
    extract::Query,
    http::{header, HeaderMap},
    response::Html,
};
use serde::Deserialize;

use crate::web::{language, pages, shell};

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

pub async fn home(headers: HeaderMap, Query(q): Query<LangQuery>) -> Html<String> {
    let lang = resolve_lang(&headers, q.lang.as_deref());
    let titles = lang.pack().titles;
    Html(shell(lang, titles.start, &pages::home::render(lang)))
}

pub async fn menu(headers: HeaderMap, Query(q): Query<MenuQuery>) -> Html<String> {
    let lang = resolve_lang(&headers, q.lang.as_deref());
    let titles = lang.pack().titles;
    let cat = q.cat.as_deref();
    let title = cat.unwrap_or(titles.menu);
    Html(shell(lang, title, &pages::menu::render(lang, cat)))
}

pub async fn recipes_list(headers: HeaderMap, Query(q): Query<LangQuery>) -> Html<String> {
    let lang = resolve_lang(&headers, q.lang.as_deref());
    let titles = lang.pack().titles;
    Html(shell(lang, titles.chef_blog, &pages::recipes::list(lang)))
}

pub async fn delivery(headers: HeaderMap, Query(q): Query<LangQuery>) -> Html<String> {
    let lang = resolve_lang(&headers, q.lang.as_deref());
    let titles = lang.pack().titles;
    Html(shell(lang, titles.delivery, &pages::delivery::render(lang)))
}

pub async fn booking(headers: HeaderMap, Query(q): Query<LangQuery>) -> Html<String> {
    let lang = resolve_lang(&headers, q.lang.as_deref());
    let titles = lang.pack().titles;
    Html(shell(lang, titles.booking, &pages::booking::render(lang)))
}

pub async fn recipe_detail(
    axum::extract::Path(id): axum::extract::Path<String>,
    headers: HeaderMap,
    Query(q): Query<LangQuery>,
) -> Html<String> {
    let lang = resolve_lang(&headers, q.lang.as_deref());
    let titles = lang.pack().titles;
    Html(shell(
        lang,
        titles.recipe_detail,
        &pages::recipes::detail(lang, &id),
    ))
}

pub async fn about(headers: HeaderMap, Query(q): Query<LangQuery>) -> Html<String> {
    let lang = resolve_lang(&headers, q.lang.as_deref());
    let titles = lang.pack().titles;
    Html(shell(lang, titles.about, &pages::about::render(lang)))
}

pub async fn cookie(headers: HeaderMap, Query(q): Query<LangQuery>) -> Html<String> {
    let lang = resolve_lang(&headers, q.lang.as_deref());
    let titles = lang.pack().titles;
    Html(shell(lang, titles.cookie, &pages::cookie::render(lang)))
}

pub async fn privacy(headers: HeaderMap, Query(q): Query<LangQuery>) -> Html<String> {
    let lang = resolve_lang(&headers, q.lang.as_deref());
    let titles = lang.pack().titles;
    Html(shell(lang, titles.privacy, &pages::legal::privacy(lang)))
}

pub async fn terms(headers: HeaderMap, Query(q): Query<LangQuery>) -> Html<String> {
    let lang = resolve_lang(&headers, q.lang.as_deref());
    let titles = lang.pack().titles;
    Html(shell(lang, titles.terms, &pages::legal::terms(lang)))
}

pub async fn not_found(headers: HeaderMap) -> Html<String> {
    let lang = resolve_lang(&headers, None);
    let titles = lang.pack().titles;
    let page_text = pages::i18n::pack(lang);
    Html(shell(lang, titles.not_found, page_text.not_found_html))
}
