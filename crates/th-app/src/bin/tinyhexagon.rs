//! The `tinyhexagon` binary (plan 002 issue 07 — the tiny runtime; the
//! issue-03 stub is replaced in place, same bin name / path): the
//! desktop windowed kid game — the `_RELEASE` root, the kid's in-memory
//! config, the real `th_audio::Audio` (gentle SFX), the `th-synth`
//! soundtrack, and the 1024×768 window driving
//! [`th_app::tiny::TinyRuntime::frame`].
//!
//! The composition (07's "Composition"): no profiles, no replay, no
//! Steam/Discord, no menu — `Config::default()` + `load_and_reapply`
//! (the feed-forward's `DesktopModeSource::Lazy`) + the kid overrides
//! (`th_app::tiny::kid_config`); the config is never written back
//! (`config.json` / `Profiles/` / `Replays/` stay untouched — every run
//! starts stage 0).
//!
//! Dev flag: `--tiny-shot=SECONDS` arms the frame's `mustTakeScreenshot`
//! after `SECONDS` (the bridge's `HGGraphics.cpp:305-313` save —
//! `screenshot.png` in the CWD) and closes the window once the bridge
//! has consumed it — the plan's screenshot evidence.

fn main() -> std::process::ExitCode {
    let argv: Vec<String> = std::env::args().collect();

    // The desktop-mode probe child (the winit single-`EventLoop` fix —
    // `th_app::desktop_mode`'s module docs): the ONLY
    // `VideoMode::get_desktop_mode` caller, before any FS / config /
    // window work (exit with the returned code when it returns `Some`).
    if let Some(code) = th_app::desktop_mode::run_probe_child_if_requested(&argv) {
        return std::process::ExitCode::from((code & 0xff) as u8);
    }

    // The dev screenshot half (the plan's evidence): `--tiny-shot=SECONDS`.
    let shot_after = argv.iter().find_map(|a| {
        a.strip_prefix("--tiny-shot=")
            .and_then(|s| s.parse::<f32>().ok())
    });

    let cwd = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("[::main] current_dir failed: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_default();
    let root = th_app::lifecycle::resolve_game_root(&cwd, &exe_dir);

    // The assets (the C++ `HGAssets{nullptr, headless}` — the kid's
    // desktop run is windowed, so headless = false; the `levels_only`
    // half stays false — the styles load with their levels).
    let assets = match th_assets::Assets::new(&root, false, false) {
        Ok(a) => std::sync::Arc::new(a),
        Err(e) => {
            eprintln!("[::main] Asset loading failed: {}", e.reason);
            return std::process::ExitCode::FAILURE;
        }
    };

    // The kid's config: `Config::default()` → `load_and_reapply` (the C++
    // `mainClient`'s load + reapply ONCE — the `Lazy` probe gate; 02c1
    // retired the config-save scope guard, so this never writes) → the
    // kid overrides (they win over the file's values).
    let mut config = th_config::Config::default();
    if let Err(e) = th_app::lifecycle::load_and_reapply(
        &mut config,
        &root,
        &argv,
        th_app::lifecycle::DesktopModeSource::Lazy,
    ) {
        eprintln!("[::mainClient] config load failed: {e}");
        return std::process::ExitCode::FAILURE;
    }
    let config = th_app::tiny::kid_config(config);

    // The SFX (07's composition's `Some(Arc<th_audio::Audio>)`) at a
    // gentle volume (the plan's "set a gentle sound volume" — the SFML
    // 0..=100 units; the synth's master gain stays independent).
    let audio = std::sync::Arc::new(th_audio::Audio::new(assets.clone()));
    audio.set_sound_volume(60.0);
    // The soundtrack (05a: the live `rust-synth` engine; the fallback
    // clock when no device — the `BeatView` keeps advancing either way).
    let music = th_synth::MusicDirector::start();

    th_app::tiny::run_windowed(assets, root, config, audio, music, shot_after);
    std::process::ExitCode::SUCCESS
}
