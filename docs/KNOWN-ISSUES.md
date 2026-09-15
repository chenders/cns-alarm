# Known issues and required follow-up

This list is intentionally blunt. An item remains open until its evidence and
verification command exist.

## Critical implementation gaps

- [ ] No production Bluetooth adapters are in this repository yet.
- [ ] No production GPIO/alarm-output driver exists.
- [ ] No independent hardware dead-man switch or current-sensed output exists.
- [ ] No watchdog supervisor exists.
- [ ] No field-ready battery, charging, enclosure, or tamper design exists.
- [ ] No local-only operator/participant interaction has been designed for a
      person without reliable internet, housing, or access to a screen.
- [ ] The Rust engine is a first porting scaffold; it has not been equivalence
      tested against every Swift `CNSRisk` test or reviewed for clinical use.
- [ ] The stdin harness is for replay only and must never be installed as the
      alarm daemon.

## Sensor uncertainty

- [ ] Wellue/PC-60F exact-model semantics, packet cadence, contact quality,
      perfusion behavior, battery behavior, and loss/replay behavior need live
      captures from the exact selected hardware.
- [ ] EMAY is not assumed equivalent to Wellue and is not currently a trusted
      alarm input.
- [ ] Polar H10 is corroborating heart-rate/RR evidence, not a standalone
      CNS-depression confirmation source.
- [ ] AirSense BLE therapy lifecycle, mask-off/large-leak state, and stream
      freshness need exact-device validation. A fresh packet is not automatically
      evidence that therapy is active.
- [ ] Respiratory-rate provenance and valid range need to be fixed before RR can
      drive an alarm.
- [ ] Accelerometer placement, orientation, contact, sampling rate, motion
      features, sleep movement, vibration, and sensor-removal failure modes are
      unvalidated. Stillness must never be interpreted as safety.

## Safety and field risks

- [ ] The system must prove `unknown` after power loss, clock failure, sensor
      removal, adapter restart, process restart, and IPC loss.
- [ ] Alert clearing must require fresh primary evidence; data silence must hold
      an alert or raise an integrity fault.
- [ ] Alarm audibility in outdoor/noisy environments is unmeasured.
- [ ] False-alarm rate and participant response/acknowledgement workflow are
      unmeasured. A harmful or embarrassing alarm can cause abandonment.
- [ ] The design needs explicit consent, privacy, retention, device ownership,
      data-access, and safe handoff rules for vulnerable participants.
- [ ] No emergency response path should be inferred. A local klaxon does not
      guarantee that another person hears it or that help arrives.
- [ ] Temperature, rain, sweat, dirt, impact, theft, battery depletion, and
      intermittent charging have not been tested.
- [ ] No recovery test proves operation after SD/filesystem corruption or a
      partially completed write.

## Performance and platform

- [ ] Pi Zero 2 W CPU/RAM/thermal behavior is estimated but not measured under
      worst-case sampling, Bluetooth reconnect, watchdog, and alarm output load.
- [ ] The intended 64-bit OS image, kernel, BlueZ version, and adapter support
      are not frozen.
- [ ] No bounded-memory soak test exists.
- [ ] No latency test exists from validated sample to physical alarm output.
- [ ] No test exists for scheduler starvation while the alarm remains fail-safe.

## Evidence required before field use

A real acceptance run must include normal operation and injected failures:

1. fresh participant-consented sensor setup;
2. healthy primary stream;
3. controlled low-signal/artifact conditions;
4. safe synthetic/replay risk fixtures with no real participant harm;
5. sensor removal and radio loss;
6. Pi reboot during an active observation;
7. power loss and battery recovery;
8. detector hang and output-path failure;
9. measured output audibility/current;
10. exact logs, versions, timestamps, and exit codes.

No item above may be silently changed from unknown to passing because a process
is running or a value is visible.
