use time;

use std::vec::*;
use std::collections::BTreeMap;

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
pub struct PerformaceCounters {
    samples: usize,
    acum_time: f64,
}

/// some structure to count events
impl PerformaceCounters {
    pub fn new() -> PerformaceCounters {
        PerformaceCounters {
            samples: 0,
            acum_time: 0.0,
        }
    }

    fn append(&mut self, delta: f64) {
        self.samples += 1;
        self.acum_time += delta;
    }
    fn get_fps(&self) -> f64 {
        self.samples as f64 / self.acum_time
    }
    fn reset(&mut self) {
        self.samples = 0;
        self.acum_time = 0 as f64;
    }
}

/// infinite loop with iterations/second reporting every x seconds
/// it will pass delta time to function body
pub fn loop_with_report<'a, F: FnMut(f64)>(mut body: F, x: u32) {
    let mut pc = PerformaceCounters::new();
    if x == 0 {
        loop {
            body(0.0);
        }
    } else {
        loop {
            let mut delta: f64 = 0.0;

            let start = time::PreciseTime::now();
            while start.to(time::PreciseTime::now()) < time::Duration::seconds(x as i64) {
                let start_t = time::precise_time_s();

                body(delta);

                let end_t = time::precise_time_s();
                delta = end_t - start_t;
                pc.append(delta);
            }

            println!("fps: {} ", pc.get_fps());
            pc.reset();
        }
    }
}
