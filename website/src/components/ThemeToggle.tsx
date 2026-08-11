"use client";

import { useEffect, useSyncExternalStore } from "react";

type Theme = "light" | "dark";

const STORAGE_KEY = "theme";

// The theme's home is `localStorage`, not React state.
//
// This component used to mirror it into `useState` and populate that from an
// effect, which meant every mount rendered the wrong theme once and then
// corrected itself — the cascading render `react-hooks/set-state-in-effect`
// warns about. Treating storage as the external system it already is removes
// the mirror, and `useSyncExternalStore` is built precisely for reading one.

const listeners = new Set<() => void>();

function subscribe(onChange: () => void) {
  listeners.add(onChange);
  // A change in another tab should move this one too.
  window.addEventListener("storage", onChange);
  return () => {
    listeners.delete(onChange);
    window.removeEventListener("storage", onChange);
  };
}

function notify() {
  for (const listener of listeners) listener();
}

/// Returns a primitive, so `useSyncExternalStore`'s identity check is a value
/// comparison and this can be recomputed on every render safely.
function readTheme(): Theme {
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored === "light" || stored === "dark") return stored;
  return window.matchMedia("(prefers-color-scheme: light)").matches
    ? "light"
    : "dark";
}

/// Never notifies: constant per environment, so this is a hydration probe
/// rather than a real subscription.
const neverChanges = () => () => {};

export default function ThemeToggle() {
  // `false` on the server and during the first client render, `true` after
  // hydration — which is what the placeholder below needs to keep the server
  // and client markup identical.
  const mounted = useSyncExternalStore(neverChanges, () => true, () => false);

  // `"dark"` is the server snapshot, matching the default in `globals.css`.
  const theme = useSyncExternalStore(subscribe, readTheme, (): Theme => "dark");

  // The only effect left writes to the DOM and reads nothing back, which is
  // what effects are for.
  useEffect(() => {
    if (!mounted) return;
    document.documentElement.setAttribute("data-theme", theme);
  }, [theme, mounted]);

  const toggle = () => {
    localStorage.setItem(STORAGE_KEY, theme === "dark" ? "light" : "dark");
    notify();
  };

  if (!mounted) {
    return (
      <button className="p-2 rounded w-9 h-9" aria-label="Toggle theme" />
    );
  }

  return (
    <button
      onClick={toggle}
      className="p-2 rounded hover:bg-[var(--color-bg-secondary)] transition-colors text-[var(--color-text-secondary)] hover:text-[var(--color-primary)]"
      aria-label={`Switch to ${theme === "dark" ? "light" : "dark"} mode`}
      title={`Switch to ${theme === "dark" ? "light" : "dark"} mode`}
    >
      {theme === "dark" ? (
        /* Sun icon — switch to light (vault unsealed) */
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
          <circle cx="12" cy="12" r="5" />
          <line x1="12" y1="1" x2="12" y2="3" />
          <line x1="12" y1="21" x2="12" y2="23" />
          <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" />
          <line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
          <line x1="1" y1="12" x2="3" y2="12" />
          <line x1="21" y1="12" x2="23" y2="12" />
          <line x1="4.22" y1="19.78" x2="5.64" y2="18.36" />
          <line x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
        </svg>
      ) : (
        /* Shield icon — switch to dark (vault sealed) */
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
          <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
        </svg>
      )}
    </button>
  );
}
