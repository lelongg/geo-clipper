# geo-clipper

This crate allows to perform boolean and offset operations on polygons.

[![crate.io](https://img.shields.io/crates/v/geo-clipper.svg)](https://crates.io/crates/geo-clipper)
[![docs.rs](https://docs.rs/geo-clipper/badge.svg)](https://docs.rs/geo-clipper)

It makes use of [clipper-sys](https://github.com/lelongg/clipper-sys) which is a binding to the C++ version of [Clipper](http://www.angusj.com/delphi/clipper.php).

## Example

The following example shows how to compute the intersection of two polygons.
The [`intersection`] method (as well as [`difference`], [`union`] and [`xor`]) is provided by the [`Clipper`] trait which is implemented for some [geo-types](https://docs.rs/geo-types/0.4.3/geo_types/).

```rust
use geo_types::{Coord, LineString, Polygon};
use geo_clipper::Clipper;

let subject = Polygon::new(
    LineString(vec![
        Coord { x: 180.0, y: 200.0 },
        Coord { x: 260.0, y: 200.0 },
        Coord { x: 260.0, y: 150.0 },
        Coord { x: 180.0, y: 150.0 },
    ]),
    vec![LineString(vec![
        Coord { x: 215.0, y: 160.0 },
        Coord { x: 230.0, y: 190.0 },
        Coord { x: 200.0, y: 190.0 },
    ])],
);

let clip = Polygon::new(
    LineString(vec![
        Coord { x: 190.0, y: 210.0 },
        Coord { x: 240.0, y: 210.0 },
        Coord { x: 240.0, y: 130.0 },
        Coord { x: 190.0, y: 130.0 },
    ]),
    vec![],
);

let result = subject.intersection(&clip, 1.0);
```

[`Clipper`]: https://docs.rs/geo-clipper/latest/geo_clipper/trait.Clipper.html
[`intersection`]: https://docs.rs/geo-clipper/latest/geo_clipper/trait.Clipper.html#method.intersection
[`difference`]: https://docs.rs/geo-clipper/latest/geo_clipper/trait.Clipper.html#method.difference
[`union`]: https://docs.rs/geo-clipper/latest/geo_clipper/trait.Clipper.html#method.union
[`xor`]: https://docs.rs/geo-clipper/latest/geo_clipper/trait.Clipper.html#method.xor
