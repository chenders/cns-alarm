//! Deterministic stdin harness for the CNS engine.
//!
//! This is deliberately not the production GPIO/BLE daemon yet. It provides a
//! small, auditable input seam for replay and Pi benchmarking while adapters are
//! developed and validated.

use std::io::{self, BufRead};

use cns_engine::{Baselines, Engine, Sample, SignalKind, Source, Thresholds, Tier};

fn main() {
    if let Err(error) = run() {
        eprintln!("cns-alarm: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut engine = Engine::new(Thresholds::default(), false);
    let stdin = io::stdin();
    let mut samples: Vec<Sample> = Vec::new();
    for (line_number, line) in stdin.lock().lines().enumerate() {
        let line = line.map_err(|error| format!("read line {}: {error}", line_number + 1))?;
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let sample =
            parse_sample(&line).map_err(|error| format!("line {}: {error}", line_number + 1))?;
        let now = sample.timestamp_micros;
        samples.retain(|item| now.saturating_sub(item.timestamp_micros) <= 60_000_000);
        samples.push(sample);
        let (assessment, tier) = engine.process(&samples, Baselines::default(), now);
        println!(
            "{} tier={} evidence={}",
            now,
            tier_name(tier),
            evidence_name(&assessment)
        );
    }
    Ok(())
}

fn parse_sample(line: &str) -> Result<Sample, String> {
    let fields: Vec<&str> = line.split(',').map(str::trim).collect();
    if fields.len() != 6 {
        return Err(
            "expected timestamp_micros,kind,source,value,perfusion_index,artifact".to_owned(),
        );
    }
    let timestamp_micros = fields[0]
        .parse::<i64>()
        .map_err(|_| "invalid timestamp_micros".to_owned())?;
    let kind = match fields[1] {
        "spo2" => SignalKind::Spo2,
        "respiratory_rate" => SignalKind::RespiratoryRate,
        "heart_rate" => SignalKind::HeartRate,
        "hrv" => SignalKind::Hrv,
        _ => return Err(format!("unknown signal kind {:?}", fields[1])),
    };
    let source = match fields[2] {
        "wellue" => Source::Wellue,
        "polar_h10" => Source::PolarH10,
        "air11" => Source::Air11,
        _ => return Err(format!("unknown source {:?}", fields[2])),
    };
    let value = fields[3]
        .parse::<f64>()
        .map_err(|_| "invalid value".to_owned())?;
    let perfusion_index = match fields[4] {
        "-" | "absent" => None,
        value => Some(
            value
                .parse::<f64>()
                .map_err(|_| "invalid perfusion_index".to_owned())?,
        ),
    };
    let artifact = match fields[5] {
        "0" | "false" => false,
        "1" | "true" => true,
        _ => return Err("artifact must be 0, 1, false, or true".to_owned()),
    };
    Ok(Sample {
        kind,
        source,
        value,
        timestamp_micros,
        perfusion_index,
        artifact,
    })
}

fn tier_name(tier: Tier) -> &'static str {
    match tier {
        Tier::Clear => "clear",
        Tier::Watch => "watch",
        Tier::Confirm => "confirm",
        Tier::Klaxon => "klaxon",
    }
}

fn evidence_name(assessment: &cns_engine::RiskAssessment) -> &'static str {
    match assessment {
        cns_engine::RiskAssessment::InsufficientData => "insufficient_data",
        cns_engine::RiskAssessment::MonitoringDegraded => "monitoring_degraded",
        cns_engine::RiskAssessment::MonitoringPaused => "monitoring_paused",
        cns_engine::RiskAssessment::Assessed { .. } => "assessed",
    }
}
