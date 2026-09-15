# CNS Alarm project instructions

## Purpose

This repository is the focused, safety-plane project for the best-effort
central-nervous-system (CNS) depression early-warning alarm on a Raspberry Pi Zero 2 W.
It is intentionally separate from AnxietyWatch2's storage, dashboard, historical analysis,
and device-integration work.

This is an early-warning aid, not a medical device, rescue system, or substitute for
emergency care. No implementation may describe missing data as safe.

## Acceptance

Acceptance is the exit code of a command that was actually run. Do not report success for a
command that was not run. Every task must have a deterministic Verify command before it is
considered complete.

## Safety invariants

- Three states always: measured-and-good, measured-and-bad, and not measured.
- Missing, stale, invalid, or low-quality evidence contributes nothing and never clears an alarm.
- Only validated primary evidence (SpO2 and/or respiratory rate) may confirm or clear CNS risk.
  Heart rate, HRV, and accelerometer data corroborate; they cannot clear a missing primary.
- Never clamp a physiological input into range. Reject it and name the reason.
- A sustained critical SpO2 signal from a validated continuous oximeter must have a fast path.
- Every escalation and clearance requires observed samples continuously within its sustain window;
  time spent disconnected does not count.
- An adapter restart or sensor reconnect is an evidence boundary, not proof of health.
- The alarm output must fail closed at boot and must be independently testable from the detector.
  A Pi process heartbeat is not a hardware dead-man switch.
- Do not add therapy-control commands, automatic emergency calls, or cloud dependencies.

## Scope boundary

The production target is a small Rust process on 64-bit Raspberry Pi OS. Keep the detector pure:
`(state, normalized evidence, explicit timestamp) -> (state, decision)`. Bluetooth, serial,
GPIO, files, networking, and clocks belong in thin adapters with explicit health states.

The Pi Zero 2 W may run the detector, watchdog, and alarm output. PostgreSQL/TimescaleDB,
the bedside page, bulk historical storage, and unrelated collectors remain in AnxietyWatch2.
The current project contains a reference copy of the existing Swift CNSRisk implementation;
it is not a Pi runtime dependency and must not be silently treated as a completed Rust port.

## Accelerometer policy

An accelerometer is useful as an arousal/body-movement and sensor-contact signal, not as a direct
CNS-depression measurement. No movement is not reassurance: a sleeping healthy person is still.
Motion may corroborate an already-valid primary event or identify motion/artifact windows, but it
must not independently escalate, clear, or suppress a primary physiological alarm until hardware
validation establishes placement, orientation, sampling rate, artifact behavior, and failure modes.

## Rust rules

- Rust, no unsafe code, and `cargo clippy --workspace --all-targets -- -D warnings`.
- No `unwrap`, `expect`, `panic!`, indexing of attacker-controlled input, or unchecked casts outside
  assertion-only tests.
- Decode bytes with pure functions and bounds checks; add a fuzz target for every radio decoder.
- Use fixed reference timestamps in tests; never assert floating-point equality.
- Every changed behavior gets a test. Keep the workspace small and dependency-light.
- Keep all thresholds in one explicit configuration type. Move them to a reviewed registry/config
  file before accepting production deployment; never scatter literals through adapters.

## Documentation

Update `docs/KNOWN-ISSUES.md` and `docs/IMPLEMENTATION.md` when behavior, thresholds, input
provenance, alarm output, or deployment assumptions change. Preserve uncertainty and negative
hardware results. Do not claim a device or sensor is validated from a family resemblance.

## Git

Never work directly on `main`. Create a branch first. Stage named files only, never `git add .`
or `git add -A`. Agent-authored commits include a `Co-Authored-By:` trailer. Commit after each
coherent task. The project is MIT licensed permanently; do not introduce a copyleft dependency
or change the license without explicit owner authorization.
