//! Live updates: one Supabase Realtime channel hands every database change
//! (tables in the supabase_realtime publication, migration 0027) to the app,
//! which refreshes what's on screen when someone else made it. The Phoenix
//! messages and that decision live here, pure, so they're tested without a
//! socket. Port of `live-refresh.ts`.

use serde_json::{Value, json};

pub const TOPIC: &str = "realtime:live-refresh";

/// One row change, as Realtime's postgres_changes sends it.
#[derive(Clone, Debug, PartialEq)]
pub struct LiveChange {
    pub table: String,
    /// INSERT, UPDATE or DELETE.
    pub kind: String,
    pub record: Value,
    pub old_record: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Incoming {
    Change(LiveChange),
    /// The server accepted the channel.
    Joined,
    /// The channel was refused or closed: reconnect.
    Failed(String),
    Other,
}

pub fn join(token: &str, msg_ref: u64) -> String {
    json!({
        "topic": TOPIC,
        "event": "phx_join",
        "payload": {
            "config": {
                "broadcast": { "ack": false, "self": false },
                "presence": { "key": "" },
                "postgres_changes": [{ "event": "*", "schema": "public" }],
                "private": false,
            },
            "access_token": token,
        },
        "ref": msg_ref.to_string(),
        "join_ref": msg_ref.to_string(),
    })
    .to_string()
}

/// Sent every 25 s, or the server drops the socket.
pub fn heartbeat(msg_ref: u64) -> String {
    json!({ "topic": "phoenix", "event": "heartbeat", "payload": {}, "ref": msg_ref.to_string() }).to_string()
}

/// A refreshed access token, so the channel outlives the old one's hour.
pub fn access_token(token: &str, msg_ref: u64) -> String {
    json!({ "topic": TOPIC, "event": "access_token", "payload": { "access_token": token }, "ref": msg_ref.to_string() })
        .to_string()
}

/// `v.key`, or null.
fn at<'a>(v: &'a Value, key: &str) -> &'a Value {
    v.get(key).unwrap_or(&Value::Null)
}

fn text<'a>(v: &'a Value, key: &str) -> &'a str {
    at(v, key).as_str().unwrap_or_default()
}

pub fn parse(raw: &str) -> Incoming {
    let Ok(m) = serde_json::from_str::<Value>(raw) else { return Incoming::Other };
    if text(&m, "topic") != TOPIC {
        return Incoming::Other;
    }
    let p = at(&m, "payload");
    let status = text(p, "status");
    match text(&m, "event") {
        "postgres_changes" => {
            let d = at(p, "data");
            Incoming::Change(LiveChange {
                table: text(d, "table").to_string(),
                kind: text(d, "type").to_string(),
                record: at(d, "record").clone(),
                old_record: at(d, "old_record").clone(),
            })
        }
        "phx_reply" if status == "ok" && at(p, "response").get("postgres_changes").is_some() => Incoming::Joined,
        "phx_reply" if status == "error" => Incoming::Failed(at(p, "response").to_string()),
        "system" if status == "error" => Incoming::Failed(text(p, "message").to_string()),
        e @ ("phx_error" | "phx_close") => Incoming::Failed(e.to_string()),
        _ => Incoming::Other,
    }
}

/// Replaced on every save of their parent, whose own row changes in the same save.
const CHILD_TABLES: [&str; 3] = ["hamper_items", "quote_items", "presentation_slides"];

/// True when someone else made the change, so the screen needs a refresh.
/// A delete carries only the row's id, so it can't say who: child-table
/// deletes are skipped (the parent's update says it), the rest refresh.
/// Your own saves are already on your screen.
pub fn is_others_change(c: &LiveChange, user_id: &str) -> bool {
    if c.kind == "DELETE" {
        return !CHILD_TABLES.contains(&c.table.as_str());
    }
    at(&c.record, "updated_by").as_str() != Some(user_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn change(table: &str, kind: &str, row: Value) -> LiveChange {
        let (record, old_record) = if kind == "DELETE" { (json!({}), row) } else { (row, json!({})) };
        LiveChange { table: table.into(), kind: kind.into(), record, old_record }
    }

    #[test]
    fn skips_your_own_writes_and_refreshes_for_everyone_elses() {
        let me = "me";
        assert!(is_others_change(&change("products", "UPDATE", json!({ "id": "1", "updated_by": "other" })), me));
        assert!(!is_others_change(&change("products", "UPDATE", json!({ "id": "1", "updated_by": me })), me));
        assert!(!is_others_change(&change("clients", "INSERT", json!({ "id": "1", "updated_by": me })), me));
        assert!(is_others_change(&change("clients", "INSERT", json!({ "id": "1", "updated_by": "other" })), me));
        // Cron and service-role writes have no user.
        assert!(is_others_change(&change("products", "UPDATE", json!({ "id": "1", "updated_by": null })), me));
    }

    #[test]
    fn child_table_deletes_ride_on_the_parents_update_other_deletes_refresh() {
        for t in CHILD_TABLES {
            assert!(!is_others_change(&change(t, "DELETE", json!({ "id": "1" })), "me"));
        }
        for t in ["products", "clients"] {
            assert!(is_others_change(&change(t, "DELETE", json!({ "id": "1" })), "me"));
        }
    }

    #[test]
    fn reads_changes_replies_and_failures() {
        let msg = json!({
            "topic": TOPIC, "event": "postgres_changes", "ref": null,
            "payload": { "ids": [1], "data": {
                "schema": "public", "table": "products", "type": "UPDATE",
                "record": { "id": "p1", "updated_by": "u2" }, "old_record": { "id": "p1" },
            }},
        });
        assert!(matches!(
            parse(&msg.to_string()),
            Incoming::Change(c) if c.table == "products" && c.kind == "UPDATE" && c.record["id"] == "p1"
        ));

        let joined = json!({ "topic": TOPIC, "event": "phx_reply", "ref": "1",
            "payload": { "status": "ok", "response": { "postgres_changes": [{ "id": 1 }] } } });
        assert_eq!(parse(&joined.to_string()), Incoming::Joined);
        let heartbeat_ok =
            json!({ "topic": "phoenix", "event": "phx_reply", "payload": { "status": "ok", "response": {} } });
        assert_eq!(parse(&heartbeat_ok.to_string()), Incoming::Other);
        assert!(matches!(
            parse(&json!({ "topic": TOPIC, "event": "phx_close", "payload": {} }).to_string()),
            Incoming::Failed(_)
        ));
        assert_eq!(parse("not json"), Incoming::Other);
    }

    #[test]
    fn join_asks_for_every_public_change_with_the_token() {
        let m: Value = serde_json::from_str(&join("jwt", 1)).unwrap_or_default();
        assert_eq!(m["payload"]["access_token"], "jwt");
        assert_eq!(m["payload"]["config"]["postgres_changes"][0]["schema"], "public");
        assert_eq!(m["topic"], TOPIC);
    }
}
