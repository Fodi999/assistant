function renderLucideIcons() {
  if (window.lucide) {
    window.lucide.createIcons({
      attrs: {
        'stroke-width': 2,
        'aria-hidden': 'true',
      },
    });
  }
}

const I18N = window.CHEF_I18N || {};
const LANG_KEY = I18N.langCookie || 'chef-lang';

renderLucideIcons();

// Language switcher
function setLanguagePreference(lang) {
  localStorage.setItem(LANG_KEY, lang);
  document.cookie = `${LANG_KEY}=${lang}; path=/; max-age=31536000; SameSite=Lax`;
}

document.querySelectorAll('[data-lang-option]').forEach(link => {
  const lang = link.dataset.langOption;
  const url = new URL(window.location.href);
  url.searchParams.set('lang', lang);
  link.href = `${url.pathname}${url.search}${url.hash}`;

  link.addEventListener('click', event => {
    event.preventDefault();
    setLanguagePreference(lang);
    window.location.assign(link.href);
  });
});

if (I18N.lang) {
  setLanguagePreference(I18N.lang);
}

// Cookie consent
const cookieBanner = document.getElementById('cookieBanner');
const COOKIE_KEY = 'cookie-consent';

function setCookieConsent(value) {
  localStorage.setItem(COOKIE_KEY, value);
  document.cookie = `${COOKIE_KEY}=${value}; path=/; max-age=31536000; SameSite=Lax`;
  if (cookieBanner) cookieBanner.classList.remove('visible');
}

function showCookieBanner() {
  if (cookieBanner) cookieBanner.classList.add('visible');
}

if (!localStorage.getItem(COOKIE_KEY)) {
  showCookieBanner();
}

document.querySelectorAll('[data-cookie-choice]').forEach(btn => {
  btn.addEventListener('click', () => setCookieConsent(btn.dataset.cookieChoice));
});

document.querySelectorAll('.cookie-manage').forEach(btn => {
  btn.addEventListener('click', showCookieBanner);
});

// Mobile nav
const burger   = document.getElementById('navToggle');
const navLinks = document.getElementById('navLinks');
const burgerIcon = document.getElementById('navToggleIcon');
if (burger) {
  burger.addEventListener('click', () => {
    const open = navLinks.classList.toggle('open');
    burgerIcon.className = open ? 'bi bi-x-lg' : 'bi bi-list';
  });
  document.addEventListener('click', e => {
    if (!burger.contains(e.target) && !navLinks.contains(e.target))
      navLinks.classList.remove('open');
  });
}

// Active nav link
const path = location.pathname;
document.querySelectorAll('.nav-link').forEach(a => {
  a.classList.remove('active');
  if (a.getAttribute('href') === path) a.classList.add('active');
});

// Client-side menu filtering
const tabBtns   = document.querySelectorAll('.tab-btn[data-cat]');
const menuCards = document.querySelectorAll('.menu-card[data-cat]');
tabBtns.forEach(btn => {
  btn.addEventListener('click', e => {
    e.preventDefault();
    const cat = btn.dataset.cat;
    tabBtns.forEach(b => b.classList.remove('active'));
    btn.classList.add('active');
    let idx = 0;
    menuCards.forEach(card => {
      const show = cat === 'all' || card.dataset.cat === cat;
      card.classList.toggle('hidden', !show);
      if (show) { card.style.animationDelay = (idx++ * 0.05) + 's'; }
    });
    const url = new URL(window.location.href);
    if (cat === 'all') {
      url.searchParams.delete('cat');
    } else {
      url.searchParams.set('cat', cat);
    }
    history.replaceState(null, '', `${url.pathname}${url.search}`);
  });
});

// Lightweight order draft
document.querySelectorAll('.order-btn').forEach(btn => {
  btn.addEventListener('click', () => {
    const item = {
      dish: btn.dataset.dish,
      price: btn.dataset.price,
      addedAt: new Date().toISOString(),
    };
    const current = JSON.parse(localStorage.getItem('chefOrderDraft') || '[]');
    current.push(item);
    localStorage.setItem('chefOrderDraft', JSON.stringify(current));

    const original = btn.innerHTML;
    btn.innerHTML = `<i data-lucide="check"></i> ${I18N.orderAdded || ''}`;
    renderLucideIcons();
    btn.disabled = true;
    setTimeout(() => {
      btn.innerHTML = original;
      renderLucideIcons();
      btn.disabled = false;
    }, 1200);
  });
});

// Scroll reveal
const revealObs = new IntersectionObserver(entries => {
  entries.forEach(e => { if (e.isIntersecting) e.target.classList.add('visible'); });
}, { threshold: 0.08 });
document.querySelectorAll('.reveal').forEach(el => revealObs.observe(el));

// Animated counters
function animateCounter(el) {
  const target = parseFloat(el.dataset.target || '0');
  const suffix = el.dataset.suffix || '';
  const dur = 1400, start = performance.now();
  (function tick(now) {
    const t = Math.min((now - start) / dur, 1);
    const ease = 1 - Math.pow(1 - t, 3);
    el.textContent = Math.floor(ease * target) + suffix;
    if (t < 1) requestAnimationFrame(tick);
  })(start);
}
const cntObs = new IntersectionObserver(entries => {
  entries.forEach(e => {
    if (e.isIntersecting) { animateCounter(e.target); cntObs.unobserve(e.target); }
  });
}, { threshold: 0.5 });
document.querySelectorAll('.counter').forEach(el => cntObs.observe(el));

// Navbar shadow on scroll
const navbar = document.getElementById('navbar');
if (navbar) {
  window.addEventListener('scroll', () => {
    navbar.classList.toggle('scrolled', window.scrollY > 20);
  }, { passive: true });
}
