// The embedded app reports its height; size the glass to it.
window.addEventListener("message", (e) => {
  const d = e.data;
  if (d && d.portkeeper === "height" && Number.isFinite(d.value)) {
    document.getElementById("glass").style.setProperty("--h", Math.min(Math.max(d.value, 60), 600) + "px");
  }
});

// Highlight the visitor's platform, show the latest version and the star count.
(function () {
  const REPO = "MarcAparisiTFG/portkeeper";
  const ua = navigator.userAgent;
  const os = /Windows/i.test(ua) ? "win" : /Mac/i.test(navigator.platform || ua) ? "mac" : /Linux/i.test(ua) ? "linux" : null;
  const here = os && document.getElementById(os);
  if (here) here.classList.add("here");

  // Installer links use releases/latest/download/<version-free name>; only the label needs the version.
  fetch("https://api.github.com/repos/" + REPO + "/releases/latest")
    .then((r) => (r.ok ? r.json() : null))
    .then((rel) => { if (rel && rel.tag_name) document.getElementById("ver").textContent = rel.tag_name; })
    .catch(() => {});
  // Star count.
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
})();
