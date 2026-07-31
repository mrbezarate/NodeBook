//! Статистика дня (чистый алгоритм).
use chrono::{Datelike, Local, NaiveDate};

#[derive(Debug, Clone)]
pub struct DayInfo {
    pub date: NaiveDate, pub day_of_year: u32, pub days_lived: i64,
    pub days_remaining: i64, pub life_percentage: f32, pub day_of_week: String, pub week_number: u32,
}

impl DayInfo {
    pub fn calculate(birth_date: NaiveDate, life_expectancy: u32) -> Self {
        let today = Local::now().date_naive();
        let days_lived = (today - birth_date).num_days();
        let total_days = life_expectancy as i64 * 365;
        let days_remaining = total_days - days_lived;
        let day_of_week = match today.weekday() {
            chrono::Weekday::Mon => "Понедельник", chrono::Weekday::Tue => "Вторник",
            chrono::Weekday::Wed => "Среда", chrono::Weekday::Thu => "Четверг",
            chrono::Weekday::Fri => "Пятница", chrono::Weekday::Sat => "Суббота",
            chrono::Weekday::Sun => "Воскресенье",
        };
        Self {
            date: today, day_of_year: today.ordinal(), days_lived, days_remaining,
            life_percentage: (days_lived as f32 / total_days as f32) * 100.0,
            day_of_week: day_of_week.to_string(), week_number: today.iso_week().week(),
        }
    }

    pub fn format_ru(&self) -> String {
        format!("📅 {}, {} | День {}/{}\n🎂 Прожито: {} дней ({:.1}%) | Осталось: ~{} дней",
            self.day_of_week, self.date, self.day_of_year, 365,
            self.days_lived, self.life_percentage, self.days_remaining)
    }
}
