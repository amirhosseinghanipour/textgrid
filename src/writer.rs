use crate::types::{TextGrid, TextGridError, TierType};
use std::fs::File;
use std::io::{Write};
use std::path::Path;

pub fn write_textgrid<P: AsRef<Path>>(textgrid: &TextGrid, path: P, short_format: bool) -> Result<(), TextGridError> {
    let mut file = File::create(path)?;
    if short_format {
        write_short_format(&mut file, textgrid)?;
    } else {
        write_long_format(&mut file, textgrid)?;
    }
    Ok(())
}

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