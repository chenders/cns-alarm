# EMAY Overnight Pulse Oximeter

High-fidelity overnight SpO₂ and pulse rate from an [EMAY SleepO2](https://emayinc.com/) wrist or finger pulse oximeter. In AnxietyWatch2, `emay-collector` owns the Linux Bluetooth LE connection through BlueZ, sends approved stored-session rows through `vitalsd` and the COPY spool to PostgreSQL, and publishes only the current anonymous live stream to the bedside status path.

> **Verification needs hardware.** The `vitals-decode` crate is pure Rust and fuzz-tested, but full integration testing requires an actual EMAY device's BLE connection.

---

## What it is

EMAY oximeters are an inexpensive alternative to a sleep-lab study for capturing overnight desaturation patterns. They emit a continuous stream of SpO₂ and pulse readings over a custom UART-over-BLE GATT service.

`emay-collector` connects to the EMAY device via BlueZ, buffers the incoming UART chunks, and hands them to the pure decoders in `vitals-decode`. Stored-session readings with device-provided identity are sent to `vitalsd`; `vitalsd` writes the existing PostgreSQL binary-COPY spool, and the spool drainer appends them to the PostgreSQL `samples` table. Anonymous live `EB` frames update current display/liveness state but are not durable rows.

## Data Flow

```
EMAY SleepO2 (BLE UART TX)
       │  Stored EC/ED records; anonymous live EB frames
       ▼
emay-collector (BlueZ / tokio)  ──── Connects, pages storage, handles live status
       │
       ├── vitals-decode           ──── Pure decoding and validation
       │
       ├── vitalsd append socket   ──── Single writer boundary
       │
       └── vitals-core/COPY spool  ──── Buffers and deduplicates
                    │
                    ▼
PostgreSQL (`samples` table)     ──── Append-only UTC storage; panel history source
```

### Durable identity and live state

The official device-storage mode supplies the identity used for durable rows:

- `EC` identifies a stored session and its device recording start;
- the complete `EC` descriptor plus storage slot determines `session_id`;
- each `ED` record supplies the metric selector, slot, and device cursor;
- the source-provided `ED` cursor is `seq` and remains stable when the same
  stored record is downloaded again;
- event time is derived from the device session start and storage ordinal, then
  converted once to UTC.

The real-time `EB 01 05` notification has no sensor counter, timestamp, or
record ID. It is therefore used for current display and liveness only. It must
not be assigned a collector delivery sequence or inserted into PostgreSQL.
The exact checksum-valid `EB 01 07 7f 7f 7f 00 70` frame is no-reading/link
evidence, not a numeric value. The `0x7f` no-reading sentinel contributes no
numeric row.

### Pure Decoding

The protocol is binary. `vitals-decode` handles byte-parsing with the following constraints:

- **No I/O or allocation:** The decoder takes `&[u8]` and returns an `Option` or `Result`.
- **Safe indexing:** `get()` is used exclusively to avoid panics on malformed or truncated packets (`indexing_slicing` is denied).
- **Missing data handling:** If the device loses contact or reports no reading, the value is discarded rather than stored as zero. A zero SpO₂ reading is physically impossible and would corrupt clinical statistics.
- **No clamping:** Out-of-range values are rejected and named; they are not converted into plausible readings.

### Crate boundaries

- **`vitals-registry`**: Defines the source ID (`VitalsSource`) and metrics (`VitalsType` for SpO₂ and HR) used by EMAY.
- **`vitals-decode`**: Contains the pure protocol parser.
- **`emay-collector`**: Handles the Linux-specific BlueZ connection, stored-session paging, and live display status.
- **`vitalsd`**: Remains the single append boundary for PostgreSQL-backed samples.

---

## Known limits and hardware caveats

- **Auto power-off:** The device typically powers off after roughly two minutes without a BLE connection. If the link drops during the night and is not quickly recovered, the recording session may terminate entirely.
- **Coexistence:** A dedicated CSR8510A10 USB BLE dongle with an external antenna is strongly recommended to prevent drops caused by Raspberry Pi onboard WiFi/BT coexistence issues.
- **Single host:** The EMAY connects to only one host at a time. While `emay-collector` holds the connection, the EMAY companion mobile app cannot connect. Opening the app during Pi collection can make the device stop advertising to the Pi or take the connection away.
- **Unpaired BlueZ object:** The deployed unit is unpaired and unbonded. After a Pi/BlueZ reboot, the configured address alone may not recreate the transient BlueZ device object. In that state `bluetoothctl info` reports `not available` and the collector reports `the target object was either not present or removed`; address-only retries cannot succeed until the device is discovered again.
- **Charging and LED state:** USB charging may affect power/LED behavior, but no vendor LED specification was found. A recovery observation found green while the device was absent from BlueZ and blue after the BLE object was discovered and connected. Disconnecting the finger sensor also changed the LED, so color is supporting evidence only—not a reliable Bluetooth or charging diagnostic.

## Current deployment and recovery

The approved production path is:

- `vitals-emay.service`: enabled durable collector;
- `emay-live.service`: disabled/inactive legacy display-only collector;
- `SleepO2-2_1110`: the configured EMAY in the current deployment;
- `emay_status.json`: current live status, with sensor freshness evaluated from
  `sample_at`, not merely from file modification time;
- PostgreSQL `samples`: authoritative source for persisted EMAY panel history.

If the durable unit is retrying while BlueZ says the known EMAY object is not
available, check H10 and the shared services first, keep the vendor app
disconnected, and inspect the physical sensor. If explicitly authorized,
perform one short bounded discovery interval, stop discovery immediately after
`SleepO2-2_1110` appears, and let the durable collector retry. Verify
`Connected: yes`, resolved FF12/FF01/FF02 services, an active collector, a fresh
`sample_at`, `link=up`, and the watcher health result. Do not start
`emay-live.service` beside the durable collector or use LED color as the sole
proof of live data.

The complete incident evidence and command results are recorded in
[`docs/status-updates/2026-09-05T08-09-24Z-emay-recovery.md`](../status-updates/2026-09-05T08-09-24Z-emay-recovery.md).

---

## Future work

- Supporting off-device file transfer: If multi-hour BLE streaming proves unstable in some RF environments, adding support for pulling the onboard memory after the session (similar to the older CSV share-sheet import approach) could act as a fallback.
- A future operational improvement could make discovery of an unpaired device explicit and bounded in the collector's deployment contract. It must preserve the no-automatic-recovery policy and must not weaken the source-stable identity rules above.
