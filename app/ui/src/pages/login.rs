//! The night scene: a lighthouse beam sweeps across and reveals the sign-in
//! card (port of `login/login-form.tsx` and `login/_lighthouse/*`). Any click
//! skips straight to the card.

use crate::state::Auth;
use crate::tauri;
use lattice_core::api::{SessionInfo, SignUpOutcome};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::json;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Beam {
    Dark,
    Sweeping,
    Locked,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    SignIn,
    SignUp,
    Reset,
}

fn is_desktop() -> bool {
    window().inner_width().ok().and_then(|w| w.as_f64()).is_none_or(|w| w >= 768.0)
}

#[component]
pub fn Login() -> impl IntoView {
    let beam = RwSignal::new(Beam::Dark);
    let revealed = RwSignal::new(false);
    let desktop = RwSignal::new(is_desktop());
    // Each play gets a number, so a replay cancels the previous one's timers.
    let run = StoredValue::new(0u32);

    // 0 ms dark; 300 ms the beam turns on and sweeps; 1500 ms it passes the
    // card, which lights up in place; 2300 ms it holds steady.
    let play = move || {
        let id = run.get_value() + 1;
        run.set_value(id);
        beam.set(Beam::Dark);
        revealed.set(false);
        let at = move |ms: u64, f: Box<dyn Fn() + Send + Sync>| {
            set_timeout(
                move || {
                    if run.get_value() == id {
                        f();
                    }
                },
                std::time::Duration::from_millis(ms),
            );
        };
        at(300, Box::new(move || beam.set(Beam::Sweeping)));
        at(1500, Box::new(move || revealed.set(true)));
        at(2300, Box::new(move || beam.set(Beam::Locked)));
    };
    play();

    crate::kit::on_window(leptos::ev::resize, move |_| {
        if desktop.get_untracked() != is_desktop() {
            desktop.set(is_desktop());
            play();
        }
    });

    let skip = move |_| {
        if !revealed.get_untracked() || beam.get_untracked() != Beam::Locked {
            run.set_value(run.get_value() + 1);
            revealed.set(true);
            beam.set(Beam::Locked);
        }
    };

    // Out to sea on the left, swinging clockwise onto the card.
    let angle = move || match (beam.get(), desktop.get()) {
        (Beam::Dark, true) => -115,
        (Beam::Dark, false) => -85,
        (_, true) => 8,
        (_, false) => 50,
    };

    view! {
        <main
            on:click=skip
            class="relative flex min-h-screen w-full select-none flex-col items-center justify-between overflow-hidden bg-[#000000] text-white md:flex-row"
        >
            <div class="pointer-events-none absolute inset-0 bg-[radial-gradient(ellipse_at_top,_#080e0c_0%,_#000000_80%)]"></div>
            <div class="pointer-events-none absolute inset-0 opacity-40">
                <div class="absolute top-[12%] left-[18%] h-0.5 w-0.5 rounded-full bg-amber-100 shadow-[0_0_3px_#fff]"></div>
                <div class="absolute top-[22%] left-[34%] h-1 w-1 rounded-full bg-white/70 shadow-[0_0_4px_#fff]"></div>
                <div class="absolute top-[8%] left-[62%] h-0.5 w-0.5 rounded-full bg-amber-200/80 shadow-[0_0_3px_#fff]"></div>
                <div class="absolute top-[15%] left-[82%] h-1 w-1 rounded-full bg-white/60 shadow-[0_0_4px_#fff]"></div>
                <div class="absolute top-[28%] left-[74%] h-0.5 w-0.5 rounded-full bg-amber-100/60 shadow-[0_0_2px_#fff]"></div>
                <div class="absolute top-[35%] left-[12%] h-0.5 w-0.5 rounded-full bg-white/50"></div>
            </div>

            <div class="pointer-events-none relative z-20 flex h-[420px] w-full flex-col items-center justify-end pb-0 pl-0 md:h-screen md:w-[45%] md:items-start md:pl-6 lg:w-[42%] lg:pl-12">
                <div class="relative h-[460px] w-[320px] sm:h-[580px] sm:w-[400px] md:h-[700px] md:w-[480px] lg:h-[800px] lg:w-[560px] xl:h-[860px] xl:w-[620px]">
                    <LighthouseSvg />
                    <BeamRay beam=beam angle=Signal::derive(angle) desktop=desktop />
                </div>
            </div>

            <div class="relative z-30 flex h-full w-full items-center justify-center p-4 sm:p-6 md:w-[55%] md:p-8 lg:w-[58%]">
                <div
                    class=move || {
                        format!(
                            "relative flex items-center justify-center rounded-full transition-all duration-1000 ease-out {}",
                            if revealed.get() { "opacity-100 scale-100" } else { "opacity-0 scale-100 pointer-events-none" },
                        )
                    }
                    style="width: min(90vw, 520px); height: min(90vw, 520px); background: radial-gradient(circle at center, rgba(255, 252, 240, 0.22) 0%, rgba(254, 243, 199, 0.12) 45%, rgba(217, 180, 74, 0.04) 70%, transparent 100%); box-shadow: 0 0 130px rgba(251, 191, 36, 0.25), inset 0 0 90px rgba(254, 240, 138, 0.18); border: 1.5px solid rgba(253, 230, 138, 0.45);"
                >
                    <div class="pointer-events-none absolute inset-0 animate-pulse rounded-full border border-amber-200/20"></div>
                    <div
                        on:click=|e| e.stop_propagation()
                        class=move || {
                            format!(
                                "w-full max-w-[340px] rounded-2xl border border-amber-200/40 bg-[#080b0a]/90 p-6 text-white shadow-[0_20px_50px_rgba(0,0,0,0.85)] backdrop-blur-2xl transition-all duration-1000 ease-out sm:max-w-[370px] sm:p-7 {}",
                                if revealed.get() { "opacity-100 brightness-100" } else { "opacity-0 brightness-50 pointer-events-none" },
                            )
                        }
                    >
                        <AuthCard />
                    </div>
                </div>
            </div>

            <button
                type="button"
                on:click=move |e| {
                    e.stop_propagation();
                    play();
                }
                title="Replay Lighthouse Beam"
                class="absolute right-4 bottom-4 z-40 flex items-center gap-1.5 rounded-full border border-amber-300/30 bg-black/60 px-3 py-1.5 text-[11px] text-amber-200/80 shadow-md backdrop-blur-md transition-all hover:bg-neutral-900 hover:text-amber-100"
            >
                <svg class="h-3 w-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
                    <path d="M3 3v5h5" />
                </svg>
                <span>"Sweep Light"</span>
            </button>
        </main>
    }
}

/// The beam from the beacon (50%, 29.6% of the lighthouse box), turned to `angle`.
#[component]
fn BeamRay(beam: RwSignal<Beam>, angle: Signal<i32>, desktop: RwSignal<bool>) -> impl IntoView {
    let style = move || {
        let transition = if beam.get() == Beam::Sweeping {
            "transform 1800ms cubic-bezier(0.25, 0.1, 0.25, 1), opacity 400ms ease"
        } else {
            "opacity 500ms ease"
        };
        let opacity = if beam.get() == Beam::Dark { 0 } else { 1 };
        format!(
            "transform: rotate({}deg); transform-origin: 0 0; transition: {transition}; opacity: {opacity};",
            angle.get()
        )
    };
    let size = move || {
        format!(
            "transform: translateY(-50%); width: min(160vw, 2400px); height: {};",
            if desktop.get() { "460px" } else { "360px" }
        )
    };
    view! {
        <div class="pointer-events-none absolute top-[29.6%] left-[50%] z-10" style=style>
            <div class="relative" style=size>
                <div
                    class="absolute inset-0"
                    style="clip-path: polygon(0% 49%, 100% 0%, 100% 100%, 0% 51%); background: linear-gradient(to right, rgba(255,255,255,0.95) 0%, rgba(254,240,138,0.38) 22%, rgba(245,215,130,0.12) 58%, rgba(217,180,74,0.03) 85%, transparent 100%);"
                ></div>
                <div
                    class="absolute inset-0"
                    style="clip-path: polygon(0% 49.6%, 100% 18%, 100% 82%, 0% 50.4%); background: linear-gradient(to right, rgba(255,255,255,1) 0%, rgba(254,243,199,0.65) 12%, rgba(251,191,36,0.18) 50%, transparent 100%);"
                ></div>
            </div>
        </div>
        <div class="pointer-events-none absolute top-[29.6%] left-[50%] z-20 h-20 w-20 -translate-x-1/2 -translate-y-1/2 animate-pulse rounded-full bg-amber-200/40 blur-xl"></div>
        <div class="pointer-events-none absolute top-[29.6%] left-[50%] z-20 h-7 w-7 -translate-x-1/2 -translate-y-1/2 rounded-full bg-white shadow-[0_0_35px_#fde68a]"></div>
    }
}

/// The lighthouse on its cliff. The beacon lens sits at (160, 148) of the 320×500 viewBox.
#[component]
fn LighthouseSvg() -> impl IntoView {
    view! {
        <svg viewBox="0 0 320 500" class="h-full w-full drop-shadow-[0_10px_35px_rgba(0,0,0,0.9)]" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path d="M0 450 C40 435 90 445 140 435 C190 425 240 450 320 440 L320 500 L0 500 Z" fill="#0d110f" />
            <path d="M10 465 C60 450 110 460 160 455 C210 450 260 470 310 460 L320 500 L0 500 Z" fill="#141916" />
            <path d="M30 480 Q100 470 170 482 T310 480 L320 500 L0 500 Z" fill="#19201c" />
            <ellipse cx="160" cy="492" rx="110" ry="4" fill="rgba(245, 215, 130, 0.08)" />
            <ellipse cx="140" cy="496" rx="70" ry="2.5" fill="rgba(245, 215, 130, 0.05)" />
            <polygon points="110,435 210,435 218,455 102,455" fill="#1c221e" stroke="#262f2a" stroke-width="1.5" />
            <path d="M148 435 L148 410 Q160 398 172 410 L172 435 Z" fill="#0a0d0b" stroke="#2b352f" stroke-width="1.5" />
            <circle cx="168" cy="420" r="1.5" fill="#c4aa63" />
            <polygon points="116,435 204,435 198,360 122,360" fill="#181e1b" stroke="#242c27" stroke-width="1" />
            <polygon points="122,360 198,360 193,290 127,290" fill="#c8c2b1" stroke="#ded8c8" stroke-width="1" />
            <path d="M154 335 L154 318 Q160 312 166 318 L166 335 Z" fill="#241d0c" />
            <path d="M155 334 L155 319 Q160 314 165 319 L165 334 Z" fill="#fbbf24" opacity="0.85" />
            <polygon points="127,290 193,290 188,220 132,220" fill="#54655b" stroke="#43524a" stroke-width="1" />
            <path d="M155 260 L155 245 Q160 240 165 245 L165 260 Z" fill="#241d0c" />
            <path d="M156 259 L156 246 Q160 242 164 246 L164 259 Z" fill="#fef08a" opacity="0.9" />
            <polygon points="132,220 188,220 185,185 135,185" fill="#dcd6c5" stroke="#ede7d7" stroke-width="1" />
            <path d="M130 185 L125 178 L195 178 L190 185 Z" fill="#242c27" stroke="#343f38" />
            <rect x="122" y="174" width="76" height="4" rx="1.5" fill="#181e1b" stroke="#3a473f" />
            <line x1="124" y1="165" x2="196" y2="165" stroke="#8a968e" stroke-width="1.5" />
            {[128, 140, 160, 180, 192]
                .map(|x| view! { <line x1=x y1="165" x2=x y2="174" stroke="#8a968e" stroke-width="1" /> })
                .collect_view()}
            <rect x="136" y="125" width="48" height="49" fill="rgba(254, 240, 138, 0.18)" stroke="#242c27" stroke-width="2" />
            <line x1="136" y1="125" x2="184" y2="174" stroke="#242c27" stroke-width="1.2" />
            <line x1="184" y1="125" x2="136" y2="174" stroke="#242c27" stroke-width="1.2" />
            <line x1="160" y1="125" x2="160" y2="174" stroke="#242c27" stroke-width="1.5" />
            <circle cx="160" cy="148" r="14" fill="url(#beacon-glow)" class="animate-pulse" />
            <circle cx="160" cy="148" r="8" fill="#fffdfa" />
            <circle cx="160" cy="148" r="4" fill="#ffffff" />
            <path d="M132 125 C132 100 188 100 188 125 Z" fill="#2c3631" stroke="#1b221f" stroke-width="1.5" />
            <rect x="158" y="90" width="4" height="12" fill="#d5cfc0" />
            <circle cx="160" cy="88" r="3.5" fill="#fef08a" />
            <line x1="160" y1="85" x2="160" y2="72" stroke="#c8c2b1" stroke-width="1.5" />
            <defs>
                <radialGradient id="beacon-glow" cx="50%" cy="50%" r="50%">
                    <stop offset="0%" stop-color="#ffffff" stop-opacity="1" />
                    <stop offset="35%" stop-color="#fef08a" stop-opacity="0.9" />
                    <stop offset="70%" stop-color="#f59e0b" stop-opacity="0.5" />
                    <stop offset="100%" stop-color="#d97706" stop-opacity="0" />
                </radialGradient>
            </defs>
        </svg>
    }
}

const INPUT: &str = "w-full rounded-lg bg-black/60 border border-white/20 px-3 py-2 text-sm text-white placeholder-neutral-500 focus:outline-none focus:border-amber-300 focus:ring-1 focus:ring-amber-300/40 transition-colors";
const LABEL: &str = "block text-xs font-semibold uppercase tracking-wider text-amber-200/80 mb-1";

/// Sign in, create an account (the first one owns the workspace), or reset a password.
#[component]
fn AuthCard() -> impl IntoView {
    let auth = expect_context::<Auth>();
    let mode = RwSignal::new(Mode::SignIn);
    let email = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let full_name = RwSignal::new(String::new());
    let show_password = RwSignal::new(false);
    let busy = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let notice = RwSignal::new(auth.notice.get_untracked());

    let switch = move |to: Mode| {
        mode.set(to);
        error.set(None);
        notice.set(None);
    };

    let submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        busy.set(true);
        error.set(None);
        notice.set(None);
        let (e, p, n, m) =
            (email.get_untracked(), password.get_untracked(), full_name.get_untracked(), mode.get_untracked());
        spawn_local(async move {
            let outcome: Result<Option<String>, String> = match m {
                Mode::Reset => tauri::invoke::<_, ()>("request_password_reset", &json!({ "email": e }))
                    .await
                    // Neutral wording: never reveal whether an email is registered.
                    .map(|()| {
                        Some("If that email has an account, a reset link is on its way. Check your inbox.".into())
                    })
                    .map_err(|e| e.to_string()),
                Mode::SignUp => match tauri::invoke::<_, SignUpOutcome>(
                    "sign_up",
                    &json!({ "email": e, "password": p, "full_name": n }),
                )
                .await
                {
                    Ok(SignUpOutcome::SignedIn) => {
                        auth.reload();
                        Ok(None)
                    }
                    Ok(SignUpOutcome::ConfirmEmail) => {
                        mode.set(Mode::SignIn);
                        Ok(Some("Check your email to confirm the account, then sign in.".into()))
                    }
                    Err(e) => Err(e.to_string()),
                },
                Mode::SignIn => tauri::invoke::<_, SessionInfo>("sign_in", &json!({ "email": e, "password": p }))
                    .await
                    .map(|info| {
                        auth.notice.set(None);
                        auth.apply(info);
                        None
                    })
                    .map_err(|e| e.to_string()),
            };
            busy.set(false);
            match outcome {
                Ok(n) => notice.set(n),
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let heading = move || match mode.get() {
        Mode::SignIn => "Sign in",
        Mode::SignUp => "Create account",
        Mode::Reset => "Reset password",
    };
    let button = move || match (busy.get(), mode.get()) {
        (true, Mode::Reset) => "Sending…",
        (true, _) => "Verifying…",
        (false, Mode::SignIn) => "Sign in to workspace",
        (false, Mode::SignUp) => "Create enterprise account",
        (false, Mode::Reset) => "Send reset link",
    };

    view! {
        <div class="flex flex-col items-center text-center">
            <img
                src="/assets/lattice-lane-lockup.png"
                alt="Lattice Lane, House of Gifting"
                class="mb-1 h-16 w-auto object-contain drop-shadow-[0_2px_12px_rgba(251,191,36,0.3)] sm:h-20"
            />
            <div class="my-1.5 h-0.5 w-12 bg-linear-to-r from-transparent via-amber-300 to-transparent"></div>
            <h1 class="font-display text-xl font-bold tracking-tight text-amber-50">{heading}</h1>
            <p class="mt-0.5 text-xs font-medium text-amber-200/70">"Hamper costing and client quotations"</p>
        </div>

        <form on:submit=submit class="mt-5 space-y-3">
            <Show when=move || mode.get() == Mode::SignUp>
                <div>
                    <label class=LABEL for="fullName">"Full name"</label>
                    <input id="fullName" class=INPUT bind:value=full_name required autocomplete="name" placeholder="E.g. John Doe" />
                </div>
            </Show>
            <div>
                <label class=LABEL for="email">"Email address"</label>
                <input id="email" type="email" class=INPUT bind:value=email required autocomplete="email" placeholder="you@latticelane.com" />
            </div>
            <Show when=move || mode.get() != Mode::Reset>
                <div>
                    <label class=LABEL for="password">"Password"</label>
                    <div class="relative">
                        <input
                            id="password"
                            type=move || if show_password.get() { "text" } else { "password" }
                            class=format!("{INPUT} pr-10")
                            bind:value=password
                            required
                            minlength="8"
                            autocomplete=move || if mode.get() == Mode::SignUp { "new-password" } else { "current-password" }
                            placeholder="••••••••"
                        />
                        <button
                            type="button"
                            tabindex="-1"
                            on:click=move |_| show_password.update(|s| *s = !*s)
                            class="absolute inset-y-0 right-0 flex items-center pr-3 text-amber-200/60 transition-colors hover:text-amber-100 focus:outline-none"
                            aria-label=move || if show_password.get() { "Hide password" } else { "Show password" }
                        >
                            <EyeIcon open=show_password />
                        </button>
                    </div>
                </div>
            </Show>
            {move || error.get().map(|e| view! {
                <div role="alert" class="rounded-lg border border-red-500/50 bg-red-950/80 p-2.5 text-xs text-red-200">{e}</div>
            })}
            {move || notice.get().map(|n| view! {
                <div class="rounded-lg border border-emerald-500/50 bg-emerald-950/80 p-2.5 text-xs text-emerald-200">{n}</div>
            })}
            <button
                type="submit"
                disabled=move || busy.get()
                class="mt-2 w-full rounded-lg bg-linear-to-r from-amber-500 via-amber-400 to-amber-500 px-4 py-2.5 text-sm font-semibold text-black shadow-lg shadow-amber-500/25 transition-all duration-200 hover:from-amber-400 hover:to-amber-300 disabled:opacity-50"
            >
                {button}
            </button>
        </form>

        <div class="mt-4 space-y-2 border-t border-white/10 pt-3 text-center">
            <Show when=move || mode.get() == Mode::SignIn>
                <button
                    type="button"
                    class="block w-full text-xs text-amber-300 transition-colors hover:text-amber-200 hover:underline"
                    on:click=move |_| switch(Mode::Reset)
                >
                    "Forgot password?"
                </button>
            </Show>
            <button
                type="button"
                class="text-xs text-amber-300 transition-colors hover:text-amber-200 hover:underline"
                on:click=move |_| switch(if mode.get_untracked() == Mode::SignIn { Mode::SignUp } else { Mode::SignIn })
            >
                {move || match mode.get() {
                    Mode::SignIn => "First time here? Create an account",
                    Mode::SignUp => "Already have an account? Sign in",
                    Mode::Reset => "Back to sign in",
                }}
            </button>
            <p class="text-[10px] text-neutral-400">"The first account created owns the workspace."</p>
        </div>
    }
}

#[component]
fn EyeIcon(open: RwSignal<bool>) -> impl IntoView {
    view! {
        <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            {move || if open.get() {
                view! {
                    <path d="M10.733 5.076a10.744 10.744 0 0 1 11.205 6.575 1 1 0 0 1 0 .696 10.747 10.747 0 0 1-1.444 2.49" />
                    <path d="M14.084 14.158a3 3 0 0 1-4.242-4.242" />
                    <path d="M17.479 17.499a10.75 10.75 0 0 1-15.417-5.151 1 1 0 0 1 0-.696 10.75 10.75 0 0 1 4.446-5.143" />
                    <path d="m2 2 20 20" />
                }.into_any()
            } else {
                view! {
                    <path d="M2.062 12.348a1 1 0 0 1 0-.696 10.75 10.75 0 0 1 19.876 0 1 1 0 0 1 0 .696 10.75 10.75 0 0 1-19.876 0" />
                    <circle cx="12" cy="12" r="3" />
                }.into_any()
            }}
        </svg>
    }
}
