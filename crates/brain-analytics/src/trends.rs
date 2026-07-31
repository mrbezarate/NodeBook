//! Тренды (чистая математика).
pub struct TrendAnalyzer;

#[derive(Debug, Clone)]
pub enum TrendDirection { Up, Down, Stable }

impl TrendAnalyzer {
    pub fn moving_average(data: &[f32], window: usize) -> Vec<f32> {
        if data.len() < window { return vec![]; }
        data.windows(window).map(|w| w.iter().sum::<f32>() / window as f32).collect()
    }

    pub fn detect_trend(data: &[f32]) -> TrendDirection {
        if data.len() < 3 { return TrendDirection::Stable; }
        let half = data.len() / 2;
        let first: f32 = data[..half].iter().sum::<f32>() / half as f32;
        let second: f32 = data[half..].iter().sum::<f32>() / (data.len() - half) as f32;
        let diff = second - first;
        if diff > 0.5 { TrendDirection::Up }
        else if diff < -0.5 { TrendDirection::Down }
        else { TrendDirection::Stable }
    }

    pub fn standard_deviation(data: &[f32]) -> f32 {
        let n = data.len() as f32;
        if n <= 1.0 { return 0.0; }
        let mean = data.iter().sum::<f32>() / n;
        let variance = data.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / (n - 1.0);
        variance.sqrt()
    }
}
