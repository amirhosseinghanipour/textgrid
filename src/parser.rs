use crate::types::{Interval, Point, TextGrid, TextGridError, Tier, TierType};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub fn parse_textgrid<P: AsRef<Path>>(path: P) -> Result<TextGrid, TextGridError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;
    let mut iter = lines.iter().peekable();

    
    let first_line = iter.next().ok_or(TextGridError::Format("Empty file".into()))?;
    if first_line != "File type = \"ooTextFile\"" {
        return Err(TextGridError::Format("Invalid file type".into()));
    }

    let second_line = iter.next().ok_or(TextGridError::Format("Missing object class".into()))?;
    if second_line != "Object class = \"TextGrid\"" {
        return Err(TextGridError::Format("Invalid object class".into()));
    }
    
    let is_short_format = iter.peek().map_or(false, |line| !line.contains("xmin = "));

    if is_short_format {
        parse_short_format(&mut iter)
    } else {
        parse_long_format(&mut iter)
    }
}

fn parse_long_format(lines: &mut std::iter::Peekable<std::slice::Iter<String>>) -> Result<TextGrid, TextGridError> {
    let xmin = parse_value(lines.next(), "xmin = ")?;
    let xmax = parse_value(lines.next(), "xmax = ")?;
    let tiers_exists = lines.next().ok_or(TextGridError::Format("Missing tiers flag".into()))?;
    if !tiers_exists.contains("tiers? <exists>") {
        return Err(TextGridError::Format("Invalid tiers declaration".into()));
    }

    let size = parse_value(lines.next(), "size = ")? as usize;
    lines.next(); 

    let mut tiers = Vec::with_capacity(size);
    for _ in 0..size {
        lines.next(); 
        let class_line = lines.next().ok_or(TextGridError::Format("Missing class".into()))?;
        let tier_type = if class_line.contains("IntervalTier") {
            TierType::IntervalTier
        } else if class_line.contains("TextTier") {
            TierType::PointTier
        } else {
            return Err(TextGridError::Format("Unknown tier type".into()));
        };

        let name = extract_quoted_value(lines.next(), "name = ")?;
        let tier_xmin = parse_value(lines.next(), "xmin = ")?;
        let tier_xmax = parse_value(lines.next(), "xmax = ")?;
        let tier_size = parse_value(lines.next(), "intervals: size = ").unwrap_or_else(|_| parse_value(lines.next(), "points: size = ").unwrap()) as usize;

        let mut intervals = Vec::new();
        let mut points = Vec::new();
        match tier_type {
            TierType::IntervalTier => {
                for _ in 0..tier_size {
                    lines.next(); 
                    let xmin = parse_value(lines.next(), "xmin = ")?;
                    let xmax = parse_value(lines.next(), "xmax = ")?;
                    let text = extract_quoted_value(lines.next(), "text = ")?;
                    intervals.push(Interval { xmin, xmax, text });
                }
            }
            TierType::PointTier => {
                for _ in 0..tier_size {
                    lines.next(); 
                    let time = parse_value(lines.next(), "time = ")?;
                    let mark = extract_quoted_value(lines.next(), "mark = ")?;
                    points.push(Point { time, mark });
                }
            }
        }

        tiers.push(Tier { name, tier_type, xmin: tier_xmin, xmax: tier_xmax, intervals, points });
    }

    Ok(TextGrid { xmin, xmax, tiers })
}

fn parse_short_format(lines: &mut std::iter::Peekable<std::slice::Iter<String>>) -> Result<TextGrid, TextGridError> {
    let xmin = parse_bare_value(lines.next())?;
    let xmax = parse_bare_value(lines.next())?;
    let size = parse_bare_value(lines.next())? as usize;

    let mut tiers = Vec::with_capacity(size);
    for _ in 0..size {
        let tier_type_str = lines.next().ok_or(TextGridError::Format("Missing tier type".into()))?;
        let tier_type = if tier_type_str.contains("IntervalTier") {
            TierType::IntervalTier
        } else if tier_type_str.contains("TextTier") {
            TierType::PointTier
        } else {
            return Err(TextGridError::Format("Unknown tier type".into()));
        };

        let name = extract_quoted_value_short(lines.next())?;
        let tier_xmin = parse_bare_value(lines.next())?;
        let tier_xmax = parse_bare_value(lines.next())?;
        let tier_size = parse_bare_value(lines.next())? as usize;

        let mut intervals = Vec::new();
        let mut points = Vec::new();
        match tier_type {
            TierType::IntervalTier => {
                for _ in 0..tier_size {
                    let xmin = parse_bare_value(lines.next())?;
                    let xmax = parse_bare_value(lines.next())?;
                    let text = extract_quoted_value_short(lines.next())?;
                    intervals.push(Interval { xmin, xmax, text });
                }
            }
            TierType::PointTier => {
                for _ in 0..tier_size {
                    let time = parse_bare_value(lines.next())?;
                    let mark = extract_quoted_value_short(lines.next())?;
                    points.push(Point { time, mark });
                }
            }
        }

        tiers.push(Tier { name, tier_type, xmin: tier_xmin, xmax: tier_xmax, intervals, points });
    }

    Ok(TextGrid { xmin, xmax, tiers })
}

fn parse_value(line: Option<&String>, prefix: &str) -> Result<f64, TextGridError> {
    let line = line.ok_or(TextGridError::Format("Unexpected end of file".into()))?;
    line.trim()
        .strip_prefix(prefix)
        .ok_or_else(|| TextGridError::Format(format!("Expected prefix '{}' in '{}'", prefix, line)))?
        .parse()
        .map_err(|e| TextGridError::Format(format!("Failed to parse number: {}", e)))
}

fn parse_bare_value(line: Option<&String>) -> Result<f64, TextGridError> {
    let line = line.ok_or(TextGridError::Format("Unexpected end of file".into()))?;
    line.trim()
        .parse()
        .map_err(|e| TextGridError::Format(format!("Failed to parse number: {}", e)))
}

fn extract_quoted_value(line: Option<&String>, prefix: &str) -> Result<String, TextGridError> {
    let line = line.ok_or(TextGridError::Format("Unexpected end of file".into()))?;
    let stripped = line.trim()
        .strip_prefix(prefix)
        .ok_or_else(|| TextGridError::Format(format!("Expected prefix '{}' in '{}'", prefix, line)))?;
    if stripped.starts_with('"') && stripped.ends_with('"') {
        Ok(stripped[1..stripped.len() - 1].to_string())
    } else {
        Err(TextGridError::Format("Expected quoted string".into()))
    }
}

fn extract_quoted_value_short(line: Option<&String>) -> Result<String, TextGridError> {
    let line = line.ok_or(TextGridError::Format("Unexpected end of file".into()))?;
    let trimmed = line.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') {
        Ok(trimmed[1..trimmed.len() - 1].to_string())
    } else {
        Err(TextGridError::Format("Expected quoted string".into()))
    }
}