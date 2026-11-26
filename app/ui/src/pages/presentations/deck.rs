//! `/presentations/<id>`: the deck's slides as a draft (add, remove,
//! reorder, rename), saved in one go; PDF and PowerPoint downloads (port of
//! `presentations/[id]/*` and `export-pptx.ts`).

use super::{DeckItem, ItemPicker, PickerData, SlideInput, build_slides, deck_href, load_picker};
use crate::components::PageHeader;
use crate::db::{self, Q, insert_returning, list_files, live_with, remove_files};
use crate::kit::confirm;
use crate::studio::{Thumb, render, uuid};
use lattice_core::api::ApiError;
use lattice_core::canvas::deck::{DEFAULT_NOTE, blank_slide, ppt_text_box};
use lattice_core::canvas::{Canvas, Shape, layer_label, parse_canvas};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_params_map};
use serde::Deserialize;
use serde_json::{Value, json};

pub fn kind_label(kind: &str) -> &'static str {
    match kind {
        "cover" => "Entry slide",
        "hamper" => "Hamper",
        "product" => "Product",
        "closing" => "Ending slide",
        "blank" => "Blank",
        _ => "Slide",
    }
}

/// A slide on the page: saved ones have a database id; ones added since the
/// last save carry their full row.
#[derive(Clone, Debug, PartialEq)]
struct Draft {
    id: String,
    kind: String,
    canvas: Canvas,
    unsaved: Option<SlideInput>,
}

fn slide_name(d: &Draft) -> String {
    d.canvas
        .layers
        .iter()
        .find(|l| l.kind() == "text" && l.base.name.as_deref() == Some("Title"))
        .map(layer_label)
        .unwrap_or_else(|| kind_label(&d.kind).to_string())
}

/// New slides go before a trailing ending slide.
fn insert_before_closing(list: &mut Vec<Draft>, added: Vec<Draft>) {
    let at = if list.last().is_some_and(|s| s.kind == "closing") { list.len() - 1 } else { list.len() };
    list.splice(at..at, added);
}

fn draft_of(s: SlideInput) -> Draft {
    Draft { id: format!("new-{}", uuid()), kind: s.kind.clone(), canvas: s.canvas.clone(), unsaved: Some(s) }
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, Deserialize)]
struct SlideRow {
    id: String,
    kind: String,
    canvas: Value,
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, Deserialize)]
struct DeckData {
    id: String,
    title: String,
    slides: Vec<SlideRow>,
}

async fn load_deck(id: String) -> Result<Option<DeckData>, ApiError> {
    #[derive(Deserialize)]
    struct Head {
        id: String,
        title: String,
    }
    let Some(head) = Q::from("presentations").select("id,title").eq("id", &id).one::<Head>().await? else {
        return Ok(None);
    };
    let slides = Q::from("presentation_slides")
        .select("id,kind,canvas")
        .eq("presentation_id", &id)
        .order("position")
        .rows()
        .await?;
    Ok(Some(DeckData { id: head.id, title: head.title, slides }))
}

/// Saves the draft in one go: the title, which slides remain, their order,
/// and slides added. Returns the ids given to added slides, by temporary id
/// (port of `saveDeck`).
///
/// ponytail: several statements, not one transaction; an RPC if two people
/// editing one deck at once becomes real.
async fn save_deck(deck_id: &str, title: &str, slides: &[Draft]) -> Result<Vec<(String, String)>, ApiError> {
    let title = title.trim();
    if title.is_empty() {
        return Err(ApiError::Invalid("The title can't be empty.".into()));
    }
    #[derive(Deserialize)]
    struct Pos {
        id: String,
        position: i64,
    }
    let existing: Vec<Pos> =
        Q::from("presentation_slides").select("id,position").eq("presentation_id", deck_id).rows().await?;
    let kept: Vec<&str> = slides.iter().filter(|s| s.unsaved.is_none()).map(|s| s.id.as_str()).collect();
    if kept.iter().any(|id| !existing.iter().any(|e| e.id == *id)) {
        return Err(ApiError::Conflict(
            "Some slides were changed somewhere else. Reload the page and make your changes again.".into(),
        ));
    }
    let removed: Vec<String> =
        existing.iter().filter(|e| !kept.contains(&e.id.as_str())).map(|e| e.id.clone()).collect();
    if !removed.is_empty() {
        Q::from("presentation_slides").in_list("id", &removed).delete().await?;
    }
    let added: Vec<(usize, &Draft)> = slides.iter().enumerate().filter(|(_, s)| s.unsaved.is_some()).collect();
    let mut ids = Vec::new();
    if !added.is_empty() {
        let rows: Vec<Value> = added
            .iter()
            .filter_map(|(position, d)| d.unsaved.as_ref().map(|s| json!({ "presentation_id": deck_id, "position": position, "kind": s.kind, "hamper_id": s.hamper_id, "product_id": s.product_id, "canvas": s.canvas })))
            .collect();
        let inserted: Vec<Pos> = insert_returning("presentation_slides", &rows).await?;
        for row in inserted {
            if let Some((_, d)) = added.iter().find(|(p, _)| *p as i64 == row.position) {
                ids.push((d.id.clone(), row.id));
            }
        }
    }
    for (position, s) in slides.iter().enumerate().filter(|(_, s)| s.unsaved.is_none()) {
        if existing.iter().find(|e| e.id == s.id).is_some_and(|e| e.position != position as i64) {
            Q::from("presentation_slides").eq("id", &s.id).update(&json!({ "position": position }), None).await?;
        }
    }
    Q::from("presentations").eq("id", deck_id).update(&json!({ "title": title }), None).await?;
    db::cache_drop(&["presentations", &format!("deck:{deck_id}")]).await;
    Ok(ids)
}

async fn delete_presentation(id: &str) -> Result<(), ApiError> {
    Q::from("presentations").eq("id", id).delete().await?;
    let folder = format!("presentations/{id}");
    if let Ok(files) = list_files(&folder).await {
        let _ = remove_files(&files).await;
    }
    db::cache_drop(&["presentations", &format!("deck:{id}")]).await;
    Ok(())
}

/// Builds the .pptx natively, one slide at a time: each slide's shapes,
/// photos and background become one picture; its text stays editable.
async fn export_pptx(file_name: &str, slides: &[Canvas], progress: RwSignal<Option<String>>) -> Result<bool, ApiError> {
    let Some(first) = slides.first() else { return Ok(false) };
    crate::tauri::invoke::<_, ()>("pptx_begin", &json!({ "width_px": first.width, "height_px": first.height })).await?;
    for (i, c) in slides.iter().enumerate() {
        progress.set(Some(format!("Building slide {} of {}…", i + 1, slides.len())));
        let (png, texts) = render::render_png(c, 1.0, true).await?;
        let boxes: Vec<(String, lattice_core::canvas::deck::PptTextBox)> = texts
            .iter()
            .filter_map(|(l, h)| match &l.shape {
                Shape::Text(t) if !t.text.trim().is_empty() => Some((t.text.clone(), ppt_text_box(&l.base, t, *h))),
                _ => None,
            })
            .collect();
        let json = serde_json::to_vec(&boxes).map_err(|e| ApiError::Server(e.to_string()))?;
        let mut body = (json.len() as u32).to_be_bytes().to_vec();
        body.extend(json);
        body.extend(png);
        crate::tauri::invoke_raw::<()>("pptx_add", &body, &[]).await?;
    }
    progress.set(Some("Saving file…".into()));
    let path: Option<String> = crate::tauri::invoke("pptx_save", &json!({ "file_name": file_name })).await?;
    Ok(path.is_some())
}

#[component]
pub fn DeckOverview() -> impl IntoView {
    let params = use_params_map();
    view! {
        {move || {
            let id = params.with(|p| p.get("id")).unwrap_or_default();
            let i = id.clone();
            let deck = live_with(format!("deck:{id}"), move || load_deck(i.clone()));
            let picker = live_with("deck_picker", load_picker);
            // The page starts from the first copy and then owns its draft.
            let first = RwSignal::new(None::<Option<DeckData>>);
            Effect::new(move |_| {
                if first.with_untracked(Option::is_none)
                    && let Some(d) = deck.data.get()
                {
                    first.set(Some(d));
                }
            });
            let navigate = StoredValue::new_local(use_navigate());
            Effect::new(move |_| {
                if first.with(|f| matches!(f, Some(None))) {
                    // Deleted, or a stale link.
                    navigate.with_value(|nav| nav("/presentations", Default::default()));
                }
            });
            view! {
                {move || match (first.get(), picker.data.get()) {
                    (Some(Some(d)), Some(p)) => view! { <Overview deck=d picker=p fresh=deck.data /> }.into_any(),
                    _ => match deck.error.get() {
                        Some(e) => view! { <div class="card p-6 text-sm text-red-800">{e.to_string()}</div> }.into_any(),
                        None => view! { <div class="card h-96 animate-pulse bg-white/60"></div> }.into_any(),
                    },
                }}
            }
        }}
    }
}

#[component]
fn Overview(deck: DeckData, picker: PickerData, fresh: RwSignal<Option<Option<DeckData>>>) -> impl IntoView {
    let navigate = StoredValue::new_local(use_navigate());
    let deck_id = StoredValue::new(deck.id.clone());
    let drafts_of = |d: &DeckData| -> Vec<Draft> {
        d.slides
            .iter()
            .map(|s| Draft { id: s.id.clone(), kind: s.kind.clone(), canvas: parse_canvas(&s.canvas), unsaved: None })
            .collect()
    };
    let saved_of =
        |d: &DeckData| (d.title.clone(), d.slides.iter().map(|s| s.id.clone()).collect::<Vec<_>>().join(","));
    let initial = drafts_of(&deck);
    let slides = RwSignal::new(initial.clone());
    let initial = StoredValue::new(initial);
    let title = RwSignal::new(deck.title.clone());
    let saved = RwSignal::new(saved_of(&deck));
    let error = RwSignal::new(None::<String>);
    let notice = RwSignal::new(None::<String>);
    let exporting = RwSignal::new(None::<String>);
    let saving = RwSignal::new(false);
    let deleting = RwSignal::new(false);
    let adding = RwSignal::new(false);
    let to_add = RwSignal::new(Vec::<DeckItem>::new());
    let laying_out = RwSignal::new(false);
    let ids_now = move || slides.with(|s| s.iter().map(|d| d.id.clone()).collect::<Vec<_>>().join(","));
    let dirty = move || saved.with(|(t, ids)| title.with(|x| x.trim() != t) || ids_now() != *ids);
    crate::leave::warn_unsaved(move || untrack(dirty));
    // Someone else's save arrives with a refresh: taken unless there are
    // unsaved changes here. The first copy is the one the page opened with.
    Effect::new(move |opened: Option<()>| {
        let Some(Some(d)) = fresh.get() else { return };
        if opened.is_none() || untrack(dirty) {
            return;
        }
        let drafts = drafts_of(&d);
        initial.set_value(drafts.clone());
        slides.set(drafts);
        title.set(d.title.clone());
        saved.set(saved_of(&d));
    });
    let edit = move |f: &dyn Fn(&mut Vec<Draft>)| {
        slides.update(|s| f(s));
        error.set(None);
        notice.set(None);
    };
    let shift = move |id: String, later: bool| {
        edit(&|list| {
            if let Some(i) = list.iter().position(|d| d.id == id) {
                let j = if later { i + 1 } else { i.wrapping_sub(1) };
                if j < list.len() {
                    list.swap(i, j);
                }
            }
        });
    };
    let save = move || {
        error.set(None);
        notice.set(None);
        saving.set(true);
        let (list, t) = (slides.get_untracked(), title.get_untracked());
        spawn_local(async move {
            let res = save_deck(&deck_id.get_value(), &t, &list).await;
            saving.set(false);
            match res {
                Err(e) => error.set(Some(e.to_string())),
                Ok(new_ids) => {
                    let next: Vec<Draft> = list
                        .into_iter()
                        .map(|d| match new_ids.iter().find(|(tmp, _)| *tmp == d.id) {
                            Some((_, real)) => Draft { id: real.clone(), unsaved: None, ..d },
                            None => d,
                        })
                        .collect();
                    saved.set((t.trim().to_string(), next.iter().map(|d| d.id.clone()).collect::<Vec<_>>().join(",")));
                    slides.set(next);
                    notice.set(Some("Saved.".into()));
                }
            }
        });
    };
    let discard = move || {
        if confirm("Discard your unsaved changes?") {
            slides.set(initial.get_value());
            title.set(saved.with_untracked(|s| s.0.clone()));
        }
    };
    let add_items = move || {
        error.set(None);
        laying_out.set(true);
        let picked = to_add.get_untracked();
        spawn_local(async move {
            let res = build_slides(&picked, DEFAULT_NOTE, None).await;
            laying_out.set(false);
            match res {
                Ok(built) => {
                    edit(&|list| insert_before_closing(list, built.iter().cloned().map(draft_of).collect()));
                    to_add.set(Vec::new());
                    adding.set(false);
                }
                Err(e) => error.set(Some(e.to_string())),
            }
        });
    };
    let download_pptx = move || {
        error.set(None);
        let name = title.with_untracked(|t| t.trim().to_string());
        let name = if name.is_empty() { saved.with_untracked(|s| s.0.clone()) } else { name };
        let canvases: Vec<Canvas> = slides.with_untracked(|s| s.iter().map(|d| d.canvas.clone()).collect());
        exporting.set(Some("Starting…".into()));
        spawn_local(async move {
            if export_pptx(&name, &canvases, exporting).await.is_err() {
                error.set(Some(
                    "Could not build the PowerPoint. An image from another website may be blocking the export.".into(),
                ));
            }
            exporting.set(None);
        });
    };
    let remove_deck = move || {
        if !confirm(&format!("Delete \"{}\" and all its slides?", saved.with_untracked(|s| s.0.clone()))) {
            return;
        }
        deleting.set(true);
        spawn_local(async move {
            match delete_presentation(&deck_id.get_value()).await {
                Ok(()) => navigate.with_value(|nav| nav("/presentations", Default::default())),
                Err(e) => {
                    deleting.set(false);
                    error.set(Some(e.to_string()));
                }
            }
        });
    };
    // Leaving for a slide editor would lose the draft; ask first.
    let go = move |href: String| {
        if dirty() && !confirm("You have unsaved changes to this presentation. Leave without saving?") {
            return;
        }
        navigate.with_value(|nav| nav(&href, Default::default()));
    };
    let picker = StoredValue::new(picker);

    view! {
        <PageHeader title=Signal::derive(move || saved.with(|s| s.0.clone())) subtitle=Signal::derive(move || format!("{} slides{}", slides.with(Vec::len), if dirty() { " · unsaved changes" } else { "" }))>
            <Show when=dirty><button type="button" class="btn-secondary" disabled=move || saving.get() on:click=move |_| discard()>"Discard"</button></Show>
            <button type="button" class="btn-primary" disabled=move || !dirty() || saving.get() || title.with(|t| t.trim().is_empty()) on:click=move |_| save()>
                {move || if saving.get() { "Saving…" } else if dirty() { "Save changes" } else { "Saved" }}
            </button>
            {move || if dirty() {
                view! { <span class="btn-secondary cursor-not-allowed opacity-50" title="Save your changes first">"Download PDF"</span> }.into_any()
            } else {
                view! { <A href=format!("{}/print", deck_href(&deck_id.get_value())) attr:class="btn-secondary">"Download PDF"</A> }.into_any()
            }}
            <button type="button" class="btn-secondary" disabled=move || exporting.get().is_some() || slides.with(Vec::is_empty) on:click=move |_| download_pptx()>
                {move || exporting.get().unwrap_or_else(|| "Download PowerPoint".into())}
            </button>
        </PageHeader>
        {move || error.get().map(|e| view! { <p class="mb-4 rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-800">{e}</p> })}
        {move || notice.get().filter(|_| !dirty()).map(|n| view! { <p class="mb-4 rounded-md border border-green-200 bg-green-50 px-3 py-2 text-sm text-green-900">{n}</p> })}
        <div class="card mb-4 flex flex-wrap items-end gap-3 p-3">
            <div class="min-w-64 flex-1">
                <label class="label" for="deck-title">"Presentation name"</label>
                <input id="deck-title" class="input mt-1" prop:value=title on:input=move |e| { title.set(event_target_value(&e)); notice.set(None); } />
            </div>
            <button type="button" class="btn-danger ml-auto" disabled=move || deleting.get() on:click=move |_| remove_deck()>
                {move || if deleting.get() { "Deleting…" } else { "Delete presentation" }}
            </button>
        </div>
        <p class="mb-3 text-sm text-(--color-muted)">"Add, remove and reorder slides, then click "<strong>"Save changes"</strong>". Click a slide to edit it in the photo editor."</p>
        <div class="grid grid-cols-[repeat(auto-fill,minmax(260px,1fr))] gap-4">
            <For each=move || slides.get() key=|d| d.id.clone() let:d>
                {
                    let id = d.id.clone();
                    let pos = Memo::new({ let id = id.clone(); move |_| slides.with(|s| (s.iter().position(|x| x.id == id).unwrap_or(0), s.len())) });
                    let canvas = d.canvas.clone();
                    let href = format!("{}/slides/{}", deck_href(&deck_id.get_value()), d.id);
                    let unsaved = d.unsaved.is_some();
                    let (i1, i2, i3) = (id.clone(), id.clone(), id.clone());
                    view! {
                        <div class=if unsaved { "card overflow-hidden ring-2 ring-(--color-gold)" } else { "card overflow-hidden" }>
                            {if unsaved {
                                view! {
                                    <div class="relative bg-(--color-paper) p-2" title="Save changes to edit this slide">
                                        <Thumb canvas=Signal::stored(canvas) width=240.0 class="mx-auto rounded shadow-sm" />
                                        <span class="absolute top-3 right-3 rounded bg-(--color-ink) px-1.5 py-0.5 text-[11px] text-white">"New · save to edit"</span>
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <button type="button" class="block w-full bg-(--color-paper) p-2 hover:opacity-90" on:click=move |_| go(href.clone())>
                                        <Thumb canvas=Signal::stored(canvas) width=240.0 class="mx-auto rounded shadow-sm" />
                                    </button>
                                }.into_any()
                            }}
                            <div class="flex items-center gap-2 px-3 py-2">
                                <span class="w-6 text-sm text-(--color-muted) tabular-nums">{move || pos.get().0 + 1}</span>
                                <div class="min-w-0 flex-1">
                                    <div class="truncate text-sm font-medium">{slide_name(&d)}</div>
                                    <div class="text-xs text-(--color-muted)">{kind_label(&d.kind)}</div>
                                </div>
                                <button type="button" aria-label="Move earlier" class="px-1 disabled:opacity-30" disabled=move || pos.get().0 == 0 on:click=move |_| shift(i1.clone(), false)>"←"</button>
                                <button type="button" aria-label="Move later" class="px-1 disabled:opacity-30" disabled=move || { let (i, n) = pos.get(); i + 1 >= n } on:click=move |_| shift(i2.clone(), true)>"→"</button>
                                <button type="button" aria-label="Remove slide" class="px-1 text-red-700" on:click=move |_| { let id = i3.clone(); edit(&|list| list.retain(|x| x.id != id)); }>"×"</button>
                            </div>
                        </div>
                    }
                }
            </For>
        </div>
        <section class="mt-6 space-y-3">
            <div class="flex flex-wrap items-center gap-2">
                <button type="button" class="btn-secondary" on:click=move |_| adding.update(|a| *a = !*a)>{move || if adding.get() { "Close" } else { "Add hamper or product slides" }}</button>
                <button type="button" class="btn-secondary" on:click=move |_| edit(&|list| insert_before_closing(list, vec![draft_of(SlideInput { kind: "blank".into(), hamper_id: None, product_id: None, canvas: blank_slide() })]))>
                    "Add blank slide"
                </button>
            </div>
            <Show when=move || adding.get()>
                <ItemPicker data=picker.get_value() value=to_add />
                <button type="button" class="btn-primary" disabled=move || laying_out.get() || to_add.with(Vec::is_empty) on:click=move |_| add_items()>
                    {move || if laying_out.get() { "Laying out slides…".to_string() } else { let n = to_add.with(Vec::len); format!("Add {n} slide{}", if n == 1 { "" } else { "s" }) }}
                </button>
                <p class="text-xs text-(--color-muted)">"New slides go before the ending slide. They are saved when you click Save changes."</p>
            </Show>
        </section>
    }
}
