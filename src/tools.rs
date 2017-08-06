use time;

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
#[derive(Default)]
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
pub fn loop_with_report<F: FnMut(f64)>(mut body: F, x: u32) {
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
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
use std::rc::Rc;
use std::cell::RefCell;
use std::cell::{Ref, RefMut};
use std::clone::Clone;

pub struct RcRef<T>(Rc<RefCell<T>>);

impl<T> RcRef<T> {
    pub fn new(value: T) -> RcRef<T> {
        RcRef(Rc::new(RefCell::new(value)))
    }
    pub fn get(&self) -> Ref<T> {
        let &RcRef(ref rc) = self;
        rc.borrow()
    }

    pub fn get_mut(&mut self) -> RefMut<T> {
        let &mut RcRef(ref rc) = self;
        rc.borrow_mut()
    }
}

impl<T> Clone for RcRef<T> {
    fn clone(&self) -> Self {
        let &RcRef(ref rc) = self;
        RcRef(rc.clone())
    }
}
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

pub trait LayoutHash {
    fn hash(&self) -> u64;
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

#[cfg(test)]
mod tests {

    use super::*;

    #[derive(Debug)]
    struct NonCopiable {
        v: u32,
    }

    #[test]
    fn rcref() {

        let v = NonCopiable { v: 101 };

        let a = RcRef::new(v);
        assert!(a.get().v == 101);
        {
            let mut b = a.clone();
            assert!(b.get().v == 101);
            b.get_mut().v = 202;
            assert!(b.get().v == 202);

        }
        assert!(a.get().v == 202);


    }
}
