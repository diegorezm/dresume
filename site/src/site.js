const root = document.documentElement;
const T = { en: ["Copy", "Copied"], pt: ["Copiar", "Copiado"] }[root.lang] ?? ["Copy", "Copied"];

// Theme cards preview the app's built-in themes on the site itself; the choice is remembered per browser.
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

// Mobile menu.
const menu = document.querySelector(".menu-btn");
menu?.addEventListener("click", () => {
  const open = document.body.classList.toggle("menu-open");
  menu.setAttribute("aria-expanded", String(open));
});

// Nav shadow once scrolled.
const nav = document.querySelector(".nav");
const onScroll = () => nav.classList.toggle("scrolled", scrollY > 8);
addEventListener("scroll", onScroll, { passive: true });
onScroll();

// Fade sections in as they enter the viewport.
const io = new IntersectionObserver(
  (entries) => entries.forEach((e) => e.isIntersecting && (e.target.classList.add("in"), io.unobserve(e.target))),
  { rootMargin: "0px 0px -8% 0px" },
);
document.querySelectorAll(".reveal").forEach((el, i) => {
  el.style.setProperty("--d", `${(i % 3) * 70}ms`);
  io.observe(el);
});

// Copy buttons on code blocks.
document.querySelectorAll(".prose pre, .term").forEach((pre) => {
  const btn = document.createElement("button");
  btn.className = "copy";
  btn.textContent = T[0];
  btn.addEventListener("click", async () => {
    const text = pre.querySelector("code").innerText.replace(/^\$ /gm, "").replace(/^#.*\n/gm, "");
    try {
      await navigator.clipboard.writeText(text.trim());
      btn.textContent = T[1];
      setTimeout(() => (btn.textContent = T[0]), 1400);
    } catch {}
  });
  pre.append(btn);
});

// Highlight the current docs section in the table of contents.
const links = [...document.querySelectorAll(".toc a")];
if (links.length) {
  const heads = links.map((a) => document.getElementById(a.hash.slice(1))).filter(Boolean);
  const spy = () => {
    let cur = heads[0];
    for (const h of heads) if (h.getBoundingClientRect().top < 120) cur = h;
    links.forEach((a) => a.classList.toggle("on", a.hash === `#${cur.id}`));
  };
  addEventListener("scroll", spy, { passive: true });
  spy();
}
