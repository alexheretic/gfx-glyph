use super::*;
use linked_hash_map::LinkedHashMap;

/// Cut down version of a [`GlyphBrush`](struct.GlyphBrush.html) that can calculate pixel bounds,
/// but is unable to actually render anything.
///
/// Build using a [`HeadlessGlyphBrushBuilder`](struct.HeadlessGlyphBrushBuilder.html).
///
/// # Example
///
/// ```no_run
/// # extern crate gfx;
/// # extern crate gfx_window_glutin;
/// # extern crate glutin;
/// extern crate gfx_glyph;
/// use gfx_glyph::{HeadlessGlyphBrushBuilder, Section};
/// # fn main() {
///
/// let arial: &[u8] = include_bytes!("../examples/Arial Unicode.ttf");
/// let mut glyphs = HeadlessGlyphBrushBuilder::using_font_bytes(arial).build();
///
/// let section = Section {
///     text: "Hello gfx_glyph",
///     ..Section::default()
/// };
///
/// let bounds = glyphs.pixel_bounds(section);
/// # let _ = bounds;
/// # let _ = glyphs;
/// # }
/// ```
///
/// # Caching behaviour
///
/// Calls to [`GlyphBrush::pixel_bounds`](#method.pixel_bounds),
/// [`GlyphBrush::glyphs`](#method.glyphs) calculate the positioned glyphs for a section.
/// This is cached so future calls to any of the methods for the same section are much
/// cheaper.
///
/// Unlike a [`GlyphBrush`](struct.GlyphBrush.html) the cache has a maximum size and evicts
/// older sections when full, this is because there is no concept of actually drawing the section
/// to imply a section is no longer used.
/// [`HeadlessGlyphBrushBuilder::glyph_positioning_cache_size`](struct.HeadlessGlyphBrushBuilder.html#method.glyph_positioning_cache_size)
/// allows setting the maximum cache size.
pub struct HeadlessGlyphBrush<'font> {
    fonts: HashMap<FontId, rusttype::Font<'font>>,

    // cache of section-layout hash -> computed glyphs, this avoid repeated glyph computation
    // for identical layout/sections common to repeated frame rendering
    calculate_glyph_cache: LinkedHashMap<u64, GlyphedSection<'font>>,

    glyph_positioning_cache_size: usize,
}

impl<'font> HeadlessGlyphBrush<'font> {

    /// Returns the pixel bounding box for the input section using a custom layout.
    /// The box is a conservative whole number pixel rectangle that can contain the section.
    ///
    /// If the section is empty or would result in no drawn glyphs will return `None`
    ///
    /// Benefits from caching, see [caching behaviour](#caching-behaviour).
    pub fn pixel_bounds_custom_layout<'a, S, L>(&mut self, section: S, custom_layout: &L)
        -> Option<Rect<i32>>
        where L: GlyphPositioner + Hash,
              S: Into<Cow<'a, VariedSection<'a>>>,
    {
        let section = section.into();
        let mut x = (0, 0);
        let mut y = (0, 0);
        let mut no_match = true;

        let section_hash = self.cache_glyphs(section.borrow(), custom_layout);

        for g in self.calculate_glyph_cache[&section_hash]
            .glyphs.iter()
            .flat_map(|&GlyphedSectionText(ref g, ..)| g.iter())
        {
            if let Some(Rect{ min, max }) = g.pixel_bounding_box() {
                if no_match || min.x < x.0 { x.0 = min.x; }
                if no_match || min.y < y.0 { y.0 = min.y; }
                if no_match || max.x > x.1 { x.1 = max.x; }
                if no_match || max.y > y.1 { y.1 = max.y; }

                no_match = false;
            }
        }

        if no_match { None }
        else {
            Some(Rect {
                min: Point { x: x.0, y: y.0 },
                max: Point { x: x.1, y: y.1 },
            })
        }
    }

    /// Returns the pixel bounding box for the input section. The box is a conservative
    /// whole number pixel rectangle that can contain the section.
    ///
    /// If the section is empty or would result in no drawn glyphs will return `None`
    ///
    /// Benefits from caching, see [caching behaviour](#caching-behaviour).
    pub fn pixel_bounds<'a, S>(&mut self, section: S)
        -> Option<Rect<i32>>
        where S: Into<Cow<'a, VariedSection<'a>>>,
    {
        let section = section.into();
        let layout = section.layout;
        self.pixel_bounds_custom_layout(section, &layout)
    }

    /// Returns an iterator over the `PositionedGlyph`s of the given section with a custom layout.
    ///
    /// Benefits from caching, see [caching behaviour](#caching-behaviour).
    pub fn glyphs_custom_layout<'a, 'b, S, L>(&'b mut self, section: S, custom_layout: &L)
        -> PositionedGlyphIter<'b, 'font>
        where L: GlyphPositioner + Hash,
              S: Into<Cow<'a, VariedSection<'a>>>,
    {
        let section = section.into();
        let section_hash = self.cache_glyphs(section.borrow(), custom_layout);
        self.calculate_glyph_cache[&section_hash]
            .glyphs.iter()
            .flat_map(|&GlyphedSectionText(ref g, ..)| g.iter())
    }

    /// Returns an iterator over the `PositionedGlyph`s of the given section.
    ///
    /// Benefits from caching, see [caching behaviour](#caching-behaviour).
    pub fn glyphs<'a, 'b, S>(&'b mut self, section: S)
        -> PositionedGlyphIter<'b, 'font>
        where S: Into<Cow<'a, VariedSection<'a>>>,
    {
        let section = section.into();
        let layout = section.layout;
        self.glyphs_custom_layout(section, &layout)
    }

    /// Returns the calculate_glyph_cache key for this sections glyphs
    pub(crate) fn cache_glyphs<L>(&mut self, section: &VariedSection, layout: &L) -> u64
        where L: GlyphPositioner,
    {
        let section_hash = hash(&(section, layout));

        if self.calculate_glyph_cache.get_refresh(&section_hash).is_none() {
            self.calculate_glyph_cache.insert(section_hash, GlyphedSection {
                bounds: layout.bounds_rect(section),
                glyphs: layout.calculate_glyphs(&self.fonts, section),
                z: section.z,
            });
        }
        else {
            while self.calculate_glyph_cache.len() > self.glyph_positioning_cache_size {
                self.calculate_glyph_cache.pop_front();
            }
        }

        section_hash
    }
}


/// Builder for a [`HeadlessGlyphBrush`](struct.HeadlessGlyphBrush.html).
///
/// # Example
///
/// ```no_run
/// # extern crate gfx;
/// # extern crate gfx_window_glutin;
/// # extern crate glutin;
/// extern crate gfx_glyph;
/// use gfx_glyph::HeadlessGlyphBrushBuilder;
/// # fn main() {
///
/// let arial: &[u8] = include_bytes!("../examples/Arial Unicode.ttf");
/// let mut glyphs = HeadlessGlyphBrushBuilder::using_font_bytes(arial).build();
/// # let _ = glyphs;
/// # }
/// ```
pub struct HeadlessGlyphBrushBuilder<'a> {
    font_data: Vec<Font<'a>>,
    glyph_positioning_cache_size: usize,
}

impl<'a> HeadlessGlyphBrushBuilder<'a> {
    /// Specifies the default font data used to render glyphs.
    /// Referenced with `FontId(0)`, which is default.
    pub fn using_font_bytes<B: Into<SharedBytes<'a>>>(font_0_data: B) -> Self {
        Self::using_font(font(font_0_data).unwrap())
    }

    /// Specifies the default font used to render glyphs.
    /// Referenced with `FontId(0)`, which is default.
    pub fn using_font(font_0_data: Font<'a>) -> Self {
        Self {
            font_data: vec![font_0_data],
            glyph_positioning_cache_size: 50,
        }
    }

    /// Adds additional fonts to the one added in [`using_font`](#method.using_font) /
    /// [`using_font_bytes`](#method.using_font_bytes).
    /// Returns a [`FontId`](struct.FontId.html) to reference this font.
    pub fn add_font_bytes<B: Into<SharedBytes<'a>>>(&mut self, font_data: B) -> FontId {
        self.font_data.push(font(font_data.into()).unwrap());
        FontId(self.font_data.len() - 1)
    }

    /// Adds additional fonts to the one added in [`using_font`](#method.using_font) /
    /// [`using_font_bytes`](#method.using_font_bytes).
    /// Returns a [`FontId`](struct.FontId.html) to reference this font.
    pub fn add_font(&mut self, font_data: Font<'a>) -> FontId {
        self.font_data.push(font_data);
        FontId(self.font_data.len() - 1)
    }

    /// Sets the max number of unique sections to cache positioning data. After processing
    /// over this amount of sections oldest section positioning data will be evicted from the cache.
    ///
    /// Defaults to 50
    pub fn glyph_positioning_cache_size(mut self, entries: usize) -> Self {
        self.glyph_positioning_cache_size = entries;
        self
    }

    /// Builds a `HeadlessGlyphBrush`
    pub fn build(self) -> HeadlessGlyphBrush<'a> {
        let fonts = self.font_data.into_iter().enumerate()
            .map(|(idx, data)| (FontId(idx), data))
            .collect();

        let cache = LinkedHashMap::with_capacity(self.glyph_positioning_cache_size + 1);

        HeadlessGlyphBrush {
            fonts,
            calculate_glyph_cache: cache,
            glyph_positioning_cache_size: self.glyph_positioning_cache_size,
        }
    }
}
