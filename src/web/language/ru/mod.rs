pub mod pages;

use super::{JsText, PageTitles, ShellText};

pub const SHELL: ShellText = ShellText {
    html_lang: "ru",
    brand_plain: "Dima",
    brand_accent: "Chef",
    nav_start: "Главная",
    nav_menu: "Меню",
    nav_delivery: "Доставка",
    nav_booking: "Бронирование",
    nav_about: "О шефе",
    nav_table: "Столик",
    nav_cart: "Корзина",
    nav_order: "Заказать",
    nav_language: "Язык",
    aria_menu: "Меню",
    cookie_aria: "Согласие на cookie",
    cookie_title: "Файлы cookie",
    cookie_intro: "Мы используем необходимые cookie и, с вашего согласия, аналитические cookie для улучшения работы сайта.",
    cookie_necessary: "Только необходимые",
    cookie_accept: "Принимаю",
    trust_delivery: "Доставка 45-60 мин",
    trust_pickup: "Самовывоз без ожидания",
    trust_booking: "Бронирование столиков",
    trust_author: "Авторская кухня шефа",
    footer_tagline: "Авторская кухня Димы Фомина: доставка, самовывоз и спокойные ужины за столиком.",
    footer_guests: "Гостям",
    footer_restaurant: "Ресторан",
    footer_contact: "Контакт",
    footer_menu: "Меню",
    footer_delivery: "Доставка",
    footer_booking: "Бронирование столика",
    footer_blog: "Блог шефа",
    footer_about: "О шефе",
    footer_haccp: "Стандарт HACCP",
    footer_privacy: "Конфиденциальность",
    footer_terms: "Правила",
    footer_cookie: "Политика cookie",
    footer_manage_cookie: "Управлять cookie",
    footer_copy: "© 2026 Dima Fomin. Все права защищены.",
};

pub const TITLES: PageTitles = PageTitles {
    start: "Главная",
    menu: "Меню",
    chef_blog: "Блог шефа",
    delivery: "Доставка",
    booking: "Бронирование",
    recipe_detail: "Запись шефа",
    about: "О шефе",
    ingredients: "Каталог ингредиентов",
    cookie: "Политика cookie",
    privacy: "Политика конфиденциальности",
    terms: "Правила",
    not_found: "404",
};

pub const JS: JsText = JsText {
    order_added: "Добавлено",
    cart_remove: "Удалить",
};
