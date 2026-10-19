//! plan 002 issue 07 — the tiny runtime's headless tests (07's plan
//! "Headless tests"): the 60 s-class simulated run (stage advance, no
//! death, wall/player batches), the forced collision (bump, no death,
//! stage/time preserved, the timescale), the reset (stage unchanged,
//! elapsed reset, `new_game` on the same level/multiplier), the
//! left/right input (the player angle), the zero text draws with the
//! all-false `DrawOptions`, and the no-persistence proof (the temp root
//! stays free of `config.json` / `Profiles/` / `Replays/`).
//!
//! The composition shape mirrors `tests/bridge.rs` + `tests/common`
//! (the `th_app::tiny::TinyRuntime` composes the same de-scoped engine;
//! the tests pass `audio: None` — the engine's null-audio seam — and pin
//! the game stream with the 02c2 seed-injection idiom).

mod common;

use std::sync::Arc;

use th_render::Vec2;
use th_rng::RandomNumberGenerator;
use th_synth::MusicDirector;

use common::{release_dir, DrawEvent, Recording};
use th_app::tiny::{kid_config, FrameEvents, TinyRuntime, PACK};

/// The `_RELEASE` load idiom (the bridge test's).
fn load_release() -> Arc<th_assets::Assets> {
    Arc::new(th_assets::Assets::new(release_dir(), true, false).expect("_RELEASE loads headless"))
}

/// A windowed-runtime composition with the kid config (the tests' shape).
fn kid_runtime(assets: Arc<th_assets::Assets>, window: bool) -> TinyRuntime {
    TinyRuntime::new(
        assets,
        release_dir(),
        window,
        kid_config(th_config::Config::default()),
        None,
        MusicDirector::start(),
    )
}

/// The frame's no-input state (the kid's idle hand).
fn idle_events() -> FrameEvents {
    FrameEvents::default()
}

/// `dt` in f32-exact seconds (1/64 s — 64 frames per game-second, so a
/// 60 s run is exactly `60 * 64` frames).
const DT: f32 = 1.0 / 64.0;

/// The 60 s simulated run (65 s here — 5 s into the stage-2 level, so
/// the frame's wall batch is live): no panic, the stage advances at 30 s
/// (same level — `set_difficulty_mult`) and 60 s (the level change —
/// `new_game` on the river level), no `has_died` (the soft collisions),
/// and the frame carries the wall / player / background / flash batches.
#[test]
fn the_sixty_second_run_advances_the_stage_and_never_dies() {
    let assets = load_release();
    let mut rt = kid_runtime(assets, true);
    // Pin the game stream (the bridge test's seed injection — the 02c2
    // replacement for the retired replay seed).
    rt.game().state_handle().lock().rng = RandomNumberGenerator::new(0);

    for _ in 0..(65 * 64) {
        rt.frame(DT, &idle_events());
    }

    assert_eq!(rt.journey.stage, 2, "the journey advanced at 30 s and 60 s");
    assert!(
        (rt.journey.elapsed - 5.0).abs() < 1e-6,
        "5 s into the stage-2 level"
    );
    assert!(
        !rt.game().get_status().has_died,
        "the soft collisions never set has_died"
    );
    assert!(rt.game().get_status().started, "the run is live");
    // The stage-2 (even) transition ran `new_game` on the river level
    // with the stage-2 multiplier. The engine's `LevelData` carries the
    // level's short id (the JSON's `"id"`) + the full pack id; the
    // registry's `{pack}_{level}` key is what `new_game` takes.
    let level = rt.game().get_level_data();
    assert_eq!(level.pack_id, PACK, "the tiny pack");
    assert_eq!(level.id, "river", "stage 2 plays the river level");
    assert!(
        (rt.game().difficulty_mult() - 0.61).abs() < 1e-6,
        "the stage-2 multiplier (0.55 + 0.03 × 2)"
    );

    // The frame carries the scene: the wall + the player batches (plus
    // the background and the flash polygon).
    let mut rec = Recording::default();
    rt.draw(&mut rec, 1024, 768);
    let counts: Vec<usize> = rec
        .events
        .iter()
        .filter_map(|e| match e {
            DrawEvent::Vertices { count, .. } => Some(*count),
            _ => None,
        })
        .collect();
    assert!(
        counts.contains(&18),
        "the background (6 sides × 3): {counts:?}"
    );
    assert!(
        counts.iter().any(|c| *c > 0 && *c % 4 == 0),
        "the wall quads (4 verts each): {counts:?}"
    );
    assert!(counts.contains(&3), "the player tris: {counts:?}");
    assert!(counts.contains(&6), "the flash polygon: {counts:?}");
}

/// A forced collision: the player spins into the first wall — the soft
/// collision KILLS THE WALL (the bump: the SFX / the synth duck / the
/// 0.35 s @ 0.35× slow-motion), never the player; the stage / the stage
/// clock are preserved by the collision (the journey is real-time, not
/// collision-driven).
#[test]
fn a_forced_collision_bumps_without_dying_and_applies_the_timescale() {
    let assets = load_release();
    let mut rt = kid_runtime(assets, false);
    rt.game().state_handle().lock().rng = RandomNumberGenerator::new(0);
    // The `alwaysSpinRight` (the bridge test's input idiom): the engine
    // overrides the impl flags each tick, so the helping hand cannot
    // rescue the player from the first wall.
    rt.game().set_always_spin_right(true);

    let mut bumped = false;
    for _ in 0..(20 * 64) {
        // The level's first pattern spawns its walls ~3 s in (the script's
        // `t_wait(180)`); at meadow's 0.55× speed multiplier the walls then
        // take ~12 s more to travel the default 1600 px spawn distance in,
        // so the first soft bump lands ~16 s in (seed-0 stream) — the 20 s
        // budget covers it.
        rt.frame(DT, &idle_events());
        if rt.game().bump_serial() > 0 {
            bumped = true;
            break;
        }
    }
    assert!(bumped, "the soft collision bumped within the 20 s budget");
    assert!(
        !rt.game().get_status().has_died,
        "the soft collision never sets has_died"
    );
    // The bump re-armed the slow-motion (the plan's 0.35 s @ 0.35×).
    assert!(rt.bump_active(), "the bump's slow-motion is active");
    // The stage / the stage clock are preserved (the collision is not a
    // stage event — the journey kept running in real time).
    let (stage, elapsed) = (rt.journey.stage, rt.journey.elapsed);
    rt.frame(DT, &idle_events());
    assert_eq!(
        rt.journey.stage, stage,
        "the stage is preserved by the bump"
    );
    assert!(
        (rt.journey.elapsed - elapsed - DT).abs() < 1e-6,
        "the stage clock kept advancing (real time, bump or not)"
    );
}

/// The reset (the icon's mouse click): the stage is unchanged, the stage
/// clock resets, and the engine ran `new_game` + `start` on the SAME
/// level + multiplier (the stage-0 meadow @ 0.55×) — the player angle
/// re-zeroes (the fresh `CPlayer`).
#[test]
fn the_reset_restarts_the_stage_without_changing_it() {
    let assets = load_release();
    let mut rt = kid_runtime(assets, false);
    rt.game().state_handle().lock().rng = RandomNumberGenerator::new(0);

    // Play 5 s into stage 0 (the meadow level @ 0.55×).
    for _ in 0..(5 * 64) {
        rt.frame(DT, &idle_events());
    }
    assert!(
        (rt.journey.elapsed - 5.0).abs() < 1e-6,
        "5 s into the stage"
    );

    // The reset icon's mouse click (the top-left ~64×64 at (32, 32) — the
    // `FrameEvents.mouse_left` edge is the click source; `frame` reads it
    // straight), then the release (no re-tap).
    let down = FrameEvents {
        mouse_left: true,
        mouse_pos: Vec2::new(32.0, 32.0),
        width: 1024.0,
        ..Default::default()
    };
    let actions = rt.frame(DT, &down);
    assert!(!actions.close && !actions.toggle_fullscreen);
    let up = idle_events();
    rt.frame(DT, &up);

    // No stage change (still 0 — the meadow @ 0.55×); the stage clock
    // restarted (two frames' worth); the engine re-zeroed the game.
    assert_eq!(rt.journey.stage, 0, "no stage change");
    assert!(
        (rt.journey.elapsed - 2.0 * DT).abs() < 1e-6,
        "the stage clock restarted"
    );
    assert!(
        (rt.game().difficulty_mult() - 0.55).abs() < 1e-6,
        "the stage-0 multiplier"
    );
    assert_eq!(
        rt.game().get_level_data().id,
        "meadow",
        "`new_game` re-ran on the same level"
    );
    assert!(
        rt.game().get_status().started,
        "the reset re-armed the start"
    );
    assert!(
        rt.game().get_player_angle().abs() < 1e-6,
        "the player angle re-zeroed (the fresh CPlayer)"
    );
}

/// Left/right input changes the player angle: the keyboard A/LEFT hold
/// steers counter-clockwise (the angle falls), the D/RIGHT hold steers
/// clockwise (it rises back over zero), and the touch halves drive the
/// same impl flags (the kid's mobile input — a left-half finger turns
/// counter-clockwise).
#[test]
fn left_and_right_input_change_the_player_angle() {
    let assets = load_release();
    let mut rt = kid_runtime(assets, false);
    // 08a: the helping hand is armed before any input (the web probe's
    // `help=auto` baseline observable).
    assert!(
        !rt.help_disabled(),
        "the helping hand is armed before any input"
    );

    // A (or LEFT) held → counter-clockwise.
    let mut ev = FrameEvents::default();
    ev.input.set_key_pressed(th_render::Key::A, true);
    for _ in 0..64 {
        rt.frame(DT, &ev);
    }
    let ccw = rt.game().get_player_angle();
    assert!(ccw < 0.0, "A steers counter-clockwise (angle {ccw})");
    // 08a: ANY input disables the helping hand for the session (the web
    // probe's `help=off` after-input observable).
    assert!(
        rt.help_disabled(),
        "any input disables the helping hand for the session"
    );

    // D (or RIGHT) held → clockwise, re-crossing zero.
    let mut ev = FrameEvents::default();
    ev.input.set_key_pressed(th_render::Key::D, true);
    for _ in 0..128 {
        rt.frame(DT, &ev);
    }
    let cw = rt.game().get_player_angle();
    assert!(cw > 0.0, "the D steer re-crosses zero (angle {cw})");

    // The touch halves: a left-half finger steers counter-clockwise
    // (from the current angle, which is positive).
    let before = rt.game().get_player_angle();
    let mut ev = FrameEvents {
        finger_count: 1,
        width: 1024.0,
        ..Default::default()
    };
    ev.fingers[0] = (0, Vec2::new(200.0, 400.0));
    for _ in 0..32 {
        rt.frame(DT, &ev);
    }
    assert!(
        rt.game().get_player_angle() < before,
        "the left half steers counter-clockwise ({before} → \
         {})",
        rt.game().get_player_angle()
    );
}

/// With the kid's all-false `DrawOptions`, a recording target sees ZERO
/// text draws (and no key icons / level info) while the scene (the
/// background / walls / player / flash) + the two corner icons (screen-
/// space rectangles + the reset's triangle) keep drawing.
#[test]
fn the_kid_options_draw_zero_text_with_the_scene_intact() {
    let assets = load_release();
    let mut rt = kid_runtime(assets, true);
    rt.game().state_handle().lock().rng = RandomNumberGenerator::new(0);
    for _ in 0..(10 * 64) {
        rt.frame(DT, &idle_events());
    }

    let mut rec = Recording::default();
    rt.draw(&mut rec, 1024, 768);

    assert_eq!(
        rec.events
            .iter()
            .filter(|e| matches!(e, DrawEvent::Text { .. }))
            .count(),
        0,
        "the all-false DrawOptions draw zero text"
    );
    assert_eq!(
        rec.events
            .iter()
            .filter(|e| matches!(e, DrawEvent::Rectangle { .. }))
            .count(),
        6,
        "the two icons' six rectangles (1 backing + 1 backing + 4 expand \
         bars) and no level-info box"
    );
    let counts: Vec<usize> = rec
        .events
        .iter()
        .filter_map(|e| match e {
            DrawEvent::Vertices { count, .. } => Some(*count),
            _ => None,
        })
        .collect();
    assert!(
        counts.contains(&18) && counts.contains(&6) && counts.contains(&3),
        "the scene (background / flash / player) still draws: {counts:?}"
    );
    assert!(
        counts.contains(&3)
            && rec
                .events
                .iter()
                .filter(|e| matches!(e, DrawEvent::Vertices { count: 3, .. }))
                .count()
                >= 2,
        "the player tris + the reset icon's triangle: {counts:?}"
    );
}

/// No persistence: a run (composition + 10 s of frames + a rendered
/// frame) against a TEMP root (the `_RELEASE` content reached through
/// `Assets` / `Packs` symlinks) leaves the root free of
/// `config.json` / `Profiles/` / `Replays/` — the kid's config is
/// in-memory and the composition persists nothing.
#[test]
#[cfg(unix)]
fn the_run_writes_no_config_profiles_or_replays() {
    let release = release_dir();
    let thread = std::thread::current()
        .name()
        .unwrap_or("t")
        .replace(|c: char| !c.is_ascii_alphanumeric(), "_");
    let root = std::env::temp_dir().join(format!(
        "th-app-tiny-no-storage-{}-{thread}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("temp root");
    std::os::unix::fs::symlink(release.join("Assets"), root.join("Assets"))
        .expect("Assets symlink");
    std::os::unix::fs::symlink(release.join("Packs"), root.join("Packs")).expect("Packs symlink");
    let config_before = std::fs::read(release.join("config.json")).expect("_RELEASE config.json");

    let assets =
        Arc::new(th_assets::Assets::new(&root, true, false).expect("the temp root loads headless"));
    let mut rt = TinyRuntime::new(
        assets,
        root.clone(),
        true,
        kid_config(th_config::Config::default()),
        None,
        MusicDirector::start(),
    );
    for _ in 0..(10 * 64) {
        rt.frame(DT, &idle_events());
    }
    let mut rec = Recording::default();
    rt.draw(&mut rec, 1024, 768);
    assert!(!rec.events.is_empty(), "the frame rendered");

    // The temp root's own entries: nothing but the two planted symlinks.
    let entries: Vec<String> = std::fs::read_dir(&root)
        .expect("root read")
        .map(|e| e.expect("entry").file_name().into_string().expect("name"))
        .filter(|n| n != "Assets" && n != "Packs")
        .collect();
    assert!(
        entries.is_empty(),
        "no writes beside the symlinks: {entries:?}"
    );
    assert!(!root.join("config.json").exists(), "no config.json");
    assert!(!root.join("Profiles").exists(), "no Profiles/");
    assert!(!root.join("Replays").exists(), "no Replays/");
    // The `_RELEASE` config is byte-identical (the kid's config never
    // saves back — a write-through the symlinks would still be a write
    // the runtime did not intend).
    assert_eq!(
        std::fs::read(release.join("config.json")).expect("config.json"),
        config_before,
        "_RELEASE/config.json untouched"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// The journey's accelerated 10-minute run (07's verification): 600 s of
/// real time reaches the stage-20 plateau and holds it (no further
/// difficulty / tempo / hue / level increase) — the stage-up WORK the
/// runtime performs per transition is pinned by the 65 s run above
/// (both the even `new_game` and the odd `set_difficulty_mult` halves of
/// the first two transitions); the table itself is the journey module's
/// territory.
#[test]
fn the_ten_minute_run_reaches_stage_20_and_plateaus() {
    let mut journey = th_app::journey::Journey::new();
    let mut last_advanced: Option<u32> = None;
    for second in 0..600 {
        // Advance in 64 f32-exact slices (the frame dt shape).
        for _ in 0..64 {
            if let Some(stage) = journey.advance(DT) {
                last_advanced = Some(stage);
            }
        }
        let _ = second;
    }
    assert_eq!(
        journey.stage, 20,
        "600 s of real time reaches the plateau stage"
    );
    assert_eq!(last_advanced, Some(20), "the last transition was 19 → 20");
    assert!(journey.elapsed < th_app::journey::STAGE_DURATION_S);

    // The plateau: another 60 s, no transition, the table holds.
    let (diff, bpm, hue) = (
        th_app::journey::difficulty_for_stage(20),
        th_app::journey::bpm_for_stage(20),
        th_app::journey::base_hue_for_stage(20),
    );
    for _ in 0..(60 * 64) {
        assert!(journey.advance(DT).is_none(), "the plateau holds");
    }
    assert_eq!(journey.stage, 20);
    assert_eq!(th_app::journey::difficulty_for_stage(20), diff);
    assert_eq!(th_app::journey::bpm_for_stage(20), bpm);
    assert_eq!(th_app::journey::base_hue_for_stage(20), hue);
    assert!(
        (th_app::journey::difficulty_for_stage(20) - 1.15).abs() < 1e-6
            && (th_app::journey::bpm_for_stage(20) - 100.0).abs() < 1e-6,
        "the plateau's capped values"
    );
}
