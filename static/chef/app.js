
// Mobile nav
const burger   = document.getElementById('navBurger');
const navLinks = document.getElementById('navLinks');
const burgerIcon = document.getElementById('burgerIcon');
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
    history.replaceState(null, '', cat === 'all' ? '/menu' : '/menu?cat=' + cat);
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
