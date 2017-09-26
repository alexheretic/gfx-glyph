gfx_glyph
<a href="https://crates.io/crates/gfx_glyph">
  <img src="http://img.shields.io/crates/v/gfx_glyph.svg">
</a>
<a href="https://docs.rs/gfx_glyph">
  <img src="https://docs.rs/gfx_glyph/badge.svg">
</a>
================

Fast GPU cached text rendering using [gfx-rs](https://github.com/gfx-rs/gfx) & [rusttype](https://github.com/dylanede/rusttype).

Makes use of three kinds of caching to optimise frame performance.

* Caching of glyph positioning output to avoid repeated cost of identical text
rendering on sequential frames.
* Caches draw calculations to avoid repeated cost of identical text rendering on
sequential frames.
* GPU cache logic to dynamically maintain a GPU texture of rendered glyphs.

```rust
extern crate gfx_glyph;
use gfx_glyph::{Section, GlyphBrushBuilder};

let arial: &[u8] = include_bytes!("GaramondNo8-Reg.ttf");
let mut glyph_brush = GlyphBrushBuilder::using_font(arial)
    .build(gfx_factory.clone());

let section = Section {
    text: "Hello gfx_glyph",
    ..Section::default() // color, position, etc
};

glyph_brush.queue(section);
glyph_brush.queue(some_other_section);

glyph_brush.draw_queued(&mut gfx_encoder, &gfx_color, &gfx_depth).unwrap();
```

## Limitations
The current implementation only supports OpenGL 3.0 or later. But other rendering languages (that are supported by gfx) should be easy enough to add. Send in your PRs!

## Examples
Have a look at
* `cargo run --example paragraph --release`
* `cargo run --example performance --release`
* `cargo run --example varied --release`
* `cargo run --example depth --release`
