//! plan 002 issue 04b — the beat-driven palette/pulse:
//! `HexagonGame::apply_music_beat` stores the latest beat, and `update`
//! applies it to the style in the SAME frame (AFTER the style's own
//! update). `status.pulse` (the simulation/RNG feed) is never touched.
//!
//! Fixture: a fresh composed engine (apeirogon), no player input, and the
//! game RNG pinned to a fixed seed after `new_game` (the same idiom as
//! the `th-app` bridge tests) so two engines run bit-identical
//! simulations.

mod common;

use std::sync::Arc;

use th_config::TIME_STEP;
use th_math::hg::color::Color;
use th_rng::RandomNumberGenerator;

use common::{compose, release_dir};

const PACK: &str = "ohvrvanilla_vittorio_romeo_cube_1";
const LEVEL: &str = "ohvrvanilla_vittorio_romeo_cube_1_apeirogon";
const SEED: u64 = 0;

/// Apeirogon's style facts the assertions lean on (
/// `_RELEASE/Packs/cube/Styles/apeirogon.json`): hue range [149, 360],
/// pulse range [0, 1.8], a STATIC main colour (253,253,253,255) and a
/// first background colour (90,90,90,255) with pulse (75,15,15,0) — so
/// the beat hue is observed on `currentHue` and the beat pulse on
/// `colors[0]`.
const HUE_BASE: f32 = 250.0; // inside [149, 360] — no clamp interference

fn load_release() -> Arc<th_assets::Assets> {
    Arc::new(th_assets::Assets::new(release_dir(), true, false).expect("_RELEASE loads headless"))
}

/// A fresh engine with the game started and the game RNG pinned to
/// `SEED` (two engines then simulate bit-identically).
fn fixture(assets: &Arc<th_assets::Assets>) -> common::Composed {
    let c = compose(assets.clone(), release_dir(), false);
    c.game.new_game(PACK, LEVEL, true, 1.0);
    c.game.state_handle().lock().rng = RandomNumberGenerator::new(SEED);
    c.game.start();
    c
}

#[test]
fn apply_music_beat_sets_beat_pulse_immediately() {
    let assets = load_release();
    let c = fixture(&assets);

    // The energy mirrors into status.beat_pulse on the call (the style
    // application itself happens in the next `update`).
    c.game.apply_music_beat(0.0, 0.0, 20.0, HUE_BASE);
    assert_eq!(
        c.game.get_status().beat_pulse,
        20.0,
        "status.beat_pulse mirrors the energy"
    );
}

#[test]
fn beat_hue_and_pulse_apply_to_the_style_in_the_same_frame() {
    let assets = load_release();
    let c = fixture(&assets);

    // Baseline (before any update): hue = hueMin, the background colour
    // at pulse 0, the static main colour.
    let style0 = c.game.get_style_data();
    assert_eq!(style0.get_current_hue(), 149.0);
    assert_eq!(style0.get_colors()[0], Color::new(90, 90, 90, 255));
    assert_eq!(*style0.get_main_color(), Color::new(253, 253, 253, 255));

    // Phases 0 ⇒ no shimmer; energy 1.0 inside the pulse range [0, 1.8].
    c.game.apply_music_beat(0.0, 0.0, 1.0, HUE_BASE);
    c.game.update(TIME_STEP, 1.0);

    let style1 = c.game.get_style_data();
    assert_eq!(
        style1.get_current_hue(),
        HUE_BASE,
        "the beat hue lands in the SAME frame (the style animation would \
         only have moved 149.0 → 149.25)"
    );
    // colours[0] = (90, 90, 90, 255) + (75, 15, 15, 0) × 1.0.
    assert_eq!(
        style1.get_colors()[0],
        Color::new(165, 105, 105, 255),
        "the beat pulse lands in the SAME frame"
    );
    // The static (non-dynamic) main colour is untouched by the beat.
    assert_eq!(
        *style1.get_main_color(),
        Color::new(253, 253, 253, 255),
        "static colours do not follow the beat"
    );
}

#[test]
fn the_beat_shimmer_is_tiny_and_deterministic() {
    let assets = load_release();

    // Phase 0 on both cycles: zero shimmer — the base hue lands exactly.
    let c = fixture(&assets);
    c.game.apply_music_beat(0.0, 0.0, 0.0, HUE_BASE);
    c.game.update(TIME_STEP, 1.0);
    assert_eq!(
        c.game.get_style_data().get_current_hue(),
        HUE_BASE,
        "phase 0: base hue is the dominant (exact) term"
    );

    // Quarter-phase on both cycles: +1.5° (two half-amplitude sines at
    // their maxima).
    let c = fixture(&assets);
    c.game.apply_music_beat(0.25, 0.25, 0.0, HUE_BASE);
    c.game.update(TIME_STEP, 1.0);
    let hue = c.game.get_style_data().get_current_hue();
    assert!(
        (hue - (HUE_BASE + 1.5)).abs() < 0.01,
        "shimmer at quarter-phase ≈ +1.5° (got {hue})"
    );

    // Three-quarter phase: −1.5°.
    let c = fixture(&assets);
    c.game.apply_music_beat(0.75, 0.75, 0.0, HUE_BASE);
    c.game.update(TIME_STEP, 1.0);
    let hue = c.game.get_style_data().get_current_hue();
    assert!(
        (hue - (HUE_BASE - 1.5)).abs() < 0.01,
        "shimmer at three-quarter phase ≈ −1.5° (got {hue})"
    );
}

#[test]
fn no_beat_keeps_the_existing_animation_and_the_simulation_is_untouched() {
    let assets = load_release();

    // Engine A: plain updates (no beat).
    let a = fixture(&assets);
    // Engine C: the same run with a beat applied every frame.
    let c = fixture(&assets);
    for _ in 0..3 {
        c.game.apply_music_beat(0.0, 0.0, 1.0, HUE_BASE);
        a.game.update(TIME_STEP, 1.0);
        c.game.update(TIME_STEP, 1.0);
    }

    let (a_st, c_st) = (a.game.get_status(), c.game.get_status());
    assert_eq!(
        a_st.pulse, c_st.pulse,
        "status.pulse (the simulation/RNG feed) is identical with and \
         without the beat"
    );
    assert_eq!(
        a.game.state_handle().lock().rng.engine_state(),
        c.game.state_handle().lock().rng.engine_state(),
        "the game RNG stream is untouched by the beat (no extra draws, \
         and the beat values are not fed back into the RNG advance)"
    );

    // The no-beat engine keeps the style's own animation (hueMin +
    // 3 × increment × FT × dm^0.8 = 149.0 + 0.75).
    assert_eq!(a.game.get_style_data().get_current_hue(), 149.75);
    // …while the beat engine carries the beat hue.
    assert_eq!(c.game.get_style_data().get_current_hue(), HUE_BASE);
}
