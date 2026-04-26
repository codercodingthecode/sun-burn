// sun-burn landing page — script.js
// Version badge injection.
// window.SUNBURN_VERSION and window.SUNBURN_SHA are injected by the GH Actions
// deploy workflow (sed replacing {{VERSION}} / {{SHA}} in the inline script).
// Fall back to defaults if the placeholders were never replaced.

(function () {
  var rawVersion = window.SUNBURN_VERSION;
  var rawSha = window.SUNBURN_SHA;

  // Detect unreplaced placeholders (local preview)
  var version = (rawVersion && !rawVersion.includes('{{')) ? rawVersion : '0.1.0';
  var sha = (rawSha && !rawSha.includes('{{')) ? rawSha : 'local';

  var text = 'v' + version + ' (' + sha + ')';

  var heroBadge = document.getElementById('version-badge');
  if (heroBadge) heroBadge.textContent = text;

  var footerBadge = document.getElementById('footer-version-badge');
  if (footerBadge) footerBadge.textContent = text;
})();
