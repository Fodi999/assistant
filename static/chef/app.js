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

// Menu cart
const cartStorageKey = `chefCart:${I18N.lang || 'pl'}`;
const cartPanel = document.querySelector('[data-cart-panel]');
const cartItemsEl = document.querySelector('[data-cart-items]');
const cartEmptyEl = document.querySelector('[data-cart-empty]');
const cartSubtotalEl = document.querySelector('[data-cart-subtotal]');
const cartTotalEl = document.querySelector('[data-cart-total]');
const cartCountEls = document.querySelectorAll('[data-cart-count]');
const cartClearBtn = document.querySelector('[data-cart-clear]');

function escapeHtml(value) {
  return String(value)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');
}

function parsePrice(value) {
  const match = String(value || '').replace(',', '.').match(/[\d.]+/);
  return match ? Number.parseFloat(match[0]) : 0;
}

function formatMoney(value) {
  return `${Math.round(value)} zł`;
}

function readCart() {
  try {
    const raw = localStorage.getItem(cartStorageKey);
    const parsed = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

function saveCart(cart) {
  localStorage.setItem(cartStorageKey, JSON.stringify(cart));
}

function normalizeCart(cart) {
  return cart.reduce((acc, item) => {
    const key = item.key || `${item.dish || ''}|${item.priceLabel || item.price || ''}|${item.weight || ''}`;
    const quantity = Math.max(1, Number.parseInt(item.quantity, 10) || 1);
    const existing = acc.find(entry => entry.key === key);

    if (existing) {
      existing.quantity += quantity;
    } else {
      acc.push({
        key,
        dish: item.dish || '',
        priceLabel: item.priceLabel || item.price || '0 zł',
        priceValue: Number.isFinite(Number(item.priceValue))
          ? Number(item.priceValue)
          : parsePrice(item.priceLabel || item.price),
        weight: item.weight || '',
        quantity,
      });
    }

    return acc;
  }, []);
}

function migrateOldDraft() {
  if (localStorage.getItem(cartStorageKey)) return;
  try {
    const draft = JSON.parse(localStorage.getItem('chefOrderDraft') || '[]');
    if (!Array.isArray(draft) || draft.length === 0) return;
    const migrated = draft.reduce((acc, item) => {
      const key = `${item.dish || ''}|${item.price || ''}|${item.weight || ''}`;
      const existing = acc.find(entry => entry.key === key);
      if (existing) {
        existing.quantity += 1;
      } else {
        acc.push({
          key,
          dish: item.dish || '',
          priceLabel: item.price || '0 zł',
          priceValue: parsePrice(item.price),
          weight: item.weight || '',
          quantity: 1,
        });
      }
      return acc;
    }, []);
    saveCart(migrated);
    localStorage.removeItem('chefOrderDraft');
  } catch {
    localStorage.removeItem('chefOrderDraft');
  }
}

function updateCartButtonState(btn, enabled) {
  if (!btn) return;
  btn.disabled = !enabled;
}

function renderCart() {
  if (!cartPanel || !cartItemsEl || !cartEmptyEl || !cartSubtotalEl || !cartTotalEl) {
    return;
  }

  const cart = normalizeCart(readCart());
  const count = cart.reduce((sum, item) => sum + item.quantity, 0);
  const subtotal = cart.reduce((sum, item) => sum + (item.priceValue * item.quantity), 0);

  cartCountEls.forEach(el => {
    el.textContent = String(count);
  });
  cartSubtotalEl.textContent = formatMoney(subtotal);
  cartTotalEl.textContent = formatMoney(subtotal);
  cartEmptyEl.classList.toggle('is-hidden', cart.length > 0);
  cartItemsEl.innerHTML = cart.map(item => `
    <article class="cart-item" data-cart-key="${escapeHtml(item.key)}">
      <div class="cart-item-top">
        <div>
          <div class="cart-item-name">${escapeHtml(item.dish)}</div>
          <div class="cart-item-meta">${escapeHtml(item.weight || '')}</div>
        </div>
        <div class="cart-item-price">${formatMoney(item.priceValue * item.quantity)}</div>
      </div>
      <div class="cart-item-controls">
        <div class="cart-stepper">
          <button type="button" data-cart-action="decrement" data-cart-key="${escapeHtml(item.key)}" aria-label="-">−</button>
          <span class="cart-stepper-qty">${item.quantity}</span>
          <button type="button" data-cart-action="increment" data-cart-key="${escapeHtml(item.key)}" aria-label="+">+</button>
        </div>
        <button type="button" class="cart-remove" data-cart-action="remove" data-cart-key="${escapeHtml(item.key)}">${escapeHtml(I18N.cartRemove || 'Remove')}</button>
      </div>
    </article>
  `).join('');
}

function setCart(cart) {
  saveCart(normalizeCart(cart));
  renderCart();
}

function addToCart(button) {
  const key = button.dataset.cartKey || `${button.dataset.dish || ''}|${button.dataset.price || ''}|${button.dataset.weight || ''}`;
  const dish = button.dataset.dish || '';
  const priceLabel = button.dataset.price || '0 zł';
  const priceValue = parsePrice(priceLabel);
  const weight = button.dataset.weight || '';

  const cart = readCart();
  const existing = cart.find(item => item.key === key);
  if (existing) {
    existing.quantity += 1;
  } else {
    cart.push({ key, dish, priceLabel, priceValue, weight, quantity: 1 });
  }
  setCart(cart);

  const original = button.innerHTML;
  button.innerHTML = `<i data-lucide="check"></i> ${I18N.orderAdded || ''}`;
  renderLucideIcons();
  updateCartButtonState(button, false);
  setTimeout(() => {
    button.innerHTML = original;
    renderLucideIcons();
    updateCartButtonState(button, true);
  }, 1200);
}

migrateOldDraft();
renderCart();

document.querySelectorAll('.order-btn').forEach(btn => {
  btn.addEventListener('click', () => addToCart(btn));
});

if (cartClearBtn) {
  cartClearBtn.addEventListener('click', () => {
    saveCart([]);
    renderCart();
  });
}

if (cartItemsEl) {
  cartItemsEl.addEventListener('click', event => {
    const actionButton = event.target.closest('[data-cart-action]');
    if (!actionButton) return;

    const action = actionButton.dataset.cartAction;
    const itemEl = actionButton.closest('.cart-item');
    const key = itemEl?.dataset.cartKey || actionButton.dataset.cartKey;
    const cart = normalizeCart(readCart());
    const index = cart.findIndex(item => item.key === key);
    if (index === -1) return;

    if (action === 'increment') {
      cart[index].quantity += 1;
    } else if (action === 'decrement') {
      cart[index].quantity -= 1;
      if (cart[index].quantity < 1) {
        cart.splice(index, 1);
      }
    } else if (action === 'remove') {
      cart.splice(index, 1);
    }

    setCart(cart);
  });
}

// Public ingredient catalog
const ingredientPage = document.querySelector('[data-ingredient-page]');
const ingredientGrid = document.querySelector('[data-ingredient-grid]');
const ingredientCount = document.querySelector('[data-ingredient-count]');
const ingredientSearch = document.querySelector('[data-ingredient-search]');
const ingredientCategory = document.querySelector('[data-ingredient-category]');

function fieldForLang(base, item) {
  const lang = I18N.lang || 'pl';
  return item[`${base}_${lang}`] || item[`${base}_pl`] || item[`${base}_en`] || item[`${base}_ru`] || item[base] || '';
}

function formatMacro(value, suffix = 'g') {
  if (value === null || value === undefined || Number.isNaN(Number(value))) return '—';
  const number = Number(value);
  const rounded = Number.isInteger(number) ? String(number) : number.toFixed(1).replace(/\.0$/, '');
  return `${rounded}${suffix}`;
}

function ingredientInitial(name) {
  return String(name || '?').trim().charAt(0).toUpperCase() || '?';
}

function renderIngredientCards(items) {
  if (!ingredientGrid || !ingredientPage || !ingredientCount) return;

  const countLabel = ingredientPage.dataset.countLabel || '';
  ingredientCount.textContent = `${items.length} ${countLabel}`.trim();

  if (items.length === 0) {
    ingredientGrid.innerHTML = `<div class="ingredient-state">${escapeHtml(ingredientPage.dataset.empty || '')}</div>`;
    return;
  }

  ingredientGrid.innerHTML = items.map(item => {
    const name = fieldForLang('name', item) || item.slug;
    const category = fieldForLang('category_name', item);
    const image = item.image_url
      ? `<img src="${escapeHtml(item.image_url)}" alt="${escapeHtml(name)}" loading="lazy">`
      : `<span>${escapeHtml(ingredientInitial(name))}</span>`;

    const href = `/ingredient-catalog/${encodeURIComponent(item.slug || '')}`;

    return `
      <a class="ingredient-card-link" href="${href}" aria-label="${escapeHtml(name)}">
        <article class="ingredient-card">
          <div class="ingredient-photo${item.image_url ? ' has-image' : ''}">${image}</div>
          <div class="ingredient-body">
            <h3>${escapeHtml(name)}</h3>
            ${category ? `<p class="ingredient-meta">${escapeHtml(category)}</p>` : ''}
          </div>
        </article>
      </a>
    `;
  }).join('');
}

function setupIngredientCatalog(items) {
  if (!ingredientPage || !ingredientCategory) return;

  const categories = [...new Set(items.map(item => fieldForLang('category_name', item)).filter(Boolean))].sort((a, b) => a.localeCompare(b));
  const allLabel = ingredientPage.dataset.allLabel || '';
  ingredientCategory.innerHTML = `<option value="">${escapeHtml(allLabel)}</option>` + categories
    .map(category => `<option value="${escapeHtml(category)}">${escapeHtml(category)}</option>`)
    .join('');

  function applyFilters() {
    const query = (ingredientSearch?.value || '').trim().toLowerCase();
    const selectedCategory = ingredientCategory.value;
    const filtered = items.filter(item => {
      const name = fieldForLang('name', item).toLowerCase();
      const slug = String(item.slug || '').toLowerCase();
      const category = fieldForLang('category_name', item);
      return (!query || name.includes(query) || slug.includes(query))
        && (!selectedCategory || category === selectedCategory);
    });
    renderIngredientCards(filtered);
  }

  ingredientSearch?.addEventListener('input', applyFilters);
  ingredientCategory.addEventListener('change', applyFilters);
  applyFilters();
}

if (ingredientPage && ingredientGrid) {
  ingredientGrid.innerHTML = `<div class="ingredient-state">${escapeHtml(ingredientPage.dataset.loading || '')}</div>`;
  fetch('/public/ingredients-full')
    .then(response => {
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      return response.json();
    })
    .then(data => setupIngredientCatalog(Array.isArray(data.items) ? data.items : []))
    .catch(() => {
      ingredientGrid.innerHTML = `<div class="ingredient-state">${escapeHtml(ingredientPage.dataset.error || '')}</div>`;
    });
}

const ingredientDetailPage = document.querySelector('[data-ingredient-detail-page]');

function detailLang() {
  return I18N.lang || 'pl';
}

function detailName(item) {
  const lang = detailLang();
  return item?.[`name_${lang}`] || item?.name_pl || item?.name_en || item?.name_ru || item?.slug || '';
}

function detailValue(value, unit = '') {
  if (value === null || value === undefined || Number.isNaN(Number(value))) return '—';
  const num = Number(value);
  const fixed = Number.isInteger(num) ? String(num) : num.toFixed(1).replace(/\.0$/, '');
  return `${fixed}${unit}`;
}

function detailList(entries) {
  if (!Array.isArray(entries) || entries.length === 0) return '<p class="text-muted">—</p>';
  return `<ul class="ingredient-bullet-list">${entries.map(entry => `<li>${escapeHtml(String(entry))}</li>`).join('')}</ul>`;
}

function detailGrid(entries, unit = '') {
  const valid = entries.filter(entry => entry && (entry.value !== null && entry.value !== undefined));
  if (valid.length === 0) return '<p class="text-muted">—</p>';
  return `<div class="ingredient-info-grid">${valid.map(entry => `
    <div>
      <dt>${escapeHtml(entry.label)}</dt>
      <dd>${escapeHtml(detailValue(entry.value, unit))}</dd>
    </div>
  `).join('')}</div>`;
}

function pickLocalizedJson(obj, keyBase) {
  if (!obj) return [];
  const lang = detailLang();
  const value = obj[`${keyBase}_${lang}`] || obj[`${keyBase}_pl`] || obj[`${keyBase}_en`] || obj[`${keyBase}_ru`] || obj[`${keyBase}_uk`];
  if (Array.isArray(value)) return value;
  return [];
}

function formatCalculatorValue(value, suffix = '') {
  if (value === null || value === undefined || Number.isNaN(Number(value))) return '—';
  const number = Number(value);
  const rounded = Number.isInteger(number) ? String(number) : number.toFixed(1).replace(/\.0$/, '');
  return `${rounded}${suffix}`;
}

if (ingredientDetailPage) {
  const slug = ingredientDetailPage.dataset.slug || '';
  const loading = ingredientDetailPage.dataset.loading || '';
  const error = ingredientDetailPage.dataset.error || '';
  const empty = ingredientDetailPage.dataset.empty || '';
  const content = ingredientDetailPage.querySelector('[data-ingredient-detail-content]');
  const template = ingredientDetailPage.querySelector('[data-ingredient-detail-template]');

  if (content) {
    content.innerHTML = `<div class="ingredient-state">${escapeHtml(loading)}</div>`;
  }

  const labels = {
    ru: {
      calories: 'Калории', protein: 'Белки', fat: 'Жиры', carbs: 'Углеводы', fiber: 'Клетчатка', water: 'Вода',
      shelf: 'Срок хранения', temp: 'Температура', texture: 'Текстура', notes: 'Заметки', score: 'Качество данных',
      density: 'Плотность', cup: '1 стакан', tbsp: '1 ст. ложка', tsp: '1 ч. ложка',
      sweetness: 'Сладость', acidity: 'Кислотность', bitterness: 'Горечь', umami: 'Umami', aroma: 'Аромат',
      gi: 'Гликемический индекс', gl: 'Гликемическая нагрузка', ph: 'pH', aw: 'Активность воды', smoke: 'Точка дыма',
      processingMethod: 'Лучший метод готовки',
      vegan: 'Веган', vegetarian: 'Вегетарианский', keto: 'Keto', paleo: 'Paleo', gluten_free: 'Без глютена', mediterranean: 'Средиземноморская', low_carb: 'Low carb'
    },
    en: {
      calories: 'Calories', protein: 'Protein', fat: 'Fat', carbs: 'Carbs', fiber: 'Fiber', water: 'Water',
      shelf: 'Shelf life', temp: 'Temperature', texture: 'Texture', notes: 'Notes', score: 'Data quality',
      density: 'Density', cup: '1 cup', tbsp: '1 tbsp', tsp: '1 tsp',
      sweetness: 'Sweetness', acidity: 'Acidity', bitterness: 'Bitterness', umami: 'Umami', aroma: 'Aroma',
      gi: 'Glycemic index', gl: 'Glycemic load', ph: 'pH', aw: 'Water activity', smoke: 'Smoke point',
      processingMethod: 'Best cooking method',
      vegan: 'Vegan', vegetarian: 'Vegetarian', keto: 'Keto', paleo: 'Paleo', gluten_free: 'Gluten free', mediterranean: 'Mediterranean', low_carb: 'Low carb'
    },
    pl: {
      calories: 'Kalorie', protein: 'Białko', fat: 'Tłuszcz', carbs: 'Węglow.', fiber: 'Błonnik', water: 'Woda',
      shelf: 'Trwałość', temp: 'Temperatura', texture: 'Tekstura', notes: 'Notatki', score: 'Jakość danych',
      density: 'Gęstość', cup: '1 szklanka', tbsp: '1 łyżka', tsp: '1 łyżeczka',
      sweetness: 'Słodycz', acidity: 'Kwasowość', bitterness: 'Gorycz', umami: 'Umami', aroma: 'Aromat',
      gi: 'Indeks glikemiczny', gl: 'Ładunek glikemiczny', ph: 'pH', aw: 'Aktywność wody', smoke: 'Punkt dymienia',
      processingMethod: 'Najlepsza metoda obróbki',
      vegan: 'Wegańskie', vegetarian: 'Wegetariańskie', keto: 'Keto', paleo: 'Paleo', gluten_free: 'Bez glutenu', mediterranean: 'Śródziemnomorskie', low_carb: 'Low carb'
    }
  };

  const t = labels[detailLang()] || labels.pl;

  Promise.allSettled([
    fetch(`/public/nutrition/${encodeURIComponent(slug)}?lang=${encodeURIComponent(detailLang())}`).then(r => r.ok ? r.json() : null),
    fetch(`/public/ingredients/${encodeURIComponent(slug)}?lang=${encodeURIComponent(detailLang())}`).then(r => r.ok ? r.json() : null),
    fetch(`/public/ingredients/${encodeURIComponent(slug)}/states?lang=${encodeURIComponent(detailLang())}`).then(r => r.ok ? r.json() : null)
  ]).then((results) => {
    const nutrition = results[0].status === 'fulfilled' ? results[0].value : null;
    const ingredient = results[1].status === 'fulfilled' ? results[1].value : null;
    const statesPayload = results[2].status === 'fulfilled' ? results[2].value : null;

    if (!nutrition && !ingredient) {
      if (content) content.innerHTML = `<div class="ingredient-state">${escapeHtml(error)}</div>`;
      return;
    }

    if (!template || !content) return;

    content.innerHTML = template.innerHTML;
    const basic = nutrition?.basic || ingredient || {};
    const name = detailName(basic);
    const description = ingredient?.description || basic.description_en || '';
    const states = Array.isArray(statesPayload?.states) ? statesPayload.states : [];
    const rawState = states.find(state => state.state === 'raw') || states[0] || null;

    const titleEl = ingredientDetailPage.querySelector('[data-ingredient-title]');
    const subtitleEl = ingredientDetailPage.querySelector('[data-ingredient-subtitle]');
    if (titleEl) titleEl.textContent = name || titleEl.textContent;
    if (subtitleEl && description) subtitleEl.textContent = description;

    const imageHolder = content.querySelector('[data-detail-image]');
    if (imageHolder) {
      const imageUrl = basic.image_url || ingredient?.image_url;
      imageHolder.innerHTML = imageUrl
        ? `<img src="${escapeHtml(imageUrl)}" alt="${escapeHtml(name)}" loading="lazy">`
        : `<span>${escapeHtml((name || '?').charAt(0).toUpperCase() || '?')}</span>`;
      imageHolder.classList.toggle('has-image', Boolean(imageUrl));
    }

    const nameEl = content.querySelector('[data-detail-name]');
    if (nameEl) nameEl.textContent = name;
    const descEl = content.querySelector('[data-detail-description]');
    if (descEl) descEl.textContent = description || empty;

    const flagEl = content.querySelector('[data-detail-flags]');
    if (flagEl) {
      const flags = nutrition?.diet_flags || {};
      const activeFlags = Object.keys(flags).filter(key => flags[key] === true);
      flagEl.innerHTML = activeFlags.length
        ? activeFlags.map(key => `<span class="ingredient-flag">✓ ${escapeHtml(t[key] || key)}</span>`).join('')
        : '<span class="text-muted">—</span>';
    }

    const seasonEl = content.querySelector('[data-detail-seasons]');
    if (seasonEl) {
      seasonEl.textContent = Array.isArray(ingredient?.localized_seasons) && ingredient.localized_seasons.length
        ? ingredient.localized_seasons.join(', ')
        : '—';
    }

    const statesEl = content.querySelector('[data-detail-states]');
    const stateCardEl = content.querySelector('[data-detail-state-card]');

    function renderStateCard(state) {
      if (!stateCardEl) return;
      if (!state) {
        stateCardEl.innerHTML = '<p class="text-muted">—</p>';
        return;
      }

      stateCardEl.innerHTML = `
        <div class="ingredient-state-summary">
          <strong>${escapeHtml(String(state.state || 'raw'))}</strong>
          <span>${escapeHtml(t.shelf)}: ${escapeHtml(detailValue((state.shelf_life_hours || 0) / 24, ' дн'))}</span>
          <span>${escapeHtml(t.temp)}: ${escapeHtml(detailValue(state.storage_temp_c, '°C'))}</span>
          <span>${escapeHtml(t.texture)}: ${escapeHtml(String(state.texture || '—'))}</span>
          <span>${escapeHtml(t.score)}: ${escapeHtml(detailValue(state.data_score, '%'))}</span>
        </div>
        <div class="ingredient-info-grid">
          <div><dt>${escapeHtml(t.calories)}</dt><dd>${escapeHtml(detailValue(state.calories_per_100g))} kcal</dd></div>
          <div><dt>${escapeHtml(t.protein)}</dt><dd>${escapeHtml(detailValue(state.protein_per_100g, ' g'))}</dd></div>
          <div><dt>${escapeHtml(t.fat)}</dt><dd>${escapeHtml(detailValue(state.fat_per_100g, ' g'))}</dd></div>
          <div><dt>${escapeHtml(t.carbs)}</dt><dd>${escapeHtml(detailValue(state.carbs_per_100g, ' g'))}</dd></div>
          <div><dt>${escapeHtml(t.fiber)}</dt><dd>${escapeHtml(detailValue(state.fiber_per_100g, ' g'))}</dd></div>
          <div><dt>${escapeHtml(t.water)}</dt><dd>${escapeHtml(detailValue(state.water_percent, ' %'))}</dd></div>
        </div>
        ${state.notes_ru || state.notes_en || state.notes_pl || state.notes_uk ? `<p>${escapeHtml(state.notes_ru || state.notes_en || state.notes_pl || state.notes_uk)}</p>` : ''}
      `;
    }

    if (statesEl) {
      statesEl.innerHTML = states.length
        ? states.map((state, index) => `<button class="ingredient-chip${rawState === state ? ' active' : ''}" type="button" data-state-index="${index}">${escapeHtml(String(state.state || 'raw'))}</button>`).join('')
        : '<span class="text-muted">—</span>';
      statesEl.addEventListener('click', (event) => {
        const button = event.target.closest('[data-state-index]');
        if (!button) return;
        const index = Number(button.dataset.stateIndex);
        const state = states[index];
        statesEl.querySelectorAll('.ingredient-chip').forEach(chip => chip.classList.remove('active'));
        button.classList.add('active');
        renderStateCard(state);
      });
    }

    renderStateCard(rawState);

    const macrosEl = content.querySelector('[data-detail-macros]');
    if (macrosEl) {
      const source = rawState || nutrition?.macros || ingredient?.nutrition || {};
      macrosEl.innerHTML = detailGrid([
        { label: t.calories, value: source.calories_per_100g || source.calories_kcal },
        { label: t.protein, value: source.protein_per_100g || source.protein_g },
        { label: t.fat, value: source.fat_per_100g || source.fat_g },
        { label: t.carbs, value: source.carbs_per_100g || source.carbs_g },
        { label: t.fiber, value: source.fiber_per_100g || source.fiber_g },
        { label: t.water, value: source.water_percent || source.water_g }
      ]);
    }

    const mineralsEl = content.querySelector('[data-detail-minerals]');
    if (mineralsEl) {
      const m = nutrition?.minerals || {};
      mineralsEl.innerHTML = detailGrid([
        { label: 'Calcium', value: m.calcium },
        { label: 'Iron', value: m.iron },
        { label: 'Magnesium', value: m.magnesium },
        { label: 'Phosphorus', value: m.phosphorus },
        { label: 'Potassium', value: m.potassium },
        { label: 'Sodium', value: m.sodium },
        { label: 'Zinc', value: m.zinc }
      ], ' mg');
    }

    const vitaminsEl = content.querySelector('[data-detail-vitamins]');
    if (vitaminsEl) {
      const v = nutrition?.vitamins || {};
      vitaminsEl.innerHTML = detailGrid([
        { label: 'Vitamin A', value: v.vitamin_a },
        { label: 'Vitamin C', value: v.vitamin_c },
        { label: 'Vitamin D', value: v.vitamin_d },
        { label: 'Vitamin E', value: v.vitamin_e },
        { label: 'Vitamin K', value: v.vitamin_k },
        { label: 'Vitamin B1', value: v.vitamin_b1 },
        { label: 'Vitamin B2', value: v.vitamin_b2 },
        { label: 'Vitamin B3', value: v.vitamin_b3 },
        { label: 'Vitamin B6', value: v.vitamin_b6 },
        { label: 'Vitamin B9', value: v.vitamin_b9 },
        { label: 'Vitamin B12', value: v.vitamin_b12 }
      ]);
    }

    const culinaryEl = content.querySelector('[data-detail-culinary]');
    if (culinaryEl) {
      const c = nutrition?.culinary || {};
      culinaryEl.innerHTML = detailGrid([
        { label: t.sweetness, value: c.sweetness },
        { label: t.acidity, value: c.acidity },
        { label: t.bitterness, value: c.bitterness },
        { label: t.umami, value: c.umami },
        { label: t.aroma, value: c.aroma }
      ]) + (c.texture ? `<p>${escapeHtml(c.texture)}</p>` : '');
    }

    const propsEl = content.querySelector('[data-detail-properties]');
    if (propsEl) {
      const p = nutrition?.food_properties || {};
      propsEl.innerHTML = detailGrid([
        { label: t.gi, value: p.glycemic_index },
        { label: t.gl, value: p.glycemic_load },
        { label: t.ph, value: p.ph },
        { label: t.aw, value: p.water_activity },
        { label: t.smoke, value: p.smoke_point }
      ]);
    }

    const healthEl = content.querySelector('[data-detail-health]');
    if (healthEl) {
      const hp = nutrition?.health_profile || {};
      const compounds = pickLocalizedJson(hp, 'bioactive_compounds');
      const effects = pickLocalizedJson(hp, 'health_effects');
      const contraindications = pickLocalizedJson(hp, 'contraindications');
      healthEl.innerHTML = `
        ${hp.food_role ? `<p><strong>${escapeHtml(hp.food_role)}</strong></p>` : ''}
        ${hp.orac_score ? `<p>ORAC: ${escapeHtml(detailValue(hp.orac_score))}</p>` : ''}
        ${detailList(compounds)}
        ${detailList(effects)}
        ${detailList(contraindications)}
        ${hp[`absorption_notes_${detailLang()}`] || hp.absorption_notes_en ? `<p>${escapeHtml(hp[`absorption_notes_${detailLang()}`] || hp.absorption_notes_en)}</p>` : ''}
      `;
    }

    const sugarEl = content.querySelector('[data-detail-sugar]');
    if (sugarEl) {
      const s = nutrition?.sugar_profile || {};
      sugarEl.innerHTML = detailGrid([
        { label: 'Glucose', value: s.glucose },
        { label: 'Fructose', value: s.fructose },
        { label: 'Sucrose', value: s.sucrose },
        { label: 'Lactose', value: s.lactose },
        { label: 'Maltose', value: s.maltose },
        { label: 'Total sugars', value: s.total_sugars },
        { label: 'Added sugars', value: s.added_sugars },
        { label: 'Sugar alcohols', value: s.sugar_alcohols },
        { label: 'Perceived sweetness', value: s.sweetness_perception }
      ], ' g');
    }

    const processingEl = content.querySelector('[data-detail-processing]');
    if (processingEl) {
      const p = nutrition?.processing_effects || {};
      processingEl.innerHTML = detailGrid([
        { label: 'Vitamin retention', value: p.vitamin_retention_pct },
        { label: 'Protein denature temp', value: p.protein_denature_temp },
        { label: 'Maillard temp', value: p.maillard_temp }
      ], ' %') +
      `${p.mineral_leaching_risk ? `<p>${escapeHtml(p.mineral_leaching_risk)}</p>` : ''}` +
      `${p[`best_cooking_method_${detailLang()}`] || p.best_cooking_method_en ? `<p><strong>${escapeHtml(t.processingMethod)}:</strong> ${escapeHtml(p[`best_cooking_method_${detailLang()}`] || p.best_cooking_method_en)}</p>` : ''}` +
      `${p[`processing_notes_${detailLang()}`] || p.processing_notes_en ? `<p>${escapeHtml(p[`processing_notes_${detailLang()}`] || p.processing_notes_en)}</p>` : ''}`;
    }

    const behaviorEl = content.querySelector('[data-detail-behavior]');
    if (behaviorEl) {
      const behavior = nutrition?.culinary_behavior?.behaviors;
      if (Array.isArray(behavior)) {
        behaviorEl.innerHTML = detailList(behavior.map(item => {
          if (typeof item === 'string') return item;
          if (item && typeof item === 'object') {
            return [item.title || item.name || item.type, item.effect || item.impact].filter(Boolean).join(' - ');
          }
          return '';
        }).filter(Boolean));
      } else if (behavior && typeof behavior === 'object') {
        behaviorEl.innerHTML = `<pre class="ingredient-json">${escapeHtml(JSON.stringify(behavior, null, 2))}</pre>`;
      } else {
        behaviorEl.innerHTML = '<p class="text-muted">—</p>';
      }
    }

    const measuresEl = content.querySelector('[data-detail-measures]');
    if (measuresEl) {
      const m = ingredient?.measures || {};
      measuresEl.innerHTML = detailGrid([
        { label: t.cup, value: m.grams_per_cup },
        { label: t.tbsp, value: m.grams_per_tbsp },
        { label: t.tsp, value: m.grams_per_tsp },
        { label: t.density, value: ingredient?.density_g_per_ml }
      ], ' g');
    }

    const nutritionCalcRoot = content.querySelector('[data-nutrition-calculator]');
    const measureCalcRoot = content.querySelector('[data-measure-calculator]');

    async function runNutritionCalculator() {
      if (!nutritionCalcRoot) return;
      const amountInput = nutritionCalcRoot.querySelector('[data-nutrition-amount]');
      const unitSelect = nutritionCalcRoot.querySelector('[data-nutrition-unit]');
      const resultEl = nutritionCalcRoot.querySelector('[data-nutrition-result]');
      const amount = amountInput?.value || '100';
      const unit = unitSelect?.value || 'g';

      if (resultEl) {
        resultEl.textContent = loading;
      }

      try {
        const response = await fetch(`/public/tools/nutrition?slug=${encodeURIComponent(slug)}&amount=${encodeURIComponent(amount)}&unit=${encodeURIComponent(unit)}&lang=${encodeURIComponent(detailLang())}`);
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        const data = await response.json();
        const perAmount = data.for_amount || {};
        const score = data.nutrition_score ?? '—';

        if (resultEl) {
          resultEl.innerHTML = `
            <div class="ingredient-calculator-result-grid">
              <div><strong>${formatCalculatorValue(data.amount_g, ' g')}</strong><span>${I18N.lang === 'ru' ? 'вес' : I18N.lang === 'en' ? 'weight' : 'waga'}</span></div>
              <div><strong>${formatCalculatorValue(perAmount.calories_kcal)}</strong><span>kcal</span></div>
              <div><strong>${formatCalculatorValue(perAmount.protein_g, ' g')}</strong><span>${t.protein}</span></div>
              <div><strong>${formatCalculatorValue(perAmount.fat_g, ' g')}</strong><span>${t.fat}</span></div>
              <div><strong>${formatCalculatorValue(perAmount.carbs_g, ' g')}</strong><span>${t.carbs}</span></div>
              <div><strong>${formatCalculatorValue(perAmount.fiber_g, ' g')}</strong><span>${t.fiber}</span></div>
            </div>
            <p class="ingredient-note">${I18N.lang === 'ru' ? 'Качество данных' : I18N.lang === 'en' ? 'Data quality' : 'Jakość danych'}: <strong>${score}%</strong></p>
          `;
        }
      } catch {
        if (resultEl) {
          resultEl.innerHTML = `<p class="ingredient-empty">${escapeHtml(error)}</p>`;
        }
      }
    }

    async function runMeasureCalculator() {
      if (!measureCalcRoot) return;
      const amountInput = measureCalcRoot.querySelector('[data-measure-amount]');
      const unitSelect = measureCalcRoot.querySelector('[data-measure-unit]');
      const resultEl = measureCalcRoot.querySelector('[data-measure-result]');
      const amount = amountInput?.value || '1';
      const unit = unitSelect?.value || 'cup';

      if (resultEl) {
        resultEl.textContent = loading;
      }

      try {
        const response = await fetch(`/public/tools/measure-conversion?ingredient=${encodeURIComponent(slug)}&from=${encodeURIComponent(unit)}&to=g&lang=${encodeURIComponent(detailLang())}&value=${encodeURIComponent(amount)}`);
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        const data = await response.json();

        if (resultEl) {
          resultEl.innerHTML = `
            <div class="ingredient-calculator-result-grid">
              <div><strong>${formatCalculatorValue(data.result, ' g')}</strong><span>${I18N.lang === 'ru' ? 'в граммах' : I18N.lang === 'en' ? 'in grams' : 'w gramach'}</span></div>
              <div><strong>${escapeHtml(data.ingredient_name || name)}</strong><span>${escapeHtml(data.from_label || unit)}</span></div>
            </div>
            <p class="ingredient-note">${escapeHtml(data.answer || '')}</p>
          `;
        }
      } catch {
        if (resultEl) {
          resultEl.innerHTML = `<p class="ingredient-empty">${escapeHtml(error)}</p>`;
        }
      }
    }

    if (nutritionCalcRoot) {
      nutritionCalcRoot.querySelector('[data-nutrition-run]')?.addEventListener('click', runNutritionCalculator);
      nutritionCalcRoot.querySelector('[data-nutrition-amount]')?.addEventListener('change', runNutritionCalculator);
      nutritionCalcRoot.querySelector('[data-nutrition-unit]')?.addEventListener('change', runNutritionCalculator);
      runNutritionCalculator();
    }

    if (measureCalcRoot) {
      measureCalcRoot.querySelector('[data-measure-run]')?.addEventListener('click', runMeasureCalculator);
      measureCalcRoot.querySelector('[data-measure-amount]')?.addEventListener('change', runMeasureCalculator);
      measureCalcRoot.querySelector('[data-measure-unit]')?.addEventListener('change', runMeasureCalculator);
      runMeasureCalculator();
    }

    const pairingsEl = content.querySelector('[data-detail-pairings]');
    if (pairingsEl) {
      const pairings = Array.isArray(nutrition?.pairings) ? nutrition.pairings : [];
      pairingsEl.innerHTML = pairings.length
        ? pairings.map(pairing => {
            const pairName = detailName(pairing);
            const pairHref = `/ingredient-catalog/${encodeURIComponent(pairing.slug || '')}`;
            return `
              <a href="${pairHref}" class="ingredient-pairing-card">
                <strong>${escapeHtml(pairName)}</strong>
                <span>${escapeHtml(detailValue(pairing.pair_score))}/10</span>
              </a>
            `;
          }).join('')
        : '<p class="text-muted">—</p>';
    }
  }).catch(() => {
    if (content) {
      content.innerHTML = `<div class="ingredient-state">${escapeHtml(error)}</div>`;
    }
  });
}

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
