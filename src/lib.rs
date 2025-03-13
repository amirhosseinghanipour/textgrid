mod parser;
mod types;
mod writer;
mod validator;

pub use types::{Interval, Point, TextGrid, TextGridError, Tier, TierType};
use std::path::Path;

impl TextGrid {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, TextGridError> {
        let textgrid = parser::parse_textgrid(path)?;
        validator::validate_textgrid(&textgrid)?;
        Ok(textgrid)
    }

    pub fn to_file<P: AsRef<Path>>(&self, path: P, short_format: bool) -> Result<(), TextGridError> {
        validator::validate_textgrid(self)?;
        writer::write_textgrid(self, path, short_format)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_long_format_roundtrip() {
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
        let path = "test_long.TextGrid";
        std::fs::write(path, sample).unwrap();
        let textgrid = TextGrid::from_file(path).unwrap();
        let out_path = "test_long_out.TextGrid";
        textgrid.to_file(out_path, false).unwrap();
        let reread = TextGrid::from_file(out_path).unwrap();
        std::fs::remove_file(path).unwrap();
        std::fs::remove_file(out_path).unwrap();

        assert_eq!(textgrid.xmin, reread.xmin);
        assert_eq!(textgrid.tiers[0].intervals[0].text, reread.tiers[0].intervals[0].text);
    }

    #[test]
    fn test_short_format_roundtrip() {
        let sample = r#"File type = "ooTextFile"
Object class = "TextGrid"
0
2.53
1
"IntervalTier"
"words"
0
2.53
2
0
1.125
"Hello"
1.125
2.53
"World"
"#;
        let path = "test_short.TextGrid";
        std::fs::write(path, sample).unwrap();
        let textgrid = TextGrid::from_file(path).unwrap();
        let out_path = "test_short_out.TextGrid";
        textgrid.to_file(out_path, true).unwrap();
        let reread = TextGrid::from_file(out_path).unwrap();
        std::fs::remove_file(path).unwrap();
        std::fs::remove_file(out_path).unwrap();

        assert_eq!(textgrid.xmin, reread.xmin);
        assert_eq!(textgrid.tiers[0].intervals[0].text, reread.tiers[0].intervals[0].text);
    }

    #[test]
    fn test_validation() {
        let mut textgrid = TextGrid {
            xmin: 0.0,
            xmax: 2.0,
            tiers: vec![Tier {
                name: "test".to_string(),
                tier_type: TierType::IntervalTier,
                xmin: 0.0,
                xmax: 2.0,
                intervals: vec![
                    Interval { xmin: 0.0, xmax: 1.0, text: "a".to_string() },
                    Interval { xmin: 0.5, xmax: 2.0, text: "b".to_string() }, // Overlap
                ],
                points: vec![],
            }],
        };
        assert!(validator::validate_textgrid(&textgrid).is_err());

        textgrid.tiers[0].intervals[1].xmin = 1.0; // Fix overlap
        assert!(validator::validate_textgrid(&textgrid).is_ok());
    }
}