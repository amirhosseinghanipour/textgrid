//! Text format writing for Praat `.TextGrid` files.
//!
//! This module provides functionality to write Praat TextGrid files in both long and short formats.
//! It converts a `TextGrid` structure (from the crate's types module) into the appropriate text format
//! and saves it to a file, supporting both IntervalTiers and PointTiers (TextTiers in Praat terminology).
//!
//! ## Supported Formats
//! - **Long Format**: Verbose format with labeled fields (e.g., `xmin = 0`).
//! - **Short Format**: Compact format with bare values (e.g., `0` instead of `xmin = 0`).
//!
//! ## Usage
//! ```rust
//! use textgrid::{TextGrid, Tier, TierType, Interval, write_textgrid};
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
//!     // Write to a file in long format
//!     write_textgrid(&tg, "output.TextGrid", false)?;
//!     Ok(())
//! }
//! ```

use crate::types::{TextGrid, TextGridError, TierType};
use std::fs::File;
use std::io::{Write};
use std::path::Path;

/// Writes a `TextGrid` to a Praat `.TextGrid` file.
///
/// # Arguments
/// * `textgrid` - The `TextGrid` to write.
/// * `path` - Path to the output file, implementing `AsRef<Path>`.
/// * `short_format` - If `true`, writes in short format; otherwise, uses long format.
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
/// textgrid::write_textgrid(&tg, "test.TextGrid", true).unwrap();
/// ```
pub fn write_textgrid<P: AsRef<Path>>(textgrid: &TextGrid, path: P, short_format: bool) -> Result<(), TextGridError> {
    let mut file = File::create(path)?;
    if short_format {
        write_short_format(&mut file, textgrid)?;
    } else {
        write_long_format(&mut file, textgrid)?;
    }
    Ok(())
}

/// Writes a `TextGrid` to a file in the long (verbose) format.
///
/// # Arguments
/// * `file` - The file to write to.
/// * `textgrid` - The `TextGrid` to write.
///
/// # Returns
/// Returns a `Result` indicating success (`Ok(())`) or a `TextGridError`.
///
/// # Errors
/// - `TextGridError::IO` if writing to the file fails.
fn write_long_format(file: &mut File, textgrid: &TextGrid) -> Result<(), TextGridError> {
    writeln!(file, "File type = \"ooTextFile\"")?;
    writeln!(file, "Object class = \"TextGrid\"")?;
    writeln!(file, "xmin = {}", textgrid.xmin)?;
    writeln!(file, "xmax = {}", textgrid.xmax)?;
    writeln!(file, "tiers? <exists>")?;
    writeln!(file, "size = {}", textgrid.tiers.len())?;
    writeln!(file, "item []:")?;

    for (i, tier) in textgrid.tiers.iter().enumerate() {
        writeln!(file, "    item [{}]:", i + 1)?;
        writeln!(
            file,
            "        class = \"{}\"",
            match tier.tier_type {
                TierType::IntervalTier => "IntervalTier",
                TierType::PointTier => "TextTier",
            }
        )?;
        writeln!(file, "        name = \"{}\"", tier.name)?;
        writeln!(file, "        xmin = {}", tier.xmin)?;
        writeln!(file, "        xmax = {}", tier.xmax)?;
        match tier.tier_type {
            TierType::IntervalTier => {
                writeln!(file, "        intervals: size = {}", tier.intervals.len())?;
                for (j, interval) in tier.intervals.iter().enumerate() {
                    writeln!(file, "        intervals [{}]:", j + 1)?;
                    writeln!(file, "            xmin = {}", interval.xmin)?;
                    writeln!(file, "            xmax = {}", interval.xmax)?;
                    writeln!(file, "            text = \"{}\"", interval.text)?;
                }
            }
            TierType::PointTier => {
                writeln!(file, "        points: size = {}", tier.points.len())?;
                for (j, point) in tier.points.iter().enumerate() {
                    writeln!(file, "        points [{}]:", j + 1)?;
                    writeln!(file, "            time = {}", point.time)?;
                    writeln!(file, "            mark = \"{}\"", point.mark)?;
                }
            }
        }
    }
    Ok(())
}

/// Writes a `TextGrid` to a file in the short (compact) format.
///
/// # Arguments
/// * `file` - The file to write to.
/// * `textgrid` - The `TextGrid` to write.
///
/// # Returns
/// Returns a `Result` indicating success (`Ok(())`) or a `TextGridError`.
///
/// # Errors
/// - `TextGridError::IO` if writing to the file fails.
fn write_short_format(file: &mut File, textgrid: &TextGrid) -> Result<(), TextGridError> {
    writeln!(file, "File type = \"ooTextFile\"")?;
    writeln!(file, "Object class = \"TextGrid\"")?;
    writeln!(file, "{}", textgrid.xmin)?;
    writeln!(file, "{}", textgrid.xmax)?;
    writeln!(file, "{}", textgrid.tiers.len())?;

    for tier in &textgrid.tiers {
        writeln!(
            file,
            "\"{}\"",
            match tier.tier_type {
                TierType::IntervalTier => "IntervalTier",
                TierType::PointTier => "TextTier",
            }
        )?;
        writeln!(file, "\"{}\"", tier.name)?;
        writeln!(file, "{}", tier.xmin)?;
        writeln!(file, "{}", tier.xmax)?;
        match tier.tier_type {
            TierType::IntervalTier => {
                writeln!(file, "{}", tier.intervals.len())?;
                for interval in &tier.intervals {
                    writeln!(file, "{}", interval.xmin)?;
                    writeln!(file, "{}", interval.xmax)?;
                    writeln!(file, "\"{}\"", interval.text)?;
                }
            }
            TierType::PointTier => {
                writeln!(file, "{}", tier.points.len())?;
                for point in &tier.points {
                    writeln!(file, "{}", point.time)?;
                    writeln!(file, "\"{}\"", point.mark)?;
                }
            }
        }
    }
    Ok(())
}