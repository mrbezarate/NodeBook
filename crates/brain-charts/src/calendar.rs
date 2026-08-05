use plotters::prelude::*;
use chrono::{Datelike, NaiveDate};
use std::collections::HashMap;

/// Generate a 1-month calendar grid image
pub fn draw_monthly_calendar(
    year: i32,
    month: u32,
    activity_data: &HashMap<NaiveDate, u32>,
) -> Result<Vec<u8>, String> {
    let mut buffer = vec![0; 800 * 600 * 3];
    {
        let root = BitMapBackend::with_buffer(&mut buffer, (800, 600)).into_drawing_area();
        root.fill(&RGBColor(30, 30, 30)).map_err(|e| e.to_string())?;

        let title_style = ("sans-serif", 40).into_font().color(&WHITE);
        root.draw_text(
            &format!("{:04}-{:02} Activity", year, month),
            &title_style,
            (30, 20),
        ).map_err(|e| e.to_string())?;

        let start_date = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
        let days_in_month = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap() - start_date
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap() - start_date
        }.num_days();

        let start_weekday = start_date.weekday().num_days_from_monday();

        let cell_w = 100;
        let cell_h = 80;
        let offset_x = 50;
        let offset_y = 100;

        let weekdays = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        for (i, wd) in weekdays.iter().enumerate() {
            root.draw_text(
                *wd,
                &("sans-serif", 20).into_font().color(&WHITE),
                (offset_x + (i as i32) * cell_w + 30, offset_y - 30),
            ).map_err(|e| e.to_string())?;
        }

        for day in 1..=days_in_month {
            let idx = start_weekday as i32 + (day as i32) - 1;
            let row = idx / 7;
            let col = idx % 7;

            let x = offset_x + col * cell_w;
            let y = offset_y + row * cell_h;

            let date = NaiveDate::from_ymd_opt(year, month, day as u32).unwrap();
            let count = activity_data.get(&date).copied().unwrap_or(0);

            let bg_color = match count {
                0 => RGBColor(50, 50, 50),
                1..=3 => RGBColor(0, 100, 0),
                4..=7 => RGBColor(0, 150, 0),
                _ => RGBColor(0, 200, 0),
            };

            root.draw(&Rectangle::new(
                [(x, y), (x + cell_w - 5, y + cell_h - 5)],
                bg_color.filled(),
            )).map_err(|e| e.to_string())?;

            root.draw_text(
                &day.to_string(),
                &("sans-serif", 15).into_font().color(&WHITE),
                (x + 5, y + 5),
            ).map_err(|e| e.to_string())?;

            if count > 0 {
                root.draw_text(
                    &format!("{}", count),
                    &("sans-serif", 20).into_font().color(&RGBColor(255, 200, 0)),
                    (x + 30, y + 30),
                ).map_err(|e| e.to_string())?;
            }
        }

        root.present().map_err(|e| e.to_string())?;
    }

    // Convert raw RGB buffer to PNG
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
