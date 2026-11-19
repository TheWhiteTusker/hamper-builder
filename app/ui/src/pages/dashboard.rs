//! Dashboard: active products, recent hampers and quotes, open value,
//! follow-ups due (port of `(app)/dashboard/page.tsx`).

use crate::components::{EmptyState, PageHeader, Stat};
use crate::state::{age, use_query};
use lattice_core::api::{Dashboard as Data, Query};
use lattice_core::js;
use lattice_core::money::format_money;
use lattice_core::records::{HamperSummary, Profile, QuoteSummary, UserRole};
use leptos::prelude::*;
use leptos_router::components::A;

const CLOSED: [&str; 3] = ["Won", "Lost", "Expired"];

fn is_open(q: &QuoteSummary) -> bool {
    !CLOSED.contains(&q.quote.status.as_str())
}

/// Sum of grand total (else subtotal) over open recent quotes, as the old app added it.
fn open_value(quotes: &[QuoteSummary]) -> String {
    let total =
        quotes.iter().filter(|q| is_open(q)).fold(0.0, |sum, q| sum + js::f(q.grand_total.unwrap_or(q.subtotal)));
    format_money(Some(js::dec(total)))
}

#[component]
pub fn Dashboard(profile: Profile) -> impl IntoView {
    let q = use_query::<Data>(Query::Dashboard);
    let first = profile.full_name.as_deref().and_then(|n| n.split(' ').next()).filter(|n| !n.is_empty());
    let title = first.map_or_else(|| "Welcome back".to_string(), |n| format!("Welcome back, {n}"));
    let role = profile.role;

    let data = move || q.data.get().map(|c| c.value);
    let status = move || match (q.data.get(), q.refreshing.get()) {
        (_, true) => "Refreshing…".to_string(),
        (Some(c), false) => format!("Updated {}", age(c.fetched_at)),
        (None, false) => String::new(),
    };

    view! {
        <PageHeader title=title subtitle="Hamper costing and client quotations">
            <span class="text-xs text-(--color-muted)">{status}</span>
            {role.can_manage().then(|| view! { <A href="/hampers/new" attr:class="btn-secondary">"New hamper"</A> })}
            <A href="/quotes/new" attr:class="btn-primary">"New quote"</A>
        </PageHeader>
        {move || q.error.get().filter(|_| q.data.get().is_none()).map(|e| view! {
            <div role="alert" class="card mb-4 border-red-200 bg-red-50 p-3 text-sm text-red-800">{e.to_string()}</div>
        })}
        {move || match data() {
            None => view! { <DashboardSkeleton /> }.into_any(),
            Some(d) if d.hampers.is_empty() && d.quotes.is_empty() => empty(role).into_any(),
            Some(d) => view! { <Filled data=d /> }.into_any(),
        }}
    }
}

fn empty(role: UserRole) -> impl IntoView {
    let admin = role.is_admin();
    let hint = if admin {
        "Import your spreadsheet to bring across products, saved hampers and past quotations."
    } else {
        "Once an admin imports the product list you can start building quotes."
    };
    view! {
        <EmptyState
            title="Nothing here yet"
            hint=hint
            action=admin.then_some(("/admin/import", "Import old workbook"))
        />
    }
}

#[component]
fn DashboardSkeleton() -> impl IntoView {
    view! {
        <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
            {(0..4).map(|_| view! { <div class="card h-[88px] animate-pulse bg-white/60"></div> }).collect_view()}
        </div>
    }
}

#[component]
fn Filled(data: Data) -> impl IntoView {
    let open = data.quotes.iter().filter(|q| is_open(q)).count();
    let value = open_value(&data.quotes);
    let follow_ups = data.follow_ups.clone();
    view! {
        <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
            <Stat label="Active products" value=data.active_products.to_string() />
            <Stat label="Saved hampers" value=data.hampers.len().to_string() hint="most recent" />
            <Stat label="Open quotes" value=open.to_string() />
            <Stat label="Open value" value=value hint="recent quotes, incl. GST" />
        </div>
        {(!follow_ups.is_empty()).then(|| view! {
            <section class="card mt-6 p-4">
                <h2 class="text-sm font-semibold">"Follow-ups due"</h2>
                <ul class="mt-2 divide-y divide-(--color-line)">
                    {follow_ups.into_iter().map(|f| view! {
                        <li class="flex items-center justify-between py-2 text-sm">
                            <A href=format!("/quotes/{}", f.doc_no) attr:class="hover:underline">
                                <span class="font-mono">{f.doc_no.clone()}</span>
                                <span class="ml-2 text-(--color-muted)">{f.client_name}</span>
                            </A>
                            <span class="text-(--color-muted)">{f.follow_up_date.unwrap_or_default()}</span>
                        </li>
                    }).collect_view()}
                </ul>
            </section>
        })}
        <div class="mt-6 grid gap-4 lg:grid-cols-2">
            <RecentHampers rows=data.hampers />
            <RecentQuotes rows=data.quotes />
        </div>
    }
}

#[component]
fn Section(title: &'static str, href: &'static str, children: Children) -> impl IntoView {
    view! {
        <section class="card overflow-hidden">
            <div class="flex items-center justify-between border-b border-line px-4 py-2.5">
                <h2 class="text-sm font-semibold">{title}</h2>
                <A href=href attr:class="text-sm text-(--color-brand) hover:underline">"View all"</A>
            </div>
            {children()}
        </section>
    }
}

#[component]
fn RecentHampers(rows: Vec<HamperSummary>) -> impl IntoView {
    view! {
        <Section title="Recent hampers" href="/hampers">
            {if rows.is_empty() {
                view! { <p class="px-4 py-6 text-sm text-(--color-muted)">"No hampers yet."</p> }.into_any()
            } else {
                view! {
                    <table class="w-full border-collapse text-sm">
                        <thead><tr>
                            <th class="th">"Code"</th><th class="th">"Name"</th>
                            <th class="th num">"Cost"</th><th class="th num">"Price"</th>
                        </tr></thead>
                        <tbody>
                            {rows.into_iter().map(|h| view! {
                                <tr>
                                    <td class="td font-mono">
                                        <A href=format!("/hampers/{}", h.code) attr:class="hover:underline">{h.code.clone()}</A>
                                    </td>
                                    <td class="td">{h.name}</td>
                                    <td class="td num">{format_money(Some(h.total_cp))}</td>
                                    <td class="td num">{format_money(h.final_catalogue_sp)}</td>
                                </tr>
                            }).collect_view()}
                        </tbody>
                    </table>
                }.into_any()
            }}
        </Section>
    }
}

#[component]
fn RecentQuotes(rows: Vec<QuoteSummary>) -> impl IntoView {
    view! {
        <Section title="Recent quotes" href="/quotes">
            {if rows.is_empty() {
                view! { <p class="px-4 py-6 text-sm text-(--color-muted)">"No quotations yet."</p> }.into_any()
            } else {
                view! {
                    <table class="w-full border-collapse text-sm">
                        <thead><tr>
                            <th class="th">"Document"</th><th class="th">"Client"</th>
                            <th class="th">"Status"</th><th class="th num">"Total"</th>
                        </tr></thead>
                        <tbody>
                            {rows.into_iter().map(|q| view! {
                                <tr>
                                    <td class="td font-mono">
                                        <A href=format!("/quotes/{}", q.quote.doc_no) attr:class="hover:underline">{q.quote.doc_no.clone()}</A>
                                    </td>
                                    <td class="td">{q.quote.client_name.clone()}</td>
                                    <td class="td"><span class="badge">{q.quote.status.clone()}</span></td>
                                    <td class="td num">{format_money(Some(q.grand_total.unwrap_or(q.subtotal)))}</td>
                                </tr>
                            }).collect_view()}
                        </tbody>
                    </table>
                }.into_any()
            }}
        </Section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lattice_core::Decimal;
    use lattice_core::records::Quote;

    fn quote(status: &str, subtotal: &str, grand: Option<&str>) -> QuoteSummary {
        let d = |s: &str| s.parse::<Decimal>().unwrap_or_default();
        QuoteSummary {
            quote: Quote { status: status.into(), ..Default::default() },
            subtotal: d(subtotal),
            grand_total: grand.map(d),
            ..Default::default()
        }
    }

    #[test]
    fn open_value_skips_closed_and_falls_back_to_subtotal() {
        let qs = [quote("Sent", "100", Some("118")), quote("Draft", "50", None), quote("Won", "999", Some("999"))];
        assert_eq!(open_value(&qs), "₹168.00");
    }
}
