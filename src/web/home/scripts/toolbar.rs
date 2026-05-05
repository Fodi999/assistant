pub fn toolbar_js() -> &'static str {
    r##"
    const toolLabels = {
      "Select": "Select mode",
      "Move":   "Move mode · drag objects",
      "Object": "Object mode · edit properties",
      "Recipe": "Recipe mode · build dish"
    };

    document.querySelectorAll(".tool-button").forEach((button) => {
      button.addEventListener("click", () => {
        document.querySelectorAll(".tool-button").forEach((item) => {
          item.classList.remove("active");
        });
        button.classList.add("active");
        const label = button.textContent.trim();
        const el = document.getElementById('current-tool-label');
        if (el) el.textContent = toolLabels[label] || label;
      });
    });
"##
}
