pub fn cards_js() -> &'static str {
    // No HTML cards — replaced by WebGPU sphere particles
    ""
}

pub fn _unused_cards_js() -> &'static str {
    r##"
    // ── GPU card data (archived) ──────────────────────────────────
    // x/y = screen position 0..100%
    // s   = visual scale (CSS size, also maps to GPU quad scale)
    // ry  = rotateY: left cards tilt right (+), right tilt left (-), center = 0
    // tz  = translateZ: center closest to camera, edges furthest
    const ingredientCards = [
      { gpuIdx:3, name:"Salmon",            type:"fish",     kcal:208, info:"Protein 20g · Fat 13g",     action:"Salmon · add to recipe",           emoji:"🐟", x:"22%", y:"56%", s:0.62, ry:"14deg",  tz:"0px"   },
      { gpuIdx:1, name:"Milk",              type:"dairy",    kcal:42,  info:"Protein 3.4g · Fat 1g",     action:"Milk · add to recipe",             emoji:"🥛", x:"34%", y:"51%", s:0.82, ry:"8deg",   tz:"55px"  },
      { gpuIdx:0, name:"Mozzarella cheese", type:"dairy",    kcal:318, info:"Protein 22.2g · Fat 24.5g", action:"Mozzarella cheese · add to recipe", emoji:"🧀", x:"50%", y:"47%", s:1.00, ry:"0deg",   tz:"110px" },
      { gpuIdx:2, name:"Beer",              type:"beverage", kcal:43,  info:"Carbs 3.6g · beverage",     action:"Beer · check beverage cost",       emoji:"🍺", x:"66%", y:"51%", s:0.82, ry:"-8deg",  tz:"55px"  },
      { gpuIdx:4, name:"Strawberry",        type:"fruit",    kcal:32,  info:"Vitamin C · Antioxidant",   action:"Strawberry · add to dessert",      emoji:"🍓", x:"78%", y:"56%", s:0.62, ry:"-14deg", tz:"0px"   },
    ];

    // shared with WebGPU render loop
    let gpuActiveIdx = 0;

    function selectGpuCard(card) {
      document.querySelectorAll('.gpu-card').forEach(c => {
        c.classList.remove('active');
        c.style.setProperty('--ry', c.dataset.origRy || '0deg');
      });
      card.classList.add('active');
      gpuActiveIdx = parseInt(card.dataset.gpuIdx ?? '0', 10);

      const sel = (id, val) => { const el = document.getElementById(id); if(el) el.textContent = val; };
      sel('selected-name',         card.dataset.name   || 'Object');
      sel('selected-type',         card.dataset.type   || 'ingredient');
      sel('selected-kcal',         card.dataset.kcal   || '—');
      sel('selected-info',         card.dataset.info   || 'No data');
      sel('selected-action-title', card.dataset.action || 'Ready');
    }

    function renderGpuCards(items) {
      const overlay = document.getElementById('gpu-card-overlay');
      if (!overlay) return;
      overlay.innerHTML = '';
      items.forEach((item, i) => {
        const card = document.createElement('button');
        card.type = 'button';
        card.className = 'gpu-card' + (i === 2 ? ' active' : '');
        card.style.setProperty('--x',  item.x);
        card.style.setProperty('--y',  item.y);
        card.style.setProperty('--s',  item.s);
        card.style.setProperty('--ry', item.ry  || '0deg');
        card.style.setProperty('--tz', item.tz  || '0px');
        card.dataset.origRy = item.ry || '0deg';
        card.style.width    = Math.round(138 * item.s) + 'px';
        card.style.minHeight= Math.round(210 * item.s) + 'px';
        card.dataset.gpuIdx = item.gpuIdx;
        card.dataset.name   = item.name;
        card.dataset.type   = item.type;
        card.dataset.kcal   = item.kcal + ' kcal';
        card.dataset.info   = item.info;
        card.dataset.action = item.action;
        card.innerHTML = `
          <div class="gpu-card-image">
            ${item.image ? `<img src="${item.image}" alt="${item.name}">` : `<span>${item.emoji}</span>`}
          </div>
          <strong>${item.name}</strong>
          <small>${item.kcal} kcal</small>
        `;
        card.addEventListener('click', () => selectGpuCard(card));
        overlay.appendChild(card);
      });
    }

    renderGpuCards(ingredientCards);
"##
}
