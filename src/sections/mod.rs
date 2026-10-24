//! Independent regions of the badge screen.
//!
//! Each [`Section`] owns a rectangle of the display and draws into a cropped target, so it cannot
//! draw outside its own region.  The [`Dashboard`] asks every section on each frame whether it
//! changed, and each section decides for itself how often that happens.  Adding a section is just
//! implementing the trait and adding it to the dashboard's list.

use embassy_time::Duration;
use embedded_graphics::{
    draw_target::{Cropped, DrawTargetExt},
    primitives::Rectangle,
};

use crate::ui::{Button, Canvas, Response, Screen};

mod identity;
mod status_line;
mod wordmark;

pub use identity::Identity;
pub use status_line::StatusLine;
pub use wordmark::Wordmark;

/// What a section draws on: the display, cropped to the section's area.  Coordinates are relative
/// to the section's top-left corner, not the screen's.
pub type SectionTarget<'a> = Cropped<'a, Canvas>;

/// One region of the badge screen with its own drawing and update schedule.
pub trait Section {
    /// The part of the screen this section owns.
    fn area(&self) -> Rectangle;

    /// Draw the section's contents, in coordinates relative to [`Section::area`].  Anything
    /// outside the area is clipped away.
    fn draw(&self, target: &mut SectionTarget<'_>);

    /// Called once per frame.  Return `true` when something changed and the screen needs redrawing.
    fn tick(&mut self) -> bool {
        false
    }
}

/// A stack of sections, each in its own part of the screen, redrawn together whenever any of them
/// changes.
pub struct Dashboard<'a, const N: usize> {
    sections: [&'a mut dyn Section; N],
}

impl<'a, const N: usize> Dashboard<'a, N> {
    /// How often sections are asked whether they changed.  Pushing the whole framebuffer over SPI
    /// takes several milliseconds, so this is about as fast as the screen can usefully go.
    pub const FRAME: Duration = Duration::from_millis(70);

    pub fn new(sections: [&'a mut dyn Section; N]) -> Self {
        const { assert!(N > 0, "a dashboard needs at least one section") };
        Self { sections }
    }
}

impl<const N: usize> Screen for Dashboard<'_, N> {
    fn draw(&self, canvas: &mut Canvas) {
        for section in &self.sections {
            let area = section.area();
            section.draw(&mut canvas.cropped(&area));
        }
    }

    fn handle(&mut self, _: Button) -> Response {
        Response::Ignored
    }

    fn tick_interval(&self) -> Option<Duration> {
        Some(Self::FRAME)
    }

    fn tick(&mut self) -> Response {
        let changed = self
            .sections
            .iter_mut()
            .fold(false, |changed, section| section.tick() | changed);
        if changed {
            Response::Redraw
        } else {
            Response::Ignored
        }
    }
}
