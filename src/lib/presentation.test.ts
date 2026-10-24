import { test } from "node:test";
import assert from "node:assert/strict";
import { CanvasSchema } from "./hamper-canvas-schema.ts";
import {
  arrangeRow,
  closingSlide,
  coverSlide,
  DECK_COLORS,
  formatInr,
  hamperSlide,
  pptTextBox,
  productSlide,
  SLIDE,
  splitTitle,
  type DeckItemInfo,
} from "./presentation.ts";

const valid = (c: unknown) => {
  const r = CanvasSchema.safeParse(c);
  assert.ok(r.success, r.success ? "" : JSON.stringify(r.error.issues[0]));
};

const item = (name: string, caption: string, url: string | null): DeckItemInfo => ({
  product_id: null,
  name,
  caption,
  image_url: url,
});

const sizes: Record<string, { width: number; height: number }> = {
  "https://e.com/box.png": { width: 1200, height: 1000 },
  "https://e.com/mug.png": { width: 900, height: 1000 },
  "https://e.com/book.png": { width: 750, height: 1000 },
  "https://e.com/card.png": { width: 1470, height: 1000 },
  "https://e.com/logo.png": { width: 1600, height: 887 },
};
const sizeOf = (u: string) => sizes[u];
const logo = { url: "https://e.com/logo.png", size: sizes["https://e.com/logo.png"] };

test("generated slides are valid designer canvases with unique layer ids", () => {
  const slides = [
    coverSlide("Premium", "Giveaway Hamper", logo),
    hamperSlide(
      {
        id: "h", code: "H1", name: "Essential Hamper", price: 3250, image_url: null,
        items: [
          item("Card Holder", "Genuine Leather", "https://e.com/card.png"),
          item("A5 Note Book", "Custom", "https://e.com/book.png"),
          item("Wooden Lift Top Box", "Box & Packaging", "https://e.com/box.png"),
          item("Cork Base Coffee Mug", "Drinkware", "https://e.com/mug.png"),
          item("No photo item", "Edibles", null),
        ],
      },
      sizeOf,
      "MOQ note",
      logo,
    ),
    hamperSlide({ id: "h2", code: "H2", name: "Bare", price: null, image_url: null, items: [] }, sizeOf, "", null),
    productSlide({ id: "11111111-1111-4111-8111-111111111111", code: "P1", name: "Mug", caption: "Drinkware", price: 900, image_url: "https://e.com/mug.png" }, sizeOf, "", logo),
    closingSlide("Thank You", "for your time", "+91 81975 44505", logo),
  ];
  for (const s of slides) {
    valid(s);
    assert.equal(s.width, SLIDE.width);
  }
  const ids = slides.flatMap((s) => s.layers.map((l) => l.id));
  assert.equal(new Set(ids).size, ids.length);

  // Entry and closing slides are green; item slides are white with a green band.
  assert.deepEqual(slides[0].background.fill, { type: "solid", color: DECK_COLORS.green });
  const hamper = slides[1];
  assert.deepEqual(hamper.background.fill, { type: "solid", color: DECK_COLORS.page });
  assert.ok(hamper.layers.some((l) => l.name === "Band" && l.kind === "rect" && l.fill.type === "solid" && l.fill.color === DECK_COLORS.green));
  assert.ok(hamper.layers.some((l) => l.kind === "text" && l.text === "Price : INR 3,250 + Tax"));
  assert.ok(hamper.layers.some((l) => l.kind === "text" && l.text === "Essential"));

  // The logo is on every slide that was given one.
  for (const s of [slides[0], slides[1], slides[3], slides[4]]) assert.ok(s.layers.some((l) => l.name === "Logo"));
  assert.ok(!slides[2].layers.some((l) => l.name === "Logo"));

  // Photos: one per item with an image, the box drawn first (behind the rest).
  const photos = hamper.layers.filter((l) => l.kind === "image" && l.name !== "Logo");
  assert.equal(photos.length, 4);
  assert.equal(photos[0].name, "Wooden Lift Top Box");

  // No price and no photos: no price layers, and a hint on the band instead.
  assert.ok(!slides[2].layers.some((l) => l.name === "Price" || l.name === "Terms"));
  assert.ok(slides[2].layers.some((l) => l.name === "Placeholder"));
  assert.ok(slides[4].layers.some((l) => l.kind === "text" && l.textDecoration === "underline"));
});

test("arrangeRow puts the box in the middle, keeps proportions, fits the row and stands photos on one baseline", () => {
  const row = arrangeRow([
    { item: item("Card", "Leather", "c"), url: "c", size: sizes["https://e.com/card.png"] },
    { item: item("Book", "Custom", "b"), url: "b", size: sizes["https://e.com/book.png"] },
    { item: item("Box", "Box & Packaging", "x"), url: "x", size: sizes["https://e.com/box.png"] },
    { item: item("Mug", "Drinkware", "m"), url: "m", size: sizes["https://e.com/mug.png"] },
  ]);
  assert.deepEqual(row.map((p) => p.item.name), ["Card", "Book", "Box", "Mug"]);
  assert.equal(row.find((p) => p.isBox)?.item.name, "Box");
  for (const p of row) {
    assert.ok(Math.abs(p.y + p.height - 775) < 1e-9, "stands on the baseline");
    const src = { Card: 1.47, Book: 0.75, Box: 1.2, Mug: 0.9 }[p.item.name]!;
    assert.ok(Math.abs(p.width / p.height - src) < 1e-9, "keeps aspect ratio");
  }
  const left = row[0].x;
  const right = row.at(-1)!.x + row.at(-1)!.width;
  assert.ok(right - left <= 1500 + 1e-9);
  assert.ok(Math.abs((left + right) / 2 - 960) < 1e-9, "centred on the slide");
  assert.deepEqual(arrangeRow([]), []);
});

test("splitTitle and formatInr follow the deck's wording", () => {
  assert.deepEqual(splitTitle("Essential Hamper", "Hamper"), ["Essential", "Hamper"]);
  assert.deepEqual(splitTitle("Deskmate", "Hamper"), ["Deskmate", "Hamper"]);
  assert.deepEqual(splitTitle("Diwali hampers", "Hamper"), ["Diwali", "Hampers"]);
  assert.equal(formatInr(3250), "INR 3,250");
  assert.equal(formatInr(125000.4), "INR 1,25,000");
});

test("pptTextBox converts px to inches/points and keeps the rotated centre", () => {
  const layer = {
    id: "t", visible: true, locked: false, kind: "text" as const, x: 144, y: 288, rotation: 0,
    scaleX: 1, scaleY: 1, opacity: 0.75, text: "Hi", fontSize: 48, fontFamily: "Inter",
    fontStyle: "bold" as const, align: "center" as const, width: 288, letterSpacing: 4, lineHeight: 1.2,
    textDecoration: "underline" as const, fill: { type: "solid" as const, color: "#54655b" },
  };
  const b = pptTextBox(layer, 144);
  assert.deepEqual([b.x, b.y, b.w, b.h], [1, 2, 2, 1]);
  assert.equal(b.fontSize, 24);
  assert.equal(b.charSpacing, 2);
  assert.equal(b.color, "54655B");
  assert.equal(b.transparency, 25);
  assert.deepEqual(b.underline, { style: "sng" });

  const r = pptTextBox({ ...layer, rotation: 90 }, 144);
  assert.ok(Math.abs(r.x - (144 - 72 - 144) / 144) < 1e-9);
  assert.ok(Math.abs(r.y - (288 + 144 - 72) / 144) < 1e-9);
});
