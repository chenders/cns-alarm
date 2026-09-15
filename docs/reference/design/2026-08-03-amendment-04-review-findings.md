# Amendment 04 — Multi-model review findings

**Date:** 2026-08-03
**Status:** Proposed
**Amends:** design §3, §4, §5 · amendment 01 §2, §3 · amendment 02 §1, §4, §5 · amendment 03 §1, §3
**Sources:** `docs/reviews/` — grok-4.5, deepseek-v4-pro, kimi-k2, gemini-3.1-pro
**Invalidates:** the `samples` primary key in T-D1's generated SQL, `db/deploy/samples.sql`, and the
COPY column layout T-D3 has not yet written

Findings from four independent reviewers, with the fixes. Ordered by severity. Where a reviewer was
right and the design was wrong, that is stated rather than softened.

---

## 1. `PRIMARY KEY (source, type, timestamp)` destroys RR intervals

**Severity: critical. This is silent data loss dressed as idempotency.**

> "RR, accel, and raw PPG cannot use `(source, type, timestamp)`… The schema gives you one row per
> source/type/instant and calls collisions idempotency. **For Polar that is data loss.**" — grok-4.5

A Polar H10 delivers several RR intervals in a single `0x2A37` notification. Stamped with packet
arrival time they share a timestamp, and `ON CONFLICT DO NOTHING` discards all but the first — while
every health metric reports normal.

### 1.1 The fix is the timestamp, not the key

A second reviewer found the cause:

> "Notification arrival time ≠ beat time. Multiple RRs in one packet need timestamps derived from a
> running clock of RR sums, not `clock_gettime` once per callback." — grok-4.5

**Each RR interval has a real, distinct instant.** Reconstruct it by accumulating the interval
durations backward from packet arrival:

```
packet arrives at T with intervals [r₁, r₂, r₃]   (units of 1/1024 s)
  beat₃ = T
  beat₂ = T − r₃
  beat₁ = T − r₃ − r₂
```

RR intervals are 200–2000 ms apart, and `TIMESTAMPTZ` carries microsecond precision, so derived beat
times do not collide. **The primary key stays as it is; the timestamp becomes correct.** One fix
closes both findings.

This also repairs everything downstream that the wrong stamps were breaking: gap detection, HRV
windows, sleep staging, and any rate estimator. Grok's detection signal — *"RR series jitter-clusters
on packet boundaries"* — was describing exactly this artifact.

### 1.2 Dedup must be observable

The remaining risk is that dedup silently fires for some *other* reason. So it stops being silent:

`vitalsd` counts rows rejected by `ON CONFLICT` per `(source, type)` and exposes the counter on the
health endpoint. **A nonzero RR dedup count is a fault, not housekeeping** — it means something
upstream is stamping badly. The admin console surfaces it, and the daily self-test fails on it.

Grok's check stands as an acceptance test: `samples_1min.sample_count` for RR must track heart rate,
not cap near 60.

---

## 2. The OOM killer will terminate the alarm

**Severity: critical. Caused by the Postgres tuning in the design.**

> "Postgres is tuned to use up to 6 GB while a Wayland compositor also runs concurrently… because
> `klaxond` and `vitalsd` lack negative adjustments, the kernel will kill your life-safety daemons to
> save the database. **The system prefers the database over your life.**" — gemini-3.1-pro

Design §4.1 sets `shared_buffers=2GB` and `effective_cache_size=4GB` on an 8 GB machine, with
wayfire also resident. Under pressure the kernel selects by OOM score and the safety daemons are
ordinary processes.

**Fix — every unit on the life-safety path:**

```ini
[Service]
OOMScoreAdjust=-1000        # never selected
MemoryMin=64M               # protected from reclaim
MemoryHigh=192M             # throttle before it becomes the problem
```

Applies to `vitalsd`, `klaxond`, `cns-engine`, and every collector. Postgres and the Docker
conveniences get **positive** adjustments so they are chosen first. Losing the database costs a
restart; losing `klaxond` costs the thing the system is for.

`vitals-verify-integrity` asserts the adjustments are in force, because a config that silently
reverts is the same as not having it.

---

## 3. "Independent failure domains" was false confidence

**Severity: high. Amendment 03 §3.1 is wrong as written.**

> "Amendment 03 §3.1 computes independence based on peripherals (GPIO vs USB), completely ignoring
> that all channels rely on the exact same CPU, kernel, PMIC, and `klaxond` process. If `klaxond`
> panics or the kernel locks up, all 'independent' channels die simultaneously." — gemini-3.1-pro

Correct. Every channel funnels through one process on one kernel. Counting GPIO and USB as separate
domains produces a reassuring number that describes nothing.

### 3.1 Honest accounting

Independence is reported in **tiers**, and the shared dependency is named rather than hidden:

| Tier | Survives | Channels |
|---|---|---|
| 0 — peripheral | a USB reset, an ALSA fault, a bulb losing power | siren vs DAC vs Zigbee |
| 1 — process | `klaxond` panicking | **nothing currently** |
| 2 — kernel | a kernel lockup or OOM | **nothing currently** |
| 3 — host | the Pi losing power | APNs and voice, only if already fired |

The console reports all four tiers. Amendment 03's claim of "N independent failure domains" is
replaced by "N peripheral-independent channels, **1 process, 1 kernel, 1 host**."

### 3.2 The hardware dead-man's switch

> "True independence requires a hardware dead-man's switch (e.g. a GPIO heartbeat pulsing a 555
> timer) that sounds an alarm if the software stack halts." — gemini-3.1-pro

`klaxond` pulses a GPIO pin at 1 Hz. A **555 monostable** retriggers on each pulse; if pulses stop
for ~10 s it times out and drives the siren **directly, in hardware**.

That is genuine tier-2 independence: it fires when `klaxond` panics, when the kernel locks, when the
scheduler stalls — every case where software cannot report its own failure. Roughly $3 in parts.

The BCM2712 hardware watchdog reboots a hung kernel but says nothing while it is hung. The 555 makes
that silence audible.

**New task: T-K6 — hardware dead-man's switch.** Arguably the highest-value item in the entire
alerting design, because it is the only thing that survives the software failing entirely.

---

## 4. The LUKS mitigation trains you to ignore the alarm

**Severity: high. Amendment 02 §4.1 is self-defeating.**

> "This intentionally turns every mundane OS reboot or power bounce into a blaring 3 a.m. emergency.
> **An alarm that trains the user to expect false positives is functionally a broken alarm.**"
> — gemini-3.1-pro

Right, and it is the worst possible place to introduce false positives.

**Revised behaviour on unlock failure:**

| | |
|---|---|
| Distinct, non-alarm signal | Panel shows `STORAGE LOCKED`; bulbs go amber; a short chime, not the klaxon |
| Escalates only if unresolved | If still locked after 15 minutes **and** it is within a monitoring window, then the alarm |
| Never at boot after a clean shutdown | A deliberate reboot is not an emergency |
| APNs immediately | The out-of-band path carries the urgency instead |

The klaxon tone is reserved for physiological events. A storage fault must be *loud in a different
way*, or the one sound that must mean something stops meaning it.

---

## 5. The alarm self-test does not test the alarm

**Severity: high. Found independently by two reviewers.**

> "ALSA write acceptance ≠ sound produced." — deepseek-v4-pro
> "That tests the digital path into the USB gadget, not pressure waves in the room." — grok-4.5

And deepseek supplied the mechanism: a USB DAC that resets gets a **new ALSA card index**, while
`/etc/asound.conf` from T-A2 hardcodes the old one. Writes keep succeeding, to nothing.

**Fixes, all required:**

1. Enroll the DAC by **stable identity** — `/dev/snd/by-id` or the USB VID:PID — never card index.
   Self-test fails if the enrolled device is absent, rather than writing to whatever is at index *N*.
2. Disable USB autosuspend on the DAC (`usbcore.autosuspend=-1` or a udev rule). A suspended DAC
   accepts writes and produces nothing.
3. Assert a hard volume floor at self-test — a muted mixer is a silent alarm.
4. **Current-sense on the piezo siren.** Current flowed or it did not, which is the closest thing to
   physical proof available without a microphone.
5. The 555 dead-man's switch (§3.2) is itself testable by withholding the heartbeat and confirming
   the siren fires.

---

## 6. Two documents contradict each other on the staging table

**Severity: high. Both are mine.**

> "Amendment 01 §2.2 says `CREATE TEMP TABLE staging … ON COMMIT DROP`. The sqitch deploy creates
> `CREATE UNLOGGED TABLE samples_staging` — a permanent, unlogged, **shared** table. If systemd
> respawns `vitals-spool-drain` before the old process exits… two drainers COPY into the same staging
> table simultaneously." — deepseek-v4-pro

The consequence is a partial batch committing while the other drainer's rows are lost, and it fires
under exactly the condition the system is built to survive: Postgres flaky, drainer restarted.

**Fix, both halves:**

- `db/deploy/samples.sql` drops `samples_staging`. The drainer uses a genuine `TEMP` table, private
  per session, per amendment 01 §2.2.
- The drainer takes `pg_advisory_lock(0x5641_4C53)` on startup and exits cleanly if it cannot. Two
  drainers are then impossible rather than merely unlikely.
- The spool files get `flock` advisory locks via the `fs4` crate, so a manual run cannot race a
  systemd timer.

---

## 7. A schema change orphans buffered spool rows

**Severity: high.**

> "Neither sqitch nor the tasks enforce the ordering… those rows are now unrecoverable — they're in
> the old format and the table is in the new format, with no downgrade path." — deepseek-v4-pro

Amendment 01 §2.3 documented the hazard and specified `--verify`, then the sqitch change set was
written without a single check that the spool is empty. Documented, not enforced — the exact pattern
this project exists to eliminate.

**Fix:**

- A sqitch change that touches `samples` begins with a guard that raises unless the spool is drained,
  read from a `spool_status` table the drainer maintains.
- `vitals-spool-drain.service` gains `Conflicts=sqitch-deploy.service`.
- The spool header carries a **schema fingerprint** — a hash of column names, order and types. The
  drainer refuses a mismatch rather than corrupting, and says which field differs.

---

## 8. Drain liveness, disk-full, and the rest

**Drain progress is unmonitored** (all three round-1 reviewers). `systemctl is-active` is true while
the drainer makes no progress — Postgres in recovery, a blocking lock, a `--verify` mismatch. The
health endpoint exposes **lag bytes, lag age, and last-advanced timestamp**; a monotonic offset that
stops advancing is a named degraded state.

**Disk-full is unspecified** (deepseek). A night of RR is ~250 MB; four nights of drainer downtime is
~1 GB. Add a watermark check: **80% warns, 95% refuses new spool segments** — segments, not samples,
so ingest degrades to in-memory rather than stopping. `pgbackrest` and Postgres must not be the
things that discover the disk is full.

**The HLC is vestigial** (grok, deepseek). Replication progress is a byte offset now; no two writers
share a key, since sources partition the key space by construction. `hlc_physical` and `hlc_logical`
are removed. **`node_id` stays** — it is provenance, and provenance is still wanted. This drops two
columns from every row and from the COPY layout, which is another reason to land the schema change
once.

**RTC absence breaks more than the timeline** (gemini). A 1970 clock also causes **TLS certificate
validation to fail**, so APNs and Twilio break at the same moment. The RTC battery is not a
nice-to-have; `chrony` must be `Before=` the collectors, and out-of-band alerting must report
degraded until the clock is sane.

**BLE MAC randomization** (gemini). T-B1 uses `bluetoothctl trust <MAC>`. Devices implementing LE
Privacy rotate their address, and static-MAC trust fails when they do. Pair properly so the IRK is
stored and resolvable private addresses work; identify devices by IRK, not address.

---

## 9. The tactile claim was over-read

> "You are conflating deafness (a mechanical/sensory deficit in a healthy brain) with CNS depression
> (a pharmacological suppression of the brainstem's arousal centres). Assuming a shaker overrides CNS
> depression because it bypasses deaf ears is a massive, unproven clinical leap." — gemini-3.1-pro

Fair, and amendment 03 §1 is corrected. What the evidence supports:

- Bed shakers with an intermittent pulse are highly effective at waking people **who cannot hear the
  alarm**. That is well established.
- A tactile channel has **dependencies disjoint** from the audio stack, needs no air path, and works
  through a closed door.

What it does **not** support: that tactile stimulus penetrates pharmacological arousal suppression
better than sound. Nobody has measured that.

So tactile stays a **first-class channel on redundancy grounds**, not on a claimed physiological
advantage. Fire everything simultaneously and claim nothing about which works.

---

## 10. Two claims in amendment 02 that do not survive

**The hash chain achieves little against root** (gemini). Root can alter a `samples` row, recompute
the hashes, and rewrite the chain to match. **In-database tamper evidence against root is theatre
unless the chain head leaves the device.**

Fix: publish the chain head — hash plus TPM monotonic counter — off-box hourly, to the APNs payload
and an append-only remote log. A local chain then detects *accidental* corruption; the published head
detects deliberate rewriting.

**The verification script is trivially bypassed** (gemini): the monotonic counter is checked by a
shell script root can edit to `exit 0`. Fix: verification runs where the data lands, not where it is
produced, and its output is signed by the TPM. A verifier root can patch verifies nothing.

**Signed ingest is process authentication, not clinical provenance** (gemini): a collector with a
decode bug signs its garbage faithfully. This is a real limit and is now stated in amendment 02 §2.4
rather than implied away. It proves *which process* produced a value, never that the value is
correct — the fuzzers and decoder tests do that job, and they are a different guarantee.

---

## 11. Tools to adopt

| Tool | Replaces | Why |
|---|---|---|
| **`kani`** | `cargo-fuzz` for `decode_sfloat` | Symbolic execution *proves* no panics, overflows or bounds violations across all 2¹⁶ inputs, in milliseconds. Keep fuzzing for the length-varying decoders where the input space is unbounded |
| **`rppal`** | sysfs GPIO / C wrappers | The maintained Rust GPIO crate for the Pi |
| **`fs4`** | hand-rolled locking | `lock_exclusive` on the spool offset and segments |
| **`sqlx`** | runtime-checked SQL | Compile-time-verified queries against a live schema |
| **`metrics`** | ad-hoc counters | Prometheus for lag, dedup counts, self-test ages |

`kani` on the decoder is the notable one: the existing exhaustive `u16` test already covers the input
space, but `kani` extends the same guarantee to `bp_decode_2a35`, where the input is a slice and
exhaustive testing is impossible.

---

## 12. What the reviewers explicitly endorsed

Recorded so it is not re-litigated:

- **Rust over C11**, specifically the argument about not hand-lowering Swift's `Option`/`Result` into
  C sentinels — "an excellent, concrete architectural argument" (gemini).
- Quarantine out-of-range rather than clamp (grok).
- The `0x2A35` CCCD `0x0002` indication note — "the kind of scar that deserves to be structural"
  (grok).
- `WITH NO DATA` on the continuous aggregate, avoiding a full scan at creation (deepseek).
- Arming the watchdog and governor in Stage 0 before any daemon depends on them — "this ordering
  discipline is the right instinct and it's applied consistently" (deepseek).
- The stage-deliverables progression from static artifacts to a live console (kimi).
- The panel as a pure subscriber — "right dependency direction" (grok).

## 13. Deferred with reasons

- **Wayfire on a life-safety device** (gemini) — a full Wayland compositor is real risk surface, and
  direct DRM/KMS would remove it. Deferred to Stage 8 because the panel is not on the safety path and
  the compositor already works. Revisit if it proves unstable.
- **"TimescaleDB is bypassed by the safety path"** (gemini) — true, and intentional: the database
  serves history, the admin console and analysis, while detection runs on in-memory ring buffers.
  §2's memory limits address the resource competition. Not a wrong architecture, but the reviewer is
  right that it must be justified rather than assumed.
