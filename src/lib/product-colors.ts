/** Product colours, their two-letter codes and swatches. */

export const STANDARD_PRODUCT_COLORS = [
  { name: "Walnut", code: "WL", hex: "#5c4033" },
  { name: "Natural", code: "NT", hex: "#e4cdad" },
  { name: "Black", code: "BL", hex: "#222222" },
] as const;

export type StandardColorName = (typeof STANDARD_PRODUCT_COLORS)[number]["name"];
export type StandardColorCode = (typeof STANDARD_PRODUCT_COLORS)[number]["code"];

export const COLOR_TO_CODE: Record<string, StandardColorCode> = {
  walnut: "WL",
  natural: "NT",
  black: "BL",
  wl: "WL",
  nt: "NT",
  bl: "BL",
  "walnut finish": "WL",
  "natural finish": "NT",
  "black finish": "BL",
};

export const CODE_TO_COLOR: Record<StandardColorCode, StandardColorName> = {
  WL: "Walnut",
  NT: "Natural",
  BL: "Black",
};

const STANDARD_CODES: readonly string[] = STANDARD_PRODUCT_COLORS.map((c) => c.code);

/**
 * Codes pinned by hand where the rule below would clash. Black Leather and
 * Brown Leather both derive BR; Black Leather products already use BR.
 */
const FIXED_CODES: Record<string, string> = { "black leather": "BR", "brown leather": "BN" };

/** The colour in `names` whose code another one already has, or null. Codes must be unique. */
export function codeClash(names: string[]): { name: string; code: string; with: string } | null {
  const byCode = new Map<string, string>();
  for (const name of names) {
    const code = colorCode(name);
    const other = byCode.get(code);
    if (other) return { name, code, with: other };
    byCode.set(code, name);
  }
  return null;
}

/**
 * Two-letter code for any colour: the standard three keep WL/NT/BL, anything
 * added in Settings gets its initials ("Dark Oak" -> DO, "Teak" -> TE). A
 * clash with a standard code falls back to first + last letter ("Blue" -> BE).
 */
export function colorCode(name: string): string {
  const key = (name || "").trim().toLowerCase();
  if (COLOR_TO_CODE[key]) return COLOR_TO_CODE[key];
  if (FIXED_CODES[key]) return FIXED_CODES[key];
  const code = deriveCategoryCode(name);
  if (!STANDARD_CODES.includes(code)) return code;
  const letters = key.replace(/[^a-z0-9]/g, "");
  return (letters[0] + letters[letters.length - 1]).toUpperCase();
}

export type ProductColor = { name: string; code: string; hex: string };

/** Swatch for a colour that has no picked hex yet. */
export const FALLBACK_COLOR_HEX = "#94a3b8";

/**
 * Colours from Settings with code and swatch. The hex picked in the Master
 * wins, then the standard swatch, then neutral grey.
 */
export function resolveColors(
  names: string[],
  hexByName: Record<string, string> = {},
): ProductColor[] {
  return names.map((name) => ({
    name,
    code: colorCode(name),
    hex:
      hexByName[name] ??
      STANDARD_PRODUCT_COLORS.find((c) => c.name.toLowerCase() === name.toLowerCase())?.hex ??
      FALLBACK_COLOR_HEX,
  }));
}

/**
 * Derive 2-letter uppercase category code from category name.
 * e.g. "Lights & Candles" -> "LC", "Box & Packaging" -> "BP", "Decor" -> "DC"
 */
export function deriveCategoryCode(name: string): string {
  if (!name) return "XX";
  const words = name
    .trim()
    .replace(/[&/\\#,+()$~%.'":*?<>{}]/g, " ")
    .split(/\s+/)
    .filter(Boolean);

  if (words.length >= 2) {
    return (words[0][0] + words[1][0]).toUpperCase();
  }
  const clean = words[0].replace(/[^A-Za-z0-9]/g, "");
  return clean.slice(0, 2).toUpperCase().padEnd(2, "X");
}

/** An image's colour tag from a picked name or code: standard colours get their full name. */
export function colorTag(raw: string | null | undefined) {
  const code = raw ? colorCode(raw) : null;
  return { color: (code && (CODE_TO_COLOR as Record<string, string>)[code]) || raw || null, color_code: code };
}
