//! Scan-me pages, generated from `badge.yaml`: a QR code as large as the screen allows, with the
//! title and a short label beside it.

use embedded_graphics::{
    Drawable,
    geometry::{Point, Size},
    mono_font::MonoTextStyle,
    pixelcolor::BinaryColor,
    primitives::{Primitive, PrimitiveStyle, Rectangle},
    text::Text,
};

use crate::generated::{LINKS, Link};
use crate::ui::{self, Button, Canvas, Response, Screen};

/// Every generated QR code is version 3, so they all have the same size.
const QR_SIZE: usize = 29;
/// As close to the corner as the code can go, leaving a quiet zone.
const QR_X: i32 = 1;
const QR_Y: i32 = 1;
/// The text column, right of the QR code.
const TEXT_X: i32 = 34;

const fn check_links() {
    let mut link = 0;
    while link < LINKS.len() {
        let qr = LINKS[link].qr;
        assert!(
            qr.len() == QR_SIZE,
            "QR codes must be 29 rows; rerun tools/generate.py"
        );
        let mut row = 0;
        while row < qr.len() {
            assert!(qr[row].len() == QR_SIZE, "QR codes must be 29 columns");
            row += 1;
        }
        link += 1;
    }
}

const _: () = check_links();

/// Scan-me links: Up/Down or Left/Right flip pages, Center leaves.
pub struct Links {
    page: usize,
}

impl Links {
    pub const fn new() -> Self {
        Self { page: 0 }
    }

    fn draw_qr(&self, canvas: &mut Canvas, link: &Link) {
        let ink = PrimitiveStyle::with_fill(BinaryColor::On);

        for (row, line) in link.qr.iter().enumerate() {
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
                    Point::new(QR_X + start as i32, QR_Y + row as i32),
                    Size::new((x - start) as u32, 1),
                )
                .into_styled(ink)
                .draw(canvas)
                .unwrap();
            }
        }
    }
}

impl Screen for Links {
    fn enter(&mut self) {
        self.page = 0;
    }

    fn draw(&self, canvas: &mut Canvas) {
        if LINKS.is_empty() {
            Text::with_text_style("No links", Point::new(2, 4), ui::TEXT, ui::TOP_LEFT)
                .draw(canvas)
                .unwrap();
            Text::with_text_style("● back", Point::new(2, 18), ui::SMALL_TEXT, ui::TOP_LEFT)
                .draw(canvas)
                .unwrap();
            return;
        }

        let Some(link) = LINKS.get(self.page) else {
            return;
        };
        self.draw_qr(canvas, link);

        Text::with_text_style(
            link.title,
            Point::new(TEXT_X, 0),
            MonoTextStyle::new(ui::FONT, BinaryColor::On),
            ui::TOP_LEFT,
        )
        .draw(canvas)
        .unwrap();

        Text::with_text_style(
            link.label,
            Point::new(TEXT_X, 11),
            ui::SMALL_TEXT,
            ui::TOP_LEFT,
        )
        .draw(canvas)
        .unwrap();

        Text::with_text_style(
            "▲▼ pages ● back",
            Point::new(TEXT_X, 23),
            ui::SMALL_TEXT,
            ui::TOP_LEFT,
        )
        .draw(canvas)
        .unwrap();
    }

    fn handle(&mut self, button: Button) -> Response {
        if LINKS.is_empty() {
            return if button == Button::Center {
                Response::Exit
            } else {
                Response::Ignored
            };
        }
        match button {
            Button::Up | Button::Left => {
                self.page = (self.page + LINKS.len() - 1) % LINKS.len();
                Response::Redraw
            }
            Button::Down | Button::Right => {
                self.page = (self.page + 1) % LINKS.len();
                Response::Redraw
            }
            Button::Center => Response::Exit,
        }
    }
}
