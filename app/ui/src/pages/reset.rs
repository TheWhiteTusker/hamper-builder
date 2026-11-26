//! After a reset link signs the person in: choose a new password
//! (port of `reset-password/reset-form.tsx`).

use crate::state::Auth;
use crate::tauri;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::json;

#[component]
pub fn ResetPassword() -> impl IntoView {
    let auth = expect_context::<Auth>();
    let password = RwSignal::new(String::new());
    let confirm = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    let submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let (p, c) = (password.get_untracked(), confirm.get_untracked());
        if p != c {
            error.set(Some("The two passwords don't match.".into()));
            return;
        }
        busy.set(true);
        error.set(None);
        spawn_local(async move {
            match tauri::invoke::<_, ()>("update_password", &json!({ "password": p })).await {
                Ok(()) => auth.reload(),
                Err(e) => error.set(Some(e.to_string())),
            }
            busy.set(false);
        });
    };

    view! {
        <main class="flex min-h-screen items-center justify-center bg-black px-4 text-white">
            <form on:submit=submit class="w-full max-w-sm space-y-3 rounded-2xl border border-amber-200/40 bg-[#080b0a] p-7">
                <h1 class="font-display text-xl font-bold text-amber-50">"Choose a new password"</h1>
                <p class="text-xs text-amber-200/70">"You're signed in from the reset link. Set a new password to continue."</p>
                <input type="password" class="input bg-black/60 text-white" placeholder="New password" bind:value=password required minlength="8" autocomplete="new-password" />
                <input type="password" class="input bg-black/60 text-white" placeholder="Repeat it" bind:value=confirm required minlength="8" autocomplete="new-password" />
                {move || error.get().map(|e| view! {
                    <div role="alert" class="rounded-lg border border-red-500/50 bg-red-950/80 p-2.5 text-xs text-red-200">{e}</div>
                })}
                <button type="submit" disabled=move || busy.get() class="w-full rounded-lg bg-amber-400 py-2.5 text-sm font-semibold text-black disabled:opacity-50">
                    {move || if busy.get() { "Saving…" } else { "Save password" }}
                </button>
            </form>
        </main>
    }
}
