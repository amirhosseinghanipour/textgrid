use crate::types::{TextGrid, TextGridError, TierType};

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
                if prev_xmax != tier.xmax {
                    return Err(TextGridError::Format("Intervals do not cover the entire tier".into()));
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