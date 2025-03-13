//! Text format parsing for Praat `.TextGrid` files.
//!
//! This module provides functionality to parse Praat TextGrid files in both long and short formats.
//! It supports reading IntervalTiers and PointTiers (TextTiers in Praat terminology) from `.TextGrid`
//! files, converting them into the `TextGrid` structure defined in the crate's types module.
//!
//! ## Supported Formats
//! - **Long Format**: Verbose format with labeled fields (e.g., `xmin = 0`).
//! - **Short Format**: Compact format with bare values (e.g., `0` instead of `xmin = 0`).
//!
//! ## Usage
//! ```rust
//! use textgrid::parse_textgrid;
//!
//! fn main() -> Result<(), textgrid::TextGridError> {
//!     let textgrid = parse_textgrid("example.TextGrid")?;
//!     println!("TextGrid bounds: {} to {}", textgrid.xmin, textgrid.xmax);
//!     for tier in &textgrid.tiers {
//!         println!("Tier: {}", tier.name);
//!     }
//!     Ok(())
//! }
//! ```

use crate::types::{Interval, Point, TextGrid, TextGridError, Tier, TierType};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Parses a Praat `.TextGrid` file from the given path.
///
/// # Arguments
/// * `path` - Path to the `.TextGrid` file, implementing `AsRef<Path>`.
///
/// # Returns
/// Returns a `Result` containing the parsed `TextGrid` or a `TextGridError`.
///
/// # Errors
/// - `TextGridError::IO` if the file cannot be opened or read.
/// - `TextGridError::Format` if the file is malformed (e.g., invalid headers, missing data, or incorrect syntax).
///
/// # Examples
/// ```rust
/// let tg = textgrid::parse_textgrid("test.TextGrid").unwrap();
/// assert_eq!(tg.tiers.len(), 1); // Assuming test.TextGrid has one tier
/// ```
pub fn parse_textgrid<P: AsRef<Path>>(path: P) -> Result<TextGrid, TextGridError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;
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

/// Parses a TextGrid file in the long (verbose) format.
///
/// # Arguments
/// * `lines` - Iterator over the lines of the file, with peekable functionality.
///
/// # Returns
/// Returns a `Result` containing the parsed `TextGrid` or a `TextGridError`.
///
/// # Errors
/// - `TextGridError::Format` if the file structure is invalid or data cannot be parsed.
fn parse_long_format(lines: &mut std::iter::Peekable<std::slice::Iter<String>>) -> Result<TextGrid, TextGridError> {
    let xmin = parse_value(lines.next(), "xmin = ")?;
    let xmax = parse_value(lines.next(), "xmax = ")?;
    let tiers_exists = lines.next().ok_or(TextGridError::Format("Missing tiers flag".into()))?;
    if !tiers_exists.contains("tiers? <exists>") {
        return Err(TextGridError::Format("Invalid tiers declaration".into()));
    }

    let size = parse_value(lines.next(), "size = ")? as usize;
    lines.next(); // Skip "item []:" line

    let mut tiers = Vec::with_capacity(size);
    for _ in 0..size {
        lines.next(); // Skip "item [n]:" line
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
                    lines.next(); // Skip "intervals [n]:" line
                    let xmin = parse_value(lines.next(), "xmin = ")?;
                    let xmax = parse_value(lines.next(), "xmax = ")?;
                    let text = extract_quoted_value(lines.next(), "text = ")?;
                    intervals.push(Interval { xmin, xmax, text });
                }
            }
            TierType::PointTier => {
                for _ in 0..tier_size {
                    lines.next(); // Skip "points [n]:" line
                    let time = parse_value(lines.next(), "time = ")?;
                    let mark = extract_quoted_value(lines.next(), "mark = ")?;
                    points.push(Point { time, mark });
                }
            }
        }

        tiers.push(Tier { name, tier_type, xmin: tier_xmin, xmax: tier_xmax, intervals, points });
    }

    Ok(TextGrid::new(xmin, xmax)?.with_tiers(tiers))
}

/// Parses a TextGrid file in the short (compact) format.
///
/// # Arguments
/// * `lines` - Iterator over the lines of the file, with peekable functionality.
///
/// # Returns
/// Returns a `Result` containing the parsed `TextGrid` or a `TextGridError`.
///
/// # Errors
/// - `TextGridError::Format` if the file structure is invalid or data cannot be parsed.
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

    Ok(TextGrid::new(xmin, xmax)?.with_tiers(tiers))
}

/// Parses a numeric value from a line with a given prefix (e.g., "xmin = 0").
///
/// # Arguments
/// * `line` - Optional line to parse.
/// * `prefix` - Expected prefix before the value.
///
/// # Returns
/// Returns a `Result` containing the parsed `f64` value or a `TextGridError`.
///
/// # Errors
/// - `TextGridError::Format` if the line is missing, lacks the prefix, or the value cannot be parsed as a number.
fn parse_value(line: Option<&String>, prefix: &str) -> Result<f64, TextGridError> {
    let line = line.ok_or(TextGridError::Format("Unexpected end of file".into()))?;
    line.trim()
        .strip_prefix(prefix)
        .ok_or_else(|| TextGridError::Format(format!("Expected prefix '{}' in '{}'", prefix, line)))?
        .parse()
        .map_err(|e| TextGridError::Format(format!("Failed to parse number: {}", e)))
}

/// Parses a bare numeric value from a line (e.g., "0").
///
/// # Arguments
/// * `line` - Optional line to parse.
///
/// # Returns
/// Returns a `Result` containing the parsed `f64` value or a `TextGridError`.
///
/// # Errors
/// - `TextGridError::Format` if the line is missing or the value cannot be parsed as a number.
fn parse_bare_value(line: Option<&String>) -> Result<f64, TextGridError> {
    let line = line.ok_or(TextGridError::Format("Unexpected end of file".into()))?;
    line.trim()
        .parse()
        .map_err(|e| TextGridError::Format(format!("Failed to parse number: {}", e)))
}

/// Extracts a quoted string value from a line with a given prefix (e.g., `text = "hello"`).
///
/// # Arguments
/// * `line` - Optional line to parse.
/// * `prefix` - Expected prefix before the quoted value.
///
/// # Returns
/// Returns a `Result` containing the extracted `String` or a `TextGridError`.
///
/// # Errors
/// - `TextGridError::Format` if the line is missing, lacks the prefix, or the value is not quoted.
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

/// Extracts a quoted string value from a bare line (e.g., `"hello"`).
///
/// # Arguments
/// * `line` - Optional line to parse.
///
/// # Returns
/// Returns a `Result` containing the extracted `String` or a `TextGridError`.
///
/// # Errors
/// - `TextGridError::Format` if the line is missing or the value is not quoted.
fn extract_quoted_value_short(line: Option<&String>) -> Result<String, TextGridError> {
    let line = line.ok_or(TextGridError::Format("Unexpected end of file".into()))?;
    let trimmed = line.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') {
        Ok(trimmed[1..trimmed.len() - 1].to_string())
    } else {
        Err(TextGridError::Format("Expected quoted string".into()))
    }
}