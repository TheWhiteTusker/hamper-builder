//! The Supabase URL and anon key (both public) from the app's `supabase.env`,
//! so the Worker and the app can never point at different projects.

fn main() {
    let file = std::fs::read_to_string("../app/src-tauri/supabase.env").unwrap_or_default();
    for key in ["LATTICE_SUPABASE_URL", "LATTICE_SUPABASE_ANON_KEY"] {
        let value = std::env::var(key).ok().or_else(|| {
            file.lines().filter_map(|l| l.split_once('=')).find(|(k, _)| k.trim() == key).map(|(_, v)| v.trim().to_string())
        });
        if let Some(v) = value {
            println!("cargo:rustc-env={key}={v}");
        }
        println!("cargo:rerun-if-env-changed={key}");
    }
    println!("cargo:rerun-if-changed=../app/src-tauri/supabase.env");
}
