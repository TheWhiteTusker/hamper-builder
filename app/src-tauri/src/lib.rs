//! Lattice Lane, the native side: Supabase, the session, the on-device cache
//! and (later phases) exports and images. The UI is `app/ui`, compiled to WASM.

mod cache;
mod commands;
mod db;
mod files;
mod pptx;
mod session;
mod supabase;
mod worker;

use commands::AppState;
use tauri::{AppHandle, Manager};
use tauri_plugin_deep_link::DeepLinkExt;

/// Logs to a daily file in the app's log folder, so field problems can be diagnosed.
fn start_logging(app: &AppHandle) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let dir = app.path().app_log_dir().ok()?;
    std::fs::create_dir_all(&dir).ok()?;
    let (writer, guard) = tracing_appender::non_blocking(tracing_appender::rolling::daily(dir, "lattice.log"));
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,lattice_app=debug"));
    tracing_subscriber::fmt().with_writer(writer).with_ansi(false).with_env_filter(filter).try_init().ok()?;
    std::panic::set_hook(Box::new(|info| tracing::error!("panic: {info}")));
    Some(guard)
}

fn open_urls(app: &AppHandle, urls: Vec<url::Url>) {
    for url in urls {
        let is_callback = url.scheme() == "latticelane"
            && format!("{}{}", url.host_str().unwrap_or_default(), url.path()).trim_end_matches('/') == "auth/callback";
        if !is_callback {
            continue;
        }
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let state = app.state::<AppState>();
            commands::handle_auth_callback(&app, &state, &url).await;
        });
    }
}

pub fn run() {
    let mut builder = tauri::Builder::default();

    // One running copy: a second launch (or a deep link) focuses the first.
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }));
    }

    let result = builder
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let guard = start_logging(&handle);
            app.manage(guard);

            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let api = supabase::Supabase::new().map_err(|e| e.to_string())?;
            app.manage(AppState {
                api,
                sessions: session::Sessions::load(session::SecretStore::new(data_dir.clone())),
                cache: cache::Cache::open(&data_dir.join("cache.sqlite")),
                recovering: false.into(),
            });

            // Installed builds register the scheme in the installer; dev runs register it here.
            #[cfg(all(desktop, debug_assertions))]
            if let Err(e) = app.deep_link().register_all() {
                tracing::warn!("could not register latticelane:// for this dev run: {e}");
            }
            let links = handle.clone();
            app.deep_link().on_open_url(move |event| open_urls(&links, event.urls()));
            if let Ok(Some(urls)) = app.deep_link().get_current() {
                open_urls(&handle, urls);
            }
            // Dev runs only: `LATTICE_OPEN=/products cargo tauri dev` opens on that screen.
            #[cfg(debug_assertions)]
            if let (Ok(path), Some(w), Some(dev)) =
                (std::env::var("LATTICE_OPEN"), app.get_webview_window("main"), app.config().build.dev_url.clone())
                && let Ok(url) = dev.join(path.trim_start_matches('/'))
            {
                let _ = w.navigate(url);
                if let Some(zoom) = std::env::var("LATTICE_ZOOM").ok().and_then(|z| z.parse().ok()) {
                    let _ = w.set_zoom(zoom);
                }
            }
            tracing::info!("Lattice Lane {} started", env!("CARGO_PKG_VERSION"));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::session_state,
            commands::sign_in,
            commands::sign_up,
            commands::sign_out,
            commands::request_password_reset,
            commands::update_password,
            commands::query,
            db::db_select,
            db::db_count,
            db::db_insert,
            db::db_update,
            db::db_delete,
            db::db_rpc,
            db::cache_get,
            db::cache_put,
            db::cache_drop,
            db::storage_upload,
            db::storage_remove,
            db::storage_list,
            db::storage_public_url,
            worker::gstin_lookup,
            worker::print_page,
            files::save_xlsx,
            files::save_text,
            files::read_xlsx,
            files::save_bytes,
            pptx::save_pptx,
        ])
        .run(tauri::generate_context!());

    if let Err(e) = result {
        tracing::error!("the app stopped: {e}");
        eprintln!("Lattice Lane could not start: {e}");
        std::process::exit(1);
    }
}
