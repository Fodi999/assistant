use crate::web::{language, pages::i18n};

pub fn render(lang: language::Lang, cat: Option<&str>) -> String {
    let text = i18n::pack(lang);
    let current = cat.unwrap_or("all");
    let mut out = String::new();

    out.push_str(r#"<div class="menu-layout">"#);
    out.push_str(r#"<section class="menu-main">"#);
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
        let cart_key = format!("{}|{}|{}", item.name, item.price, item.weight);
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
        <button class="btn btn-primary btn-full mt-sm order-btn" type="button" data-cart-key="{}" data-dish="{}" data-price="{}" data-weight="{}"><i class="bi bi-cart-plus"></i> {}</button>
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
            cart_key,
            item.name,
            item.price,
            item.weight,
            text.menu_order
        ));
    }
    out.push_str("</section>");
    out.push_str("</section>");
    out.push_str(&format!(
                r#"<aside class="cart-panel reveal" id="cart" data-cart-panel>
    <div class="cart-panel-top">
        <div>
            <span class="section-eyebrow"><i class="bi bi-cart3"></i> {}</span>
            <h2 class="cart-title">{}</h2>
        </div>
        <button class="cart-clear" type="button" data-cart-clear>{}</button>
    </div>
    <div class="cart-summary-bar">
        <span class="cart-summary-pill" data-cart-count>0</span>
        <span class="cart-summary-label">{}</span>
    </div>
    <div class="cart-empty-state" data-cart-empty>
        <div class="cart-empty-icon"><i class="bi bi-bag-plus"></i></div>
        <p>{}</p>
    </div>
    <div class="cart-items" data-cart-items></div>
    <div class="cart-footer">
        <div class="cart-total-row">
            <span>{}</span>
            <strong data-cart-subtotal>0 zł</strong>
        </div>
        <div class="cart-total-row cart-total-row-main">
            <span>{}</span>
            <strong data-cart-total>0 zł</strong>
        </div>
        <a href="/booking" class="btn btn-primary btn-full"><i class="bi bi-arrow-right-circle"></i> {}</a>
        <p class="cart-note">{}</p>
    </div>
</aside>"#,
                text.menu_cart_title,
                text.menu_cart_title,
                text.menu_cart_clear,
                text.menu_cart_title,
                text.menu_cart_empty,
                text.menu_cart_subtotal,
                text.menu_cart_total,
                text.menu_cart_checkout,
                text.menu_cart_hint,
        ));
    out.push_str("</div>");
    out
}
