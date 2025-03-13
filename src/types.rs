//! Core data structures and manipulation methods for TextGrid.
//!
//! This module defines the fundamental types for representing and manipulating Praat TextGrid data,
//! including support for tiers (IntervalTiers and PointTiers), intervals, points, and a history
//! mechanism for undo/redo operations.

use std::collections::VecDeque;
use std::fmt;
use thiserror::Error;

// === Error Handling ===

/// Errors that can occur during TextGrid operations.
#[derive(Debug, Error)]
pub enum TextGridError {
    /// Input/output error, typically from file operations.
    #[error("IO error: {0}")]
    IO(std::io::Error),
    /// Formatting or validation error with a descriptive message.
    #[error("Format error: {0}")]
    Format(String),
    /// Error due to invalid time specifications.
    #[error("Invalid time specification")]
    InvalidTime,
}

impl From<std::io::Error> for TextGridError {
    fn from(error: std::io::Error) -> Self {
        TextGridError::IO(error)
    }
}

impl From<std::string::FromUtf8Error> for TextGridError {
    fn from(_: std::string::FromUtf8Error) -> Self {
        TextGridError::Format("Invalid UTF-8 sequence".into())
    }
}

// === Core Types ===

/// Type of a tier, either interval-based or point-based.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TierType {
    /// A tier containing time intervals with text annotations.
    IntervalTier,
    /// A tier containing discrete time points with marks.
    PointTier,
}

/// Represents a time interval with associated text.
#[derive(Debug, Clone)]
pub struct Interval {
    /// Start time of the interval.
    pub xmin: f64,
    /// End time of the interval.
    pub xmax: f64,
    /// Text annotation for the interval.
    pub text: String,
}

/// Represents a single time point with a mark.
#[derive(Debug, Clone)]
pub struct Point {
    /// Time of the point.
    pub time: f64,
    /// Mark or label associated with the point.
    pub mark: String,
}

/// Represents a tier in a TextGrid, containing intervals or points.
#[derive(Debug, Clone)]
pub struct Tier {
    /// Name of the tier.
    pub name: String,
    /// Type of the tier (IntervalTier or PointTier).
    pub tier_type: TierType,
    /// Start time of the tier.
    pub xmin: f64,
    /// End time of the tier.
    pub xmax: f64,
    /// List of intervals (for IntervalTier).
    pub intervals: Vec<Interval>,
    /// List of points (for PointTier).
    pub points: Vec<Point>,
}

/// Represents a change operation for undo/redo.
#[derive(Debug, Clone)]
enum Change {
    AddTier(Tier),
    RemoveTier(usize, Tier),
    AddInterval(String, Interval),
    RemoveInterval(String, usize, Interval),
    AddPoint(String, Point),
    RemovePoint(String, usize, Point),
    SplitInterval(String, usize, Interval, Interval),
    MergeIntervals(String, Vec<Interval>, Vec<Interval>),
    RenameTier(String, String),
    MergeTiers(String, String, String, Tier),
    AdjustBounds(f64, f64),
    InsertSilence(String, Vec<Interval>, Vec<Interval>),
}

/// Main structure representing a Praat TextGrid with tiers and history.
#[derive(Debug)]
pub struct TextGrid {
    /// Start time of the entire TextGrid.
    pub xmin: f64,
    /// End time of the entire TextGrid.
    pub xmax: f64,
    /// List of tiers in the TextGrid.
    pub tiers: Vec<Tier>,
    /// History of changes for undo operations.
    history: VecDeque<Change>,
    /// Stack of undone changes for redo operations.
    redo_stack: VecDeque<Change>,
    /// Maximum number of changes stored in history.
    max_history: usize,
}

impl Interval {
    /// Splits an interval into two at the specified time.
    ///
    /// # Arguments
    /// * `time` - The time at which to split the interval.
    ///
    /// # Returns
    /// Returns a `Result` containing a tuple of the two resulting intervals or a `TextGridError`.
    ///
    /// # Errors
    /// Returns `TextGridError::Format` if the split time is outside the interval bounds.
    pub fn split(&self, time: f64) -> Result<(Interval, Interval), TextGridError> {
        if time <= self.xmin || time >= self.xmax {
            return Err(TextGridError::Format("Split time must be within interval bounds".into()));
        }
        Ok((
            Interval { xmin: self.xmin, xmax: time, text: self.text.clone() },
            Interval { xmin: time, xmax: self.xmax, text: self.text.clone() },
        ))
    }
}

impl Tier {
    /// Adds an interval to an IntervalTier.
    ///
    /// # Arguments
    /// * `interval` - The interval to add.
    ///
    /// # Returns
    /// Returns `Ok(())` on success or a `TextGridError` on failure.
    ///
    /// # Errors
    /// - `TextGridError::Format` if the tier is not an IntervalTier or if the interval is out of bounds.
    pub fn add_interval(&mut self, interval: Interval) -> Result<(), TextGridError> {
        if self.tier_type != TierType::IntervalTier {
            return Err(TextGridError::Format("Cannot add interval to PointTier".into()));
        }
        if interval.xmin < self.xmin || interval.xmax > self.xmax {
            return Err(TextGridError::Format("Interval out of tier bounds".into()));
        }
        self.intervals.push(interval.clone());
        self.sort_intervals();
        Ok(())
    }

    /// Adds a point to a PointTier.
    ///
    /// # Arguments
    /// * `point` - The point to add.
    ///
    /// # Returns
    /// Returns `Ok(())` on success or a `TextGridError` on failure.
    ///
    /// # Errors
    /// - `TextGridError::Format` if the tier is not a PointTier or if the point is out of bounds.
    pub fn add_point(&mut self, point: Point) -> Result<(), TextGridError> {
        if self.tier_type != TierType::PointTier {
            return Err(TextGridError::Format("Cannot add point to IntervalTier".into()));
        }
        if point.time < self.xmin || point.time > self.xmax {
            return Err(TextGridError::Format("Point out of tier bounds".into()));
        }
        self.points.push(point.clone());
        self.sort_points();
        Ok(())
    }

    /// Removes an interval from an IntervalTier by index.
    ///
    /// # Arguments
    /// * `index` - Index of the interval to remove.
    ///
    /// # Returns
    /// Returns the removed `Interval` or a `TextGridError` if the operation fails.
    ///
    /// # Errors
    /// - `TextGridError::Format` if the tier is not an IntervalTier or the index is out of bounds.
    pub fn remove_interval(&mut self, index: usize) -> Result<Interval, TextGridError> {
        if self.tier_type != TierType::IntervalTier || index >= self.intervals.len() {
            return Err(TextGridError::Format("Invalid interval removal".into()));
        }
        Ok(self.intervals.remove(index))
    }

    /// Removes a point from a PointTier by index.
    ///
    /// # Arguments
    /// * `index` - Index of the point to remove.
    ///
    /// # Returns
    /// Returns the removed `Point` or a `TextGridError` if the operation fails.
    ///
    /// # Errors
    /// - `TextGridError::Format` if the tier is not a PointTier or the index is out of bounds.
    pub fn remove_point(&mut self, index: usize) -> Result<Point, TextGridError> {
        if self.tier_type != TierType::PointTier || index >= self.points.len() {
            return Err(TextGridError::Format("Invalid point removal".into()));
        }
        Ok(self.points.remove(index))
    }

    /// Splits an interval at the specified time into two intervals.
    ///
    /// # Arguments
    /// * `index` - Index of the interval to split.
    /// * `time` - Time at which to split the interval.
    ///
    /// # Returns
    /// Returns a tuple of the two new intervals or a `TextGridError` if the operation fails.
    ///
    /// # Errors
    /// - `TextGridError::Format` if the tier is not an IntervalTier, index is invalid, or time is out of bounds.
    pub fn split_interval(&mut self, index: usize, time: f64) -> Result<(Interval, Interval), TextGridError> {
        if self.tier_type != TierType::IntervalTier || index >= self.intervals.len() {
            return Err(TextGridError::Format("Invalid split operation".into()));
        }
        let interval = self.intervals.remove(index);
        let (left, right) = interval.split(time)?;
        self.intervals.insert(index, left.clone());
        self.intervals.insert(index + 1, right.clone());
        Ok((left, right))
    }

    /// Merges adjacent intervals with matching text.
    ///
    /// # Returns
    /// Returns the original intervals before merging or a copy of the current intervals if no merge is possible.
    ///
    /// # Errors
    /// Returns an empty `Ok` vector if the tier is not an IntervalTier or has fewer than 2 intervals.
    pub fn merge_intervals(&mut self) -> Result<Vec<Interval>, TextGridError> {
        if self.tier_type != TierType::IntervalTier || self.intervals.len() <= 1 {
            return Ok(self.intervals.clone());
        }
        self.sort_intervals();
        let before = self.intervals.clone();
        let mut merged = Vec::new();
        let mut current = self.intervals[0].clone();
        for next in self.intervals.iter().skip(1) {
            if current.xmax == next.xmin && current.text == next.text {
                current.xmax = next.xmax;
            } else {
                merged.push(current);
                current = next.clone();
            }
        }
        merged.push(current);
        self.intervals = merged;
        Ok(before)
    }

    /// Sorts intervals by their start time (`xmin`).
    fn sort_intervals(&mut self) {
        self.intervals.sort_by(|a, b| a.xmin.partial_cmp(&b.xmin).unwrap());
    }

    /// Sorts points by their time.
    fn sort_points(&mut self) {
        self.points.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    }

    /// Renames the tier and returns the old name.
    ///
    /// # Arguments
    /// * `new_name` - New name for the tier.
    ///
    /// # Returns
    /// Returns the previous name of the tier.
    pub fn rename(&mut self, new_name: String) -> String {
        let old_name = self.name.clone();
        self.name = new_name;
        old_name
    }

    /// Finds intervals containing the specified time.
    ///
    /// # Arguments
    /// * `time` - Time to search for.
    ///
    /// # Returns
    /// Returns a vector of references to intervals that overlap with the given time.
    pub fn find_intervals_by_time(&self, time: f64) -> Vec<&Interval> {
        if self.tier_type != TierType::IntervalTier {
            return Vec::new();
        }
        self.intervals.iter().filter(|i| i.xmin <= time && i.xmax >= time).collect()
    }

    /// Finds points at the specified time.
    ///
    /// # Arguments
    /// * `time` - Time to search for.
    ///
    /// # Returns
    /// Returns a vector of references to points at the given time.
    pub fn find_points_by_time(&self, time: f64) -> Vec<&Point> {
        if self.tier_type != TierType::PointTier {
            return Vec::new();
        }
        self.points.iter().filter(|p| p.time == time).collect()
    }

    /// Finds intervals containing the specified text substring.
    ///
    /// # Arguments
    /// * `text` - Substring to search for in interval texts.
    ///
    /// # Returns
    /// Returns a vector of references to intervals whose text contains the given substring.
    pub fn find_intervals_by_text(&self, text: &str) -> Vec<&Interval> {
        if self.tier_type != TierType::IntervalTier {
            return Vec::new();
        }
        self.intervals.iter().filter(|i| i.text.contains(text)).collect()
    }
}

impl TextGrid {
    /// Creates a new empty TextGrid with given bounds.
    ///
    /// # Arguments
    /// * `xmin` - Start time of the TextGrid.
    /// * `xmax` - End time of the TextGrid.
    ///
    /// # Returns
    /// Returns a `Result` containing the new `TextGrid` or a `TextGridError`.
    ///
    /// # Errors
    /// Returns `TextGridError::Format` if `xmin` is not less than `xmax`.
    pub fn new(xmin: f64, xmax: f64) -> Result<Self, TextGridError> {
        if xmin >= xmax {
            return Err(TextGridError::Format("xmin must be less than xmax".into()));
        }
        Ok(TextGrid {
            xmin,
            xmax,
            tiers: Vec::new(),
            history: VecDeque::new(),
            redo_stack: VecDeque::new(),
            max_history: 100,
        })
    }

    /// Saves a change to the history stack for undo/redo functionality.
    fn save_change(&mut self, change: Change) {
        if self.history.len() >= self.max_history {
            self.history.pop_front();
        }
        self.history.push_back(change);
        self.redo_stack.clear();
    }

    /// Undoes the last change made to the TextGrid.
    ///
    /// # Returns
    /// Returns `Ok(())` on success or a `TextGridError` if there are no changes to undo or if the undo fails.
    pub fn undo(&mut self) -> Result<(), TextGridError> {
        if let Some(change) = self.history.pop_back() {
            match change {
                Change::AddTier(tier) => {
                    let index = self.tiers.iter().position(|t| t.name == tier.name).ok_or(TextGridError::Format("Tier not found".into()))?;
                    self.tiers.remove(index);
                    self.redo_stack.push_back(Change::AddTier(tier));
                }
                Change::RemoveTier(index, tier) => {
                    self.tiers.insert(index, tier.clone());
                    self.redo_stack.push_back(Change::RemoveTier(index, tier));
                }
                Change::AddInterval(tier_name, interval) => {
                    let tier = self.get_tier_mut(&tier_name).ok_or(TextGridError::Format("Tier not found".into()))?;
                    let index = tier.intervals.iter().position(|i| i.xmin == interval.xmin && i.xmax == interval.xmax && i.text == interval.text).ok_or(TextGridError::Format("Interval not found".into()))?;
                    tier.intervals.remove(index);
                    self.redo_stack.push_back(Change::AddInterval(tier_name, interval));
                }
                Change::RemoveInterval(tier_name, index, interval) => {
                    let tier = self.get_tier_mut(&tier_name).ok_or(TextGridError::Format("Tier not found".into()))?;
                    tier.intervals.insert(index, interval.clone());
                    self.redo_stack.push_back(Change::RemoveInterval(tier_name, index, interval));
                }
                Change::AddPoint(tier_name, point) => {
                    let tier = self.get_tier_mut(&tier_name).ok_or(TextGridError::Format("Tier not found".into()))?;
                    let index = tier.points.iter().position(|p| p.time == point.time && p.mark == point.mark).ok_or(TextGridError::Format("Point not found".into()))?;
                    let removed = tier.points.remove(index);
                    self.redo_stack.push_back(Change::RemovePoint(tier_name, index, removed));
                }
                Change::RemovePoint(tier_name, index, point) => {
                    let tier = self.get_tier_mut(&tier_name).ok_or(TextGridError::Format("Tier not found".into()))?;
                    tier.points.insert(index, point.clone());
                    self.redo_stack.push_back(Change::AddPoint(tier_name, point));
                }
                Change::SplitInterval(tier_name, index, orig, left) => {
                    let tier = self.get_tier_mut(&tier_name).ok_or(TextGridError::Format("Tier not found".into()))?;
                    tier.intervals.remove(index);
                    tier.intervals.remove(index);
                    tier.intervals.insert(index, orig.clone());
                    self.redo_stack.push_back(Change::SplitInterval(tier_name, index, orig, left));
                }
                Change::MergeIntervals(tier_name, before, _) => {
                    let tier = self.get_tier_mut(&tier_name).ok_or(TextGridError::Format("Tier not found".into()))?;
                    let after = tier.intervals.clone();
                    tier.intervals = before.clone();
                    self.redo_stack.push_back(Change::MergeIntervals(tier_name, before, after));
                }
                Change::RenameTier(old_name, new_name) => {
                    let tier = self.get_tier_mut(&new_name).ok_or(TextGridError::Format("Tier not found".into()))?;
                    tier.name = old_name.clone();
                    self.redo_stack.push_back(Change::RenameTier(old_name, new_name));
                }
                Change::MergeTiers(t1, t2, new_name, tier) => {
                    let index = self.tiers.iter().position(|t| t.name == new_name).ok_or(TextGridError::Format("Tier not found".into()))?;
                    self.tiers.remove(index);
                    self.redo_stack.push_back(Change::MergeTiers(t1, t2, new_name, tier));
                }
                Change::AdjustBounds(old_xmin, old_xmax) => {
                    let new_xmin = self.xmin;
                    let new_xmax = self.xmax;
                    self.xmin = old_xmin;
                    self.xmax = old_xmax;
                    for tier in &mut self.tiers {
                        tier.xmin = old_xmin;
                        tier.xmax = old_xmax;
                    }
                    self.redo_stack.push_back(Change::AdjustBounds(new_xmin, new_xmax));
                }
                Change::InsertSilence(tier_name, before, _) => {
                    let tier = self.get_tier_mut(&tier_name).ok_or(TextGridError::Format("Tier not found".into()))?;
                    let after = tier.intervals.clone();
                    tier.intervals = before.clone();
                    self.redo_stack.push_back(Change::InsertSilence(tier_name, before, after));
                }
            }
            Ok(())
        } else {
            Err(TextGridError::Format("No more actions to undo".into()))
        }
    }
    
    /// Redoes the last undone change.
    ///
    /// # Returns
    /// Returns `Ok(())` on success or a `TextGridError` if there are no changes to redo or if the redo fails.
    pub fn redo(&mut self) -> Result<(), TextGridError> {
        if let Some(change) = self.redo_stack.pop_back() {
            match change {
                Change::AddTier(tier) => {
                    self.tiers.push(tier.clone());
                    self.save_change(Change::AddTier(tier));
                }
                Change::RemoveTier(index, tier) => {
                    if index < self.tiers.len() && self.tiers[index].name == tier.name {
                        let removed = self.tiers.remove(index);
                        self.save_change(Change::RemoveTier(index, removed));
                    } else {
                        return Err(TextGridError::Format("Tier not found or index mismatch for redo".into()));
                    }
                }
                Change::AddInterval(tier_name, interval) => {
                    self.tier_add_interval(&tier_name, interval)?;
                }
                Change::RemoveInterval(tier_name, index, interval) => {
                    self.tier_remove_interval(&tier_name, index)?;
                }
                Change::AddPoint(tier_name, point) => {
                    self.tier_add_point(&tier_name, point)?;
                }
                Change::RemovePoint(tier_name, index, point) => {
                    self.tier_remove_point(&tier_name, index)?;
                }
                Change::SplitInterval(tier_name, index, orig, left) => {
                    self.tier_split_interval(&tier_name, index, left.xmax)?;
                }
                Change::MergeIntervals(tier_name, before, after) => {
                    self.tier_merge_intervals(&tier_name)?;
                }
                Change::RenameTier(old_name, new_name) => {
                    self.rename_tier(&old_name, new_name)?;
                }
                Change::MergeTiers(t1, t2, new_name, tier) => {
                    self.add_tier(tier)?;
                }
                Change::AdjustBounds(new_xmin, new_xmax) => {
                    self.adjust_bounds(new_xmin, new_xmax)?;
                }
                Change::InsertSilence(tier_name, before, after) => {
                    let tier = self.get_tier_mut(&tier_name).ok_or(TextGridError::Format("Tier not found".into()))?;
                    tier.intervals = after.clone();
                    self.save_change(Change::InsertSilence(tier_name, before, after));
                }
            }
            Ok(())
        } else {
            Err(TextGridError::Format("No more actions to redo".into()))
        }
    }

    /// Adds a tier to the TextGrid with undo support.
    ///
    /// # Arguments
    /// * `tier` - The tier to add.
    ///
    /// # Returns
    /// Returns `Ok(())` on success or a `TextGridError` if the tier bounds are invalid.
    pub fn add_tier(&mut self, tier: Tier) -> Result<(), TextGridError> {
        if tier.xmin < self.xmin || tier.xmax > self.xmax {
            return Err(TextGridError::Format("Tier bounds must be within TextGrid bounds".into()));
        }
        self.save_change(Change::AddTier(tier.clone()));
        self.tiers.push(tier);
        Ok(())
    }

    /// Removes a tier from the TextGrid by index with undo support.
    ///
    /// # Arguments
    /// * `index` - Index of the tier to remove.
    ///
    /// # Returns
    /// Returns `Ok(())` on success or a `TextGridError` if the index is out of bounds.
    pub fn remove_tier(&mut self, index: usize) -> Result<(), TextGridError> {
        if index >= self.tiers.len() {
            return Err(TextGridError::Format("Tier index out of bounds".into()));
        }
        let tier = self.tiers.remove(index);
        self.save_change(Change::RemoveTier(index, tier));
        Ok(())
    }

    /// Gets a mutable reference to a tier by name.
    ///
    /// # Arguments
    /// * `name` - Name of the tier to find.
    ///
    /// # Returns
    /// Returns an `Option` containing a mutable reference to the tier if found, or `None` if not.
    pub fn get_tier_mut(&mut self, name: &str) -> Option<&mut Tier> {
        self.tiers.iter_mut().find(|t| t.name == name)
    }

    /// Gets an immutable reference to a tier by name.
    ///
    /// # Arguments
    /// * `name` - Name of the tier to find.
    ///
    /// # Returns
    /// Returns an `Option` containing a reference to the tier if found, or `None` if not.
    pub fn get_tier(&self, name: &str) -> Option<&Tier> {
        self.tiers.iter().find(|t| t.name == name)
    }

    /// Renames a tier with undo support.
    ///
    /// # Arguments
    /// * `old_name` - Current name of the tier.
    /// * `new_name` - New name for the tier.
    ///
    /// # Returns
    /// Returns `Ok(())` on success or a `TextGridError` if the tier is not found.
    pub fn rename_tier(&mut self, old_name: &str, new_name: String) -> Result<(), TextGridError> {
        if let Some(tier) = self.get_tier_mut(old_name) {
            let old_name = tier.rename(new_name.clone());
            self.save_change(Change::RenameTier(old_name, new_name));
            Ok(())
        } else {
            Err(TextGridError::Format("Tier not found".into()))
        }
    }

    /// Merges two tiers using a custom strategy with undo support.
    ///
    /// # Arguments
    /// * `name1` - Name of the first tier.
    /// * `name2` - Name of the second tier.
    /// * `new_name` - Name for the resulting merged tier.
    /// * `merge_strategy` - Function to determine how overlapping intervals are merged.
    ///
    /// # Returns
    /// Returns `Ok(())` on success or a `TextGridError` if the tiers are not found or not IntervalTiers.
    pub fn merge_tiers_with_strategy<F>(
        &mut self,
        name1: &str,
        name2: &str,
        new_name: String,
        merge_strategy: F,
    ) -> Result<(), TextGridError>
    where
        F: Fn(&Interval, &Interval) -> Option<Interval>,
    {
        let tier1 = self.get_tier(name1).ok_or(TextGridError::Format("First tier not found".into()))?.clone();
        let tier2 = self.get_tier(name2).ok_or(TextGridError::Format("Second tier not found".into()))?.clone();

        if tier1.tier_type != TierType::IntervalTier || tier2.tier_type != TierType::IntervalTier {
            return Err(TextGridError::Format("Can only merge IntervalTiers".into()));
        }

        let mut combined_intervals = Vec::new();
        combined_intervals.extend(tier1.intervals.clone());
        combined_intervals.extend(tier2.intervals.clone());
        combined_intervals.sort_by(|a, b| a.xmin.partial_cmp(&b.xmin).unwrap());

        let mut new_intervals = Vec::new();
        if !combined_intervals.is_empty() {
            let mut current = combined_intervals[0].clone();
            for next in combined_intervals.iter().skip(1) {
                if current.xmax > next.xmin {
                    if let Some(merged) = merge_strategy(&current, next) {
                        current = merged;
                    } else {
                        new_intervals.push(current);
                        current = next.clone();
                    }
                } else {
                    new_intervals.push(current);
                    current = next.clone();
                }
            }
            new_intervals.push(current);
        }

        let new_tier = Tier {
            name: new_name.clone(),
            tier_type: TierType::IntervalTier,
            xmin: self.xmin,
            xmax: self.xmax,
            intervals: new_intervals,
            points: Vec::new(),
        };
        self.save_change(Change::MergeTiers(name1.to_string(), name2.to_string(), new_name, new_tier.clone()));
        self.tiers.push(new_tier);
        Ok(())
    }

    /// Merges two tiers with a default strategy (merges if text matches or one is empty).
    ///
    /// # Arguments
    /// * `name1` - Name of the first tier.
    /// * `name2` - Name of the second tier.
    /// * `new_name` - Name for the resulting merged tier.
    ///
    /// # Returns
    /// Returns `Ok(())` on success or a `TextGridError` if the tiers are not found or not IntervalTiers.
    pub fn merge_tiers(&mut self, name1: &str, name2: &str, new_name: String) -> Result<(), TextGridError> {
        self.merge_tiers_with_strategy(name1, name2, new_name, |current, next| {
            if current.text == next.text || current.text.is_empty() || next.text.is_empty() {
                Some(Interval {
                    xmin: current.xmin,
                    xmax: current.xmax.max(next.xmax),
                    text: if current.text.is_empty() { next.text.clone() } else { current.text.clone() },
                })
            } else {
                None
            }
        })
    }

    /// Adjusts the bounds of the TextGrid and all tiers.
    ///
    /// # Arguments
    /// * `new_xmin` - New start time.
    /// * `new_xmax` - New end time.
    ///
    /// # Returns
    /// Returns `Ok(())` on success or a `TextGridError` if the new bounds are invalid or don't encompass all data.
    pub fn adjust_bounds(&mut self, new_xmin: f64, new_xmax: f64) -> Result<(), TextGridError> {
        if new_xmin >= new_xmax {
            return Err(TextGridError::Format("New xmin must be less than xmax".into()));
        }
        for tier in &self.tiers {
            if !tier.intervals.is_empty() && (tier.intervals[0].xmin < new_xmin || tier.intervals.last().unwrap().xmax > new_xmax) {
                return Err(TextGridError::Format("New bounds must encompass all tier data".into()));
            }
            if !tier.points.is_empty() && (tier.points[0].time < new_xmin || tier.points.last().unwrap().time > new_xmax) {
                return Err(TextGridError::Format("New bounds must encompass all tier data".into()));
            }
        }
        self.save_change(Change::AdjustBounds(self.xmin, self.xmax));
        self.xmin = new_xmin;
        self.xmax = new_xmax;
        for tier in &mut self.tiers {
            tier.xmin = new_xmin;
            tier.xmax = new_xmax;
        }
        Ok(())
    }

    /// Inserts a silent interval into an IntervalTier.
    ///
    /// # Arguments
    /// * `tier_name` - Name of the tier to modify.
    /// * `start` - Start time of the silence.
    /// * `end` - End time of the silence.
    ///
    /// # Returns
    /// Returns `Ok(())` on success or a `TextGridError` if the tier is not found, not an IntervalTier, or bounds are invalid.
    pub fn insert_silence(&mut self, tier_name: &str, start: f64, end: f64) -> Result<(), TextGridError> {
        let tier = self.get_tier_mut(tier_name).ok_or(TextGridError::Format("Tier not found".into()))?;
        if tier.tier_type != TierType::IntervalTier {
            return Err(TextGridError::Format("Can only insert silence in IntervalTier".into()));
        }
        if start < tier.xmin || end > tier.xmax || start >= end {
            return Err(TextGridError::Format("Invalid silence bounds".into()));
        }

        let before = tier.intervals.clone();
        let mut new_intervals = Vec::new();
        for interval in &before {
            if interval.xmax <= start || interval.xmin >= end {
                new_intervals.push(interval.clone());
            } else {
                if interval.xmin < start {
                    new_intervals.push(Interval { xmin: interval.xmin, xmax: start, text: interval.text.clone() });
                }
                if interval.xmax > end {
                    new_intervals.push(Interval { xmin: end, xmax: interval.xmax, text: interval.text.clone() });
                }
            }
        }
        new_intervals.push(Interval { xmin: start, xmax: end, text: "".to_string() });
        tier.intervals = new_intervals.clone();
        tier.sort_intervals();
        self.save_change(Change::InsertSilence(tier_name.to_string(), before, new_intervals));
        Ok(())
    }

    /// Queries all tiers for intervals containing the specified time.
    ///
    /// # Arguments
    /// * `time` - Time to search for.
    ///
    /// # Returns
    /// Returns a vector of tuples containing tiers and their matching intervals.
    pub fn query_intervals_by_time(&self, time: f64) -> Vec<(&Tier, Vec<&Interval>)> {
        self.tiers.iter().map(|t| (t, t.find_intervals_by_time(time))).filter(|(_, v)| !v.is_empty()).collect()
    }

    /// Queries all tiers for points at the specified time.
    ///
    /// # Arguments
    /// * `time` - Time to search for.
    ///
    /// # Returns
    /// Returns a vector of tuples containing tiers and their matching points.
    pub fn query_points_by_time(&self, time: f64) -> Vec<(&Tier, Vec<&Point>)> {
        self.tiers.iter().map(|t| (t, t.find_points_by_time(time))).filter(|(_, v)| !v.is_empty()).collect()
    }

    /// Queries all tiers for intervals containing the specified text substring.
    ///
    /// # Arguments
    /// * `text` - Substring to search for in interval texts.
    ///
    /// # Returns
    /// Returns a vector of tuples containing tiers and their matching intervals.
    pub fn query_intervals_by_text(&self, text: &str) -> Vec<(&Tier, Vec<&Interval>)> {
        self.tiers.iter().map(|t| (t, t.find_intervals_by_text(text))).filter(|(_, v)| !v.is_empty()).collect()
    }

    /// Adds an interval to a tier with undo support.
    ///
    /// # Arguments
    /// * `tier_name` - Name of the tier.
    /// * `interval` - Interval to add.
    ///
    /// # Returns
    /// Returns `Ok(())` on success or a `TextGridError` if the tier is not found or operation fails.
    pub fn tier_add_interval(&mut self, tier_name: &str, interval: Interval) -> Result<(), TextGridError> {
        let tier = self.get_tier_mut(tier_name).ok_or(TextGridError::Format("Tier not found".into()))?;
        tier.add_interval(interval.clone())?;
        self.save_change(Change::AddInterval(tier_name.to_string(), interval));
        Ok(())
    }

    /// Removes an interval from a tier with undo support.
    ///
    /// # Arguments
    /// * `tier_name` - Name of the tier.
    /// * `index` - Index of the interval to remove.
    ///
    /// # Returns
    /// Returns `Ok(())` on success or a `TextGridError` if the tier is not found or operation fails.
    pub fn tier_remove_interval(&mut self, tier_name: &str, index: usize) -> Result<(), TextGridError> {
        let tier = self.get_tier_mut(tier_name).ok_or(TextGridError::Format("Tier not found".into()))?;
        let interval = tier.remove_interval(index)?;
        self.save_change(Change::RemoveInterval(tier_name.to_string(), index, interval));
        Ok(())
    }

    /// Adds a point to a tier with undo support.
    ///
    /// # Arguments
    /// * `tier_name` - Name of the tier.
    /// * `point` - Point to add.
    ///
    /// # Returns
    /// Returns `Ok(())` on success or a `TextGridError` if the tier is not found or operation fails.
    pub fn tier_add_point(&mut self, tier_name: &str, point: Point) -> Result<(), TextGridError> {
        let tier = self.get_tier_mut(tier_name).ok_or(TextGridError::Format("Tier not found".into()))?;
        tier.add_point(point.clone())?;
        self.save_change(Change::AddPoint(tier_name.to_string(), point));
        Ok(())
    }

    /// Removes a point from a tier with undo support.
    ///
    /// # Arguments
    /// * `tier_name` - Name of the tier.
    /// * `index` - Index of the point to remove.
    ///
    /// # Returns
    /// Returns `Ok(())` on success or a `TextGridError` if the tier is not found or operation fails.
    pub fn tier_remove_point(&mut self, tier_name: &str, index: usize) -> Result<(), TextGridError> {
        let tier = self.get_tier_mut(tier_name).ok_or(TextGridError::Format("Tier not found".into()))?;
        let point = tier.remove_point(index)?;
        self.save_change(Change::RemovePoint(tier_name.to_string(), index, point));
        Ok(())
    }

    /// Splits an interval in a tier with undo support.
    ///
    /// # Arguments
    /// * `tier_name` - Name of the tier.
    /// * `index` - Index of the interval to split.
    /// * `time` - Time at which to split the interval.
    ///
    /// # Returns
    /// Returns `Ok(())` on success or a `TextGridError` if the tier is not found or operation fails.
    pub fn tier_split_interval(&mut self, tier_name: &str, index: usize, time: f64) -> Result<(), TextGridError> {
        let tier = self.get_tier_mut(tier_name).ok_or(TextGridError::Format("Tier not found".into()))?;
        let orig = tier.intervals[index].clone();
        let (left, right) = tier.split_interval(index, time)?;
        self.save_change(Change::SplitInterval(tier_name.to_string(), index, orig, left));
        Ok(())
    }

    /// Merges intervals in a tier with undo support.
    ///
    /// # Arguments
    /// * `tier_name` - Name of the tier.
    ///
    /// # Returns
    /// Returns `Ok(())` on success or a `TextGridError` if the tier is not found or operation fails.
    pub fn tier_merge_intervals(&mut self, tier_name: &str) -> Result<(), TextGridError> {
        let tier = self.get_tier_mut(tier_name).ok_or(TextGridError::Format("Tier not found".into()))?;
        let before = tier.merge_intervals()?;
        let after = tier.intervals.clone();
        self.save_change(Change::MergeIntervals(tier_name.to_string(), before, after));
        Ok(())
    }
}

impl TextGrid {
    /// Adds tiers to an existing TextGrid and returns the modified instance.
    ///
    /// # Arguments
    /// * `tiers` - Vector of tiers to add.
    ///
    /// # Returns
    /// Returns the `TextGrid` with the added tiers.
    pub fn with_tiers(mut self, tiers: Vec<Tier>) -> Self {
        self.tiers = tiers;
        self
    }
}