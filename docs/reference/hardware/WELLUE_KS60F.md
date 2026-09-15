# Wellue / Viatom KS-60F-family fingertip oximeter

**Status:** identified; BLE protocol and live integration pending hardware verification.

## Identity evidence

The unboxing photographs and Amazon listing supplied with this work identify the device as:

- Brand: Wellue; technology branding: Viatom
- Manufacturer: Shenzhen Creative Industry Co., Ltd.
- Exact model printed on the device label: `KS-60FWB`
- Model family printed on the user manual: `KS-60F / KS-60FW / KS-60FB / KS-60FWB`
- FCC ID printed on the box: `A49PC-60FW`
- Power: two AAA batteries
- Displayed signals visible in the photographs: SpO₂, pulse rate (PR), and perfusion index (PI)
- Built-in memory: the listing says up to 12 groups of oxygen, pulse, and perfusion readings
- Companion software: the listing says Android/iOS app synchronization while the app is open

The photographs are sufficient to identify the exact hardware model and its advertised signals.
They do not reveal the device Bluetooth address or confirm the live GATT service/characteristic
layout. The latter must be measured from the actual device; it must not be inferred from the
existing EMAY implementation.

The listing also says that this product is for sports or aviation use and is not a medical device.
AnxietyWatch2 must therefore present readings as observational data and must not imply diagnosis,
treatment, or a medically validated overdose decision.

## Integration boundary

The registry contains source `wellue_ks60fwb` (stable id 4) with `spo2`, `hr`, and
`perfusion_index`. Independent PC-60F-family protocol references describe the Nordic UART Service
and PI as a one-byte value scaled by 0.1 percent. AnxietyWatch2 now has a provisional pure decoder
for that representation; a live capture from this exact KS-60FWB must still confirm it before the
signal is allowed to influence safety alerting.

The provisional collector may store observational Wellue rows with provenance metadata, but no
Wellue reading may influence a safety decision until all of these are known and tested:

1. GATT service and characteristic UUIDs;
2. whether the device streams without a vendor command;
3. packet framing, checksum, and absent/no-finger values;
4. whether timestamps and PI are present, and whether the provisional PI units match;
5. reconnect and memory-download behavior; and
6. agreement between decoded values, the device display, and (where available) the companion app.

## First hardware session

Use the isolated `wellue-probe` binary. It performs discovery, connects, inventories GATT, and can
capture raw notifications without guessing a protocol or writing commands to the device.

On the Pi, from the repository checkout:

```sh
cargo build --release -p wellue-probe
sudo btmgmt find
WELLUE_MAC='<address>' ./target/release/wellue-probe
```

Record the service/characteristic inventory in the setup log. Once a notifiable characteristic is
identified, capture raw bytes for a short, repeatable run:

```sh
WELLUE_MAC='<address>' \
WELLUE_NOTIFY_UUID='<characteristic UUID>' \
WELLUE_RAW_LOG="$HOME/vitals/wellue/raw-$(date -u +%Y%m%dT%H%M%SZ).log" \
WELLUE_CAPTURE_SECONDS=120 \
./target/release/wellue-probe
```

Do not pair the device or send vendor commands until the inventory is recorded. The app may be
useful later for comparison, but it is not evidence of the Pi wire protocol. The provisional
collector expects the NUS TX UUID above and sends no device commands:

```sh
sudo install -m755 target/release/wellue-collector /usr/local/bin/wellue-collector
sudo install -m644 deploy/wellue-collector.service /etc/systemd/system/wellue-collector.service
sudo install -d -m750 /etc/anxietywatch
printf 'WELLUE_MAC=%s\\n' '<address>' | sudo tee /etc/anxietywatch/wellue.env >/dev/null
sudo systemctl daemon-reload
sudo systemctl enable --now wellue-collector
```

The service writes samples through `vitalsd`, syncs each raw notification to the journal before
parsing, and publishes a status snapshot for the kiosk. It is deliberately marked provisional in
metadata and must not be used to claim a validated alarm until T-W2 live verification is complete.
For offline testing, `./scripts/wellue-replay-test.sh` exercises the decoder, socket, and spool with
explicitly marked synthetic rows; it does not enable the collector or any alarm.
The current kiosk live status path displays the Wellue identity and PI when pointed at this status
file, while retaining `NOT ARMED` / `CAN'T ASSESS` rather than inventing a safety tier.

## Safety-path policy

SpO₂ and pulse may become corroborating inputs to an anxiety/respiratory-depression warning only
after the signal quality, gap behavior, and alert policy are tested. A missing, stale, invalid, or
unverified reading is **not measured**; it is never rendered or scored as reassuring. A normal
oximeter reading does not rule out overdose or respiratory depression. Suspected overdose remains an
emergency requiring the established emergency-response plan, not this device alone.

## Completion checklist

- [x] Exact model `KS-60FWB` and device label recorded; the serial number is intentionally not retained.
- [ ] GATT inventory captured from the exact KS-60FWB.
- [ ] Raw notification capture retained and checksummed from live hardware.
- [x] Provisional decoder implemented as pure `no_std` Rust with malformed/absent-value tests and a fuzz target.
- [ ] PI semantics and range confirmed against live display/app values before safety use.
- [x] Collector uses generated `VitalsSource::WellueKs60fwb` and canonical registry units.
- [x] Append-only raw journal and durable ingest path implemented; live crash/restart testing remains.
- [ ] Reconnect, no-finger, stale-link, and battery behavior tested.
- [ ] Device setup exposed in admin as configured / deliberately off / silently absent / not measured.
- [ ] User-facing setup guide updated from the dated setup log.
