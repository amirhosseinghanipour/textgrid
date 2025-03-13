# textgrid
TextGrid is a Rust crate for working with Praat .TextGrid files, providing parsing, writing, manipulation, and history tracking for TextGrid data. This library is designed for researchers, linguists, and developers working with phonetic annotations or time-aligned data, supporting both text (long/short) and binary formats as per Praat's specification.

## Features
- Parsing and Writing: Read and write TextGrid files in long/short text formats and Praat's binary format.
- Manipulation: Add, remove, split, merge, and query tiers, intervals, and points with undo/redo support.
- Validation: Ensure data integrity with bounds and overlap checks.
- Extensibility: Flexible tier merging with custom strategies and robust querying capabilities.
- Performance: Efficient data structures with optional history tracking for operations.

## Installation
Add `textgrid` to your `Cargo.toml`:
```toml
[dependencies]
textgrid = "0.1.0"
```
Or use `cargo add`:
```bash
cargo add textgrid
```

## Usage
### Creating and manipulating a TextGrid
```rust
use textgrid::{TextGrid, Tier, TierType, Interval};

fn main() -> Result<(), textgrid::TextGridError> {
    // Create a new TextGrid
    let mut tg = TextGrid::new(0.0, 10.0)?;

    // Add an IntervalTier
    let tier = Tier {
        name: "words".to_string(),
        tier_type: TierType::IntervalTier,
        xmin: 0.0,
        xmax: 10.0,
        intervals: vec![Interval { xmin: 1.0, xmax: 2.0, text: "hello".to_string() }],
        points: vec![],
    };
    tg.add_tier(tier)?;

    // Split an interval
    tg.tier_split_interval("words", 0, 1.5)?;

    // Undo the split
    tg.undo()?;

    // Save to a text file
    tg.to_file("example.TextGrid", false)?;

    Ok(())
}
```
### Loading and querying a TextGrid
```rust
use textgrid::TextGrid;

fn main() -> Result<(), textgrid::TextGridError> {
    // Load from a file
    let tg = TextGrid::from_file("example.TextGrid")?;

    // Query intervals by text
    let results = tg.query_intervals_by_text("hello");
    for (tier, intervals) in results {
        println!("Tier '{}': {:?}", tier.name, intervals);
    }

    Ok(())
}
```
### Working with binary format
```rust
use textgrid::{TextGrid, Tier, TierType, Interval};

fn main() -> Result<(), textgrid::TextGridError> {
    // Create and save a TextGrid in binary format
    let mut tg = TextGrid::new(0.0, 5.0)?;
    tg.add_tier(Tier {
        name: "test".to_string(),
        tier_type: TierType::IntervalTier,
        xmin: 0.0,
        xmax: 5.0,
        intervals: vec![Interval { xmin: 0.0, xmax: 2.0, text: "hello".to_string() }],
        points: vec![],
    })?;
    tg.to_binary_file("test.textgridbin")?;

    // Load it back
    let loaded = TextGrid::from_file("test.textgridbin")?;
    assert_eq!(loaded.tiers[0].intervals[0].text, "hello");

    Ok(())
}
```
### Custom tier merging
```rust
use textgrid::{TextGrid, Tier, TierType, Interval};

fn main() -> Result<(), textgrid::TextGridError> {
    let mut tg = TextGrid::new(0.0, 5.0)?;
    tg.add_tier(Tier {
        name: "t1".to_string(),
        tier_type: TierType::IntervalTier,
        xmin: 0.0,
        xmax: 5.0,
        intervals: vec![Interval { xmin: 0.0, xmax: 2.0, text: "a".to_string() }],
        points: vec![],
    })?;
    tg.add_tier(Tier {
        name: "t2".to_string(),
        tier_type: TierType::IntervalTier,
        xmin: 0.0,
        xmax: 5.0,
        intervals: vec![Interval { xmin: 1.0, xmax: 3.0, text: "b".to_string() }],
        points: vec![],
    })?;

    // Merge tiers with a custom strategy
    tg.merge_tiers_with_strategy("t1", "t2", "merged".to_string(), |a, b| {
        Some(Interval {
            xmin: a.xmin,
            xmax: a.xmax.max(b.xmax),
            text: format!("{}-{}", a.text, b.text),
        })
    })?;

    let merged = tg.get_tier("merged").unwrap();
    assert_eq!(merged.intervals[0].text, "a-b");

    Ok(())
}
```

## API Documentation
Full API documentation is available on (Docs.rs)[docs.rs]. Key components include:
- ``TextGrid`: Main structure with tiers and history.
- ``Tier`: Represents an IntervalTier or PointTier.
- ``Interval` and `Point`: Data types for annotations.
- ``TextGridError`: Error handling for I/O and format issues.

## Building and Testing
To build and test the crate locally:
```bash
cargo build
cargo test
```
The test suite include checks for undo/redo, tier merging, querying, and binary I/O.

## Contributing
Contributions are welcome! Please follow these steps:
1. Fork the repository.
2. Create a feature branch (`git checkout -b feature/your-feature`).
3. Commit your changes (`git commit -am 'Add your feature'`).
4. Push to the branch (`git push origin feature/your-feature`).
5. Open a pull request.
Please ensure your code passes `cargo test` and adheres to Rust formatting (`cargo fmt`).

## License
This project is licensed under the CC-BY-NC-4.0 License.

## Acknowledgments
- Built with Rust for performance and safety.
- Inspired by Praat's TextGrid format for phonetic research.