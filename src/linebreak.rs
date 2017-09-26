use super::*;
use std::iter;
use std::iter::Skip;
use std::str::Chars;
use std::fmt;
use unicode_normalization::*;
use xi_unicode;

/// Indicator that a character is a line break, soft or hard. Includes the offset position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineBreak {
    /// Soft line break (offset).
    Soft(usize),
    /// Hard line break (offset).
    Hard(usize),
}

impl LineBreak {
    /// Returns the offset of the line break, the index after the breaking character.
    pub fn offset(&self) -> usize {
        match *self {
            LineBreak::Soft(offset) | LineBreak::Hard(offset) => offset,
        }
    }
}

/// Producer of a [`LineBreak`](enum.LineBreak.html) iterator. Used to allow to the
/// [`Layout`](enum.Layout.html) to be line break aware in a generic way.
pub trait LineBreaker: fmt::Debug + Copy + Hash {
    fn line_breaks<'a>(&self, glyph_info: &GlyphInfo<'a>) -> Box<Iterator<Item=LineBreak> + 'a>;
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum BuiltInLineBreaker {
    StandardLineBreaker,
    AnyCharLineBreaker,
}

impl Default for BuiltInLineBreaker {
    fn default() -> Self {
        BuiltInLineBreaker::StandardLineBreaker
    }
}

// Iterator that indicates all characters are soft line breaks, except hard ones which are hard.
struct AnyCharLineBreakerIter<'a> {
    chars: iter::Enumerate<Skip<Recompositions<Chars<'a>>>>,
    breaks: xi_unicode::LineBreakIterator<'a>,
    current_break: Option<(usize, bool)>,
}

impl<'a> Iterator for AnyCharLineBreakerIter<'a> {
    type Item = LineBreak;

    fn next(&mut self) -> Option<LineBreak> {
        if let Some((index, _)) = self.chars.next() {
            while self.current_break.is_some() {
                if self.current_break.as_ref().unwrap().0 < index + 1 {
                    self.current_break = self.breaks.next();
                }
                else { break; }
            }
            if let Some((break_index, true)) = self.current_break {
                if break_index == index + 1 {
                    return Some(LineBreak::Hard(break_index));
                }
            }
            Some(LineBreak::Soft(index + 1))
        }
        else { None }
    }
}

impl LineBreaker for BuiltInLineBreaker {
    fn line_breaks<'a>(&self, glyph_info: &GlyphInfo<'a>) -> Box<Iterator<Item=LineBreak> + 'a> {
        match *self {
            BuiltInLineBreaker::StandardLineBreaker => {
                Box::new(xi_unicode::LineBreakIterator::new(glyph_info.substring())
                    .map(|(offset, hard)| {
                        if hard { LineBreak::Hard(offset) } else { LineBreak::Soft(offset)}
                    }))
            }
            BuiltInLineBreaker::AnyCharLineBreaker => {
                let mut unicode_breaker = xi_unicode::LineBreakIterator::new(glyph_info.substring());
                let current_break = unicode_breaker.next();

                Box::new(AnyCharLineBreakerIter {
                    chars: glyph_info.remaining_chars().enumerate(),
                    breaks: unicode_breaker,
                    current_break
                })
            }
        }
    }
}
