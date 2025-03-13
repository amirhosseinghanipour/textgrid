//! Validation logic for TextGrid data.
//!
//! This module provides functionality to validate the integrity of a `TextGrid` structure.
//! It checks for consistent time bounds, non-overlapping intervals, and proper tier alignment,
//! ensuring the data adheres to the expected constraints of a Praat TextGrid.
//!
//! ## Validation Checks
//! - **TextGrid Bounds**: Ensures `xmin < xmax`.
//! - **Tier Bounds**: Verifies each tier's bounds are within the TextGrid's bounds and `xmin < xmax`.
//! - **IntervalTiers**: Confirms intervals are non-overlapping, sequential, and have valid bounds (`xmin < xmax`).
//! - **PointTiers**: Ensures all points fall within the tier's time bounds.
//!
//! ## Usage
//! ```rust
//! use textgrid::{TextGrid, Tier, TierType, Interval, validate_textgrid};
//!
//! fn main() -> Result<(), textgrid::TextGridError> {
//!     // Create a valid TextGrid
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
//!     // Validate the TextGrid
//!     validate_textgrid(&tg)?;
//!     println!("TextGrid is valid!");
//!     Ok(())
//! }
//! ```

use crate::types::{TextGrid, TextGridError, TierType};

/// Validates the integrity of a `TextGrid` structure.
///
/// # Arguments
/// * `textgrid` - The `TextGrid` to validate.
///
/// # Returns
/// Returns a `Result` indicating success (`Ok(())`) or a `TextGridError` if validation fails.
///
/// # Errors
/// - `TextGridError::Format` if any of the following conditions are met:
///   - TextGrid `xmin >= xmax`.
///   - Tier bounds are outside TextGrid bounds or `xmin >= xmax`.
///   - IntervalTiers have overlapping or invalid intervals (`xmin >= xmax`).
///   - PointTiers have points outside tier bounds.
///
/// # Examples
/// ```rust
/// let tg = TextGrid::new(0.0, 5.0).unwrap(); // Assume valid tiers are added
/// assert!(textgrid::validate_textgrid(&tg).is_ok());
/// ```
pub fn validate_textgrid(textgrid: &TextGrid) -> Result<(), TextGridError> {
    if textgrid.xmin >= textgrid.xmax {
        return Err(TextGridError::Format("TextGrid xmin must be less than xmax".into()));
    }

    for tier in &textgrid.tiers {
        if tier.xmin < textgrid.xmin || tier.xmax > textgrid.xmax {
            return Err(TextGridError::Format("Tier bounds must be within TextGrid bounds".into()));
        }
        if tier.xmin >= tier.xmax {
            return Err(TextGridError::Format("Tier xmin must be less than xmax".into()));
        }

        match tier.tier_type {
            TierType::IntervalTier => {
                if tier.intervals.is_empty() {
                    continue;
                }
                let mut prev_xmax = tier.xmin;
                for interval in &tier.intervals {
                    if interval.xmin < prev_xmax {
                        return Err(TextGridError::Format("Overlapping intervals detected".into()));
                    }
                    if interval.xmin >= interval.xmax {
                        return Err(TextGridError::Format("Interval xmin must be less than xmax".into()));
                    }
                    prev_xmax = interval.xmax;
                }
            }
            TierType::PointTier => {
                for point in &tier.points {
                    if point.time < tier.xmin || point.time > tier.xmax {
                        return Err(TextGridError::Format("Point time out of tier bounds".into()));
                    }
                }
            }
        }
    }
    Ok(())
}