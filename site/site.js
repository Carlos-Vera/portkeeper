// The embedded app reports its height; size the glass to it.
window.addEventListener("message", (e) => {
  const d = e.data;
  if (d && d.portkeeper === "height" && Number.isFinite(d.value)) {
    document.getElementById("glass").style.setProperty("--h", Math.min(Math.max(d.value, 60), 600) + "px");
  }
});

// Highlight the visitor's platform and point the buttons at the latest installers.
(function () {
  const REPO = "MarcAparisiTFG/portkeeper";
  const FALLBACK = "0.2.0"; // bumped at release time
  const ua = navigator.userAgent;
  const os = /Windows/i.test(ua) ? "win" : /Mac/i.test(navigator.platform || ua) ? "mac" : /Linux/i.test(ua) ? "linux" : null;
  const here = os && document.getElementById(os);
  if (here) here.classList.add("here");

  function apply(v) {
    document.getElementById("ver").textContent = "v" + v;
    for (const a of document.querySelectorAll("[data-asset]")) {
      a.href = "https://github.com/" + REPO + "/releases/download/v" + v + "/Portkeeper_" + v + "_" + a.dataset.asset;
    }
    // macOS has two builds and a browser cannot tell them apart: the release page lists both.
  }
  apply(FALLBACK);
  // Star count (visible once the repository is public).
  fetch("https://api.github.com/repos/" + REPO)
    .then((r) => (r.ok ? r.json() : null))
    .then((repo) => {
      if (!repo || typeof repo.stargazers_count !== "number") return;
      const el = document.getElementById("stars");
      const n = repo.stargazers_count;
      el.textContent = n >= 1000 ? (n / 1000).toFixed(1).replace(/\.0$/, "") + "k" : String(n);
      el.hidden = false;
    })
    .catch(() => {});
  fetch("https://api.github.com/repos/" + REPO + "/releases/latest")
    .then((r) => (r.ok ? r.json() : null))
    .then((rel) => rel && rel.tag_name && apply(String(rel.tag_name).replace(/^v/, "")))
    .catch(() => {});
})();
