# Amendment index

**Mandatory reading for every task**, regardless of that task's `Reads:` list — see `AGENTS.md`.

Amendments supersede the original design and, where they disagree with a task brief, **the amendment
wins**. If you find a contradiction, report it rather than implementing either version.

---

## Accepted amendments

| # | Title | Supersedes | Affects |
|---|---|---|---|
| **01** | [Drop SQLite; COPY-binary spool + TimescaleDB](2026-08-03-amendment-01-timescale-copy-spool.md) | design §2.1, §2.5, §4.1 | T-C1, T-D3, T-D5 |
| **02** | [Integrity, provenance, encryption at rest](2026-08-03-amendment-02-integrity-and-encryption.md) | design §2 | T-C1, T-D3, T-D4, + T-C4/C5/C6, T-D7, T-H1 |
| **03** | [Multi-modal alerting](2026-08-03-amendment-03-multimodal-alerting.md) | design §5, T-A2 | Stage 5, + T-K1…T-K6 |
| **04** | [Multi-model review findings](2026-08-03-amendment-04-review-findings.md) | design §3–§5, amendments 01–03 | most of Group D |
| **05** | [Corrections and runtime safety](2026-08-03-amendment-05-corrections-and-runtime-safety.md) | **amendment 04 §1 and §3.2 — both were wrong** | T-B1, T-D3…D6, T-K2, T-K6 |
| **06** | [BP identity and durability](2026-08-03-amendment-06-bp-identity-and-durability.md) | **amendment 05 §5 (T-D6 row) — the hardware disproved it**, and **§1.1's identity rule, which it narrows**; extends §4.3; restates A01 §5's durability rule | T-B1, T-D3, T-D4, T-D6 |
| **07** | [The HTTP API and admin console are Rust, not Python](2026-08-05-amendment-07-api-in-rust.md) | **`language-choices.md` §1 row "HTTP API, admin, analysis → Python 3.12 + FastAPI", and §3 in full** | admin console M0–M2, web dashboard backend, client write API |
| **08** | [Layout selection by live evaluation, not upfront choice](2026-08-06-amendment-08-layout-evaluation.md) | the implicit "pick a design direction before M1" in `SPEC-admin-console.md` §3.1 | admin console M1 (all three directions behind a runtime switcher), web dashboard (gains a plan home: port B/C onto the `model()` seam + switcher, gated on the vitalsd query path) |
| **09** | [What the admin console is *for*](2026-08-09-amendment-09-admin-console-purpose.md) · [changes & rationale](2026-08-09-amendment-09-changes-and-rationale.md) | **`SPEC-admin-console.md` §3 (IA order and milestones), §4.7 (Analysis — was job plumbing only), §4.8 (Records — was "prompt-input curation")**; adds §4.11 | admin console M2 (feature control + analysis read-only, both pulled earlier), M3 (analysis runs), the analysis data model, `PROMPT-admin.md` → `PROMPT-admin-v2.md` |
| **GOV** | [Repository governance, recovery and worktree lifecycle](../../openspec/changes/repository-recovery-and-lifecycle/proposed-amendment-governance-workflow.md) · [owner decision](../../openspec/changes/repository-recovery-and-lifecycle/reviews/governance/owner-decision-2026-09-06.md) | `AGENTS.md` worktree-per-task wording becomes one branch/worktree per **concurrently active** task; extends task, preservation, review, publication and deployment contracts | repository/documentation work and future governance tooling; no physiological or schema changes |
| — | [C vs Rust evaluation](2026-08-03-c-vs-rust-evaluation.md) | the C11 decision | all code tasks |
| — | [Review synthesis and fix tracker](../reviews/SYNTHESIS.md) | — | status of every finding |

---

## The short version — what changed since the original design

Read this before any implementation task. Each line replaces something the original design or an
early task brief still says.

### Repository governance and authoring

**GOV is owner-accepted as policy, not implemented tooling.** Serial tasks may use the canonical
checkout under exclusive ownership; concurrent writers still require separate branches/worktrees.
Preservation, bounded inventory, evidence, task and closure requirements apply as specified in GOV.
New/enhanced enforcement remains unavailable unless actual evidence establishes otherwise; the
separately authorized squash-only restriction is verified. Copilot remains the required route until
an explicitly approved, verified replacement transition. No blanket cleanup, store migration,
private disclosure, runner installation or deployment permission follows from acceptance.

The owner also approved [OpenSpec authoring guidance](../../openspec/changes/repository-recovery-and-lifecycle/reviews/openspec-authoring-guidance.md)
under GOV unit B; see its [approval record](../../openspec/changes/repository-recovery-and-lifecycle/reviews/governance/authoring-owner-decision-2026-09-06.md).
Use its requirement/scenario/task/evidence structure for the next scoped implementation plans.
Numeric GOV numbering is deferred; do not overwrite the existing numbered drafts.

### Language

**Rust, not C11.** Cargo workspace, not meson. `#![forbid(unsafe_code)]` in decoder crates. Clippy
lints at `deny`: `indexing_slicing`, `unwrap_used`, `panic`, and the cast lints. `panic = "abort"` in
release. Toolchain pinned in `rust-toolchain.toml`.

### Storage

**SQLite is gone.** There is no on-Pi SQLite store, no GRDB, no WAL tail.

Collectors append rows **already encoded in PostgreSQL binary `COPY` format** to a spool file.
`vitals-spool-drain` streams those bytes into `COPY … FROM STDIN (FORMAT binary)` via a **private
`TEMP` staging table** — never a shared one — then `INSERT … ON CONFLICT DO NOTHING`.

PostgreSQL 16 + **TimescaleDB**: `samples` is a hypertable, compressed after 7 days, with a
`samples_1min` continuous aggregate carrying min/max. Retention drops raw at 2 years.

**Never use `locf()` or `interpolate()` on a physiological series.** Use `time_bucket_gapfill` so
gaps come back as explicit NULLs.

### Schema

```sql
PRIMARY KEY (source, type, session_id, seq)   -- stable ingest identity
timestamp   TIMESTAMPTZ NOT NULL              -- best-estimate event time, NOT the key
batch_id    BIGINT REFERENCES ingest_batch(id) -- NULL = did not arrive signed
```

- **`hlc_physical` and `hlc_logical` are removed.** `node_id` stays, as provenance.
- `seq` is a monotonic per-session index derived from whatever the source makes **stable across
  retransmission** — for a *streaming* source that is the sensor's own data, never host timing
  (amendment 05 §1.1). For a **confirmed store-and-forward** source it is the delivery index
  (amendment 06 §1.2). A source that is neither cannot be given a correct identity and must not be
  ingested. A retransmission reuses the same `seq` and dedupes correctly; a reconnect mints a **new
  `session_id`**, so there is no false dedup across a discontinuity.
- **`session_id` must be unique across process restarts.** A per-process counter from 0 makes a
  restarted collector's first rows collide with a previous run's and `ON CONFLICT DO NOTHING` drops
  them silently. Near-total for BP, where `seq` is 0–3 every time.
- `max(seq) - count(*)` per session is the **lost-beat count** — packet loss becomes measurable.
- **Never key on packet arrival time.** Several RR intervals arrive in one notification; anchoring
  them to arrival either collides (dropping real data) or shifts on retransmit (admitting
  duplicates). See amendment 04 §1 and `SYNTHESIS.md` A2.
- **`seq` derives from whatever the source makes stable across retransmission** (amendment 06 §1.2).
  For a streaming source that is the sensor's own data. For a **confirmed store-and-forward** source
  — the A&D cuff, which sends no timestamp, user id or counter — it is the delivery index, because
  the ATT confirmation makes redelivery impossible. A source that is neither confirmed nor
  payload-identifiable cannot be given a correct identity and **must not be ingested**.

### Durability

**`append { sync: true }` fsyncs before `vitalsd` replies** (amendment 06 §2). Used by
`bp-collector` and nothing else: BP frames are deleted by the device on the kernel's ATT
confirmation and are reconstructible from nothing, while a night of RR is ~250 MB and would be
destroyed by per-append fsync. Durability cost is paid only where loss is unrecoverable.

Every synced source also keeps a **raw-frame journal** written before decode, so a decoder bug is
replayable. A lost frame never is.

### Migrations

**Sqitch** in `db/`, three scripts per change: deploy, revert, **verify**. A change touching
`samples` must guard on the spool being empty — binary COPY freezes column count and order, so
migrating with a non-empty spool orphans buffered rows unrecoverably.

### Safety-path invariants

- **`OOMScoreAdjust=-1000`** plus `MemoryMin`/`MemoryHigh` on `vitalsd`, `klaxond`, `cns-engine` and
  every collector. Postgres and Docker get *positive* adjustments. Without this the kernel kills the
  alarm to save the database.
- **Every alarm output must be provably OFF from power-on until `klaxond` takes control** — in
  hardware, not by software timing. GPIO floats before configuration, so a 10 kΩ gate-to-source
  pulldown is mandatory on every MOSFET. This rule exists because false-alarm-on-boot has now been
  designed in three separate times, at three different layers.
- **Never let a storage or infrastructure fault fire the klaxon tone.** A failed LUKS unlock uses a
  distinct signal and escalates only after 15 minutes inside a monitoring window. An alarm that
  cries wolf is a broken alarm.
- **Self-tests must be able to fail.** ALSA accepting a write is not proof of sound; enroll the DAC
  by stable identity, disable USB autosuspend, assert a volume floor, and current-sense the siren.

### Alerting

Channels are a trait with declared dependencies. **Independence is reported in tiers** — peripheral,
process, kernel, host — because everything shares one CPU and one `klaxond`. Amendment 03's flat
"N independent domains" was false confidence.

A **555 hardware dead-man's switch** provides the only tier-2 independence. Its heartbeat is
**conditional on the whole chain being alive** — a monitoring window active, a fresh primary sample,
a recent tier evaluation, and drain progress — because a healthy `klaxond` pulsing while every
collector is dead is the worst false negative available. A shutdown inhibit keeps deliberate reboots
silent, and it fails safe (armed by default). Its test injects upstream faults; stopping the
heartbeat only tests the 555. See amendment 05 §2.

### UI direction

**No layout is chosen from mockups** (amendment 08). The admin console's M1 renders every screen
through all three design directions (Service Mode / Preflight / Flowpath) behind a `?dir=` +
cookie switcher; the web dashboard ports Night Report and Terminal onto the existing `model()`
seam with the same switcher, gated on the vitalsd query path. One view-model per screen, three
renderings, no direction-specific data logic. The direction determination is recorded after real
use, then the losing template sets are deleted. The bedside panel's shipped eight-view swipe
switcher is the precedent and needs no change.

### The admin console is not only a monitor

**An analysis run produces a clinical document, not a job status** (amendment 09). If a task brief
describes Analysis purely as queue states, elapsed time and a cost ledger, it is describing half the
feature. The artifact has a specified shape — severity and confidence as **separate mandatory axes**,
a closed registry-governed category per insight, supporting data, full provenance (model, tokens,
cost, raw exchange), and export for printing. **Coverage gaps are findings ranked among the clinical
ones**, never footnotes: the three-state law applies to a whole analysis window, not just a sensor.

**Records hold artifacts with lifecycles**, not prompt inputs. A conflict is a symmetric two-party
record that moves active → resolved.

**The console can enable and disable things** (new §4.11), and *deliberately off* is a distinct
render from *silently absent* and from *not configured*. **Anything running on emulated or synthetic
data surfaces on the Overview and is stamped on every analysis produced over that window.**

**Order changed:** Feature control and analysis **read-only** are M2; analysis **runs** are M3.
Analysis was M4.

### Tools adopted

`kani` (prove decoders panic-free) · `rppal` (GPIO) · `fs4` (spool locking) · `sqlx`
(compile-time-checked queries) · `metrics` (Prometheus) · `tokio` · `bluer` (BlueZ) ·
`tokio-postgres` with the `binary-copy` feature.

---

## Still open

All of `SYNTHESIS.md` §C is now specified in **amendment 05 §4** — drain liveness, disk-full, BLE
IRK pairing, off-box backup, spool fingerprint, spool locking, stuck-detector detection, gap-fill
enforcement, the 2 s p99 latency SLO, warm replay after restart, and subscriber backpressure.

**Specified is not implemented.** Track build status in `SYNTHESIS.md` §H.
