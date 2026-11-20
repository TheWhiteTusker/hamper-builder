//! Presentations (port of `(app)/presentations/*`): client decks of a cover,
//! a slide per hamper or product, and a closing slide. Every slide is a
//! studio canvas, so it opens in the editor afterwards.

mod deck;
mod new;
mod print;
mod slide;

pub use deck::DeckOverview;
pub use new::NewPresentation;
pub use print::PrintDeck;
pub use slide::SlideEditor;

use crate::components::PageHeader;
use crate::db::{Q, live};
use crate::studio::{brand_logo, load_image, uuid};
use futures::join;
use lattice_core::Decimal;
use lattice_core::api::ApiError;
use lattice_core::canvas::Canvas;
use lattice_core::canvas::deck::{DeckItemInfo, HamperInfo, Photo, ProductInfo, Size, closing_slide, cover_slide, hamper_slide, product_slide};
use lattice_core::images::thumb_url;
use lattice_core::js::f;
use lattice_core::lenient;
use lattice_core::money::format_money;
use leptos::prelude::*;
use leptos_router::components::A;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub fn deck_href(id: &str) -> String {
    format!("/presentations/{id}")
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct DeckRow {
    id: String,
    title: String,
    updated_at: Option<String>,
    #[serde(default)]
    presentation_slides: Vec<CountRow>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct CountRow {
    count: i64,
}

/// "19 Nov 2026, 3:42 pm", as `toLocaleString("en-IN", { dateStyle: "medium", timeStyle: "short" })`.
fn when(ts: &str) -> String {
    let d = js_sys::Date::new(&ts.into());
    let opts = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&opts, &"dateStyle".into(), &"medium".into());
    let _ = js_sys::Reflect::set(&opts, &"timeStyle".into(), &"short".into());
    d.to_locale_string("en-IN", &opts).as_string().unwrap_or_default()
}

#[component]
pub fn Presentations() -> impl IntoView {
    let decks = live::<DeckRow>("presentations", Q::from("presentations").select("id,title,updated_at,presentation_slides(count)").order_desc("updated_at"));
    view! {
        <PageHeader title="Presentations" subtitle="Client decks: a cover, a slide per hamper or product, and a closing slide">
            <A href="/presentations/new" attr:class="btn-primary">"New presentation"</A>
        </PageHeader>
        <div class="card overflow-x-auto">
            <table class="w-full border-collapse text-sm">
                <thead><tr><th class="th">"Title"</th><th class="th num">"Slides"</th><th class="th">"Last edited"</th></tr></thead>
                <tbody>
                    {move || match decks.data.get() {
                        None => view! { <tr><td colspan="3" class="td h-32 animate-pulse bg-white/60"></td></tr> }.into_any(),
                        Some(rows) if rows.is_empty() => view! {
                            <tr><td colspan="3" class="td py-10 text-center text-sm text-(--color-muted)">
                                "No presentations yet."<A href="/presentations/new" attr:class="ml-2 underline">"Make the first one"</A>
                            </td></tr>
                        }.into_any(),
                        Some(rows) => rows.into_iter().map(|d| view! {
                            <tr>
                                <td class="td"><A href=deck_href(&d.id) attr:class="font-medium hover:underline">{d.title.clone()}</A></td>
                                <td class="td num">{d.presentation_slides.first().map_or(0, |c| c.count)}</td>
                                <td class="td text-(--color-muted)">{d.updated_at.as_deref().map(when).unwrap_or_default()}</td>
                            </tr>
                        }).collect_view().into_any(),
                    }}
                </tbody>
            </table>
        </div>
    }
}

// ------------------------------------------------------------- the picker

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemKind {
    Hamper,
    Product,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeckItem {
    pub kind: ItemKind,
    pub id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pickable {
    pub kind: ItemKind,
    pub id: String,
    pub code: String,
    pub name: String,
    pub price: Option<Decimal>,
    pub image_url: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PickerData {
    pub hampers: Vec<Pickable>,
    pub products: Vec<Pickable>,
}

#[derive(Deserialize)]
struct PickRow {
    id: String,
    code: String,
    name: String,
    #[serde(default, alias = "default_sp", deserialize_with = "lenient::opt_decimal")]
    final_catalogue_sp: Option<Decimal>,
    image_url: Option<String>,
}

/// Hampers and active products for the slide picker.
pub async fn load_picker() -> Result<PickerData, ApiError> {
    let (h, p) = (
        Q::from("hamper_summary").select("id,code,name,final_catalogue_sp,image_url").order("code"),
        Q::from("products").select("id,code,name,default_sp,image_url").eq("is_active", "true").null("deleted_at").order("name"),
    );
    let (hampers, products) = join!(h.rows::<PickRow>(), p.rows::<PickRow>());
    let map = |kind: ItemKind| move |r: PickRow| Pickable { kind, id: r.id, code: r.code, name: r.name, price: r.final_catalogue_sp, image_url: r.image_url };
    Ok(PickerData {
        hampers: hampers?.into_iter().map(map(ItemKind::Hamper)).collect(),
        products: products?.into_iter().map(map(ItemKind::Product)).collect(),
    })
}

/// Tick hampers and products on the left; the right column is the slide
/// order, which the arrows rearrange.
#[component]
pub fn ItemPicker(data: PickerData, value: RwSignal<Vec<DeckItem>>) -> impl IntoView {
    let tab = RwSignal::new(ItemKind::Hamper);
    let q = RwSignal::new(String::new());
    let data = StoredValue::new(data);
    let (nh, np) = data.with_value(|d| (d.hampers.len(), d.products.len()));
    let picked = move |i: &Pickable| value.with(|v| v.iter().any(|x| x.kind == i.kind && x.id == i.id));
    let toggle = move |i: &Pickable| {
        let (kind, id) = (i.kind, i.id.clone());
        value.update(|v| {
            if let Some(pos) = v.iter().position(|x| x.kind == kind && x.id == id) {
                v.remove(pos);
            } else {
                v.push(DeckItem { kind, id });
            }
        });
    };
    let shift = move |index: usize, down: bool| {
        value.update(|v| {
            let to = if down { index + 1 } else { index.wrapping_sub(1) };
            if to < v.len() {
                v.swap(index, to);
            }
        });
    };
    let find = move |item: &DeckItem| data.with_value(|d| d.hampers.iter().chain(&d.products).find(|p| p.kind == item.kind && p.id == item.id).cloned());
    view! {
        <div class="grid gap-4 lg:grid-cols-[1fr_360px]">
            <div class="card flex min-h-0 flex-col p-3">
                <div class="mb-3 flex flex-wrap items-center gap-2">
                    <button type="button" class=move || if tab.get() == ItemKind::Hamper { "btn-primary" } else { "btn-secondary" } on:click=move |_| tab.set(ItemKind::Hamper)>{format!("Hampers ({nh})")}</button>
                    <button type="button" class=move || if tab.get() == ItemKind::Product { "btn-primary" } else { "btn-secondary" } on:click=move |_| tab.set(ItemKind::Product)>{format!("Products ({np})")}</button>
                    <input class="input ml-auto max-w-xs" placeholder="Search name or code" prop:value=q on:input=move |e| q.set(event_target_value(&e)) />
                </div>
                <ul class="max-h-[480px] divide-y divide-(--color-line) overflow-y-auto">
                    {move || {
                        let s = q.get().trim().to_lowercase();
                        let list: Vec<Pickable> = data.with_value(|d| if tab.get() == ItemKind::Hamper { d.hampers.clone() } else { d.products.clone() })
                            .into_iter()
                            .filter(|i| s.is_empty() || i.name.to_lowercase().contains(&s) || i.code.to_lowercase().contains(&s))
                            .collect();
                        if list.is_empty() {
                            return view! { <li class="py-8 text-center text-sm text-(--color-muted)">"Nothing matches."</li> }.into_any();
                        }
                        list.into_iter().map(|i| {
                            let on = picked(&i);
                            let it = i.clone();
                            view! {
                                <li>
                                    <label class="flex cursor-pointer items-center gap-3 px-2 py-2 hover:bg-(--color-paper)">
                                        <input type="checkbox" class="h-4 w-4 accent-(--color-brand)" prop:checked=on on:change=move |_| toggle(&it) />
                                        {match i.image_url.clone() {
                                            Some(u) => view! { <img src=thumb_url(&u) alt="" loading="lazy" class="h-10 w-10 rounded-md border border-(--color-line) object-cover" /> }.into_any(),
                                            None => view! { <span class="h-10 w-10 rounded-md border border-dashed border-(--color-line) bg-(--color-paper)"></span> }.into_any(),
                                        }}
                                        <span class="min-w-0 flex-1">
                                            <span class="block truncate text-sm font-medium">{i.name.clone()}</span>
                                            <span class="font-mono text-xs text-(--color-muted)">{i.code.clone()}</span>
                                        </span>
                                        <span class="text-sm text-(--color-muted) tabular-nums">{i.price.map_or_else(|| "—".to_string(), |p| format_money(Some(p)))}</span>
                                    </label>
                                </li>
                            }
                        }).collect_view().into_any()
                    }}
                </ul>
            </div>
            <div class="card self-start p-3">
                <div class="label mb-2">{move || format!("Slide order ({})", value.with(Vec::len))}</div>
                {move || {
                    let items = value.get();
                    if items.is_empty() {
                        return view! { <p class="py-6 text-center text-sm text-(--color-muted)">"Tick hampers or products to add their slides."</p> }.into_any();
                    }
                    let n = items.len();
                    view! {
                        <ol class="space-y-1">
                            {items.into_iter().enumerate().map(|(index, v)| {
                                let item = find(&v);
                                let label = if v.kind == ItemKind::Hamper { "hamper" } else { "product" };
                                view! {
                                    <li class="flex items-center gap-2 rounded-md bg-(--color-paper) px-2 py-1.5 text-sm">
                                        <span class="w-5 text-right text-(--color-muted) tabular-nums">{index + 1}</span>
                                        <span class="min-w-0 flex-1 truncate">{item.as_ref().map_or_else(|| "Unknown".to_string(), |i| i.name.clone())}</span>
                                        <span class="badge">{label}</span>
                                        <button type="button" aria-label="Move up" class="px-1 disabled:opacity-30" disabled={index == 0} on:click=move |_| shift(index, false)>"↑"</button>
                                        <button type="button" aria-label="Move down" class="px-1 disabled:opacity-30" disabled={index + 1 == n} on:click=move |_| shift(index, true)>"↓"</button>
                                        <button type="button" aria-label="Remove" class="px-1 text-red-700" on:click=move |_| if let Some(i) = &item { toggle(i) }>"×"</button>
                                    </li>
                                }
                            }).collect_view()}
                        </ol>
                    }.into_any()
                }}
            </div>
        </div>
    }
}

// ---------------------------------------------------------- laying out

/// A slide as stored in `presentation_slides`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SlideInput {
    pub kind: String,
    pub hamper_id: Option<String>,
    pub product_id: Option<String>,
    pub canvas: Canvas,
}

/// The entry and closing slides' words.
#[derive(Clone, Debug, Default)]
pub struct DeckText {
    pub title: String,
    pub subtitle: String,
    pub closing_heading: String,
    pub closing_tagline: String,
    pub closing_contact: String,
}

#[derive(Deserialize)]
struct LineRow {
    hamper_id: String,
    product_id: Option<String>,
    product_name: String,
    category_name: Option<String>,
    products: Option<ImageOnly>,
}

#[derive(Deserialize)]
struct ImageOnly {
    image_url: Option<String>,
}

#[derive(Deserialize)]
struct ProductRow {
    id: String,
    code: String,
    name: String,
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    default_sp: Option<Decimal>,
    image_url: Option<String>,
    categories: Option<NameOnly>,
}

#[derive(Deserialize)]
struct NameOnly {
    name: String,
}

#[derive(Deserialize)]
struct CatRow {
    name: String,
    counts_as_item: bool,
}

/// Everything the layout needs for the picked items (port of `loadDeckData`).
/// Packaging that doesn't count as an item (ribbons, fillers) stays off the slides.
async fn load_deck_data(items: &[DeckItem]) -> Result<(Vec<HamperInfo>, Vec<ProductInfo>, Option<Photo>), ApiError> {
    let hamper_ids: Vec<&str> = items.iter().filter(|i| i.kind == ItemKind::Hamper).map(|i| i.id.as_str()).collect();
    let product_ids: Vec<&str> = items.iter().filter(|i| i.kind == ItemKind::Product).map(|i| i.id.as_str()).collect();
    let hq = Q::from("hamper_summary").select("id,code,name,final_catalogue_sp,image_url").in_list("id", &hamper_ids);
    let lq = Q::from("hamper_items").select("hamper_id,product_id,product_name,category_name,products(image_url)").in_list("hamper_id", &hamper_ids).order("line_no");
    let pq = Q::from("products").select("id,code,name,default_sp,image_url,categories(name)").in_list("id", &product_ids);
    let cq = Q::from("categories").select("name,counts_as_item");
    let none = async { Ok(Vec::new()) };
    let (hampers, lines, products, cats, logo) = join!(
        async { if hamper_ids.is_empty() { Ok(Vec::new()) } else { hq.rows::<PickRow>().await } },
        async { if hamper_ids.is_empty() { Ok(Vec::new()) } else { lq.rows::<LineRow>().await } },
        async { if product_ids.is_empty() { none.await } else { pq.rows::<ProductRow>().await } },
        cq.rows::<CatRow>(),
        brand_logo(),
    );
    let (hampers, lines, products, cats) = (hampers?, lines?, products?, cats?);
    let counts: HashMap<String, bool> = cats.into_iter().map(|c| (c.name.trim().to_lowercase(), c.counts_as_item)).collect();
    let counts_as_item = |c: &Option<String>| counts.get(&c.as_deref().unwrap_or("").trim().to_lowercase()).copied().unwrap_or(true);
    let hamper_infos = hamper_ids
        .iter()
        .filter_map(|id| hampers.iter().find(|h| h.id == *id))
        .map(|h| HamperInfo {
            id: h.id.clone(),
            code: h.code.clone(),
            name: h.name.clone(),
            price: h.final_catalogue_sp.map(f),
            image_url: h.image_url.clone(),
            items: lines
                .iter()
                .filter(|l| l.hamper_id == h.id && counts_as_item(&l.category_name))
                .map(|l| DeckItemInfo {
                    product_id: l.product_id.clone(),
                    name: l.product_name.clone(),
                    caption: l.category_name.clone().unwrap_or_default(),
                    image_url: l.products.as_ref().and_then(|p| p.image_url.clone()),
                })
                .collect(),
        })
        .collect();
    let product_infos = product_ids
        .iter()
        .filter_map(|id| products.iter().find(|p| p.id == *id))
        .map(|p| ProductInfo {
            id: p.id.clone(),
            code: p.code.clone(),
            name: p.name.clone(),
            caption: p.categories.as_ref().map(|c| c.name.clone()).unwrap_or_default(),
            price: p.default_sp.map(f),
            image_url: p.image_url.clone(),
        })
        .collect();
    Ok((hamper_infos, product_infos, logo))
}

/// Lays out slides for the picked items once the photos' proportions are
/// known (photos that fail to load are left out). With `text`, the entry and
/// closing slides too (port of `buildSlides`).
pub async fn build_slides(items: &[DeckItem], note: &str, text: Option<&DeckText>) -> Result<Vec<SlideInput>, ApiError> {
    let (hampers, products, logo) = load_deck_data(items).await?;
    let mut urls: Vec<String> = Vec::new();
    for h in &hampers {
        urls.extend(h.image_url.clone());
        urls.extend(h.items.iter().filter_map(|i| i.image_url.clone()));
    }
    urls.extend(products.iter().filter_map(|p| p.image_url.clone()));
    urls.extend(logo.as_ref().map(|l| l.url.clone()));
    urls.sort();
    urls.dedup();
    let mut sizes: HashMap<String, Size> = HashMap::new();
    for u in urls {
        if let Ok(img) = load_image(&u).await {
            sizes.insert(u, Size { width: f64::from(img.natural_width()), height: f64::from(img.natural_height()) });
        }
    }
    let size_of = |u: &str| sizes.get(u).copied();
    let logo = logo.filter(|l| sizes.contains_key(&l.url));
    let mut ids = uuid;
    let mut middle = Vec::new();
    for item in items {
        match item.kind {
            ItemKind::Hamper => {
                if let Some(h) = hampers.iter().find(|h| h.id == item.id) {
                    middle.push(SlideInput { kind: "hamper".into(), hamper_id: Some(h.id.clone()), product_id: None, canvas: hamper_slide(&mut ids, h, &size_of, note, logo.as_ref()) });
                }
            }
            ItemKind::Product => {
                if let Some(p) = products.iter().find(|p| p.id == item.id) {
                    middle.push(SlideInput { kind: "product".into(), hamper_id: None, product_id: Some(p.id.clone()), canvas: product_slide(&mut ids, p, &size_of, note, logo.as_ref()) });
                }
            }
        }
    }
    let Some(t) = text else { return Ok(middle) };
    let mut out = vec![SlideInput { kind: "cover".into(), hamper_id: None, product_id: None, canvas: cover_slide(&mut ids, &t.title, &t.subtitle, logo.as_ref()) }];
    out.extend(middle);
    out.push(SlideInput {
        kind: "closing".into(),
        hamper_id: None,
        product_id: None,
        canvas: closing_slide(&mut ids, &t.closing_heading, &t.closing_tagline, &t.closing_contact, logo.as_ref()),
    });
    Ok(out)
}
