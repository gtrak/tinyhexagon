# Issue 08a fix 2 — wasm-bindgen descriptor panic

**Worktree:** `.agents/worktrees/slot-2` (the 08a code is in; host gate 567).
Main/slot-1 off-limits. **No commit.**

## Root cause (confirmed by inspection)

`vendor/cpal-0.15.3/src/lib.rs`:

```rust
#[cfg(target_os = "emscripten")]
impl wasm_bindgen::describe::WasmDescribe for BufferSize {
    fn describe() {}          // <-- EMPTY
}
```

`StreamConfig` is `#[wasm_bindgen]` with `pub buffer_size: BufferSize`, so
wasm-bindgen generates the `__wbg_get_streamconfig_buffer_size` getter
import. Its `__wbindgen_describe_…` body calls `BufferSize::describe()`,
which emits nothing; the 0.2.129 CLI's `Descriptor::decode` gets an empty
slice → `descriptor.rs:324` "len is 0". The `IntoWasmAbi`/`FromWasmAbi`
for `BufferSize` already use `Option<u32>`, so the descriptor must match.

## Fix

```rust
#[cfg(target_os = "emscripten")]
impl wasm_bindgen::describe::WasmDescribe for BufferSize {
    fn describe() {
        <Option<u32> as wasm_bindgen::describe::WasmDescribe>::describe();
    }
}
```

Then rebuild and fix any further descriptor panics the same way (only for
types whose describe is empty/mismatched to their `*WasmAbi::Abi`; the
other cpal `#[wasm_bindgen]` types are `SampleRate`/`Stream`/`Data`/
`OutputCallbackInfo` — if one panics, give it the descriptor of its ABI
type, not an empty body). Do **not** touch the host/ALSA paths.

## Verify

1. emsdk build (the pinned podman recipe, `tools/build-wasm-emsdk.sh`);
   iterate until the `-sWASM_BINDGEN` step succeeds.
2. Stage `_WEB` via `tools/package-web.sh`; serve it; run
   `node tools/web-stub-probe.mjs --shot=<path>` (real-game assertions).
   Save the screenshot under `.agents/evidence/08a/`.
3. Host gate (`bash tools/gate.sh`) must stay 567; the vendor change is
   emscripten-only.
4. Update `issue-008a-report.md` and FEED-FORWARD §08a with the root cause,
   build/probe evidence and screenshot.

## Constraints

- The vendor patch is the truth; no wasm-bindgen downgrade, no music drop.
- No Android/packaging, no commit.
