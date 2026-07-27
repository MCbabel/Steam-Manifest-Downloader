(function () {
  'use strict';

  // Scroll Reveal with Intersection Observer
  const revealElements = document.querySelectorAll('.reveal');

  if ('IntersectionObserver' in window) {
    const revealObserver = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            entry.target.classList.add('visible');
            revealObserver.unobserve(entry.target);
          }
        });
      },
      {
        threshold: 0.1,
        rootMargin: '0px 0px -40px 0px',
      }
    );

    revealElements.forEach((el) => revealObserver.observe(el));
  } else {
    // Fallback: show everything
    revealElements.forEach((el) => el.classList.add('visible'));
  }

  // Navbar scroll effect
  const nav = document.getElementById('nav');

  function handleNavScroll() {
    if (window.scrollY > 50) {
      nav.classList.add('scrolled');
    } else {
      nav.classList.remove('scrolled');
    }
  }

  window.addEventListener('scroll', handleNavScroll, { passive: true });
  handleNavScroll();

  // Smooth scroll for anchor links
  document.querySelectorAll('a[href^="#"]').forEach((anchor) => {
    anchor.addEventListener('click', function (e) {
      const targetId = this.getAttribute('href');
      if (targetId === '#') return;

      const target = document.querySelector(targetId);
      if (target) {
        e.preventDefault();
        target.scrollIntoView({ behavior: 'smooth', block: 'start' });
      }
    });
  });

  // Step number count-up animation
  const stepNumbers = document.querySelectorAll('.step-number');

  if ('IntersectionObserver' in window) {
    const stepObserver = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            const el = entry.target;
            const target = parseInt(el.getAttribute('data-count'), 10);
            animateCount(el, target);
            stepObserver.unobserve(el);
          }
        });
      },
      { threshold: 0.5 }
    );

    stepNumbers.forEach((el) => stepObserver.observe(el));
  }

  function animateCount(el, target) {
    let current = 0;
    const duration = 600;
    const step = duration / target;

    function tick() {
      current++;
      el.textContent = current;
      if (current < target) {
        setTimeout(tick, step);
      }
    }

    el.textContent = '0';
    setTimeout(tick, 200);
  }

  // Active nav link highlight on scroll
  const sections = document.querySelectorAll('section[id]');
  const navLinks = document.querySelectorAll('.nav-links a');

  function highlightNavLink() {
    const scrollY = window.scrollY + 120;

    // If at the bottom of the page, force-highlight the last section's nav link
    const atBottom = window.innerHeight + window.scrollY >= document.body.offsetHeight - 2;
    if (atBottom && sections.length > 0) {
      const lastId = sections[sections.length - 1].getAttribute('id');
      navLinks.forEach((link) => {
        link.style.color = '';
        if (link.getAttribute('href') === '#' + lastId) {
          link.style.color = '#e6edf3';
        }
      });
      return;
    }

    sections.forEach((section) => {
      const top = section.offsetTop;
      const height = section.offsetHeight;
      const id = section.getAttribute('id');

      if (scrollY >= top && scrollY < top + height) {
        navLinks.forEach((link) => {
          link.style.color = '';
          if (link.getAttribute('href') === '#' + id) {
            link.style.color = '#e6edf3';
          }
        });
      }
    });
  }

  window.addEventListener('scroll', highlightNavLink, { passive: true });
  highlightNavLink();

  // Mobile Hamburger Menu Toggle
  const hamburger = document.getElementById('navHamburger');
  const mobileMenu = document.getElementById('mobileMenu');
  const mobileMenuClose = document.getElementById('mobileMenuClose');

  function openMobileMenu() {
    if (mobileMenu) {
      mobileMenu.classList.add('open');
      document.body.style.overflow = 'hidden';
    }
  }

  function closeMobileMenu() {
    if (mobileMenu) {
      mobileMenu.classList.remove('open');
      document.body.style.overflow = '';
    }
  }

  if (hamburger) {
    hamburger.addEventListener('click', function () {
      if (mobileMenu && mobileMenu.classList.contains('open')) {
        closeMobileMenu();
      } else {
        openMobileMenu();
      }
    });
  }

  if (mobileMenuClose) {
    mobileMenuClose.addEventListener('click', closeMobileMenu);
  }

  // Close mobile menu when a link is clicked
  if (mobileMenu) {
    mobileMenu.querySelectorAll('a').forEach(function (link) {
      link.addEventListener('click', closeMobileMenu);
    });
  }

  // Close mobile menu on window resize to desktop
  window.addEventListener('resize', function () {
    if (window.innerWidth > 768) {
      closeMobileMenu();
    }
  });

  // Download buttons: point straight at the asset matching the visitor's OS.
  // Windows → signed NSIS installer, Linux → AppImage. Anything else keeps the
  // releases page so users can pick AUR / Flatpak / portable themselves.
  const ua = navigator.userAgent || '';
  const osKey = /windows|win32|win64/i.test(ua)
    ? 'windows'
    : /linux|x11/i.test(ua) && !/android|cros/i.test(ua)
      ? 'linux'
      : null;

  const assetPickers = {
    windows: (assets) =>
      assets.find((a) => /-setup\.exe$/i.test(a.name)) || assets.find((a) => /\.exe$/i.test(a.name)),
    linux: (assets) => assets.find((a) => /\.AppImage$/i.test(a.name)),
  };

  const osLabels = { windows: 'Windows', linux: 'Linux' };

  const downloadButtons = [
    {
      btn: document.getElementById('hero-download-btn'),
      label: document.getElementById('hero-download-label'),
    },
    {
      btn: document.getElementById('cta-download-btn'),
      label: document.getElementById('cta-download-label'),
    },
  ].filter((entry) => entry.btn);

  if (osKey && downloadButtons.length) {
    fetch('https://api.github.com/repos/MCbabel/Steam-Manifest-Downloader/releases/latest', {
      headers: { Accept: 'application/vnd.github+json' },
    })
      .then((r) => (r.ok ? r.json() : Promise.reject(r.status)))
      .then((release) => {
        const assets = Array.isArray(release && release.assets) ? release.assets : [];
        const asset = assetPickers[osKey](assets);
        if (!asset || !asset.browser_download_url) return;

        const suffix = release.tag_name ? ' · ' + release.tag_name : '';
        downloadButtons.forEach(({ btn, label }) => {
          btn.href = asset.browser_download_url;
          btn.removeAttribute('target');
          btn.removeAttribute('rel');
          if (label) label.textContent = 'Download for ' + osLabels[osKey] + suffix;
        });
      })
      .catch(() => {
        // network / api error → leave the buttons pointing at the releases page
      });
  }
})();
