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

        if recent_metrics.is_empty() && recent_events.is_empty() {
            return Ok(
                "📊 <b>Недостаточно данных для глубокого анализа</b>\n\n\
                В вашей базе пока нет сохранённых записей дневника или заметок за выбранный период.\n\n\
                💡 <i>Совет: Заполните вечерний обзор (/diary) или отправьте свои мысли и заметки в бот, чтобы AI смог выявить персональные инсайты о сне, настроении и продуктивности!</i>"
                .to_string()
            );
        }
        
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
            7 => "последние 7 дней",
            30 => "последние 30 дней",
            365 => "этот год",
            _ => "всё время",
        };

        let prompt = format!(
            "Ты — аналитик личной эффективности и ментального здоровья. Проанализируй дневниковые метрики пользователя \
            (настроение, стресс, сон, продуктивность) и события базы знаний за {}.\n\
            Сформулируй 2-3 четких, лаконичных и вдохновляющих вывода (инсайта) с практическими советами.\n\
            Пиши на русском языке с пунктами и эмодзи. Не лей воду, давай только конкретику.\n\n{}",
            period_str, prompt_data
        );
        
        match self.ai_provider.complete(&prompt).await {
            Ok(insight) => Ok(insight),
            Err(e) => {
                tracing::warn!("AI insight completion failed: {}", e);
                // Fallback rule-based stats
                let avg_mood: f32 = if !recent_metrics.is_empty() {
                    let sum: f32 = recent_metrics.iter().map(|m| m.mood.unwrap_or(7) as f32).sum();
                    sum / recent_metrics.len() as f32
                } else {
                    7.0
                };
                let avg_sleep: f32 = if !recent_metrics.is_empty() {
                    let sum: f32 = recent_metrics.iter().map(|m| m.sleep_hours.unwrap_or(7.5) as f32).sum();
                    sum / recent_metrics.len() as f32
                } else {
                    7.5
                };
                Ok(format!(
                    "📈 <b>Базовые инсайты за {}:</b>\n\n\
                    • Средний уровень настроения: <b>{:.1}/10</b>\n\
                    • Средняя продолжительность сна: <b>{:.1} ч</b>\n\
                    • Зафиксировано заметок и событий: <b>{}</b>\n\n\
                    💡 <i>(Для генерации глубоких нейросетевых инсайтов убедитесь в доступности AI-провайдера)</i>",
                    period_str, avg_mood, avg_sleep, recent_events.len()
                ))
            }
        }
    }
}
