# Form-factor options

This project should not assume that a bedside wearable is the right physical
shape. For people who may be homeless, mobile, alone, exposed to weather, or
protective of their belongings, comfort, dignity, retention, charging, theft,
and ease of replacement are safety requirements.

The options below are engineering candidates, not endorsements or medical
claims. Open hardware documentation makes repair and audit easier; it does not
make a sensor physiologically validated.

## Recommended starting shape: detachable belt/pendant alarm node

A small sealed node worn on a belt, lanyard, bag strap, or clothing clip is the
most practical base station:

- protected battery, processor, accelerometer, watchdog, buzzer/vibration motor,
  status LED, and local storage;
- no screen required for the alarm path;
- replaceable or separately chargeable sensor modules;
- easy to move between sleeping and walking use;
- large tactile acknowledge/test control that cannot be confused with power off;
- external alarm hardware can be tested without opening the enclosure.

A lanyard must include a breakaway safety feature. A belt clip must not be the
only retention method. The node needs a rugged enclosure, splash/dust target,
strain relief, protected charging connector, battery fuel gauge, and a known
OFF alarm state during boot.

## Candidate sensor/node forms

| Form | Strengths | Risks / open work | Open-hardware direction |
|---|---|---|---|
| **Belt or pendant node** | Best battery and alarm volume; room for IMU and watchdog; accessible for charging | Theft/loss, snagging, weather, body movement differs from chest | nRF52840 or RP2040-class board with published schematic; custom enclosure and external dead-man circuit |
| **Chest strap module** | Strongest motion/respiration context; stable placement | Intimate, uncomfortable, hard to fit, sweat and removal, can be rejected by wearer | Use a replaceable open-schematic sensor pod; do not infer respiration until placement validation |
| **Wristband** | Familiar and socially acceptable; useful for motion and pulse | Poor SpO₂ contact during sleep, loose fit, charging burden, false motion artifacts | PineTime-class/open-schematic wearable as a development platform; sensor quality still requires validation |
| **Upper-arm band** | More stable than wrist; larger battery than wrist | Less familiar, clothing interference, difficult oximeter geometry | Custom open-schematic band with detachable optical sensor; requires exact placement study |
| **Adhesive patch** | Stable contact and low motion | Skin irritation, replacement cost, sweat, inability to recharge, consent concerns | Open flex-PCB designs are possible but not an initial target; needs biocompatibility and secure disposal work |
| **Ear/temple sensor** | Potentially good pulse/oxygen signal and low body motion | Intrusive, hygiene, sleeping position, loss, cold-weather effects | Research only; no use without exact optical validation |
| **Bedside puck** | Large speaker, battery, and robust enclosure; simple for sheltered sleeping | Not portable; cannot distinguish wearer from nearby person; loss of contact is ambiguous | Good indoor accessory, not the sole alarm node for unsheltered use |
| **Backpack/bag node** | Large battery and weather protection | Bag may be removed or stolen; sensor placement becomes unknown | Secondary enclosure only; must report placement unknown |

## Open-hardware development platforms

These are useful for prototyping the alarm node and accelerometer path because
schematics, board files, or hardware documentation are publicly available.
They are not automatically suitable for participant deployment:

- **PineTime**: open hardware/software wrist platform; useful for UI, battery,
  vibration, and IMU experiments. Verify the exact board revision, sensor
  availability, and Linux/Rust toolchain before choosing it.
- **Adafruit Feather nRF52840 Sense / similar nRF52840 Sense boards**: practical
  BLE + IMU development platforms with public design documentation. Treat their
  battery charger, enclosure, and connector as prototype components until
  field-tested.
- **Arduino Nano 33 BLE Sense Rev2**: public schematic and an integrated IMU;
  useful bench hardware but not rugged, waterproof, or a finished wearable.
- **SparkFun nRF52840 and RP2040 development boards**: good for early firmware
  and sensor experiments where open schematics are available; generally need a
  custom battery, watchdog, charger, enclosure, and alarm board.
- **MAX30101/MAX30102-based optical boards**: open schematics are common and can
  accelerate optical experiments, but breakout-board readings are not evidence
  of validated SpO₂ performance. Optical geometry, LED current, ambient light,
  skin tone, perfusion, temperature, and motion must be characterized.
- **Open-source IMU breakout boards** using LIS2DH12, LIS3DH, BMI270, or similar:
  useful for motion experiments. The selected part must be frozen, its driver
  audited, and its interrupt/power behavior tested under battery depletion.

“Open source hardware” must be checked against the actual revision: published
firmware alone is not a published schematic, and a public schematic alone is not
an open enclosure, bill of materials, or reproducible manufacturing package.
Record URLs, licenses, revision identifiers, and availability in a bill of
materials before relying on a candidate.

## Field requirements regardless of shape

1. **No single point of contact.** Sensor removal, loose fit, no-finger, wet
   contact, low perfusion, or lost radio must be visible as unknown/degraded.
2. **No hidden charging dependency.** The user must be able to understand low
   battery and charge the device without proprietary infrastructure. Consider
   replaceable battery modules or a small pool of swap-and-charge units.
3. **Alarm remains local.** It must be audible and/or tactile without phone,
   Wi-Fi, cellular service, or a cloud account. Test audibility in traffic,
   shelters, and outdoor conditions without assuming another person hears it.
4. **Dignity and consent.** The form must not look like a restraint or imply
   surveillance. Explain what is measured, what is stored, who can access it,
   and what the alarm can and cannot do.
5. **Loss and handoff.** Use a local pseudonymous identifier, encrypted local
   data where retention is necessary, a recovery/reset process, and no exposed
   participant identity on the enclosure.
6. **Repairability.** Prefer standard connectors, published pinouts, replaceable
   straps/sensors/batteries, and a manufacturing record. A lost or broken unit
   must not strand the person from monitoring.
7. **Environmental validation.** Test rain, sweat, dirt, cold, heat, vibration,
   impact, sleeping surfaces, clothing, and repeated charging before field use.

## Selection process

Choose the form factor only after measuring:

- retention for a full sleep period and during ordinary movement;
- SpO₂/pulse signal quality and perfusion range;
- respiratory-rate agreement against a reviewed reference;
- accelerometer artifact and placement stability;
- battery life including alarm duty cycle and cold temperature;
- alarm latency, sound/vibration level, and false-alarm response;
- time to clean, charge, repair, replace, or hand off the unit;
- participant acceptance and informed-consent feedback.

The first build should be a belt/pendant node with a removable validated
oximeter and a separately tested IMU. Do not force all sensors into one wearable
before the failure modes are understood.
