use thiserror::Error;

#[derive(Error, Debug)]
pub enum TextGridError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
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