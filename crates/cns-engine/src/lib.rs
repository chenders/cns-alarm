//! Small, deterministic CNS-depression risk engine for the dedicated alarm node.
//!
//! This crate intentionally knows nothing about Bluetooth, `PostgreSQL`, files,
//! clocks, GPIO, or a user interface. Adapters provide normalized samples and
//! the caller supplies the current timestamp. Missing or invalid evidence is
//! never converted into a safe score.

use core::cmp::Ordering;

/// Physiological signal kind accepted by the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalKind {
    /// Oxygen saturation in percent, not a 0–1 fraction.
    Spo2,
    /// Respiratory rate in breaths per minute.
    RespiratoryRate,
    /// Heart rate in beats per minute.
    HeartRate,
    /// Heart-rate variability in milliseconds, compared with a baseline.
    Hrv,
}

/// Source identity for a normalized sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Continuous oximeter stream (Wellue/PC-60F family).
    Wellue,
    /// Polar H10 ECG/heart-rate stream.
    PolarH10,
    /// Read-only `AirSense` bridge stream.
    Air11,
}

/// One already-normalized physiological observation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    /// Signal carried by this observation.
    pub kind: SignalKind,
    /// Physical source of this observation.
    pub source: Source,
    /// Canonical value for `kind`.
    pub value: f64,
    /// UTC Unix timestamp in microseconds.
    pub timestamp_micros: i64,
    /// Oximeter perfusion index, when supplied by the source.
    pub perfusion_index: Option<f64>,
    /// Upstream artifact flag.
    pub artifact: bool,
}

/// Personal baseline values. Missing baselines reduce confidence or make HRV
/// unscorable; they never manufacture a baseline.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Baselines {
    /// Personal overnight `SpO₂` nadir in percent.
    pub spo2_nadir: Option<f64>,
    /// Resting heart rate in beats per minute.
    pub resting_heart_rate: Option<f64>,
    /// Personal mean HRV in milliseconds.
    pub hrv_mean: Option<f64>,
}

/// All tunable engine thresholds in one value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Thresholds {
    /// Rolling evidence window.
    pub gate_window_micros: i64,
    /// Minimum contiguous good evidence.
    pub gate_min_contiguous_micros: i64,
    /// Maximum acceptable inter-sample gap.
    pub gate_max_gap_micros: i64,
    /// Minimum observations in a window.
    pub gate_min_samples: usize,
    /// PI below which an oximeter sample cannot reassure.
    pub perfusion_soft_floor: f64,
    /// Maximum artifact fraction.
    pub max_artifact_fraction: f64,
    /// Population `SpO₂` onset and floor.
    pub spo2_onset: f64,
    /// `SpO₂` ramp width.
    pub spo2_ramp_width: f64,
    /// Absolute `SpO₂` danger floor.
    pub spo2_absolute_danger_floor: f64,
    /// Respiratory-rate onset and floor.
    pub respiratory_rate_onset: f64,
    /// Respiratory-rate floor.
    pub respiratory_rate_floor: f64,
    /// Resting-HR onset margin.
    pub heart_rate_baseline_margin: f64,
    /// Default HR onset and floor.
    pub heart_rate_onset: f64,
    /// Heart-rate floor.
    pub heart_rate_floor: f64,
    /// HRV onset and floor as baseline fractions.
    pub hrv_onset_fraction: f64,
    /// HRV floor fraction.
    pub hrv_floor_fraction: f64,
    /// Confidence when no personal baseline exists.
    pub missing_baseline_factor: f64,
    /// Minimum confidence that contributes.
    pub minimum_confidence: f64,
    /// Confidence soft-scale floor.
    pub confidence_scale_floor: f64,
    /// Corroborating contribution scale.
    pub corroboration_scale: f64,
    /// Per-source corroboration cap.
    pub corroboration_per_signal_cap: f64,
    /// Total corroboration cap.
    pub corroboration_cap: f64,
    /// Multi-source bonus.
    pub multi_source_bonus: f64,
    /// Severity needed to count as elevated.
    pub elevated_severity: f64,
    /// Confidence needed to count as elevated.
    pub elevated_confidence: f64,
    /// Corroborating-only score cap.
    pub corroborating_only_cap: f64,
    /// Entry tiers for watch, confirm, and klaxon.
    pub watch_threshold: f64,
    /// Confirm threshold.
    pub confirm_threshold: f64,
    /// Klaxon threshold.
    pub klaxon_threshold: f64,
    /// Alone-mode threshold reduction.
    pub alone_threshold_delta: f64,
    /// Normal rise sustain.
    pub rise_sustain_micros: i64,
    /// Alone-mode rise sustain.
    pub alone_rise_sustain_micros: i64,
    /// Normal confirm-to-klaxon sustain.
    pub klaxon_rise_sustain_micros: i64,
    /// Alone-mode confirm-to-klaxon sustain.
    pub alone_klaxon_rise_sustain_micros: i64,
    /// Critical fast-path sustain.
    pub critical_fast_path_micros: i64,
    /// Raw primary severity needed for critical fast path.
    pub critical_primary_severity: f64,
    /// Clearing sustain.
    pub clear_sustain_micros: i64,
    /// Clearing hysteresis.
    pub clear_hysteresis: f64,
    /// Maximum gap within a tier sustain.
    pub sustain_max_gap_micros: i64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            gate_window_micros: 60_000_000,
            gate_min_contiguous_micros: 30_000_000,
            gate_max_gap_micros: 3_000_000,
            gate_min_samples: 2,
            perfusion_soft_floor: 0.6,
            max_artifact_fraction: 0.05,
            spo2_onset: 88.0,
            spo2_ramp_width: 3.0,
            spo2_absolute_danger_floor: 80.0,
            respiratory_rate_onset: 10.0,
            respiratory_rate_floor: 5.0,
            heart_rate_baseline_margin: 15.0,
            heart_rate_onset: 50.0,
            heart_rate_floor: 40.0,
            hrv_onset_fraction: 0.6,
            hrv_floor_fraction: 0.3,
            missing_baseline_factor: 0.8,
            minimum_confidence: 0.2,
            confidence_scale_floor: 0.5,
            corroboration_scale: 0.3,
            corroboration_per_signal_cap: 0.15,
            corroboration_cap: 0.3,
            multi_source_bonus: 0.1,
            elevated_severity: 0.5,
            elevated_confidence: 0.5,
            corroborating_only_cap: 0.5,
            watch_threshold: 0.3,
            confirm_threshold: 0.6,
            klaxon_threshold: 0.85,
            alone_threshold_delta: 0.05,
            rise_sustain_micros: 60_000_000,
            alone_rise_sustain_micros: 45_000_000,
            klaxon_rise_sustain_micros: 30_000_000,
            alone_klaxon_rise_sustain_micros: 15_000_000,
            critical_fast_path_micros: 12_000_000,
            critical_primary_severity: 0.85,
            clear_sustain_micros: 120_000_000,
            clear_hysteresis: 0.1,
            sustain_max_gap_micros: 5_000_000,
        }
    }
}

/// A per-stream assessment that contributed to fusion.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignalAssessment {
    /// Signal kind.
    pub kind: SignalKind,
    /// Source identity.
    pub source: Source,
    /// Baseline-relative danger severity, 0–1.
    pub severity: f64,
    /// Evidence confidence, 0–1.
    pub confidence: f64,
}

/// Result of one evaluation tick.
#[derive(Debug, Clone, PartialEq)]
pub enum RiskAssessment {
    /// There was no honest evidence to score.
    InsufficientData,
    /// The `AirSense` bridge was not usable.
    MonitoringDegraded,
    /// The `AirSense` stream indicates mask-off/large-leak pause.
    MonitoringPaused,
    /// A score and the exact counted contributions.
    Assessed {
        /// Fused risk score, 0–1.
        score: f64,
        /// Contributions included in that score.
        contributions: Vec<SignalAssessment>,
    },
}

/// Alarm tier. Missing data never lowers a tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    /// No sustained risk currently established.
    Clear,
    /// Sustained concern.
    Watch,
    /// Sustained confirmed concern.
    Confirm,
    /// Sustained emergency-level concern.
    Klaxon,
}

/// State-bearing detector. It has no clock; `process` receives timestamps.
#[derive(Debug, Clone)]
pub struct Engine {
    thresholds: Thresholds,
    companion_present: bool,
    tier: Tier,
    can_assess: bool,
    rise_since: Option<i64>,
    rise_last: Option<i64>,
    clear_since: Option<i64>,
    clear_last: Option<i64>,
    critical_since: Option<i64>,
    critical_last: Option<i64>,
}

impl Engine {
    /// Creates a detector with explicit companion-mode state.
    #[must_use]
    pub fn new(thresholds: Thresholds, companion_present: bool) -> Self {
        Self {
            thresholds,
            companion_present,
            tier: Tier::Clear,
            can_assess: false,
            rise_since: None,
            rise_last: None,
            clear_since: None,
            clear_last: None,
            critical_since: None,
            critical_last: None,
        }
    }

    /// Current alert tier.
    #[must_use]
    pub const fn tier(&self) -> Tier {
        self.tier
    }

    /// Whether at least one honest assessment has occurred.
    #[must_use]
    pub const fn can_assess(&self) -> bool {
        self.can_assess
    }

    /// Changes companion mode without resetting sustain clocks.
    pub const fn set_companion_present(&mut self, present: bool) {
        self.companion_present = present;
    }

    /// Processes a bounded rolling buffer and advances the hysteretic tier.
    #[must_use]
    pub fn process(
        &mut self,
        samples: &[Sample],
        baselines: Baselines,
        now_micros: i64,
    ) -> (RiskAssessment, Tier) {
        let risk = evaluate(samples, baselines, now_micros, &self.thresholds);
        let tier = self.ingest(&risk, now_micros);
        (risk, tier)
    }

    fn ingest(&mut self, risk: &RiskAssessment, now: i64) -> Tier {
        match risk {
            RiskAssessment::Assessed {
                score,
                contributions,
            } => {
                self.can_assess = true;
                let primary = contributions.iter().filter(|item| {
                    matches!(item.kind, SignalKind::Spo2 | SignalKind::RespiratoryRate)
                });
                let primary_informed = primary.clone().next().is_some();
                let critical = primary.clone().any(|item| {
                    item.severity >= self.thresholds.critical_primary_severity
                        && fidelity(item.kind, item.source) >= 0.8
                });
                self.advance_ladder(*score, primary_informed, now);
                self.advance_critical(critical, primary_informed, now);
                self.advance_clear(*score, primary_informed, now);
            }
            RiskAssessment::InsufficientData
            | RiskAssessment::MonitoringDegraded
            | RiskAssessment::MonitoringPaused => {
                self.can_assess = false;
                self.reset_clear();
                // Deliberately hold rise clocks. Their last qualifying timestamp
                // invalidates the candidate after sustain_max_gap_micros.
            }
        }
        self.tier
    }

    fn entry_threshold(&self, tier: Tier) -> f64 {
        let base = match tier {
            Tier::Clear => 0.0,
            Tier::Watch => self.thresholds.watch_threshold,
            Tier::Confirm => self.thresholds.confirm_threshold,
            Tier::Klaxon => self.thresholds.klaxon_threshold,
        };
        if self.companion_present {
            base
        } else {
            base - self.thresholds.alone_threshold_delta
        }
    }

    fn rise_sustain(&self, tier: Tier) -> i64 {
        if self.companion_present {
            if tier == Tier::Klaxon {
                self.thresholds.klaxon_rise_sustain_micros
            } else {
                self.thresholds.rise_sustain_micros
            }
        } else if tier == Tier::Klaxon {
            self.thresholds.alone_klaxon_rise_sustain_micros
        } else {
            self.thresholds.alone_rise_sustain_micros
        }
    }

    fn advance_ladder(&mut self, score: f64, primary: bool, now: i64) {
        let Some(target) = next_tier(self.tier) else {
            self.reset_rise();
            return;
        };
        if target > Tier::Watch && !primary {
            return;
        }
        if score < self.entry_threshold(target) {
            if primary {
                self.reset_rise();
            }
            return;
        }
        let gap = self
            .rise_last
            .is_some_and(|last| now.saturating_sub(last) > self.thresholds.sustain_max_gap_micros);
        if self.rise_since.is_none() || gap {
            self.rise_since = Some(now);
            self.rise_last = Some(now);
            return;
        }
        self.rise_last = Some(now);
        if now.saturating_sub(self.rise_since.unwrap_or(now)) >= self.rise_sustain(target) {
            self.tier = target;
            self.reset_clear();
            self.reset_rise();
        }
    }

    fn advance_critical(&mut self, critical: bool, primary: bool, now: i64) {
        if self.tier == Tier::Klaxon {
            self.reset_critical();
            return;
        }
        if !critical {
            if primary {
                self.reset_critical();
            }
            return;
        }
        let gap = self
            .critical_last
            .is_some_and(|last| now.saturating_sub(last) > self.thresholds.sustain_max_gap_micros);
        if self.critical_since.is_none() || gap {
            self.critical_since = Some(now);
            self.critical_last = Some(now);
            return;
        }
        self.critical_last = Some(now);
        if now.saturating_sub(self.critical_since.unwrap_or(now))
            >= self.thresholds.critical_fast_path_micros
        {
            self.tier = Tier::Klaxon;
            self.reset_clear();
            self.reset_rise();
            self.reset_critical();
        }
    }

    fn advance_clear(&mut self, score: f64, primary: bool, now: i64) {
        if self.tier == Tier::Clear || !primary {
            self.reset_clear();
            return;
        }
        if score >= self.entry_threshold(self.tier) - self.thresholds.clear_hysteresis {
            self.reset_clear();
            return;
        }
        let gap = self
            .clear_last
            .is_some_and(|last| now.saturating_sub(last) > self.thresholds.sustain_max_gap_micros);
        if self.clear_since.is_none() || gap {
            self.clear_since = Some(now);
            self.clear_last = Some(now);
            return;
        }
        self.clear_last = Some(now);
        if now.saturating_sub(self.clear_since.unwrap_or(now))
            >= self.thresholds.clear_sustain_micros
        {
            if let Some(lower) = lower_tier(self.tier) {
                self.tier = lower;
            }
            self.reset_clear();
        }
    }

    fn reset_rise(&mut self) {
        self.rise_since = None;
        self.rise_last = None;
    }
    fn reset_clear(&mut self) {
        self.clear_since = None;
        self.clear_last = None;
    }
    fn reset_critical(&mut self) {
        self.critical_since = None;
        self.critical_last = None;
    }
}

fn next_tier(tier: Tier) -> Option<Tier> {
    match tier {
        Tier::Clear => Some(Tier::Watch),
        Tier::Watch => Some(Tier::Confirm),
        Tier::Confirm => Some(Tier::Klaxon),
        Tier::Klaxon => None,
    }
}
fn lower_tier(tier: Tier) -> Option<Tier> {
    match tier {
        Tier::Clear => None,
        Tier::Watch => Some(Tier::Clear),
        Tier::Confirm => Some(Tier::Watch),
        Tier::Klaxon => Some(Tier::Confirm),
    }
}

fn valid_sample(sample: &Sample, now: i64, thresholds: &Thresholds) -> bool {
    if !sample.value.is_finite()
        || sample.timestamp_micros > now
        || now.saturating_sub(sample.timestamp_micros) > thresholds.gate_window_micros
    {
        return false;
    }
    let valid_value = match sample.kind {
        SignalKind::Spo2 | SignalKind::RespiratoryRate => (0.0..=100.0).contains(&sample.value),
        SignalKind::HeartRate => (0.0..=300.0).contains(&sample.value),
        SignalKind::Hrv => (0.0..=5_000.0).contains(&sample.value),
    };
    let valid_pi = sample
        .perfusion_index
        .is_none_or(|pi| pi.is_finite() && (0.0..=100.0).contains(&pi));
    valid_value && valid_pi
}

fn good(sample: &Sample, thresholds: &Thresholds) -> bool {
    !sample.artifact
        && sample
            .perfusion_index
            .is_none_or(|pi| pi >= thresholds.perfusion_soft_floor)
}

fn fidelity(kind: SignalKind, source: Source) -> f64 {
    match (kind, source) {
        (SignalKind::HeartRate, Source::Wellue) => 0.8,
        (SignalKind::HeartRate, Source::PolarH10) => 0.95,
        (SignalKind::Spo2, Source::Wellue)
        | (SignalKind::Hrv, Source::PolarH10)
        | (SignalKind::RespiratoryRate, Source::Air11) => 0.9,
        _ => 0.0,
    }
}

fn median(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let middle = values.len() / 2;
    if values.len().is_multiple_of(2) {
        Some(values[middle - 1].midpoint(values[middle]))
    } else {
        Some(values[middle])
    }
}

fn stream_assessment(
    kind: SignalKind,
    source: Source,
    samples: &[Sample],
    baselines: Baselines,
    now: i64,
    thresholds: &Thresholds,
) -> Option<SignalAssessment> {
    let mut in_window: Vec<Sample> = samples
        .iter()
        .copied()
        .filter(|sample| {
            sample.kind == kind && sample.source == source && valid_sample(sample, now, thresholds)
        })
        .collect();
    if in_window.len() < thresholds.gate_min_samples {
        return None;
    }
    in_window.sort_by_key(|sample| sample.timestamp_micros);
    let artifacts = in_window.iter().filter(|sample| sample.artifact).count();
    let artifacts = f64::from(u32::try_from(artifacts).ok()?);
    let sample_count = f64::from(u32::try_from(in_window.len()).ok()?);
    if artifacts / sample_count > thresholds.max_artifact_fraction {
        return None;
    }
    let good_samples: Vec<Sample> = in_window
        .iter()
        .copied()
        .filter(|sample| good(sample, thresholds))
        .collect();
    let good_count = f64::from(u32::try_from(good_samples.len()).ok()?);
    let window_micros = f64::from(u32::try_from(thresholds.gate_window_micros).ok()?);
    let coverage = (good_count / (window_micros / 1_000_000.0)).clamp(0.5, 1.0);
    let mut longest = 0_i64;
    let mut run_start = good_samples
        .first()
        .map_or(now, |sample| sample.timestamp_micros);
    let mut previous = run_start;
    for sample in good_samples.iter().skip(1) {
        if sample.timestamp_micros.saturating_sub(previous) > thresholds.gate_max_gap_micros {
            longest = longest.max(previous.saturating_sub(run_start));
            run_start = sample.timestamp_micros;
        }
        previous = sample.timestamp_micros;
    }
    longest = longest.max(previous.saturating_sub(run_start));
    if longest < thresholds.gate_min_contiguous_micros {
        return None;
    }
    let mut values: Vec<f64> = good_samples.iter().map(|sample| sample.value).collect();
    let representative = median(&mut values)?;
    let (severity, baseline_available) = severity(kind, representative, baselines, thresholds)?;
    let confidence = fidelity(kind, source)
        * coverage
        * if baseline_available {
            1.0
        } else {
            thresholds.missing_baseline_factor
        };
    (confidence >= thresholds.minimum_confidence).then_some(SignalAssessment {
        kind,
        source,
        severity,
        confidence,
    })
}

fn severity(
    kind: SignalKind,
    value: f64,
    baselines: Baselines,
    thresholds: &Thresholds,
) -> Option<(f64, bool)> {
    let ramp = |value: f64, onset: f64, floor: f64| {
        if onset > floor {
            ((onset - value) / (onset - floor)).clamp(0.0, 1.0)
        } else if value <= floor {
            1.0
        } else {
            0.0
        }
    };
    match kind {
        SignalKind::Spo2 => {
            let nadir = baselines
                .spo2_nadir
                .filter(|v| v.is_finite() && (50.0..=100.0).contains(v));
            let onset = nadir.map_or(thresholds.spo2_onset, |v| {
                thresholds.spo2_onset.min(v - 3.0)
            });
            let floor = thresholds
                .spo2_onset
                .min(onset - thresholds.spo2_ramp_width);
            Some((
                if value <= thresholds.spo2_absolute_danger_floor {
                    1.0
                } else {
                    ramp(value, onset, floor)
                },
                nadir.is_some(),
            ))
        }
        SignalKind::RespiratoryRate => Some((
            ramp(
                value,
                thresholds.respiratory_rate_onset,
                thresholds.respiratory_rate_floor,
            ),
            false,
        )),
        SignalKind::HeartRate => {
            let resting = baselines
                .resting_heart_rate
                .filter(|v| v.is_finite() && (30.0..=120.0).contains(v));
            let onset = resting.map_or(thresholds.heart_rate_onset, |v| v - 15.0);
            Some((
                ramp(value, onset, thresholds.heart_rate_floor),
                resting.is_some(),
            ))
        }
        SignalKind::Hrv => {
            let mean = baselines.hrv_mean.filter(|v| v.is_finite() && *v > 0.0)?;
            let fraction = value / mean;
            Some((
                ramp(
                    fraction,
                    thresholds.hrv_onset_fraction,
                    thresholds.hrv_floor_fraction,
                ),
                true,
            ))
        }
    }
}

/// Evaluates all supported source/kind streams deterministically.
#[must_use]
pub fn evaluate(
    samples: &[Sample],
    baselines: Baselines,
    now_micros: i64,
    thresholds: &Thresholds,
) -> RiskAssessment {
    let mut contributions = Vec::new();
    for kind in [
        SignalKind::Spo2,
        SignalKind::RespiratoryRate,
        SignalKind::HeartRate,
        SignalKind::Hrv,
    ] {
        for source in [Source::Wellue, Source::PolarH10, Source::Air11] {
            if let Some(item) =
                stream_assessment(kind, source, samples, baselines, now_micros, thresholds)
            {
                contributions.push(item);
            }
        }
    }
    if contributions.is_empty() {
        return RiskAssessment::InsufficientData;
    }
    let primary: Vec<SignalAssessment> = contributions
        .iter()
        .copied()
        .filter(|item| matches!(item.kind, SignalKind::Spo2 | SignalKind::RespiratoryRate))
        .collect();
    let corroborating: Vec<SignalAssessment> = contributions
        .iter()
        .copied()
        .filter(|item| matches!(item.kind, SignalKind::HeartRate | SignalKind::Hrv))
        .collect();
    let primary_score = primary
        .iter()
        .map(|item| {
            item.severity
                * (thresholds.confidence_scale_floor
                    + (1.0 - thresholds.confidence_scale_floor) * item.confidence)
        })
        .fold(0.0, f64::max);
    let corroboration = corroborating
        .iter()
        .map(|item| {
            (item.severity * item.confidence * thresholds.corroboration_scale)
                .min(thresholds.corroboration_per_signal_cap)
        })
        .sum::<f64>()
        .min(thresholds.corroboration_cap);
    let elevated_sources = contributions
        .iter()
        .filter(|item| {
            item.severity >= thresholds.elevated_severity
                && item.confidence >= thresholds.elevated_confidence
        })
        .map(|item| item.source)
        .fold(Vec::new(), |mut sources, source| {
            if !sources.contains(&source) {
                sources.push(source);
            }
            sources
        });
    let mut score = (primary_score
        + corroboration
        + if elevated_sources.len() >= 2 {
            thresholds.multi_source_bonus
        } else {
            0.0
        })
    .clamp(0.0, 1.0);
    if !primary.iter().any(|item| item.severity >= 0.2) {
        score = score.min(thresholds.corroborating_only_cap);
    }
    RiskAssessment::Assessed {
        score,
        contributions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn series(value: f64, kind: SignalKind, source: Source, now: i64) -> Vec<Sample> {
        (0..=30)
            .map(|offset| Sample {
                kind,
                source,
                value,
                timestamp_micros: now - 30_000_000 + offset * 1_000_000,
                perfusion_index: Some(2.0),
                artifact: false,
            })
            .collect()
    }

    #[test]
    fn no_data_is_not_clear_evidence() {
        assert_eq!(
            evaluate(
                &[],
                Baselines::default(),
                60_000_000,
                &Thresholds::default()
            ),
            RiskAssessment::InsufficientData
        );
    }

    #[test]
    fn healthy_wellue_series_is_assessed() {
        let result = evaluate(
            &series(96.0, SignalKind::Spo2, Source::Wellue, 60_000_000),
            Baselines::default(),
            60_000_000,
            &Thresholds::default(),
        );
        assert!(matches!(result, RiskAssessment::Assessed { score, .. } if score.abs() < 1e-12));
    }

    #[test]
    fn deep_spo2_is_critical_and_fast_path_reaches_klaxon() {
        let thresholds = Thresholds::default();
        let samples = series(79.0, SignalKind::Spo2, Source::Wellue, 60_000_000);
        let mut engine = Engine::new(thresholds, false);
        let (_, first) = engine.process(&samples, Baselines::default(), 60_000_000);
        assert_eq!(first, Tier::Clear);
        let (_, second) = engine.process(&samples, Baselines::default(), 64_000_000);
        assert_eq!(second, Tier::Clear);
        let (_, third) = engine.process(&samples, Baselines::default(), 68_000_000);
        assert_eq!(third, Tier::Clear);
        let (_, fourth) = engine.process(&samples, Baselines::default(), 72_000_000);
        assert_eq!(fourth, Tier::Klaxon);
    }

    #[test]
    fn artifact_only_window_is_not_assessed() {
        let mut samples = series(96.0, SignalKind::Spo2, Source::Wellue, 60_000_000);
        for sample in &mut samples {
            sample.artifact = true;
        }
        assert_eq!(
            evaluate(
                &samples,
                Baselines::default(),
                60_000_000,
                &Thresholds::default()
            ),
            RiskAssessment::InsufficientData
        );
    }
}
