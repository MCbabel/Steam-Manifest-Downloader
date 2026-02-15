/* ========================================
   Steam Manifest Downloader — Landing Page
   Scroll animations & interactions
   ======================================== */

(function () {
  'use strict';

  // --- Scroll Reveal with Intersection Observer ---
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

  // --- Navbar scroll effect ---
  const nav = document.getElementById('nav');
  let lastScroll = 0;

  function handleNavScroll() {
    const scrollY = window.scrollY;
    if (scrollY > 50) {
      nav.classList.add('scrolled');
    } else {
      nav.classList.remove('scrolled');
    }
    lastScroll = scrollY;
  }

  window.addEventListener('scroll', handleNavScroll, { passive: true });
  handleNavScroll();

  // --- Smooth scroll for anchor links ---
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

  // --- Step number count-up animation ---
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

  // --- Active nav link highlight on scroll ---
  const sections = document.querySelectorAll('section[id]');
  const navLinks = document.querySelectorAll('.nav-links a');

  function highlightNavLink() {
    const scrollY = window.scrollY + 120;

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
})();
