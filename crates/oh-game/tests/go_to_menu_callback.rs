//! Plan 002 issue 02c3: the non-replay `go_to_menu` / `fn_go_to_menu`
//! coverage (the C++ `goToMenu(mSendScores, mError)` + the app-level
//! `fnGoToMenu` callback, HexagonGame.cpp:1241-1280).
//!
//! The C++-facing callers of the path are pinned elsewhere: the
//! Lippincott error path `goToMenu(false, true)` (`score_override_error.rs`),
//! the joystick Exit edge `goToMenu(true, false)` (`joystick.rs`), and the
//! windowed `onRenderStage` error path in the `oh-app` bridge. This file
//! pins the API itself: the C++ hardcoded-Escape / `Tid::Exit` bind body
//! (`trigger_go_to_menu` → `goToMenu()` = the default-argument call
//! `goToMenu(true /*mSendScores*/, false /*mError*/)`, HexagonGame.hpp:417 —
//! the `mSendScores` argument's `saveReplay()` half was retired by plan 002
//! issue 02c2 with the replay removal) and the error form
//! `go_to_menu(false, true)` the bridge's Lippincott error path uses —
//! each asserting the callback fires with the matching `mError` flag.
//!
//! Windowed: the C++ `goToMenu` head-gate (`window == nullptr → log +
//! return`, HexagonGame.cpp:1241-1247) skips the `fnGoToMenu` callback
//! headless, and the observable of this test is exactly that callback.

mod common;

use std::sync::Arc;

use common::{capture_go_to_menu, compose, release_dir};

const PACK: &str = "ohvrvanilla_vittorio_romeo_cube_1";
const LEVEL: &str = "ohvrvanilla_vittorio_romeo_cube_1_apeirogon";

#[test]
fn trigger_go_to_menu_fires_the_callback_without_the_error_flag() {
    let assets = Arc::new(
        oh_assets::Assets::new(release_dir(), true, false).expect("_RELEASE loads headless"),
    );
    let c = compose(assets, release_dir(), true);
    let seen = capture_go_to_menu(&c.game);

    c.game.new_game(PACK, LEVEL, true, 1.0);
    assert_eq!(*seen.lock().expect("poisoned"), None, "no menu return yet");

    // The C++ hardcoded-Escape / `Tid::Exit` bind body
    // (HexagonGame.cpp:379-383): `goToMenu()`.
    c.game.trigger_go_to_menu();

    assert_eq!(
        *seen.lock().expect("poisoned"),
        Some(false),
        "the Escape/Exit bind body must take the non-error menu path"
    );
}

#[test]
fn go_to_menu_error_form_fires_the_callback_with_the_error_flag() {
    let assets = Arc::new(
        oh_assets::Assets::new(release_dir(), true, false).expect("_RELEASE loads headless"),
    );
    let c = compose(assets, release_dir(), true);
    let seen = capture_go_to_menu(&c.game);

    c.game.new_game(PACK, LEVEL, true, 1.0);

    // The Lippincott error form — the exact call the `oh-app` bridge's
    // `onRenderStage` / `run_lua_file` error paths make.
    c.game.go_to_menu(false, true);

    assert_eq!(
        *seen.lock().expect("poisoned"),
        Some(true),
        "the error form must take the mError menu path"
    );
}
