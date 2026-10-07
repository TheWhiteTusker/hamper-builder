"use client";

import { useEffect, useRef, useState } from "react";
import Link from "@/components/link";
import { useRouter } from "next/navigation";
import { Eye, EyeOff } from "lucide-react";
import type { SupabaseClient } from "@supabase/supabase-js";
import { createClient } from "@/lib/supabase/client";

const inputCls =
  "w-full rounded-lg bg-black/60 border border-white/20 px-3 py-2 text-sm text-white placeholder-neutral-500 focus:outline-none focus:border-amber-300 focus:ring-1 focus:ring-amber-300/40 transition-colors";

type Status = "checking" | "ready" | "invalid" | "done";

function Card({ children }: { children: React.ReactNode }) {
  return (
    <div className="w-full max-w-[360px] p-6 sm:p-7 rounded-2xl bg-[#080b0a]/90 backdrop-blur-2xl border border-amber-200/40 shadow-[0_20px_50px_rgba(0,0,0,0.85)] text-white">
      <h1 className="font-display text-xl font-bold tracking-tight text-amber-50 text-center">Reset password</h1>
      {children}
    </div>
  );
}

/**
 * Landing page for the recovery email link. The browser client detects the
 * code in the URL and exchanges it for a session (PKCE, same device), firing
 * an auth event once the recovery session is live. Only then can the user set
 * a new password via updateUser.
 */
export function ResetForm() {
  const router = useRouter();
  const supabaseRef = useRef<SupabaseClient | null>(null);
  const [status, setStatus] = useState<Status>("checking");
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [show, setShow] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const supabase = createClient();
    supabaseRef.current = supabase;

    const {
      data: { subscription },
    } = supabase.auth.onAuthStateChange((_event, session) => {
      if (session) setStatus((s) => (s === "done" ? s : "ready"));
    });

    // Fallback: if no recovery session materialises, the link is stale or was
    // opened on a different device than it was requested from.
    const timer = setTimeout(async () => {
      const { data } = await supabase.auth.getSession();
      setStatus((s) => (s === "checking" ? (data.session ? "ready" : "invalid") : s));
    }, 3000);

    return () => {
      subscription.unsubscribe();
      clearTimeout(timer);
    };
  }, []);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    if (password !== confirm) return setError("Passwords do not match.");
    setBusy(true);
    const { error } = await supabaseRef.current!.auth.updateUser({ password });
    if (error) {
      setError(error.message);
      return setBusy(false);
    }
    setStatus("done");
    setBusy(false);
  }

  if (status === "checking") {
    return <Card><p className="mt-4 text-center text-xs text-neutral-400">Verifying your reset link…</p></Card>;
  }

  if (status === "invalid") {
    return (
      <Card>
        <p className="mt-4 text-center text-xs text-red-200">
          This reset link is invalid or has expired. Open it on the same device you requested it from, or request a new one.
        </p>
        <Link href="/login" className="mt-4 block text-center text-xs text-amber-300 hover:text-amber-200 hover:underline">
          Back to sign in
        </Link>
      </Card>
    );
  }

  if (status === "done") {
    return (
      <Card>
        <p className="mt-4 text-center text-xs text-emerald-200">Password updated. You can sign in with your new password.</p>
        <button
          type="button"
          onClick={() => {
            router.replace("/login");
            router.refresh();
          }}
          className="mt-4 w-full rounded-lg bg-linear-to-r from-amber-500 via-amber-400 to-amber-500 hover:from-amber-400 hover:to-amber-300 text-black font-semibold py-2.5 px-4 text-sm shadow-lg shadow-amber-500/25 transition-all"
        >
          Go to sign in
        </button>
      </Card>
    );
  }

  return (
    <Card>
      <p className="mt-1 text-center text-xs text-amber-200/70">Choose a new password for your account.</p>
      <form onSubmit={submit} className="mt-5 space-y-3">
        <div className="relative">
          <input
            type={show ? "text" : "password"}
            className={`${inputCls} pr-10`}
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
            minLength={8}
            autoComplete="new-password"
            placeholder="New password"
            aria-label="New password"
          />
          <button
            type="button"
            onClick={() => setShow((p) => !p)}
            className="absolute inset-y-0 right-0 flex items-center pr-3 text-amber-200/60 hover:text-amber-100 transition-colors focus:outline-none"
            aria-label={show ? "Hide password" : "Show password"}
            tabIndex={-1}
          >
            {show ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
          </button>
        </div>
        <input
          type={show ? "text" : "password"}
          className={inputCls}
          value={confirm}
          onChange={(e) => setConfirm(e.target.value)}
          required
          minLength={8}
          autoComplete="new-password"
          placeholder="Confirm new password"
          aria-label="Confirm new password"
        />

        {error && (
          <div role="alert" className="rounded-lg bg-red-950/80 border border-red-500/50 p-2.5 text-xs text-red-200">
            {error}
          </div>
        )}

        <button
          type="submit"
          disabled={busy}
          className="w-full rounded-lg bg-linear-to-r from-amber-500 via-amber-400 to-amber-500 hover:from-amber-400 hover:to-amber-300 text-black font-semibold py-2.5 px-4 text-sm shadow-lg shadow-amber-500/25 transition-all disabled:opacity-50 mt-2"
        >
          {busy ? "Updating…" : "Update password"}
        </button>
      </form>
    </Card>
  );
}
