//! Confidence scoring.
#[derive(Debug, Clone)]
pub struct ConfidenceScore { pub value: f32, pub source: ScoreSource }

#[derive(Debug, Clone)]
pub enum ScoreSource { RuleEngine, AiFallback, Hybrid { rule_weight: f32, ai_weight: f32 } }

/// Объединить несколько оценок уверенности.
pub fn merge_scores(scores: &[ConfidenceScore]) -> ConfidenceScore {
    if scores.is_empty() { return ConfidenceScore { value: 0.0, source: ScoreSource::RuleEngine }; }
    let avg = scores.iter().map(|s| s.value).sum::<f32>() / scores.len() as f32;
    ConfidenceScore { value: avg, source: ScoreSource::Hybrid { rule_weight: 0.7, ai_weight: 0.3 } }
}
