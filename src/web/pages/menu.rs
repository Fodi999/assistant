use crate::web::{language, pages::i18n};

pub fn render(lang: language::Lang, cat: Option<&str>) -> String {
    let text = i18n::pack(lang);
    let current = cat.unwrap_or("all");
    let mut out = String::new();

    out.push_str(&format!(
        r#"<section class="page-header"><h1>{}</h1><p class="page-header-sub">{}</p></section>"#,
        text.menu_title, text.menu_subtitle
    ));
    out.push_str(r#"<div class="tabs-nav">"#);
    out.push_str(&format!(
        r#"<button class="tab-btn{}" data-cat="all"><i class="bi bi-grid"></i> {}</button>"#,
        if current == "all" { " active" } else { "" },
        text.menu_all
    ));
    for category in text.menu_categories {
        out.push_str(&format!(
            r#"<button class="tab-btn{}" data-cat="{}"><i class="bi {}"></i> {}</button>"#,
            if current == category.id {
                " active"
            } else {
                ""
            },
            category.id,
            category.icon,
            category.label
        ));
    }
    out.push_str("</div>");
    out.push_str(r#"<section class="menu-grid">"#);
    for item in text.menu_items {
        let hidden = if current == "all" || current == item.category {
            ""
        } else {
            " hidden"
        };
        let badge_html = if item.badge.is_empty() {
            String::new()
        } else {
            format!(
                r#"<span class="menu-card-badge"><i class="bi bi-star"></i> {}</span>"#,
                item.badge
            )
        };
        out.push_str(&format!(
            r#"<div class="menu-card{} reveal" data-cat="{}">
  <div class="menu-card-placeholder"><i class="bi {}"></i></div>
  <div class="menu-card-body">
    <div class="menu-card-header">
      <span class="menu-card-title">{}</span>
      <span class="menu-card-price">{}</span>
    </div>
    <p class="menu-card-desc">{}</p>
    <div class="menu-card-footer">
      <span class="recipe-meta-item"><i class="bi bi-speedometer2"></i> {}</span>
      {}
    </div>
    <button class="btn btn-primary btn-full mt-sm order-btn" type="button" data-dish="{}" data-price="{}"><i class="bi bi-cart-plus"></i> {}</button>
  </div>
</div>"#,
            hidden,
            item.category,
            item.icon,
            item.name,
            item.price,
            item.desc,
            item.weight,
            badge_html,
            item.name,
            item.price,
            text.menu_order
        ));
    }
    out.push_str("</section>");
    out
}
