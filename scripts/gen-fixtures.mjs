// Runs the old app's TypeScript pricing/costing on thousands of generated
// inputs and writes input → output pairs to scripts/fixtures/parity.json.
// crates/core/tests/parity.rs replays them against the Rust port.
//
//   node scripts/gen-fixtures.mjs [path to old repo]
//
// Needs Node 23.6+ (runs .ts directly). Seeded, so reruns are identical.

import { writeFileSync, mkdirSync } from "node:fs";
import { pathToFileURL, fileURLToPath } from "node:url";
import { join, dirname } from "node:path";

const OLD = process.argv[2] ?? "D:/Github/lattice lane hamper builder";
const lib = (f) => import(pathToFileURL(join(OLD, "src/lib", f)).href);

const pricing = await lib("pricing.ts");
const costing = await lib("costing.ts");
const mrp = await lib("mrp.ts");
const codes = await lib("product-code.ts");

// mulberry32: small, seeded, good enough for test data.
let seed = 0x1a77ce;
const rand = () => {
  seed |= 0;
  seed = (seed + 0x6d2b79f5) | 0;
  let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
  t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
  return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
};
const pick = (xs) => xs[Math.floor(rand() * xs.length)];
const int = (lo, hi) => lo + Math.floor(rand() * (hi - lo + 1));
/** A figure as a person types it: whole, or to 1–2 decimals. */
const money = (hi) => Number((rand() * hi).toFixed(pick([0, 0, 1, 2])));
const pct = () => pick([0, 5, 10, 12.5, 18, 25, 33, 35, 40, 50, 55, 60, 75, money(99)]);

const STAGES = ["material", "hardware", "finishing", "machine", "miscellaneous", "bought_out"];
const UNITS = ["sq ft", "sqft", "sq inch", "sq mm", "sq cm", "feet", "running ft", "inch", "mm", "cm", "meter", "piece", "set", "kg", "min", "hour", ""];

function costLine() {
  const stage = pick(STAGES);
  const l = {
    stage_code: stage,
    unit: stage === "machine" ? pick(["min", "hour", "", "sq inch"]) : pick(UNITS),
    rate: money(500),
    qty: pick([1, 1, 2, 3, 4, 0, 0.5, null]),
    wastage_pct: pick([0, 0, 5, 10, 15, 2.5]),
    length: pick([null, int(1, 48), money(60)]),
    breadth: pick([null, int(1, 48), money(60)]),
    dimension_unit: pick(["inch", "mm", "cm", ""]),
    duration_minutes: pick([null, int(1, 90), money(45)]),
  };
  if (stage === "bought_out" && rand() < 0.5) l.mrp = int(50, 2000);
  return l;
}

const cases = { hamper: [], quote: [], line: [], sheet: [], mrp: [], round_sp: [], scaled: [], margin: [], words: [], code: [] };

for (let i = 0; i < 1500; i++) {
  const lines = Array.from({ length: int(0, 8) }, () => ({
    qty: int(0, 6),
    unitCp: money(900),
    unitSp: money(1500),
    countsAsItem: rand() > 0.2,
  }));
  const opts = {
    discountPct: pick([0, 0.05, 0.1, 0.15, 0.125]),
    targetSp: rand() < 0.5 ? null : money(5000),
    finalCatalogueSp: rand() < 0.3 ? null : pick([0, money(6000)]),
  };
  cases.hamper.push({ lines, opts, out: pricing.priceHamper(lines, opts) });
}

for (let i = 0; i < 1500; i++) {
  const lines = Array.from({ length: int(0, 6) }, () => ({
    qty: int(1, 500),
    cataloguePrice: money(5000),
    discountPct: pick([0, 0, 0.05, 0.1, 0.075]),
  }));
  const opts = {
    quoteStructure: pick(["Combined Order", "Option Based"]),
    orderDiscount: pick([0, money(2000)]),
    adj1: pick([0, money(1500)]),
    adj2: pick([0, money(500)]),
    gstRate: pick([0.18, 0.12, 0.05, 0.28, 0]),
  };
  cases.quote.push({ lines, opts, out: pricing.priceQuote(lines, opts) });
}

for (let i = 0; i < 3000; i++) {
  const line = costLine();
  cases.line.push({ line, out: costing.calculateLineCost(line) });
}

for (let i = 0; i < 1500; i++) {
  const lines = Array.from({ length: int(1, 8) }, costLine);
  const markup = pct();
  const overheads = Object.fromEntries(STAGES.filter(() => rand() < 0.3).map((s) => [s, pick([5, 10, 12.5, 20])]));
  const totals = costing.calculateCostSheetTotals(lines, markup, overheads);
  const gst = pick([0.18, 0.12, 0]);
  const limit = mrp.mrpLimit(lines, overheads, totals.total_cost, gst);
  cases.sheet.push({ lines, markup, overheads, out: totals });
  cases.mrp.push({ lines, overheads, total_cost: totals.total_cost, gst, out: limit });
}

for (let i = 0; i < 1000; i++) {
  const sp = money(3000);
  const limit = rand() < 0.3 ? null : int(1, 3000);
  cases.round_sp.push({ sp, limit, out: mrp.roundSp(sp, limit) });
  const [oc, os, nc, fb] = [pick([0, money(800)]), pick([0, money(2000)]), money(900), money(2000)];
  cases.scaled.push({ old_cost: oc, old_sp: os, new_cost: nc, fallback: fb, out: costing.scaledPrice(oc, os, nc, fb) });
  const [cp, m, s] = [money(900), pct(), money(2000)];
  cases.margin.push({ cp, margin: m, sp: s, sp_at: pricing.spAtMargin(cp, m), pct_of: pricing.marginPctOf(cp, s) });
  const amt = pick([money(1000), money(100000), money(99999999), int(0, 1e9)]);
  cases.words.push({ amount: amt, out: pricing.amountInWords(amt) });
}

const COLORS = ["Walnut", "natural", "BL", "Brown Paper", "Teak", "Blue", "Black Leather", "Brown Leather", "Gold", "Dark Oak", "  wl ", ""];
const RAW = ["LC/0001/WL", "lc/12/walnut", "LC010", "ED/0025/NT", "AB/12345/XYZ", "??", "", "X1/0002/BP", "DC0099"];
for (let i = 0; i < 300; i++) {
  const raw = pick(RAW);
  const color = pick(COLORS);
  const cat = pick(["LC", "ed", "", "A&B", "Lights & Candles"]);
  const serial = pick(["1", "0123", "42", "", "0", "abc7"]);
  cases.code.push({
    raw,
    color,
    cat,
    serial,
    parsed: codes.parseProductCode(raw),
    formatted: codes.formatProductCode(cat, serial, color),
    color_code: color.trim() ? codes.colorCode(color) : null,
    derived: cat.trim() ? codes.deriveCategoryCode(cat) : null,
  });
}

const out = join(dirname(fileURLToPath(import.meta.url)), "fixtures", "parity.json");
mkdirSync(dirname(out), { recursive: true });
writeFileSync(out, JSON.stringify(cases));
console.log(`wrote ${Object.entries(cases).map(([k, v]) => `${v.length} ${k}`).join(", ")} → ${out}`);
