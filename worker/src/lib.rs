//! Lattice Lane's Cloudflare Worker. The app talks to Supabase directly (RLS
//! decides everything); this holds only what needs a secret a device must not
//! carry:
//!
//! - `GET /gstin/{gstin}`: the GST register lookup (API key `GSTIN_API_KEY`),
//!   for signed-in users only.
//! - the nightly cron: removes Recycle Bin items kept past 30 days, files
//!   first, then the rows (`SUPABASE_SERVICE_ROLE_KEY`; skipped without it).

use lattice_core::bin_expiry::RETENTION_DAYS;
use lattice_core::gstin::{is_valid, read_gstinapi};
use lattice_core::images::{thumb_path, view_path};
use serde_json::{Value, json};
use worker::{
    Context, Env, Fetch, Headers, Method, Request, RequestInit, Response, Result, RouteContext, Router, ScheduleContext,
    ScheduledEvent, console_error, console_log, event,
};

const BUCKET: &str = "product-images";

fn supabase() -> Result<(&'static str, &'static str)> {
    match (option_env!("LATTICE_SUPABASE_URL"), option_env!("LATTICE_SUPABASE_ANON_KEY")) {
        (Some(url), Some(anon)) => Ok((url.trim_end_matches('/'), anon)),
        _ => Err("This build has no Supabase project configured (supabase.env).".into()),
    }
}

fn error(status: u16, message: &str) -> Result<Response> {
    Ok(Response::from_json(&json!({ "error": message }))?.with_status(status))
}

async fn send(method: Method, url: &str, key: &str, token: &str, body: Option<&Value>) -> Result<Response> {
    let headers = Headers::new();
    headers.set("apikey", key)?;
    headers.set("authorization", &format!("Bearer {token}"))?;
    let mut init = RequestInit::new();
    init.with_method(method).with_headers(headers);
    if let Some(b) = body {
        init.headers.set("content-type", "application/json")?;
        init.with_body(Some(b.to_string().into()));
    }
    Fetch::Request(Request::new_with_init(url, &init)?).send().await
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    Router::new().get_async("/gstin/:gstin", gstin).run(req, env).await
}

/// A signed-in user's JWT, checked with Supabase Auth itself.
async fn signed_in(req: &Request) -> Result<bool> {
    let Some(token) = req.headers().get("authorization")?.and_then(|h| h.strip_prefix("Bearer ").map(str::to_string)) else {
        return Ok(false);
    };
    let (url, anon) = supabase()?;
    let resp = send(Method::Get, &format!("{url}/auth/v1/user"), anon, &token, None).await?;
    Ok(resp.status_code() == 200)
}

async fn gstin(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    if !signed_in(&req).await? {
        return error(401, "Your session has ended. Please sign in again.");
    }
    let value = ctx.param("gstin").map(|g| g.trim().to_uppercase()).unwrap_or_default();
    if !is_valid(&value) {
        return error(400, "Enter a valid 15-character GSTIN first.");
    }
    let Ok(key) = ctx.env.secret("GSTIN_API_KEY") else {
        return error(503, "GST lookup is not configured. Enter the name and address by hand.");
    };
    let headers = Headers::new();
    headers.set("x-api-key", key.to_string().trim())?;
    let mut init = RequestInit::new();
    init.with_headers(headers);
    // ponytail: one attempt; the API asks for backoff on 429/502, worth adding
    // only if this stops being a button someone presses by hand.
    let lookup = Request::new_with_init(&format!("https://www.gstinapi.in/v1/gstin/{value}"), &init)?;
    let Ok(mut resp) = Fetch::Request(lookup).send().await else {
        return error(502, "Could not reach the GST service. Enter the details by hand.");
    };
    let ok = (200..300).contains(&resp.status_code());
    let body: Value = resp.json().await.unwrap_or(Value::Null);
    match read_gstinapi(ok, &body) {
        Ok(details) => Response::from_json(&details),
        Err(message) => error(502, &message),
    }
}

#[event(scheduled)]
async fn scheduled(_event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    match purge(&env).await {
        Ok(summary) => console_log!("bin purge: {summary}"),
        Err(e) => console_error!("bin purge failed: {e}"),
    }
}

/// Port of the Bin's `purgeExpiredAction`: storage files of expired photos,
/// hampers and products, then `purge_expired_bin_records()` for the rows.
async fn purge(env: &Env) -> Result<String> {
    let Ok(service) = env.secret("SUPABASE_SERVICE_ROLE_KEY") else {
        return Ok("skipped: SUPABASE_SERVICE_ROLE_KEY is not set".into());
    };
    let (service, (url, _)) = (service.to_string(), supabase()?);
    let cutoff_ms = js_sys::Date::now() - (RETENTION_DAYS as f64) * 86_400_000.0;
    let cutoff = String::from(js_sys::Date::new(&cutoff_ms.into()).to_iso_string());
    let expired = |table: &str, cols: &str| format!("{url}/rest/v1/{table}?select={cols}&deleted_at=lte.{cutoff}");
    let rows = |r: Value| r.as_array().cloned().unwrap_or_default();
    let get = async |u: String| -> Result<Vec<Value>> { Ok(rows(send(Method::Get, &u, &service, &service, None).await?.json().await?)) };
    let remove = async |paths: Vec<String>| -> Result<()> {
        if !paths.is_empty() {
            send(Method::Delete, &format!("{url}/storage/v1/object/{BUCKET}"), &service, &service, Some(&json!({ "prefixes": paths }))).await?;
        }
        Ok(())
    };
    let folder = async |prefix: String| -> Result<Vec<String>> {
        let body = json!({ "prefix": prefix, "limit": 1000, "offset": 0 });
        let listed = rows(send(Method::Post, &format!("{url}/storage/v1/object/list/{BUCKET}"), &service, &service, Some(&body)).await?.json().await?);
        Ok(listed.iter().filter_map(|f| f.get("name").and_then(Value::as_str)).map(|n| format!("{prefix}/{n}")).collect())
    };

    let mut files = 0;
    for img in get(expired("product_images", "id,storage_path")).await? {
        if let Some(p) = img.get("storage_path").and_then(Value::as_str).filter(|p| !p.is_empty()) {
            remove(vec![p.to_string(), thumb_path(p), view_path(p)]).await?;
            files += 3;
        }
    }
    for (table, dir) in [("hampers", "hampers"), ("products", "products")] {
        for row in get(expired(table, "id")).await? {
            if let Some(id) = row.get("id").and_then(Value::as_str) {
                let paths = folder(format!("{dir}/{id}")).await?;
                files += paths.len();
                remove(paths).await?;
            }
        }
    }
    let mut done = send(Method::Post, &format!("{url}/rest/v1/rpc/purge_expired_bin_records"), &service, &service, Some(&json!({}))).await?;
    let counts: Value = done.json().await.unwrap_or(Value::Null);
    Ok(format!("{files} files removed; rows {counts}"))
}
