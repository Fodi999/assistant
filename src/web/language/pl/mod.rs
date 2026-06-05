pub mod pages;

use super::{JsText, PageTitles, ShellText};

pub const SHELL: ShellText = ShellText {
    html_lang: "pl",
    brand_plain: "Dima",
    brand_accent: "Chef",
    nav_start: "Start",
    nav_menu: "Menu",
    nav_delivery: "Dostawa",
    nav_booking: "Rezerwacja",
    nav_about: "O szefie",
    nav_table: "Stolik",
    nav_cart: "Koszyk",
    nav_order: "Zamów",
    nav_language: "Język",
    aria_menu: "Menu",
    cookie_aria: "Zgoda na pliki cookie",
    cookie_title: "Pliki cookie",
    cookie_intro: "Używamy niezbędnych plików cookie oraz, za Twoją zgodą, analitycznych plików cookie do poprawy działania strony.",
    cookie_necessary: "Tylko niezbędne",
    cookie_accept: "Akceptuję",
    trust_delivery: "Dostawa 45-60 min",
    trust_pickup: "Odbiór bez czekania",
    trust_booking: "Rezerwacja stolików",
    trust_author: "Autorska kuchnia szefa",
    footer_tagline: "Autorska kuchnia Dimy Fomina: dostawa, odbiór osobisty i spokojne kolacje przy stoliku.",
    footer_guests: "Dla gości",
    footer_restaurant: "Restauracja",
    footer_contact: "Kontakt",
    footer_menu: "Menu",
    footer_delivery: "Dostawa",
    footer_booking: "Rezerwacja stolika",
    footer_blog: "Blog szefa",
    footer_about: "O szefie",
    footer_haccp: "Standard HACCP",
    footer_privacy: "Prywatność",
    footer_terms: "Regulamin",
    footer_cookie: "Polityka cookie",
    footer_manage_cookie: "Zarządzaj cookie",
    footer_copy: "© 2026 Dima Fomin. Wszelkie prawa zastrzeżone.",
};

pub const TITLES: PageTitles = PageTitles {
    start: "Start",
    menu: "Menu",
    chef_blog: "Blog szefa",
    delivery: "Dostawa",
    booking: "Rezerwacja",
    recipe_detail: "Wpis szefa",
    about: "O szefie",
    ingredients: "Katalog składników",
    cookie: "Polityka cookie",
    privacy: "Polityka prywatności",
    terms: "Regulamin",
    not_found: "404",
};

pub const JS: JsText = JsText {
    order_added: "Dodano",
    cart_remove: "Usuń",
};
