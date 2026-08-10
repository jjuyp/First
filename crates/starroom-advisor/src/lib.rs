//! Explainable, local-first editing suggestions for Starroom v0.2.
//! This crate contains deterministic statistics/rules only. No network or generative AI.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct AnalysisStats {
    pub shadow_fraction: f32,
    pub highlight_fraction: f32,
    pub black_clip_fraction: f32,
    pub white_clip_fraction: f32,
    pub median_luminance: f32,
    pub estimated_warmth_bias: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Suggestion {
    pub id: String,
    pub control: String,
    pub value: f32,
    pub confidence: f32,
    pub reason: String,
}

/// Builds deterministic editing statistics from linear Rec.2020 RGB samples.
/// The thresholds are intentionally explicit so the advisor remains explainable and testable.
pub fn analyze_linear_rgb(samples: &[[f32; 3]]) -> AnalysisStats {
    if samples.is_empty() {
        return AnalysisStats::default();
    }

    let mut luminance_values = Vec::with_capacity(samples.len());
    let mut shadow_count = 0usize;
    let mut highlight_count = 0usize;
    let mut black_clip_count = 0usize;
    let mut white_clip_count = 0usize;
    let mut warmth_sum = 0.0_f32;

    for [red, green, blue] in samples.iter().copied() {
        let red = if red.is_finite() { red.max(0.0) } else { 0.0 };
        let green = if green.is_finite() { green.max(0.0) } else { 0.0 };
        let blue = if blue.is_finite() { blue.max(0.0) } else { 0.0 };
        let luminance = 0.2627 * red + 0.6780 * green + 0.0593 * blue;
        luminance_values.push(luminance);

        if luminance < 0.12 {
            shadow_count += 1;
        }
        if luminance > 0.72 {
            highlight_count += 1;
        }
        if luminance <= 0.003 {
            black_clip_count += 1;
        }
        if luminance >= 0.985 || red >= 0.995 || green >= 0.995 || blue >= 0.995 {
            white_clip_count += 1;
        }

        let chromatic_energy = red + blue;
        if chromatic_energy > 1.0e-5 {
            warmth_sum += (red - blue) / chromatic_energy;
        }
    }

    luminance_values.sort_by(|left, right| left.total_cmp(right));
    let middle = luminance_values.len() / 2;
    let median_luminance = if luminance_values.len() % 2 == 0 {
        (luminance_values[middle - 1] + luminance_values[middle]) * 0.5
    } else {
        luminance_values[middle]
    };

    let count = samples.len() as f32;
    AnalysisStats {
        shadow_fraction: shadow_count as f32 / count,
        highlight_fraction: highlight_count as f32 / count,
        black_clip_fraction: black_clip_count as f32 / count,
        white_clip_fraction: white_clip_count as f32 / count,
        median_luminance,
        estimated_warmth_bias: (warmth_sum / count).clamp(-1.0, 1.0),
    }
}

pub fn advise(stats: AnalysisStats) -> Vec<Suggestion> {
    let mut output = Vec::new();

    if stats.shadow_fraction >= 0.34 && stats.black_clip_fraction < 0.02 {
        let value = ((stats.shadow_fraction - 0.30) * 130.0).clamp(8.0, 32.0);
        output.push(Suggestion {
            id: "lift-shadows".into(),
            control: "shadows".into(),
            value,
            confidence: 0.78,
            reason: format!(
                "Dark tones occupy {:.0}% of the frame while black clipping remains limited.",
                stats.shadow_fraction * 100.0
            ),
        });
    }

    if stats.white_clip_fraction >= 0.01 || stats.highlight_fraction >= 0.22 {
        let value = -((stats.white_clip_fraction * 900.0) + stats.highlight_fraction * 55.0)
            .clamp(10.0, 45.0);
        output.push(Suggestion {
            id: "recover-highlights".into(),
            control: "highlights".into(),
            value,
            confidence: 0.82,
            reason: format!(
                "Bright tones are concentrated and {:.1}% of samples are near white clipping.",
                stats.white_clip_fraction * 100.0
            ),
        });
    }

    if stats.median_luminance < 0.12 && stats.white_clip_fraction < 0.005 {
        output.push(Suggestion {
            id: "raise-exposure".into(),
            control: "exposure".into(),
            value: 0.25,
            confidence: 0.64,
            reason: "Median luminance is low and the image has headroom before white clipping."
                .into(),
        });
    }

    if stats.estimated_warmth_bias.abs() >= 0.18 {
        output.push(Suggestion {
            id: "neutralize-cast".into(),
            control: "temperature".into(),
            value: (-stats.estimated_warmth_bias * 35.0).clamp(-20.0, 20.0),
            confidence: 0.55,
            reason: "A broad warm/cool cast was detected; this is a relative encoded-image correction, not a physical Kelvin estimate."
                .into(),
        });
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyzes_dark_samples_and_median_without_nan() {
        let samples = [
            [0.02, 0.02, 0.02],
            [0.04, 0.03, 0.02],
            [0.08, 0.07, 0.06],
            [0.40, 0.35, 0.30],
        ];
        let stats = analyze_linear_rgb(&samples);
        assert!(stats.shadow_fraction >= 0.5);
        assert!(stats.median_luminance.is_finite());
        assert!(stats.white_clip_fraction == 0.0);
    }

    #[test]
    fn detects_white_clip_and_warm_bias() {
        let samples = [
            [1.0, 0.90, 0.45],
            [0.95, 0.80, 0.30],
            [0.70, 0.55, 0.20],
        ];
        let stats = analyze_linear_rgb(&samples);
        assert!(stats.white_clip_fraction > 0.0);
        assert!(stats.estimated_warmth_bias > 0.18);
    }

    #[test]
    fn suggests_shadow_lift_only_when_black_clipping_is_not_dominant() {
        let suggestions = advise(AnalysisStats {
            shadow_fraction: 0.45,
            black_clip_fraction: 0.004,
            ..Default::default()
        });
        assert!(
            suggestions
                .iter()
                .any(|item| item.control == "shadows" && item.value > 0.0)
        );

        let clipped = advise(AnalysisStats {
            shadow_fraction: 0.45,
            black_clip_fraction: 0.08,
            ..Default::default()
        });
        assert!(!clipped.iter().any(|item| item.control == "shadows"));
    }

    #[test]
    fn highlight_rule_is_explainable_and_bounded() {
        let suggestions = advise(AnalysisStats {
            highlight_fraction: 0.30,
            white_clip_fraction: 0.025,
            ..Default::default()
        });
        let item = suggestions
            .iter()
            .find(|item| item.control == "highlights")
            .expect("highlight suggestion");
        assert!(item.value <= -10.0 && item.value >= -45.0);
        assert!(!item.reason.is_empty());
    }
}
