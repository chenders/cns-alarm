# Review and work prompts

These prompts are intended to be pasted into separate review/work sessions after
reading `AGENTS.md`. They are deliberately staged: first establish that the
repository is coherent, then choose one bounded implementation task.

## Readiness prompts

### 1. Repository integrity review

> Read `AGENTS.md`, `README.md`, `docs/IMPLEMENTATION.md`, and
> `docs/KNOWN-ISSUES.md`. Inspect the full repository and run the documented
> verification commands. Report only evidence from commands actually run.
> Identify missing files, unsafe claims, stale copied documentation, dependency
> problems, license drift, and any place where unknown data can become clear.
> Do not modify code. Return PASS/FAIL per command and a prioritized blocker list.

### 2. Rust engine safety review

> Review `crates/cns-engine` as a safety-critical deterministic state machine.
> Compare its behavior with the reference Swift CNSRisk files under
> `docs/reference/swift-cns-risk`. Check units, valid ranges, baseline poisoning,
> low perfusion, artifacts, sparse windows, timestamp gaps, primary-vs-
> corroborating rules, hysteresis, fast-path escalation, and missing-data
> behavior. Run tests and clippy. Do not call an untested behavior safe. Produce
> concrete findings with file/line, severity, and a regression-test proposal.

### 3. Field reliability review

> Review this project for operation by a vulnerable person who may be homeless,
> alone, outdoors, without reliable charging, Wi-Fi, cellular service, or access
> to a screen. Evaluate battery, rain, dirt, sweat, impact, theft, sensor
> removal, consent, privacy, audibility, false alarms, reboot recovery, and
> failure indication. Assume the network and the Pi 5 are unavailable. Return
> design changes and hardware tests; do not weaken the unknown/not-measured
> invariant for convenience.

### 4. Physiology and measurement review

> Independently review every signal and threshold in this repository and the
> referenced CNSRisk design. Verify canonical units, plausible ranges, source
> fidelity, use of SpO2/RR as primary evidence, the role of pulse/HRV, and the
> proposed accelerometer role. Reject claims unsupported by exact-device data.
> Identify false-reassurance paths and false-alarm paths. Return a list of
> values that require qualified clinical review and tests that must precede
> participant use.

### 5. Threat-model and privacy review

> Review the project assuming the device can be lost, stolen, inspected, shared,
> or intermittently connected. Identify participant-identifying data, raw sensor
> retention, logs, local IPC, update/authentication, physical access, and
> deletion/handoff risks. Keep the project MIT licensed. Do not add cloud or
> emergency-contact behavior without explicit consent and a reviewed design.

### 6. Pi Zero performance review

> Build for 64-bit Raspberry Pi OS and benchmark the engine under worst-case
> sample rate, reconnect storms, malformed input, bounded-memory pressure,
> scheduler delay, and alarm-output activity. Measure CPU, RSS, latency, queue
> depth, thermal behavior, and recovery after power loss. Report exact hardware,
> kernel, Rust, and BlueZ versions and exact command exit codes. Do not infer
> readiness from a successful desktop build.

## Implementation task prompts

### 1. Add a normalized local input protocol

> Design and implement one bounded, versioned local IPC protocol for normalized
> sensor samples and source health. Read `AGENTS.md` and `docs/IMPLEMENTATION.md`
> first. Specify identity, UTC and receipt timestamps, sequence/duplicate rules,
> invalid/missing fields, restart boundaries, backpressure, memory limits, and
> `unknown` states. Add pure parser tests, fuzz coverage for hostile input, and
> a replay fixture. Do not add Bluetooth or GPIO in this task.

### 2. Port the Wellue input safely

> Add an isolated Wellue/PC-60F adapter only for the exact hardware and packet
> captures named in the task. Preserve raw evidence before decode where needed,
> reject invalid units/ranges/checksums, expose contact/perfusion/battery/link
> health, and never turn no-finger or radio loss into normal values. Add fuzz
> coverage, replay fixtures, exact-device acceptance steps, and documentation of
> every unresolved protocol assumption. Do not change CNS thresholds in this task.

### 3. Add Polar H10 corroboration

> Add the standard Heart Rate Service input behind the normalized protocol.
> Preserve RR artifacts and sequence behavior. Prove with tests that H10 alone
> cannot confirm or clear CNS depression and that disconnects become unknown.
> Include resource and reconnect tests on the Pi Zero 2 W. Do not add alarm GPIO
> in this task.

### 4. Add accelerometer evidence

> Define a motion evidence model that records bounded movement energy,
> orientation/placement, contact, and artifact status without treating stillness
> as safety. Validate the chosen chip and placement across sleep positions,
> vibration, transport, sensor removal, sweat, and impact. Add replay fixtures
> and prove motion alone cannot escalate or clear a primary physiological alert.
> Do not promote raw acceleration to a vital sign without a separate physiology
> review.

### 5. Implement the independent alarm output

> Design the GPIO/electrical alarm boundary for fail-safe boot-off behavior,
> external pulldown/dead-man supervision, audible output, self-test, battery
> behavior, and acknowledgement. Keep output side effects outside the pure
> engine. Add a hardware-in-the-loop test that measures actual current/voltage
> and output latency, not merely syscall success. Review physical safety before
> connecting a loud output to a participant device.

### 6. Implement the watchdog

> Add a local watchdog that distinguishes process alive, detector evaluating,
> fresh primary evidence, output path healthy, clock valid, and battery/sensor
> health. Make every blocker name the missing evidence. Test process hang,
> scheduler starvation, IPC loss, reboot, filesystem failure, and sensor removal.
> Ensure watchdog failure cannot silently clear or suppress a real alarm.

### 7. Pi Zero field acceptance

> Prepare a bounded, consented acceptance run on the actual Pi Zero 2 W and
> enclosure. Include healthy streams, no-data, low-perfusion/artifact, sensor
> removal, adapter loss, reboot during a session, power loss, battery recovery,
> detector failure, alarm-output failure, and no-network operation. Record exact
> versions, commands, exit codes, measured resource/latency data, and all
> unknown results. Do not test a dangerous physiological scenario on a person;
> use reviewed replay fixtures and safe hardware tests.
