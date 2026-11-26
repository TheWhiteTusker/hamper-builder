//! Asked before a shortcut changes screen (shortcuts.rs): a screen with unsaved
//! work can hold it (port of `unsaved-warning.ts`).

use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub enum LeaveTo {
    Back,
    Forward,
    Path(String),
}

type Guard = Rc<dyn Fn(&LeaveTo) -> bool>;

thread_local! {
    static GUARDS: RefCell<Vec<(u64, Guard)>> = const { RefCell::new(Vec::new()) };
    static NEXT: RefCell<u64> = const { RefCell::new(0) };
}

/// True when no screen objects to leaving. Every guard is asked, as every
/// listener of a cancelable event would be.
pub fn may_leave(to: &LeaveTo) -> bool {
    let guards: Vec<Guard> = GUARDS.with(|g| g.borrow().iter().map(|(_, f)| f.clone()).collect());
    let mut ok = true;
    for f in &guards {
        ok &= f(to);
    }
    ok
}

/// While the calling component lives, `allow` decides whether a shortcut may
/// leave its screen: false holds it there.
pub fn guard(allow: impl Fn(&LeaveTo) -> bool + 'static) {
    let id = NEXT.with(|n| {
        *n.borrow_mut() += 1;
        *n.borrow()
    });
    GUARDS.with(|g| g.borrow_mut().push((id, Rc::new(allow))));
    on_cleanup(move || GUARDS.with(|g| g.borrow_mut().retain(|(i, _)| *i != id)));
}

/// While `dirty`: confirm before a shortcut leaves the screen.
pub fn warn_unsaved(dirty: impl Fn() -> bool + 'static) {
    guard(move |_| {
        !dirty() || window().confirm_with_message("Leave without saving? Your changes will be lost.").unwrap_or(false)
    });
}
