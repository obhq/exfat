# exFAT in pure Rust
[![Crates.io](https://img.shields.io/crates/v/exfat)](https://crates.io/crates/exfat)

This is an implementation of exFAT in pure Rust. Currently it is supports only reading, not writing; and not all features is implemented but if all you need is listing the directories and read the files then you are good to go.

## Usage

```rust
use exfat::image::Image;
use std::fs::File;

let image = File::open("exfat.img").expect("cannot open exfat.img");
let image = Image::open(image).expect("cannot open exFAT image from exfat.img");
let root = Root::open(image).expect("cannot open the root directory");

for item in root {
    // item will be either file or directory.
}
```

## Breaking changes

### 0.1 to 0.2

My Rust skill has improved a lot since version 0.1 so I take this semver breaking change to make a lot of things better. That mean version 0.2 is not compatible with 0.1 in any ways.

## License

MIT
