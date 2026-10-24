use defmt::debug;
use embassy_rp::gpio::Input;
use embassy_time::{Duration, Instant, Timer};

#[derive(Clone, Copy)]
enum State {
    Pressed,
    NotPressed,
}

pub struct Button {
    gpio: Input<'static>,
    deadline: Option<Instant>,
    last_state: State,
}

impl Button {
    pub fn new(gpio: Input<'static>) -> Self {
        Self {
            gpio,
            deadline: None,
            last_state: State::NotPressed,
        }
    }
}

impl Button {
    async fn wait_for_change(&mut self) -> State {
        if let Some(d) = self.deadline {
            debug!("waiting until {:?}", d);
            Timer::at(d).await;
            debug!("debounce timer expired");
            // don't bother awaiting a timer next time if the future gets cancelled after this
            self.deadline = None;
        }

        match self.last_state {
            State::Pressed => {
                self.gpio.wait_for_high().await;
                self.last_state = State::NotPressed
            }
            State::NotPressed => {
                self.gpio.wait_for_low().await;
                self.last_state = State::Pressed
            }
        }

        self.deadline = Some(Instant::now() + Duration::from_millis(20));
        self.last_state
    }

    pub async fn wait_for_pressed(&mut self) {
        while let State::NotPressed = self.wait_for_change().await {}
    }
}
