//! Binary format support for Praat `.TextGrid` files, matching Praat's specification.
//!
//! This module provides functionality to read and write Praat TextGrid files in their binary format.
//! It follows Praat's binary specification for efficient storage and retrieval of TextGrid data,
//! supporting both IntervalTiers and PointTiers (TextTiers in Praat terminology).
//!
//! ## Binary Format Overview
//! - Uses little-endian byte order.
//! - Starts with a `"ooBinaryFile"` header, followed by the object class `"TextGrid"`.
//! - Stores time values as 64-bit floats (`f64`), lengths as 16-bit or 32-bit integers, and text as UTF-8 strings with length prefixes.
//!
//! ## Usage
//! ```rust
//! use textgrid::{TextGrid, Tier, TierType, Interval, write_binary, read_binary};
//!
//! fn main() -> Result<(), textgrid::TextGridError> {
//!     // Create a simple TextGrid
//!     let mut tg = TextGrid::new(0.0, 10.0)?;
//!     let tier = Tier {
//!         name: "words".to_string(),
//!         tier_type: TierType::IntervalTier,
//!         xmin: 0.0,
//!         xmax: 10.0,
//!         intervals: vec![Interval {
//!             xmin: 1.0,
//!             xmax: 2.0,
//!             text: "hello".to_string(),
//!         }],
//!         points: vec![],
//!     };
//!     tg.add_tier(tier)?;
//!
//!     // Write to a binary file
//!     write_binary(&tg, "output.TextGrid")?;
//!
//!     // Read it back
//!     let read_tg = read_binary("output.TextGrid")?;
//!     assert_eq!(read_tg.tiers[0].intervals[0].text, "hello");
//!     Ok(())
//! }
//! ```

use crate::types::{TextGrid, TextGridError, Tier, TierType, Interval, Point};
use std::fs::File;
use std::io::{Read, Write, BufReader, BufWriter};
use std::path::Path;

/// Reads a Praat `.TextGrid` file from the binary format.
///
/// # Arguments
/// * `path` - Path to the binary `.TextGrid` file, implementing `AsRef<Path>`.
///
/// # Returns
/// Returns a `Result` containing the parsed `TextGrid` or a `TextGridError`.
///
/// # Errors
/// - `TextGridError::IO` if the file cannot be opened or read.
/// - `TextGridError::Format` if the file does not match the Praat binary format (e.g., wrong header, invalid class, or malformed data).
///
/// # Examples
/// ```rust
/// let tg = textgrid::read_binary("test.TextGrid").unwrap();
/// assert_eq!(tg.tiers.len(), 1); // Assuming test.TextGrid has one tier
/// ```
pub fn read_binary<P: AsRef<Path>>(path: P) -> Result<TextGrid, TextGridError> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;

    let mut cursor = 0;
    if &buffer[cursor..cursor + 12] != b"ooBinaryFile" {
        return Err(TextGridError::Format("Not a Praat binary TextGrid".into()));
    }
    cursor += 12;

    let obj_len = u16::from_le_bytes(buffer[cursor..cursor + 2].try_into().unwrap()) as usize;
    cursor += 2;
    if &buffer[cursor..cursor + obj_len] != b"TextGrid" {
        return Err(TextGridError::Format("Invalid object class".into()));
    }
    cursor += obj_len;

    let xmin = f64::from_le_bytes(buffer[cursor..cursor + 8].try_into().unwrap());
    cursor += 8;
    let xmax = f64::from_le_bytes(buffer[cursor..cursor + 8].try_into().unwrap());
    cursor += 8;
    let size = u32::from_le_bytes(buffer[cursor..cursor + 4].try_into().unwrap()) as usize;
    cursor += 4;

    let mut tiers = Vec::with_capacity(size);
    for _ in 0..size {
        let class_len = u16::from_le_bytes(buffer[cursor..cursor + 2].try_into().unwrap()) as usize;
        cursor += 2;
        let class = String::from_utf8(buffer[cursor..cursor + class_len].to_vec())?;
        let tier_type = if class == "IntervalTier" {
            TierType::IntervalTier
        } else if class == "TextTier" {
            TierType::PointTier
        } else {
            return Err(TextGridError::Format("Unknown tier type".into()));
        };
        cursor += class_len;

        let name_len = u16::from_le_bytes(buffer[cursor..cursor + 2].try_into().unwrap()) as usize;
        cursor += 2;
        let name = String::from_utf8(buffer[cursor..cursor + name_len].to_vec())?;
        cursor += name_len;

        let tier_xmin = f64::from_le_bytes(buffer[cursor..cursor + 8].try_into().unwrap());
        cursor += 8;
        let tier_xmax = f64::from_le_bytes(buffer[cursor..cursor + 8].try_into().unwrap());
        cursor += 8;

        let count = u32::from_le_bytes(buffer[cursor..cursor + 4].try_into().unwrap()) as usize;
        cursor += 4;

        let mut intervals = Vec::new();
        let mut points = Vec::new();
        match tier_type {
            TierType::IntervalTier => {
                for _ in 0..count {
                    let xmin = f64::from_le_bytes(buffer[cursor..cursor + 8].try_into().unwrap());
                    cursor += 8;
                    let xmax = f64::from_le_bytes(buffer[cursor..cursor + 8].try_into().unwrap());
                    cursor += 8;
                    let text_len = u16::from_le_bytes(buffer[cursor..cursor + 2].try_into().unwrap()) as usize;
                    cursor += 2;
                    let text = String::from_utf8(buffer[cursor..cursor + text_len].to_vec())?;
                    cursor += text_len;
                    intervals.push(Interval { xmin, xmax, text });
                }
            }
            TierType::PointTier => {
                for _ in 0..count {
                    let time = f64::from_le_bytes(buffer[cursor..cursor + 8].try_into().unwrap());
                    cursor += 8;
                    let mark_len = u16::from_le_bytes(buffer[cursor..cursor + 2].try_into().unwrap()) as usize;
                    cursor += 2;
                    let mark = String::from_utf8(buffer[cursor..cursor + mark_len].to_vec())?;
                    cursor += mark_len;
                    points.push(Point { time, mark });
                }
            }
        }

        tiers.push(Tier { name, tier_type, xmin: tier_xmin, xmax: tier_xmax, intervals, points });
    }

    Ok(TextGrid::new(xmin, xmax)?.with_tiers(tiers))
}

/// Writes a `TextGrid` to a Praat `.TextGrid` file in binary format.
///
/// # Arguments
/// * `textgrid` - The `TextGrid` to write.
/// * `path` - Path to the output file, implementing `AsRef<Path>`.
///
/// # Returns
/// Returns a `Result` indicating success (`Ok(())`) or a `TextGridError`.
///
/// # Errors
/// - `TextGridError::IO` if the file cannot be created or written to.
///
/// # Examples
/// ```rust
/// let tg = TextGrid::new(0.0, 5.0).unwrap(); // Assume tiers are added
/// textgrid::write_binary(&tg, "test.TextGrid").unwrap();
/// ```
pub fn write_binary<P: AsRef<Path>>(textgrid: &TextGrid, path: P) -> Result<(), TextGridError> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    writer.write_all(b"ooBinaryFile")?;
    let class = b"TextGrid";
    writer.write_all(&(class.len() as u16).to_le_bytes())?;
    writer.write_all(class)?;
    writer.write_all(&textgrid.xmin.to_le_bytes())?;
    writer.write_all(&textgrid.xmax.to_le_bytes())?;
    writer.write_all(&(textgrid.tiers.len() as u32).to_le_bytes())?;

    for tier in &textgrid.tiers {
        let class = match tier.tier_type {
            TierType::IntervalTier => b"IntervalTier" as &[u8],
            TierType::PointTier => b"TextTier" as &[u8],
        };
        writer.write_all(&(class.len() as u16).to_le_bytes())?;
        writer.write_all(class)?;

        let name_bytes = tier.name.as_bytes();
        writer.write_all(&(name_bytes.len() as u16).to_le_bytes())?;
        writer.write_all(name_bytes)?;

        writer.write_all(&tier.xmin.to_le_bytes())?;
        writer.write_all(&tier.xmax.to_le_bytes())?;

        match tier.tier_type {
            TierType::IntervalTier => {
                writer.write_all(&(tier.intervals.len() as u32).to_le_bytes())?;
                for interval in &tier.intervals {
                    writer.write_all(&interval.xmin.to_le_bytes())?;
                    writer.write_all(&interval.xmax.to_le_bytes())?;
                    let text_bytes = interval.text.as_bytes();
                    writer.write_all(&(text_bytes.len() as u16).to_le_bytes())?;
                    writer.write_all(text_bytes)?;
                }
            }
            TierType::PointTier => {
                writer.write_all(&(tier.points.len() as u32).to_le_bytes())?;
                for point in &tier.points {
                    writer.write_all(&point.time.to_le_bytes())?;
                    let mark_bytes = point.mark.as_bytes();
                    writer.write_all(&(mark_bytes.len() as u16).to_le_bytes())?;
                    writer.write_all(mark_bytes)?;
                }
            }
        }
    }
    writer.flush()?;
    Ok(())
}