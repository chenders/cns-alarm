# Technical Handoff: Communicating Directly With a ResMed AirSense 11 — BLE Reality vs. the CAN-Bus Path

## TL;DR
- **The single most important finding: the AirSense 11's Bluetooth Low Energy interface is NOT a viable control/data channel.** BLE on the AS11 is used only for phone pairing, WiFi/account provisioning, and the "Test Drive" pressure-trial feature; actual therapy data egresses over the built-in **cellular modem** to ResMed's cloud.
- **The community's working interface into the AS11 is the CAN bus on the 24 V power connector, not Bluetooth.** The actively developed `m-kozlowski/aircannect` ESP32 bridge talks JSON-RPC over CAN at 1 Mbit/s and can read/write all therapy settings.
- **Because of this, AnxietyWatch2 utilizes the EzShare SD card method (hms-cpap) to ingest therapy data.** Do not attempt to use `vitalsd` to connect to the AS11 over BlueZ. It will not yield therapy data.

## Key Findings

1. **BLE is a provisioning channel, not a therapy channel.** ResMed's own materials and multiple resellers confirm the AS11's Bluetooth is used to pair the phone without typing the serial number and run Test Drive. Therapy/compliance data reaches myAir indirectly via cellular.

2. **No published AS11 GATT map.** Despite extensive community CPAP tooling, no nRF Connect scan dump or decompiled service UUID table for the AS11 surfaced that provides access to therapy data.

3. **ResMed applies real BLE security.** Application-level encryption for the transmission of data means even a successful GATT connection likely faces an encrypted/authenticated payload, not cleartext commands.

4. **The AS10 vs AS11 connectivity difference is fundamental.** The AirSense 10 has no user Bluetooth at all. Both machines write the same OSCAR-compatible EDF data structure to the SD card, so **the SD card remains the universal, reliable data path for both**.

5. **A working direct-communication project exists — over CAN, not BLE.** `m-kozlowski/aircannect` (an ESP32-S3 bridge) reads/writes all therapy settings, streams live pressure/flow/leak/SpO2, and provides a raw TCP bridge over the CAN bus exposed on the AS11 power connector.

## Recommendations

If the objective is to *read therapy data* on your AS11 for AnxietyWatch2, the BLE route is the wrong tool: use the **EzShare SD Card `hms-cpap` approach** (proven, zero reverse engineering) to import data into the PostgreSQL `samples` table.
