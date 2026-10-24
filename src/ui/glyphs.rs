//! Custom glyphs for the badge's buttons, so on-screen help can say "▶ next" instead of "> next".
//!
//! The glyphs live in their own little fonts, one per size of [`super::FONT`] and
//! [`super::SMALL_FONT`], and [`GlyphTextStyle`] switches to them for just the glyph characters.
//! Any of [`GLYPHS`] can be used in text drawn with the `ui` styles:
//!
//! | char | button |
//! |------|--------|
//! | ▲    | Up     |
//! | ▼    | Down   |
//! | ◀    | Left   |
//! | ▶    | Right  |
//! | ●    | Center |

use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{Point, Size},
    image::ImageRaw,
    mono_font::{DecorationDimensions, MonoFont, MonoTextStyle, mapping::StrGlyphMapping},
    pixelcolor::BinaryColor,
    primitives::Rectangle,
    text::{
        Baseline,
        renderer::{TextMetrics, TextRenderer},
    },
};

/// The characters with custom glyphs, in the order they're drawn in each font's image.
pub const GLYPHS: &str = "▲▼◀▶●";

/// Glyphs to go with [`super::FONT`] (6x10), whose capitals fill the 5x7 box at (0, 1).
pub const FONT: MonoFont<'static> = glyph_font(&FONT_IMAGE, Size::new(6, 10), 7);
const FONT_IMAGE: [u8; 4 * 10] = pack(&[
    // ▲      ▼      ◀      ▶      ●
    "...... ...... ...... ...... ......",
    "...... ...... ...... ...... ......",
    "...... ...... ...#.. .#.... .###..",
    "..#... #####. ..##.. .##... #####.",
    ".###.. .###.. .###.. .###.. #####.",
    "#####. ..#... ..##.. .##... #####.",
    "...... ...... ...#.. .#.... .###..",
    "...... ...... ...... ...... ......",
    "...... ...... ...... ...... ......",
    "...... ...... ...... ...... ......",
]);

/// Glyphs to go with [`super::SMALL_FONT`] (4x6), whose capitals fill the 3x5 box at (0, 0).
pub const SMALL_FONT: MonoFont<'static> = glyph_font(&SMALL_FONT_IMAGE, Size::new(4, 6), 4);
const SMALL_FONT_IMAGE: [u8; 3 * 6] = pack(&[
    // ▲    ▼    ◀    ▶    ●
    ".... .... .... .... ....",
    ".... .... ..#. #... .#..",
    ".#.. ###. .##. ##.. ###.",
    "###. .#.. ..#. #... .#..",
    ".... .... .... .... ....",
    ".... .... .... .... ....",
]);

const fn glyph_font(
    image: &'static [u8],
    character_size: Size,
    baseline: u32,
) -> MonoFont<'static> {
    MonoFont {
        image: ImageRaw::new(image, GLYPH_COUNT as u32 * character_size.width),
        character_size,
        character_spacing: 0,
        baseline,
        strikethrough: DecorationDimensions::new(character_size.height / 2, 1),
        underline: DecorationDimensions::new(baseline + 2, 1),
        glyph_mapping: &MAPPING,
    }
}

const GLYPH_COUNT: usize = {
    // `chars().count()` isn't const, so count the bytes that start a UTF-8 character instead
    let bytes = GLYPHS.as_bytes();
    let mut count = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] & 0xc0 != 0x80 {
            count += 1;
        }
        i += 1;
    }
    count
};
const MAPPING: StrGlyphMapping<'static> = StrGlyphMapping::new(GLYPHS, 0);

/// Packs rows of `#` (on) and `.` (off) into a 1 bit per pixel image, each row padded out to a
/// whole byte.  Spaces are ignored, so they can separate the glyphs.
const fn pack<const BYTES: usize>(rows: &[&str]) -> [u8; BYTES] {
    let mut image = [0; BYTES];
    let stride = BYTES / rows.len();
    assert!(
        stride * rows.len() == BYTES,
        "image size isn't a whole number of rows"
    );
    let mut y = 0;
    while y < rows.len() {
        let row = rows[y].as_bytes();
        let mut x = 0;
        let mut i = 0;
        while i < row.len() {
            match row[i] {
                b' ' => {
                    i += 1;
                    continue;
                }
                b'#' => image[y * stride + x / 8] |= 0x80 >> (x % 8),
                b'.' => {}
                _ => panic!("glyph pixels must be '#' or '.'"),
            }
            x += 1;
            i += 1;
        }
        assert!(
            x.div_ceil(8) == stride,
            "row width doesn't match the image size"
        );
        y += 1;
    }
    image
}

/// A [`MonoTextStyle`] that draws [`GLYPHS`] from a matching glyph font.  Both fonts must be the
/// same size, so text with glyphs lines up the same as text without.
#[derive(Clone, Copy)]
pub struct GlyphTextStyle {
    text: MonoTextStyle<'static, BinaryColor>,
    glyphs: MonoTextStyle<'static, BinaryColor>,
}

impl GlyphTextStyle {
    pub const fn new(
        text: MonoTextStyle<'static, BinaryColor>,
        glyphs: &'static MonoFont<'static>,
    ) -> Self {
        assert!(
            text.font.character_size.width == glyphs.character_size.width
                && text.font.character_size.height == glyphs.character_size.height
                && text.font.baseline == glyphs.baseline,
            "glyph font doesn't match the text font"
        );
        // same colors, different font
        let mut glyph_style = text;
        glyph_style.font = glyphs;
        Self {
            text,
            glyphs: glyph_style,
        }
    }

    /// Split `text` into runs of either all glyphs or no glyphs, and say which style draws each.
    fn runs<'t>(
        &self,
        text: &'t str,
    ) -> impl Iterator<Item = (&'t str, &MonoTextStyle<'static, BinaryColor>)> {
        let mut rest = text;
        core::iter::from_fn(move || {
            let glyph = GLYPHS.contains(rest.chars().next()?);
            let end = rest
                .find(|c| GLYPHS.contains(c) != glyph)
                .unwrap_or(rest.len());
            let (run, tail) = rest.split_at(end);
            rest = tail;
            Some((run, if glyph { &self.glyphs } else { &self.text }))
        })
    }
}

impl TextRenderer for GlyphTextStyle {
    type Color = BinaryColor;

    fn draw_string<D>(
        &self,
        text: &str,
        mut position: Point,
        baseline: Baseline,
        target: &mut D,
    ) -> Result<Point, D::Error>
    where
        D: DrawTarget<Color = BinaryColor>,
    {
        for (run, style) in self.runs(text) {
            position = style.draw_string(run, position, baseline, target)?;
        }
        Ok(position)
    }

    fn draw_whitespace<D>(
        &self,
        width: u32,
        position: Point,
        baseline: Baseline,
        target: &mut D,
    ) -> Result<Point, D::Error>
    where
        D: DrawTarget<Color = BinaryColor>,
    {
        self.text.draw_whitespace(width, position, baseline, target)
    }

    fn measure_string(&self, text: &str, position: Point, baseline: Baseline) -> TextMetrics {
        // the fonts are the same size, so each run's box continues on from the last
        let mut metrics = self.text.measure_string("", position, baseline);
        for (run, style) in self.runs(text) {
            let run_metrics = style.measure_string(run, metrics.next_position, baseline);
            let width = metrics.bounding_box.size.width + run_metrics.bounding_box.size.width;
            metrics = TextMetrics {
                bounding_box: Rectangle::new(
                    if metrics.bounding_box.size.width == 0 {
                        run_metrics.bounding_box.top_left
                    } else {
                        metrics.bounding_box.top_left
                    },
                    Size::new(width, run_metrics.bounding_box.size.height),
                ),
                next_position: run_metrics.next_position,
            };
        }
        metrics
    }

    fn line_height(&self) -> u32 {
        self.text.line_height()
    }
}
