# giffy

[![crates.io](https://img.shields.io/crates/v/giffy.svg)](https://crates.io/crates/giffy)

A simple GIF decoder written in Rust.

## Usage
```rust
use giffy;
use std::fs::File;

let mut src = File::open("<gif path>").expect("File not found");
match giffy::load(&mut src) {
    Ok(gif) => {
        for frame in gif.image_frames {
            // do something with the frame
        }
    }

    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```

## Try it
```
cargo run --example example <gif file path> <output folder path>
```

### Disclaimer
At this time, this decoder is meant to be for educational/learning purposes only.
