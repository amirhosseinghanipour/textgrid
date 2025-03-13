use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TextGridError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Invalid TextGrid format: {0}")]
    Format(String),
}

#[derive(Debug, PartialEq)]
pub enum TierType {
    IntervalTier,
    PointTier,
}

#[derive(Debug)]
pub struct Interval {
    pub xmin: f64,
    pub xmax: f64,
    pub text: String,
}

#[derive(Debug)]
pub struct Point {
    pub time: f64,
    pub mark: String,
}

#[derive(Debug)]
pub struct Tier {
    pub name: String,
    pub tier_type: TierType,
    pub xmin: f64,
    pub xmax: f64,
    pub intervals: Vec<Interval>,
    pub points: Vec<Point>,
}

#[derive(Debug)]
pub struct TextGrid {
    pub xmin: f64,
    pub xmax: f64,
    pub tiers: Vec<Tier>,
}

impl TextGrid {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, TextGridError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        let header = lines.next().ok_or(TextGridError::Format("Empty file".into()))??;
        if header != "File type = \"ooTextFile\"" {
            return Err(TextGridError::Format("Invalid file type".into()));
        }

        let object_class = lines.next().ok_or(TextGridError::Format("Missing object class".into()))??;
        if object_class != "Object class = \"TextGrid\"" {
            return Err(TextGridError::Format("Invalid object class".into()));
        }

        let xmin_line = lines.next().ok_or(TextGridError::Format("Missing xmin".into()))??;
        let xmax_line = lines.next().ok_or(TextGridError::Format("Missing xmax".into()))??;
        let tiers_exists = lines.next().ok_or(TextGridError::Format("Missing tiers flag".into()))??;

        let xmin = parse_value(&xmin_line, "xmin = ")?;
        let xmax = parse_value(&xmax_line, "xmax = ")?;
        if !tiers_exists.contains("tiers? <exists>") {
            return Err(TextGridError::Format("Invalid tiers declaration".into()));
        }

        let size_line = lines.next().ok_or(TextGridError::Format("Missing size".into()))??;
        let size = parse_value(&size_line, "size = ")? as usize;

        let mut tiers = Vec::with_capacity(size);
        lines.next(); 

        for _ in 0..size {
            let item_line = lines.next().ok_or(TextGridError::Format("Missing item".into()))??;
            if !item_line.contains("item [") {
                return Err(TextGridError::Format("Invalid item declaration".into()));
            }

            let class_line = lines.next().ok_or(TextGridError::Format("Missing class".into()))??;
            let tier_type = if class_line.contains("IntervalTier") {
                TierType::IntervalTier
            } else if class_line.contains("TextTier") {
                TierType::PointTier
            } else {
                return Err(TextGridError::Format("Unknown tier type".into()));
            };

            let name_line = lines.next().ok_or(TextGridError::Format("Missing name".into()))??;
            let name = extract_quoted_value(&name_line, "name = ")?;

            let tier_xmin_line = lines.next().ok_or(TextGridError::Format("Missing tier xmin".into()))??;
            let tier_xmax_line = lines.next().ok_or(TextGridError::Format("Missing tier xmax".into()))??;
            let tier_xmin = parse_value(&tier_xmin_line, "xmin = ")?;
            let tier_xmax = parse_value(&tier_xmax_line, "xmax = ")?;

            let size_line = lines.next().ok_or(TextGridError::Format("Missing intervals/points size".into()))??;
            let tier_size = parse_value(&size_line, "intervals: size = ").unwrap_or_else(|_| {
                parse_value(&size_line, "points: size = ").expect("Failed to parse both intervals and points size")
            }) as usize;

            let mut intervals = Vec::new();
            let mut points = Vec::new();

            match tier_type {
                TierType::IntervalTier => {
                    for _ in 0..tier_size {
                        lines.next(); 
                        let xmin_line = lines.next().ok_or(TextGridError::Format("Missing interval xmin".into()))??;
                        let xmax_line = lines.next().ok_or(TextGridError::Format("Missing interval xmax".into()))??;
                        let text_line = lines.next().ok_or(TextGridError::Format("Missing interval text".into()))??;

                        let xmin = parse_value(&xmin_line, "xmin = ")?;
                        let xmax = parse_value(&xmax_line, "xmax = ")?;
                        let text = extract_quoted_value(&text_line, "text = ")?;

                        intervals.push(Interval { xmin, xmax, text });
                    }
                }
                TierType::PointTier => {
                    for _ in 0..tier_size {
                        lines.next(); 
                        let time_line = lines.next().ok_or(TextGridError::Format("Missing point time".into()))??;
                        let mark_line = lines.next().ok_or(TextGridError::Format("Missing point mark".into()))??;

                        let time = parse_value(&time_line, "time = ")?; 
                        let mark = extract_quoted_value(&mark_line, "mark = ")?;

                        points.push(Point { time, mark });
                    }
                }
            }

            tiers.push(Tier {
                name,
                tier_type,
                xmin: tier_xmin,
                xmax: tier_xmax,
                intervals,
                points,
            });
        }

        Ok(TextGrid { xmin, xmax, tiers })
    }
}


fn parse_value(line: &str, prefix: &str) -> Result<f64, TextGridError> {
    line.trim()
        .strip_prefix(prefix)
        .ok_or_else(|| TextGridError::Format(format!("Expected prefix '{}' in '{}'", prefix, line)))?
        .parse()
        .map_err(|e| TextGridError::Format(format!("Failed to parse number: {}", e)))
}

fn extract_quoted_value(line: &str, prefix: &str) -> Result<String, TextGridError> {
    let stripped = line.trim()
        .strip_prefix(prefix)
        .ok_or_else(|| TextGridError::Format(format!("Expected prefix '{}' in '{}'", prefix, line)))?;
    if stripped.starts_with('"') && stripped.ends_with('"') {
        Ok(stripped[1..stripped.len() - 1].to_string())
    } else {
        Err(TextGridError::Format("Expected quoted string".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_textgrid() {
        let sample = r#"File type = "ooTextFile"
Object class = "TextGrid"
xmin = 0
xmax = 2.53
tiers? <exists>
size = 1
item []:
    item [1]:
        class = "IntervalTier"
        name = "words"
        xmin = 0
        xmax = 2.53
        intervals: size = 2
        intervals [1]:
            xmin = 0
            xmax = 1.125
            text = "Hello"
        intervals [2]:
            xmin = 1.125
            xmax = 2.53
            text = "World"
"#;
        let path = "test.TextGrid";
        std::fs::write(path, sample).unwrap();
        let textgrid = TextGrid::from_file(path).unwrap();
        std::fs::remove_file(path).unwrap();

        assert_eq!(textgrid.xmin, 0.0);
        assert_eq!(textgrid.xmax, 2.53);
        assert_eq!(textgrid.tiers.len(), 1);

        let tier = &textgrid.tiers[0];
        assert_eq!(tier.name, "words");
        assert_eq!(tier.tier_type, TierType::IntervalTier);
        assert_eq!(tier.intervals.len(), 2);

        let interval1 = &tier.intervals[0];
        assert_eq!(interval1.xmin, 0.0);
        assert_eq!(interval1.xmax, 1.125);
        assert_eq!(interval1.text, "Hello");

        let interval2 = &tier.intervals[1];
        assert_eq!(interval2.xmin, 1.125);
        assert_eq!(interval2.xmax, 2.53);
        assert_eq!(interval2.text, "World");
    }
}