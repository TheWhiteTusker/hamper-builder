//! "Update the app" (port of `release-actions.ts`): an admin starts
//! .github/workflows/desktop.yml, which builds the Windows app from main and
//! publishes it; installed apps then offer the update on their next launch.
//! Needs the secret GITHUB_TOKEN: a fine-grained token for the repo with
//! Actions: Read and write.

use crate::{error, is_admin};
use serde_json::{Value, json};
use worker::{Fetch, Headers, Method, Request, RequestInit, Response, Result, RouteContext};

const REPO: &str = "https://api.github.com/repos/TheWhiteTusker/hamper-builder";

async fn github(method: Method, path: &str, token: &str, body: Option<Value>) -> Result<(u16, Value)> {
    let headers = Headers::new();
    headers.set("authorization", &format!("Bearer {token}"))?;
    headers.set("accept", "application/vnd.github+json")?;
    headers.set("user-agent", "lattice-lane")?;
    let mut init = RequestInit::new();
    init.with_method(method).with_headers(headers);
    if let Some(b) = body {
        init.with_body(Some(b.to_string().into()));
    }
    let mut resp = Fetch::Request(Request::new_with_init(&format!("{REPO}{path}"), &init)?).send().await?;
    Ok((resp.status_code(), resp.json().await.unwrap_or(Value::Null)))
}

/// The admin check and the token, or the response saying why not.
async fn guard(req: &Request, ctx: &RouteContext<()>) -> Result<std::result::Result<String, Response>> {
    if !is_admin(req).await? {
        return Ok(Err(error(403, "Only an admin can publish the app.")?));
    }
    match ctx.env.secret("GITHUB_TOKEN") {
        Ok(t) => Ok(Ok(t.to_string())),
        Err(_) => Ok(Err(error(503, "GITHUB_TOKEN is not set on the Worker.")?)),
    }
}

fn runs(v: &Value) -> Vec<Value> {
    v.get("workflow_runs").and_then(Value::as_array).cloned().unwrap_or_default()
}

fn text<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or_default()
}

pub async fn start(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let token = match guard(&req, &ctx).await? {
        Ok(t) => t,
        Err(resp) => return Ok(resp),
    };
    // Skip the build when main hasn't moved since the last successful release.
    let (head, last) = (
        github(Method::Get, "/commits/main", &token, None).await?,
        github(Method::Get, "/actions/workflows/desktop.yml/runs?status=success&per_page=1", &token, None).await?,
    );
    if head.0 == 200 && last.0 == 200 && runs(&last.1).first().is_some_and(|r| text(r, "head_sha") == text(&head.1, "sha")) {
        return error(409, "Nothing new to package: the app already has the latest changes.");
    }
    let (status, body) =
        github(Method::Post, "/actions/workflows/desktop.yml/dispatches", &token, Some(json!({ "ref": "main" }))).await?;
    if !(200..300).contains(&status) {
        return error(502, &format!("GitHub refused ({status}): {body}"));
    }
    Response::from_json(&json!({ "ok": true }))
}

/// Progress of the latest release. GitHub gives no percentage, so it is
/// estimated from elapsed time against the last successful run's duration,
/// and held at 99% until GitHub reports the run finished.
pub async fn status(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let idle = || Response::from_json(&json!({ "state": "idle", "percent": 0 }));
    let token = match guard(&req, &ctx).await? {
        Ok(t) => t,
        Err(resp) => return Ok(resp),
    };
    let (code, body) = github(Method::Get, "/actions/workflows/desktop.yml/runs?per_page=10", &token, None).await?;
    let list = runs(&body);
    let Some(run) = list.first().filter(|_| code == 200) else { return idle() };
    let (url, created_at) = (text(run, "html_url"), text(run, "created_at"));
    if text(run, "status") == "completed" {
        let ok = text(run, "conclusion") == "success";
        return Response::from_json(&json!({
            "state": if ok { "success" } else { "failure" },
            "percent": if ok { 100 } else { 0 },
            "url": url, "created_at": created_at,
        }));
    }
    let ms = |v: &Value, k: &str| js_sys::Date::parse(text(v, k));
    let expected = list
        .iter()
        .find(|r| text(r, "status") == "completed" && text(r, "conclusion") == "success")
        .map_or(10.0 * 60_000.0, |r| ms(r, "updated_at") - ms(r, "run_started_at"));
    let elapsed = js_sys::Date::now() - ms(run, "run_started_at");
    let percent = if text(run, "status") == "queued" { 0.0 } else { (elapsed / expected * 100.0).round().clamp(1.0, 99.0) };
    let id = run.get("id").and_then(Value::as_i64).unwrap_or_default();
    let (_, jobs) = github(Method::Get, &format!("/actions/runs/{id}/jobs"), &token, None).await?;
    let step = jobs
        .pointer("/jobs/0/steps")
        .and_then(Value::as_array)
        .and_then(|steps| steps.iter().find(|s| text(s, "status") == "in_progress"))
        .map(|s| text(s, "name").to_string());
    Response::from_json(&json!({
        "state": "running", "percent": percent, "step": step, "url": url, "created_at": created_at,
    }))
}
