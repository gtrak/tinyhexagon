# Review gate (plan 002)

The forked `reviewer` receives the issue contract, the worker report, the
diff, the gate output and the current upstream high-water mark. It reviews
adversarially and returns exactly one final line.

## Checklist

1. **Contract**: every item in the issue file is implemented; nothing outside
   the scope fence was changed.
2. **Correctness**: the code does what the issue says; no silent fallbacks,
   no fake evidence, no stubs except where the issue explicitly allows them.
3. **Tests**: the new tests discriminate (would they fail on the pre-fix
   code?); no assertion was weakened; retired tests were deleted for the
   stated reason, not ignored.
4. **Regressions**: the surviving suite still passes; removed subsystems are
   not referenced; no accidental behaviour change in non-kid paths.
5. **Docs/feed-forward**: the issue report, `FEED-FORWARD.md`, and the
   affected later issue files reflect what changed. Upstream mapping/HWM is
   updated when upstream work landed.
6. **Targets**: host green; Android/wasm/packaging checks green when the
   issue requires them; no target was silently skipped.
7. **Discipline**: no commit by the worker; no unrelated files; no model or
   scope substitution; skills are truth.
8. **Evidence**: the commands and outputs in the report are reproducible;
   the orchestrator will re-run the gate.

## Verdict

End with exactly one of:

```
VERDICT: PASS
```

```
VERDICT: BLOCKING -- 1. <file:line — must fix> 2. ...
```

Non-blocking suggestions may appear above the verdict. A blocking verdict
must be actionable and tied to the issue contract or a real regression, not
to a preference.
