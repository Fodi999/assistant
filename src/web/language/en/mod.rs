pub mod pages;

use super::{JsText, PageTitles, ShellText};

pub const SHELL: ShellText = ShellText {
    html_lang: "en",
    brand_plain: "Dima",
    brand_accent: "Chef",
    nav_start: "Home",
    nav_menu: "Menu",
    nav_delivery: "Delivery",
    nav_booking: "Booking",
    nav_about: "About",
    nav_table: "Table",
    nav_cart: "Cart",
    nav_order: "Order",
    nav_language: "Language",
    aria_menu: "Menu",
    cookie_aria: "Cookie consent",
    cookie_title: "Cookies",
    cookie_intro:
        "We use essential cookies and, with your consent, analytics cookies to improve the website.",
    cookie_necessary: "Essential only",
    cookie_accept: "Accept",
    trust_delivery: "Delivery 45-60 min",
    trust_pickup: "Pickup without waiting",
    trust_booking: "Table booking",
    trust_author: "Chef's signature cuisine",
    footer_tagline:
        "Dima Fomin's signature cuisine: delivery, pickup and calm dinners at the table.",
    footer_guests: "For guests",
    footer_restaurant: "Restaurant",
    footer_contact: "Contact",
    footer_menu: "Menu",
    footer_delivery: "Delivery",
    footer_booking: "Table booking",
    footer_blog: "Chef blog",
    footer_about: "About",
    footer_haccp: "HACCP standard",
    footer_privacy: "Privacy",
    footer_terms: "Terms",
    footer_cookie: "Cookie policy",
    footer_manage_cookie: "Manage cookies",
    footer_copy: "© 2026 Dima Fomin. All rights reserved.",
};

pub const TITLES: PageTitles = PageTitles {
    start: "Home",
    menu: "Menu",
    chef_blog: "Chef blog",
    delivery: "Delivery",
    booking: "Booking",
    recipe_detail: "Chef article",
    about: "About",
    ingredients: "Ingredient catalog",
    cookie: "Cookie policy",
    privacy: "Privacy policy",
    terms: "Terms",
    not_found: "404",
};

pub const JS: JsText = JsText {
    order_added: "Added",
    cart_remove: "Remove",
};
