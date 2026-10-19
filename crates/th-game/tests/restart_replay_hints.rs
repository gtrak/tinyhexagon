//! 29a2 — the `newGame` restart input-hint string (the C++
//! `HexagonGame.cpp:758-821` `if(window != nullptr)` block's RESTART
//! half, ported VERBATIM; the C++'s replay-input half was retired by
//! plan 002 issue 02c2 with the replay removal):
//! `getKeyboardBindNames(Tid::Restart)` trimmed to the FIRST key (the
//! `find(',')` + `erase` half), + `getJoystickBindName` (the
//! vendor-aware button name — MS `0x045E` / Sony `0x54C` / the
//! plain-number fallback) assembled into the "PRESS … TO RESTART" /
//! "OR JOYSTICK <btn>" / "PRESS JOYSTICK <btn> TO RESTART" / "NO
//! RESTART BUTTON SET" forms, each with the C++'s trailing `\n`.
//! `update_text` already reads the field (under the `has_died` gate),
//! so a windowed player who died now gets the C++'s hint; the string
//! must match the C++ CHARACTER-FOR-CHARACTER.
//!
//! ONE `#[test]`: the scenarios share one windowed composition (each
//! `new_game` re-derives the hints from the CURRENT config + the seam's
//! vendor id — the C++'s live reads), plus one headless composition for
//! the `window != nullptr` gate.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, RwLock};

use th_config::{Combo, Config, Jdir, Jid, Tid};
use th_game::seams::{Joystick, PcgCosmeticRng};

mod common;

use common::{compose_with_config, release_dir};

const PACK: &str = "ohvrvanilla_vittorio_romeo_cube_1";
const LEVEL: &str = "ohvrvanilla_vittorio_romeo_cube_1_apeirogon";

/// The `Joystick` seam with a configurable `vendor_id()` (the C++
/// `getJoystickBindName`'s `isConnected(0) ? vendorId : 0` read — the
/// seam's answer); every other query false (the machine is not involved
/// in the hint block).
#[derive(Clone)]
struct VendorJoystick {
    vendor: Arc<AtomicU32>,
}

impl Joystick for VendorJoystick {
    fn update(&self, _: f32) {}
    fn pressed_dir(&self, _: Jdir) -> bool {
        false
    }
    fn pressed_id(&self, _: Jid) -> bool {
        false
    }
    fn rising_edge_id(&self, _: Jid) -> bool {
        false
    }
    fn vendor_id(&self) -> u32 {
        self.vendor.load(Ordering::Relaxed)
    }
}

#[test]
fn new_game_builds_the_cpp_restart_replay_hints_verbatim() {
    let assets = Arc::new(
        th_assets::Assets::new(release_dir(), true, false).expect("_RELEASE loads headless"),
    );
    let vendor = Arc::new(AtomicU32::new(0));
    let joystick: Arc<dyn Joystick> = Arc::new(VendorJoystick {
        vendor: vendor.clone(),
    });

    // Defaults: t_restart = [SPACE, ENTER, MMB], t_replay = [Y],
    // j_restart = 0, j_replay = 6 (config.rs:499-510).
    let config = Arc::new(RwLock::new(Config::default()));
    let c = compose_with_config(
        assets,
        release_dir(),
        true, /* window — the C++'s `window != nullptr` gate */
        config.clone(),
        None,
        joystick,
        Arc::new(PcgCosmeticRng::new()),
    );

    // ------------------------------------------------------------------
    // A: vendor 0 (the C++'s `isConnected(0) == false` read —
    //    `getJoystickBindName`'s `ssvu::toStr(value)` plain-number
    //    fallback, Config.cpp:1603).
    // ------------------------------------------------------------------
    vendor.store(0, Ordering::Relaxed);
    c.game.new_game(PACK, LEVEL, true, 1.0);
    let status = c.game.get_status();
    assert_eq!(
        status.restart_input, "PRESS SPACE OR JOYSTICK 0 TO RESTART\n",
        "t_restart 'SPACE, ENTER, MMB' trimmed to the first key + the \
         plain-number j_restart name (value 0, vendor 0)"
    );

    // ------------------------------------------------------------------
    // B: the Microsoft vendor (0x045E) — the XBOX 360 button names
    //    (Config.cpp:1565-1572 column 0: 0 = "A", 6 = "BACK").
    // ------------------------------------------------------------------
    vendor.store(0x045E, Ordering::Relaxed);
    c.game.new_game(PACK, LEVEL, true, 1.0);
    let status = c.game.get_status();
    assert_eq!(
        status.restart_input,
        "PRESS SPACE OR JOYSTICK A TO RESTART\n"
    );

    // ------------------------------------------------------------------
    // C: the Sony vendor (0x54C) — column 1 (0 = "SQUARE", 6 = "L2").
    // ------------------------------------------------------------------
    vendor.store(0x54C, Ordering::Relaxed);
    c.game.new_game(PACK, LEVEL, true, 1.0);
    let status = c.game.get_status();
    assert_eq!(
        status.restart_input,
        "PRESS SPACE OR JOYSTICK SQUARE TO RESTART\n"
    );
    // ------------------------------------------------------------------
    // D: NO keyboard restart bind (the "PRESS JOYSTICK <btn>" variant).
    // ------------------------------------------------------------------
    {
        let mut cfg = config.write().expect("config poisoned");
        *cfg.get_trigger(Tid::Restart).get_combos_mut() = vec![Combo::cleared()];
    }
    c.game.new_game(PACK, LEVEL, true, 1.0);
    let status = c.game.get_status();
    assert_eq!(
        status.restart_input, "PRESS JOYSTICK SQUARE TO RESTART\n",
        "the empty keyboard name's variant (Sony vendor still 0x54C)"
    );

    // ------------------------------------------------------------------
    // E: NOTHING set on either side (the "NO … BUTTON SET" variants —
    //    `getJoystickBindName` returns "" for the value-33 unbound
    //    marker, Config.cpp:1574-1576).
    // ------------------------------------------------------------------
    {
        let mut cfg = config.write().expect("config poisoned");
        cfg.set_joystick_bind_value(Jid::Restart, 33);
    }
    c.game.new_game(PACK, LEVEL, true, 1.0);
    let status = c.game.get_status();
    assert_eq!(status.restart_input, "NO RESTART BUTTON SET\n");

    // ------------------------------------------------------------------
    // F: the C++ `value >= 12` guard (MS/Sony vendors): a joystick bind
    //    at 12+ reads as UNBOUND (the "" name) → the "NO … BUTTON SET"
    //    variant even with the keyboard side bound… no — the keyboard
    //    side wins: "PRESS SPACE TO RESTART" (no joystick half).
    // ------------------------------------------------------------------
    {
        let mut cfg = config.write().expect("config poisoned");
        cfg.set_joystick_bind_value(Jid::Restart, 12);
        *cfg.get_trigger(Tid::Restart).get_combos_mut() = vec![{
            let mut c = Combo::new();
            c.add_key(th_config::Key::SPACE);
            c
        }];
    }
    c.game.new_game(PACK, LEVEL, true, 1.0);
    let status = c.game.get_status();
    assert_eq!(
        status.restart_input, "PRESS SPACE TO RESTART\n",
        "the vendor's value>=12 guard empties the joystick name — the \
         keyboard-only form (no ' OR JOYSTICK' half)"
    );

    // ------------------------------------------------------------------
    // G: HEADLESS (the C++ `window != nullptr` gate — a second
    //    composition, `window = false`): the string stays EMPTY even
    //    with the restart bind bound (the C++ headless tests' state).
    // ------------------------------------------------------------------
    vendor.store(0x045E, Ordering::Relaxed);
    {
        let mut cfg = config.write().expect("config poisoned");
        cfg.set_joystick_bind_value(Jid::Restart, 0);
    }
    let assets2 = Arc::new(
        th_assets::Assets::new(release_dir(), true, false).expect("_RELEASE loads headless"),
    );
    let vendor2 = Arc::new(AtomicU32::new(0x54C));
    let c_headless = compose_with_config(
        assets2,
        release_dir(),
        false, /* headless — the C++'s `window == nullptr` */
        config.clone(),
        None,
        Arc::new(VendorJoystick { vendor: vendor2 }),
        Arc::new(PcgCosmeticRng::new()),
    );
    c_headless.game.new_game(PACK, LEVEL, true, 1.0);
    let status = c_headless.game.get_status();
    assert_eq!(
        status.restart_input, "",
        "headless: the C++'s `window == nullptr` gate leaves the hint empty"
    );
}
