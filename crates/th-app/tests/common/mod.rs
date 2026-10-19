//! Test-side composition for the draw-bridge tests (issue 15 step (a)).
//!
//! The composition itself (`LuaScriptHost` / `RealAssetsApi` / `Composed` /
//! the two-phase build) lives in `th_app::compose` — promoted from this
//! file by issue 15 step (b) so the binary and the step (a) tests share
//! ONE composition (the spec's dedup; the two-phase build and the
//! `SharedLua` drop-order contract are documented there). What remains
//! test-local: the `_RELEASE` root idiom, the C++ `initText` font pair, and
//! the recording `DrawTarget` for the stage-sequence assertions.

use std::path::{Path, PathBuf};
use std::sync::Arc;

pub use th_app::compose::{compose_with, Composed};

/// The step-(a) test composition: the shared `th_app::compose` build.
/// `allow(dead_code)`: not every test binary composes an engine (each
/// compiles this module separately).
#[allow(dead_code)]
#[must_use]
pub fn compose(assets: Arc<th_assets::Assets>, root: PathBuf, window: bool) -> Composed {
    compose_with(assets, root, window)
}

/// The `_RELEASE` root (the process-CWD equivalent; the same idiom as
/// `th-assets/tests/release_tree.rs`).
#[must_use]
pub fn release_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../_RELEASE")
        .to_path_buf()
}

/// The C++ `initText` fonts (`HexagonGame.cpp:289-290`: the assets
/// registry's `OpenSquare-Regular.ttf` / `OpenSquare-Bold.ttf`).
/// `allow(dead_code)`: only the bridge test binary loads fonts.
#[allow(dead_code)]
#[must_use]
pub fn test_fonts() -> (th_render::font::Font, th_render::font::Font) {
    let root = release_dir().join("Assets");
    (
        th_render::font::Font::load(&root.join("OpenSquare-Regular.ttf"))
            .expect("OpenSquare-Regular.ttf"),
        th_render::font::Font::load(&root.join("OpenSquare-Bold.ttf"))
            .expect("OpenSquare-Bold.ttf"),
    )
}

// ---------------------------------------------------------------------
// A recording DrawTarget for the stage-sequence assertions — the same
// event model as `th-render`'s private `Recording` target
// (`graphics.rs:1561`, not importable — private to that crate's test
// module).
// ---------------------------------------------------------------------

use th_render::draw::RenderStates;
use th_render::graphics::DrawTarget;
use th_render::view::View;

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)] // only the bridge test binary records events
pub enum DrawEvent {
    Clear,
    SetView(View),
    Vertices { count: usize, states: RenderStates },
    Sprite,
    Text { states: RenderStates },
    Rectangle { states: RenderStates },
    Screenshot(String),
}

#[derive(Default)]
#[allow(dead_code)] // only the bridge test binary records events
pub struct Recording {
    pub events: Vec<DrawEvent>,
}

impl DrawTarget for Recording {
    fn clear_black(&mut self) {
        self.events.push(DrawEvent::Clear);
    }
    fn set_view(&mut self, view: View) {
        self.events.push(DrawEvent::SetView(view));
    }
    fn draw_vertices(&mut self, vertices: &[th_render::draw::Vertex], states: RenderStates) {
        self.events.push(DrawEvent::Vertices {
            count: vertices.len(),
            states,
        });
    }
    fn draw_sprite(&mut self, _sprite: &th_render::texture::Sprite) {
        self.events.push(DrawEvent::Sprite);
    }
    fn draw_text(&mut self, _text: &th_render::font::Text, states: RenderStates) {
        self.events.push(DrawEvent::Text { states });
    }
    fn draw_rectangle(&mut self, _rect: &th_render::types::RectangleShape, states: RenderStates) {
        self.events.push(DrawEvent::Rectangle { states });
    }
    fn save_screenshot(&mut self, path: &str) {
        self.events.push(DrawEvent::Screenshot(path.to_string()));
    }
}
