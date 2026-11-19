//! Undo/redo for the designer (port of `canvas-history.ts`).

const LIMIT: usize = 100;
const GROUP_MS: f64 = 1000.0;

#[derive(Clone, Debug, PartialEq)]
pub struct History<T> {
    pub past: Vec<T>,
    pub present: T,
    pub future: Vec<T>,
    /// Consecutive edits with the same group within a second become one step.
    group: Option<String>,
    at: f64,
}

impl<T: Clone + PartialEq> History<T> {
    pub fn new(present: T) -> Self {
        Self { past: Vec::new(), present, future: Vec::new(), group: None, at: 0.0 }
    }

    /// Records a new present. Pass a group for continuous edits (a slider, a
    /// colour picker, typing) so dragging one control is one undo step, not fifty.
    pub fn record(&mut self, next: T, group: Option<&str>, now_ms: f64) {
        if next == self.present {
            return;
        }
        if let Some(g) = group
            && self.group.as_deref() == Some(g)
            && now_ms - self.at < GROUP_MS
        {
            self.present = next;
            self.future.clear();
            self.at = now_ms;
            return;
        }
        let prev = std::mem::replace(&mut self.present, next);
        self.past.push(prev);
        if self.past.len() > LIMIT {
            self.past.remove(0);
        }
        self.future.clear();
        self.group = group.map(str::to_string);
        self.at = now_ms;
    }

    pub fn undo(&mut self) {
        if let Some(prev) = self.past.pop() {
            let cur = std::mem::replace(&mut self.present, prev);
            self.future.insert(0, cur);
            self.group = None;
            self.at = 0.0;
        }
    }

    pub fn redo(&mut self) {
        if !self.future.is_empty() {
            let next = self.future.remove(0);
            let cur = std::mem::replace(&mut self.present, next);
            self.past.push(cur);
            self.group = None;
            self.at = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_redo_grouping_and_redo_cleared() {
        let mut h = History::new(0);
        h.record(1, None, 0.0);
        h.record(2, Some("slider"), 100.0);
        h.record(3, Some("slider"), 200.0);
        assert_eq!((h.past.clone(), h.present), (vec![0, 1], 3));
        h.record(4, Some("slider"), 5000.0);
        assert_eq!(h.past, vec![0, 1, 3]);
        h.undo();
        h.undo();
        assert_eq!(h.present, 1);
        assert_eq!(h.future, vec![3, 4]);
        h.redo();
        assert_eq!(h.present, 3);
        h.record(9, None, 6000.0);
        assert!(h.future.is_empty());
        let before = h.clone();
        h.record(9, None, 7000.0);
        assert_eq!(h, before);
        let mut fresh = History::new(0);
        fresh.undo();
        assert_eq!(fresh.present, 0);
    }
}
