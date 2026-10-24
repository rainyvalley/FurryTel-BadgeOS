//! Files: pictures generated from `badge.yaml`, plus a little note you can type on the badge.

use embedded_graphics::{
    Drawable,
    geometry::{Point, Size},
    pixelcolor::BinaryColor,
    primitives::{Primitive, PrimitiveStyle, Rectangle},
    text::Text,
};

use crate::generated::{PICTURES, Picture};
use crate::{
    hardware, store,
    ui::{self, Button, Canvas, Response, Screen, text_input::TextInput},
};

/// The extra entry after the pictures.
const NOTE: &str = "Note";

const fn check_pictures() {
    let mut picture = 0;
    while picture < PICTURES.len() {
        let rows = PICTURES[picture].rows;
        let scale = PICTURES[picture].scale;
        assert!(!rows.is_empty(), "a picture needs rows");
        let width = rows[0].len() as u32 * scale;
        assert!(
            rows.len() as u32 * scale <= crate::hardware::DISPLAY_HEIGHT,
            "picture is too tall"
        );
        assert!(
            width <= crate::hardware::DISPLAY_WIDTH,
            "picture is too wide"
        );
        let mut row = 0;
        while row < rows.len() {
            assert!(
                rows[row].len() as u32 * scale == width,
                "all rows must be the same width"
            );
            row += 1;
        }
        picture += 1;
    }
}

const _: () = check_pictures();

/// The file list: pictures to look at, and a note you can type.  Up/Down picks a file, Center
/// opens it, Left leaves.
pub struct Files {
    selected: usize,
    /// the picture being shown, if one is open
    viewing: Option<usize>,
    editing: bool,
    note: TextInput<128>,
}

impl Files {
    pub fn new() -> Self {
        Self {
            selected: 0,
            viewing: None,
            editing: false,
            note: TextInput::new(">", store::data().note()),
        }
    }

    fn count() -> usize {
        PICTURES.len() + 1
    }

    fn label(index: usize) -> &'static str {
        PICTURES.get(index).map_or(NOTE, |picture| picture.name)
    }

    fn draw_list(&self, canvas: &mut Canvas) {
        let row_height = ui::FONT.character_size.height;
        let visible = (hardware::DISPLAY_HEIGHT / row_height) as usize;
        let count = Self::count();
        let top = self
            .selected
            .saturating_sub(visible / 2)
            .min(count.saturating_sub(visible));

        for (row, index) in (top..count).take(visible).enumerate() {
            let y = (row as u32 * row_height) as i32;
            let style = if index == self.selected {
                Rectangle::new(
                    Point::new(0, y),
                    Size::new(hardware::DISPLAY_WIDTH, row_height),
                )
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(canvas)
                .unwrap();
                ui::TEXT_INVERTED
            } else {
                ui::TEXT
            };
            Text::with_text_style(Self::label(index), Point::new(2, y), style, ui::TOP_LEFT)
                .draw(canvas)
                .unwrap();
        }
    }

    fn draw_picture(&self, canvas: &mut Canvas, picture: &Picture) {
        let ink = PrimitiveStyle::with_fill(BinaryColor::On);
        let scale = picture.scale as i32;
        let width = picture.rows[0].len() as u32 * picture.scale;
        let height = picture.rows.len() as u32 * picture.scale;
        let left = (hardware::DISPLAY_WIDTH - width) / 2;
        let top = (hardware::DISPLAY_HEIGHT - height) / 2;

        for (row, line) in picture.rows.iter().enumerate() {
            let bytes = line.as_bytes();
            let mut x = 0;
            while x < bytes.len() {
                if bytes[x] != b'#' {
                    x += 1;
                    continue;
                }
                let start = x;
                while x < bytes.len() && bytes[x] == b'#' {
                    x += 1;
                }
                Rectangle::new(
                    Point::new(
                        left as i32 + start as i32 * scale,
                        top as i32 + row as i32 * scale,
                    ),
                    Size::new((x - start) as u32 * picture.scale, picture.scale),
                )
                .into_styled(ink)
                .draw(canvas)
                .unwrap();
            }
        }
    }
}

impl Screen for Files {
    fn enter(&mut self) {
        self.viewing = None;
        self.editing = false;
    }

    fn draw(&self, canvas: &mut Canvas) {
        if self.editing {
            self.note.draw(canvas);
        } else if let Some(index) = self.viewing {
            if let Some(picture) = PICTURES.get(index) {
                self.draw_picture(canvas, picture);
            }
        } else {
            self.draw_list(canvas);
        }
    }

    fn handle(&mut self, button: Button) -> Response {
        if self.editing {
            if self.note.handle(button) == Response::Exit {
                self.editing = false;
                if self.note.accepted() {
                    store::set_note(self.note.value());
                    store::save();
                }
            }
            return Response::Redraw;
        }

        if self.viewing.is_some() {
            self.viewing = None;
            return Response::Redraw;
        }

        let count = Self::count().max(1);
        match button {
            Button::Up => self.selected = self.selected.saturating_sub(1),
            Button::Down => self.selected = (self.selected + 1).min(count - 1),
            Button::Center | Button::Right => {
                if self.selected < PICTURES.len() {
                    self.viewing = Some(self.selected);
                } else {
                    // open the saved note so it can be read and amended
                    self.note.enter();
                    self.editing = true;
                }
            }
            Button::Left => return Response::Exit,
        }
        Response::Redraw
    }
}
