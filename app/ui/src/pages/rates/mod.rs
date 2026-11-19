//! The Rates & Hierarchy Master (port of `cost-calculator/master/*`): stage →
//! category → subcategory → variety, edited in place; move or duplicate a
//! subcategory; colours and swatches; and "Update products" for old rates.

mod actions;

use crate::data::{use_cost_rows, use_settings};
use crate::db::{self, me};
use crate::kit::{Menu, MenuItem, confirm};
use crate::pages::calculator::num_text;
use actions::{Saved, VarietyForm};
use lattice_core::Decimal;
use lattice_core::colors::{FALLBACK_COLOR_HEX, ProductColor, STANDARD_PRODUCT_COLORS, color_code, resolve_colors};
use lattice_core::cost_tree::{
    CategoryNode, CostRows, StageNode, SubcategoryNode, apply_cost_changes, build_cost_tree,
};
use lattice_core::dimensions::COMMON_UNITS;
use lattice_core::money::format_money;
use lattice_core::numbers::parse_num;
use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::HashSet;
use std::future::Future;

#[derive(Clone, Copy)]
struct Master {
    rows: RwSignal<Option<CostRows>>,
    expanded: RwSignal<HashSet<String>>,
    query: RwSignal<String>,
    busy: RwSignal<bool>,
    error: RwSignal<Option<String>>,
    success: RwSignal<Option<String>>,
    editing: RwSignal<Option<VarietyForm>>,
    transfer: RwSignal<Option<(SubcategoryNode, bool, String)>>,
}

impl Master {
    fn searching(&self) -> bool {
        !self.query.get().trim().is_empty()
    }
    fn is_open(&self, id: &str) -> bool {
        self.searching() || self.expanded.with(|e| e.contains(id))
    }
    fn toggle(&self, id: &str) {
        self.expanded.update(|e| {
            if !e.remove(id) {
                e.insert(id.to_string());
            }
        });
    }
    fn expand(&self, id: &str) {
        self.expanded.update(|e| {
            e.insert(id.to_string());
        });
    }
    /// Runs a save, patches the rows it changed, and shows its message (or `success`).
    fn run<F>(self, action: F, success: String, after: impl FnOnce() + 'static)
    where
        F: Future<Output = Saved> + 'static,
    {
        self.error.set(None);
        self.success.set(None);
        self.busy.set(true);
        spawn_local(async move {
            let res = action.await;
            self.busy.set(false);
            match res {
                Ok((changes, message)) => {
                    if !changes.is_empty() {
                        self.rows.update(|r| {
                            if let Some(r) = r {
                                *r = apply_cost_changes(r, &changes);
                                db::cache_put("cost_rows", r);
                            }
                        });
                    }
                    after();
                    self.success.set(Some(message.unwrap_or(success)));
                }
                Err(e) => self.error.set(Some(e.to_string())),
            }
        });
    }
}

/// The tree trimmed to a search: a matching level keeps everything beneath it.
fn filter_stages(stages: Vec<StageNode>, q: &str) -> Vec<StageNode> {
    if q.is_empty() {
        return stages;
    }
    let hit = |s: &str| s.to_lowercase().contains(q);
    stages
        .into_iter()
        .filter_map(|mut stage| {
            if hit(&stage.row.name) {
                return Some(stage);
            }
            stage.categories = stage
                .categories
                .into_iter()
                .filter_map(|mut cat| {
                    if hit(&cat.row.name) {
                        return Some(cat);
                    }
                    cat.subcategories = cat
                        .subcategories
                        .into_iter()
                        .filter_map(|mut sub| {
                            if hit(&sub.row.name) {
                                return Some(sub);
                            }
                            sub.varieties
                                .retain(|v| hit(&v.name) || v.notes.as_deref().is_some_and(hit) || hit(&v.unit));
                            (!sub.varieties.is_empty()).then_some(sub)
                        })
                        .collect();
                    (!cat.subcategories.is_empty()).then_some(cat)
                })
                .collect();
            (!stage.categories.is_empty()).then_some(stage)
        })
        .collect()
}

#[component]
fn Chevron(#[prop(into)] open: Signal<bool>) -> impl IntoView {
    view! {
        <svg class=move || format!("h-4 w-4 shrink-0 text-(--color-muted) transition-transform duration-150 {}", if open.get() { "rotate-90" } else { "" })
            viewBox="0 0 20 20" fill="currentColor" aria-hidden="true">
            <path fill-rule="evenodd" clip-rule="evenodd" d="M7.21 14.77a.75.75 0 01.02-1.06L11.168 10 7.23 6.29a.75.75 0 111.04-1.08l4.5 4.25a.75.75 0 010 1.08l-4.5 4.25a.75.75 0 01-1.06-.02z" />
        </svg>
    }
}

#[component]
pub fn RatesMaster(
    /// Inside Settings → Rates: no page header or calculator tabs.
    #[prop(optional)]
    embedded: bool,
) -> impl IntoView {
    let live = use_cost_rows();
    let settings = use_settings();
    let m = Master {
        rows: RwSignal::new(None),
        expanded: RwSignal::new(HashSet::new()),
        query: RwSignal::new(String::new()),
        busy: RwSignal::new(false),
        error: RwSignal::new(None),
        success: RwSignal::new(None),
        editing: RwSignal::new(None),
        transfer: RwSignal::new(None),
    };
    // Each copy from the network replaces local rows, as the server re-render did.
    Effect::new(move |_| {
        if let Some(r) = live.data.get() {
            m.rows.set(Some(r));
        }
    });
    let admin = me().role.is_admin();
    let stages = Memo::new(move |_| {
        m.rows.get().map(|r| filter_stages(build_cost_tree(&r), &m.query.get().trim().to_lowercase()))
    });

    view! {
        {(!embedded).then(|| view! {
            <crate::components::PageHeader title="Product Cost Calculator" subtitle="Work out what a product costs: materials, hardware, finishing and machine time">
                <crate::pages::calculator::Tabs master=true />
            </crate::components::PageHeader>
        })}
        <div class="space-y-4">
            <div class="flex items-center gap-2">
                <input type="search" class="input text-sm" placeholder="Search materials, varieties or notes…" aria-label="Search rates" bind:value=m.query />
                {admin.then(|| view! {
                    <Menu label="Excel" items=vec![
                        MenuItem { href: "/cost-calculator/master/excel/add", title: "Bulk add", hint: "New varieties from a sheet" },
                        MenuItem { href: "/cost-calculator/master/excel/edit", title: "Bulk edit", hint: "Change rates in Excel" },
                    ] />
                })}
            </div>
            {move || m.error.get().map(|e| view! { <div role="alert" class="rounded-lg border border-red-200 bg-red-50 p-3 text-sm font-medium text-red-700">{e}</div> })}
            {move || m.success.get().map(|s| view! { <div role="status" class="rounded-lg border border-emerald-200 bg-emerald-50 p-3 text-sm font-medium text-emerald-800">{s}</div> })}
            {move || match stages.get() {
                None => view! { <div class="card h-64 animate-pulse bg-white/60"></div> }.into_any(),
                Some(list) if list.is_empty() && m.searching() => view! {
                    <p class="text-sm italic text-(--color-muted)">{format!("Nothing matches “{}”.", m.query.get().trim())}</p>
                }.into_any(),
                Some(list) => view! {
                    <div class="space-y-2">{list.into_iter().map(|s| view! { <StageCard m=m stage=s /> }).collect_view()}</div>
                }.into_any(),
            }}
            <Show when=move || !m.searching()>
                {move || settings.data.get().map(|s| {
                    let hex = s.color_hex.clone().into_iter().collect();
                    view! { <ColorsCard colors=resolve_colors(&s.product_colors, &hex) /> }
                })}
                <SyncRatesButton />
            </Show>
            {move || m.editing.get().map(|f| view! { <VarietyDialog m=m form=f /> })}
            {move || m.transfer.get().map(|(sub, duplicate, current)| view! { <TransferDialog m=m sub=sub duplicate=duplicate current=current /> })}
        </div>
    }
}

/// Inline name box for adding a category or subcategory.
#[component]
fn InlineAdd(
    placeholder: String,
    save_label: &'static str,
    m: Master,
    class: &'static str,
    on_save: impl Fn(String) + 'static,
    on_cancel: impl Fn() + 'static,
) -> impl IntoView {
    let value = RwSignal::new(String::new());
    let input = NodeRef::<html::Input>::new();
    Effect::new(move |_| {
        if let Some(el) = input.get() {
            let _ = el.focus();
        }
    });
    view! {
        <div class=format!("flex flex-wrap items-center gap-3 rounded-lg border border-(--color-brand) p-3 {class}")>
            <input node_ref=input class="input max-w-xs py-1.5 text-xs" placeholder=placeholder bind:value=value />
            <button type="button" class="btn-primary py-1.5 text-xs" disabled=move || m.busy.get() || value.get().trim().is_empty()
                on:click=move |_| on_save(value.get_untracked().trim().to_string())>{save_label}</button>
            <button type="button" class="btn-secondary py-1.5 text-xs" on:click=move |_| on_cancel()>"Cancel"</button>
        </div>
    }
}

#[component]
fn StageCard(m: Master, stage: StageNode) -> impl IntoView {
    let adding = RwSignal::new(false);
    let id = stage.row.id.clone();
    let (i1, i2, i3) = (id.clone(), id.clone(), id.clone());
    let open = Signal::derive(move || m.is_open(&i1));
    let code = stage.row.code.clone();
    let example = match code.as_str() {
        "bought_out" => " (e.g. Ceramics → Jars → 200ml Amber)",
        "material" => " (e.g. Woodbased → Birch → 8mm)",
        _ => "",
    };
    let count = stage.categories.len();
    let is_machine = code == "machine";
    let placeholder = format!(
        "Category name under {} (e.g. {})",
        stage.row.name,
        if code == "bought_out" { "Packaging, Ceramics, Goodies" } else { "Woodbased, Magnets" }
    );
    let cats = stage.categories.clone();
    view! {
        <div class="card">
            <div class="flex items-center justify-between gap-3 px-4 py-2.5">
                <button type="button" class="flex min-w-0 flex-1 items-center gap-2 text-left" aria-expanded=move || open.get().to_string() on:click=move |_| m.toggle(&i2)>
                    <Chevron open=open />
                    <h3 class="text-base font-semibold text-(--color-ink)">{stage.row.name.clone()}</h3>
                    <span class="text-xs text-(--color-muted)">{format!("{count} {}", if count == 1 { "category" } else { "categories" })}</span>
                </button>
                <button type="button" class="btn-secondary px-3 py-1 text-xs" on:click=move |_| { adding.set(true); m.expand(&i3); }>"+ Category"</button>
            </div>
            <Show when=move || open.get()>
                <div class="space-y-3 border-t border-(--color-line) px-4 pt-3 pb-4">
                    <p class="text-xs text-(--color-muted)">{format!("Category → Subcategory → Variety{example}")}</p>
                    {
                        let (placeholder, id) = (placeholder.clone(), id.clone());
                        move || adding.get().then(|| {
                            let id = id.clone();
                            view! {
                                <InlineAdd placeholder=placeholder.clone() save_label="Save category" m=m class="bg-emerald-50/50"
                                    on_save=move |name| m.run(actions::add_category(id.clone(), name), "Category added successfully!".into(), move || adding.set(false))
                                    on_cancel=move || adding.set(false) />
                            }
                        })
                    }
                    <div class="space-y-3">
                        {cats.clone().into_iter().map(|cat| view! { <CategoryBlock m=m cat=cat is_machine=is_machine stage_code=code.clone() /> }).collect_view()}
                    </div>
                </div>
            </Show>
        </div>
    }
}

#[component]
fn CategoryBlock(m: Master, cat: CategoryNode, is_machine: bool, stage_code: String) -> impl IntoView {
    let adding = RwSignal::new(false);
    let renaming = RwSignal::new(false);
    let name = RwSignal::new(cat.row.name.clone());
    let id = cat.row.id.clone();
    let (i1, i2, i3) = (id.clone(), id.clone(), id.clone());
    let open = Signal::derive(move || m.is_open(&i1));
    let original = cat.row.name.clone();
    let bought = stage_code == "bought_out";
    let n_subs = cat.subcategories.len();
    let subs = cat.subcategories.clone();
    let cat_name = cat.row.name.clone();
    let rename = {
        let (id, original) = (id.clone(), original.clone());
        move |e: leptos::ev::SubmitEvent| {
            e.prevent_default();
            let trimmed = name.get_untracked().trim().to_string();
            if trimmed.is_empty() || trimmed == original {
                renaming.set(false);
                name.set(original.clone());
                return;
            }
            m.run(
                actions::rename_category(id.clone(), trimmed.clone()),
                format!("Category renamed to \"{trimmed}\"."),
                move || renaming.set(false),
            );
        }
    };
    let delete = {
        let (id, nm) = (id.clone(), cat_name.clone());
        move |_| {
            if confirm(&format!(
                "Are you sure you want to delete category \"{nm}\" and all its subcategories and varieties?"
            )) {
                m.run(actions::delete_category(id.clone()), format!("Category \"{nm}\" deleted."), || {});
            }
        }
    };
    let o2 = original.clone();
    view! {
        <div class="min-w-0 max-w-full space-y-3 rounded-lg border border-(--color-line) bg-slate-50/40 p-3">
            <div class=move || format!("flex flex-wrap items-center justify-between gap-2 {}", if open.get() { "border-b border-(--color-line) pb-2" } else { "" })>
                {move || if renaming.get() {
                    let o = o2.clone();
                    view! {
                        <form on:submit=rename.clone() class="flex min-w-0 flex-1 flex-wrap items-center gap-2 py-0.5">
                            <span class="text-xs font-semibold uppercase tracking-wide text-(--color-muted)">"Category:"</span>
                            <input class="input w-full py-1 text-xs font-bold sm:w-auto sm:max-w-xs" placeholder="Category name" autofocus bind:value=name />
                            <button type="submit" class="btn-primary px-2.5 py-1 text-xs" disabled=move || m.busy.get() || name.get().trim().is_empty()>{move || if m.busy.get() { "Saving…" } else { "Save" }}</button>
                            <button type="button" class="btn-secondary px-2.5 py-1 text-xs" disabled=move || m.busy.get() on:click=move |_| { renaming.set(false); name.set(o.clone()); }>"Cancel"</button>
                        </form>
                    }.into_any()
                } else {
                    let (i2, i3, nm) = (i2.clone(), i3.clone(), cat_name.clone());
                    view! {
                        <button type="button" class="flex min-w-0 items-center gap-2 text-left" aria-expanded=move || open.get().to_string() on:click=move |_| m.toggle(&i2)>
                            <Chevron open=open />
                            <span class="truncate text-sm font-semibold text-(--color-ink)">{nm}</span>
                            <span class="shrink-0 text-xs text-(--color-muted)">{format!("{n_subs} {}", if n_subs == 1 { "subcategory" } else { "subcategories" })}</span>
                        </button>
                        <div class="flex shrink-0 flex-wrap items-center gap-2">
                            <button type="button" class="btn-secondary px-2.5 py-1 text-xs" title="Rename category" on:click=move |_| renaming.set(true)>"Rename"</button>
                            <button type="button" class="btn-secondary px-2.5 py-1 text-xs" on:click=move |_| { adding.set(true); m.expand(&i3); }>"+ Subcategory"</button>
                            <button type="button" class="p-1 text-xs text-red-600 hover:text-red-800" title="Delete category" on:click=delete.clone()>"Delete"</button>
                        </div>
                    }.into_any()
                }}
            </div>
            <Show when=move || open.get()>
                {
                    let (id, nm) = (id.clone(), cat.row.name.clone());
                    move || adding.get().then(|| {
                        let id = id.clone();
                        view! {
                            <InlineAdd placeholder=format!("Subcategory under {nm} (e.g. {})", if bought { "Glass Bottles, Scented Candles" } else { "Birch, Rubberwood" })
                                save_label="Save Subcategory" m=m class="bg-white"
                                on_save=move |name| m.run(actions::add_subcategory(id.clone(), name), "Subcategory added successfully!".into(), move || adding.set(false))
                                on_cancel=move || adding.set(false) />
                        }
                    })
                }
                {if subs.is_empty() {
                    view! { <p class="py-2 text-xs italic text-(--color-muted)">{format!("No subcategories in {} yet. Use “+ Subcategory” above.", cat.row.name)}</p> }.into_any()
                } else {
                    let (cid, sc) = (id.clone(), stage_code.clone());
                    view! {
                        <div class="space-y-3">{subs.clone().into_iter().map(|sub| view! { <SubcategoryBlock m=m sub=sub category_id=cid.clone() is_machine=is_machine stage_code=sc.clone() /> }).collect_view()}</div>
                    }.into_any()
                }}
            </Show>
        </div>
    }
}

#[component]
fn SubcategoryBlock(
    m: Master,
    sub: SubcategoryNode,
    category_id: String,
    is_machine: bool,
    stage_code: String,
) -> impl IntoView {
    let renaming = RwSignal::new(false);
    let name = RwSignal::new(sub.row.name.clone());
    let id = sub.row.id.clone();
    let i1 = id.clone();
    let open = Signal::derive(move || m.is_open(&i1));
    let bought = stage_code == "bought_out";
    let original = sub.row.name.clone();
    let count = sub.varieties.len();
    let sub_store = StoredValue::new(sub.clone());
    let rename = {
        let (id, original) = (id.clone(), original.clone());
        move |e: leptos::ev::SubmitEvent| {
            e.prevent_default();
            let trimmed = name.get_untracked().trim().to_string();
            if trimmed.is_empty() || trimmed == original {
                renaming.set(false);
                name.set(original.clone());
                return;
            }
            m.run(
                actions::rename_subcategory(id.clone(), trimmed.clone()),
                format!("Subcategory renamed to \"{trimmed}\"."),
                move || renaming.set(false),
            );
        }
    };
    let o2 = original.clone();
    view! {
        <div class="min-w-0 max-w-full rounded-lg border border-slate-200 bg-white p-3 shadow-xs">
            <div class=move || format!("flex flex-wrap items-center justify-between gap-2 {}", if open.get() { "mb-2 border-b border-slate-100 pb-2" } else { "" })>
                {move || if renaming.get() {
                    let o = o2.clone();
                    view! {
                        <form on:submit=rename.clone() class="flex min-w-0 flex-1 flex-wrap items-center gap-2 py-0.5">
                            <span class="text-xs font-semibold text-(--color-muted)">"Subcategory:"</span>
                            <input class="input w-full py-1 text-xs font-bold text-(--color-brand-dark) sm:w-auto sm:max-w-xs" placeholder="Subcategory name" autofocus bind:value=name />
                            <button type="submit" class="btn-primary px-2.5 py-1 text-xs" disabled=move || m.busy.get() || name.get().trim().is_empty()>{move || if m.busy.get() { "Saving…" } else { "Save" }}</button>
                            <button type="button" class="btn-secondary px-2.5 py-1 text-xs" disabled=move || m.busy.get() on:click=move |_| { renaming.set(false); name.set(o.clone()); }>"Cancel"</button>
                        </form>
                    }.into_any()
                } else {
                    let (id1, id2, id3, cid1, cid2) = (id.clone(), id.clone(), id.clone(), category_id.clone(), category_id.clone());
                    let nm = original.clone();
                    let nm2 = original.clone();
                    view! {
                        <button type="button" class="flex min-w-0 items-center gap-2 text-left" aria-expanded=move || open.get().to_string() on:click=move |_| m.toggle(&id1)>
                            <Chevron open=open />
                            <span class="truncate text-sm font-medium text-(--color-brand-dark)">{nm.clone()}</span>
                            <span class="shrink-0 text-xs text-(--color-muted)">{format!("{count} {}", if count == 1 { "variety" } else { "varieties" })}</span>
                        </button>
                        <div class="flex shrink-0 flex-wrap items-center gap-1.5">
                            <button type="button" class="rounded bg-emerald-50 px-2 py-0.5 text-xs font-semibold text-(--color-brand-dark) transition-colors hover:bg-emerald-100"
                                on:click=move |_| {
                                    m.expand(&id2);
                                    m.editing.set(Some(VarietyForm {
                                        subcategory_id: id2.clone(),
                                        unit: (if is_machine { "min" } else if bought { "piece" } else { "sq ft" }).into(),
                                        default_rate: Decimal::from(if is_machine { 15 } else if bought { 100 } else { 50 }),
                                        default_wastage_pct: Decimal::from(if bought { 0 } else if is_machine { 5 } else { 10 }),
                                        ..Default::default()
                                    }));
                                }>"+ Variety"</button>
                            <button type="button" title="Move this subcategory to another category" class="rounded bg-sky-50 px-2 py-0.5 text-xs font-semibold text-sky-700 transition-colors hover:bg-sky-100"
                                on:click=move |_| m.transfer.set(Some((sub_store.get_value(), false, cid1.clone())))>"Move"</button>
                            <button type="button" title="Duplicate this subcategory and its varieties" class="rounded bg-indigo-50 px-2 py-0.5 text-xs font-semibold text-indigo-700 transition-colors hover:bg-indigo-100"
                                on:click=move |_| m.transfer.set(Some((sub_store.get_value(), true, cid2.clone())))>"Duplicate"</button>
                            <button type="button" title="Rename Subcategory" class="rounded bg-slate-100 px-2 py-0.5 text-xs font-semibold text-slate-700 transition-colors hover:bg-slate-200" on:click=move |_| renaming.set(true)>"Rename"</button>
                            <button type="button" title="Delete Subcategory" class="px-1 text-xs text-red-500 transition-colors hover:text-red-700"
                                on:click=move |_| if confirm(&format!("Are you sure you want to delete subcategory \"{nm2}\" and all its varieties?")) {
                                    m.run(actions::delete_subcategory(id3.clone()), format!("Subcategory \"{nm2}\" deleted."), || {});
                                }>"Delete"</button>
                        </div>
                    }.into_any()
                }}
            </div>
            <Show when=move || open.get()>
                <VarietyTable m=m sub=sub_store.get_value() />
            </Show>
        </div>
    }
}

#[component]
fn VarietyTable(m: Master, sub: SubcategoryNode) -> impl IntoView {
    let sub_id = sub.row.id.clone();
    view! {
        <div class="min-w-0 max-w-full overflow-x-auto">
            <table class="w-full min-w-125 text-left text-xs">
                <thead>
                    <tr class="border-b border-slate-100 font-medium text-(--color-muted)">
                        <th class="pb-1 font-semibold">"Variety / Spec"</th>
                        <th class="pb-1 font-semibold">"Default Rate"</th>
                        <th class="pb-1 font-semibold">"Unit"</th>
                        <th class="pb-1 font-semibold">"Default Wastage %"</th>
                        <th class="pb-1 font-semibold">"Notes"</th>
                        <th class="pb-1 text-right font-semibold">"Actions"</th>
                    </tr>
                </thead>
                <tbody class="divide-y divide-slate-50">
                    {if sub.varieties.is_empty() {
                        view! { <tr><td colspan="6" class="py-2 italic text-(--color-muted)">{format!("No varieties in {} yet. Use “+ Variety” above.", sub.row.name)}</td></tr> }.into_any()
                    } else {
                        sub.varieties.into_iter().map(|v| {
                            let form = VarietyForm {
                                id: Some(v.id.clone()),
                                subcategory_id: sub_id.clone(),
                                name: v.name.clone(),
                                default_rate: v.default_rate,
                                unit: v.unit.clone(),
                                default_wastage_pct: v.default_wastage_pct,
                                notes: v.notes.clone(),
                                sort_order: v.sort_order,
                            };
                            let (id, nm) = (v.id.clone(), v.name.clone());
                            view! {
                                <tr class="transition-colors hover:bg-slate-50">
                                    <td class="py-1.5 font-semibold text-(--color-ink)">{v.name.clone()}</td>
                                    <td class="py-1.5 font-mono font-semibold">{format_money(Some(v.default_rate))}</td>
                                    <td class="py-1.5 text-(--color-muted)">{v.unit.clone()}</td>
                                    <td class="py-1.5 text-(--color-muted)">{format!("{}%", num_text(v.default_wastage_pct))}</td>
                                    <td class="py-1.5 text-(--color-muted)">{v.notes.clone().unwrap_or_else(|| "—".into())}</td>
                                    <td class="py-1.5 text-right">
                                        <div class="flex items-center justify-end gap-2">
                                            <button type="button" class="text-xs text-(--color-brand) hover:underline" on:click=move |_| m.editing.set(Some(form.clone()))>"Edit"</button>
                                            <button type="button" class="text-xs text-red-600 hover:underline"
                                                on:click=move |_| if confirm(&format!("Are you sure you want to delete variety \"{nm}\"?")) {
                                                    m.run(actions::delete_variety(id.clone()), format!("Variety \"{nm}\" deleted."), || {});
                                                }>"Delete"</button>
                                        </div>
                                    </td>
                                </tr>
                            }
                        }).collect_view().into_any()
                    }}
                </tbody>
            </table>
        </div>
    }
}

/// Add or edit one variety under a subcategory.
#[component]
fn VarietyDialog(m: Master, form: VarietyForm) -> impl IntoView {
    let name = RwSignal::new(form.name.clone());
    let rate = RwSignal::new(num_text(form.default_rate));
    let unit = RwSignal::new(if form.unit.is_empty() { "sq ft".to_string() } else { form.unit.clone() });
    let wastage = RwSignal::new(num_text(form.default_wastage_pct));
    let sort = RwSignal::new(form.sort_order.to_string());
    let notes = RwSignal::new(form.notes.clone().unwrap_or_default());
    let editing = form.id.is_some();
    let original_unit = form.unit.clone();
    let submit = move |e: leptos::ev::SubmitEvent| {
        e.prevent_default();
        let f = VarietyForm {
            name: name.get_untracked(),
            default_rate: parse_num(&rate.get_untracked()),
            unit: unit.get_untracked(),
            default_wastage_pct: parse_num(&wastage.get_untracked()),
            sort_order: sort.get_untracked().trim().parse().unwrap_or(0),
            notes: Some(notes.get_untracked()),
            ..form.clone()
        };
        m.run(actions::save_variety(f), "Variety / Specification saved successfully!".into(), move || {
            m.editing.set(None)
        });
    };
    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4 backdrop-blur-xs">
            <form on:submit=submit class="card w-full max-w-md space-y-4 bg-white p-5 shadow-2xl">
                <h3 class="text-base font-bold text-(--color-ink)">{if editing { "Edit Variety / Specification" } else { "New Variety / Specification" }}</h3>
                <div>
                    <label class="label" for="variety-name">"Variety / Specification Name *"</label>
                    <input id="variety-name" class="input mt-1 text-sm font-semibold" placeholder="e.g. 8mm, 12mm, 10x2mm, Matt" required bind:value=name />
                </div>
                <div class="grid grid-cols-2 gap-3">
                    <div>
                        <label class="label" for="variety-rate">"Default Rate (₹) *"</label>
                        <input id="variety-rate" type="number" step="any" required class="input input-num mt-1 font-mono text-sm" bind:value=rate />
                    </div>
                    <div>
                        <label class="label" for="variety-unit">"Unit *"</label>
                        <select id="variety-unit" class="select mt-1 text-sm" required on:change=move |e| unit.set(event_target_value(&e))>
                            {COMMON_UNITS.iter().map(|u| view! { <option value=*u selected=move || unit.get() == *u>{*u}</option> }).collect_view()}
                            {(!original_unit.is_empty() && !COMMON_UNITS.contains(&original_unit.as_str())).then(|| view! { <option value=original_unit.clone() selected=true>{original_unit.clone()}</option> })}
                        </select>
                    </div>
                </div>
                {editing.then(|| view! {
                    <p class="-mt-2 rounded-md bg-amber-50 px-2 py-1.5 text-[11px] text-amber-900">
                        "A new rate or unit reprices every product costed with this variety. Quotations and invoices keep their prices."
                    </p>
                })}
                <div class="grid grid-cols-2 gap-3">
                    <div>
                        <label class="label" for="variety-wastage">"Default Wastage %"</label>
                        <input id="variety-wastage" type="number" step="any" class="input input-num mt-1 text-sm" bind:value=wastage />
                    </div>
                    <div>
                        <label class="label" for="variety-sort">"Sort Order"</label>
                        <input id="variety-sort" type="number" class="input input-num mt-1 text-sm" bind:value=sort />
                    </div>
                </div>
                <div>
                    <label class="label" for="variety-notes">"Notes / Specs"</label>
                    <input id="variety-notes" class="input mt-1 text-sm" placeholder="e.g. Grade B/BB Russian Birch, 12mm thickness" bind:value=notes />
                </div>
                <div class="flex items-center justify-end gap-2 border-t border-(--color-line) pt-3">
                    <button type="button" class="btn-secondary text-sm" on:click=move |_| m.editing.set(None)>"Cancel"</button>
                    <button type="submit" class="btn-primary text-sm" disabled=move || m.busy.get()>{move || if m.busy.get() { "Saving…" } else { "Save variety" }}</button>
                </div>
            </form>
        </div>
    }
}

/// Move a subcategory to another category, or duplicate it (with its varieties).
#[component]
fn TransferDialog(m: Master, sub: SubcategoryNode, duplicate: bool, current: String) -> impl IntoView {
    let stages = m.rows.get_untracked().map(|r| build_cost_tree(&r)).unwrap_or_default();
    let all: Vec<(String, String)> =
        stages.iter().flat_map(|s| s.categories.iter().map(|c| (c.row.id.clone(), c.row.name.clone()))).collect();
    let mode_dup = RwSignal::new(duplicate);
    let others: Vec<String> = all.iter().filter(|(id, _)| *id != current).map(|(id, _)| id.clone()).collect();
    let target = RwSignal::new(if !duplicate {
        others.first().cloned().unwrap_or_else(|| current.clone())
    } else {
        current.clone()
    });
    let copy_name = format!("{} (Copy)", sub.row.name);
    let dup_name =
        RwSignal::new(if target.get_untracked() == current { copy_name.clone() } else { sub.row.name.clone() });
    let current_name =
        all.iter().find(|(id, _)| *id == current).map_or_else(|| "Unknown".to_string(), |(_, n)| n.clone());
    let (cur, cur2, cur3) = (current.clone(), current.clone(), current.clone());
    let move_invalid = move || !mode_dup.get() && (target.get().is_empty() || target.get() == cur);
    let dup_invalid = move || mode_dup.get() && (target.get().is_empty() || dup_name.get().trim().is_empty());
    let (sub_name, sub_id) = (sub.row.name.clone(), sub.row.id.clone());
    let set_mode = {
        let others = others.clone();
        move |dup: bool| {
            mode_dup.set(dup);
            if !dup
                && target.get_untracked() == cur2
                && let Some(o) = others.first()
            {
                target.set(o.clone());
            }
        }
    };
    let set_mode2 = set_mode.clone();
    let change_target = {
        let (sn, cn) = (sub_name.clone(), copy_name.clone());
        move |id: String| {
            let n = dup_name.get_untracked();
            if mode_dup.get_untracked() && (n.is_empty() || n == sn || n == cn) {
                dup_name.set(if id == cur3 { cn.clone() } else { sn.clone() });
            }
            target.set(id);
        }
    };
    let (mi, di) = (move_invalid.clone(), dup_invalid);
    let submit = move |e: leptos::ev::SubmitEvent| {
        e.prevent_default();
        let (sid, tid) = (sub_id.clone(), target.get_untracked());
        if mode_dup.get_untracked() {
            if di() {
                return;
            }
            let t2 = tid.clone();
            m.run(
                actions::duplicate_subcategory(sid, tid, dup_name.get_untracked().trim().to_string()),
                "Subcategory duplicated successfully!".into(),
                move || {
                    m.transfer.set(None);
                    m.expand(&t2);
                },
            );
        } else {
            if mi() {
                return;
            }
            let (t2, s2) = (tid.clone(), sid.clone());
            m.run(actions::move_subcategory(sid, tid), "Subcategory moved successfully!".into(), move || {
                m.transfer.set(None);
                m.expand(&t2);
                m.expand(&s2);
            });
        }
    };
    let tab = |active: bool, color: &'static str| {
        format!(
            "rounded-md px-2.5 py-1 transition-colors {}",
            if active {
                format!("bg-white font-bold {color} shadow-xs")
            } else {
                "text-slate-600 hover:text-slate-900".to_string()
            }
        )
    };
    let current4 = current.clone();
    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4 backdrop-blur-xs">
            <form on:submit=submit class="card w-full max-w-lg space-y-4 bg-white p-5 shadow-2xl">
                <div class="flex items-center justify-between border-b border-(--color-line) pb-3">
                    <div>
                        <h3 class="text-base font-bold text-(--color-ink)">{move || if mode_dup.get() { "Duplicate Subcategory" } else { "Move Subcategory" }}</h3>
                        <p class="mt-0.5 text-xs text-(--color-muted)">
                            "Source: " <span class="font-semibold text-(--color-ink)">{sub_name.clone()}</span> " in "
                            <span class="font-medium text-(--color-brand-dark)">{current_name}</span>
                        </p>
                    </div>
                    <div class="inline-flex rounded-lg bg-slate-100 p-1 text-xs font-medium">
                        <button type="button" class=move || tab(!mode_dup.get(), "text-sky-700") on:click=move |_| set_mode(false)>"Move"</button>
                        <button type="button" class=move || tab(mode_dup.get(), "text-indigo-700") on:click=move |_| set_mode2(true)>"Duplicate"</button>
                    </div>
                </div>
                <div>
                    <label class="label" for="transfer-dest">"Destination Category *"</label>
                    <select id="transfer-dest" class="select mt-1 w-full text-sm" required on:change=move |e| change_target(event_target_value(&e))>
                        {stages.into_iter().map(|s| {
                            let label = format!("{} ({})", s.row.name, s.categories.len());
                            let cur = current4.clone();
                            view! {
                                <optgroup label=label>
                                    {s.categories.into_iter().map(|c| {
                                        let id = c.row.id.clone();
                                        let is_current = c.row.id == cur;
                                        view! {
                                            <option value=c.row.id.clone() selected=move || target.get() == id disabled=move || !mode_dup.get() && is_current>
                                                {format!("{}{}", c.row.name, if is_current { " (Current Category)" } else { "" })}
                                            </option>
                                        }
                                    }).collect_view()}
                                </optgroup>
                            }
                        }).collect_view()}
                    </select>
                    <Show when=move_invalid.clone()>
                        <p class="mt-1 text-xs text-amber-600">"Please choose a different category to move this subcategory to."</p>
                    </Show>
                </div>
                <Show when=move || mode_dup.get()>
                    <div>
                        <label class="label" for="transfer-name">"New Subcategory Name *"</label>
                        <input id="transfer-name" class="input mt-1 w-full text-sm font-semibold" placeholder="Subcategory name" required bind:value=dup_name />
                    </div>
                </Show>
                <div class="space-y-2 rounded-lg border border-slate-200 bg-slate-50 p-3">
                    <div class="flex items-center justify-between text-xs">
                        <span class="font-semibold text-slate-700">{format!("Varieties ({}):", sub.varieties.len())}</span>
                        <span class="text-[11px] text-(--color-muted)">{move || if mode_dup.get() { "Will be duplicated with rates & specs" } else { "Will be relocated with the subcategory" }}</span>
                    </div>
                    {if sub.varieties.is_empty() {
                        view! { <p class="text-xs italic text-(--color-muted)">"No varieties in this subcategory."</p> }.into_any()
                    } else {
                        view! {
                            <div class="flex max-h-24 flex-wrap gap-1.5 overflow-y-auto">
                                {sub.varieties.iter().map(|v| view! {
                                    <span class="inline-flex items-center rounded-md border border-slate-200 bg-white px-2 py-0.5 text-[11px] font-medium text-slate-700">
                                        {v.name.clone()}<span class="ml-1 text-[10px] text-slate-400">{format!("({})", v.unit)}</span>
                                    </span>
                                }).collect_view()}
                            </div>
                        }.into_any()
                    }}
                </div>
                <div class="flex items-center justify-end gap-2 border-t border-(--color-line) pt-3">
                    <button type="button" class="btn-secondary text-sm" disabled=move || m.busy.get() on:click=move |_| m.transfer.set(None)>"Cancel"</button>
                    <button type="submit" class=move || format!("btn-primary text-sm {}", if mode_dup.get() { "border-indigo-700 bg-indigo-600 hover:bg-indigo-700" } else { "" })
                        disabled=move || m.busy.get() || move_invalid() || dup_invalid()>
                        {move || match (m.busy.get(), mode_dup.get()) {
                            (true, false) => "Moving…",
                            (true, true) => "Duplicating…",
                            (false, false) => "Move Subcategory",
                            (false, true) => "Duplicate Subcategory",
                        }}
                    </button>
                </div>
            </form>
        </div>
    }
}

#[component]
fn ColorsCard(colors: Vec<ProductColor>) -> impl IntoView {
    let list = RwSignal::new(colors);
    let new_name = RwSignal::new(String::new());
    let new_hex = RwSignal::new(FALLBACK_COLOR_HEX.to_string());
    let busy = RwSignal::new(false);
    let msg = RwSignal::new(String::new());
    // A swatch picker fires on every drag step: save once it settles.
    let timer = StoredValue::new(0u32);
    let save = move |updated: Vec<ProductColor>, text: &'static str| {
        busy.set(true);
        spawn_local(async move {
            let res = actions::save_colors(&updated).await;
            busy.set(false);
            msg.set(match res {
                Ok(()) => text.to_string(),
                Err(e) => e.to_string(),
            });
            set_timeout(move || msg.set(String::new()), std::time::Duration::from_secs(3));
        });
    };
    let add = move || {
        let trimmed = new_name.get_untracked().trim().to_string();
        if trimmed.is_empty() {
            return;
        }
        if list.with_untracked(|l| l.iter().any(|c| c.name.eq_ignore_ascii_case(&trimmed))) {
            return msg.set("That colour is already in the list.".into());
        }
        let mut updated = list.get_untracked();
        updated.push(ProductColor { code: color_code(&trimmed), name: trimmed, hex: new_hex.get_untracked() });
        list.set(updated.clone());
        new_name.set(String::new());
        new_hex.set(FALLBACK_COLOR_HEX.into());
        save(updated, "Colour added.");
    };
    view! {
        <div class="card p-4">
            <div class="flex flex-wrap items-center justify-between gap-2">
                <div>
                    <h3 class="text-base font-semibold text-(--color-ink)">"Colours & finishes"</h3>
                    <p class="mt-0.5 text-xs text-(--color-muted)">"Walnut, Natural and Black are built in. Each colour’s 2-letter code goes on the end of its product codes."</p>
                </div>
                <span class="rounded-full border border-emerald-200 bg-emerald-50 px-2.5 py-1 text-xs font-bold text-emerald-800">{move || format!("{} colours", list.with(Vec::len))}</span>
            </div>
            <div class="mt-3 grid gap-2 sm:grid-cols-3">
                {move || list.get().into_iter().map(|c| {
                    let (n1, n2, n3) = (c.name.clone(), c.name.clone(), c.name.clone());
                    let standard = STANDARD_PRODUCT_COLORS.iter().any(|s| s.name == c.name);
                    view! {
                        <div class="flex items-center gap-3 rounded-xl border border-slate-200 bg-white p-3 shadow-xs">
                            <label title=format!("Pick the {} swatch", c.name) class="relative h-7 w-7 shrink-0 cursor-pointer rounded-full border border-black/20 shadow-xs" style=format!("background-color: {}", c.hex)>
                                <input type="color" value=c.hex.clone() aria-label=format!("{} swatch color", c.name) class="absolute inset-0 h-full w-full cursor-pointer opacity-0"
                                    on:input=move |e| {
                                        let hex = event_target_value(&e);
                                        list.update(|l| if let Some(x) = l.iter_mut().find(|x| x.name == n1) { x.hex = hex });
                                        let id = timer.get_value() + 1;
                                        timer.set_value(id);
                                        set_timeout(move || if timer.get_value() == id { save(list.get_untracked(), "Colour updated.") }, std::time::Duration::from_millis(400));
                                    } />
                            </label>
                            <div class="min-w-0 flex-1">
                                <div class="flex items-center justify-between gap-2">
                                    <span class="text-sm font-bold text-(--color-ink)">{n2}</span>
                                    <span class="flex items-center gap-1">
                                        <span title=format!("Product codes end in /{}", c.code) class="rounded bg-slate-100 px-1.5 py-0.5 font-mono text-xs font-black text-slate-800">{c.code.clone()}</span>
                                        {(!standard).then(|| view! {
                                            <button type="button" title=format!("Remove {}", c.name) class="px-1 text-slate-400 transition-colors hover:text-red-600"
                                                on:click=move |_| { let updated: Vec<_> = list.get_untracked().into_iter().filter(|x| x.name != n3).collect(); list.set(updated.clone()); save(updated, "Colour removed."); }>"×"</button>
                                        })}
                                    </span>
                                </div>
                            </div>
                        </div>
                    }
                }).collect_view()}
            </div>
            <div class="mt-3 flex max-w-sm items-center gap-2">
                <input type="color" title="Swatch for the new colour" aria-label="Swatch for the new colour" class="h-8 w-10 shrink-0 cursor-pointer rounded border border-slate-300 bg-white p-0.5" bind:value=new_hex />
                <input class="input py-1.5 text-xs" placeholder="New colour, e.g. Teak" bind:value=new_name on:keydown=move |e| if e.key() == "Enter" { e.prevent_default(); add(); } />
                <button type="button" class="btn-primary whitespace-nowrap py-1.5 text-xs" disabled=move || busy.get() || new_name.get().trim().is_empty() on:click=move |_| add()>"Add colour"</button>
            </div>
            <Show when=move || !msg.get().is_empty()>
                <p class="mt-2 text-xs font-medium text-emerald-700">{move || msg.get()}</p>
            </Show>
        </div>
    }
}

/// One click to bring every saved costing up to the master's current rates.
#[component]
fn SyncRatesButton() -> impl IntoView {
    let busy = RwSignal::new(false);
    let result = RwSignal::new(None::<Result<String, String>>);
    let sync = move |_| {
        if !confirm(
            "Update every product costed with an old rate? Their cost and selling price are recalculated (each keeps its markup). Quotes and invoices don't change.",
        ) {
            return;
        }
        result.set(None);
        busy.set(true);
        spawn_local(async move {
            let r = actions::sync_all_rates().await.map_err(|e| e.to_string());
            busy.set(false);
            result.set(Some(r));
        });
    };
    view! {
        <div class="card flex flex-wrap items-center justify-between gap-3 p-4">
            <div>
                <h3 class="text-base font-semibold text-(--color-ink)">"Products on old rates"</h3>
                <p class="mt-0.5 text-xs text-(--color-muted)">"Changing a rate updates products automatically. Use this for products costed before a change."</p>
                {move || result.get().map(|r| match r {
                    Ok(m) => view! { <p class="mt-1.5 text-xs font-medium text-emerald-700">{m}</p> }.into_any(),
                    Err(e) => view! { <p class="mt-1.5 text-xs font-medium text-red-700">{e}</p> }.into_any(),
                })}
            </div>
            <button type="button" class="btn-secondary text-xs" disabled=move || busy.get() on:click=sync>{move || if busy.get() { "Updating…" } else { "Update products" }}</button>
        </div>
    }
}
