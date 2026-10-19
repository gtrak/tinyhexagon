//! plan 002 issue 07 — the tiny runtime: the kid's game (the
//! journey-driven fixed-step loop over the de-scoped engine composition,
//! `th-synth` music, `th-audio` SFX, the soft-collision bump / slow-motion,
//! the beat palette, and the two corner icons — no text, no menus, no
//! persistence).
//!
//! # Loop shape (issue 08a made the entry target-agnostic)
//!
//! [`TinyRuntime::frame`] is the window-agnostic per-frame half: it takes
//! the drained input ([`FrameEvents`]), runs the fixed 240 Hz engine ticks
//! (frame-dt clamp + the bump slow-motion), the journey, the beat palette
//! and the SFX / ducking, and returns the window layer's actions
//! ([`FrameActions`]). [`run`] is the shared windowed entry (issue 08a):
//! it takes the platform's built window — the desktop's winit window
//! ([`run_windowed`] is its thin caller), the web's emscripten canvas
//! window ([`tiny_web`] is its boot) — and drives the loop; issue 08b
//! feeds it the Android activity's window the same way.
//!
//! # Composition (07's "Composition")
//!
//! `Config::default()` + the kid values ([`kid_config`] — in-memory: no
//! `config.json` / `Profiles/` / `Replays/` writes, every run starts stage
//! 0), the `compose_with_config` de-scope (no profiles / replay / Steam /
//! Discord / menu), `set_soft_collisions(true)` once after composition
//! (04a), the engine's `no_music` (the OGG level music is off — `th-synth`
//! owns the soundtrack; `th-audio` owns the SFX), the engine's `Renderer`
//! seam `None` (the bridge draws through
//! [`crate::draw_target::GlDrawTarget`]), `DrawOptions` all false (04b —
//! zero text / HUD), the music `MusicDirector::set_stage` / `bump`, and
//! `apply_music_beat` every frame (the 04b P2 — the beat hue is never
//! cleared, so the app must call it every frame).

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use th_audio::Audio as TinyAudio;
use th_config::{Config, TIME_STEP};
use th_game::hexagon_game::HexagonGame;
use th_game::seams::{Audio, NoJoystick};
use th_lua::api::state::GameApiState;
use th_math::ssvu::math::{PI, TAU};
use th_render::graphics::{DrawOptions, DrawTarget};
use th_render::input::{InputState, FINGER_COUNT};
use th_render::types::RectangleShape;
use th_render::{Button, Color, Font, RenderStates, Vec2, Vertex, View};
use th_synth::MusicDirector;

use crate::bridge::{draw as bridge_draw, AppDraw};
use crate::compose::compose_with_config;
use crate::draw_target::{AssetsPackShaderProvider, GlDrawTarget};
use crate::journey::Journey;
use crate::touch::{icon_at, icon_position, Icon, TouchState, ICON_SIZE};

/// The tiny pack (06): `tinyhexagon_tinyhexagon_tiny_1` — the levels are
/// `{PACK}_{meadow,river,stars}` (the journey's level cycle).
pub const PACK: &str = "tinyhexagon_tinyhexagon_tiny_1";

/// The fixed engine tick rate (240 Hz — the C++ `TICKS_PER_SECOND`): one
/// `TIME_STEP` of frametime per tick, accumulated from the frame's `dt`.
const TICKS_PER_SECOND: f32 = 240.0;
/// The frame-`dt` clamp (the plan's "~0.1 s"): a hung / hidden frame must
/// not fast-forward the journey or drain the accumulator in one burst.
const MAX_FRAME_DT: f32 = 0.1;
/// The bump slow-motion: 0.35× timescale for 0.35 s (the plan's
/// "0.35/0.35 s").
const BUMP_TIMESCALE: f32 = 0.35;
const BUMP_DURATION_S: f32 = 0.35;
/// The helping hand (stage 0 only): engaged after this much idle time
/// (the plan's 2 s); any input disables it for the session.
const HELP_IDLE_S: f32 = 2.0;
/// The helping hand's dead zone (radians): hold the steering side until
/// the player is within this of the gap centre, then coast.
const HELP_DEAD_ZONE_RAD: f32 = 0.002;
/// The stage-up burst flash (the engine's flash value — no reusable
/// wall-burst particle spawner exists, 04a's soft-burst outcome; the
/// death flash is 255, the soft bump's 120 — the stage-up cue sits
/// between them, gentler).
const STAGE_UP_FLASH: f32 = 90.0;

/// The SFX ids (the plan's SFX table — the `_RELEASE/Assets` OGGs):
/// start `go.ogg` (the engine's `start`), collision `beep.ogg`, stage-up
/// `levelUp.ogg`, reset `restart.ogg`, icon tap `select.ogg`, fullscreen
/// `beep.ogg`.
const SFX_BUMP: &str = "beep.ogg";
const SFX_STAGE_UP: &str = "levelUp.ogg";
const SFX_RESET: &str = "restart.ogg";
const SFX_ICON_TAP: &str = "select.ogg";

/// The kid's config (07's "Composition"): the in-memory `Config::default()`
/// with the kid overrides applied over the given base — the binary runs
/// `load_and_reapply` over the base first (the C++ `mainClient`'s load +
/// reapply, the feed-forward's `DesktopModeSource::Lazy`), then this.
/// No OGG level music (the synth owns the soundtrack), 3D off, 1024×768.
/// The beat pulse stays ON (the default — the 04b `apply_music_beat`
/// drives `status.beat_pulse` directly), and the text / HUD suppression is
/// the `DrawOptions` (not the config). The tiny runtime persists nothing
/// (this `Config` never writes back).
#[must_use]
pub fn kid_config(mut config: Config) -> Config {
    config.set_no_music(true);
    config.set_3d(false);
    config.set_current_resolution(1024, 768);
    config
}

/// One frame's drained input (the window layer's responsibility — 08
/// feeds this from the Android activity / the web rAF too).
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameEvents {
    /// The window's current input state (keys / buttons).
    pub input: InputState,
    /// The down fingers `(id, position)` (the window's
    /// `finger_down_positions` order — finger 0 first); `finger_count`
    /// of them.
    pub fingers: [(u32, Vec2); FINGER_COUNT],
    pub finger_count: usize,
    /// The window's mouse left-button level — the icon's click source
    /// (`frame` reads THIS field for the mouse tap edge; `run_windowed`
    /// mirrors it from the drained `InputState`'s left button, so the
    /// windowed and the test-fed paths can never diverge).
    pub mouse_left: bool,
    /// The mouse position (window pixels).
    pub mouse_pos: Vec2,
    /// The window size (the icon hit tests + the half split).
    pub width: f32,
    pub height: f32,
}

/// The frame's window-layer actions (the engine side is already applied
/// by [`TinyRuntime::frame`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameActions {
    /// Esc was pressed — close the window.
    pub close: bool,
    /// The fullscreen icon was tapped (mouse click / touch) — the window
    /// layer toggles `set_fullscreen(!get_fullscreen())`.
    pub toggle_fullscreen: bool,
}

/// The tiny runtime (the kid's whole game): the composed engine + the
/// journey + the music director + the touch / helping-hand / bump state.
/// Main-thread object (the `MusicDirector` is; the engine's `LuaContext`
/// is).
pub struct TinyRuntime {
    composed: crate::compose::Composed,
    /// The SFX surface (`None` = the engine's null audio — the headless
    /// tests).
    audio: Option<Arc<TinyAudio>>,
    pub music: MusicDirector,
    /// The journey clock (stage 0..=20, seconds into the stage).
    pub journey: Journey,
    touch: TouchState,
    /// The windowed engine's draw state (`None` headless — the bridge's
    /// `window_present` guard no-ops the scene there).
    app_draw: Option<AppDraw>,
    /// The last-seen `bump_serial` (the delta fires the SFX / duck /
    /// slow-motion once per physical bump — 04a).
    last_bump: u64,
    /// The bump slow-motion's remaining time (real seconds).
    bump_remaining: f32,
    /// The helping hand is off for the session (any input set it).
    help_off: bool,
    /// Seconds without left/right input (the helping hand's engage timer).
    help_idle: f32,
    // (08a: `help_off` is the web probe's input-reached-the-frame
    // observable — [`TinyRuntime::help_disabled`]).
    /// The previous frame's Esc / mouse-left levels (the edges).
    last_esc: bool,
    last_mouse_left: bool,
    /// The fixed-step accumulator (leftover real seconds).
    accumulator: f32,
}

impl TinyRuntime {
    /// The kid composition (07): the caller's `Config` (usually
    /// [`kid_config`] over the loaded / default config), soft collisions,
    /// stage 0 (`new_game` + `start`), `DrawOptions` all false.
    /// `audio: None` = the engine's null-audio seam (the headless tests);
    /// `Some` = the real `th_audio::Audio` (the SFX + the engine's
    /// `go.ogg` start sound).
    #[must_use]
    pub fn new(
        assets: Arc<th_assets::Assets>,
        root: PathBuf,
        window: bool,
        config: Config,
        audio: Option<Arc<TinyAudio>>,
        music: MusicDirector,
    ) -> Self {
        let engine_audio: Option<Arc<dyn Audio>> = audio.as_ref().map(|a| {
            let engine_audio: Arc<dyn Audio> = a.clone();
            engine_audio
        });
        let composed = compose_with_config(
            assets,
            root.clone(),
            window,
            Arc::new(RwLock::new(config.clone())),
            None, // the engine's Renderer seam (the bridge draws — 07's plan)
            engine_audio,
            Arc::new(NoJoystick), // the kid has no controller binds
        );

        // 04a: the kid composition enables the soft (no-death) collisions
        // ONCE after composition; then stage 0 (the meadow level @ 0.55×)
        // starts now (the kid's run never waits for input to start).
        composed.game.set_soft_collisions(true);
        let journey = Journey::new();
        composed
            .game
            .new_game(PACK, &journey.level_id(PACK), true, journey.difficulty());
        composed.game.start();

        let app_draw = if window {
            let dir = root.join("Assets");
            let font =
                Font::load(&dir.join("OpenSquare-Regular.ttf")).expect("OpenSquare-Regular.ttf");
            let font_bold =
                Font::load(&dir.join("OpenSquare-Bold.ttf")).expect("OpenSquare-Bold.ttf");
            // The kid config's zoom factor (the C++ `initText` sizes are
            // divided by it — inert for the kid: every text pass is off).
            let mut app = AppDraw::new(&config, &dir, font, font_bold);
            // 04b: the kid's all-false options — zero text / level info /
            // key icons / FPS (the scene + the flash keep drawing).
            app.set_draw_options(DrawOptions {
                show_text: false,
                show_level_info: false,
                show_key_icons: false,
                show_fps: false,
            });
            Some(app)
        } else {
            None
        };

        Self {
            composed,
            audio,
            music,
            journey,
            touch: TouchState::new(),
            app_draw,
            last_bump: 0,
            bump_remaining: 0.0,
            help_off: false,
            help_idle: 0.0,
            last_esc: false,
            last_mouse_left: false,
            accumulator: 0.0,
        }
    }

    /// The engine (the tests pin its state / the bin arms the screenshot).
    #[must_use]
    pub fn game(&self) -> &HexagonGame<GameApiState> {
        &self.composed.game
    }

    /// The bump slow-motion is active (the engine ticks at 0.35× while it
    /// is).
    #[must_use]
    pub fn bump_active(&self) -> bool {
        self.bump_remaining > 0.0
    }

    /// The helping hand is disabled for the session (any input set it —
    /// 08a: the web probe's "the input reached the frame's edge"
    /// observable, the `status()` line's `help=` half).
    #[must_use]
    pub fn help_disabled(&self) -> bool {
        self.help_off
    }

    /// The C++ `window->getFPS()` half (the bridge's `onRenderStage`
    /// frametime's `60.f / fps` divisor — the window's live FPS).
    pub fn fps(&mut self, fps: f32) {
        if let Some(app) = &mut self.app_draw {
            app.fps = fps;
        }
    }

    /// One frame (the plan's loop steps 1–8; step 9 — the swap — is the
    /// window layer's [`th_render::window::Window`]): drain the input
    /// edges, set the held left/right, run the engine ticks with the bump
    /// timescale, consume the bumps (SFX / duck / slow-motion), advance
    /// the journey (stage-up / reset), and apply the beat.
    pub fn frame(&mut self, dt: f32, ev: &FrameEvents) -> FrameActions {
        let dt = dt.min(MAX_FRAME_DT);

        // -- 1. The input edges (keyboard / mouse / touch) ---------------
        let (touch_left, touch_right, touch_tap) =
            self.touch.feed(&ev.fingers[..ev.finger_count], ev.width);
        let key_left = ev.input.is_key_pressed(th_render::Key::A)
            || ev.input.is_key_pressed(th_render::Key::LEFT);
        let key_right = ev.input.is_key_pressed(th_render::Key::D)
            || ev.input.is_key_pressed(th_render::Key::RIGHT);
        let left_held = key_left || touch_left;
        let right_held = key_right || touch_right;

        // The Esc edge (the plan's "Esc: close the window").
        let esc = ev.input.is_key_pressed(th_render::Key::ESCAPE);
        let close = esc && !self.last_esc;
        self.last_esc = esc;

        // The icon taps: the touch down-edge + the mouse left down-edge.
        // The click source is `FrameEvents.mouse_left` (the window feed's
        // field — `run_windowed` mirrors it from the `InputState`'s left
        // button), so the icon tap fires from ONE place only.
        let mouse_down = ev.mouse_left;
        let mouse_tap = mouse_down && !self.last_mouse_left;
        self.last_mouse_left = mouse_down;
        let mouse_icon = mouse_tap
            .then(|| icon_at(ev.mouse_pos.x, ev.mouse_pos.y, ev.width))
            .flatten();

        // 2. The icon edges (the plan's SFX table: icon tap = select.ogg;
        //    the reset edge = restart.ogg; the fullscreen edge = beep.ogg).
        let mut toggle_fullscreen = false;
        if let Some(icon) = touch_tap.or(mouse_icon) {
            self.sfx(SFX_ICON_TAP);
            match icon {
                Icon::Reset => self.reset_stage(),
                Icon::Fullscreen => {
                    toggle_fullscreen = true;
                    self.sfx(SFX_BUMP); // the plan's "Fullscreen: beep.ogg"
                }
            }
        }

        // The helping hand (stage 0 only): any input disables it for the
        // session; after `HELP_IDLE_S` without it, it steers toward the
        // nearest gap.
        let any_input = key_left
            || key_right
            || ev.finger_count > 0
            || touch_tap.is_some()
            || mouse_icon.is_some();
        if any_input {
            self.help_off = true;
        } else {
            self.help_idle += dt;
        }
        let help_active = !self.help_off
            && self.journey.stage == 0
            && self.help_idle >= HELP_IDLE_S
            && self.composed.game.get_status().started;

        // -- 3. The fixed 240 Hz engine ticks (dt clamp + bump timescale) -
        self.accumulator += dt;
        let tick = 1.0 / TICKS_PER_SECOND;
        while self.accumulator >= tick {
            self.accumulator -= tick;
            // The bump slow-motion: 0.35× for BUMP_DURATION_S after a bump
            // (the slice that STARTS while the bump is live gets the slow
            // timescale).
            let slow = self.bump_remaining > 0.0;
            let timescale = if slow { BUMP_TIMESCALE } else { 1.0 };
            if slow {
                self.bump_remaining = (self.bump_remaining - tick).max(0.0);
            }
            // 4. The held left/right (the engine's impl flags — the C++'s
            //    `refreshInput`-once-per-frame shape: the burst's ticks all
            //    read this frame's state; the helping hand re-steers per
            //    tick, the player angle moving under it).
            if help_active {
                match self.helping_direction() {
                    Some(dir) => {
                        self.composed.game.set_input_impl_ccw(dir < 0);
                        self.composed.game.set_input_impl_cw(dir > 0);
                    }
                    None => {
                        self.composed.game.set_input_impl_ccw(false);
                        self.composed.game.set_input_impl_cw(false);
                    }
                }
            } else {
                self.composed.game.set_input_impl_ccw(left_held);
                self.composed.game.set_input_impl_cw(right_held);
            }
            self.composed.game.update(TIME_STEP, timescale);
            self.composed.game.post_update();
        }

        // -- 5. The bumps (the soft collisions — the `bump_serial` delta) -
        let serial = self.composed.game.bump_serial();
        if serial > self.last_bump {
            self.last_bump = serial;
            // The plan's SFX table: collision = beep.ogg (the gentle SFX
            // volume); the synth ducks (0.55 → 0.22 over 0.6 s — 05a); the
            // slow-motion re-arms. The engine's flash (120) is the bump's
            // visual (04a).
            self.sfx(SFX_BUMP);
            self.music.bump();
            self.bump_remaining = BUMP_DURATION_S;
        }

        // -- 6. The journey (real time — the bump slow-motion does not
        //      delay the stages) ------------------------------------------
        if let Some(stage) = self.journey.advance(dt) {
            self.stage_up(stage);
        }

        // -- 7. The beat palette (EVERY frame — the 04b P2: `beat_hue` is
        //      never cleared, so the app keeps feeding it) ----------------
        let beat = self.music.tick(dt);
        self.composed.game.apply_music_beat(
            beat.beat_phase,
            beat.bar_phase,
            beat.energy,
            self.journey.base_hue(),
        );

        FrameActions {
            close,
            toggle_fullscreen,
        }
    }

    /// One rendered frame (plan loop step 8): the C++ `HexagonGame::draw`
    /// bridge (the `DrawOptions` are all false — zero text) + the two
    /// corner icons in screen space (rectangles / triangles — no fonts).
    pub fn draw(&mut self, target: &mut dyn DrawTarget, width: u32, height: u32) {
        let Some(app) = &mut self.app_draw else {
            return; // a window-less engine: the bridge's guard no-ops the scene
        };
        bridge_draw(&self.composed.game, app, target);
        // The icons (screen space — the bridge's final view is the
        // scene's; the 1:1 view, the `GlDrawTarget`'s initial view's shape).
        target.set_view(View::new(
            Vec2::new(width as f32 / 2.0, height as f32 / 2.0),
            Vec2::new(width as f32, height as f32),
        ));
        let w = width as f32;
        self.draw_icon(target, Icon::Reset, w);
        self.draw_icon(target, Icon::Fullscreen, w);
    }

    /// The manual reset (the plan's "Manual reset"): restart the CURRENT
    /// stage — `new_game` with the current level + multiplier (no stage
    /// change), reset the stage clock, play `restart.ogg`.
    fn reset_stage(&mut self) {
        self.composed.game.new_game(
            PACK,
            &self.journey.level_id(PACK),
            false,
            self.journey.difficulty(),
        );
        self.composed.game.start();
        self.journey.reset_elapsed();
        self.bump_remaining = 0.0;
        self.sfx(SFX_RESET);
    }

    /// The stage-up (the journey's transition work): every 2 stages the
    /// level changes (the `meadow` / `river` / `stars` cycle) → a fresh
    /// `new_game` with the new level + the stage's multiplier; otherwise
    /// the live `set_difficulty_mult` (the same level). The music stage
    /// updates (05a's layer unlocks: 0 pad+drone, 2 pluck, 5 bell, 8 bass,
    /// 11 shimmer, 14 heartbeat, 17 supersaw), the burst flashes, the
    /// stage-up SFX plays.
    fn stage_up(&mut self, stage: u32) {
        if stage.is_multiple_of(2) {
            self.composed.game.new_game(
                PACK,
                &self.journey.level_id(PACK),
                false,
                self.journey.difficulty(),
            );
            // The kid's run never waits for input to start the new level.
            self.composed.game.start();
        } else {
            self.composed
                .game
                .set_difficulty_mult(self.journey.difficulty());
        }
        self.music.set_stage(stage);
        // The stage-up burst: the engine's flash (no reusable wall-burst
        // particle spawner exists — 04a's soft-burst outcome).
        {
            let mut st = self.composed.game.state_handle().lock();
            st.status.flash_effect = st.status.flash_effect.max(STAGE_UP_FLASH);
        }
        self.sfx(SFX_STAGE_UP);
    }

    /// The helping hand's steering (stage 0): the side that closes the
    /// angle toward `nearest_gap_angle` (04a's kid API); `None` = coast
    /// (no gap yet, or within the dead zone of the gap centre).
    fn helping_direction(&self) -> Option<i32> {
        let game = &self.composed.game;
        let target = game.nearest_gap_angle(game.get_player_angle())?;
        let diff = (target - game.get_player_angle()).rem_euclid(TAU);
        let diff = if diff > PI { diff - TAU } else { diff };
        if diff.abs() < HELP_DEAD_ZONE_RAD {
            None
        } else if diff > 0.0 {
            Some(1) // clockwise (the angle rises)
        } else {
            Some(-1) // counter-clockwise
        }
    }

    /// The SFX (the engine seam's null-audio half no-ops the headless
    /// tests' `None`).
    fn sfx(&self, id: &str) {
        if let Some(a) = &self.audio {
            a.play_sound_override(id);
        }
    }

    /// One icon (the plan's "the two corner icons … no fonts"): a ~64×64
    /// translucent backing square + a shape glyph (the reset's restart
    /// triangle / the fullscreen's expand box — `DrawTarget` rectangles
    /// + a triangle, no fonts).
    fn draw_icon(&self, target: &mut dyn DrawTarget, icon: Icon, width: f32) {
        let origin = icon_position(icon, width);
        let mut back = RectangleShape::default();
        back.set_size(Vec2::new(ICON_SIZE, ICON_SIZE));
        back.set_position(origin);
        back.set_fill_color(Color::new(0, 0, 0, 90));
        back.set_outline_thickness(0.0);
        target.draw_rectangle(&back, RenderStates::default());

        let glyph = Color::new(255, 255, 255, 210);
        match icon {
            Icon::Reset => {
                // The restart triangle (a right-pointing chevron).
                let v =
                    |dx: f32, dy: f32| Vertex::new(Vec2::new(origin.x + dx, origin.y + dy), glyph);
                target.draw_vertices(
                    &[v(20.0, 16.0), v(20.0, 48.0), v(46.0, 32.0)],
                    RenderStates::default(),
                );
            }
            Icon::Fullscreen => {
                // The expand box: four thin bars centred on the icon.
                let mut bar = RectangleShape::default();
                bar.set_fill_color(glyph);
                bar.set_outline_thickness(0.0);
                let t = 5.0; // the bar thickness
                let s = 12.0; // half the box
                let cx = origin.x + ICON_SIZE / 2.0;
                let cy = origin.y + ICON_SIZE / 2.0;
                bar.set_size(Vec2::new(2.0 * s, t));
                bar.set_position(Vec2::new(cx - s, cy - s));
                target.draw_rectangle(&bar, RenderStates::default());
                bar.set_position(Vec2::new(cx - s, cy + s - t));
                target.draw_rectangle(&bar, RenderStates::default());
                bar.set_size(Vec2::new(t, 2.0 * s));
                bar.set_position(Vec2::new(cx - s, cy - s + t));
                target.draw_rectangle(&bar, RenderStates::default());
                bar.set_position(Vec2::new(cx + s - t, cy - s + t));
                target.draw_rectangle(&bar, RenderStates::default());
            }
        }
    }
}

/// The shared windowed run (plan 002 issue 08a's target-agnostic half):
/// the caller builds the PLATFORM window (the desktop's winit window —
/// [`run_windowed`]'s `Window::create` + icon; the web's emscripten
/// canvas window — [`tiny_web`] over the `th-render` emscripten layer
/// that owns the canvas / GL / input; the Android activity's window —
/// 08b) and hands it here with the composition's parts. This drives
/// [`TinyRuntime::frame`] + [`TinyRuntime::draw`] per frame until the
/// window closes.
///
/// No desktop-only calls in the loop (08a): no filesystem root
/// resolution, no desktop-mode probe, no xvfb — those live in the
/// platform entries (the desktop bin's `resolve_game_root` + the
/// `desktop_mode` probe child; the web's fixed `/RELEASE` MEMFS root).
/// The one platform branch is the fullscreen action: the desktop's
/// winit toggle; the web's browser Fullscreen API (the in-canvas icon
/// tap carries the user activation a `requestFullscreen` needs — the
/// `#fs` button's click sink runs the same apply, both read the
/// browser's actual state, so the two controls cannot disagree).
///
/// `screenshot_after`: the dev half of the plan's evidence
/// (`--tiny-shot=SECONDS` — the desktop bin only) — after that many
/// seconds the frame's `mustTakeScreenshot` arms (the bridge's
/// `HGGraphics.cpp:305-313` save — `screenshot.png` in the CWD), and the
/// window closes once the bridge has consumed it.
pub fn run(
    window: th_render::window::Window,
    assets: Arc<th_assets::Assets>,
    root: PathBuf,
    config: Config,
    audio: Arc<TinyAudio>,
    music: MusicDirector,
    screenshot_after: Option<f32>,
) {
    let mut runtime = TinyRuntime::new(assets.clone(), root, true, config, Some(audio), music);

    let shot = screenshot_after;
    let mut shot_armed = false;
    let mut elapsed = 0.0_f32;
    let mut last = std::time::Instant::now();
    window.run(move |w| {
        // The frame's dt (the plan's "accumulated from the frame dt"; the
        // `MAX_FRAME_DT` clamp lives in `frame`).
        let now = std::time::Instant::now();
        let dt = now.duration_since(last).as_secs_f32();
        last = now;
        elapsed += dt.min(MAX_FRAME_DT);

        // 0. The web's first-gesture audio unlock (08a): the
        // `resumeAudio()` export's flag (set on the first user-activated
        // gesture — the export's `navigator.userActivation` check). The
        // no-activation fallback guard stays: an input that grants no
        // activation keeps the fallback clock running (the 041
        // measurement); the 10-19 Chrome 148 probe run DID grant
        // activation on the synthetic `keydown`, so it measured
        // `audio=live`. Arm the live synth engine now (idempotent — a
        // no-op while already live; on failure the fallback clock keeps
        // running, never a panic). The `th-audio` SFX device (emscripten's
        // OpenAL) unlocks on the SAME first gesture through its own
        // listeners (emscripten's `autoResumeAudioContext` + the
        // module's probe listener) — there is no `th-audio` call to make.
        #[cfg(target_os = "emscripten")]
        if web::take_unlock_request() {
            runtime.music.start_on_gesture();
        }

        // -- 1. The frame's input (the window's drained state) -----------
        let input = *w.get_input_state();
        let positions = w.finger_down_positions();
        let mut fingers = [(0u32, Vec2::new(0.0, 0.0)); FINGER_COUNT];
        let mut finger_count = 0;
        for (i, p) in positions.iter().take(FINGER_COUNT).enumerate() {
            fingers[finger_count] = (i as u32, *p);
            finger_count += 1;
        }
        let ev = FrameEvents {
            input,
            fingers,
            finger_count,
            mouse_left: input.is_button_pressed(Button::LEFT),
            mouse_pos: w.mouse_pixel(),
            width: w.get_width() as f32,
            height: w.get_height() as f32,
        };

        // -- 2. The frame (engine + journey + beat + SFX) -----------------
        let actions = runtime.frame(dt, &ev);
        if actions.toggle_fullscreen {
            // The desktop's winit toggle (the plan's icon action).
            #[cfg(not(target_os = "emscripten"))]
            w.set_fullscreen(!w.get_fullscreen());
            // The web's browser Fullscreen API apply (the 08a icon tap's
            // user activation; the `#fs` button's click sink runs the SAME
            // apply — both read the browser's actual state).
            #[cfg(target_os = "emscripten")]
            crate::web_fullscreen::toggle_browser_fullscreen();
        }
        if actions.close {
            w.close(); // the plan's "Esc: close the window"
        }

        // -- 3. The frame's draw (the bridge + the two icons) + the swap --
        if let Some(gl) = w.gl() {
            let mut target = GlDrawTarget::new(gl)
                .with_provider(Arc::new(AssetsPackShaderProvider::new(assets.clone())));
            if let Some(after) = shot {
                if !shot_armed && elapsed >= after {
                    runtime.game().set_must_take_screenshot(true);
                    shot_armed = true;
                }
            }
            runtime.draw(&mut target, w.get_width(), w.get_height());
            target.present(); // the C++ `renderWindow.display()` (GameWindow.hpp:143)
            if shot_armed && !runtime.game().must_take_screenshot() {
                w.close(); // the screenshot saved (the bridge consumed the flag)
            }
        }
        // The C++ `window->getFPS()` half (the `onRenderStage` frametime's
        // divisor — the window's live FPS).
        runtime.fps(w.get_fps());

        // The web's status statics (the `status()` probe's per-frame feed
        // — the 03c status hook, now the real game's state).
        #[cfg(target_os = "emscripten")]
        web::record_frame(
            runtime.journey.stage,
            runtime.game().get_status().started,
            runtime.game().get_player_angle(),
            runtime.music.is_live(),
            runtime.help_disabled(),
        );
    });
}

/// The desktop `tinyhexagon` bin's windowed run (08a's thin caller of the
/// shared [`run`]): the 1024×768 window (the plan's
/// `Window::create(1024, 768, "tinyhexagon", 4)`) + the window icon — the
/// desktop-only pieces that stay out of the shared loop (the plan's
/// `Window::create(1024, 768, "tinyhexagon", 4)`).
pub fn run_windowed(
    assets: Arc<th_assets::Assets>,
    root: PathBuf,
    config: Config,
    audio: Arc<TinyAudio>,
    music: MusicDirector,
    screenshot_after: Option<f32>,
) {
    let mut window = th_render::window::Window::create(1024, 768, "tinyhexagon", 4);
    // The window icon (`Assets/icon.png` — the C++ `main.cpp:312-328`
    // half; the missing file is the C++'s "Failed to load icon image").
    // The shared loop does no filesystem path work (08a) — the desktop's
    // icon stays on the desktop's side.
    if let Ok(icon) = th_render::Image::load_png(&root.join("Assets/icon.png")) {
        window.set_icon(&icon);
    }
    run(window, assets, root, config, audio, music, screenshot_after);
}

// ---------------------------------------------------------------------
// The web (emscripten) surface (plan 002 issue 08a)
//
// `th-web`'s entry over the shared [`run`]: the boot (the 07 composition
// over the preloaded `_RELEASE` MEMFS tree + the canvas window + the
// `web_fullscreen` sinks), the `status()` probe's per-frame feed, and the
// first-gesture `resumeAudio()` hand-off (the `!Send`/`!Sync`
// `MusicDirector` cannot sit in a `static`, so the hand-off is a flag the
// frame loop consumes — one display tick at most after the gesture, well
// inside the browser's ~5 s transient-activation window).
// ---------------------------------------------------------------------

#[cfg(target_os = "emscripten")]
mod web {
    use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

    /// The preloaded `_RELEASE` tree's MEMFS mount (the root
    /// `.cargo/config.toml`'s `--preload-file=_RELEASE@/RELEASE` +
    /// `-sFORCE_FILESYSTEM=1`) — the web boot's content root (`config.json`,
    /// `Packs/`, `Assets/`, the fonts), read through emscripten's libc.
    /// (The desktop bin's `resolve_game_root` walk has no web twin: the
    /// preload point IS the root.)
    pub(super) const ROOT: &str = "/RELEASE";

    /// The boot state: `true` once [`super::tiny_web`] has composed and
    /// scheduled the loop; a failed boot records its reason in
    /// [`BOOT_ERROR`] instead (the `status()` line reports both — the
    /// probe's wait fails fast on the `error=` half).
    static BOOTED: AtomicBool = AtomicBool::new(false);
    static BOOT_ERROR: std::sync::OnceLock<String> = std::sync::OnceLock::new();

    /// The rAF loop's per-frame feed (the `status()` line's data — the
    /// 03c status hook, now the real game's state).
    static TICKS: AtomicU64 = AtomicU64::new(0);
    static STAGE: AtomicU32 = AtomicU32::new(0);
    static STARTED: AtomicBool = AtomicBool::new(false);
    static AUDIO_LIVE: AtomicBool = AtomicBool::new(false);
    static HELP_OFF: AtomicBool = AtomicBool::new(false);
    static ANGLE: AtomicU32 = AtomicU32::new(0.0_f32.to_bits());

    /// The first-gesture audio unlock's request flag (the `resumeAudio()`
    /// export's write; the frame loop's consume — the flag-not-handle
    /// rationale in the module docs: the `MusicDirector` is
    /// `!Send`/`!Sync`, so it cannot sit in a `static`).
    static UNLOCK_REQUESTED: AtomicBool = AtomicBool::new(false);

    pub(super) fn mark_boot_failed(reason: String) {
        let _ = BOOT_ERROR.set(reason);
    }

    pub(super) fn mark_booted() {
        BOOTED.store(true, Ordering::SeqCst);
    }

    pub(super) fn record_frame(
        stage: u32,
        started: bool,
        angle: f32,
        audio_live: bool,
        help_off: bool,
    ) {
        TICKS.fetch_add(1, Ordering::Relaxed);
        STAGE.store(stage, Ordering::Relaxed);
        STARTED.store(started, Ordering::Relaxed);
        AUDIO_LIVE.store(audio_live, Ordering::Relaxed);
        HELP_OFF.store(help_off, Ordering::Relaxed);
        ANGLE.store(angle.to_bits(), Ordering::Relaxed);
    }

    pub(super) fn request_unlock() {
        UNLOCK_REQUESTED.store(true, Ordering::Relaxed);
    }

    pub(super) fn take_unlock_request() -> bool {
        UNLOCK_REQUESTED.swap(false, Ordering::Relaxed)
    }

    pub(super) fn status_line() -> String {
        if let Some(err) = BOOT_ERROR.get() {
            return format!("tinyhexagon web: booted=false;error={err}");
        }
        if !BOOTED.load(Ordering::SeqCst) {
            return "tinyhexagon web: booted=false".to_string();
        }
        format!(
            "tinyhexagon web: booted=true;ticks={};stage={};started={};angle={:.4};audio={};help={}",
            TICKS.load(Ordering::Relaxed),
            STAGE.load(Ordering::Relaxed),
            STARTED.load(Ordering::Relaxed),
            f32::from_bits(ANGLE.load(Ordering::Relaxed)),
            if AUDIO_LIVE.load(Ordering::Relaxed) { "live" } else { "fallback" },
            if HELP_OFF.load(Ordering::Relaxed) { "off" } else { "auto" },
        )
    }
}

/// The web boot (the shared tiny entry's emscripten half — plan 002
/// issue 08a): the kid composition over the preloaded `_RELEASE` MEMFS
/// tree — the 07 composition verbatim (`Config::default()` →
/// `load_and_reapply` over the fixed MEMFS root → the kid overrides;
/// in-memory, nothing written back), the real `th-audio` SFX at the
/// gentle volume, the `th-synth` soundtrack on the fallback clock (the
/// autoplay policy — the live engine arms on the first-gesture
/// unlock), the emscripten canvas window (the `th-render` layer's
/// `Window::create` — the canvas / GL / input), the `web_fullscreen`
/// sinks, and the shared [`run`] loop. Returns after scheduling the
/// first rAF frame (the browser owns the loop — the loop's self-owning
/// closure keeps the engine + the heap-pinned Lua context alive across
/// the caller's `main` returning).
#[cfg(target_os = "emscripten")]
pub fn tiny_web() {
    let root = PathBuf::from(web::ROOT);

    // The assets (the C++ `HGAssets{nullptr, headless}` — the kid's web
    // run is windowed (the canvas IS the window), so headless = false;
    // the `levels_only` half stays false — the styles load with their
    // levels, through emscripten's libc over MEMFS).
    let assets = match th_assets::Assets::new(&root, false, false) {
        Ok(a) => Arc::new(a),
        Err(e) => {
            web::mark_boot_failed(format!("asset loading failed: {}", e.reason));
            th_game::log_sink::emit_log(
                "web",
                &format!("[th-web] asset loading failed: {}", e.reason),
            );
            return;
        }
    };

    // The kid's config: `Config::default()` → `load_and_reapply` (the C++
    // `mainClient`'s load + reapply ONCE — `Lazy` is safe on emscripten:
    // the `desktop_mode` probe never spawns there (the zero-mode arm),
    // and the read goes through MEMFS) → the kid overrides (they win
    // over the file's values). Never written back.
    let mut config = th_config::Config::default();
    if let Err(e) = crate::lifecycle::load_and_reapply(
        &mut config,
        &root,
        &[],
        crate::lifecycle::DesktopModeSource::Lazy,
    ) {
        web::mark_boot_failed(format!("config load failed: {e}"));
        th_game::log_sink::emit_log("web", &format!("[th-web] config load failed: {e}"));
        return;
    }
    let config = kid_config(config);

    // The SFX at the gentle volume (the plan's "set a gentle sound
    // volume" — the SFML 0..=100 units; the synth's master gain stays
    // independent). On the web the device is emscripten's OpenAL (the
    // `th-render` `AudioDevice` seam's web arm) — it unlocks itself on
    // the first gesture (its probe listener + emscripten's
    // `autoResumeAudioContext`).
    let audio = Arc::new(th_audio::Audio::new(assets.clone()));
    audio.set_sound_volume(60.0);
    // The soundtrack (05a: the fallback clock on emscripten at boot —
    // the autoplay policy; `start_on_gesture` on the first user-activated
    // gesture, the `resumeAudio()` export's hand-off).
    let music = th_synth::MusicDirector::start();

    let mut window = th_render::window::Window::create(1024, 768, "tinyhexagon", 4);
    // The browser Fullscreen API's sinks (the 02a salvage's first caller
    // — 08a): the `#fs` button's click + the `fullscreenchange` re-sync.
    // The config they write is bookkeeping (the tiny runtime has no menu /
    // config UI — the browser state is the fullscreen source of truth the
    // sinks read, the `toggle_browser_fullscreen`'s doc).
    let shared_config = Arc::new(RwLock::new(config.clone()));
    crate::web_fullscreen::install_sinks(&mut window, &shared_config);

    web::mark_booted();
    run(window, assets, root, config, audio, music, None);
}

/// The web's `status()` probe line (the 03c hook, now the real game's
/// state — `th-web`'s `#[wasm_bindgen] status()` forwards to this): the
/// boot state (+ the failure's reason) / the rAF tick count / the journey
/// stage / the engine's started flag / the player angle / the synth's
/// live-vs-fallback state / the helping-hand state.
///
/// # Safety-free by construction
///
/// All the statics are plain atomics (the `!Send`/`!Sync`
/// [`MusicDirector`] never enters a `static` — only its `is_live()`
/// reading does), and the writes (the rAF loop) + the reads (the export)
/// are the web's single main thread.
#[cfg(target_os = "emscripten")]
#[must_use]
pub fn web_status() -> String {
    web::status_line()
}

/// The first-gesture audio unlock's export half (plan 002 issue 08a):
/// `th-web`'s `resumeAudio()` export calls this on the page's first
/// `pointerdown` / `keydown` (and the `visibilitychange` → visible
/// re-fire). It is idempotent (the flag is consumed once per set; the
/// engine start itself is a no-op while live).
///
/// The autoplay-policy guard (the 08 contract's risk + the 05b hazard):
/// the live engine's `AudioContext.resume()` must run INSIDE a user
/// activation, and cpal 0.15.3's emscripten host PANICS on the
/// rejected resume promise (the `spawn_local`'s `Err` arm). So the
/// live start is requested only when the browser reports user
/// activation (`is_active` — the transient or persistent state at the
/// call instant, inside the gesture handler); otherwise the fallback
/// clock keeps running until a permitted gesture (the re-fire's path).
/// Whether a synthetic input grants activation is the BROWSER's: the
/// 041 measurement saw none (fallback clock there), while the 10-19
/// Chrome 148 probe run's synthetic `keydown` DID grant it — that run
/// measured `audio=live`. Either outcome is a clean result; this guard
/// is the no-activation half's safety.
#[cfg(target_os = "emscripten")]
pub fn web_request_audio_unlock() {
    let activated = web_sys::window()
        // web-sys 0.3.106: `Window::navigator()` returns a `Navigator`
        // (never null) — a `Some` arm, not an `and_then` Option arm.
        .map(|w| w.navigator())
        .is_some_and(|n| n.user_activation().is_active());
    if activated {
        web::request_unlock();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The kid config's overrides land (no music / 3D off / 1024×768;
    /// the beat pulse stays on — the default).
    #[test]
    fn the_kid_config_applies_the_kid_overrides() {
        let config = kid_config(Config::default());
        assert!(
            config.get_no_music(),
            "no OGG level music (the synth owns the soundtrack)"
        );
        assert!(!config.get_3d(), "3D off");
        assert_eq!(config.get_windowed_width(), 1024);
        assert_eq!(config.get_windowed_height(), 768);
        assert!(
            config.get_beat_pulse(),
            "the beat pulse stays on (the default)"
        );
    }
}
