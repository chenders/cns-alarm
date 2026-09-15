# Amendment 05 — Corrected identity model, dead-man redesign, runtime safety

**Date:** 2026-08-03
**Status:** Proposed
**Supersedes:** amendment 04 §1 (RR identity) and §3.2 (dead-man switch) — **both were wrong**
**Closes:** `docs/reviews/SYNTHESIS.md` A2, A3, and C1–C11
**Affects:** T-B1, T-D3, T-D4, T-D5, T-D6, T-K2, T-K3, T-K6

Amendment 04 answered the first review round. A third round found two of its fixes wrong and eleven
items still open. This corrects both and closes the rest.

---

## 1. RR identity — corrected (supersedes A04 §1)

A04 claimed that reconstructing beat times backward from packet arrival made timestamps distinct, so
`PRIMARY KEY (source, type, timestamp)` could stay. That was wrong:

> "Backward accumulation can estimate [event time], but **packet arrival is not a sufficiently
> stable anchor** to supply [ingest identity]… retransmission/replay after reconnect: the same RR
> payload anchored to a later arrival receives entirely new timestamps, **so the PK accepts
> duplicates**." — gpt-5.6-sol

The failure *inverted*. The original bug dropped real data; the "fix" would have admitted duplicates.

### 1.1 Three concepts, three fields

| Concept | Field | Property required |
|---|---|---|
| Ingest identity | `(source, type, session_id, seq)` | **stable** across retransmission and jitter |
| Event time | `timestamp TIMESTAMPTZ` | best estimate; may be revised |
| Observation time | `ingest_batch.ts_first/ts_last` | when the host saw it |

**`seq` is a monotonic per-session index derived from the sensor's own data, never from host
timing.** For RR intervals it is the beat index within the session — the count of intervals received
since the session opened, which the cumulative RR sum reproduces deterministically.

### 1.2 Why this is stable where arrival time was not

| Event | Behaviour |
|---|---|
| **Retransmission within a session** | Same beats, same `seq`. `ON CONFLICT DO NOTHING` dedupes **correctly** — this is the one case where dedup is right |
| **Connection-interval batching** | `seq` is unaffected; only `timestamp` shifts, and `timestamp` is an estimate |
| **Host scheduling stall** | Same — the estimate moves, identity does not |
| **Reconnect** | A **new `session_id`**. Continuity across a disconnect cannot be guaranteed, so claiming it would be dishonest |

The reconnect case is the important one. A new session means no false dedup against the old session's
`seq` values, and the discontinuity is explicit in the data rather than hidden inside a timestamp.

### 1.3 A capability the old schema did not have

`seq` is dense within a session, so **`max(seq) - count(*)` is the number of lost beats.** Packet
loss becomes a measurable quantity per session rather than an invisible one.

This is the detection signal for the whole class: a nonzero gap count means dropped notifications; a
nonzero `ON CONFLICT` count within a session means retransmission (expected, benign); a nonzero
conflict count *across* sessions means `session_id` minting is broken.

All three go on the health endpoint.

### 1.4 Event-time reconstruction, and its honest limits

`timestamp` is still reconstructed by accumulating RR durations backward from packet arrival — but as
an estimate, which is now allowed:

```
packet arrives at T with intervals [r₁, r₂, r₃]   (units of 1/1024 s)
  beat₃ = T − latency_estimate
  beat₂ = beat₃ − r₃
  beat₁ = beat₂ − r₂
```

**The sensor's clock drifts against the host's.** The H10 reports intervals in its own time base; over
eight hours the accumulated difference is real. Two consequences, both recorded rather than papered
over:

1. Per-session, fit a linear correction between cumulative RR sum and host arrival times. Store the
   fit coefficients on the session so timestamps can be **revised post-hoc** without touching
   identity — which is only possible because `timestamp` is not the key.
2. Report the residual. A drift rate outside ±0.5% is a sensor or host clock fault, not normal.

**T-B1 must record the observed drift** as part of its gap histogram. It is a property of this
hardware pairing that nothing else will measure.

---

## 2. Dead-man switch — corrected (supersedes A04 §3.2)

A04 specified a 555 monostable retriggered by a 1 Hz `klaxond` heartbeat. It fails in **both**
directions.

**False negative** — gpt-5.6-sol:
> "A healthy `klaxond` can continue pulsing while `vitalsd`, `cns-engine`, every BLE collector, or
> the live subscription path is dead. **That is the worst false negative available.**"

**False positive** — deepseek:
> "During a deliberate `sudo reboot`… the 555 times out mid-shutdown and the siren blares. **Every
> clean reboot produces a false alarm.**"

### 2.1 The heartbeat attests the chain, not the process

`klaxond` pulses the pin **only if all of the following hold**:

```
  monitoring window is active
    AND a sample from a primary source arrived within the last 30 s
    AND cns-engine produced a tier evaluation within the last 30 s
    AND the spool drain offset advanced, or the spool is empty
```

Outside a monitoring window the heartbeat is **unconditional** — nothing is expected, so absence is
not a fault. Silencing an active alarm does **not** stop the heartbeat: an acknowledged alarm is a
working system, not a dead one.

The heartbeat now means *"the path from radio to alarm is intact,"* which is the property worth
attesting. `klaxond` merely being scheduled is worth nothing.

### 2.2 Shutdown inhibit

A second GPIO drives a hardware disable on the 555's output stage.

```ini
# klaxond.service
ExecStartPre=/usr/local/bin/deadman-inhibit release
ExecStop=/usr/local/bin/deadman-inhibit assert
```

`assert` before the pulse stops; `release` only once `klaxond` is running and the chain is verified.
A deliberate reboot is silent. A crash, a kernel lockup or a power-loss-then-hang is not, because
nothing asserted the inhibit.

**The inhibit must fail safe:** it is released by default at power-on and asserted only by an explicit
command, so a failure of `deadman-inhibit` leaves the alarm armed rather than disarmed.

### 2.3 The test must inject upstream faults

A04's proposed test — stop the heartbeat, confirm the siren — tests the 555 and nothing else.

The real test, monthly, and it must be able to fail:

| Injected fault | Expected |
|---|---|
| `SIGKILL vitalsd`, leave `klaxond` running | siren within ~15 s |
| `SIGSTOP cns-engine` | siren within ~15 s |
| Kill every collector | siren within ~15 s |
| Block the drain (revoke `INSERT`) | siren within ~15 s |
| `systemctl stop klaxond` (deliberate) | **silence** |
| `reboot` | **silence** |

The last two are as important as the first four. A dead-man switch that cannot distinguish a crash
from a shutdown is an alarm-fatigue generator.

---

## 3. Every alarm output is OFF from power-on — in hardware

Three separate designs have now produced a false alarm on an ordinary reboot: the LUKS unlock policy
(A04 §4), the dead-man switch (§2 above), and a floating MOSFET gate. The alarm output attracts this
because it is the one signal where the failure state is *on*, and every uninitialised default is
*unknown*.

**Rule: an alarm output must be provably OFF from power-on until `klaxond` explicitly asserts it, by
hardware, not by software timing.**

- **10 kΩ gate-to-source pulldown on every MOSFET.** Pi GPIO floats from power-on until configured.
  Not optional; without it the siren sounds at every boot.
- Flyback diode across every inductive load (the bed shaker is a motor).
- The 555 inhibit defaults to *released* (armed), asserted only deliberately — the opposite polarity
  to the MOSFET gates, and correct: failing to disarm is safe, failing to arm is not.
- **T-K2's acceptance includes a cold-boot test**: power on with a scope or meter on the siren line
  and confirm it never rises.

---

## 4. Closing the open items

### 4.1 Drain liveness (C1)

`systemctl is-active` is true while the drainer makes no progress. The health endpoint exposes
`lag_bytes`, `lag_age_seconds`, `last_advanced_at`; a monotonic offset that stops advancing for more
than 5 minutes with a non-empty spool is a **named degraded state** that says which of Postgres
down / lock held / fingerprint mismatch is responsible. Landed in T-D5.

### 4.2 Disk-full (C2)

A night of RR is ~250 MB. Warn at 80%; at 95% refuse new spool **segments** — not samples — so ingest
degrades to in-memory rather than stopping. Postgres and pgBackRest must not be the components that
discover the disk is full. A `vitals-disk-watch` timer surfaces it, and the panel shows projected
days-to-full rather than a percentage, because a percentage is not actionable.

### 4.3 BLE MAC randomisation (C3)

T-B1 uses `bluetoothctl trust <MAC>`. Devices implementing LE Privacy rotate their address and static
trust then fails. **Pair properly so the IRK is stored**, identify devices by IRK rather than address,
and treat "device not found" as distinct from "device out of range" — the first is a pairing fault,
the second is normal.

### 4.4 Off-box backup (C4)

Currently all three tiers live on one disk, which makes "restore from the failed device" theatre.
**Required before Stage 5 puts the klaxon into service**, per amendment 01 §7. A `repo2-*` block in
`pgbackrest.conf` plus one `stanza-upgrade`.

### 4.5 Spool format version (C5)

The spool header carries a schema fingerprint — a hash of column names, order and types. The drainer
refuses a mismatch and names the differing field. A sqitch change touching `samples` guards on
`spool_status` being drained. Landed in T-D3 and T-D5.

### 4.6 Spool locking (C6)

`fs4::FileExt::lock_exclusive` on the segment and the offset file, plus `pg_advisory_lock` on the
drainer. A manual run cannot race a systemd timer, and two drainers cannot exist. Landed in T-D5.

### 4.7 Stuck `cns-engine` (C7)

A wedged detector is invisible: no error, no crash, no output. `cns-engine` emits a tier evaluation
on every sample **even when the tier is unchanged**, so the absence of evaluations is detectable.
`vitalsd` tracks time-since-last-evaluation, and it feeds the §2.1 heartbeat condition — a stuck
detector therefore fires the hardware dead-man switch.

### 4.8 Gap-fill enforcement (C8)

`time_bucket_gapfill` must be used on every chart and scoring read so gaps return explicit NULLs.
**`locf()` and `interpolate()` are banned on physiological series** — they exist to fill gaps
plausibly, which is precisely the failure this system cannot tolerate. A CI grep over `sqlx` query
files enforces it; permitted only for genuinely step-valued config series.

### 4.9 End-to-end latency SLO (C9)

Undefined latency means nobody notices it degrading. **Radio to tier evaluation: p99 under 2 s.**
Measured continuously from the sensor's own timestamp to the evaluation, exported via `metrics`, and
surfaced on the panel. A p99 above 5 s is a degraded state — the alarm is only as timely as this
number.

### 4.10 Warm replay after a `vitalsd` restart (C10)

`vitalsd` restarting empties `cns-engine`'s ring buffers, leaving detection blind for the warm-up
window with nothing saying so. On restart, `cns-engine` **replays the last 5 minutes from Postgres**
before declaring itself ready, and reports `warming` — not `clear` — until its buffers are full.
Indeterminate is not safe, and that applies to the detector's own state.

### 4.11 Subscriber backpressure (C11)

A slow subscriber must not block ingest, and dropping samples from detection while they are durable
in the spool is a silent hole. Policy: a bounded per-subscriber queue; on overflow **drop and count**,
never block the collector, and **raise a degraded state naming the subscriber**. A detector that
missed samples must know it missed them — the count feeds the same "can't assess" path as a sensor
gap.

---

## 5. Task changes

| Task | Change |
|---|---|
| T-B1 | Record clock drift and `seq` gap count alongside the gap histogram; use IRK pairing |
| T-D3 | `session_id`/`seq` derivation; drift fit stored per session |
| T-D4 | Heartbeat condition (§2.1); subscriber backpressure (§4.11); liveness counters |
| T-D5 | Already carries the drain lock, fingerprint and watermarks |
| T-D6 | `seq` from the cuff's own measurement sequence, not arrival |
| **T-K2** | Cold-boot test: confirm the siren line never rises from power-on |
| **T-K6** | Conditional heartbeat, shutdown inhibit, fault-injection test matrix |

## 6. What this does not change

The narrow-waist store, the registry as single source of truth, Rust, TimescaleDB, the spool, sqitch,
signed ingest, and the alerting channel abstraction all stand. This amendment corrects two mechanisms
and closes gaps; it does not revisit the architecture.
