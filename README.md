# CNS Alarm

A focused, MIT-licensed safety-plane project for a **best-effort early warning of
central-nervous-system (CNS) respiratory depression** on a Raspberry Pi Zero 2 W.
It is being designed for people who may be homeless, unstably housed, alone,
without reliable charging, and without dependable Wi-Fi or cellular service.
That operating environment changes the engineering target: the alarm must fail
loudly when it cannot assess, survive ordinary abuse and interruption, and never
turn silence into reassurance.

> **This is not a medical device, overdose rescue system, or substitute for
> emergency care.** A prototype must not be represented as a guarantee of safety.
> A person in immediate danger needs emergency services and human help.

## Why this is a separate project

`AnxietyWatch2` is the broader data platform: PostgreSQL/TimescaleDB, historical
storage, bedside pages, CPAP integration, and many device experiments. This
repository contains only the safety-plane work needed to acquire validated live
evidence, evaluate CNS risk, and drive an independent alarm. Storage, dashboards,
cloud synchronization, and unrelated devices do not belong in the critical path.

The repository includes a reference copy of the existing Swift `CNSRisk`
implementation under [`docs/reference/swift-cns-risk`](docs/reference/swift-cns-risk).
The runtime target is Rust; the Swift files are provenance and behavior reference,
not a claim that Swift runs on the Pi.

## Current contents and honest status

- `crates/cns-engine`: small, deterministic Rust quality gate, severity scorer,
  fusion layer, and hysteretic tier machine.
- `crates/cns-alarm`: a bounded CSV/stdin replay and benchmarking harness.
- `docs/IMPLEMENTATION.md`: target architecture and staged implementation plan.
- `docs/KNOWN-ISSUES.md`: unresolved protocol, physiology, hardware, and field
  reliability questions.
- `docs/reference/`: copied design decisions, hardware research, and the prior
  CNSRisk implementation.

The Bluetooth and GPIO adapters are **not production-ready here yet**. The
current executable deliberately accepts replay input rather than pretending that
an unvalidated radio or alarm driver is safe. The first deliverable is a tested
engine with explicit `insufficient_data`, followed by hardware adapters and an
acceptance run on the actual field device.

## Design principles for this population

1. **No network dependency.** Detection and the local alarm must continue without
   Wi-Fi, cellular data, a phone, or a server.
2. **Power loss is an evidence boundary.** On reboot, the system starts in
   `unknown` until it receives fresh, validated evidence. It never restores a
   previous `clear` or `active` claim as current fact.
3. **No data is not good data.** Missing SpO₂, respiratory rate, pulse, or motion
   is surfaced as not measured. It cannot clear a warning or prove a person is
   safe.
4. **Primary evidence leads.** Continuous SpO₂ and validated respiratory rate can
   drive confirmation. Pulse and HRV corroborate. An accelerometer can describe
   movement, arousal, placement, and artifact; no movement alone is not evidence
   of depression or safety.
5. **Robustness over convenience.** Use local durable configuration, watchdogs,
   brownout-safe output, weather/impact-aware enclosure design, replaceable
   battery/charging plans, and a loud, independently testable output.
6. **Privacy by default.** Do not store names, GPS, raw audio, drug identity, or
   raw physiological streams unless an explicit, reviewed need exists. A local
   device identifier must not become a person identifier.
7. **Uncertainty is visible.** Every blocker names the evidence that is absent or
   invalid. Unknown is never rendered or scored as clear.

## Planned hardware input set

- **Continuous oximeter:** Wellue/PC-60F-family input is the leading SpO₂/pulse
  candidate, but exact-device framing and signal quality still require field
  validation.
- **Pulse/heart rate:** Polar H10 is the strongest validated heart-rate source in
  the existing work. It is corroborating, not a standalone CNS confirmation
  source.
- **Respiratory rate:** Prefer a directly validated respiratory signal. AirSense
  data is useful context but its therapy lifecycle and BLE stream state must be
  known before it can inform the alarm.
- **Accelerometer:** A planned chip on the alarm node may detect body movement,
  wake/arousal activity, device displacement, and motion artifact. It must not be
  promoted to a direct depression detector without placement-specific validation.

For people sleeping outdoors or moving between locations, the design must also
handle sensor removal, wet/dirty contacts, temperature, vibration, occlusion,
low battery, charging interruption, theft/loss, and inability to reach a screen.
Each condition must produce a distinct `unknown`/degraded state and an actionable
local indication rather than a silent green light.

## Development

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

# Replay one normalized observation per line:
# timestamp_micros,kind,source,value,perfusion_index,artifact
printf '%s\n' \
  '10000000,spo2,wellue,96,2.0,0' \
  '11000000,heart_rate,polar_h10,70,absent,0' \
  | cargo run -p cns-alarm
```

The harness is not an alarm driver. It has no GPIO side effect and must not be
used as a field safety device. A real adapter must be added behind a typed,
health-reporting interface and tested with fault injection before deployment.

## Review expectations

Every change involving thresholds, signal interpretation, input adapters, or the
alarm output needs independent review from the relevant disciplines:

- Rust safety/concurrency review;
- physiological measurement and clinical-domain review;
- Bluetooth/electrical/hardware review for sensor or output changes;
- privacy/threat-model review for identity, logs, or telemetry;
- field-reliability review for battery, weather, abuse, accessibility, and
  unattended restart behavior.

A review is not evidence of hardware validation. Preserve real captures,
negative results, and exact verification commands.

## License

MIT. This project will remain MIT licensed.
