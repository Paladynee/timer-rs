use std::time::{Duration, Instant};

pub struct Timer<A, T: FnMut() -> A> {
    f: T,
    elapsed: Option<Duration>,
}

impl<A, T> Timer<A, T>
where
    T: FnMut() -> A,
{
    pub const fn new(f: T) -> Timer<A, T> {
        Timer { f, elapsed: None }
    }

    pub fn exec(&mut self) -> A {
        let start = Instant::now();
        let res = (self.f)();
        self.elapsed = Some(start.elapsed());
        res
    }

    pub const fn get_elapsed(&self) -> Option<Duration> {
        self.elapsed
    }
}

pub trait IntoTimer<A, T: FnMut() -> A> {
    fn into_timer(self) -> Timer<A, T>;
}

impl<A, T: FnMut() -> A> IntoTimer<A, T> for T {
    fn into_timer(self) -> Timer<A, T> {
        Timer::new(self)
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use super::*;

    #[test]
    fn it_works() {
        let my_func = || 5 + 3;
        let mut timer = Timer::new(my_func);
        let eight = timer.exec();

        assert_eq!(eight, 8);
    }

    #[test]
    fn it_waits() {
        let my_func = || thread::sleep(Duration::from_secs(1));
        let mut timer = Timer::new(my_func);
        let none = timer.get_elapsed();

        assert!(none.is_none());

        timer.exec();
        let at_least_1_sec = timer.get_elapsed().unwrap();
        assert!(at_least_1_sec >= Duration::from_secs(1));
    }

    #[test]
    fn it_traits() {
        let my_func = || 5 + 3;
        let mut timer = my_func.into_timer();
        let eight = timer.exec();

        assert_eq!(eight, 8);
    }
}
