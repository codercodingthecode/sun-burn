// sun-burn landing page — script.js

(function () {
  // ---- Version badge ----
  var rawVersion = window.SUNBURN_VERSION;
  var rawSha = window.SUNBURN_SHA;
  var version = (rawVersion && !rawVersion.includes('{{')) ? rawVersion : '0.1.0';
  var sha = (rawSha && !rawSha.includes('{{')) ? rawSha : 'local';
  var text = 'v' + version + ' (' + sha + ')';

  var heroBadge = document.getElementById('version-badge');
  if (heroBadge) heroBadge.textContent = text;

  var footerBadge = document.getElementById('footer-version-badge');
  if (footerBadge) footerBadge.textContent = text;

  // ---- Lightbox ----
  var overlay = document.createElement('div');
  overlay.id = 'lightbox';
  overlay.innerHTML =
    '<img id="lightbox-img" alt="" />' +
    '<button id="lightbox-close" aria-label="Close">✕</button>' +
    '<span id="lightbox-caption"></span>';
  document.body.appendChild(overlay);

  var lbImg = document.getElementById('lightbox-img');
  var lbClose = document.getElementById('lightbox-close');
  var lbCaption = document.getElementById('lightbox-caption');

  function openLightbox(src, alt) {
    lbImg.src = src;
    lbImg.alt = alt;
    lbCaption.textContent = alt;
    overlay.classList.add('open');
    document.body.style.overflow = 'hidden';
  }

  function closeLightbox() {
    overlay.classList.remove('open');
    document.body.style.overflow = '';
  }

  document.querySelectorAll('.screenshot-img').forEach(function (img) {
    img.addEventListener('click', function () {
      openLightbox(img.src, img.alt);
    });
  });

  overlay.addEventListener('click', function (e) {
    if (e.target === overlay) closeLightbox();
  });

  lbClose.addEventListener('click', closeLightbox);

  document.addEventListener('keydown', function (e) {
    if (e.key === 'Escape') closeLightbox();
  });
})();
