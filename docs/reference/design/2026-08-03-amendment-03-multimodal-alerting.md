# Amendment 03 — Multi-modal alerting

**Date:** 2026-08-03
**Status:** Proposed
**Amends:** design §5 (`klaxond`), T-A2, Stage 5
**Adds tasks:** T-K1 (channel abstraction) · T-K2 (GPIO siren) · T-K3 (tactile) · T-K4 (Zigbee) · T-K5 (out-of-band)

The original design has one alarm channel: a USB DAC into an amplifier and speaker. That is a single
point of failure for the only thing this system exists to do, and the multi-model review found the
self-test does not actually test it — ALSA accepting a write proves the software stack, not that a
sound occurred.

**Nothing here is required.** Every channel is optional and discovered at runtime. Use what you own.

---

## 1. The finding that reorders the design

From the fire-safety literature on waking sleeping people who cannot rely on sound:

> **Bed shakers with an intermittent pulse were 100% effective for all hearing levels, even from deep
> sleep.** Low-frequency alarms, bed shakers and/or pillow shakers are the most reliable existing
> technologies for awakening people who are deaf or hard of hearing.
> — *Alarm Technologies to Wake Sleeping People Who are Deaf or Hard of Hearing*, Fire Technology

This matters beyond hearing loss. The event being detected is **CNS depression — arousal itself is
impaired.** That is functionally the same problem as an alarm that cannot be heard: the stimulus must
reach a nervous system that is less responsive than usual.

**A bed shaker is physically coupled to the sleeper.** It does not depend on air, room acoustics, a
closed door, earplugs, or a speaker cone that has not been tested since installation.

**Consequence: tactile becomes the primary channel and sound becomes secondary.** The original design
had that backwards, and it had it backwards because a phone can only make noise.

---

## 2. Channels, and what each one depends on

Redundancy is only redundancy if the dependencies are **disjoint**. Two channels that both need USB
are one channel.

| Channel | Depends on | Independent of |
|---|---|---|
| **Bed shaker** via relay | GPIO, relay, 12 V supply | USB, ALSA, network, mains-switched devices |
| **GPIO piezo siren** | GPIO, MOSFET, 12 V supply | **the entire audio stack** |
| USB DAC + amp + speaker | USB, ALSA, DAC firmware, card enumeration, amp power | GPIO |
| **Zigbee lights** (Hue/other) | USB dongle, zigbee2mqtt, bulb mains power | ALSA, internet, vendor cloud, hub |
| Hue via Bridge | + bridge, ethernet | GPIO |
| LIFX / WiFi bulbs | WiFi, LAN | GPIO, USB |
| APNs push | internet, Apple, phone | everything local |
| **Voice call** (Twilio) | internet **or LTE modem**, carrier | home network if LTE |
| Watch haptic | phone, watch, BLE | mains power |

### 2.1 The piezo siren answers a review finding directly

Two reviewers independently flagged that the klaxon self-test cannot prove sound was produced, and
deepseek found the specific mechanism: a USB DAC that resets gets a **new ALSA card number**, and
`/etc/asound.conf` from T-A2 hardcodes card *N*. ALSA keeps accepting writes — to the wrong device.

**A GPIO-driven piezo siren bypasses that entire class of failure.** No USB, no ALSA, no card
enumeration, no DAC firmware, no mixer state. A pin, a MOSFET, and a 12 V sounder. It cannot produce
a pleasant tone and it does not need to.

It is also *self-testable in a way the DAC is not*: drive the pin briefly and sense current draw on
the load. Current flowed or it did not — which is closer to "a sound happened" than any software
acknowledgement.

---

## 3. Architecture — pluggable channels

```rust
/// One way of getting a human's attention. Implementations are discovered at
/// startup; absent hardware is absent, never an error.
trait AlertChannel {
    fn id(&self) -> &str;
    fn modality(&self) -> Modality;      // Tactile | Sound | Light | OutOfBand
    /// What this channel needs in order to work. Used to compute whether the
    /// configured set is actually redundant, rather than assuming it is.
    fn dependencies(&self) -> &[Dependency];

    fn probe(&self) -> Availability;      // Present | Absent | Degraded(reason)
    fn fire(&self, tier: Tier) -> Result<()>;
    fn silence(&self) -> Result<()>;
    /// Must be able to fail. A self-test that cannot fail tests nothing.
    fn self_test(&self) -> TestOutcome;
}
```

`klaxond` fires **every available channel** on `klaxon`, and a configurable subset at `confirm`.

### 3.1 Independence is computed, not assumed

At startup and on every self-test, `klaxond` groups the available channels by dependency set and
reports the count of **independent** groups — not the count of channels.

Three channels that all require USB report as **one** independent path, and the admin console says
so. "You have 4 alert channels" is comfortable and possibly false; "you have 4 channels across 2
independent failure domains" is a fact you can act on.

Below two independent domains, the system reports **degraded** and names what is missing.

---

## 4. Hardware options

### 4.1 Tactile — the primary channel

| Option | Interface | Notes |
|---|---|---|
| Bed shaker (Sonic Alert, Bellman & Symfon, generic) | 12 V barrel or 3.5 mm | Relay or MOSFET on GPIO. **Intermittent pulse, not continuous** — that is what the research measured |
| Pillow shaker | same | Weaker coupling; better if a partner should not be woken |
| Under-mattress transducer | 12 V | Stronger, more expensive |

Drive it with a GPIO relay HAT, a bare MOSFET, or a Zigbee/Z-Wave smart plug. Prefer GPIO: a smart
plug adds a radio and a hub to the dependency set of the most important channel.

### 4.2 Light

| Option | Interface | Notes |
|---|---|---|
| **Zigbee bulbs incl. Hue** via USB coordinator | USB (Sonoff ZBDongle-E, ConBee II, SLZB-06) | **No bridge, no cloud, works with the internet down.** Also brings smart plugs and sensors |
| Hue via Bridge | Ethernet + local API | Use if you already have a bridge; adds a device to the chain |
| LIFX / WiFi bulbs | LAN | No hub, but depends on WiFi |
| 12 V strobe on a relay | GPIO | Cheapest, brightest, most independent; "strongest option" per the literature |

**For light as an *alarm*, go bright and cold** — the opposite of the bedside panel's warm-dim design.
The panel is ambient and must not disturb sleep; the alarm exists to end it. Full brightness,
blue-enriched, with a hard flash pattern.

A Zigbee coordinator on the Pi is the standout: one $25 dongle gives local control of Hue and most
other Zigbee devices with **no bridge, no vendor cloud, and no internet dependency** — which matches
this project's "must work when the network is down" requirement, unlike any cloud-connected bulb.

### 4.3 Out-of-band — for when you do not wake

| Option | Notes |
|---|---|
| APNs push | Already designed. Depends on internet + Apple |
| **Voice call** (Twilio or similar) | Rings through Do Not Disturb when set as an emergency bypass contact. Uses the **carrier network**, not your WiFi |
| LTE HAT / USB modem | Makes the voice path independent of home internet entirely |
| SMS/call to a second person | The real safety net — see §5 |

---

## 5. Escalation and acknowledgement

Multi-channel alerting needs an answer to "what if none of it works," and that answer is a person.

```
klaxon fires  ──▶  all channels
      │
      ├─ acknowledged (hold-to-silence on the panel, 3 s)  ──▶  silenced 10 min, tier held at
      │                                                          CAN'T ASSESS, never CLEAR
      │
      └─ NOT acknowledged after N minutes  ──▶  escalate: voice call to a contact
```

The unacknowledged path is the point. If the alarm is firing and nobody silences it, the most likely
explanations are that it is working and the person cannot respond, or that every local channel has
failed. Both warrant a human.

Escalation contact, delay, and whether it is enabled are all configuration — this is somebody's
bedroom, not a pager rotation.

---

## 6. Self-test — per channel, and it must be able to fail

The existing daily self-test extends to every channel:

| Channel | What "tested" means |
|---|---|
| Bed shaker | Pulse briefly, sense current draw on the load |
| Piezo siren | Same — current flowed or it did not |
| USB DAC | ALSA accepted the write **and** the card number still matches the one enrolled at setup |
| Zigbee light | Round-trip a state change and read the reported state back from the device |
| APNs | Silent push, confirm delivery receipt |
| Voice | Monthly, not daily |

Results are per-channel and per-modality. A green summary while the tactile channel has been dead for
three weeks is exactly the failure this project exists to prevent, so the console shows the **oldest
passing test per modality**, not an aggregate.

The DAC card-number check closes the specific hole the review found.

---

## 7. Tasks

| Task | Tier | Host | Notes |
|---|---|---|---|
| **T-K1** — `AlertChannel` trait, discovery, independence computation, per-channel self-test | implement-complex | either | Pure logic; testable on macOS |
| **T-K2** — GPIO piezo siren + current-sense self-test | implement-standard | pi | Highest value per pound; independent of the audio stack |
| **T-K3** — tactile channel: relay driver, intermittent pulse pattern | implement-standard | pi | **Primary channel** per §1 |
| **T-K4** — Zigbee coordinator + light channel | implement-standard | pi | Local, no bridge, no cloud |
| **T-K5** — out-of-band: voice call, escalation timer, unacknowledged path | implement-complex | pi | Needs a provider account |

**Suggested order: T-K1, T-K2, T-K3, then the rest.** The siren and the shaker are cheap, wired to
GPIO, and independent of everything already fragile. Together they give two independent failure
domains before any radio or network is involved.

---

## 8. Shopping list, if buying

Nothing here is required, and any subset works.

| Item | ~Cost | Buys |
|---|---|---|
| 12 V piezo siren + MOSFET/relay | $10 | A sound path that owes nothing to USB or ALSA |
| Bed shaker (12 V) | $20–40 | The channel the research says is 100% effective from deep sleep |
| GPIO relay HAT (2–4 ch) | $10 | Drives both of the above |
| 12 V PSU | $10 | Powers both |
| Zigbee USB coordinator | $25 | Hue and friends, locally, no bridge, no cloud |
| 12 V strobe | $15 | The strongest visual option, on the same relay board |
| LTE USB modem | $30+ | Makes the out-of-band path independent of home internet |

**~$65 covers siren + shaker + relay + PSU**, which is the single largest reliability improvement
available to this system, and less than the difference between two of the HSM options.
