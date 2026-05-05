mod cards;
mod toolbar;
mod webgpu;

pub fn open_close_js() -> &'static str {
    r##"
    const openButton  = document.getElementById("open-chefos");
    const closeButton = document.getElementById("close-chefos");

    openButton?.addEventListener("click", () => {
      document.body.classList.add("engine-open");

      const canvas = document.getElementById('webgpu-canvas');
      canvas.width  = window.innerWidth;
      canvas.height = window.innerHeight;

      const diag = document.getElementById('gpu-diag');
      if (diag) {
        diag.style.display = 'block';
        diag.innerHTML = `
          <b style="color:#67e8f9">WebGPU Диагностика</b><br><br>
          navigator.gpu: <b style="color:${navigator.gpu ? '#34d399' : '#f87171'}">${navigator.gpu ? '✓ есть' : '✗ нет'}</b><br>
          canvas: <b>${canvas.width}×${canvas.height}</b><br>
          dpr: <b>${window.devicePixelRatio}</b><br><br>
          <span style="color:#94a3b8">запускаем WebGPU…</span>
        `;
      }

      setTimeout(startWebGpuScene, 100);
    });

    closeButton?.addEventListener("click", () => {
      document.body.classList.remove("engine-open");
    });
"##
}

pub fn all_scripts() -> String {
    format!(
        "<script>{}{}{}{}</script>",
        open_close_js(),
        cards::cards_js(),
        toolbar::toolbar_js(),
        webgpu::webgpu_js(),
    )
}
