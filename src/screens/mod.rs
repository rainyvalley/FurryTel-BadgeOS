//! The badge's actual screens, built on [`crate::ui`].

mod about;
mod bootloader;
mod crossy;
mod files;
mod flappy;
mod home;
mod led_colors;
mod leds;
mod links;
mod snake;
mod status;
mod tetris;

pub use about::About;
pub use bootloader::Bootloader;
pub use crossy::Crossy;
pub use files::Files;
pub use flappy::Flappy;
pub use home::Home;
pub use led_colors::LedColors;
pub use leds::Leds;
pub use links::Links;
pub use snake::Snake;
pub use status::Status;
pub use tetris::Tetris;
