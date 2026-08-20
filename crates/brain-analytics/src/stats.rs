//! Базовая статистика.
pub fn mean(data: &[f32]) -> f32 { if data.is_empty() { 0.0 } else { data.iter().sum::<f32>() / data.len() as f32 } }

pub fn median(data: &mut [f32]) -> f32 {
    if data.is_empty() { return 0.0; }
    data.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = data.len() / 2;
    if data.len() % 2 == 0 { (data[mid - 1] + data[mid]) / 2.0 } else { data[mid] }
}

pub fn min_max(data: &[f32]) -> (f32, f32) {
    let min = data.iter().cloned().fold(f32::INFINITY, f32::min);
    let max = data.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    (min, max)
}

/// Посчитать streak (серию дней подряд с условием).
pub fn calculate_streak(values: &[bool]) -> usize {
    let mut max_streak = 0;
    let mut current = 0;
    for &v in values { if v { current += 1; max_streak = max_streak.max(current); } else { current = 0; } }
    max_streak
}
