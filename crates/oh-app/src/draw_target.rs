//! The reusable GL draw target (plan 002 issue 02a — extracted from the
//! retired `windowed` module, which carried the adult loop / menu
//! front-end with it): [`GlDrawTarget`] — the `DrawTarget` over a live
//! `oh_render::GlParts` context — plus the no-op [`NullDrawTarget`]
//! fallback and the 029n1 (K6) pack-shader provider seam
//! ([`AssetsPackShaderProvider`], the `PackShaderProvider` over the
//! composition's `Arc<oh_assets::Assets>`). The tiny runtime's windowed
//! wiring (Issues 07/08) builds a `GlDrawTarget` per frame over the
//! window's `GlParts` and drives [`oh_render::graphics::draw_frame`]
//! through it.

use std::sync::Arc;

use oh_math::ssvs::Vec2;
use oh_render::batching::GlBatchUploader;
use oh_render::draw::{BlendMode, RenderStates, Vertex};
use oh_render::font::Text;
use oh_render::graphics::DrawTarget;
use oh_render::pack_shaders::PackShaderProvider;
use oh_render::texture::Sprite;
use oh_render::types::RectangleShape;
use oh_render::view::View;
use oh_render::GlParts;

// ---------------------------------------------------------------------------
// 029n1 (K6) — the pack-shader provider seam + the shader-bound draw.
// ---------------------------------------------------------------------------

/// The `PackShaderProvider` over the composition's `Arc<Assets>` (K6):
/// the C++'s `assets.getShaderByShaderId(id)` payload (source + the
/// uniform store) as the `oh-render` GL layer's
/// [`oh_render::pack_shaders::PackShaderSnapshot`] — the `oh-assets`
/// → `oh-render` type conversion at the seam (the `oh-render`
/// `PackShaderProvider` trait's implementation; `oh-render` cannot
/// depend on `oh-assets`, the cycle would run through `oh-game`).
pub struct AssetsPackShaderProvider {
    assets: Arc<oh_assets::Assets>,
}

impl AssetsPackShaderProvider {
    #[must_use]
    pub fn new(assets: Arc<oh_assets::Assets>) -> Self {
        Self { assets }
    }
}

impl PackShaderProvider for AssetsPackShaderProvider {
    fn pack_shader_snapshot(
        &self,
        id: usize,
    ) -> Option<oh_render::pack_shaders::PackShaderSnapshot> {
        let snap = self.assets.shader_snapshot(id)?;
        Some(oh_render::pack_shaders::PackShaderSnapshot {
            source: snap.source,
            shader_type: match snap.shader_type {
                oh_assets::ShaderType::Fragment => oh_render::shader::ShaderType::Fragment,
                oh_assets::ShaderType::Vertex => oh_render::shader::ShaderType::Vertex,
                oh_assets::ShaderType::Geometry => oh_render::shader::ShaderType::Geometry,
            },
            uniforms: snap
                .uniforms
                .into_iter()
                .map(|(name, value)| {
                    let uniform = match value {
                        oh_assets::UniformValue::F(x) => oh_render::shader::Uniform::F(x),
                        oh_assets::UniformValue::FVec2(x, y) => {
                            oh_render::shader::Uniform::FVec2(x, y)
                        }
                        oh_assets::UniformValue::FVec3(x, y, z) => {
                            oh_render::shader::Uniform::FVec3(x, y, z)
                        }
                        oh_assets::UniformValue::FVec4(x, y, z, w) => {
                            oh_render::shader::Uniform::FVec4(x, y, z, w)
                        }
                        oh_assets::UniformValue::I(x) => oh_render::shader::Uniform::I(x),
                        oh_assets::UniformValue::IVec2(x, y) => {
                            oh_render::shader::Uniform::IVec2(x, y)
                        }
                        oh_assets::UniformValue::IVec3(x, y, z) => {
                            oh_render::shader::Uniform::IVec3(x, y, z)
                        }
                        oh_assets::UniformValue::IVec4(x, y, z, w) => {
                            oh_render::shader::Uniform::IVec4(x, y, z, w)
                        }
                    };
                    (name, uniform)
                })
                .collect(),
        })
    }
}

/// `DrawTarget` over a live [`GlParts`] context (the windowed loop's real
/// target). `clear_black` is a real `glClear`; `draw_vertices` submits the
/// scene batches through [`GlBatchUploader`] (or — 029n1 K6 — through the
/// bound stage's compiled pack fragment shader, the C++'s
/// `sf::RenderStates{shader}`'s execution half); the sprite / text /
/// rectangle passes are the issue-025c GL passes (the
/// [`oh_render::gl_passes::GlPasses`] textured-quad pipeline for sprite +
/// font-atlas text, the batch uploader's solid-colour stream for
/// rectangles — the module docs' GL-surface bullet), and the screenshot
/// save is the REAL GL readback → PNG (the C++ `GameWindow::
/// saveScreenshot`, GameWindow.hpp:167-174 — `GlParts::save_screenshot`:
/// `glReadPixels` bottom-up → row flip → `png`-crate encode; D-004 SFML
/// entry). `'a` is the window's borrow (the `GlParts` lives inside the
/// window for the whole loop).
///
/// # State hygiene (the C++ per-draw-call `RenderStates`)
///
/// Every draw call re-applies its blend mode first (SFML applies the
/// render states' blend BEFORE the draw — the C++'s `window->draw(x,
/// states)` per call; the game's stages all use the `Alpha` default,
/// `gl.enable(BLEND)` + `SRC_ALPHA, ONE_MINUS_SRC_ALPHA` — the pre-025c
/// vertex pass left GL blending at the driver default (off), which the
/// C++ never does: the correction is behaviour-neutral for the opaque
/// scene batches and enables the C++'s alpha-blended flash / 3D layers on
/// every platform). The program / texture / VAO / uniform re-application
/// is inside each pass (`oh_render::gl_passes`), per draw call — the C++
/// frame order (the caller's draw sequence) is untouched.
pub struct GlDrawTarget<'a> {
    parts: &'a GlParts,
    view: View,
    /// 029n1 (K6): the pack-shader source (`None` = no pack shaders can
    /// be resolved — the headless/fallback targets draw everything
    /// through the default program; the C++'s `window == nullptr` / the
    /// headless `Config::getDisableGameRendering` guards leave no draw
    /// path at all, so the absence of a provider there is unobservable).
    provider: Option<Arc<dyn PackShaderProvider>>,
}

impl<'a> GlDrawTarget<'a> {
    /// `parts` must outlive the target (the window does — the target is a
    /// per-frame scratch built and dropped in the frame loop's `tick`).
    /// The GL objects (the batch uploader, the texture cache, the textured
    /// program, the font atlas, the pack-shader cache) live in the
    /// `GlParts` — the target only borrows them (the SG5 state-hygiene pin:
    /// nothing is re-created per frame).
    #[must_use]
    pub fn new(parts: &'a GlParts) -> Self {
        Self {
            parts,
            view: View::new(
                Vec2::new(parts.width as f32 / 2.0, parts.height as f32 / 2.0),
                Vec2::new(parts.width as f32, parts.height as f32),
            ),
            provider: None,
        }
    }

    /// 029n1 (K6): the pack-shader source for the shader-bound
    /// `draw_vertices` (the C++'s `assets` behind the `getRenderStates`
    /// lambda's `getShaderByShaderId`).
    #[must_use]
    pub fn with_provider(mut self, provider: Arc<dyn PackShaderProvider>) -> Self {
        self.provider = Some(provider);
        self
    }

    /// 029n1 (K6): the shader-bound `draw_vertices` half — `true` when the
    /// pack program drew (the caller skips the default program), `false`
    /// when the draw falls through to it (the invalid id, the non-fragment
    /// registry entry, the sticky compile failure — each the C++'s
    /// default-render continuation).
    fn draw_vertices_with_pack_shader(
        &mut self,
        vertices: &[Vertex],
        id: usize,
        provider: &Arc<dyn PackShaderProvider>,
    ) -> bool {
        // The C++'s `getShaderByShaderId`'s `nullptr` guard
        // (`HGGraphics.cpp:93-94` — no default-state early return here:
        // the states already carry the bound id, the fallback is the
        // default program, not `RenderStates::Default`'s blend — the C++
        // `RenderStates{shader}`'s blend IS the default Alpha anyway).
        let Some(snapshot) = provider.pack_shader_snapshot(id) else {
            return false;
        };
        // The C++ binds whatever `getShaderByShaderId` hands back; a
        // vertex-only / geometry-only `sf::Shader` on a desktop GL
        // context would leave the fragment stage at the GL default.
        // The port's scene draws are the fragment-shader surface (the
        // levels' `shdr_setActiveFragmentShader` binds `.frag` files —
        // the only binding entry point, `LuaScripting.cpp:1549-1557`),
        // so a non-fragment id draws through the default program (the
        // equivalent visual result, the scope guard's geometry-shader
        // exclusion).
        if snapshot.shader_type != oh_render::shader::ShaderType::Fragment {
            return false;
        }
        // The stored-uniform upload needs the snapshot's values through
        // the (possibly first) compile — keep the owned copy beside the
        // borrow of the compiled program.
        let uniforms = snapshot.uniforms.clone();
        let mut cache = self.parts.pack_shaders();
        match cache.program(&self.parts.gl, id, &snapshot) {
            Some(program) => {
                program.upload_uniforms(&self.parts.gl, &uniforms);
                program.draw(&self.parts.gl, vertices, self.view);
                true
            }
            // The sticky compile failure (logged once by the cache) — the
            // default program draws (the C++'s load-failure continuation).
            None => false,
        }
    }

    /// 029n1 (K6): the shader-bound `draw_text` half — `true` when the
    /// pack program drew the glyph quads (the font atlas on unit 0,
    /// the C++'s font texture — the C++'s `window->draw(text,
    /// getRenderStates(RenderStage::Text))`, `HGGraphics.cpp:283`),
    /// `false` when the draw falls through to the default textured-quad
    /// pass (the invalid id, the non-fragment registry entry, the
    /// sticky compile failure, the empty text / no-atlas case — each
    /// the C++'s default-render continuation).
    fn draw_text_with_pack_shader(
        &mut self,
        text: &Text,
        id: usize,
        provider: &Arc<dyn PackShaderProvider>,
    ) -> bool {
        let Some(snapshot) = provider.pack_shader_snapshot(id) else {
            return false;
        };
        if snapshot.shader_type != oh_render::shader::ShaderType::Fragment {
            return false;
        }
        let uniforms = snapshot.uniforms.clone();
        let mut cache = self.parts.pack_shaders();
        let Some(program) = cache.program(&self.parts.gl, id, &snapshot) else {
            return false;
        };
        // The stored-uniform upload (the C++'s bind-time half — the
        // values the hook / `onUpdate` stored before this draw).
        program.upload_uniforms(&self.parts.gl, &uniforms);
        // The glyph quads + the font atlas on unit 0 (the `GlPasses`
        // owns the atlas — the two `GlParts` slots are independent
        // `RefCell`s, the single-threaded GL's borrow soundness).
        let mut passes = self.parts.passes();
        passes.draw_text_with_pack_program(&self.parts.gl, text, self.view, program)
    }
}

impl DrawTarget for GlDrawTarget<'_> {
    fn clear_black(&mut self) {
        // C++ `window->clear(sf::Color::Black)` (HGGraphics.cpp:83).
        self.parts.clear([0.0, 0.0, 0.0, 1.0]);
    }

    fn set_view(&mut self, view: View) {
        self.view = view;
    }

    fn draw_vertices(&mut self, vertices: &[Vertex], states: RenderStates) {
        if vertices.is_empty() {
            return;
        }
        // The C++ per-draw-call render states' blend (the `GlParts::
        // apply_blend` doc: the pre-025c pass left GL blending off — the
        // correction).
        self.parts.apply_blend(states.blend_mode);
        // 029n1 (K6): the bound per-stage pack fragment shader (the C++'s
        // `sf::RenderStates{assets.getShaderByShaderId(*fragmentShaderId)}`
        // — `HGGraphics.cpp:77`): draw through the compiled pack program
        // (the deferred load-time compile + the stored-uniform upload —
        // `oh_render::pack_shaders`'s module docs for the set-time vs
        // bind-time divergence), else fall through to the default
        // pass-through program (the C++'s load-failure path,
        // `Assets.cpp:889-892`'s "Failed to load shader" + the
        // default-render continuation).
        if let Some(id) = states.shader {
            if let Some(provider) = self.provider.clone() {
                if self.draw_vertices_with_pack_shader(vertices, id, &provider) {
                    return;
                }
            }
        }
        // The default pass-through program (the C++'s no-shader / the
        // load-failure draw — the batch uploads with the default
        // program). The uploader is the `GlParts`-shared one (the SG5
        // audit: the pre-025c per-frame re-creation leaked the GL
        // objects).
        let mut slot = self.parts.batch();
        let failed = slot.failed;
        match slot.uploader.as_mut() {
            Some(u) => u.draw(&self.parts.gl, vertices, self.view),
            None if !failed => match GlBatchUploader::new(&self.parts.gl) {
                Ok(mut u) => {
                    u.draw(&self.parts.gl, vertices, self.view);
                    slot.uploader = Some(u);
                }
                Err(e) => {
                    // Documented: the scene batch is not submitted (the GL
                    // program could not be created on this context).
                    eprintln!("[GlDrawTarget] GL batch uploader unavailable: {e}");
                    slot.failed = true;
                }
            },
            None => {}
        }
    }

    fn draw_sprite(&mut self, sprite: &Sprite) {
        // C++ `window->draw(sprite)` — the default render states (the Alpha
        // blend; the textured-quad pass, issue-025c SG2 — the sprite's
        // position/rotation/scale/origin + texture rect + the pool texture's
        // on-demand upload, `oh_render::gl_passes`).
        self.parts.apply_blend(BlendMode::Alpha);
        let mut passes = self.parts.passes();
        passes.draw_sprite(&self.parts.gl, sprite, self.view);
    }

    fn draw_text(&mut self, text: &Text, states: RenderStates) {
        // C++ `window->draw(text, states)` — the C++'s
        // `drawText(getRenderStates(RenderStage::Text))`
        // (`HGGraphics.cpp:283`): the font-atlas text pass
        // (issue-025c SG3) with the per-draw-call render states —
        // including 029n1 (K6)'s bound pack fragment shader (the
        // `text_shader.lua` level's `text.frag` — the hue-shifting
        // `font`-sampling text effect).
        self.parts.apply_blend(states.blend_mode);
        if let Some(id) = states.shader {
            if let Some(provider) = self.provider.clone() {
                if self.draw_text_with_pack_shader(text, id, &provider) {
                    return;
                }
            }
        }
        let mut passes = self.parts.passes();
        passes.draw_text(&self.parts.gl, text, self.view);
    }

    fn draw_rectangle(&mut self, rect: &RectangleShape, states: RenderStates) {
        // C++ `window->draw(rectangleShape, states)` — the solid-colour
        // pass (issue-025c SG4): the shape's fill quad + the outline ring's
        // four quads (`oh_render::gl_passes::rectangle_vertices`), through
        // the batch uploader's vertex-colour program.
        let vertices = oh_render::gl_passes::rectangle_vertices(rect);
        if vertices.is_empty() {
            return;
        }
        self.parts.apply_blend(states.blend_mode);
        let mut slot = self.parts.batch();
        let failed = slot.failed;
        match slot.uploader.as_mut() {
            Some(u) => u.draw(&self.parts.gl, &vertices, self.view),
            None if !failed => match GlBatchUploader::new(&self.parts.gl) {
                Ok(mut u) => {
                    u.draw(&self.parts.gl, &vertices, self.view);
                    slot.uploader = Some(u);
                }
                Err(e) => {
                    eprintln!("[GlDrawTarget] GL batch uploader unavailable: {e}");
                    slot.failed = true;
                }
            },
            None => {}
        }
    }

    fn save_screenshot(&mut self, path: &str) {
        // C++ `window->saveScreenshot("screenshot.png")` (HGGraphics.cpp:
        // 305-313, the `mustTakeScreenshot` gate; `saveScreenshot` itself
        // is GameWindow.hpp:167-174): read the default framebuffer —
        // AFTER the frame's draws (this runs inside `draw_frame`'s step 12,
        // before the swap) — flip bottom-up→top-down, write the PNG.
        // The C++ ignores the `saveToFile` result; mirror that (no return
        // on the `DrawTarget` surface).
        self.parts.save_screenshot(path);
    }

    fn present(&mut self) {
        // C++ `renderWindow.display()` (GameWindow.hpp:145 — the swap after
        // the frame's draws). Issue 25, on-device finding: this was never
        // wired — every GL platform presented nothing (the desktop live
        // check was MANUAL and never run; the emulator caught it).
        self.parts.present();
    }
}

/// A no-op target (headless windows / GL-less fallback): the C++'s
/// `window == nullptr` equivalent — the game bridge's
/// `window_present()` guard is what actually no-ops the game draw; this
/// exists so the loop's `engine.draw(target)` always has a target.
pub struct NullDrawTarget;

impl DrawTarget for NullDrawTarget {
    fn clear_black(&mut self) {}
    fn set_view(&mut self, _view: View) {}
    fn draw_vertices(&mut self, _vertices: &[Vertex], _states: RenderStates) {}
    fn draw_sprite(&mut self, _sprite: &Sprite) {}
    fn draw_text(&mut self, _text: &Text, _states: RenderStates) {}
    fn draw_rectangle(&mut self, _rect: &RectangleShape, _states: RenderStates) {}
    fn save_screenshot(&mut self, _path: &str) {}
}
