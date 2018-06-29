use super::*;
use layout::words::RelativePositionedGlyph;
use layout::words::Words;
use layout::words::ZERO_V_METRICS;
use rusttype::vector;
use std::iter::Peekable;
use std::iter::{FusedIterator, Iterator};

/// A line of `Word`s limited to a max width bound.
pub(crate) struct Line<'font> {
    pub glyphs: Vec<(RelativePositionedGlyph<'font>, Color, FontId)>,
    pub max_v_metrics: VMetrics,
    /// Width of line includes non-glyph spacing at eol.
    pub width: f32,
}

impl<'font> Line<'font> {
    #[inline]
    pub(crate) fn line_height(&self) -> f32 {
        self.max_v_metrics.ascent - self.max_v_metrics.descent + self.max_v_metrics.line_gap
    }

    /// Returns line glyphs positioned on the screen and aligned.
    pub fn aligned_on_screen(
        self,
        screen_position: (f32, f32),
        h_align: HorizontalAlign,
        v_align: VerticalAlign,
    ) -> Vec<(PositionedGlyph<'font>, Color, FontId)> {
        if self.glyphs.is_empty() {
            return Vec::new();
        }

        // implement v-aligns when they're are supported
        let screen_left = match h_align {
            HorizontalAlign::Left => point(screen_position.0, screen_position.1),
            // - Right alignment attained from left by shifting the line
            //   leftwards by the rightmost x distance from render position
            // - Central alignment is attained from left by shifting the line
            //   leftwards by half the rightmost x distance from render position
            HorizontalAlign::Center | HorizontalAlign::Right => {
                let mut shift_left = self.width;
                if h_align == HorizontalAlign::Center {
                    shift_left /= 2.0;
                }
                point(screen_position.0 - shift_left, screen_position.1)
            }
        };

        let screen_pos = match v_align {
            VerticalAlign::Top => screen_left,
            VerticalAlign::Center => {
                let mut screen_pos = screen_left;
                screen_pos.x -= self.line_height() / 2.0;
                screen_pos
            }
            VerticalAlign::Bottom => {
                let mut screen_pos = screen_left;
                screen_pos.x -= self.line_height();
                screen_pos
            }
        };

        self.glyphs
            .into_iter()
            .map(|(glyph, color, font_id)| (glyph.screen_positioned(screen_pos), color, font_id))
            .collect()
    }
}

/// `Line` iterator.
pub(crate) struct Lines<'a, 'b, 'font: 'a + 'b, L: LineBreaker> {
    pub(crate) words: Peekable<Words<'a, 'b, 'font, L>>,
    pub(crate) width_bound: f32,
}

impl<'a, 'b, 'font, L: LineBreaker> Iterator for Lines<'a, 'b, 'font, L> {
    type Item = Line<'font>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut caret = vector(0.0, 0.0);
        let mut line = Line {
            glyphs: Vec::new(),
            max_v_metrics: ZERO_V_METRICS,
            width: 0.0,
        };

        let mut progressed = false;

        #[allow(while_let_loop)] // TODO use while-peek-next when nll lands
        loop {
            if let Some(word) = self.words.peek() {
                let word_max_x = word.bounds.map(|b| b.max.x).unwrap_or(word.layout_width);
                if (caret.x + word_max_x).ceil() > self.width_bound {
                    break;
                }
            }
            else {
                break;
            }

            let word = self.words.next().unwrap();
            progressed = true;

            if word.max_v_metrics.ascent > line.max_v_metrics.ascent {
                let diff_y = word.max_v_metrics.ascent - caret.y;
                caret.y += diff_y;

                // modify all smaller lined glyphs to occupy the new larger line
                for (glyph, ..) in &mut line.glyphs {
                    glyph.relative.y += diff_y;
                }

                line.max_v_metrics = word.max_v_metrics;
            }

            if word.bounds.is_some() {
                line.glyphs
                    .extend(word.glyphs.into_iter().map(|(mut g, color, font_id)| {
                        g.relative = g.relative + caret;
                        (g, color, font_id)
                    }));
            }

            caret.x += word.layout_width;

            if word.hard_break {
                break;
            }
        }

        line.width = caret.x;

        Some(line).filter(|_| progressed)
    }
}

impl<'a, 'b, 'font, L: LineBreaker> FusedIterator for Lines<'a, 'b, 'font, L> {}
