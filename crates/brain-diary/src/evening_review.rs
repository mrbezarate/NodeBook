//! State machine вечернего обзора.
use brain_common::DiaryMetrics;
use chrono::Local;

#[derive(Debug, Clone)]
pub enum ReviewState {
    NotStarted, ShowingDayInfo, AskingDayRating, AskingMood, AskingEnergy,
    AskingStress, AskingMotivation, AskingProductivity, AskingSleep,
    AskingExercise, AskingGoodEvents, AskingBadEvents, AskingFreeThoughts, Complete,
}

#[derive(Debug, Clone)]
pub struct EveningReview {
    pub state: ReviewState,
    pub metrics: DiaryMetrics,
}

impl EveningReview {
    pub fn new() -> Self {
        Self { state: ReviewState::ShowingDayInfo, metrics: DiaryMetrics::new(Local::now().date_naive()) }
    }

    /// Следующий вопрос для пользователя.
    pub fn next_question(&self) -> &str {
        match self.state {
            ReviewState::ShowingDayInfo => "📊 Вот твоя статистика дня. Начнём обзор?",
            ReviewState::AskingDayRating => "⭐ Оцени день от 1 до 10:",
            ReviewState::AskingMood => "😊 Настроение (1-10):",
            ReviewState::AskingEnergy => "⚡ Энергия (1-10):",
            ReviewState::AskingStress => "😰 Стресс (1-10):",
            ReviewState::AskingMotivation => "🔥 Мотивация (1-10):",
            ReviewState::AskingProductivity => "📈 Продуктивность (1-10):",
            ReviewState::AskingSleep => "😴 Сколько часов сна?",
            ReviewState::AskingExercise => "🏃 Была тренировка? (да/нет)",
            ReviewState::AskingGoodEvents => "✅ Что хорошего произошло сегодня?",
            ReviewState::AskingBadEvents => "❌ Что плохого произошло?",
            ReviewState::AskingFreeThoughts => "💭 Свободные мысли:",
            ReviewState::Complete => "✨ Обзор завершён! Запись сохранена.",
            ReviewState::NotStarted => "Обзор не начат.",
        }
    }

    /// Обработать ответ и перейти к следующему состоянию.
    pub fn process_answer(&mut self, text: &str) {
        match self.state {
            ReviewState::ShowingDayInfo => self.state = ReviewState::AskingDayRating,
            ReviewState::AskingDayRating => { self.metrics.day_rating = text.trim().parse().ok(); self.state = ReviewState::AskingMood; }
            ReviewState::AskingMood => { self.metrics.mood = text.trim().parse().ok(); self.state = ReviewState::AskingEnergy; }
            ReviewState::AskingEnergy => { self.metrics.energy = text.trim().parse().ok(); self.state = ReviewState::AskingStress; }
            ReviewState::AskingStress => { self.metrics.stress = text.trim().parse().ok(); self.state = ReviewState::AskingMotivation; }
            ReviewState::AskingMotivation => { self.metrics.motivation = text.trim().parse().ok(); self.state = ReviewState::AskingProductivity; }
            ReviewState::AskingProductivity => { self.metrics.productivity = text.trim().parse().ok(); self.state = ReviewState::AskingSleep; }
            ReviewState::AskingSleep => { self.metrics.sleep_hours = text.trim().parse().ok(); self.state = ReviewState::AskingExercise; }
            ReviewState::AskingExercise => { self.metrics.exercise = Some(text.to_lowercase().contains("да") || text.to_lowercase().contains("yes")); self.state = ReviewState::AskingGoodEvents; }
            ReviewState::AskingGoodEvents => { self.metrics.good_events = Some(text.to_string()); self.state = ReviewState::AskingBadEvents; }
            ReviewState::AskingBadEvents => { self.metrics.bad_events = Some(text.to_string()); self.state = ReviewState::AskingFreeThoughts; }
            ReviewState::AskingFreeThoughts => { self.metrics.free_thoughts = Some(text.to_string()); self.state = ReviewState::Complete; }
            _ => {}
        }
    }

    pub fn is_complete(&self) -> bool { matches!(self.state, ReviewState::Complete) }
}
