use embassy_time::Duration;

use crate::{
    sections::Dashboard,
    ui::{Button, Canvas, Response, Screen, menu::Menu},
};

/// The badge's home: the dashboard of sections, with the menu a button press away.
///
/// Center or Left opens the menu; Left inside the menu comes back here.
pub struct Home<'a, const N: usize, const M: usize> {
    dashboard: Dashboard<'a, N>,
    menu: Menu<'a, M>,
    menu_open: bool,
}

impl<'a, const N: usize, const M: usize> Home<'a, N, M> {
    pub fn new(dashboard: Dashboard<'a, N>, menu: Menu<'a, M>) -> Self {
        Self {
            dashboard,
            menu,
            menu_open: false,
        }
    }
}

impl<const N: usize, const M: usize> Screen for Home<'_, N, M> {
    fn enter(&mut self) {
        self.menu_open = false;
    }

    fn draw(&self, canvas: &mut Canvas) {
        if self.menu_open {
            self.menu.draw(canvas);
        } else {
            self.dashboard.draw(canvas);
        }
    }

    fn handle(&mut self, button: Button) -> Response {
        if self.menu_open {
            return match self.menu.handle(button) {
                Response::Exit => {
                    self.menu_open = false;
                    Response::Redraw
                }
                response => response,
            };
        }

        match button {
            Button::Center | Button::Left => {
                self.menu_open = true;
                Response::Redraw
            }
            _ => Response::Ignored,
        }
    }

    fn tick_interval(&self) -> Option<Duration> {
        if self.menu_open {
            self.menu.tick_interval()
        } else {
            self.dashboard.tick_interval()
        }
    }

    fn tick(&mut self) -> Response {
        if self.menu_open {
            self.menu.tick()
        } else {
            self.dashboard.tick()
        }
    }

    fn secret_ok(&self) -> bool {
        !self.menu_open
    }
}
