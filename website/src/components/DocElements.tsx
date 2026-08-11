import Link from "next/link";
import { ReactNode } from "react";

type CodeBlockProps = {
  language?: string;
  title?: string;
  children: ReactNode;
};

export default function CodeBlock({ language = "bash", title, children }: CodeBlockProps) {
  return (
    <div className="my-4 rounded overflow-hidden border border-[var(--color-border)] glow-border theme-transition">
      {title && (
        <div className="px-4 py-2 bg-[var(--color-bg-secondary)] border-b border-[var(--color-border)] text-xs font-mono font-bold uppercase tracking-[0.15em] text-[var(--color-text-secondary)] flex items-center gap-2">
          <span className="inline-block w-1.5 h-1.5 rounded-full bg-[var(--color-primary)] opacity-50" />
          {title}
        </div>
      )}
      <div className="bg-[var(--color-bg-code)] p-4 overflow-x-auto">
        <pre className="text-sm text-gray-100">
          <code className={`language-${language}`}>{children}</code>
        </pre>
      </div>
    </div>
  );
}

type CalloutProps = {
  type?: "info" | "warning" | "tip" | "danger";
  title?: string;
  children: ReactNode;
};

const calloutStyles = {
  info: "border-[var(--color-primary)] bg-[var(--color-glow)] text-[var(--color-text)]",
  warning: "border-orange-500 bg-orange-500/5 text-[var(--color-text)]",
  tip: "border-emerald-500 bg-emerald-500/5 text-[var(--color-text)]",
  danger: "border-red-600 bg-red-600/5 text-[var(--color-text)]",
};

const calloutIcons = {
  info: "ℹ️",
  warning: "⚠️",
  tip: "💡",
  danger: "🚨",
};

export function Callout({ type = "info", title, children }: CalloutProps) {
  return (
    <div className={`my-4 border-l-4 rounded-r p-4 theme-transition ${calloutStyles[type]}`}>
      {title && (
        <p className="font-mono font-bold text-sm mb-1">
          {calloutIcons[type]} {title}
        </p>
      )}
      <div className="text-sm">{children}</div>
    </div>
  );
}

type FeatureCardProps = {
  icon: string;
  title: string;
  description: string;
  href?: string;
};

export function FeatureCard({ icon, title, description, href }: FeatureCardProps) {
  const content = (
    <div className="group relative p-6 rounded border border-[var(--color-border)] bg-[var(--color-surface)] glow-border glow-border-hover transition-all theme-transition corner-brackets">
      <div className="text-2xl mb-3">{icon}</div>
      <h3 className="text-lg font-mono font-bold mb-2 group-hover:text-[var(--color-primary)] transition-colors">
        {title}
      </h3>
      <p className="text-base text-[var(--color-text-secondary)] leading-relaxed">{description}</p>
    </div>
  );

  if (href) {
    // Statically imported above. This was a `require("next/link").default`
    // inside the component, which defeats bundling and tree-shaking and is a
    // CommonJS call in an ES module.
    return <Link href={href}>{content}</Link>;
  }

  return content;
}
