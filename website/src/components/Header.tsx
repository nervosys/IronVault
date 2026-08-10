"use client";

import Link from "next/link";
import { useState } from "react";
import { navigation } from "./Sidebar";
import { usePathname } from "next/navigation";
import ThemeToggle from "./ThemeToggle";

export default function Header() {
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const pathname = usePathname();

  return (
    <>
      <header className="fixed top-0 left-0 right-0 h-[var(--header-height)] bg-[var(--color-bg)]/95 backdrop-blur-md border-b border-[var(--color-border)] z-50 theme-transition">
        <div className="h-full flex items-center justify-between px-4 lg:px-6">
          <div className="flex items-center gap-3">
            <button
              className="lg:hidden p-2 rounded hover:bg-[var(--color-bg-secondary)] transition-colors"
              onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
              aria-label="Toggle menu"
            >
              <svg width="20" height="20" viewBox="0 0 20 20" fill="currentColor">
                {mobileMenuOpen ? (
                  <path fillRule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clipRule="evenodd" />
                ) : (
                  <path fillRule="evenodd" d="M3 5a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1zM3 10a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1zM3 15a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1z" clipRule="evenodd" />
                )}
              </svg>
            </button>
            <Link href="/" className="flex items-center gap-2.5 font-semibold text-lg group">
              {/* Vault lock icon */}
              <svg width="26" height="26" viewBox="0 0 24 24" fill="none" stroke="var(--color-primary)" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" className="group-hover:drop-shadow-[0_0_8px_var(--color-glow)] transition-all">
                <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
                <path d="M7 11V7a5 5 0 0110 0v4" />
                <circle cx="12" cy="16" r="1" />
              </svg>
              <span className="tracking-tight font-mono font-bold text-lg">AIM</span>
            </Link>
          </div>

          <nav className="hidden lg:flex items-center gap-1 text-sm font-mono">
            {[
              { href: "/docs", label: "Docs", internal: true },
              { href: "/demos", label: "Demos", internal: true },
              { href: "/docs/api", label: "API", internal: true },
              { href: "/docs/quickstart", label: "Quick Start", internal: true },
              { href: "https://github.com/nervosys/IronVault", label: "GitHub", internal: false, external: true },
            ].map((link) => {
              const isActive = pathname === link.href;
              const cls = `px-3 py-1.5 rounded transition-colors text-sm uppercase tracking-wider ${
                isActive
                  ? "text-[var(--color-primary)] bg-[var(--color-glow)]"
                  : "text-[var(--color-text-secondary)] hover:text-[var(--color-text)] hover:bg-[var(--color-bg-secondary)]"
              }`;
              if (link.external) {
                return (
                  <a key={link.href} href={link.href} target="_blank" rel="noopener noreferrer" className={cls}>
                    {link.label}
                  </a>
                );
              }
              if (!link.internal) {
                return <a key={link.href} href={link.href} className={cls}>{link.label}</a>;
              }
              return <Link key={link.href} href={link.href} className={cls}>{link.label}</Link>;
            })}
          </nav>

          <div className="flex items-center gap-2">
            <ThemeToggle />
            <span className="hidden sm:inline-flex items-center px-2 py-0.5 rounded text-sm font-mono font-bold uppercase tracking-wider border border-[var(--color-primary)]/25 text-[var(--color-primary)] bg-[var(--color-glow)]">
              v1.3.0
            </span>
            <a
              href="https://crates.io/crates/ironvault"
              target="_blank"
              rel="noopener noreferrer"
              className="hidden sm:inline-flex px-3 py-1.5 rounded text-sm font-mono font-bold uppercase tracking-wider bg-[var(--color-primary)] text-black hover:opacity-90 transition-all"
            >
              Install
            </a>
          </div>
        </div>
      </header>

      {/* Mobile menu overlay */}
      {mobileMenuOpen && (
        <div className="fixed inset-0 z-40 lg:hidden">
          <div
            className="fixed inset-0 bg-black/50 backdrop-blur-sm"
            onClick={() => setMobileMenuOpen(false)}
          />
          <div className="fixed top-[var(--header-height)] left-0 bottom-0 w-72 bg-[var(--color-sidebar-bg)] border-r border-[var(--color-border)] overflow-y-auto z-50">
            <nav className="p-4 pb-20">
              {navigation.map((section) => (
                <div key={section.title} className="mb-6">
                  <h3 className="px-3 mb-1 text-xs font-mono font-bold uppercase tracking-[0.2em] text-[var(--color-primary)] opacity-60">
                    {section.title}
                  </h3>
                  <ul className="space-y-0.5">
                    {section.items.map((item) => {
                      const isActive = pathname === item.href;
                      return (
                        <li key={item.href}>
                          <Link
                            href={item.href}
                            onClick={() => setMobileMenuOpen(false)}
                            className={`block px-3 py-1.5 rounded text-sm font-mono transition-colors ${
                              isActive
                                ? "bg-[var(--color-sidebar-active)] text-[var(--color-primary)] font-medium"
                                : "text-[var(--color-text-secondary)] hover:text-[var(--color-text)] hover:bg-[var(--color-sidebar-hover)]"
                            }`}
                          >
                            {item.label}
                          </Link>
                        </li>
                      );
                    })}
                  </ul>
                </div>
              ))}
            </nav>
          </div>
        </div>
      )}
    </>
  );
}
