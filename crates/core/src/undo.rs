//! An undo/redo history, newest last (port of `undo.ts`). Generic over the
//! step so the UI can keep closures in it and tests can keep names.

const MAX: usize = 100;

pub struct History<S> {
    past: Vec<S>,
    future: Vec<S>,
}

impl<S> Default for History<S> {
    fn default() -> Self {
        Self { past: Vec::new(), future: Vec::new() }
    }
}

impl<S> History<S> {
    /// A new change: undoable, and it ends any redo trail.
    pub fn record(&mut self, step: S) {
        self.past.push(step);
        if self.past.len() > MAX {
            self.past.remove(0);
        }
        self.future.clear();
    }

    /// The latest change to reverse, moved onto the redo trail; None when there's none.
    pub fn undo(&mut self) -> Option<&S> {
        let step = self.past.pop()?;
        self.future.push(step);
        self.future.last()
    }

    /// The latest undone change to re-apply; None when there's none.
    pub fn redo(&mut self) -> Option<&S> {
        let step = self.future.pop()?;
        self.past.push(step);
        self.past.last()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_newest_first_redo_reapplies_a_new_change_ends_the_redo_trail() {
        let mut h = History::default();
        h.record("tick");
        h.record("open product");
        assert_eq!(h.undo(), Some(&"open product"));
        assert_eq!(h.undo(), Some(&"tick"));
        assert_eq!(h.undo(), None, "nothing left: the app falls back to going back a screen");
        assert_eq!(h.redo(), Some(&"tick"));
        h.record("mark");
        assert_eq!(h.redo(), None, "\"open product\" was dropped by the new change");
    }

    #[test]
    fn keeps_the_newest_hundred() {
        let mut h = History::default();
        for i in 0..150 {
            h.record(i);
        }
        let mut n = 0;
        while h.undo().is_some() {
            n += 1;
        }
        assert_eq!(n, MAX);
    }
}
