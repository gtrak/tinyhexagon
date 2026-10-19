//! C++ `main.cpp` folder/config lifecycle pieces — the `mainClient`
//! `:275-291` section (folder creation, config load + reapply), ported
//! for the pieces that exist in the step-(b) tree. Issue 15 step (b).
//! (The C++'s two exit scope guards — the config save,
//! main.cpp:285-291, and the `assets.pSaveAll` profile save,
//! main.cpp:382-388 — were retired by plan 002 issue 02c1: the tiny
//! runtime persists no config and no profiles.)

use std::path::Path;

use oh_config::Config;
use oh_render::VideoMode;

/// C++ `createFolderIfNonExistant(folderName)` (main.cpp:65-79) with the
/// CWD made explicit as `base_dir` (the binary passes the process CWD; the
/// C++ `ssvu::FileSystem::Path{folderName}` is CWD-relative).
///
/// Ported log output on creation — the C++
/// `ssvu::lo("::createFolderIfNonExistant")` channel, printed
/// channel-prefixed like the rest of the port:
/// `'Profiles/' folder does not exist, creating`. An existing folder is
/// silent (the C++ returns before logging). The C++ `createFolder`
/// (`std::filesystem::create_directories`) throws on failure (uncaught
/// here → crash); the port reports to stderr and continues (documented
/// deviation — a failed folder never matters for the headless path).
pub fn create_folder_if_non_existant(base_dir: &Path, folder_name: &str) {
    let path = base_dir.join(folder_name);
    if path.is_dir() {
        return;
    }

    println!("[::createFolderIfNonExistant] '{folder_name}' folder does not exist, creating");

    if let Err(e) = std::fs::create_dir_all(&path) {
        eprintln!("[::createFolderIfNonExistant] failed to create '{folder_name}': {e}");
    }
}

/// C++ `main.cpp:279-283`:///
/// ```cpp
/// hg::Config::loadConfig(args);
/// hg::Config::reapplyResolution();
/// ```
///
/// `override_ids` are the residual CLI args VERBATIM (the C++ passes its
/// `args` vector — which includes `argv[0]`; a token only becomes an
/// override when `ConfigOverrides/<token>.json` exists, so the executable
/// path is harmless). `Err` is a `config.json` / linked-value sync failure
/// the C++ would throw out of `syncAllFromObj` (uncaught in `mainClient`
/// → crash); the port surfaces it so the caller exits non-zero (documented
/// deviation).
///
/// (The C++ `::loadConfig` / `::reapplyResolution` channel lines
/// (`Config.cpp:494` / `:523`) are printed by the data layer's own
/// `load_config` / `reapply_resolution` — `oh-config` carries the ported
/// console output, so `load_and_reapply` must not re-print them.)
///
/// `desktop_mode` is the C++ `sf::VideoMode::getDesktopMode()` read the
/// `applyAuto*Resolution` branches use (Config.cpp:480, :487). The C++
/// bootstrap performs this display query BEFORE the window exists (it is
/// a display query, not a window query — `loadConfig` +
/// `reapplyResolution` precede `window.emplace()`, main.cpp:279-303), so
/// the ported `main_client` passes `oh_render::VideoMode::get_desktop_mode()`
/// (the winit primary-monitor query over a throwaway event loop; the zero
/// mode on a headless host — the C++'s no-monitor `VideoMode{0, 0}`
/// fallback) for BOTH the windowed and the headless branch (the C++
/// `mainClient` does the load + reapply once, before the
/// `if(!headless)` window branch). The Android bootstrap passes the zero
/// mode (winit's monitor API exists only inside a RUNNING event loop and
/// the activity's loop is consumed by the game's own `run` — inert in
/// practice: both auto-resolution flags default false and the platform
/// window is screen-sized independently of the config values; `oh-app`
/// `android::run`'s load-call comment) and the C++ `mainServer` path
/// doesn't reapply at all (main.cpp:199-203 has no `reapplyResolution` —
/// main.cpp:283 is the C++'s only call site).
pub fn load_and_reapply(
    config: &mut Config,
    base_dir: &Path,
    override_ids: &[String],
    desktop_mode: VideoMode,
) -> Result<(), oh_data::json::JsonError> {
    config.load_config(base_dir, override_ids)?;
    config.reapply_resolution(desktop_mode.width, desktop_mode.height);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "oh-app-lifecycle-{}-{}-{tag}",
            std::process::id(),
            std::thread::current()
                .name()
                .unwrap_or("t")
                .replace(|c: char| !c.is_ascii_alphanumeric(), "_")
        ));
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    /// C++ `createFolderIfNonExistant`: creates the folder when missing,
    /// is a silent no-op when present (the C++ `isFolder()` early return —
    /// the log line is pinned by the binary's subprocess tests). The
    /// helper is folder-agnostic (02c1 retired the `Profiles/` folder the
    /// C++ call site created; the test drives it with neutral names).
    #[test]
    fn folder_created_once_idempotent_after() {
        let dir = temp_dir("folder");
        create_folder_if_non_existant(&dir, "Data/");
        create_folder_if_non_existant(&dir, "Packs/");
        assert!(dir.join("Data").is_dir());
        assert!(dir.join("Packs").is_dir());
        // Second pass: the folders exist → the C++ returns before logging
        // and before touching the filesystem again.
        create_folder_if_non_existant(&dir, "Data/");
        assert!(dir.join("Data").is_dir());
        let _ = fs::remove_dir_all(&dir);
    }

    /// No `config.json` / `ConfigOverrides/` in the base dir → the C++
    /// `root()` stays the null object (all defaults) and
    /// `reapplyResolution` recalculates sizes from them.
    #[test]
    fn load_and_reapply_missing_config_keeps_defaults() {
        let dir = temp_dir("defaults");
        let mut config = Config::default();
        load_and_reapply(&mut config, &dir, &[], oh_render::VideoMode::new(0, 0)).expect("load");
        assert_eq!(config.get_windowed_width(), 800, "NSDMI default");
        assert!(config.get_official(), "NSDMI default");
        // recalculateSizes: max(800, 600) * 1.3.
        assert_eq!(config.get_size_x(), 800.0_f32 * 1.3_f32);
        let _ = fs::remove_dir_all(&dir);
    }

    /// The desktop-mode parameter reaches the `applyAuto*Resolution`
    /// branches (Config.cpp:478-489): a config with
    /// `windowed_auto_resolution` / `fullscreen_auto_resolution` enabled
    /// takes its windowed/fullscreen sizes from the passed mode (the C++
    /// `sf::VideoMode::getDesktopMode()` read) — the 1920x1080 hardcode
    /// the port used to feed this (issue 029m, 29r1) is gone.
    #[test]
    fn load_and_reapply_feeds_the_desktop_mode_to_auto_resolution() {
        let dir = temp_dir("auto_res");
        fs::write(
            dir.join("config.json"),
            "{\"windowed_auto_resolution\": true, \"fullscreen_auto_resolution\": true}",
        )
        .expect("config");
        let mut config = Config::default();
        load_and_reapply(
            &mut config,
            &dir,
            &[],
            oh_render::VideoMode::new(2560, 1440),
        )
        .expect("load");
        assert_eq!(config.get_windowed_width(), 2560, "windowed auto");
        assert_eq!(config.get_windowed_height(), 1440, "windowed auto");
        assert_eq!(config.get_fullscreen_width(), 2560, "fullscreen auto");
        assert_eq!(config.get_fullscreen_height(), 1440, "fullscreen auto");
        let _ = fs::remove_dir_all(&dir);
    }

    /// A shipped-style `config.json` + a named override in
    /// `ConfigOverrides/`: the override is applied ONLY when its
    /// extension-less name is among the residual args (the C++
    /// `ssvu::contains(mOverridesIds, ...)` test).
    #[test]
    fn load_and_reapply_applies_only_named_overrides() {
        let dir = temp_dir("override");
        fs::create_dir_all(dir.join("ConfigOverrides")).expect("dir");
        fs::write(
            dir.join("ConfigOverrides/foo.json"),
            "{\"windowed_width\": 640}",
        )
        .expect("override");

        let mut config = Config::default();
        load_and_reapply(
            &mut config,
            &dir,
            &["open-hexagon".to_string(), "foo".to_string()],
            oh_render::VideoMode::new(0, 0),
        )
        .expect("load");
        assert_eq!(config.get_windowed_width(), 640, "named override applied");

        let mut config = Config::default();
        load_and_reapply(
            &mut config,
            &dir,
            &["open-hexagon".to_string()],
            oh_render::VideoMode::new(0, 0),
        )
        .expect("load");
        assert_eq!(
            config.get_windowed_width(),
            800,
            "unlisted override NOT applied"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
