# Issue 08a fix 3 — cpal destination connect direction

**Worktree:** `.agents/worktrees/slot-2` (base 08a code; host gate 567).
Main/slot-1 off-limits. **No commit.**

## Confirmed failure

The rebuilt wasm + probe now pass the sample-type path; the next panic is at
`vendor/cpal-0.15.3/src/host/emscripten/mod.rs:335`:

```
Could not connect the audio node to the destination: JsValue(IndexSizeError:
Failed to execute 'connect' on 'AudioNode': output index (0) exceeds number
of outputs (0))
```

The code does `context.destination().connect_with_audio_node(&node)` —
wrong direction: `AudioDestinationNode` has **0 outputs**. Connect the
source node TO the destination instead:

```rust
node.connect_with_audio_node(&context.destination())
    .expect("Could not connect the audio node to the destination");
```

(keep `node.start()` after it).

## Iterate until the probe is clean

Rebuild + probe:

```sh
podman run --rm -v "$PWD":/work \
  -v /home/gary/.rustup:/home/gary/.rustup \
  -v /home/gary/.cargo:/home/gary/.cargo \
  -e RUSTUP_HOME=/home/gary/.rustup -e CARGO_HOME=/home/gary/.cargo \
  -w /work docker.io/emscripten/emsdk:latest \
  bash -c 'export PATH=/home/gary/.cargo/bin:$PATH; bash /work/tools/package-web.sh'
python3 -m http.server 8086 --bind 127.0.0.1 --directory target/package/web/_WEB &
OH_CHROME=/home/gary/.agent-browser/browsers/chrome-148.0.7778.97/chrome \
  node tools/web-stub-probe.mjs http://127.0.0.1:8086/ \
  --shot=.agents/evidence/08a/08a-web-probe.png
```

If more browser-API panics surface in the same emscripten host (this cpal
0.15.3 backend is old), fix them minimally and rebuild; do not disable the
synth or drop audio. The goal is the probe's 19/19 with a clean console,
`audio=live` after the gesture, and the screenshot.

Then: host gate `bash tools/gate.sh` (567), update
`issue-008a-report.md` + FEED-FORWARD §08a with the final evidence, and
stop. No Android/packaging, no commit.
