use brain_common::{Result, DiaryMetrics};
use brain_core::traits::AiProvider;
use brain_events::{EventReader, Event};
use std::sync::Arc;
use std::path::PathBuf;

pub struct LifeAnalyticsEngine {
    ai_provider: Arc<dyn AiProvider>,
    event_reader: EventReader,
}

impl LifeAnalyticsEngine {
    pub fn new(ai_provider: Arc<dyn AiProvider>, event_log_path: PathBuf) -> Self {
        Self {
            ai_provider,
            event_reader: EventReader::new(event_log_path),
        }
    }

    pub async fn generate_life_insights(&self, metrics: &[DiaryMetrics], period_days: usize) -> Result<String> {
        let all_events = self.event_reader.read_all_events().await.unwrap_or_default();
        
        let events_limit = match period_days {
            7 => 30,
            30 => 100,
            365 => 200,
            _ => 300,
        };

        let recent_events: Vec<&Event> = all_events.iter().rev().take(events_limit).collect();
        let recent_metrics: Vec<&DiaryMetrics> = metrics.iter().rev().take(period_days).collect();
        
        let mut prompt_data = String::new();
        prompt_data.push_str(&format!("--- DIARY METRICS (LAST {} DAYS) ---\n", period_days));
        for m in &recent_metrics {
            prompt_data.push_str(&format!(
                "Date: {}, Mood: {:?}, Stress: {:?}, Sleep: {:?}h, Productivity: {:?}\n",
                m.date, m.mood, m.stress, m.sleep_hours, m.productivity
            ));
        }
        
        prompt_data.push_str("\n--- RECENT KNOWLEDGE EVENTS ---\n");
        for e in recent_events.iter() {
            prompt_data.push_str(&format!("Time: {}, Event: {:?}\n", e.timestamp.format("%Y-%m-%d"), e.event_type));
        }
        
        let period_str = match period_days {
            7 => "week",
            30 => "month",
            365 => "year",
            _ => "entire history",
        };

        let prompt = format!(
            "You are a Life Analytics AI. Analyze the user's Diary Metrics (mood, stress, sleep, etc.) \
            and their Knowledge Events (what they learned, searched, or wrote about) for the last {}.\n\
            Find 2-3 deep, meaningful correlations between their knowledge work and their mood/sleep/stress. \
            Format the response as a short, inspiring message with bullet points. Speak in Russian directly to the user.\n\n{}",
            period_str, prompt_data
        );
        
        let insight = self.ai_provider.complete(&prompt).await?;
        
        Ok(insight)
    }
}
