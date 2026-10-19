# Issue 08b — Android tinyhexagon entry

**Worktree:** `.agents/worktrees/slot-1`, base **the 08a commit** (the
orchestrator will tell you the SHA). Main/slot-2 off-limits. **No commit.**

**Read:** `.agents/plans/002-tinyhexagon/08-android-web-packaging.md`
(Android half), `.agents/tasks/plan-002/issue-007-report.md`,
`issue-008a-report.md`, FEED-FORWARD §07/§08a, `crates/th-android/`,
`crates/th-app/src/{tiny,lifecycle,desktop_mode,touch}.rs`, and the retired
adult Android glue from git history (`git show <02a>^:crates/th-app/src/...`).

## Goal

The same tinyhexagon loop on Android arm64-v8a: touch halves + corner
icons, synth/SFX with fallback, no text, no profiles/config/replays.

## Steps

1. **Shared Android window.** In `th_app::tiny`, add the Android entry
   (`tiny_android` / the shared `run` taking the Android `Window`), based on
   the 08a target-agnostic refactor. Use `EventLoopBuilderExtAndroid` +
   `th_render::Window::create_android` and the existing activity lifecycle.
   Keep the desktop/web paths untouched.
2. **`th-android` real entry.** Replace the 02a stub with `android_main`:
   - extract the packaged `_RELEASE` assets into the app's internal data
     dir (the verified glue), then boot the tiny runtime;
   - the no-SIGINT/Steam/Discord conditions;
   - `DesktopModeSource::Fixed(zero)` (no child probe on Android);
   - `th-synth` starts at boot (oboe/AAudio) and `th-audio` SFX; if a
     stream fails, degrade to the fallback clock/no SFX without panicking;
   - the 07 touch halves + 48–64 dp corner icons (reset/fullscreen via the
     Android window/activity), no IME/text;
   - no config/profile/replay writes.
3. **Packaging metadata.** `crates/th-android/Cargo.toml`: package
   `th-android`, cdylib `th_android`, `apk_name = "tinyhexagon"`, the
   aarch64-only target, SDK bounds, `_RELEASE` assets, fullscreen
   `NativeActivity` (keep the verified 02a metadata). Update `tools/android/`
   and `tools/package-android.sh` to `tinyhexagon.apk`. Remove the adult
   `open-hexagon` APK/asset paths. (Workflow/CI renames are 08c; do not
   edit `.github/workflows/*` beyond what Android packaging strictly needs.)
4. **Verify.** `bash tools/check-android.sh` green; build a debug APK via
   `tools/package-android.sh` / `cargo apk`; if an emulator is available,
   boot/screenshot; otherwise record the exact check/build evidence.

## Constraints

- Do not rewrite the engine; reuse the same `th-render`/bridge path.
- Never drop the synth/SFX or crash on missing audio devices.
- No persistence/text; every run starts at stage 0.
- Do not touch the web/pages/release workflow names (08c).

## Report

`.agents/tasks/plan-002/issue-008b-report.md`: the Android entry function,
asset extraction path, audio fallback, touch/icon DP, APK evidence, and any
device gap; FEED-FORWARD **§08b only**.
