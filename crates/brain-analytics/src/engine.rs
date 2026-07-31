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

    /// Анализ событий и метрик для генерации инсайтов (Event-Sourced Analytics)
    pub async fn generate_life_insights(&self, metrics: &[DiaryMetrics]) -> Result<String> {
        // 1. Прочитать последние события из лога
        let all_events = self.event_reader.read_all_events().await.unwrap_or_default();
        // Берем, например, последние 100 событий
        let recent_events: Vec<&Event> = all_events.iter().rev().take(100).collect();
        
        // 2. Собрать метрики за последнюю неделю
        let recent_metrics: Vec<&DiaryMetrics> = metrics.iter().rev().take(7).collect();
        
        // 3. Форматировать данные для ИИ
        let mut prompt_data = String::new();
        prompt_data.push_str("--- DIARY METRICS (LAST 7 DAYS) ---\n");
        for m in recent_metrics {
            prompt_data.push_str(&format!(
                "Date: {}, Mood: {:?}, Stress: {:?}, Sleep: {:?}h, Productivity: {:?}\n",
                m.date, m.mood, m.stress, m.sleep_hours, m.productivity
            ));
        }
        
        prompt_data.push_str("\n--- RECENT KNOWLEDGE EVENTS ---\n");
        for e in recent_events.iter().take(20) {
            prompt_data.push_str(&format!("Time: {}, Event: {:?}\n", e.timestamp.format("%Y-%m-%d"), e.event_type));
        }
        
        // 4. Попросить ИИ найти корреляции (Life Insights)
        let prompt = format!(
            "You are a Life Analytics AI. Analyze the user's recent Diary Metrics (mood, stress, sleep, etc.) \
            and their Knowledge Events (what they learned, searched, or wrote about recently).\n\
            Find 2-3 deep, meaningful correlations between their knowledge work and their mood/sleep/stress. \
            Format the response as a short, inspiring message with bullet points. Speak in Russian directly to the user.\n\n{}",
            prompt_data
        );
        
        let insight = self.ai_provider.complete(&prompt).await?;
        
        Ok(insight)
    }
}
