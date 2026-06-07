use crate::web::{
    language,
    pages::{blog, i18n},
};

pub fn render(lang: language::Lang) -> String {
    let mut page = i18n::pack(lang).about_html.to_string();
    let section = blog::about_section(lang);
    if let Some(index) = page.rfind("</div>") {
        page.insert_str(index, &section);
    } else {
        page.push_str(&section);
    }
    page
}
