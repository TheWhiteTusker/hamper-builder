import Link from "@/components/link";
import Image from "next/image";
import { HeroCard } from "./_landing/hero-card";
import { getLandingData } from "./_landing/landing-data";
import { ScrollingProductColumn } from "./_landing/marquee";

export const dynamic = "force-dynamic";

export default async function LandingPage() {
  const { isLoggedIn, col1, col2, col4, col5 } = await getLandingData();

  return (
    <div className="relative h-screen w-screen overflow-hidden bg-[var(--color-paper)] text-[var(--color-ink)] flex flex-col select-none">
      {/* Top Glassmorphic Navigation Bar */}
      <header className="z-30 shrink-0 border-b border-[var(--color-line)]/80 bg-[var(--color-paper)]/90 backdrop-blur-md px-6 py-3.5 flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Link href="/" className="flex items-center gap-2 group focus:outline-none">
            <Image
              src="/lattice-lane-logo-black.png"
              alt="Lattice Lane"
              width={1768}
              height={203}
              priority
              className="h-6 sm:h-7 w-auto object-contain transition-opacity group-hover:opacity-85"
            />
          </Link>
          <span className="hidden sm:inline-block text-[11px] uppercase tracking-widest text-[var(--color-muted)] font-semibold border-l border-[var(--color-line)] pl-3">
            House of Gifting
          </span>
        </div>

        <div className="flex items-center gap-3">
          {isLoggedIn ? (
            <Link
              href="/dashboard"
              className="btn-primary text-xs sm:text-sm px-4 py-2 rounded-lg shadow-sm"
            >
              Open Dashboard &rarr;
            </Link>
          ) : (
            <>
              <Link
                href="/login"
                className="btn-secondary text-xs sm:text-sm px-3.5 py-1.5 rounded-lg font-medium"
              >
                Sign In
              </Link>
              <Link
                href="/dashboard"
                className="btn-primary text-xs sm:text-sm px-4 py-2 rounded-lg shadow-sm"
              >
                Enter Builder
              </Link>
            </>
          )}
        </div>
      </header>

      {/* Main 5-Vertical Stage */}
      <main className="relative flex-1 w-full overflow-hidden flex items-stretch px-2 sm:px-4 md:px-6 py-2">
        {/* Hardware-accelerated top and bottom edge fade overlays.
            Using static gradient overlays instead of CSS mask-image prevents continuous GPU repaints
            and makes the infinite vertical marquee silky smooth at 60fps/120fps. */}
        <div className="pointer-events-none absolute inset-x-0 top-0 h-24 bg-gradient-to-b from-[var(--color-paper)] via-[var(--color-paper)]/85 to-transparent z-20" />
        <div className="pointer-events-none absolute inset-x-0 bottom-0 h-24 bg-gradient-to-t from-[var(--color-paper)] via-[var(--color-paper)]/85 to-transparent z-20" />

        {/* 5 Vertical Layout Container */}
        <div className="relative w-full h-full flex items-stretch justify-center gap-2 sm:gap-3 md:gap-4 lg:gap-5">
          {/* Column 1 (Outer Left) - Downward Marquee */}
          <div className="hidden xl:flex flex-1 min-w-0 flex-col h-full overflow-hidden group-pause">
            <ScrollingProductColumn
              items={col1}
              durationSeconds={52}
              label="Column 1"
            />
          </div>

          {/* Column 2 (Inner Left) - Downward Marquee */}
          <div className="hidden md:flex flex-1 min-w-0 flex-col h-full overflow-hidden group-pause">
            <ScrollingProductColumn
              items={col2}
              durationSeconds={68}
              label="Column 2"
            />
          </div>

          {/* Column 3 (Center Hero - Wider / Elevated) */}
          <div className="flex-[1.4] sm:flex-[1.3] md:flex-[1.35] lg:flex-[1.25] min-w-[290px] max-w-[480px] h-full flex flex-col justify-center items-center z-20 px-1 py-2">
            <HeroCard isLoggedIn={isLoggedIn} />
          </div>

          {/* Column 4 (Inner Right) - Downward Marquee */}
          <div className="hidden md:flex flex-1 min-w-0 flex-col h-full overflow-hidden group-pause">
            <ScrollingProductColumn
              items={col4}
              durationSeconds={60}
              label="Column 4"
            />
          </div>

          {/* Column 5 (Outer Right) - Downward Marquee */}
          <div className="hidden xl:flex flex-1 min-w-0 flex-col h-full overflow-hidden group-pause">
            <ScrollingProductColumn
              items={col5}
              durationSeconds={48}
              label="Column 5"
            />
          </div>
        </div>
      </main>
    </div>
  );
}
