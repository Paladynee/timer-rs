//! # Timing utilities
//!
//! This library provides a simple way to time code execution.
//!
//! ## Example 1
//! ```rust
//! # use voxell_timer::*;
//! let (result, time) = time! {
//!     for _ in 0..1_000_000 {
//!         let _ = std::hint::black_box(3);
//!     }
//!     10
//! };
//! eprintln!("Took {}ms", time.as_millis());
//! assert_eq!(result, 10);
//! ```
//!
//! ## Example 2
//! ```rust
//! # use voxell_timer::*;
//! let haystack = vec![1, 2, 3, 4, 5, 6];
//! let result = time_println!  {
//!     "Finding needle in haystack",
//!     let needle = 4;
//!     haystack.iter().find(|a| **a == needle)
//! };
//!
//! assert_eq!(result, Some(&4));
//! ```
//!
//! ## Example 3
//! ```rust
//! # use voxell_timer::*;
//! let haystack = vec![1, 2, 3, 4, 5, 6];
//! let result = time_fn_println("Finding needle in haystack", || {
//!    let needle = 4;
//!    haystack.iter().find(|a| **a == needle)
//! });
//! assert_eq!(result, Some(&4));
//! ```
#![warn(missing_docs)]
use core::time::Duration;
use std::time::Instant;

/// wrapper around a closure
pub struct LazyTimer<T, F>
where
    F: FnOnce() -> T,
{
    f: F,
}

impl<T, F> LazyTimer<T, F>
where
    F: FnOnce() -> T,
{
    /// create a new `LazyTimer` from a closure
    #[inline]
    pub const fn new(f: F) -> Self {
        Self { f }
    }

    /// execute the closure and return the result and
    /// the time it took to execute
    #[inline]
    pub fn into_exec(self) -> (T, Duration) {
        let f = self.f;

        let start = Instant::now();
        let res = f();

        (res, start.elapsed())
    }

    /// you wont be able to use the resulting closure
    /// if the closure captures non-copy types
    ///
    /// if it does not capture any non-copy types
    /// you can call the resulting closure and the timer
    /// however you want.
    #[inline]
    pub const fn as_inner(&self) -> &F {
        &self.f
    }

    /// get a mutable reference to the closure
    #[inline]
    pub const fn as_inner_mut(&mut self) -> &mut F {
        &mut self.f
    }

    /// consume the `LazyTimer` and return the closure
    #[inline]
    pub fn into_inner(self) -> F {
        self.f
    }
}

/// trait to convert a closure into a `LazyTimer`
pub trait IntoLazyTimer<T, F>
where
    F: FnOnce() -> T,
{
    /// converts a closure into a `LazyTimer`
    fn into_lazy_timer(self) -> LazyTimer<T, F>;
}

impl<T, F> IntoLazyTimer<T, F> for F
where
    F: FnOnce() -> T,
{
    #[inline]
    fn into_lazy_timer(self) -> LazyTimer<T, F> {
        LazyTimer::new(self)
    }
}

/// use when you need both the result of the closure and the time
/// it took to execute as a tuple.
#[inline]
pub fn time_fn<T, F>(f: F) -> (T, Duration)
where
    F: FnOnce() -> T,
{
    let timer = LazyTimer::new(f);
    timer.into_exec()
}

/// use for dirty debugging by printing the time it took to execute
///
/// printing is done to `stdout`
#[inline]
pub fn time_fn_println<T, F>(label: &str, f: F) -> T
where
    F: FnOnce() -> T,
{
    let timer = LazyTimer::new(f);
    let (res, dur) = timer.into_exec();
    println!("{}: {}ms", label, dur.as_millis());
    res
}

/// use for dirty debugging by printing the time it took to execute
///
/// printing is done to `stderr`
#[inline]
pub fn time_fn_eprintln<T, F>(label: &str, f: F) -> T
where
    F: FnOnce() -> T,
{
    let timer = LazyTimer::new(f);
    let (res, dur) = timer.into_exec();
    eprintln!("{}: {}ms", label, dur.as_millis());
    res
}

/// use when you need both the result of the block and the time
/// it took to execute as a tuple.
#[macro_export]
macro_rules! time {
    {$($a:tt)*} => {{
        let timer = || { $($a)* };
        timer.into_lazy_timer().into_exec()
    }};
}

/// use for dirty debugging by printing the time it took to execute
/// the given block.
///
/// can optionally be labeled with a string: `time_println!("label", ...)`
///
/// printing is done to `stdout`
#[macro_export]
macro_rules! time_println {
    {$a:expr, $($b:tt)*} => {{
        let timer = || { $($b)* };
        let (res, dur) = timer.into_lazy_timer().into_exec();
        println!("{}: {}ms", $a, dur.as_millis());
        res
    }};
    {$($a:tt)*} => {{
        let timer = || { $($a)* };
        let (res, dur) = timer.into_lazy_timer().into_exec();
        println!("{}: {}ms", stringify!($($a)*), dur.as_millis());
        res
    }};
}

/// use for dirty debugging by printing the time it took to execute
/// the given block.
///
/// can optionally be labeled with a string: `time_eprintln!("label", ...)`
///
/// printing is done to `stderr`
#[macro_export]
macro_rules! time_eprintln {
    {$a:expr, $($b:tt)*} => {{
        let timer = || { $($b)* };
        let (res, dur) = timer.into_lazy_timer().into_exec();
        eprintln!("{}: {}ms", $a, dur.as_millis());
        res
    }};
    {$($a:tt)*} => {{
        let timer = || { $($a)* };
        let (res, dur) = timer.into_lazy_timer().into_exec();
        eprintln!("{}: {}ms", stringify!($($a)*), dur.as_millis());
        res

    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::iter;

    #[test]
    fn lazy_timer_struct() {
        let my_closure = || 5 + 3;

        let timer = LazyTimer::new(my_closure);
        let (res, _dur) = timer.into_exec();

        assert_eq!(res, 8);
    }

    #[test]
    fn capture() {
        #[derive(PartialEq, Debug)]
        struct Noncopy;
        let capture = Noncopy;
        let closure = || capture;

        let timer = LazyTimer::new(closure);

        assert_eq!(timer.into_exec().0, Noncopy);
    }

    #[test]
    fn as_inner() {
        let closure = || 3 + 5;

        let timer = LazyTimer::new(closure);
        let inner = timer.as_inner();

        assert_eq!(inner(), 8);
        assert_eq!(timer.into_exec().0, 8);
    }

    #[test]
    fn as_inner_mut() {
        let my_closure = || 5 + 3;

        let mut timer = LazyTimer::new(my_closure);
        let my_closure = timer.as_inner_mut();

        assert_eq!(my_closure(), 8);
    }

    #[test]
    fn into_inner() {
        let my_closure = || 5 + 3;

        let timer = LazyTimer::new(my_closure);
        let my_closure = timer.into_inner();

        assert_eq!(my_closure(), 8);
    }

    #[test]
    fn into_lazy_timer_trait() {
        let my_closure = || 5 + 3;

        let timer = my_closure.into_lazy_timer();
        let (res, _dur) = timer.into_exec();

        assert_eq!(res, 8);
    }

    #[test]
    fn into_lazy_timer_trait_capture() {
        #[derive(PartialEq, Debug)]
        struct Noncopy;
        let capture = Noncopy;
        let closure = || capture;

        let timer = closure.into_lazy_timer();

        assert_eq!(timer.into_exec().0, Noncopy);
    }

    #[test]
    fn time_macro() {
        let (res, _dur) = time! {3 + 5};
        assert_eq!(res, 8);
    }

    #[test]
    fn time_unlabeled_println_macro() {
        let res = time_println! {3 + 5};
        assert_eq!(res, 8);
    }

    #[test]
    fn time_unlabeled_eprintln_macro() {
        let res = time_eprintln! {3 + 5};
        assert_eq!(res, 8);
    }

    #[test]
    fn time_labeled_println_macro() {
        let res = time_println! {
            "Labeled stdout println",
            3 + 5
        };
        assert_eq!(res, 8);
    }

    #[test]
    fn time_labeled_eprintln_macro() {
        let res = time_eprintln! {
            "Labeled stderr println",
            3 + 5
        };
        assert_eq!(res, 8);
    }

    #[test]
    fn extensive_test() {
        fn xorshift32(inp: &mut u32) -> u32 {
            let mut x = *inp;
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            *inp = x;
            x
        }
        let mut rng = 0xdead_c0de;
        let mut big_data = iter::from_fn(|| Some(xorshift32(&mut rng)))
            .take(1_000_000)
            .collect::<Vec<_>>();

        let (_, needle_time_unsorted) = time! {
            big_data.iter().find(|&&a| a >= 0xffff_f000)
        };

        let ((), sort_time) = time_fn(|| {
            big_data.sort_unstable();
        });

        let (_, needle_time_sorted) = time_fn(|| big_data.binary_search_by(|a| a.cmp(&0xffff_f000)));

        eprintln!("Unsorted: {}ms", needle_time_unsorted.as_millis());
        eprintln!("Sort: {}ms", sort_time.as_millis());
        eprintln!("Sorted: {}ms", needle_time_sorted.as_millis());
    }
}
