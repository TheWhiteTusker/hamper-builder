"use client";

import { useState } from "react";
import Image from "next/image";
import { useRouter, useSearchParams } from "next/navigation";
import { Eye, EyeOff } from "lucide-react";
// Supabase client is ~60 kB gzipped: fetched when the form is first touched, not with the page.
const loadClient = () => import("@/lib/supabase/client");

const inputCls =
  "w-full rounded-lg bg-black/60 border border-white/20 px-3 py-2 text-sm text-white placeholder-neutral-500 focus:outline-none focus:border-amber-300 focus:ring-1 focus:ring-amber-300/40 transition-colors";

function Label({ htmlFor, children }: { htmlFor: string; children: React.ReactNode }) {
  return (
    <label className="block text-xs font-semibold uppercase tracking-wider text-amber-200/80 mb-1" htmlFor={htmlFor}>
      {children}
    </label>
  );
}

/** Sign in, or create an account; the first account owns the workspace. */
export function AuthCard() {
  const router = useRouter();
  const params = useSearchParams();
  const [mode, setMode] = useState<"signin" | "signup" | "reset">("signin");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [fullName, setFullName] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    setNotice(null);
    const supabase = (await loadClient()).createClient();

    if (mode === "reset") {
      const { error } = await supabase.auth.resetPasswordForEmail(email, {
        redirectTo: `${window.location.origin}/reset-password`,
      });
      if (error) {
        setError(error.message);
        return setBusy(false);
      }
      // Neutral wording: never reveal whether an email is registered.
      setNotice("If that email has an account, a reset link is on its way. Check your inbox.");
      return setBusy(false);
    }

    if (mode === "signup") {
      const { data, error } = await supabase.auth.signUp({ email, password, options: { data: { full_name: fullName } } });
      if (error) {
        setError(error.message);
        return setBusy(false);
      }
      // Email confirmation on: there is no session yet.
      if (!data.session) {
        setNotice("Check your email to confirm the account, then sign in.");
        setMode("signin");
        return setBusy(false);
      }
    } else {
      const { error } = await supabase.auth.signInWithPassword({ email, password });
      if (error) {
        setError(error.message);
        return setBusy(false);
      }
    }

    router.replace(params.get("next") || "/dashboard");
    router.refresh();
  }

  return (
    <>
      <div className="flex flex-col items-center text-center">
        <Image
          src="/lattice-lane-lockup.png"
          alt="Lattice Lane — House of Gifting"
          width={1600}
          height={976}
          priority
          className="h-16 sm:h-20 w-auto object-contain drop-shadow-[0_2px_12px_rgba(251,191,36,0.3)] mb-1"
        />
        <div className="h-0.5 w-12 bg-linear-to-r from-transparent via-amber-300 to-transparent my-1.5" />
        <h1 className="font-display text-xl font-bold tracking-tight text-amber-50">
          {mode === "signin" ? "Sign in" : mode === "signup" ? "Create account" : "Reset password"}
        </h1>
        <p className="mt-0.5 text-xs text-amber-200/70 font-medium">Hamper costing and client quotations</p>
      </div>

      <form onSubmit={submit} onFocus={loadClient} className="mt-5 space-y-3">
        {mode === "signup" && (
          <div>
            <Label htmlFor="fullName">Full name</Label>
            <input id="fullName" className={inputCls} value={fullName} onChange={(e) => setFullName(e.target.value)} required autoComplete="name" placeholder="E.g. Smriti Sharma" />
          </div>
        )}

        <div>
          <Label htmlFor="email">Email address</Label>
          <input id="email" type="email" className={inputCls} value={email} onChange={(e) => setEmail(e.target.value)} required autoComplete="email" placeholder="you@latticelane.com" />
        </div>

        {mode !== "reset" && (
        <div>
          <Label htmlFor="password">Password</Label>
          <div className="relative">
            <input
              id="password"
              type={showPassword ? "text" : "password"}
              className={`${inputCls} pr-10`}
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
              minLength={8}
              autoComplete={mode === "signup" ? "new-password" : "current-password"}
              placeholder="••••••••"
            />
            <button
              type="button"
              onClick={() => setShowPassword((prev) => !prev)}
              className="absolute inset-y-0 right-0 flex items-center pr-3 text-amber-200/60 hover:text-amber-100 transition-colors focus:outline-none"
              aria-label={showPassword ? "Hide password" : "Show password"}
              tabIndex={-1}
            >
              {showPassword ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
            </button>
          </div>
        </div>
        )}

        {error && (
          <div role="alert" className="rounded-lg bg-red-950/80 border border-red-500/50 p-2.5 text-xs text-red-200">
            {error}
          </div>
        )}
        {notice && (
          <div className="rounded-lg bg-emerald-950/80 border border-emerald-500/50 p-2.5 text-xs text-emerald-200">{notice}</div>
        )}

        <button
          type="submit"
          className="w-full rounded-lg bg-linear-to-r from-amber-500 via-amber-400 to-amber-500 hover:from-amber-400 hover:to-amber-300 text-black font-semibold py-2.5 px-4 text-sm shadow-lg shadow-amber-500/25 transition-all duration-200 disabled:opacity-50 mt-2"
          disabled={busy}
        >
          {busy
            ? mode === "reset"
              ? "Sending…"
              : "Verifying…"
            : mode === "signin"
              ? "Sign in to workspace"
              : mode === "signup"
                ? "Create enterprise account"
                : "Send reset link"}
        </button>
      </form>

      <div className="mt-4 pt-3 border-t border-white/10 text-center space-y-2">
        {mode === "signin" && (
          <button
            type="button"
            className="block w-full text-xs text-amber-300 hover:text-amber-200 hover:underline transition-colors"
            onClick={() => {
              setMode("reset");
              setError(null);
              setNotice(null);
            }}
          >
            Forgot password?
          </button>
        )}
        <button
          type="button"
          className="text-xs text-amber-300 hover:text-amber-200 hover:underline transition-colors"
          onClick={() => {
            setMode(mode === "signin" ? "signup" : "signin");
            setError(null);
            setNotice(null);
          }}
        >
          {mode === "signin"
            ? "First time here? Create an account"
            : mode === "signup"
              ? "Already have an account? Sign in"
              : "Back to sign in"}
        </button>
        <p className="text-[10px] text-neutral-400">The first account created owns the workspace.</p>
      </div>
    </>
  );
}
