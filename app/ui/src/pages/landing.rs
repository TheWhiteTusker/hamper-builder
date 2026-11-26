//! The start screen, ported from the old app's landing page: the brand card in
//! the middle, product and hamper photos scrolling down either side.

use crate::db::{self, Q, live_with};
use crate::exit::ExitOnEscape;
use lattice_core::api::ApiError;
use lattice_core::images::thumb_url;
use leptos::prelude::*;
use leptos_router::components::A;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Serialize, Deserialize)]
struct Photo {
    id: String,
    url: String,
    name: String,
    category: String,
}

/// Curated gifting photos (Unsplash id, name, label) that fill the columns
/// around the catalogue's own. w=360 keeps decoding cheap.
const CURATED: [(&str, &str, &str); 28] = [
    ("1549007994-cb92caebd54b", "Artisan Chocolate Truffles", "Gourmet Confections"),
    ("1514432324607-a09d9b4aefdd", "Handcrafted Ceramic Mug", "Artisanal Living"),
    ("1603006905003-be475563bc59", "Hand-Poured Amber Candle", "Home Fragrance"),
    ("1576092768241-dec231879fc3", "Single-Estate Darjeeling Tea", "Beverages"),
    ("1528751014936-863e6e7a319c", "Saffron Pistachios & Almonds", "Royal Dry Fruits"),
    ("1513885535751-8b9238bd345a", "Velvet Keepsake Hamper Box", "Packaging"),
    ("1551024709-8f23befc6f87", "Brushed Brass Barware Set", "Entertaining"),
    ("1559056199-641a0ac8b55e", "Monsoon Malabar Coffee Beans", "Specialty Roast"),
    ("1512909006721-3d6018887383", "Wax-Sealed Greeting Accents", "Finishing Touches"),
    ("1587049352846-4a222e784d38", "Organic Acacia Honey with Dipper", "Gourmet Pantry"),
    ("1616401784845-180882ba9ba8", "Bespoke Marble Coaster Quad", "Home Decor"),
    ("1556881286-fc6915169721", "Sparkling Botanical Elixir Flutes", "Celebration"),
    ("1606787366850-de6330128bfc", "Handmade Brass Keepsake Diya", "Festive Tokens"),
    ("1549465220-1a8b9238cd48", "Heritage Wooden Gift Trunk", "Signature Hampers"),
    ("1599599810769-bcde5a160d32", "Artisanal Hazelnut Brittle", "Sweet Delights"),
    ("1615529182904-14819c35db37", "Organic Belgian Linen Napkins", "Tableware"),
    ("1544716278-ca5e3f4abd8c", "Hand-Bound Leather Journal", "Stationery & Desk"),
    ("1569864358642-9d1684040f43", "Parisian Macaron Collection", "Patisserie"),
    ("1608248597359-0a6e0339a9ec", "Botanical Essential Oil Blend", "Wellness"),
    ("1596560548464-f010549b84d7", "Roasted Spiced Cashews Jar", "Savouries"),
    ("1578749556568-bc2c40e68b61", "Matte Ceramic Flora Bud Vase", "Artisanal Living"),
    ("1517256064527-09c73fc73e38", "Gilded Fine Bone China Cup", "Drinkware"),
    ("1544816155-12df9643f363", "Loom-Woven Willow Basket", "Packaging"),
    ("1606313564200-e75d5e30476c", "Single-Origin 72% Dark Chocolate", "Gourmet Confections"),
    ("1602928321679-560bb453f190", "Cedar & Bergamot Reed Diffuser", "Home Fragrance"),
    ("1558961363-fa8fdf82db35", "Cardamom & Almond Biscotti", "Bakery"),
    ("1529699211952-734e80c4d42b", "Rosewood Pocket Game Board", "Luxury Leisure"),
    ("1513201099705-a9746e1e201f", "Celebration Ribbon & Tag Kit", "Packaging"),
];

/// Catalogue photos (`landing_photos`, migration 0019), then hamper images,
/// then the curated set, each URL once.
async fn photos() -> Result<Vec<Photo>, ApiError> {
    #[derive(Deserialize)]
    struct Hamper {
        id: String,
        name: String,
        image_url: String,
    }
    let mut all: Vec<Photo> = db::rpc("landing_photos", &serde_json::json!({})).await?;
    let hampers: Vec<Hamper> = Q::from("hampers")
        .select("id,name,image_url")
        .not_null("image_url")
        .null("deleted_at")
        .limit(20)
        .rows()
        .await?;
    all.extend(hampers.into_iter().map(|h| Photo {
        id: h.id,
        url: h.image_url,
        name: h.name,
        category: "Hamper Presentation".into(),
    }));
    all.extend(CURATED.iter().enumerate().map(|(i, (id, name, category))| Photo {
        id: format!("f-{}", i + 1),
        url: format!("https://images.unsplash.com/photo-{id}?q=75&w=360&auto=format&fit=crop"),
        name: (*name).into(),
        category: (*category).into(),
    }));
    let mut seen = HashSet::new();
    all.retain(|p| seen.insert(p.url.clone()));
    Ok(all)
}

#[component]
pub fn Landing() -> impl IntoView {
    let photos = live_with("landing", photos);
    // Four columns that never share a photo: every 4th item goes to the same one.
    let column = move |n: usize| {
        photos.data.with(|all| {
            all.iter().flatten().enumerate().filter(|(i, _)| i % 4 == n).map(|(_, p)| p.clone()).collect::<Vec<_>>()
        })
    };

    view! {
        // Over the nav bar: the start screen is the whole window, as before.
        <div class="fixed inset-0 z-40 flex select-none flex-col overflow-hidden bg-paper text-(--color-ink)">
            <ExitOnEscape />
            <header class="z-30 flex shrink-0 items-center justify-between border-b border-line/80 bg-paper/90 px-6 py-3.5 backdrop-blur-md">
                <div class="flex items-center gap-3">
                    <img src="/assets/lattice-lane-logo-black.png" alt="Lattice Lane" class="h-7 w-auto object-contain" />
                    <span class="border-l border-line pl-3 text-[11px] font-semibold uppercase tracking-widest text-(--color-muted)">
                        "House of Gifting"
                    </span>
                </div>
                <A href="/dashboard" attr:class="btn-primary rounded-lg px-4 py-2 text-sm shadow-sm">"Open Dashboard →"</A>
            </header>

            <main class="relative flex w-full flex-1 items-stretch overflow-hidden px-6 py-2">
                // Static fades rather than a mask, so the scrolling stays smooth.
                <div class="pointer-events-none absolute inset-x-0 top-0 z-20 h-24 bg-linear-to-b from-paper via-paper/85 to-transparent"></div>
                <div class="pointer-events-none absolute inset-x-0 bottom-0 z-20 h-24 bg-linear-to-t from-paper via-paper/85 to-transparent"></div>
                <div class="relative flex h-full w-full items-stretch justify-center gap-5">
                    {move || view! { <Column items=column(0) secs=52 class="hidden xl:flex" /> }}
                    {move || view! { <Column items=column(1) secs=68 class="hidden md:flex" /> }}
                    <div class="z-20 flex h-full min-w-72.5 max-w-120 flex-[1.25] flex-col items-center justify-center px-1 py-2">
                        <HeroCard />
                    </div>
                    {move || view! { <Column items=column(2) secs=60 class="hidden md:flex" /> }}
                    {move || view! { <Column items=column(3) secs=48 class="hidden xl:flex" /> }}
                </div>
            </main>
        </div>
    }
}

/// One downward-scrolling column: the photos twice over, so the loop has no seam.
#[component]
fn Column(items: Vec<Photo>, secs: u32, class: &'static str) -> impl IntoView {
    let cards = move |hidden: bool| {
        let list = items
            .iter()
            .map(|p| {
                view! {
                    <article class="group w-full shrink-0 cursor-default rounded-2xl border border-line bg-white p-2.5 shadow-xs">
                        <div class="relative aspect-4/3 w-full overflow-hidden rounded-xl bg-paper">
                            <img
                                src=thumb_url(&p.url)
                                alt=p.name.clone()
                                decoding="async"
                                loading="lazy"
                                class="h-full w-full object-cover transition-transform duration-300 ease-out group-hover:scale-105"
                            />
                        </div>
                        <div class="mt-2 px-0.5 text-left">
                            <p class="truncate text-[11px] font-semibold uppercase tracking-wider text-(--color-muted)">{p.category.clone()}</p>
                            <h3 class="truncate text-sm font-medium text-(--color-ink) transition-colors group-hover:text-(--color-brand)">{p.name.clone()}</h3>
                        </div>
                    </article>
                }
            })
            .collect_view();
        view! { <div class="flex flex-col gap-3 pb-3" aria-hidden=hidden.then_some("true")>{list}</div> }
    };
    view! {
        <div class=format!("{class} group-pause h-full min-w-0 flex-1 flex-col overflow-hidden")>
            <div class="animate-marquee-down flex flex-col" style=format!("animation-duration: {secs}s")>
                {cards(false)}
                {cards(true)}
            </div>
        </div>
    }
}

/// The centre card: brand lockup, what the suite does, and the way in.
#[component]
fn HeroCard() -> impl IntoView {
    let feature = |title: &'static str, hint: &'static str| {
        view! {
            <div class="rounded-xl border border-line bg-white/80 px-3 py-2 text-xs">
                <span class="block font-semibold text-(--color-ink)">{title}</span>
                <span class="text-[11px] text-(--color-muted)">{hint}</span>
            </div>
        }
    };
    view! {
        <div class="relative flex max-h-[92vh] w-full flex-col items-center justify-between overflow-y-auto rounded-3xl border-2 border-gold/60 bg-paper/95 p-8 text-center shadow-[0_20px_50px_rgba(44,51,47,0.12)] backdrop-blur-xl transition-shadow duration-300 hover:shadow-[0_25px_60px_rgba(44,51,47,0.18)]">
            <div class="absolute left-3 top-3 h-3 w-3 border-l-2 border-t-2 border-gold opacity-70"></div>
            <div class="absolute right-3 top-3 h-3 w-3 border-r-2 border-t-2 border-gold opacity-70"></div>
            <div class="absolute bottom-3 left-3 h-3 w-3 border-b-2 border-l-2 border-gold opacity-70"></div>
            <div class="absolute bottom-3 right-3 h-3 w-3 border-b-2 border-r-2 border-gold opacity-70"></div>

            <div class="flex flex-col items-center pt-2">
                <span class="mb-2 text-[10px] font-semibold uppercase tracking-[0.25em] text-(--color-brand)">
                    "Bespoke Corporate & Celebratory Gifting"
                </span>
                <img src="/assets/lattice-lane-lockup.png" alt="Lattice Lane" class="mb-1 h-24 w-auto object-contain brightness-0" />
                <div class="my-2 h-0.5 w-14 bg-linear-to-r from-transparent via-gold to-transparent"></div>
            </div>

            <div class="my-5 space-y-3">
                <h1 class="font-display text-3xl font-bold leading-tight tracking-tight text-(--color-brand-dark)">
                    "Hamper Builder & Costing Suite"
                </h1>
                <p class="mx-auto max-w-sm text-sm leading-relaxed text-(--color-muted)">
                    "Architect unforgettable impressions. Curate artisanal gifts, simulate container limits, calculate precise gross margins, and generate proforma quotations in seconds."
                </p>
            </div>

            <div class="my-2 grid w-full grid-cols-2 gap-2 text-left">
                {feature("📦 Dynamic Canvas", "Visual layout & items")}
                {feature("⚡ Margin Control", "Instant pricing & GST")}
                {feature("📊 PowerPoint Deck", "Automated pitch exports")}
                {feature("📜 Quotations & PI", "Print-ready proforma")}
            </div>

            <div class="w-full pt-4">
                <A
                    href="/dashboard"
                    attr:class="group flex w-full items-center justify-center gap-2 rounded-xl bg-(--color-brand) px-6 py-3 text-sm font-semibold text-white shadow-md transition-all duration-200 hover:bg-(--color-brand-dark) hover:shadow-lg"
                >
                    <span>"Open Workspace"</span>
                    <span class="font-bold transition-transform duration-200 group-hover:translate-x-1">"→"</span>
                </A>
            </div>

            <div class="mt-3 w-full border-t border-line/60 pt-4">
                <p class="text-[10px] font-semibold uppercase tracking-wider text-(--color-muted)">
                    "Private Enterprise System • Lattice Lane"
                </p>
            </div>
        </div>
    }
}
