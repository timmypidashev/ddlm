use crate::buffer::{Buffer, BufferError};
use crate::color::Color;

use std::collections::HashMap;

use lazy_static::lazy_static;
use rusttype::{point, Font as RustFont, Scale};
use thiserror::Error;

pub static COMIC_CODE_FONT_DATA: &[u8] = include_bytes!("../fonts/ComicCode-Regular.otf");

lazy_static! {
    pub static ref COMICCODE_REGULAR: RustFont<'static> =
        RustFont::try_from_bytes(COMIC_CODE_FONT_DATA as &[u8])
            .expect("error constructing Comic Code");
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum DrawError {
    #[error("glyph for {0} not in cache")]
    GlyphNotInCache(char),
}

struct CachedGlyph {
    dimensions: (u32, u32),
    origin: (i32, i32),
    render: Vec<f32>,
}

impl CachedGlyph {
    fn new(font: &RustFont<'_>, size: f32, ch: char) -> CachedGlyph {
        let scale = Scale::uniform(size);
        let v_metrics = font.v_metrics(scale);
        let glyph = font
            .glyph(ch)
            .scaled(scale)
            .positioned(point(0.0, v_metrics.ascent));

        if let Some(bounding_box) = glyph.pixel_bounding_box() {
            let origin = (bounding_box.min.x, bounding_box.min.y);

            let dimensions = (
                (bounding_box.max.x - bounding_box.min.x) as u32,
                (bounding_box.max.y - bounding_box.min.y) as u32,
            );
            let mut render = vec![0.0; (dimensions.0 * dimensions.1) as usize];
            glyph.draw(|x, y, o| {
                let pos = x + (y * dimensions.0);
                render[pos as usize] = o;
            });
            CachedGlyph {
                origin,
                dimensions,
                render,
            }
        } else {
            CachedGlyph {
                origin: (0, 0),
                dimensions: ((size / 4.0) as u32, 0),
                render: Vec::new(),
            }
        }
    }

    fn draw(&self, buf: &mut Buffer<'_>, pos: (i32, i32), bg: &Color, c: &Color) {
        let mut x = 0;
        let mut y = 0;
        for v in &self.render {
            let _ = buf.put(
                (
                    (x + pos.0 + self.origin.0) as u32,
                    (y + pos.1 + self.origin.1) as u32,
                ),
                &bg.blend(c, *v),
            );

            if x == self.dimensions.0 as i32 - 1 {
                y += 1;
                x = 0;
            } else {
                x += 1;
            }
        }
    }
}

pub struct Font {
    glyphs: HashMap<char, CachedGlyph>,
    font: &'static RustFont<'static>,
    size: f32,
}

impl Font {
    pub fn new(font: &'static RustFont<'_>, size: f32) -> Font {
        Font {
            glyphs: HashMap::new(),
            font,
            size,
        }
    }

    pub fn add_str_to_cache(&mut self, s: &str) {
        for ch in s.chars() {
            if self.glyphs.get(&ch).is_none() {
                let glyph = CachedGlyph::new(self.font, self.size, ch);
                self.glyphs.insert(ch, glyph);
            }
        }
    }

    pub fn draw_text(
        &self,
        buf: &mut Buffer<'_>,
        bg: &Color,
        c: &Color,
        s: &str,
    ) -> Result<(u32, u32), DrawError> {
        let mut x_off = 0;
        let mut off = 0;
        let mut glyphs = Vec::with_capacity(s.len());
        for ch in s.chars() {
            let glyph = match self.glyphs.get(&ch) {
                Some(glyph) => glyph,
                None => return Err(DrawError::GlyphNotInCache(ch)),
            };
            glyphs.push(glyph);
            if glyph.origin.1 < off {
                off = glyph.origin.1
            }
        }
        for glyph in glyphs {
            glyph.draw(buf, (x_off, -off), bg, c);
            x_off += glyph.dimensions.0 as i32 + glyph.origin.0;
        }

        Ok((x_off as u32, self.size as u32))
    }

    pub fn auto_draw_text(
        &mut self,
        buf: &mut Buffer<'_>,
        bg: &Color,
        c: &Color,
        s: &str,
    ) -> Result<(u32, u32), DrawError> {
        self.add_str_to_cache(s);
        self.draw_text(buf, bg, c, s)
    }
}

pub fn draw_box(buf: &mut Buffer<'_>, c: &Color, dim: (u32, u32)) -> Result<(), BufferError> {
    for x in 0..dim.0 {
        let _ = buf.put((x, 0), c);
        let _ = buf.put((x, dim.1 - 1), c);
    }
    for y in 0..dim.1 {
        buf.put((0, y), c)?;
        buf.put((dim.0 - 1, y), c)?;
    }

    Ok(())
}
