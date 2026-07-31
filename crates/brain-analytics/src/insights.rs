//! Инсайты из данных дневника.
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub enum InsightType { SleepTrend, MoodCorrelation, ProductivityPattern, HabitStreak, RecurringTheme }

#[derive(Debug, Clone)]
pub struct Insight {
    pub text: String, pub insight_type: InsightType,
    pub confidence: f32, pub data_points: usize, pub generated_at: DateTime<Utc>,
}

pub struct InsightEngine;
impl InsightEngine {
    pub fn generate(data: &[brain_common::DiaryMetrics]) -> Vec<Insight> {
        let mut insights = Vec::new();
        if data.len() < 7 { return insights; }
        // Пример: тренд сна
        let sleep: Vec<f32> = data.iter().filter_map(|d| d.sleep_hours).collect();
        if sleep.len() >= 7 {
            let trend = crate::trends::TrendAnalyzer::detect_trend(&sleep);
            let text = match trend {
                crate::trends::TrendDirection::Down => "⚠️ Ты стал хуже спать за последнюю неделю",
                crate::trends::TrendDirection::Up => "✅ Качество сна улучшается",
                crate::trends::TrendDirection::Stable => "😴 Сон стабильный",
            };
            insights.push(Insight {
                text: text.to_string(), insight_type: InsightType::SleepTrend,
                confidence: 0.7, data_points: sleep.len(), generated_at: Utc::now(),
            });
        }
        insights
    }
}
