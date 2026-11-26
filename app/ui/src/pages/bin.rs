//! The Recycle Bin (port of `(app)/bin/*`): binned hampers, products and
//! photos, kept 30 days; restore, delete for good, empty, purge expired.

use crate::components::PageHeader;
use crate::db::{self, Q, list_files, live_with, me, remove_files};
use crate::kit::{FallbackImg, confirm, dstr};
use futures::join;
use lattice_core::Decimal;
use lattice_core::api::ApiError;
use lattice_core::bin_expiry::calculate_expiry;
use lattice_core::images::{thumb_path, thumb_url, view_path};
use lattice_core::lenient;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;
use leptos_router::hooks::use_query_map;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Kind {
    Hamper,
    Product,
    Image,
}

impl Kind {
    fn key(self) -> &'static str {
        match self {
            Self::Hamper => "hamper",
            Self::Product => "product",
            Self::Image => "image",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Hamper => "Hamper",
            Self::Product => "Product",
            Self::Image => "Photo",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct BinItem {
    id: String,
    kind: Kind,
    title: String,
    code: Option<String>,
    subtitle: String,
    image_url: Option<String>,
    deleted_at: String,
    /// Unix ms, for sorting; days left are worked out from it on screen.
    deleted_ms: i64,
    /// A photo's product: deleting that product for good takes the photo too.
    product_id: Option<String>,
}

#[derive(Deserialize)]
struct HamperRow {
    id: String,
    code: String,
    name: String,
    collection: Option<String>,
    status: String,
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    final_catalogue_sp: Option<Decimal>,
    image_url: Option<String>,
    deleted_at: String,
}

#[derive(Deserialize)]
struct Named {
    name: Option<String>,
    code: Option<String>,
}

#[derive(Deserialize)]
struct ProductRow {
    id: String,
    code: String,
    name: String,
    #[serde(default, deserialize_with = "lenient::decimal")]
    default_sp: Decimal,
    #[serde(default, deserialize_with = "lenient::decimal")]
    cost_price: Decimal,
    image_url: Option<String>,
    deleted_at: String,
    categories: Option<Named>,
}

#[derive(Deserialize)]
struct ImageRow {
    id: String,
    product_id: Option<String>,
    url: Option<String>,
    color: Option<String>,
    deleted_at: String,
    products: Option<Named>,
}

fn ms_of(ts: &str) -> i64 {
    OffsetDateTime::parse(ts, &Rfc3339).map_or(0, |t| i64::try_from(t.unix_timestamp_nanos() / 1_000_000).unwrap_or(0))
}

fn now() -> OffsetDateTime {
    let nanos = (js_sys::Date::now() * 1_000_000.0) as i128;
    OffsetDateTime::from_unix_timestamp_nanos(nanos).unwrap_or(OffsetDateTime::UNIX_EPOCH)
}

fn days_left(deleted_ms: i64) -> i64 {
    let deleted = OffsetDateTime::from_unix_timestamp_nanos(i128::from(deleted_ms) * 1_000_000)
        .unwrap_or(OffsetDateTime::UNIX_EPOCH);
    calculate_expiry(deleted, now()).days_remaining
}

async fn load_bin() -> Result<Vec<BinItem>, ApiError> {
    let (h, p, i) = (
        Q::from("hampers")
            .select("id,code,name,collection,status,final_catalogue_sp,image_url,deleted_at")
            .not_null("deleted_at")
            .order_desc("deleted_at"),
        Q::from("products")
            .select("id,code,name,default_sp,cost_price,image_url,deleted_at,categories(name)")
            .not_null("deleted_at")
            .order_desc("deleted_at"),
        Q::from("product_images")
            .select("id,product_id,url,storage_path,color,is_primary,deleted_at,products(name,code)")
            .not_null("deleted_at")
            .order_desc("deleted_at"),
    );
    let (hampers, products, images) = join!(h.rows::<HamperRow>(), p.rows::<ProductRow>(), i.rows::<ImageRow>());
    let mut items: Vec<BinItem> = Vec::new();
    for h in hampers? {
        let collection = h.collection.filter(|c| !c.is_empty()).map(|c| format!("{c} • ")).unwrap_or_default();
        let sp = h.final_catalogue_sp.map_or_else(|| "—".to_string(), dstr);
        items.push(BinItem {
            subtitle: format!("{collection}{} • SP: ₹{sp}", h.status),
            deleted_ms: ms_of(&h.deleted_at),
            id: h.id,
            kind: Kind::Hamper,
            title: h.name,
            code: Some(h.code),
            image_url: h.image_url,
            deleted_at: h.deleted_at,
            product_id: None,
        });
    }
    for p in products? {
        let cat =
            p.categories.and_then(|c| c.name).filter(|c| !c.is_empty()).map(|c| format!("{c} • ")).unwrap_or_default();
        items.push(BinItem {
            subtitle: format!("{cat}SP: ₹{} • Cost: ₹{}", dstr(p.default_sp), dstr(p.cost_price)),
            deleted_ms: ms_of(&p.deleted_at),
            id: p.id,
            kind: Kind::Product,
            title: p.name,
            code: Some(p.code),
            image_url: p.image_url,
            deleted_at: p.deleted_at,
            product_id: None,
        });
    }
    for img in images? {
        let (name, code) = img.products.map(|p| (p.name, p.code)).unwrap_or_default();
        let name = name.filter(|n| !n.is_empty());
        let color = img.color.filter(|c| !c.is_empty()).map(|c| format!(" • Color: {c}")).unwrap_or_default();
        items.push(BinItem {
            title: name.as_ref().map_or_else(|| "Product Photo".to_string(), |n| format!("Photo for {n}")),
            subtitle: format!(
                "Product: {} ({}){color}",
                name.as_deref().unwrap_or("Unknown"),
                code.as_deref().unwrap_or("")
            ),
            deleted_ms: ms_of(&img.deleted_at),
            id: img.id,
            kind: Kind::Image,
            code,
            image_url: img.url,
            deleted_at: img.deleted_at,
            product_id: img.product_id,
        });
    }
    // Newest deletions first.
    items.sort_by_key(|i| std::cmp::Reverse(i.deleted_ms));
    Ok(items)
}

/// Other screens' copies that a restore or delete makes stale.
async fn forget() {
    db::cache_drop(&["bin", "hampers", "products", "catalog", "dashboard", "quote_options"]).await;
}

fn may_act(kind: Kind, manage: bool, admin: bool) -> bool {
    if kind == Kind::Product { admin } else { manage }
}

async fn restore(item: &BinItem) -> Result<String, ApiError> {
    #[derive(Deserialize)]
    struct Coded {
        code: String,
        name: String,
    }
    let role = me().role;
    match item.kind {
        Kind::Hamper | Kind::Product => {
            let table = if item.kind == Kind::Hamper { "hampers" } else { "products" };
            if item.kind == Kind::Hamper && !role.can_manage() {
                return Err(ApiError::Forbidden("You don't have permission to restore hampers.".into()));
            }
            if item.kind == Kind::Product && !role.is_admin() {
                return Err(ApiError::Forbidden("Only administrators can restore products.".into()));
            }
            let row =
                Q::from(table).select("id,code,name").eq("id", &item.id).not_null("deleted_at").one::<Coded>().await?;
            let Some(row) = row else {
                return Err(ApiError::NotFound(format!("{} not found in Bin.", item.kind.label())));
            };
            // Codes are unique among active rows only: a new one may have taken it since.
            let taken = Q::from(table).select("id").eq("code", &row.code).null("deleted_at").one::<Value>().await?;
            if taken.is_some() {
                return Err(ApiError::Conflict(if item.kind == Kind::Hamper {
                    format!(
                        "Cannot restore: an active hamper with code \"{}\" already exists. Please rename or delete it first.",
                        row.code
                    )
                } else {
                    format!("Cannot restore: an active product with code \"{}\" already exists.", row.code)
                }));
            }
            Q::from(table).eq("id", &item.id).update(&json!({ "deleted_at": null }), None).await?;
            Ok(format!("{} \"{}\" ({}) restored.", item.kind.label(), row.name, row.code))
        }
        Kind::Image => {
            if !role.can_manage() {
                return Err(ApiError::Forbidden("You don't have permission to restore photos.".into()));
            }
            #[derive(Deserialize)]
            struct Img {
                product_id: String,
                url: String,
            }
            let Some(img) = Q::from("product_images")
                .select("id,product_id,url")
                .eq("id", &item.id)
                .not_null("deleted_at")
                .one::<Img>()
                .await?
            else {
                return Err(ApiError::NotFound("Photo not found in Bin.".into()));
            };
            Q::from("product_images").eq("id", &item.id).update(&json!({ "deleted_at": null }), None).await?;
            // A product left without a photo gets this one as its primary.
            #[derive(Deserialize)]
            struct Prod {
                image_url: Option<String>,
            }
            let prod = Q::from("products").select("image_url").eq("id", &img.product_id).one::<Prod>().await?;
            if prod.is_some_and(|p| p.image_url.is_none()) {
                Q::from("products").eq("id", &img.product_id).update(&json!({ "image_url": img.url }), None).await?;
                Q::from("product_images").eq("id", &item.id).update(&json!({ "is_primary": true }), None).await?;
            }
            Ok("Product photo restored.".into())
        }
    }
}

#[derive(Deserialize)]
struct StoredPath {
    storage_path: Option<String>,
}

/// Deletes one binned row for good, files first (an orphaned file is harmless;
/// a row pointing at a missing file is not).
async fn delete_for_good(kind: Kind, id: &str) -> Result<(), ApiError> {
    match kind {
        Kind::Hamper => {
            remove_files(&list_files(&format!("hampers/{id}")).await?).await?;
            Q::from("hampers").eq("id", id).delete().await?;
        }
        Kind::Product => {
            let images: Vec<StoredPath> =
                Q::from("product_images").select("storage_path").eq("product_id", id).rows().await?;
            remove_files(
                &images.into_iter().filter_map(|i| i.storage_path).filter(|p| !p.is_empty()).collect::<Vec<_>>(),
            )
            .await?;
            remove_files(&list_files(&format!("products/{id}")).await?).await?;
            Q::from("product_images").eq("product_id", id).delete().await?;
            Q::from("products").eq("id", id).delete().await?;
        }
        Kind::Image => {
            let img = Q::from("product_images").select("storage_path").eq("id", id).one::<StoredPath>().await?;
            if let Some(p) = img.and_then(|i| i.storage_path).filter(|p| !p.is_empty()) {
                remove_files(&[thumb_path(&p), view_path(&p), p]).await?;
            }
            Q::from("product_images").eq("id", id).delete().await?;
        }
    }
    Ok(())
}

fn deny_message(kind: Kind) -> &'static str {
    match kind {
        Kind::Hamper => "You don't have permission to permanently delete hampers.",
        Kind::Product => "Only administrators can permanently delete products.",
        Kind::Image => "You don't have permission to delete photos.",
    }
}

/// Every binned item of `kinds` this user may delete. Returns how many went.
async fn empty_bin(kinds: &[Kind]) -> Result<(usize, Vec<Kind>), ApiError> {
    let role = me().role;
    let (mut count, mut emptied) = (0, Vec::new());
    for &kind in kinds {
        if !may_act(kind, role.can_manage(), role.is_admin()) {
            continue;
        }
        emptied.push(kind);
        let table = match kind {
            Kind::Hamper => "hampers",
            Kind::Product => "products",
            Kind::Image => "product_images",
        };
        #[derive(Deserialize)]
        struct Id {
            id: String,
        }
        for row in Q::from(table).select("id").not_null("deleted_at").rows::<Id>().await? {
            delete_for_good(kind, &row.id).await?;
            count += 1;
        }
    }
    Ok((count, emptied))
}

/// Port of `purgeExpiredAction`: files of everything past 30 days, then
/// `purge_expired_bin_records()` for the rows (the Worker's cron does the same nightly).
async fn purge_expired() -> Result<(), ApiError> {
    if !me().role.can_manage() {
        return Err(ApiError::Forbidden("Permission denied.".into()));
    }
    let cutoff = String::from(js_sys::Date::new(&(js_sys::Date::now() - 30.0 * 86_400_000.0).into()).to_iso_string());
    #[derive(Deserialize)]
    struct Expired {
        id: String,
        #[serde(default)]
        storage_path: Option<String>,
    }
    let old = |table: &str, cols: &str| Q::from(table).select(cols).not_null("deleted_at").lte("deleted_at", &cutoff);
    for img in old("product_images", "id,storage_path").rows::<Expired>().await? {
        if let Some(p) = img.storage_path.filter(|p| !p.is_empty()) {
            remove_files(&[thumb_path(&p), view_path(&p), p]).await?;
        }
    }
    for (table, dir) in [("hampers", "hampers"), ("products", "products")] {
        for row in old(table, "id").rows::<Expired>().await? {
            remove_files(&list_files(&format!("{dir}/{}", row.id)).await?).await?;
        }
    }
    db::rpc::<_, Value>("purge_expired_bin_records", &json!({})).await?;
    Ok(())
}

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// `toLocaleDateString("en-IN", { day: "numeric", month: "short", year: "numeric" })`.
fn short_date(ms: i64) -> String {
    let d = js_sys::Date::new(&(ms as f64).into());
    let month = MONTHS.get(d.get_month() as usize).copied().unwrap_or("");
    format!("{} {month} {}", d.get_date(), d.get_full_year())
}

#[component]
pub fn Bin() -> impl IntoView {
    let profile = me();
    let (manage, admin) = (profile.role.can_manage(), profile.role.is_admin());
    let bin = live_with("bin", load_bin);
    let query = use_query_map();
    // `/bin?tab=hampers` (the link after binning a hamper) opens on that tab.
    let tab = RwSignal::new(match query.with_untracked(|q| q.get("tab")).as_deref() {
        Some("hampers") => Some(Kind::Hamper),
        Some("products") => Some(Kind::Product),
        Some("images" | "photos") => Some(Kind::Image),
        _ => None,
    });
    let search = RwSignal::new(String::new());
    let notice = RwSignal::new(None::<(String, bool)>);
    let pending = RwSignal::new(false);
    let items = Memo::new(move |_| bin.data.get().unwrap_or_default());
    let count = move |k: Option<Kind>| items.with(|i| i.iter().filter(|x| k.is_none_or(|k| x.kind == k)).count());
    let expired = move || items.with(|i| i.iter().filter(|x| days_left(x.deleted_ms) == 0).count());
    let shown = Memo::new(move |_| {
        let q = search.get().trim().to_lowercase();
        let t = tab.get();
        items.with(|all| {
            all.iter()
                .filter(|i| t.is_none_or(|t| i.kind == t))
                .filter(|i| {
                    q.is_empty()
                        || i.title.to_lowercase().contains(&q)
                        || i.code.as_deref().unwrap_or("").to_lowercase().contains(&q)
                        || i.subtitle.to_lowercase().contains(&q)
                })
                .cloned()
                .collect::<Vec<_>>()
        })
    });
    // Drops rows on screen; a product deleted for good takes its binned photos.
    let drop_rows = move |gone: &dyn Fn(&BinItem) -> bool| {
        bin.patch(|list| {
            let products: Vec<String> =
                list.iter().filter(|i| i.kind == Kind::Product && gone(i)).map(|i| i.id.clone()).collect();
            list.retain(|i| {
                !gone(i) && !(i.kind == Kind::Image && i.product_id.as_ref().is_some_and(|p| products.contains(p)))
            });
        });
    };
    let run = move |work: std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, ApiError>>>>,
                    gone: Box<dyn Fn(&BinItem) -> bool>| {
        pending.set(true);
        spawn_local(async move {
            let res = work.await;
            pending.set(false);
            match res {
                Ok(msg) => {
                    drop_rows(&*gone);
                    forget().await;
                    notice.set(Some((msg, false)));
                }
                Err(e) => notice.set(Some((e.to_string(), true))),
            }
        });
    };
    let on_restore = move |item: BinItem| {
        let (id, kind) = (item.id.clone(), item.kind);
        run(Box::pin(async move { restore(&item).await }), Box::new(move |i| i.id == id && i.kind == kind));
    };
    let on_delete = move |item: BinItem| {
        if !confirm(&format!(
            "Permanently delete \"{}\"? This will delete all its data and storage files and CANNOT be undone.",
            item.title
        )) {
            return;
        }
        let (id, kind) = (item.id.clone(), item.kind);
        let role = me().role;
        let allowed = may_act(kind, role.can_manage(), role.is_admin());
        run(
            Box::pin(async move {
                if !allowed {
                    return Err(ApiError::Forbidden(deny_message(kind).into()));
                }
                delete_for_good(kind, &id).await?;
                Ok(format!("{} permanently deleted.", kind.label()))
            }),
            Box::new(move |i| i.id == item.id && i.kind == kind),
        );
    };
    let on_empty = move || {
        let t = tab.get_untracked();
        let scope = match t {
            None => "all items",
            Some(Kind::Hamper) => "all deleted hampers",
            Some(Kind::Product) => "all deleted products",
            Some(Kind::Image) => "all deleted photos",
        };
        if !confirm(&format!(
            "Permanently delete {scope} from the Bin? This will delete all records and stored images permanently and cannot be undone."
        )) {
            return;
        }
        let kinds = t.map_or_else(|| vec![Kind::Hamper, Kind::Product, Kind::Image], |k| vec![k]);
        let emptied = std::rc::Rc::new(std::cell::RefCell::new(Vec::<Kind>::new()));
        let out = emptied.clone();
        run(
            Box::pin(async move {
                let (n, kinds) = empty_bin(&kinds).await?;
                *out.borrow_mut() = kinds;
                Ok(format!("Successfully deleted {n} item{} permanently.", if n == 1 { "" } else { "s" }))
            }),
            Box::new(move |i| emptied.borrow().contains(&i.kind)),
        );
    };
    let on_purge = move || {
        run(
            Box::pin(async move {
                purge_expired().await?;
                Ok("Expired items have been permanently purged.".to_string())
            }),
            Box::new(|i| days_left(i.deleted_ms) == 0),
        );
    };
    let tabs = [
        (None, "All Items"),
        (Some(Kind::Hamper), "Hampers"),
        (Some(Kind::Product), "Products"),
        (Some(Kind::Image), "Photos"),
    ];

    view! {
        <div class="space-y-4">
            <PageHeader title="Recycle Bin" subtitle=Signal::derive(move || { let n = count(None); format!("{n} item{} in bin", if n == 1 { "" } else { "s" }) })>
                <A href="/settings" attr:class="btn-secondary">"Settings"</A>
            </PageHeader>
            {move || notice.get().map(|(text, error)| view! {
                <div role="status" class=if error { "flex items-center justify-between rounded-lg border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800" } else { "flex items-center justify-between rounded-lg border border-(--color-line) bg-(--color-paper) px-4 py-3 text-sm text-(--color-ink)" }>
                    <div class="flex items-center gap-2">
                        <span class=if error { "inline-block h-2 w-2 rounded-full bg-red-500" } else { "inline-block h-2 w-2 rounded-full bg-(--color-brand)" }></span>
                        <span>{text}</span>
                    </div>
                    <button type="button" class="text-xs font-semibold uppercase hover:underline" on:click=move |_| notice.set(None)>"Dismiss"</button>
                </div>
            })}
            <div class="card flex flex-wrap items-center justify-between gap-3 border-(--color-line) bg-white p-4">
                <div class="text-sm text-(--color-muted)">
                    <p><strong class="text-(--color-ink)">"30-Day Retention Policy:"</strong>" Deleted hampers, products, and photos are safely stored here for 30 days before being permanently removed. You can restore them to active use at any time within this window."</p>
                </div>
                <div class="flex items-center gap-2">
                    <Show when={move || manage && expired() > 0}>
                        <button type="button" class="btn-danger px-3 py-1.5 text-xs" disabled=move || pending.get() on:click=move |_| on_purge()>{move || format!("Purge Expired ({})", expired())}</button>
                    </Show>
                    <Show when={move || (manage || admin) && count(None) > 0}>
                        <button type="button" class="btn-secondary px-3 py-1.5 text-xs hover:border-red-300 hover:text-red-700" disabled=move || pending.get() on:click=move |_| on_empty()>
                            {move || format!("Empty {}", tab.get().map_or("Bin", Kind::key))}
                        </button>
                    </Show>
                </div>
            </div>
            <div class="flex flex-wrap items-center justify-between gap-3">
                <div class="flex items-center gap-1 rounded-full border border-(--color-line) bg-white p-1 shadow-2xs">
                    {tabs.map(|(k, label)| view! {
                        <button type="button" on:click=move |_| tab.set(k)
                            class=move || format!("rounded-full px-3.5 py-1 text-xs font-medium transition-colors {}", if tab.get() == k { "bg-(--color-brand) text-white" } else { "text-(--color-muted) hover:text-(--color-ink)" })>
                            {move || format!("{label} ({})", count(k))}
                        </button>
                    }).collect_view()}
                </div>
                <div class="min-w-55">
                    <input type="search" class="input text-xs" placeholder="Filter by name or code…" prop:value=search on:input=move |e| search.set(event_target_value(&e)) />
                </div>
            </div>
            {move || match (bin.data.with(Option::is_some), shown.get()) {
                (false, _) => view! { <div class="card h-48 animate-pulse bg-white/60"></div> }.into_any(),
                (true, list) if list.is_empty() => {
                    let title = if !search.get().trim().is_empty() {
                        "No matching items found".to_string()
                    } else {
                        tab.get().map_or_else(|| "The Bin is empty".to_string(), |k| format!("No deleted {}s in the Bin", k.key()))
                    };
                    view! {
                        <div class="card flex flex-col items-center justify-center p-12 text-center">
                            <div class="mb-3 flex h-12 w-12 items-center justify-center rounded-full border border-(--color-line) bg-(--color-paper) text-xl text-(--color-muted)">"🗑️"</div>
                            <h3 class="text-base font-semibold text-(--color-ink)">{title}</h3>
                            <p class="mt-1 max-w-sm text-xs text-(--color-muted)">
                                {if search.get().trim().is_empty() { "When hampers, products, or photos are deleted, they will sit here for 30 days before permanent deletion." } else { "Try adjusting your search query or switching tabs." }}
                            </p>
                        </div>
                    }.into_any()
                }
                (true, list) => view! {
                    <div class="card overflow-x-auto shadow-2xs">
                        <table class="w-full min-w-212.5 border-collapse text-sm">
                            <thead><tr>
                                <th class="th w-14"></th><th class="th w-24">"Type"</th><th class="th w-32">"Code"</th><th class="th">"Item Details"</th>
                                <th class="th">"Deleted On"</th><th class="th">"Retention"</th><th class="th text-right">"Actions"</th>
                            </tr></thead>
                            <tbody>
                                {list.into_iter().map(|item| view! { <Row item=item manage=manage admin=admin pending=pending on_restore=on_restore on_delete=on_delete /> }).collect_view()}
                            </tbody>
                        </table>
                    </div>
                }.into_any(),
            }}
        </div>
    }
}

#[component]
fn Row(
    item: BinItem,
    manage: bool,
    admin: bool,
    pending: RwSignal<bool>,
    on_restore: impl Fn(BinItem) + Copy + 'static,
    on_delete: impl Fn(BinItem) + Copy + 'static,
) -> impl IntoView {
    let days = days_left(item.deleted_ms);
    let badge = match item.kind {
        Kind::Hamper => "bg-[#54655b]/10 text-[#54655b] border-[#54655b]/30",
        Kind::Product => "bg-amber-100/80 text-amber-900 border-amber-300",
        Kind::Image => "bg-blue-100/80 text-blue-900 border-blue-300",
    };
    let (expiry_class, expiry_text) = match days {
        0 => ("border-red-300 bg-red-50 font-semibold text-red-700", "Expired (0d)".to_string()),
        1..=3 => (
            "border-rose-300 bg-rose-50 font-semibold text-rose-700",
            format!("{days} {} left", if days == 1 { "day" } else { "days" }),
        ),
        4..=10 => ("border-amber-300 bg-amber-50 font-medium text-amber-800", format!("{days} days left")),
        _ => ("border-(--color-line) bg-(--color-paper) text-(--color-muted)", format!("{days} days left")),
    };
    let can = may_act(item.kind, manage, admin);
    let (a, b) = (item.clone(), item.clone());
    view! {
        <tr class=move || if pending.get() { "opacity-40 transition-opacity" } else { "transition-opacity" }>
            <td class="td w-14 py-2.5">
                <div class="relative h-12 w-12 shrink-0 overflow-hidden rounded-md border border-(--color-line) bg-(--color-paper)">
                    {match item.image_url.clone() {
                        Some(url) => view! { <FallbackImg src=thumb_url(&url) fallback=url alt="" class="h-full w-full object-cover" /> }.into_any(),
                        None => view! { <div class="flex h-full w-full items-center justify-center text-xs text-(--color-muted)">"No pic"</div> }.into_any(),
                    }}
                </div>
            </td>
            <td class="td w-24 whitespace-nowrap py-2.5">
                <span class=format!("inline-block rounded-full border px-2.5 py-0.5 text-[11px] font-medium tracking-wide uppercase {badge}")>{item.kind.label()}</span>
            </td>
            <td class="td w-32 whitespace-nowrap py-2.5 font-mono text-xs font-medium text-(--color-ink)">{item.code.clone().unwrap_or_else(|| "—".into())}</td>
            <td class="td py-2.5">
                <div class="font-medium text-(--color-ink)">{item.title.clone()}</div>
                <div class="text-xs text-(--color-muted)">{item.subtitle.clone()}</div>
            </td>
            <td class="td whitespace-nowrap py-2.5 text-xs text-(--color-muted)">{short_date(item.deleted_ms)}</td>
            <td class="td whitespace-nowrap py-2.5">
                <span class=format!("inline-flex items-center gap-1 rounded-full border px-2.5 py-0.5 text-xs {expiry_class}")>{expiry_text}</span>
            </td>
            <td class="td whitespace-nowrap py-2.5 text-right">
                {if can {
                    view! {
                        <div class="inline-flex items-center justify-end gap-2">
                            <button type="button" title="Restore to active use" class="btn-secondary px-3 py-1 text-xs font-medium hover:border-(--color-brand) hover:text-(--color-brand)"
                                disabled=move || pending.get() on:click=move |_| on_restore(a.clone())>"Restore"</button>
                            <button type="button" title="Permanently remove" class="btn-danger px-3 py-1 text-xs font-medium"
                                disabled=move || pending.get() on:click=move |_| on_delete(b.clone())>"Delete"</button>
                        </div>
                    }.into_any()
                } else {
                    view! { <span class="text-xs text-(--color-muted)">"Read-only"</span> }.into_any()
                }}
            </td>
        </tr>
    }
}
