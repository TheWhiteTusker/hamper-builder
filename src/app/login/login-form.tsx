"use client";

import { RotateCcw } from "lucide-react";
import { AuthCard } from "./_lighthouse/auth-card";
import { Beam } from "./_lighthouse/beam";
import { LighthouseSvg } from "./_lighthouse/lighthouse-svg";
import { useLighthouse } from "./_lighthouse/use-lighthouse";

/** Night scene: a lighthouse beam sweeps across and reveals the sign-in card. */
export function LoginForm() {
  const lh = useLighthouse();

  return (
    <main
      onClick={lh.skip}
      className="relative min-h-screen w-full bg-[#000000] text-white flex flex-col md:flex-row items-center justify-between overflow-hidden select-none"
    >
      {/* Deep night ambient stars & ocean mist */}
      <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(ellipse_at_top,#080e0c_0%,#000000_80%)]" />

      {/* Tiny shimmering stars */}
      <div className="pointer-events-none absolute inset-0 opacity-40">
        <div className="absolute top-[12%] left-[18%] h-0.5 w-0.5 rounded-full bg-amber-100 shadow-[0_0_3px_#fff]" />
        <div className="absolute top-[22%] left-[34%] h-1 w-1 rounded-full bg-white/70 shadow-[0_0_4px_#fff]" />
        <div className="absolute top-[8%] left-[62%] h-0.5 w-0.5 rounded-full bg-amber-200/80 shadow-[0_0_3px_#fff]" />
        <div className="absolute top-[15%] left-[82%] h-1 w-1 rounded-full bg-white/60 shadow-[0_0_4px_#fff]" />
        <div className="absolute top-[28%] left-[74%] h-0.5 w-0.5 rounded-full bg-amber-100/60 shadow-[0_0_2px_#fff]" />
        <div className="absolute top-[35%] left-[12%] h-0.5 w-0.5 rounded-full bg-white/50" />
      </div>

      {/* -------------------------------------------------------------
          LEFT SIDE: THE LIGHTHOUSE (LARGER & MAJESTIC)
          ------------------------------------------------------------- */}
      <div className="relative z-20 w-full md:w-[45%] lg:w-[42%] h-105 md:h-screen flex flex-col justify-end items-center md:items-start pl-0 md:pl-6 lg:pl-12 pb-0 pointer-events-none">
        <div className="relative w-[320px] sm:w-100 md:w-120 lg:w-140 xl:w-155 h-115 sm:h-145 md:h-175 lg:h-200 xl:h-215">
          {/* Lighthouse SVG Illustration */}
          <LighthouseSvg />
          <Beam state={lh.beamState} angle={lh.angle} isDesktop={lh.isDesktop} />
        </div>
      </div>

      {/* -------------------------------------------------------------
          RIGHT SIDE: THE CIRCULAR SPOTLIGHT & THE LOGIN BOX
          The box was already sitting there; the light makes it appear
          ------------------------------------------------------------- */}
      <div className="relative z-30 w-full md:w-[55%] lg:w-[58%] h-full flex items-center justify-center p-4 sm:p-6 md:p-8">
        {/* The Circle of Light */}
        <div
          className={`relative flex items-center justify-center rounded-full transition-all duration-1000 ease-out ${
            lh.isCardRevealed
              ? "opacity-100 scale-100"
              : "opacity-0 scale-100 pointer-events-none"
          }`}
          style={{
            width: "min(90vw, 520px)",
            height: "min(90vw, 520px)",
            background:
              "radial-gradient(circle at center, rgba(255, 252, 240, 0.22) 0%, rgba(254, 243, 199, 0.12) 45%, rgba(217, 180, 74, 0.04) 70%, transparent 100%)",
            boxShadow:
              "0 0 130px rgba(251, 191, 36, 0.25), inset 0 0 90px rgba(254, 240, 138, 0.18)",
            border: "1.5px solid rgba(253, 230, 138, 0.45)",
          }}
        >
          {/* Subtle rotating ray ring on the circular beam edge */}
          <div className="pointer-events-none absolute inset-0 rounded-full border border-amber-200/20 animate-pulse" />

          {/* The Login Card inside the Circle of Light: Revealed in place by the passing light */}
          <div
            onClick={(e) => e.stopPropagation()}
            className={`w-full max-w-85 sm:max-w-92.5 p-6 sm:p-7 rounded-2xl bg-[#080b0a]/90 backdrop-blur-2xl border border-amber-200/40 shadow-[0_20px_50px_rgba(0,0,0,0.85)] text-white transition-all duration-1000 ease-out ${
              lh.isCardRevealed
                ? "opacity-100 brightness-100"
                : "opacity-0 brightness-50 pointer-events-none"
            }`}
          >
            <AuthCard />
          </div>
        </div>
      </div>

      {/* Replay Beam Sweep Control in bottom corner */}
      <button
        type="button"
        onClick={(e) => {
          e.stopPropagation();
          lh.replay();
        }}
        title="Replay Lighthouse Beam"
        className="absolute bottom-4 right-4 z-40 flex items-center gap-1.5 rounded-full bg-black/60 hover:bg-neutral-900 border border-amber-300/30 px-3 py-1.5 text-[11px] text-amber-200/80 hover:text-amber-100 transition-all backdrop-blur-md shadow-md"
      >
        <RotateCcw className="h-3 w-3" />
        <span>Sweep Light</span>
      </button>
    </main>
  );
}
