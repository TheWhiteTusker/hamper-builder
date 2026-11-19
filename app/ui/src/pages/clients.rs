//! Clients (port of `(app)/clients/*`): the companies picked on quotations
//! and proforma invoices.

use crate::components::PageHeader;
use crate::db::{self, Q, insert_returning, live, me};
use crate::kit::{GstinField, confirm};
use lattice_core::api::ApiError;
use lattice_core::gstin;
use lattice_core::records::Client;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::NavigateOptions;
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_query_map};
use serde_json::json;

#[component]
pub fn Clients() -> impl IntoView {
    let clients = live::<Client>("clients", Q::from("clients").select("*").order("name"));
    let query = use_query_map();
    let navigate = StoredValue::new_local(use_navigate());
    let param = move |k: &str| query.with(|m| m.get(k).unwrap_or_default());
    let set_q = move |v: String| {
        let mut m = query.get_untracked();
        if v.is_empty() {
            m.remove("q");
        } else {
            m.replace("q", v);
        }
        let qs = m.to_query_string();
        navigate.with_value(|nav| nav(&format!("/clients{qs}"), NavigateOptions { replace: true, scroll: false, ..Default::default() }));
    };
    let rows = Memo::new(move |_| {
        let q = param("q").trim().to_lowercase();
        clients.data.with(|all| {
            all.as_ref().map(|all| {
                all.iter()
                    .filter(|c| q.is_empty() || c.name.to_lowercase().contains(&q) || c.gstin.as_deref().is_some_and(|g| g.to_lowercase().contains(&q)))
                    .cloned()
                    .collect::<Vec<_>>()
            })
        })
    });
    let subtitle = move || {
        let n = clients.data.with(|d| d.as_ref().map_or(0, Vec::len));
        format!("{n} client{} ·picked on quotations and proforma invoices", if n == 1 { "" } else { "s" })
    };
    // The form starts again only when the client being edited changes (or the
    // list first arrives), never on a background refresh mid-typing.
    let resets = RwSignal::new(0u32);
    let editing_key = Memo::new(move |_| (param("edit"), clients.data.with(Option::is_some), resets.get()));

    view! {
        <PageHeader title="Clients" subtitle=Signal::derive(subtitle)>""</PageHeader>
        <div class="grid items-start gap-4 lg:grid-cols-[400px_1fr]">
            {move || {
                let (id, _, _) = editing_key.get();
                let client = clients.data.with_untracked(|d| d.iter().flatten().find(|c| c.id == id).cloned());
                view! { <ClientForm client=client clients=clients.data resets=resets /> }
            }}
            <div>
                <div class="card mb-4 flex items-end gap-3 p-3">
                    <div class="flex-1">
                        <label class="label" for="q">"Search"</label>
                        <input id="q" placeholder="Company name or GSTIN" class="input mt-1" prop:value=move || param("q") on:input=move |e| set_q(event_target_value(&e)) />
                    </div>
                </div>
                {move || match rows.get() {
                    None => view! { <div class="card h-40 animate-pulse bg-white/60"></div> }.into_any(),
                    Some(list) if list.is_empty() => view! {
                        <p class="card px-4 py-8 text-center text-sm text-(--color-muted)">
                            {if param("q").trim().is_empty() { "No clients yet. Add one on the left." } else { "No clients match that search." }}
                        </p>
                    }.into_any(),
                    Some(list) => view! {
                        <div class="card overflow-x-auto">
                            <table class="w-full border-collapse text-sm">
                                <thead><tr><th class="th">"Company"</th><th class="th">"GSTIN"</th><th class="th">"Contact"</th><th class="th"></th></tr></thead>
                                <tbody>
                                    {list.into_iter().map(|c| {
                                        let contact = [c.contact_person.as_deref(), c.phone.as_deref(), c.email.as_deref()]
                                            .into_iter().flatten().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · ");
                                        view! {
                                            <tr class="align-top">
                                                <td class="td">
                                                    <div class="font-medium">{c.name.clone()}</div>
                                                    {c.billing_address.clone().filter(|a| !a.is_empty()).map(|a| view! { <div class="whitespace-pre-line text-xs text-(--color-muted)">{a}</div> })}
                                                </td>
                                                <td class="td whitespace-nowrap font-mono">{c.gstin.clone().filter(|g| !g.is_empty()).unwrap_or_else(|| "—".into())}</td>
                                                <td class="td text-(--color-muted)">{if contact.is_empty() { "—".to_string() } else { contact }}</td>
                                                <td class="td num">
                                                    <A href=format!("/clients?edit={}", c.id) attr:class="font-medium text-(--color-brand) hover:underline">"Edit"</A>
                                                </td>
                                            </tr>
                                        }
                                    }).collect_view()}
                                </tbody>
                            </table>
                        </div>
                    }.into_any(),
                }}
            </div>
        </div>
    }
}

fn text(s: String) -> Option<String> {
    Some(s.trim().to_string()).filter(|s| !s.is_empty())
}

#[component]
fn ClientForm(client: Option<Client>, clients: RwSignal<Option<Vec<Client>>>, resets: RwSignal<u32>) -> impl IntoView {
    let can_delete = me().role.can_manage();
    let navigate = StoredValue::new_local(use_navigate());
    let go = move |to: &str| navigate.with_value(|nav| nav(to, Default::default()));
    let field = |f: fn(&Client) -> Option<String>| RwSignal::new(client.as_ref().and_then(f).unwrap_or_default());
    let (gstin_v, name, address) = (field(|c| c.gstin.clone()), field(|c| Some(c.name.clone())), field(|c| c.billing_address.clone()));
    let (contact, phone, email) = (field(|c| c.contact_person.clone()), field(|c| c.phone.clone()), field(|c| c.email.clone()));
    let id = StoredValue::new(client.as_ref().map(|c| c.id.clone()));
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(None::<&'static str>);
    let editing = client.is_some();

    let save = move || {
        error.set(None);
        let n = name.get_untracked().trim().to_string();
        let g = gstin_v.get_untracked().trim().to_uppercase();
        if n.is_empty() {
            return error.set(Some("Company name is required".into()));
        }
        if !g.is_empty() && !gstin::is_valid(&g) {
            return error.set(Some("That GSTIN does not look right (15 characters).".into()));
        }
        let values = json!({
            "name": n,
            "gstin": text(g),
            "billing_address": text(address.get_untracked()),
            "contact_person": text(contact.get_untracked()),
            "phone": text(phone.get_untracked()),
            "email": text(email.get_untracked()),
        });
        busy.set(Some("save"));
        let target = id.get_value();
        spawn_local(async move {
            let res: Result<Vec<Client>, ApiError> = match &target {
                Some(id) => Q::from("clients")
                    .eq("id", id)
                    .update(&values, Some("*"))
                    .await
                    .and_then(|v| serde_json::from_value(v).map_err(|e| ApiError::Server(e.to_string()))),
                None => insert_returning("clients", &values).await,
            };
            busy.set(None);
            match res {
                Ok(saved) => {
                    // Shown at once; the list's next refresh confirms it.
                    clients.update(|list| {
                        if let Some(list) = list {
                            for c in saved {
                                list.retain(|x| x.id != c.id);
                                list.push(c);
                            }
                            list.sort_by_key(|c| c.name.to_lowercase());
                        }
                    });
                    db::cache_drop(&["quote_options"]).await;
                    resets.update(|n| *n += 1);
                    go("/clients");
                }
                Err(e) => error.set(Some(e.to_string())),
            }
        });
    };
    let delete = move || {
        let Some(target) = id.get_value() else { return };
        if !confirm(&format!("Delete {}? Quotations keep the details they were sent with.", name.get_untracked())) {
            return;
        }
        busy.set(Some("delete"));
        spawn_local(async move {
            let res = Q::from("clients").eq("id", &target).delete().await;
            busy.set(None);
            match res {
                Ok(_) => {
                    clients.update(|list| {
                        if let Some(list) = list {
                            list.retain(|c| c.id != target);
                        }
                    });
                    db::cache_drop(&["quote_options"]).await;
                    resets.update(|n| *n += 1);
                    go("/clients");
                }
                Err(e) => error.set(Some(e.to_string())),
            }
        });
    };
    let input = move |id: &'static str, label: &'static str, value: RwSignal<String>, kind: &'static str| {
        view! {
            <div>
                <label class="label" for=id>{label}</label>
                <input id=id type=kind class="input mt-1" prop:value=value on:input=move |e| value.set(event_target_value(&e)) />
            </div>
        }
    };

    view! {
        <div class="card p-4">
            <h2 class="text-sm font-semibold">{if editing { "Edit client" } else { "New client" }}</h2>
            <div class="mt-3 space-y-3">
                <GstinField id="client-gstin" label="GST number" value=gstin_v can_edit=true on_found=move |n: String, a: String| {
                    name.set(n);
                    address.set(a);
                } />
                {input("client-name", "Company name *", name, "text")}
                <div>
                    <label class="label" for="client-address">"Billing address"</label>
                    <textarea id="client-address" rows="3" class="input mt-1" prop:value=address on:input=move |e| address.set(event_target_value(&e))></textarea>
                </div>
                <div class="grid gap-3 sm:grid-cols-2">
                    {input("client-contact", "Contact person", contact, "text")}
                    {input("client-phone", "Phone", phone, "text")}
                    <div class="sm:col-span-2">{input("client-email", "Email", email, "email")}</div>
                </div>
                {move || error.get().map(|e| view! { <p role="alert" class="text-sm text-red-700">{e}</p> })}
                <div class="flex items-center gap-2">
                    <button type="button" class="btn-primary" disabled=move || busy.get().is_some() on:click=move |_| save()>
                        {move || match busy.get() {
                            Some("save") => "Saving…",
                            _ if editing => "Save changes",
                            _ => "Add client",
                        }}
                    </button>
                    {editing.then(|| view! { <A href="/clients" attr:class="btn-secondary">"Cancel"</A> })}
                </div>
            </div>
            {(editing && can_delete).then(|| view! {
                <div class="mt-4 border-t border-(--color-line) pt-3">
                    <button type="button" class="btn-danger" disabled=move || busy.get().is_some() on:click=move |_| delete()>
                        {move || if busy.get() == Some("delete") { "Deleting…" } else { "Delete client" }}
                    </button>
                </div>
            })}
        </div>
    }
}
