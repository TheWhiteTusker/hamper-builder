//! Live updates (port of `live-refresh.tsx`): someone saves anywhere, and the
//! screen re-fetches in the background, as it does when the window regains
//! focus. Changes within 300 ms fold into one refresh, and it waits while the
//! window is hidden or a text field has focus, so it never lands mid-typing.
//! Your own saves don't refresh: the saving screen already shows them.
//!
//! ponytail: screens re-fetch rather than patch rows in place as the Next app
//! did, so a change shows in ~0.5 s rather than ~0.2 s. Patch rows from
//! `LiveChange.record` if that gap is ever noticed.

use crate::state::Freshness;
use crate::tauri;
use lattice_core::live::{self, Incoming};
use leptos::prelude::*;
use serde::Deserialize;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::WebSocket;

const TYPING: &str =
    "input:not([type=checkbox],[type=radio],[type=button],[type=submit]), textarea, [contenteditable=true]";
const HEARTBEAT: Duration = Duration::from_secs(25);
/// Well inside the access token's hour.
const NEW_TOKEN: Duration = Duration::from_secs(20 * 60);

#[derive(Deserialize)]
struct RealtimeAuth {
    url: String,
    token: String,
}

type Handler = Closure<dyn FnMut(web_sys::Event)>;

struct Live {
    user: String,
    tick: RwSignal<u32>,
    ws: RefCell<Option<WebSocket>>,
    handlers: RefCell<Vec<Handler>>,
    timers: RefCell<Vec<IntervalHandle>>,
    msg_ref: Cell<u64>,
    /// Joined once already: a rejoin (after sleep, a dropped line) may have missed changes.
    joined: Cell<bool>,
    pending: Cell<bool>,
    fold: Cell<u32>,
    retries: Cell<u32>,
    stopped: Cell<bool>,
}

impl Live {
    fn next_ref(&self) -> u64 {
        self.msg_ref.set(self.msg_ref.get() + 1);
        self.msg_ref.get()
    }

    fn send(&self, text: &str) {
        if let Some(ws) = self.ws.borrow().as_ref().filter(|ws| ws.ready_state() == WebSocket::OPEN) {
            let _ = ws.send_with_str(text);
        }
    }

    fn flush(&self) {
        let typing = document().active_element().is_some_and(|el| el.matches(TYPING).unwrap_or(false));
        if self.stopped.get() || !self.pending.get() || document().hidden() || typing {
            return;
        }
        self.pending.set(false);
        self.tick.update(|n| *n += 1);
    }

    fn changed(self: &Rc<Self>) {
        self.pending.set(true);
        let fold = self.fold.get() + 1;
        self.fold.set(fold);
        let me = self.clone();
        set_timeout(
            move || {
                if me.fold.get() == fold {
                    me.flush()
                }
            },
            Duration::from_millis(300),
        );
    }

    fn heard(self: &Rc<Self>, text: &str) {
        match live::parse(text) {
            Incoming::Change(c) if live::is_others_change(&c, &self.user) => self.changed(),
            Incoming::Joined => {
                self.retries.set(0);
                if self.joined.replace(true) {
                    self.changed();
                }
            }
            Incoming::Failed(why) => {
                leptos::logging::warn!("live updates: {why}");
                self.drop_socket();
                self.retry_later();
            }
            _ => {}
        }
    }

    fn drop_socket(&self) {
        if let Some(ws) = self.ws.borrow_mut().take() {
            ws.set_onopen(None);
            ws.set_onmessage(None);
            ws.set_onclose(None);
            let _ = ws.close();
        }
        self.handlers.borrow_mut().clear();
    }

    /// 1 s, 2 s, 4 s … up to 30 s between tries.
    fn retry_later(self: &Rc<Self>) {
        let n = self.retries.get();
        self.retries.set(n + 1);
        let me = self.clone();
        set_timeout(move || me.connect(), Duration::from_secs((1u64 << n.min(5)).min(30)));
    }

    fn connect(self: &Rc<Self>) {
        if self.stopped.get() {
            return;
        }
        let me = self.clone();
        leptos::task::spawn_local(async move {
            let auth = tauri::invoke::<_, RealtimeAuth>("realtime_auth", &serde_json::json!({})).await;
            let (Ok(auth), false) = (auth, me.stopped.get()) else {
                if !me.stopped.get() {
                    me.retry_later();
                }
                return;
            };
            let Ok(ws) = WebSocket::new(&auth.url) else { return me.retry_later() };
            let (a, b, c) = (me.clone(), me.clone(), me.clone());
            let token = auth.token;
            let on_open: Handler = Closure::new(move |_| a.send(&live::join(&token, a.next_ref())));
            let on_message: Handler = Closure::new(move |e: web_sys::Event| {
                if let Some(text) = e.unchecked_into::<web_sys::MessageEvent>().data().as_string() {
                    b.heard(&text);
                }
            });
            let on_close: Handler = Closure::new(move |_| {
                c.drop_socket();
                if !c.stopped.get() {
                    c.retry_later();
                }
            });
            ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));
            ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
            ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));
            me.drop_socket();
            *me.ws.borrow_mut() = Some(ws);
            *me.handlers.borrow_mut() = vec![on_open, on_message, on_close];
        });
    }

    fn stop(&self) {
        self.stopped.set(true);
        self.drop_socket();
        for t in self.timers.borrow_mut().drain(..) {
            t.clear();
        }
    }
}

/// Live updates for as long as the calling component (the signed-in shell) lives.
pub fn start(user: String) {
    let me = Rc::new(Live {
        user,
        tick: expect_context::<Freshness>().0,
        ws: RefCell::new(None),
        handlers: RefCell::new(Vec::new()),
        timers: RefCell::new(Vec::new()),
        msg_ref: Cell::new(0),
        joined: Cell::new(false),
        pending: Cell::new(false),
        fold: Cell::new(0),
        retries: Cell::new(0),
        stopped: Cell::new(false),
    });
    me.connect();

    let (a, b) = (me.clone(), me.clone());
    let timers = [
        set_interval_with_handle(move || a.send(&live::heartbeat(a.next_ref())), HEARTBEAT),
        set_interval_with_handle(
            move || {
                let b = b.clone();
                leptos::task::spawn_local(async move {
                    if let Ok(auth) = tauri::invoke::<_, RealtimeAuth>("realtime_auth", &serde_json::json!({})).await {
                        b.send(&live::access_token(&auth.token, b.next_ref()));
                    }
                });
            },
            NEW_TOKEN,
        ),
    ];
    me.timers.borrow_mut().extend(timers.into_iter().flatten());

    // Back to the window, or focus left a text field (it has moved by the next tick).
    let (c, d) = (me.clone(), me.clone());
    let shown = window_event_listener_untyped("visibilitychange", move |_| c.flush());
    let left = window_event_listener_untyped("focusout", move |_| {
        let d = d.clone();
        set_timeout(move || d.flush(), Duration::ZERO);
    });
    let me = StoredValue::new_local(me);
    on_cleanup(move || {
        shown.remove();
        left.remove();
        me.with_value(|m| m.stop());
    });
}
