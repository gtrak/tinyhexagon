pub mod audio;
pub mod math;
pub mod persistence;
// 05b feature gate (see Cargo.toml provenance block): the TUI and the
// master-output recorder pull crossterm/mio + flacenc/vorbis_rs/hound,
// which do not build on wasm32-unknown-emscripten. `th-synth` uses only
// the `audio` + `math` modules and builds with the default features.
#[cfg(feature = "recording")]
pub mod recording;
#[cfg(feature = "tui")]
pub mod tui;
