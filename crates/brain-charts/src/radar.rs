use plotters::prelude::*;

/// Generate a radar chart (Hexagram) for metrics
pub fn draw_radar_chart(
    categories: &[String],
    values: &[f32], // Normalized 0.0 to 1.0
) -> Result<Vec<u8>, String> {
    if categories.len() != values.len() || categories.is_empty() {
        return Err("Invalid data".into());
    }
    
    crate::init_fonts();
    let mut buffer = vec![0; 800 * 600 * 3];
    {
        let root = BitMapBackend::with_buffer(&mut buffer, (800, 600)).into_drawing_area();
        root.fill(&RGBColor(30, 30, 30)).map_err(|e| e.to_string())?;

        let title_style = ("sans-serif", 40).into_font().color(&WHITE);
        root.draw_text(
            "Life Balance (Hexagram)",
            &title_style,
            (200, 30),
        ).map_err(|e| e.to_string())?;

        let center_x = 400.0;
        let center_y = 350.0;
        let radius = 200.0;
        let n = categories.len() as f64;

        // Draw axes and labels
        for i in 0..categories.len() {
            let angle = std::f64::consts::PI * 2.0 * (i as f64) / n - std::f64::consts::FRAC_PI_2;
            let end_x = center_x + radius * angle.cos();
            let end_y = center_y + radius * angle.sin();

            // Draw axis line
            root.draw(&PathElement::new(
                vec![(center_x as i32, center_y as i32), (end_x as i32, end_y as i32)],
                ShapeStyle::from(&RGBColor(100, 100, 100)).stroke_width(2),
            )).map_err(|e| e.to_string())?;

            // Draw text label
            let text_x = center_x + (radius + 40.0) * angle.cos();
            let text_y = center_y + (radius + 40.0) * angle.sin();
            root.draw_text(
                &categories[i],
                &("sans-serif", 20).into_font().color(&WHITE),
                (text_x as i32 - 30, text_y as i32),
            ).map_err(|e| e.to_string())?;
        }
        
        // Draw grid polygons (for 20%, 40%, 60%, 80%, 100%)
        for step in 1..=5 {
            let r = radius * (step as f64) / 5.0;
            let mut points = Vec::new();
            for i in 0..categories.len() {
                let angle = std::f64::consts::PI * 2.0 * (i as f64) / n - std::f64::consts::FRAC_PI_2;
                points.push((
                    (center_x + r * angle.cos()) as i32,
                    (center_y + r * angle.sin()) as i32
                ));
            }
            points.push(points[0]); // Close polygon
            root.draw(&PathElement::new(
                points,
                ShapeStyle::from(&RGBColor(80, 80, 80)).stroke_width(1),
            )).map_err(|e| e.to_string())?;
        }

        // Draw data polygon
        let mut data_points = Vec::new();
        for i in 0..categories.len() {
            let val = values[i] as f64;
            let r = radius * val;
            let angle = std::f64::consts::PI * 2.0 * (i as f64) / n - std::f64::consts::FRAC_PI_2;
            data_points.push((
                (center_x + r * angle.cos()) as i32,
                (center_y + r * angle.sin()) as i32
            ));
        }
        
        let mut filled_points = data_points.clone();
        filled_points.push(filled_points[0]); // Close
        
        let poly = Polygon::new(
            filled_points.clone(),
            RGBColor(0, 150, 255).mix(0.5).filled(),
        );
        root.draw(&poly).map_err(|e| e.to_string())?;
        
        let line = PathElement::new(
            filled_points,
            ShapeStyle::from(&RGBColor(0, 200, 255)).stroke_width(3),
        );
        root.draw(&line).map_err(|e| e.to_string())?;
        
        for pt in data_points {
            root.draw(&Circle::new(pt, 5, RGBColor(255, 255, 255).filled())).map_err(|e| e.to_string())?;
        }

        root.present().map_err(|e| e.to_string())?;
    }

    let mut png_data = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_data, 800, 600);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
        writer.write_image_data(&buffer).map_err(|e| e.to_string())?;
    }

    Ok(png_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draw_radar_chart_valid() {
        let categories = vec![
            "Здоровье".to_string(),
            "Работа".to_string(),
            "Финансы".to_string(),
            "Обучение".to_string(),
            "Отношения".to_string(),
            "Отдых".to_string(),
        ];
        let values = vec![0.8, 0.9, 0.7, 0.85, 0.6, 0.5];
        let res = draw_radar_chart(&categories, &values);
        assert!(res.is_ok());
        let png = res.unwrap();
        assert!(!png.is_empty());
        assert_eq!(&png[1..4], b"PNG");
    }

    #[test]
    fn test_draw_radar_chart_mismatched_len() {
        let categories = vec!["A".to_string(), "B".to_string()];
        let values = vec![0.5];
        let res = draw_radar_chart(&categories, &values);
        assert!(res.is_err());
    }
}
