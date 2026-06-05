import { useEffect } from 'react';

const MOTION_SELECTOR = [
  '.admin-page__header',
  '.admin-stat-card',
  '.admin-panel',
  '.admin-list-table__row',
  '.admin-media-card',
  '.admin-empty-state',
  '.admin-inline-state',
  '.admin-footer',
].join(', ');

function prefersReducedMotion() {
  return window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
}

export default function useAdminRouteMotion(pathname) {
  useEffect(() => {
    if (typeof window === 'undefined' || prefersReducedMotion()) {
      return undefined;
    }

    const canvas = document.querySelector('.admin-canvas');
    if (!canvas) {
      return undefined;
    }

    const targets = Array.from(canvas.querySelectorAll(MOTION_SELECTOR));
    targets.forEach((target, index) => {
      target.style.setProperty('--motion-order', String(index));
    });

    canvas.classList.remove('is-motion-enter');
    canvas.setAttribute('data-route-motion', 'reset');

    const frameId = window.requestAnimationFrame(() => {
      canvas.classList.add('is-motion-enter');
      canvas.setAttribute('data-route-motion', 'enter');
    });

    const cleanupId = window.setTimeout(() => {
      canvas.classList.remove('is-motion-enter');
      canvas.setAttribute('data-route-motion', 'settled');
    }, 980);

    return () => {
      window.cancelAnimationFrame(frameId);
      window.clearTimeout(cleanupId);
    };
  }, [pathname]);
}
