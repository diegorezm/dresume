// Theme cards preview the app's built-in themes on the site itself; the choice is remembered per browser.
const root = document.documentElement;
const mark = () =>
  document.querySelectorAll(".theme-card").forEach((b) => b.classList.toggle("on", b.dataset.theme === root.dataset.theme));

document.querySelectorAll(".theme-card").forEach((b) =>
  b.addEventListener("click", () => {
    const same = root.dataset.theme === b.dataset.theme;
    if (same) delete root.dataset.theme;
    else root.dataset.theme = b.dataset.theme;
    try {
      same ? localStorage.removeItem("dresume-theme") : localStorage.setItem("dresume-theme", b.dataset.theme);
    } catch {}
    mark();
  }),
);
mark();
