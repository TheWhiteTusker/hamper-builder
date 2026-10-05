import Link from "next/link";
import Image from "next/image";

/** The centre card: brand lockup, what the suite does, and the way in. */
export function HeroCard({ isLoggedIn }: { isLoggedIn: boolean }) {
  return (
      <div className="w-full max-h-[92vh] overflow-y-auto rounded-3xl bg-[var(--color-paper)]/95 backdrop-blur-xl border-2 border-[var(--color-gold)]/60 shadow-[0_20px_50px_rgba(44,51,47,0.12)] p-6 sm:p-8 flex flex-col items-center justify-between text-center relative transition-transform duration-300 hover:shadow-[0_25px_60px_rgba(44,51,47,0.18)]">
        {/* Subtle gold ornamental corner accents */}
        <div className="absolute top-3 left-3 h-3 w-3 border-t-2 border-l-2 border-[var(--color-gold)] opacity-70" />
        <div className="absolute top-3 right-3 h-3 w-3 border-t-2 border-r-2 border-[var(--color-gold)] opacity-70" />
        <div className="absolute bottom-3 left-3 h-3 w-3 border-b-2 border-l-2 border-[var(--color-gold)] opacity-70" />
        <div className="absolute bottom-3 right-3 h-3 w-3 border-b-2 border-r-2 border-[var(--color-gold)] opacity-70" />

        {/* Brand Lockup & Crest */}
        <div className="flex flex-col items-center pt-2">
          <span className="text-[10px] tracking-[0.25em] font-semibold text-[var(--color-brand)] uppercase mb-2">
            Bespoke Corporate & Celebratory Gifting
          </span>
          <Image
            src="/lattice-lane-lockup.png"
            alt="Lattice Lane"
            width={1600}
            height={976}
            priority
            className="h-20 sm:h-24 w-auto object-contain brightness-0 mb-1"
          />
          <div className="h-0.5 w-14 bg-gradient-to-r from-transparent via-[var(--color-gold)] to-transparent my-2" />
        </div>

        {/* Hero Narrative */}
        <div className="my-4 sm:my-5 space-y-3">
          <h1 className="font-display text-2xl sm:text-3xl font-bold tracking-tight text-[var(--color-brand-dark)] leading-tight">
            Hamper Builder &amp; Costing Suite
          </h1>
          <p className="text-xs sm:text-sm text-[var(--color-muted)] leading-relaxed max-w-sm mx-auto">
            Architect unforgettable impressions. Curate artisanal gifts, simulate container limits, calculate precise gross margins, and generate proforma quotations in seconds.
          </p>
        </div>

        {/* Feature Highlights Badges */}
        <div className="grid grid-cols-2 gap-2 w-full my-2 text-left">
          <div className="rounded-xl border border-[var(--color-line)] bg-white/80 px-3 py-2 text-xs">
            <span className="font-semibold text-[var(--color-ink)] block">📦 Dynamic Canvas</span>
            <span className="text-[11px] text-[var(--color-muted)]">Visual layout &amp; items</span>
          </div>
          <div className="rounded-xl border border-[var(--color-line)] bg-white/80 px-3 py-2 text-xs">
            <span className="font-semibold text-[var(--color-ink)] block">⚡ Margin Control</span>
            <span className="text-[11px] text-[var(--color-muted)]">Instant pricing &amp; GST</span>
          </div>
          <div className="rounded-xl border border-[var(--color-line)] bg-white/80 px-3 py-2 text-xs">
            <span className="font-semibold text-[var(--color-ink)] block">📊 PowerPoint Deck</span>
            <span className="text-[11px] text-[var(--color-muted)]">Automated pitch exports</span>
          </div>
          <div className="rounded-xl border border-[var(--color-line)] bg-white/80 px-3 py-2 text-xs">
            <span className="font-semibold text-[var(--color-ink)] block">📜 Quotations &amp; PI</span>
            <span className="text-[11px] text-[var(--color-muted)]">Print-ready proforma</span>
          </div>
        </div>

        {/* Call To Action Buttons */}
        <div className="w-full pt-4 space-y-2.5">
          <Link
            href="/dashboard"
            className="w-full flex items-center justify-center gap-2 rounded-xl bg-[var(--color-brand)] text-white font-semibold py-3 px-6 text-sm shadow-md hover:bg-[var(--color-brand-dark)] hover:shadow-lg transition-all duration-200 group"
          >
            <span>{isLoggedIn ? "Open Workspace" : "Enter Hamper Builder"}</span>
            <span className="transition-transform duration-200 group-hover:translate-x-1 font-bold">
              &rarr;
            </span>
          </Link>

          {!isLoggedIn && (
            <Link
              href="/login"
              className="w-full block text-center text-xs text-[var(--color-brand)] font-medium hover:underline py-1"
            >
              Team Sign In &bull; Lattice Lane Workspace
            </Link>
          )}
        </div>

        {/* Footer Trust Indicator */}
        <div className="pt-4 border-t border-[var(--color-line)]/60 w-full mt-3">
          <p className="text-[10px] text-[var(--color-muted)] uppercase tracking-wider font-semibold">
            Private Enterprise System &bull; Lattice Lane
          </p>
        </div>
      </div>
  );
}
