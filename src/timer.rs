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

    pub fn is_finished(&mut self) -> bool {
        match self.start {
            Some(instant) if instant.elapsed().as_millis() > self.target => {
                self.start = None;
                true
            }
            _ => false,
        }
    }
}
