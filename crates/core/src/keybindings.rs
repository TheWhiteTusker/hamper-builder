//! Rebindable shortcuts (Settings → Key bindings), like a game's controls
//! screen: each action has a key and an alternate, either a key combination or
//! a mouse button. Kept per device. Port of `keybindings.ts`.

use std::collections::BTreeMap;

pub struct Action {
    pub id: &'static str,
    pub label: &'static str,
    pub group: &'static str,
    pub defaults: [&'static str; 2],
    pub href: Option<&'static str>,
}

const fn act(id: &'static str, label: &'static str, group: &'static str, defaults: [&'static str; 2]) -> Action {
    Action { id, label, group, defaults, href: None }
}

const fn go(id: &'static str, label: &'static str, key: &'static str, href: &'static str) -> Action {
    Action { id, label, group: "Go to", defaults: [key, ""], href: Some(href) }
}

pub const ACTIONS: [Action; 14] = [
    act("undo", "Undo", "Editing", ["Ctrl+Z", ""]),
    act("redo", "Redo", "Editing", ["Ctrl+Y", "Ctrl+Shift+Z"]),
    act("back", "Previous screen", "Moving around", ["Mouse4", ""]),
    act("forward", "Next screen", "Moving around", ["Mouse5", ""]),
    act("search", "Search this list", "Moving around", ["/", ""]),
    go("dashboard", "Dashboard", "D", "/dashboard"),
    go("hampers", "Hampers", "H", "/hampers"),
    go("quotes", "Quotes", "Q", "/quotes"),
    go("clients", "Clients", "", "/clients"),
    go("presentations", "Presentations", "", "/presentations"),
    go("products", "Products", "P", "/products"),
    go("newProduct", "New product", "N", "/products/new"),
    go("calculator", "Cost calculator", "C", "/cost-calculator"),
    go("keys", "Key bindings", "Shift+?", "/settings?tab=keys"),
];

/// Action id → [key, alternate]; "" is an empty slot.
pub type Bindings = BTreeMap<String, [String; 2]>;

const MOD_KEYS: [&str; 5] = ["Control", "Shift", "Alt", "Meta", "AltGraph"];

/// "Ctrl+Shift+Z", "P", "Shift+?"; None for a modifier on its own. ⌘ counts as Ctrl.
pub fn combo_from_key(key: &str, ctrl: bool, meta: bool, alt: bool, shift: bool) -> Option<String> {
    if MOD_KEYS.contains(&key) {
        return None;
    }
    let key = if key == " " {
        "Space".to_string()
    } else if key.chars().count() == 1 {
        key.to_uppercase()
    } else {
        key.to_string()
    };
    let parts = [(ctrl || meta).then_some("Ctrl"), alt.then_some("Alt"), shift.then_some("Shift")];
    let mut out: Vec<&str> = parts.into_iter().flatten().collect();
    out.push(&key);
    Some(out.join("+"))
}

/// The bindable mouse buttons: middle, and the two side buttons. Left and right stay normal clicks.
pub fn combo_from_mouse(button: i16) -> Option<&'static str> {
    match button {
        1 => Some("MiddleClick"),
        3 => Some("Mouse4"),
        4 => Some("Mouse5"),
        _ => None,
    }
}

pub fn combo_label(combo: &str) -> &str {
    match combo {
        "MiddleClick" => "Middle click",
        "Mouse4" => "Mouse back button",
        "Mouse5" => "Mouse forward button",
        c => c,
    }
}

/// Saved changes over the defaults.
pub fn merge(saved: &Bindings) -> Bindings {
    ACTIONS
        .iter()
        .map(|a| {
            let slots = saved.get(a.id).cloned().unwrap_or_else(|| a.defaults.map(str::to_string));
            (a.id.to_string(), slots)
        })
        .collect()
}

pub fn defaults() -> Bindings {
    merge(&Bindings::new())
}

pub fn action_for<'a>(bindings: &Bindings, combo: Option<&str>) -> Option<&'a Action> {
    let combo = combo?;
    ACTIONS.iter().find(|a| bindings.get(a.id).is_some_and(|s| s.iter().any(|c| c == combo)))
}

/// Puts `combo` in one slot ("" clears it). Like a game, a combo already used
/// elsewhere moves here: the old slot is emptied, and its action returned.
pub fn assign(bindings: &Bindings, id: &str, slot: usize, combo: &str) -> (Bindings, Option<&'static str>) {
    let mut next = bindings.clone();
    let mut moved_from = None;
    if !combo.is_empty() {
        for a in &ACTIONS {
            for s in next.get_mut(a.id).into_iter().flat_map(|s| s.iter_mut()).filter(|s| *s == combo) {
                s.clear();
                if a.id != id {
                    moved_from = Some(a.id);
                }
            }
        }
    }
    if let Some(s) = next.get_mut(id).and_then(|s| s.get_mut(slot)) {
        *s = combo.to_string();
    }
    (next, moved_from)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(k: &str, ctrl: bool, meta: bool, shift: bool) -> Option<String> {
        combo_from_key(k, ctrl, meta, false, shift)
    }

    #[test]
    fn key_presses_become_combos() {
        assert_eq!(key("z", true, false, false).as_deref(), Some("Ctrl+Z"));
        assert_eq!(key("z", false, true, false).as_deref(), Some("Ctrl+Z"), "⌘ counts as Ctrl");
        assert_eq!(key("Z", true, false, true).as_deref(), Some("Ctrl+Shift+Z"));
        assert_eq!(key("?", false, false, true).as_deref(), Some("Shift+?"));
        assert_eq!(key("Shift", false, false, true), None, "a modifier alone isn't a combo");
        assert_eq!(combo_from_mouse(3), Some("Mouse4"));
        assert_eq!(combo_from_mouse(0), None, "left click stays a click");
    }

    #[test]
    fn defaults_find_their_actions() {
        let d = defaults();
        let id = |c| action_for(&d, Some(c)).map(|a| a.id);
        assert_eq!(id("Ctrl+Z"), Some("undo"));
        assert_eq!(id("Ctrl+Shift+Z"), Some("redo"));
        assert_eq!(id("P"), Some("products"));
        assert_eq!(id("X"), None);
    }

    #[test]
    fn a_combo_used_elsewhere_moves_to_the_new_slot() {
        let d = defaults();
        let (b, moved) = assign(&d, "clients", 0, "P");
        assert_eq!(moved, Some("products"));
        assert_eq!(b["products"], [String::new(), String::new()]);
        assert_eq!(action_for(&b, Some("P")).map(|a| a.id), Some("clients"));
        assert_eq!(d["products"], ["P".to_string(), String::new()], "the defaults are untouched");

        let (same, moved) = assign(&d, "redo", 0, "Ctrl+Shift+Z");
        assert_eq!(moved, None, "from its own alternate");
        assert_eq!(same["redo"], ["Ctrl+Shift+Z".to_string(), String::new()]);
    }
}
