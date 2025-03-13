//! A Rust crate for working with Praat `.TextGrid` files.
//!
//! Provides parsing, writing, manipulation, and history tracking for TextGrid data.
//! This crate supports both text and binary formats of Praat TextGrid files, offering a robust
//! set of tools for phonetic annotation, linguistic research, and audio analysis.
//!
//! ## Features
//! - **Parsing**: Read TextGrid files in long/short text formats and Praat's binary format.
//! - **Writing**: Write TextGrid files in long/short text formats and binary format.
//! - **Manipulation**: Add, remove, split, merge, and query tiers, intervals, and points with undo/redo support.
//! - **Validation**: Ensure data integrity with bounds and overlap checks.
//!
//! ## Usage
//! ```rust
//! use textgrid::{TextGrid, Tier, TierType, Interval};
//!
//! fn main() -> Result<(), textgrid::TextGridError> {
//!     // Create a TextGrid
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
//!     // Save to file
//!     tg.to_file("example.TextGrid", false)?;
//!
//!     // Load from file
//!     let loaded = TextGrid::from_file("example.TextGrid")?;
//!     assert_eq!(loaded.tiers[0].intervals[0].text, "hello");
//!     Ok(())
//! }
//! ```

mod parser;
mod types;
mod writer;
mod validator;
mod binary;

pub use types::{Interval, Point, TextGrid, TextGridError, Tier, TierType};
use std::path::Path;

impl TextGrid {
    /// Loads a TextGrid from a file (text or binary format).
    ///
    /// # Arguments
    /// * `path` - Path to the `.TextGrid` file, implementing `AsRef<Path>`.
    ///
    /// # Returns
    /// Returns a `Result` containing the loaded `TextGrid` or a `TextGridError`.
    ///
    /// # Errors
    /// - `TextGridError::Format` if the file extension is unsupported or missing, or if the file is malformed.
    /// - `TextGridError::IO` if the file cannot be opened or read.
    ///
    /// # Examples
    /// ```rust
    /// let tg = TextGrid::from_file("example.TextGrid").unwrap();
    /// assert_eq!(tg.tiers.len(), 1); // Assuming one tier in the file
    /// ```
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, TextGridError> {
        let path_ref = path.as_ref();
        if let Some(ext) = path_ref.extension() {
            match ext.to_str().unwrap_or("").to_lowercase().as_str() {
                "textgrid" => {
                    let textgrid = parser::parse_textgrid(path)?;
                    validator::validate_textgrid(&textgrid)?;
                    Ok(textgrid)
                }
                "textgridbin" => {
                    let textgrid = binary::read_binary(path)?;
                    validator::validate_textgrid(&textgrid)?;
                    Ok(textgrid)
                }
                _ => Err(TextGridError::Format("Unsupported file extension".into())),
            }
        } else {
            Err(TextGridError::Format("No file extension".into()))
        }
    }

    /// Writes a TextGrid to a file in text format.
    ///
    /// # Arguments
    /// * `path` - Path to the output file, implementing `AsRef<Path>`.
    /// * `short_format` - If `true`, uses the short text format; otherwise, uses the long format.
    ///
    /// # Returns
    /// Returns a `Result` indicating success (`Ok(())`) or a `TextGridError`.
    ///
    /// # Errors
    /// - `TextGridError::Format` if the TextGrid data is invalid (e.g., overlapping intervals).
    /// - `TextGridError::IO` if the file cannot be created or written to.
    ///
    /// # Examples
    /// ```rust
    /// let tg = TextGrid::new(0.0, 5.0).unwrap(); // Assume tiers are added
    /// tg.to_file("test.TextGrid", false).unwrap();
    /// ```
    pub fn to_file<P: AsRef<Path>>(&self, path: P, short_format: bool) -> Result<(), TextGridError> {
        validator::validate_textgrid(self)?;
        writer::write_textgrid(self, path, short_format)
    }

    /// Writes a TextGrid to a file in binary format.
    ///
    /// # Arguments
    /// * `path` - Path to the output file, implementing `AsRef<Path>`.
    ///
    /// # Returns
    /// Returns a `Result` indicating success (`Ok(())`) or a `TextGridError`.
    ///
    /// # Errors
    /// - `TextGridError::Format` if the TextGrid data is invalid (e.g., overlapping intervals).
    /// - `TextGridError::IO` if the file cannot be created or written to.
    ///
    /// # Examples
    /// ```rust
    /// let tg = TextGrid::new(0.0, 5.0).unwrap(); // Assume tiers are added
    /// tg.to_binary_file("test.textgridbin").unwrap();
    /// ```
    pub fn to_binary_file<P: AsRef<Path>>(&self, path: P) -> Result<(), TextGridError> {
        validator::validate_textgrid(self)?;
        binary::write_binary(self, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_undo_redo() {
        let mut textgrid = TextGrid::new(0.0, 5.0).unwrap();
        let tier = Tier {
            name: "test".to_string(),
            tier_type: TierType::IntervalTier,
            xmin: 0.0,
            xmax: 5.0,
            intervals: vec![Interval { xmin: 0.0, xmax: 5.0, text: "test".to_string() }],
            points: vec![],
        };
        textgrid.add_tier(tier.clone()).unwrap();
        assert_eq!(textgrid.tiers.len(), 1);
        textgrid.undo().unwrap();
        assert_eq!(textgrid.tiers.len(), 0);
        textgrid.redo().unwrap();
        assert_eq!(textgrid.tiers.len(), 1);

        textgrid.tier_add_interval("test", Interval { xmin: 2.0, xmax: 3.0, text: "new".to_string() }).unwrap();
        assert_eq!(textgrid.get_tier("test").unwrap().intervals.len(), 2);
        textgrid.undo().unwrap();
        assert_eq!(textgrid.get_tier("test").unwrap().intervals.len(), 1);
    }

    #[test]
    fn test_advanced_merge() {
        let mut textgrid = TextGrid::new(0.0, 5.0).unwrap();
        textgrid.add_tier(Tier {
            name: "t1".to_string(),
            tier_type: TierType::IntervalTier,
            xmin: 0.0,
            xmax: 5.0,
            intervals: vec![Interval { xmin: 0.0, xmax: 2.0, text: "a".to_string() }],
            points: vec![],
        }).unwrap();
        textgrid.add_tier(Tier {
            name: "t2".to_string(),
            tier_type: TierType::IntervalTier,
            xmin: 0.0,
            xmax: 5.0,
            intervals: vec![Interval { xmin: 1.0, xmax: 3.0, text: "b".to_string() }],
            points: vec![],
        }).unwrap();
        textgrid.merge_tiers_with_strategy("t1", "t2", "merged".to_string(), |a, b| {
            Some(Interval {
                xmin: a.xmin,
                xmax: a.xmax.max(b.xmax),
                text: format!("{}-{}", a.text, b.text),
            })
        }).unwrap();
        let merged = textgrid.get_tier("merged").unwrap();
        assert_eq!(merged.intervals[0].text, "a-b");
    }

    #[test]
    fn test_query() {
        let mut textgrid = TextGrid::new(0.0, 5.0).unwrap();
        textgrid.add_tier(Tier {
            name: "test".to_string(),
            tier_type: TierType::IntervalTier,
            xmin: 0.0,
            xmax: 5.0,
            intervals: vec![Interval { xmin: 0.0, xmax: 2.0, text: "hello".to_string() }],
            points: vec![],
        }).unwrap();
        let results = textgrid.query_intervals_by_text("hello");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1[0].text, "hello");
    }

    #[test]
    fn test_binary() {
        let mut textgrid = TextGrid::new(0.0, 5.0).unwrap();
        textgrid.add_tier(Tier {
            name: "test".to_string(),
            tier_type: TierType::IntervalTier,
            xmin: 0.0,
            xmax: 5.0,
            intervals: vec![Interval { xmin: 0.0, xmax: 2.0, text: "hello".to_string() }],
            points: vec![],
        }).unwrap();
        textgrid.to_binary_file("test.textgridbin").unwrap();
        let loaded = TextGrid::from_file("test.textgridbin").unwrap();
        assert_eq!(loaded.tiers[0].intervals[0].text, "hello");
        std::fs::remove_file("test.textgridbin").unwrap();
    }
}