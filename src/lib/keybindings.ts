// Rebindable shortcuts (Settings → Key bindings), like a game's controls
// screen: each action has a key and an alternate, either a key combination
// or a mouse button. Kept per device (this browser, or the desktop app).

export type ActionId =
  | "undo" | "redo" | "back" | "forward" | "search"
  | "dashboard" | "hampers" | "quotes" | "clients" | "presentations" | "products" | "newProduct" | "calculator"
  | "keys";

export type Slots = [string, string];

export const ACTIONS: { id: ActionId; label: string; group: string; defaults: Slots; href?: string }[] = [
  { id: "undo", label: "Undo", group: "Editing", defaults: ["Ctrl+Z", ""] },
  { id: "redo", label: "Redo", group: "Editing", defaults: ["Ctrl+Y", "Ctrl+Shift+Z"] },
  { id: "back", label: "Previous screen", group: "Moving around", defaults: ["Mouse4", ""] },
  { id: "forward", label: "Next screen", group: "Moving around", defaults: ["Mouse5", ""] },
  { id: "search", label: "Search this list", group: "Moving around", defaults: ["/", ""] },
  { id: "dashboard", label: "Dashboard", group: "Go to", defaults: ["D", ""], href: "/dashboard" },
  { id: "hampers", label: "Hampers", group: "Go to", defaults: ["H", ""], href: "/hampers" },
  { id: "quotes", label: "Quotes", group: "Go to", defaults: ["Q", ""], href: "/quotes" },
  { id: "clients", label: "Clients", group: "Go to", defaults: ["", ""], href: "/clients" },
  { id: "presentations", label: "Presentations", group: "Go to", defaults: ["", ""], href: "/presentations" },
  { id: "products", label: "Products", group: "Go to", defaults: ["P", ""], href: "/products" },
  { id: "newProduct", label: "New product", group: "Go to", defaults: ["N", ""], href: "/products/new" },
  { id: "calculator", label: "Cost calculator", group: "Go to", defaults: ["C", ""], href: "/cost-calculator" },
  { id: "keys", label: "Key bindings", group: "Go to", defaults: ["Shift+?", ""], href: "/settings?tab=keys" },
];

const MOD_KEYS = ["Control", "Shift", "Alt", "Meta", "AltGraph"];

/** "Ctrl+Shift+Z", "P", "Shift+?"; null for a modifier on its own. ⌘ counts as Ctrl. */
export function comboFromKey(e: Pick<KeyboardEvent, "key" | "ctrlKey" | "metaKey" | "altKey" | "shiftKey">): string | null {
  if (MOD_KEYS.includes(e.key)) return null;
  const key = e.key === " " ? "Space" : e.key.length === 1 ? e.key.toUpperCase() : e.key;
  return [e.ctrlKey || e.metaKey ? "Ctrl" : "", e.altKey ? "Alt" : "", e.shiftKey ? "Shift" : "", key]
    .filter(Boolean)
    .join("+");
}

const MOUSE: Record<number, string> = { 1: "MiddleClick", 3: "Mouse4", 4: "Mouse5" };

/** The bindable mouse buttons: middle, and the two side buttons. Left and right stay normal clicks. */
export const comboFromMouse = (button: number): string | null => MOUSE[button] ?? null;

const LABEL: Record<string, string> = {
  MiddleClick: "Middle click",
  Mouse4: "Mouse back button",
  Mouse5: "Mouse forward button",
};

export const comboLabel = (combo: string) => LABEL[combo] ?? combo;

export type Bindings = Record<ActionId, Slots>;

/** Saved changes over the defaults. */
export function merge(saved: Partial<Bindings>): Bindings {
  return Object.fromEntries(ACTIONS.map((a) => [a.id, saved[a.id] ?? a.defaults])) as Bindings;
}

export function actionFor(bindings: Bindings, combo: string | null): ActionId | null {
  if (!combo) return null;
  return ACTIONS.find((a) => bindings[a.id].includes(combo))?.id ?? null;
}

/**
 * Puts `combo` in one slot ("" clears it). Like a game, a combo already used
 * elsewhere moves here: the old slot is emptied, and its action returned.
 */
export function assign(bindings: Bindings, id: ActionId, slot: 0 | 1, combo: string) {
  const next = structuredClone(bindings);
  let movedFrom: ActionId | null = null;
  if (combo) {
    for (const a of ACTIONS) {
      for (const i of [0, 1] as const) {
        if (next[a.id][i] !== combo) continue;
        next[a.id][i] = "";
        if (a.id !== id) movedFrom = a.id;
      }
    }
  }
  next[id][slot] = combo;
  return { bindings: next, movedFrom };
}

// --- This device's bindings ---

const KEY = "lattice:keybindings";
const EVENT = "lattice:keybindings";
let cache: { raw: string | null; bindings: Bindings } | null = null;

export function loadBindings(): Bindings {
  let raw: string | null = null;
  try {
    raw = localStorage.getItem(KEY);
  } catch {}
  if (cache?.raw === raw) return cache.bindings;
  let saved: Partial<Bindings> = {};
  try {
    saved = raw ? JSON.parse(raw) : {};
  } catch {}
  cache = { raw, bindings: merge(saved) };
  return cache.bindings;
}

export function saveBindings(bindings: Bindings | null) {
  try {
    if (bindings) localStorage.setItem(KEY, JSON.stringify(bindings));
    else localStorage.removeItem(KEY); // back to the defaults
  } catch {}
  window.dispatchEvent(new Event(EVENT));
}

export function subscribeBindings(cb: () => void) {
  window.addEventListener(EVENT, cb);
  window.addEventListener("storage", cb); // other tabs
  return () => {
    window.removeEventListener(EVENT, cb);
    window.removeEventListener("storage", cb);
  };
}

export const DEFAULT_BINDINGS = merge({});
