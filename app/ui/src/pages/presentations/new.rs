//! `/presentations/new` (port of `presentations/new/*`).

use super::{DeckItem, DeckText, ItemPicker, PickerData, SlideInput, build_slides, deck_href, load_picker};
use crate::components::PageHeader;
use crate::data::use_settings;
use crate::db::{self, Q, insert, insert_returning, live_with};
use lattice_core::api::ApiError;
use lattice_core::canvas::deck::DEFAULT_NOTE;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;
use serde::Deserialize;
use serde_json::{Value, json};

/// Saves a deck laid out here, returning its id (port of `createPresentation`).
async fn create_presentation(title: &str, slides: Vec<SlideInput>) -> Result<String, ApiError> {
    let title = title.trim();
    if title.is_empty() {
        return Err(ApiError::Invalid("Give the presentation a title".into()));
    }
    if slides.len() < 3 {
        return Err(ApiError::Invalid("Pick at least one hamper or product".into()));
    }
    #[derive(Deserialize)]
    struct Id {
        id: String,
    }
    let deck = insert_returning::<_, Id>("presentations", &json!({ "title": title })).await?.into_iter().next().ok_or_else(|| ApiError::Server("The presentation was not created.".into()))?;
    let rows: Vec<Value> = slides
        .into_iter()
        .enumerate()
        .map(|(position, s)| json!({ "presentation_id": deck.id, "position": position, "kind": s.kind, "hamper_id": s.hamper_id, "product_id": s.product_id, "canvas": s.canvas }))
        .collect();
    if let Err(e) = insert("presentation_slides", &rows, None, None).await {
        let _ = Q::from("presentations").eq("id", &deck.id).delete().await;
        return Err(e);
    }
    db::cache_drop(&["presentations"]).await;
    Ok(deck.id)
}

#[component]
pub fn NewPresentation() -> impl IntoView {
    let picker = live_with("deck_picker", load_picker);
    let settings = use_settings();
    view! {
        <PageHeader title="New presentation" subtitle="Pick hampers and products. Every slide opens in the photo editor afterwards.">""</PageHeader>
        {move || match (picker.data.get(), settings.data.get()) {
            (Some(p), Some(s)) => {
                let c = s.company;
                let contact = [c.phone, c.address].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join("\n");
                view! { <Form picker=p contact=contact /> }.into_any()
            }
            _ => match picker.error.get() {
                Some(e) => view! { <div class="card p-6 text-sm text-red-800">{e.to_string()}</div> }.into_any(),
                None => view! { <div class="card h-96 animate-pulse bg-white/60"></div> }.into_any(),
            },
        }}
    }
}

#[component]
fn Form(picker: PickerData, contact: String) -> impl IntoView {
    let navigate = StoredValue::new_local(use_navigate());
    let (title, subtitle) = (RwSignal::new("Premium".to_string()), RwSignal::new("Giveaway Hamper".to_string()));
    let (heading, tagline) = (RwSignal::new("Thank You".to_string()), RwSignal::new("for your time".to_string()));
    let contact = RwSignal::new(contact);
    let note = RwSignal::new(DEFAULT_NOTE.to_string());
    let items = RwSignal::new(Vec::<DeckItem>::new());
    let pending = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let submit = move || {
        error.set(None);
        pending.set(true);
        let text = DeckText {
            title: title.get_untracked(),
            subtitle: subtitle.get_untracked(),
            closing_heading: heading.get_untracked(),
            closing_tagline: tagline.get_untracked(),
            closing_contact: contact.get_untracked(),
        };
        let (picked, note) = (items.get_untracked(), note.get_untracked());
        spawn_local(async move {
            let res = async {
                let slides = build_slides(&picked, &note, Some(&text)).await?;
                let name = [text.title.trim(), text.subtitle.trim()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ");
                create_presentation(&name, slides).await
            }
            .await;
            pending.set(false);
            match res {
                Ok(id) => navigate.with_value(|nav| nav(&deck_href(&id), Default::default())),
                Err(e) => error.set(Some(e.to_string())),
            }
        });
    };
    let field = |id: &'static str, label: &'static str, v: RwSignal<String>| view! {
        <div>
            <label class="label" for=id>{label}</label>
            <input id=id class="input mt-1" prop:value=v on:input=move |e| v.set(event_target_value(&e)) />
        </div>
    };
    view! {
        <div class="space-y-4">
            <div class="grid gap-4 lg:grid-cols-3">
                <section class="card space-y-3 p-4">
                    <h2 class="text-sm font-semibold">"Entry slide"</h2>
                    {field("title", "Title", title)}
                    {field("subtitle", "Subtitle", subtitle)}
                </section>
                <section class="card space-y-3 p-4">
                    <h2 class="text-sm font-semibold">"Hamper & product slides"</h2>
                    <div>
                        <label class="label" for="note">"Terms note (bottom right)"</label>
                        <textarea id="note" rows="4" class="input mt-1" prop:value=note on:input=move |e| note.set(event_target_value(&e))></textarea>
                    </div>
                    <p class="text-xs text-(--color-muted)">"Each slide shows the product photos on the green band with their names, and the price with “+ Tax”."</p>
                </section>
                <section class="card space-y-3 p-4">
                    <h2 class="text-sm font-semibold">"Ending slide"</h2>
                    <div class="grid grid-cols-2 gap-2">
                        {field("closingHeading", "Heading", heading)}
                        {field("closingTagline", "Tagline", tagline)}
                    </div>
                    <div>
                        <label class="label" for="closingContact">"Contact (under “For any Query”)"</label>
                        <textarea id="closingContact" rows="3" class="input mt-1" prop:value=contact on:input=move |e| contact.set(event_target_value(&e))></textarea>
                    </div>
                </section>
            </div>
            <ItemPicker data=picker value=items />
            {move || error.get().map(|e| view! { <p class="rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-800">{e}</p> })}
            <div class="flex flex-wrap items-center gap-2">
                <button type="button" class="btn-primary" disabled=move || pending.get() || items.with(Vec::is_empty) || title.with(|t| t.trim().is_empty()) on:click=move |_| submit()>
                    {move || if pending.get() { "Laying out slides…".to_string() } else { format!("Create presentation ({} slides)", items.with(Vec::len) + 2) }}
                </button>
                <A href="/presentations" attr:class="btn-secondary">"Cancel"</A>
            </div>
        </div>
    }
}
