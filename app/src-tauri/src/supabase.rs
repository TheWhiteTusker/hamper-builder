//! Supabase over HTTPS: Auth (GoTrue) and the database (PostgREST), using the
//! signed-in person's JWT, so row level security decides every read and write
//! exactly as it does for the web app. The device holds the anon key only.

use lattice_core::api::{ApiError, AuthUser};
use reqwest::{RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::time::Duration;

/// A signed-in session. The access token lives in memory only; the refresh
/// token is what survives a restart (in the OS keychain).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub access_token: String,
    pub refresh_token: String,
    /// Unix seconds; 0 means "refresh before use".
    pub expires_at: i64,
    pub user: AuthUser,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: Option<i64>,
    expires_at: Option<i64>,
    user: GoUser,
}

#[derive(Deserialize)]
struct GoUser {
    id: String,
    email: Option<String>,
}

impl TokenResponse {
    fn into_session(self) -> Session {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        Session {
            access_token: self.access_token,
            refresh_token: self.refresh_token,
            expires_at: self.expires_at.unwrap_or_else(|| now + self.expires_in.unwrap_or(3600)),
            user: AuthUser { id: self.user.id, email: self.user.email },
        }
    }
}

/// Where and how an upload is stored.
pub struct Upload<'a> {
    pub bucket: &'a str,
    pub path: &'a str,
    pub content_type: &'a str,
    /// Seconds, as `Cache-Control: max-age`.
    pub cache_control: &'a str,
    pub upsert: bool,
}

pub struct Supabase {
    http: reqwest::Client,
    url: String,
    anon: String,
}

const OFFLINE: &str = "Can't reach the server. Check your internet connection; anything shown is the last saved copy.";

/// A transport failure: no network, DNS, TLS or a timeout all read as offline.
pub fn net(e: &reqwest::Error) -> ApiError {
    if e.is_connect() || e.is_timeout() || e.is_request() {
        ApiError::Offline(OFFLINE.into())
    } else {
        ApiError::Server(format!("The server sent something unexpected ({e})."))
    }
}

/// The human part of a GoTrue or PostgREST error body.
fn message_of(body: &Value) -> Option<String> {
    ["msg", "message", "error_description", "error"]
        .iter()
        .find_map(|k| body.get(*k).and_then(Value::as_str))
        .map(str::to_string)
}

async fn failure(resp: Response, auth: bool) -> ApiError {
    let status = resp.status();
    let body: Value = resp.json().await.unwrap_or(Value::Null);
    let code = body.get("code").and_then(Value::as_str).unwrap_or_default().to_string();
    let message = message_of(&body).unwrap_or_else(|| format!("Request failed ({status})."));
    tracing::warn!(%status, code, message, "supabase error");

    // PostgREST: an expired or invalid JWT is a session problem, not a permission one.
    if code.starts_with("PGRST3") {
        return ApiError::SignedOut;
    }
    match status {
        StatusCode::TOO_MANY_REQUESTS => ApiError::Server("Too many attempts. Wait a minute and try again.".into()),
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN if auth => ApiError::Invalid(message),
        StatusCode::UNAUTHORIZED => ApiError::SignedOut,
        StatusCode::FORBIDDEN => ApiError::Forbidden(format!("You don't have permission to do that. ({message})")),
        StatusCode::NOT_FOUND => ApiError::NotFound(message),
        StatusCode::CONFLICT => ApiError::Conflict(message),
        _ if code == "23505" => ApiError::Conflict(message),
        _ if code == "42501" => ApiError::Forbidden(format!("You don't have permission to do that. ({message})")),
        s if s.is_client_error() => ApiError::Invalid(message),
        _ => ApiError::Server(message),
    }
}

impl Supabase {
    pub fn new() -> Result<Self, ApiError> {
        let (Some(url), Some(anon)) = (option_env!("LATTICE_SUPABASE_URL"), option_env!("LATTICE_SUPABASE_ANON_KEY"))
        else {
            return Err(ApiError::Server("This build has no Supabase project configured (supabase.env).".into()));
        };
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(45))
            .user_agent(concat!("lattice-lane/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| ApiError::Server(format!("Could not start the network client: {e}")))?;
        Ok(Self { http, url: url.trim_end_matches('/').to_string(), anon: anon.to_string() })
    }

    /// Realtime's socket address, for the UI's live updates.
    pub fn realtime_url(&self) -> String {
        let host = self.url.replacen("https://", "wss://", 1).replacen("http://", "ws://", 1);
        format!("{host}/realtime/v1/websocket?apikey={}&vsn=1.0.0", self.anon)
    }

    /// The shared network client, for the few calls that aren't Supabase's.
    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }

    async fn send(&self, req: RequestBuilder, auth: bool) -> Result<Response, ApiError> {
        let resp = req.header("apikey", &self.anon).send().await.map_err(|e| net(&e))?;
        if resp.status().is_success() { Ok(resp) } else { Err(failure(resp, auth).await) }
    }

    async fn token_grant(&self, grant: &str, body: Value) -> Result<Session, ApiError> {
        let req = self.http.post(format!("{}/auth/v1/token", self.url)).query(&[("grant_type", grant)]).json(&body);
        let resp = self.send(req, true).await?;
        let token: TokenResponse = resp.json().await.map_err(|e| net(&e))?;
        Ok(token.into_session())
    }

    pub async fn sign_in(&self, email: &str, password: &str) -> Result<Session, ApiError> {
        self.token_grant("password", json!({ "email": email, "password": password })).await
    }

    pub async fn refresh(&self, refresh_token: &str) -> Result<Session, ApiError> {
        match self.token_grant("refresh_token", json!({ "refresh_token": refresh_token })).await {
            // A refused refresh token means the session is over, whatever the wording.
            Err(ApiError::Invalid(_)) => Err(ApiError::SignedOut),
            other => other,
        }
    }

    /// Creates the account. With email confirmation on, there is no session yet.
    /// The first account becomes admin, the rest sales (`handle_new_user` in SQL).
    pub async fn sign_up(&self, email: &str, password: &str, full_name: &str) -> Result<Option<Session>, ApiError> {
        let body = json!({ "email": email, "password": password, "data": { "full_name": full_name } });
        let resp = self.send(self.http.post(format!("{}/auth/v1/signup", self.url)).json(&body), true).await?;
        let value: Value = resp.json().await.map_err(|e| net(&e))?;
        Ok(serde_json::from_value::<TokenResponse>(value).ok().map(TokenResponse::into_session))
    }

    /// Emails a reset link that comes back to `redirect_to` with a PKCE code.
    pub async fn recover(&self, email: &str, challenge: &str, redirect_to: &str) -> Result<(), ApiError> {
        let body = json!({ "email": email, "code_challenge": challenge, "code_challenge_method": "s256" });
        let req =
            self.http.post(format!("{}/auth/v1/recover", self.url)).query(&[("redirect_to", redirect_to)]).json(&body);
        self.send(req, true).await.map(drop)
    }

    pub async fn exchange_code(&self, code: &str, verifier: &str) -> Result<Session, ApiError> {
        self.token_grant("pkce", json!({ "auth_code": code, "code_verifier": verifier })).await
    }

    pub async fn update_password(&self, token: &str, password: &str) -> Result<(), ApiError> {
        let req = self
            .http
            .put(format!("{}/auth/v1/user", self.url))
            .bearer_auth(token)
            .json(&json!({ "password": password }));
        self.send(req, true).await.map(drop)
    }

    pub async fn sign_out(&self, token: &str) -> Result<(), ApiError> {
        self.send(self.http.post(format!("{}/auth/v1/logout", self.url)).bearer_auth(token), true).await.map(drop)
    }

    fn rest(&self, path: &str) -> String {
        format!("{}/rest/v1/{path}", self.url)
    }

    /// `GET /rest/v1/<table>?<query>`, as rows.
    pub async fn select<T: DeserializeOwned>(
        &self,
        token: &str,
        table: &str,
        query: &[(&str, &str)],
    ) -> Result<T, ApiError> {
        let req = self.http.get(self.rest(table)).bearer_auth(token).query(query);
        let resp = self.send(req, false).await?;
        resp.json().await.map_err(|e| ApiError::Server(format!("Unexpected data from {table}: {e}")))
    }

    /// The exact number of rows matching `query`, without fetching them.
    pub async fn count(&self, token: &str, table: &str, query: &[(&str, &str)]) -> Result<i64, ApiError> {
        let req = self.http.head(self.rest(table)).bearer_auth(token).header("Prefer", "count=exact").query(query);
        let resp = self.send(req, false).await?;
        let range = resp.headers().get("content-range").and_then(|v| v.to_str().ok()).unwrap_or_default();
        range
            .rsplit('/')
            .next()
            .and_then(|n| n.parse().ok())
            .ok_or_else(|| ApiError::Server(format!("No row count came back for {table}.")))
    }

    /// Reads a write's response: the rows when `select` asked for them, else null.
    async fn rows(resp: Response) -> Result<Value, ApiError> {
        let text = resp.text().await.map_err(|e| net(&e))?;
        if text.trim().is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&text).map_err(|e| ApiError::Server(format!("Unexpected answer from the database: {e}")))
    }

    /// `POST /rest/v1/<table>`: one row or many. With `on_conflict`, an upsert on those columns.
    pub async fn insert(
        &self,
        token: &str,
        table: &str,
        rows: &Value,
        on_conflict: Option<&str>,
        select: Option<&str>,
    ) -> Result<Value, ApiError> {
        let mut prefer = vec![if select.is_some() { "return=representation" } else { "return=minimal" }];
        let mut query: Vec<(&str, &str)> = Vec::new();
        if let Some(cols) = on_conflict {
            prefer.push("resolution=merge-duplicates");
            query.push(("on_conflict", cols));
        }
        if let Some(s) = select {
            query.push(("select", s));
        }
        let req = self
            .http
            .post(self.rest(table))
            .bearer_auth(token)
            .header("Prefer", prefer.join(","))
            .query(&query)
            .json(rows);
        Self::rows(self.send(req, false).await?).await
    }

    /// `PATCH /rest/v1/<table>?<filter>`.
    pub async fn update(
        &self,
        token: &str,
        table: &str,
        filter: &[(&str, &str)],
        patch: &Value,
        select: Option<&str>,
    ) -> Result<Value, ApiError> {
        let prefer = if select.is_some() { "return=representation" } else { "return=minimal" };
        let mut query = filter.to_vec();
        if let Some(s) = select {
            query.push(("select", s));
        }
        let req =
            self.http.patch(self.rest(table)).bearer_auth(token).header("Prefer", prefer).query(&query).json(patch);
        Self::rows(self.send(req, false).await?).await
    }

    /// `DELETE /rest/v1/<table>?<filter>`, returning the deleted rows.
    pub async fn delete(&self, token: &str, table: &str, filter: &[(&str, &str)]) -> Result<Value, ApiError> {
        let req = self
            .http
            .delete(self.rest(table))
            .bearer_auth(token)
            .header("Prefer", "return=representation")
            .query(filter);
        Self::rows(self.send(req, false).await?).await
    }

    /// `POST /rest/v1/rpc/<name>`: a database function, run in one transaction.
    pub async fn rpc(&self, token: &str, name: &str, args: &Value) -> Result<Value, ApiError> {
        let req = self.http.post(self.rest(&format!("rpc/{name}"))).bearer_auth(token).json(args);
        Self::rows(self.send(req, false).await?).await
    }

    fn storage(&self, path: &str) -> String {
        format!("{}/storage/v1/{path}", self.url)
    }

    /// The public URL of an object in a public bucket.
    pub fn public_url(&self, bucket: &str, path: &str) -> String {
        format!("{}/storage/v1/object/public/{bucket}/{path}", self.url)
    }

    /// Stores `bytes` at `bucket/path` and returns its public URL.
    pub async fn upload(&self, token: &str, u: &Upload<'_>, bytes: Vec<u8>) -> Result<String, ApiError> {
        let req = self
            .http
            .post(self.storage(&format!("object/{}/{}", u.bucket, u.path)))
            .bearer_auth(token)
            .header("Content-Type", u.content_type)
            .header("cache-control", format!("max-age={}", u.cache_control))
            .header("x-upsert", if u.upsert { "true" } else { "false" })
            .body(bytes);
        self.send(req, false).await?;
        Ok(self.public_url(u.bucket, u.path))
    }

    pub async fn remove(&self, token: &str, bucket: &str, paths: &[String]) -> Result<(), ApiError> {
        if paths.is_empty() {
            return Ok(());
        }
        let req = self
            .http
            .delete(self.storage(&format!("object/{bucket}")))
            .bearer_auth(token)
            .json(&json!({ "prefixes": paths }));
        self.send(req, false).await.map(drop)
    }

    /// The objects directly inside `prefix` (one level, like the dashboard's folder view).
    pub async fn list(&self, token: &str, bucket: &str, prefix: &str) -> Result<Value, ApiError> {
        let body =
            json!({ "prefix": prefix, "limit": 1000, "offset": 0, "sortBy": { "column": "name", "order": "asc" } });
        let req = self.http.post(self.storage(&format!("object/list/{bucket}"))).bearer_auth(token).json(&body);
        Self::rows(self.send(req, false).await?).await
    }
}

/// Against the live project, so `#[ignore]`d: `cargo test -p lattice-app -- --ignored`.
#[cfg(test)]
#[allow(clippy::panic)]
mod live {
    use super::*;

    #[tokio::test]
    #[ignore = "needs the network and the real Supabase project"]
    async fn wrong_password_is_an_invalid_input_not_a_crash() {
        let api = Supabase::new().unwrap_or_else(|e| panic!("{e}"));
        let err = api.sign_in("nobody@example.invalid", "definitely-wrong-1").await.err();
        assert!(matches!(err, Some(ApiError::Invalid(_))), "{err:?}");
    }

    #[tokio::test]
    #[ignore = "needs the network and the real Supabase project"]
    async fn an_unknown_refresh_token_signs_out() {
        let api = Supabase::new().unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(api.refresh("not-a-real-token").await.err(), Some(ApiError::SignedOut));
    }

    #[tokio::test]
    #[ignore = "needs the network and the real Supabase project"]
    async fn reading_without_a_session_is_refused_by_rls() {
        let api = Supabase::new().unwrap_or_else(|e| panic!("{e}"));
        // The anon key is a valid JWT for the `anon` role, which RLS gives nothing.
        let anon = option_env!("LATTICE_SUPABASE_ANON_KEY").unwrap_or_default();
        let rows: Result<Vec<Value>, _> = api.select(anon, "products", &[("select", "id"), ("limit", "1")]).await;
        assert!(rows.as_ref().is_ok_and(Vec::is_empty) || rows.is_err(), "anon could read products: {rows:?}");
    }
}
