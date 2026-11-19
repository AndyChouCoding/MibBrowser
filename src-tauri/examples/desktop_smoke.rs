//! Run twice with the same MIB_SMOKE_ID to check import and restart in an isolated workspace.
use tauri::Listener;

fn main() {
    let id = std::env::var("MIB_SMOKE_ID").expect("set MIB_SMOKE_ID to a new UUID");
    let id = uuid::Uuid::parse_str(&id).expect("invalid smoke test UUID");
    let phase = std::env::var("MIB_SMOKE_PHASE").expect("set MIB_SMOKE_PHASE to import or restart");
    assert!(matches!(phase.as_str(), "import" | "restart"));
    let fixture = serde_json::json!({
        "community": std::env::var("MIB_TEST_COMMUNITY").expect("run the isolated fixture"),
        "port": std::env::var("MIB_TEST_PORT").unwrap().parse::<u16>().unwrap()
    })
    .to_string();
    let mut context = tauri::generate_context!();
    context.config_mut().identifier = format!("com.mibbrowser.qualification.{id}");
    context.config_mut().app.windows[0].title = format!("MIB Browser qualification {id}");
    #[cfg(target_os = "macos")]
    {
        context.config_mut().app.windows[0].data_store_identifier = Some(*id.as_bytes());
    }
    mib_browser_lib::desktop_builder()
        .setup(|app| {
            app.listen("qualification-step", |event| {
                println!("Desktop step {}", event.payload());
            });
            let handle = app.handle().clone();
            app.listen("qualification-result", move |event| {
                let result: String = serde_json::from_str(event.payload()).unwrap_or_default();
                println!("Desktop qualification: {result}");
                handle.exit(i32::from(result != "passed"));
            });
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(100));
                eprintln!("Desktop qualification timed out");
                handle.exit(1);
            });
            Ok(())
        })
        .on_page_load(move |window, event| {
            if event.event() == tauri::webview::PageLoadEvent::Finished {
                println!("Desktop page loaded");
                let script = include_str!("desktop_smoke.js")
                    .replace("__PHASE__", &phase)
                    .replace("__FIXTURE__", &fixture);
                window.eval(script).expect("evaluate UI qualification");
            }
        })
        .run(context)
        .expect("launch qualification desktop");
}
