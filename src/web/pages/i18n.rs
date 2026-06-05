use crate::web::language::{self, en, pl, ru};

pub struct PagePack {
    pub home_html: &'static str,
    pub delivery_html: &'static str,
    pub booking_html: &'static str,
    pub about_html: &'static str,
    pub privacy_html: &'static str,
    pub terms_html: &'static str,
    pub cookie_html: &'static str,
    pub not_found_html: &'static str,
    pub menu_title: &'static str,
    pub menu_subtitle: &'static str,
    pub menu_all: &'static str,
    pub menu_order: &'static str,
    pub menu_cart_title: &'static str,
    pub menu_cart_empty: &'static str,
    pub menu_cart_subtotal: &'static str,
    pub menu_cart_total: &'static str,
    pub menu_cart_clear: &'static str,
    pub menu_cart_checkout: &'static str,
    pub menu_cart_hint: &'static str,
    pub menu_categories: &'static [pl::pages::MenuCategory],
    pub menu_items: &'static [pl::pages::MenuItem],
    pub recipes_title: &'static str,
    pub recipes_subtitle: &'static str,
    pub recipe_not_found: &'static str,
    pub recipe_back: &'static str,
    pub recipe_all: &'static str,
    pub recipe_ingredients: &'static str,
    pub recipe_preparation: &'static str,
    pub recipes: &'static [pl::pages::Recipe],
}

macro_rules! page_pack {
    ($module:ident) => {
        PagePack {
            home_html: $module::pages::HOME_HTML,
            delivery_html: $module::pages::DELIVERY_HTML,
            booking_html: $module::pages::BOOKING_HTML,
            about_html: $module::pages::ABOUT_HTML,
            privacy_html: $module::pages::PRIVACY_HTML,
            terms_html: $module::pages::TERMS_HTML,
            cookie_html: $module::pages::COOKIE_HTML,
            not_found_html: $module::pages::NOT_FOUND_HTML,
            menu_title: $module::pages::MENU_TITLE,
            menu_subtitle: $module::pages::MENU_SUBTITLE,
            menu_all: $module::pages::MENU_ALL,
            menu_order: $module::pages::MENU_ORDER,
            menu_cart_title: $module::pages::MENU_CART_TITLE,
            menu_cart_empty: $module::pages::MENU_CART_EMPTY,
            menu_cart_subtotal: $module::pages::MENU_CART_SUBTOTAL,
            menu_cart_total: $module::pages::MENU_CART_TOTAL,
            menu_cart_clear: $module::pages::MENU_CART_CLEAR,
            menu_cart_checkout: $module::pages::MENU_CART_CHECKOUT,
            menu_cart_hint: $module::pages::MENU_CART_HINT,
            menu_categories: $module::pages::MENU_CATEGORIES,
            menu_items: $module::pages::MENU_ITEMS,
            recipes_title: $module::pages::RECIPES_TITLE,
            recipes_subtitle: $module::pages::RECIPES_SUBTITLE,
            recipe_not_found: $module::pages::RECIPE_NOT_FOUND,
            recipe_back: $module::pages::RECIPE_BACK,
            recipe_all: $module::pages::RECIPE_ALL,
            recipe_ingredients: $module::pages::RECIPE_INGREDIENTS,
            recipe_preparation: $module::pages::RECIPE_PREPARATION,
            recipes: $module::pages::RECIPES,
        }
    };
}

pub fn pack(lang: language::Lang) -> PagePack {
    match lang {
        language::Lang::Pl => page_pack!(pl),
        language::Lang::Ru => page_pack!(ru),
        language::Lang::En => page_pack!(en),
    }
}
