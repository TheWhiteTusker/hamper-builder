//! One undo/redo history for the window, newest last: ticks, review marks and
//! moves between screens (shortcuts.rs). In memory, so a restart starts fresh.

use lattice_core::undo::History;
use std::cell::RefCell;
use std::rc::Rc;

type Run = Rc<dyn Fn()>;

#[derive(Clone)]
struct Step {
    undo: Run,
    redo: Run,
}

thread_local! {
    static HISTORY: RefCell<History<Step>> = RefCell::new(History::default());
}

/// A new change: undoable, and it ends any redo trail.
pub fn record(undo: impl Fn() + 'static, redo: impl Fn() + 'static) {
    HISTORY.with(|h| h.borrow_mut().record(Step { undo: Rc::new(undo), redo: Rc::new(redo) }));
}

/// Reverses the latest change; false when there's none. The step runs after
/// the history is released, so it may record or navigate freely.
pub fn undo() -> bool {
    let step = HISTORY.with(|h| h.borrow_mut().undo().cloned());
    step.map(|s| (s.undo)()).is_some()
}

/// Re-applies the latest undone change; false when there's none.
pub fn redo() -> bool {
    let step = HISTORY.with(|h| h.borrow_mut().redo().cloned());
    step.map(|s| (s.redo)()).is_some()
}
