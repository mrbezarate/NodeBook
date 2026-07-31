//! Корреляционный анализ.
use crate::stats;

/// Коэффициент корреляции Пирсона.
pub fn pearson_correlation(x: &[f32], y: &[f32]) -> f32 {
    if x.len() != y.len() || x.len() < 2 { return 0.0; }
    let n = x.len() as f32;
    let mean_x = stats::mean(x);
    let mean_y = stats::mean(y);
    let cov: f32 = x.iter().zip(y.iter()).map(|(a, b)| (a - mean_x) * (b - mean_y)).sum::<f32>() / (n - 1.0);
    let std_x: f32 = (x.iter().map(|a| (a - mean_x).powi(2)).sum::<f32>() / (n - 1.0)).sqrt();
    let std_y: f32 = (y.iter().map(|b| (b - mean_y).powi(2)).sum::<f32>() / (n - 1.0)).sqrt();
    if std_x == 0.0 || std_y == 0.0 { return 0.0; }
    cov / (std_x * std_y)
}

/// Найти сильнейшие корреляции между метриками.
pub fn find_strongest<'a>(pairs: &'a [(&'a str, &'a str, f32)]) -> Vec<(&'a str, &'a str, f32)> {
    let mut sorted: Vec<_> = pairs.iter().map(|&(a, b, c)| (a, b, c.abs())).collect();
    sorted.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
    sorted.into_iter().take(5).collect()
}
