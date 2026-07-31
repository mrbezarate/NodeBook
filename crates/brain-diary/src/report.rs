//! Генерация отчёта дневника в Markdown.
use brain_common::DiaryMetrics;

pub fn generate_report(metrics: &DiaryMetrics) -> String {
    let mut md = format!("# 📔 Дневник — {}\n\n", metrics.date);
    md.push_str("## Метрики\n\n");
    md.push_str(&format!("| Метрика | Значение |\n|---------|----------|\n"));
    if let Some(v) = metrics.day_rating { md.push_str(&format!("| ⭐ Оценка дня | {} |\n", v)); }
    if let Some(v) = metrics.mood { md.push_str(&format!("| 😊 Настроение | {} |\n", v)); }
    if let Some(v) = metrics.energy { md.push_str(&format!("| ⚡ Энергия | {} |\n", v)); }
    if let Some(v) = metrics.stress { md.push_str(&format!("| 😰 Стресс | {} |\n", v)); }
    if let Some(v) = metrics.motivation { md.push_str(&format!("| 🔥 Мотивация | {} |\n", v)); }
    if let Some(v) = metrics.productivity { md.push_str(&format!("| 📈 Продуктивность | {} |\n", v)); }
    if let Some(v) = metrics.sleep_hours { md.push_str(&format!("| 😴 Сон | {} ч |\n", v)); }
    if let Some(v) = metrics.exercise { md.push_str(&format!("| 🏃 Тренировка | {} |\n", if v { "Да" } else { "Нет" })); }
    md.push('\n');
    if let Some(ref v) = metrics.good_events { md.push_str(&format!("## ✅ Хорошее\n\n{}\n\n", v)); }
    if let Some(ref v) = metrics.bad_events { md.push_str(&format!("## ❌ Плохое\n\n{}\n\n", v)); }
    if let Some(ref v) = metrics.free_thoughts { md.push_str(&format!("## 💭 Мысли\n\n{}\n\n", v)); }
    md
}
