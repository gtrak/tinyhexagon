# tinyhexagon

A kid-friendly fork of the Open Hexagon Rust port: no menus, no reading, no
fail; a slow 10-minute arc and generated music. The adult port is retired;
this repository is mid-transition to the tiny runtime.

Active plan: [.agents/plans/002-tinyhexagon/PLAN.md](.agents/plans/002-tinyhexagon/PLAN.md)

Build the current tree: `cargo build --workspace`. `cargo run` now runs
the `tinyhexagon` stub binary (the `th-app` bin via the workspace
default-members — it prints a one-line "lands in Issue 07" message and
exits 0; the real tiny runtime replaces it in Issue 07).
