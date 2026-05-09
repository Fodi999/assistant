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

      setTimeout(startWebGpuScene, 100);
    });

    closeButton?.addEventListener("click", () => {
      document.body.classList.remove("engine-open");
    });
"##
}

pub fn all_scripts() -> String {
    format!(
        "<script>{}{}{}</script>",
        open_close_js(),
        toolbar::toolbar_js(),
        webgpu::webgpu_js(),
    )
}
