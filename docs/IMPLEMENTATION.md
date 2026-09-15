# Implementation plan

## Safety-plane boundary

The intended runtime is a small 64-bit Rust process on a Pi Zero 2 W:

```text
validated sensor adapters
        │ normalized samples + explicit health
        ▼
     cns-engine  ───► decision/status journal
        │
        ▼
 independent alarm-output adapter + watchdog
```

The engine must remain pure and deterministic. It receives a bounded set of
normalized observations, baselines, source health, and an explicit timestamp.
It returns an assessment and a tier. It does not read a wall clock, access
Bluetooth, write a file, use a network, or touch GPIO.

The alarm node must not depend on AnxietyWatch2, PostgreSQL, a web page, a phone,
cloud APIs, or the availability of the Pi 5. Those are useful for history and
operator review but are not part of the safety decision path.

## States

Every input and the overall detector use distinct states:

- **measured and good**: sufficient, recent, validated evidence;
- **measured and bad**: evidence supports risk;
- **not measured/unknown**: absent, stale, invalid, disconnected, rebooted, or
  ambiguous evidence.

Unknown holds an existing alert and prevents clearing. It must not escalate by
itself, but the watchdog/alarm-integrity path must separately report that the
system cannot assess.

## Input order of work

### 1. Replayable normalized input

The current stdin harness exists to make the engine testable without hardware.
Replace it with a versioned local IPC or bounded event queue only after defining:

- source identity and pairing/ownership;
- UTC timestamp and receipt timestamp;
- sequence and duplicate behavior;
- explicit invalid/missing fields;
- restart and queue-loss semantics;
- maximum memory and backpressure behavior.

### 2. Wellue/PC-60F-family oximeter

Port only the exact captured framing and checksum behavior needed by the selected
hardware. Preserve raw evidence before decode where loss is otherwise
irrecoverable. Validate SpO₂ units, pulse, perfusion index, finger/contact state,
packet cadence, disconnects, battery, and duplicate/reordered frames on the exact
model. A family resemblance is not protocol validation.

### 3. Polar H10

Add the standard Heart Rate Service adapter as corroborating input. Preserve RR
artifact handling and sequence behavior. H10 alone must not confirm CNS depression.

### 4. Respiratory rate

Choose one directly validated source. If using AirSense, keep therapy state,
mask-off/large-leak, stream-stalled, and collector-restart states separate. A
recent pressure packet is not proof that therapy is active or that respiratory
rate is current.

### 5. Accelerometer

Use a separate motion model. Record bounded features rather than treating raw
acceleration as a vital sign: movement energy, orientation/placement, contact,
impact, and artifact windows. Calibrate across body locations and sleep
positions. Motion can corroborate arousal or identify bad sensor contact; no
movement cannot clear a physiological alert and cannot independently trigger one.

### 6. Alarm output

Implement an output adapter with:

- GPIO configured to a known OFF state before any application logic;
- external pulldown/dead-man hardware so process death is observable;
- a loud local output usable without a screen or network;
- a self-test that proves current/voltage/output, not merely that a write syscall
  succeeded;
- rate limiting and acknowledgement rules that cannot turn a critical alert off
  because data disappeared.

### 7. Watchdog and field operation

The watchdog must distinguish detector alive, detector evaluating, fresh primary
evidence, output path healthy, battery adequate, and enclosure/sensor present.
It must fail closed on reboot and report why. Test brownouts, battery depletion,
USB removal, SD corruption, clock errors, process hangs, kernel stalls, blocked
IPC, sensor removal, and prolonged no-network operation.

## Threshold provenance

The initial Rust thresholds are a porting scaffold from the existing Swift
`CNSRisk` implementation. They are not clinical advice and are not yet a
registry-generated production configuration. Before field deployment:

1. move thresholds into a reviewed, versioned configuration/registry;
2. record units, valid ranges, evidence, and owner for every value;
3. test baseline poisoning, fraction-vs-percent errors, low perfusion, sparse
   samples, gaps, artifacts, and clock jumps;
4. have a qualified physiology/clinical reviewer sign off the behavior;
5. preserve the old and new decisions in replay comparisons before switching.

## Target deployment shape

Use a 64-bit Raspberry Pi OS image if supported by the chosen peripherals. Keep
the alarm process and watchdog small enough to reserve memory for the kernel and
Bluetooth stack. Do not place PostgreSQL or a full web stack on the Zero 2 W.
Use a read-only or recoverable filesystem, bounded logs, durable configuration,
monotonic scheduling for timers, and explicit UTC conversion at input boundaries.

The Pi Zero 2 W is an engineering target, not yet an acceptance result. Measure
worst-case CPU, RSS, scheduling latency, input backlog, alarm-output latency,
recovery after power loss, and thermal/battery behavior on the actual enclosure
and power source.
