use std::time::Instant;

pub struct SimpleTimer {
    target: u128,
    start: Option<Instant>,
}

impl SimpleTimer {
    pub fn from_ms(target: u128) -> Self {
        Self {
            target,
            start: None,
        }
    }

    pub fn start(&mut self) {
        self.start = Some(Instant::now());
    }

    pub fn current(&self) -> Option<u128> {
        self.start.map(|start| start.elapsed().as_millis())
    }

    pub fn remaining(&self) -> Option<u128> {
        self.start
            .map(|start| self.target - start.elapsed().as_millis())
    }

    pub fn is_finished(&self) -> bool {
        match self.start {
            Some(instant) if instant.elapsed().as_millis() > self.target => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::*;

    #[test]
    fn timer_300ms() {
        run_test(Test::new(
            || {},
            || {
                let mut timer = SimpleTimer::from_ms(300);
                timer.start();
                std::thread::sleep(std::time::Duration::from_millis(200));
                assert!(!timer.is_finished());
                std::thread::sleep(std::time::Duration::from_millis(200));
                assert!(timer.is_finished());
            },
            || {},
        ));
    }
}
