# Lattice Lane (Rust)

A Rust rebuild of the Lattice Lane hamper builder: costing, quotations, proformas and
presentations. It shares the existing Supabase backend (schema, views, RPCs, RLS, Auth,
Storage) with the Next.js app, so both can run side by side until cutover.

| Path | What it is |
| --- | --- |
| `crates/core` | Pure domain logic: pricing, costing, MRP caps, codes, colours, import and sheet planning, plus the UI ↔ native contract (`api.rs`). No I/O; builds for native and `wasm32`. |
| `app/src-tauri` | The native side (Tauri 2). It handles Supabase Auth and PostgREST with the user's JWT (RLS decides every read and write), keeps the session in the OS keychain, and keeps an SQLite cache so screens open instantly. |
| `app/ui` | The UI: Leptos compiled to WASM and shown in the webview. It runs `crates/core` directly, so live totals never wait on the network. |
| `supabase/migrations` | `0001`–`0025` copied verbatim from the Next.js repo. New migrations are additive only. |
| `scripts/gen-fixtures.mjs` | Runs the old app's TypeScript on generated inputs, for the parity test. |

## Numbers match the old app exactly

Values are `Decimal` at every boundary, but the arithmetic follows JavaScript doubles in the
old app's operation order (`crates/core/src/js.rs`). That keeps every figure identical to what
the TypeScript computed and stored, including its float quirks. Here is one: ₹752 at a 90%
margin prices at ₹7,530, because `752 / 0.1` is `7520.000000000002` in binary.

`crates/core/tests/parity.rs` replays 44,810 TypeScript results and requires bit-for-bit
equality. If the old app's pricing ever changes, regenerate the fixtures:

```bash
node scripts/gen-fixtures.mjs "D:/Github/lattice lane hamper builder"
```

## Development

You need Rust stable with the `wasm32-unknown-unknown` target, then
`cargo install tauri-cli --version "^2" --locked` and `cargo install trunk --locked`.

```bash
cd app/src-tauri && cargo tauri dev     # the desktop app, with the UI on trunk serve
cargo test --workspace
cargo test -p lattice-app -- --ignored  # live checks against the real Supabase project
cargo clippy --workspace --all-targets -- -D warnings
```

`app/src-tauri/supabase.env` holds the project URL and anon key. Both are public by design and
committed, as the Next app committed `.env.production`. For password-reset emails to open the
app, add `latticelane://auth/callback` under Supabase → Authentication → URL Configuration →
Redirect URLs.
