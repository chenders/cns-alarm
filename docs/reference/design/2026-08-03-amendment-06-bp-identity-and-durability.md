# Amendment 06 — BP identity from the transport, durability at the ACK boundary

**Date:** 2026-08-03
**Status:** Proposed
**Supersedes:** amendment 05 §5 (the T-D6 row) and **§1.1's identity rule, which it narrows** ·
extends §4.3 · restates the durability policy A01 set for the spool
**Evidence:** `docs/research/2026-08-03-ua651ble-ble-capture.md` — a live capture from the hardware
**Affects:** T-D3, T-D4, T-D6, T-B1

Amendment 05 §5 instructed T-D6 to take `seq` "from the cuff's own measurement sequence, not
arrival." The first live capture proves no such sequence exists. This corrects that row, adds the
device state the capture exposed, and specifies the durability interface T-D6 needs but cannot
build against the current socket protocol.

---

## 1. The A&D UA-651BLE transmits no sequence, and no timestamp (supersedes A05 §5, T-D6 row)

Every frame in the capture carried `flags = 0x14`:

```
bit0 units            0  → mmHg
bit1 timestamp        0  → ABSENT
bit2 pulse rate       1  → present
bit3 user id          0  → ABSENT
bit4 measurement status 1 → present
```

No timestamp, no user id, no counter. Four consecutive readings are byte-distinguishable only by
their values, and two identical readings would be byte-identical. **There is nothing in the payload
to derive `seq` from.**

### 1.1 Identity comes from the transport, and that is sound here

`0x2A35` is an **indication**, which requires an ATT Handle Value Confirmation. The snoop shows four
indications and four confirmations. A&D's documented behaviour is that a record is cleared once
confirmed; the capture is consistent with that but does not prove it, because proving it requires
observing an *unconfirmed* indication and its later redelivery.

**This is at-least-once, not exactly-once.** An earlier draft of this section claimed exactly-once
and that was wrong — the confirmation PDU can itself be lost to a supervision timeout, or the kernel
can confirm and the link drop before the peripheral processes it, leaving the cuff holding a record
it redelivers on the next connection. Because `session_id` is minted per connection, that redelivery
lands under a *different* key, so `ON CONFLICT DO NOTHING` cannot collapse it. And §1 has already
established that two identical readings are byte-identical, so post-hoc dedup is impossible in
principle and would drop legitimately repeated readings if attempted.

The consequence is bounded and must be stated rather than hidden: **a lost confirmation produces a
duplicate BP row, not a lost one.** For this signal that is the right direction to fail — a
duplicated reading is visible and correctable, a dropped one is neither — but the health endpoint
must surface same-value-adjacent-session pairs so the rate is measurable rather than assumed.

This is why the profile mandates indication rather than notification, and it is a better reason than
the one previously recorded: `0x0001` would not merely deliver nothing — even if it delivered, the
cuff could never safely clear its buffer without confirmations.

**`seq` for this source is the drain index within one connection**, with `session_id` minted per
connection. This is bookkeeping for ordering, not a dedup mechanism.

### 1.2 The A05 §1.1 invariant is narrowed, not broken

A05 §1.1 says `seq` is "derived from the sensor's own data, never from host timing." That rule was
written for **streaming** sources — the Polar H10, where several RR intervals arrive per notification
and host arrival is genuinely unstable as an anchor. It holds unchanged there.

For a **store-and-forward** source that delivers under confirmation, host-observed delivery order is
not an unstable anchor: it is the same order the device dequeued, and each record is delivered once
by protocol guarantee. The invariant is therefore restated with its scope made explicit:

> `seq` must be derived from whatever the source makes **stable across retransmission**. For a
> streaming source that is the sensor's own data. For a confirmed store-and-forward source it is the
> delivery index, because the confirmation makes redelivery impossible.

A source that is neither — unconfirmed and payload-anonymous — **cannot be given a correct identity
and must not be ingested** until it can. Say so rather than inventing one.

### 1.3 The exactly-once guarantee has a hole, and it is not in the protocol

The confirmation is emitted by the **kernel**, before the application has stored anything. Between
the ACK and a durable write the record exists nowhere else — the cuff has already deleted it. See §2.

**There are two holes at the ACK boundary, not one**, and they point in opposite directions:

| Direction | Hole |
|---|---|
| Toward the application | The kernel confirms before this process has stored anything. A crash in that window loses the reading, and the cuff has already deleted it. |
| Back toward the device | The confirmation PDU can be lost, so the cuff may retain and redeliver a record the host already has (§1.1). |

An earlier draft named only the first, which let the ACK be treated as a weak signal when reasoning
about loss and a strong one when reasoning about identity, two paragraphs apart. It is weak in both
directions. T-D6 must state both.

### 1.4 Timestamps: unresolved, and worth resolving

`timestamp` for this source is host arrival, which A05 §1 explicitly permits for the event-time
column (it is an estimate and is not identity). But it is a poor estimate: the cuff may have held the
reading for hours.

**Open, and assigned to T-D6 step 5:** A&D devices expose a clock. If writing Date Time (`0x2A08`) or
the vendor service `233bf000-5a34-1b6d-975c-000d5690abe4` causes `flags` bit1 to set, measurements
gain true event times and this degrades from "estimate" to "correct." Record the result either way —
a negative result is worth as much, because it closes the question.

Until then, a BP sample's `timestamp` is **arrival**, and that must be visible in the data rather
than implied: set `extra` to record that the event time is an arrival estimate, so a later consumer
cannot mistake it for a device timestamp.

---

## 2. Durability at the ACK boundary (new; T-D3, T-D4, T-D6)

Every other source in this system is reconstructible — HealthKit re-reads, sensors re-derive, the
restore path replays. **A confirmed-and-deleted BP reading is not.** It is the `QuantityHealthSample`
hole in the previous project, arriving from a different direction: app-only data with nothing
upstream to rebuild it.

### 2.1 The interface, because T-D6 could not have built this

The T-D6 brief and the research doc's finding 2 required `bp-collector` to "fsync the spool segment
on every frame" — note that **A05 itself contains no such instruction**; an earlier draft of this
section attributed it there. That instruction is unbuildable against the current design:

- `vitalsd` is the single writer and owns the spool (T-D4). A collector's only channel is the CBOR
  socket, whose verbs are `append`, `query`, `subscribe`, `health` — none carrying a durability flag.
- T-D3 sets the opposite policy: *"fsync per batch or every 250 ms"* and *"a collector must never
  block on disk to deliver a sample."* That policy originates in **amendment 01 §5**, not in T-D3,
  so this amendment restates A01's durability rule rather than merely amending a brief.

A worker following the brief would either write the spool directly from the collector — breaking the
single-writer property that T-D4 calls "the whole reason for the design" — or invent a protocol
extension. So the protocol gains one:

```
append { ..., sync: true }
```

- **`sync: true`** — `vitalsd` fsyncs the spool segment **before** replying. The collector's `append`
  does not return until the bytes are durable.
- **Default `false`** — unchanged behaviour: batched, fsync per batch or every 250 ms.
- `bp-collector` sets it. The RR collector does **not**: a night of RR is ~250 MB and roughly 100k
  appends, and fsyncing each would destroy the ingest path to protect data that is already
  reconstructible from the sensor's own buffer.

The asymmetry is the point. Durability cost is paid where loss is unrecoverable and nowhere else.

### 2.2 Ordering at the collector

`bp-collector` must `append` with `sync: true` **and receive the reply** before the frame is
considered handled. It cannot influence when BlueZ emits the ATT confirmation — that already
happened in the kernel — so this does not close the window; it makes it as small as the design
allows and, more importantly, makes the loss *visible*: an `append` that fails or times out is a
frame the collector knows it lost, and it says so rather than continuing quietly.

**A raw-frame journal is also required.** Before decoding, `bp-collector` appends the raw bytes and
their arrival time to an append-only journal, fsynced. A decoder bug is then recoverable by replay; a
lost frame never is. This is cheap at four frames a day and it is the only thing that makes a
decoding defect survivable for this source.

---

## 3. A third device state: bonded but unable to resume encryption (extends A05 §4.3)

A05 §4.3 distinguishes two states — "device not found" (a pairing fault) and "device out of range"
(normal). The capture found a third:

```
kernel 6.12.96: Bluetooth: hci0: unexpected SMP command 0x0b from <cuff>
```

SMP `0x0B` is a Security Request: a bonded peripheral asking the central to resume encryption from
the stored LTK. This kernel rejects it as arriving in an invalid state, so encryption never starts,
GATT never opens, and no data flows. The bond was intact — `[LongTermKey]`,
`[PeripheralLongTermKey]`, `[IdentityResolvingKey]` and `[ConnectionParameters]` were all present.

**This is indistinguishable from a healthy idle system.** The device is bonded, trusted, and
periodically connecting. Nothing errors at the application layer. It presents as "no measurements
taken," which is also what a normal quiet night looks like.

**Required:** a named degraded state, detected as *"a bonded device reached `Connected` but
`ServicesResolved` never fired within 10 s."* It names the device and the elapsed time, and the
documented recovery is remove-bond-and-re-pair, which sidesteps the code path entirely — a fresh
pairing establishes encryption as part of the pairing procedure and never sends a Security Request.

This belongs on the health endpoint and it feeds the same "can't assess" path as a sensor gap. A
collector that is bonded and connecting is **not** evidence that data is arriving.

---

## 4. Task changes

| Task | Change |
|---|---|
| **T-D3** | `Spool::append(rows, sync)` — the API had no fsync to call, so T-D4's instruction was unbuildable through it. Also: `session_id` unique across restarts |
| **A01 §5** | Its "never block on disk" rule gains the `sync: true` exception. Unchanged for streaming sources |
| **T-D4** | `append` gains `sync: bool`; `vitalsd` fsyncs before replying when set. Document it in `Interfaces produced` |
| **T-D6** | `seq` = drain index (§1.1); `sync: true` on append (§2.1); raw-frame journal (§2.2); bonded-but-unencrypted degraded state (§3); clock-set investigation (§1.4) |
| **T-B1** | A05 §4.3 unchanged; add the §3 state to its connection handling |

## 5. What this does not change

The narrow-waist store, the registry, Rust, TimescaleDB, the spool, sqitch, signed ingest, and the
alerting abstraction all stand. A05 §1's separation of identity / event time / observation time
stands and is what makes §1.1 expressible at all — `timestamp` can be an arrival estimate precisely
because it is not the key.
