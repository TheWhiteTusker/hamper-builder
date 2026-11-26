//! Compiles the Supabase URL and anon key into the app from `supabase.env`.
//!
//! Both values are public by design (row level security protects the data),
//! so they are committed, as the Next.js app committed `.env.production`. A
//! real environment variable of the same name wins, for other projects.

fn main() {
    let file = std::fs::read_to_string("supabase.env").unwrap_or_default();
    for key in ["LATTICE_SUPABASE_URL", "LATTICE_SUPABASE_ANON_KEY"] {
        let from_file = file
            .lines()
            .filter_map(|l| l.split_once('='))
            .find(|(k, _)| k.trim() == key)
            .map(|(_, v)| v.trim().to_string());
        match std::env::var(key).ok().or(from_file) {
            Some(value) => println!("cargo:rustc-env={key}={value}"),
            None => println!("cargo:warning={key} is not set: the app cannot reach Supabase"),
        }
        println!("cargo:rerun-if-env-changed={key}");
    }
    // Optional: without it, GST lookup says it isn't configured.
    let worker = std::env::var("LATTICE_WORKER_URL").ok().or_else(|| {
        file.lines()
            .filter_map(|l| l.split_once('='))
            .find(|(k, _)| k.trim() == "LATTICE_WORKER_URL")
            .map(|(_, v)| v.trim().to_string())
    });
    if let Some(url) = worker {
        println!("cargo:rustc-env=LATTICE_WORKER_URL={url}");
    }
    println!("cargo:rerun-if-env-changed=LATTICE_WORKER_URL");
    println!("cargo:rerun-if-changed=supabase.env");
    tauri_build::build();
}
