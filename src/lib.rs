#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
use core::time::Duration;
use std::time::Instant;

/// A more complex timer.
pub mod power_toys;

/// use when you need both the result of the closure and the time
/// it took to execute as a tuple.
#[inline]
pub fn time_fn<T, F>(f: F) -> (T, Duration)
where
    F: FnOnce() -> T,
{
    let start = Instant::now();
    let res = f();
    let dur = start.elapsed();
    (res, dur)
}

/// use for dirty debugging by printing the time it took to execute
///
/// printing is done to `stdout`
#[inline]
pub fn time_fn_println<T, F>(label: &str, f: F) -> T
where
    F: FnOnce() -> T,
{
    let (res, dur) = time_fn(f);
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
    let (res, dur) = time_fn(f);
    eprintln!("{}: {}ms", label, dur.as_millis());
    res
}

/// use when you need both the result of the block and the time
/// it took to execute as a tuple.
#[macro_export]
macro_rules! time {
    {$($a:tt)*} => {{
        let f = || { $($a)* };
        $crate::time_fn(f)
    }};
}

/// use for dirty debugging by printing the time it took to execute
/// the given block.
///
/// can optionally be labeled with a string: `time_println!("label", ...)`
///
/// or with an unquoted string for some reason: `time_println!(unquoted label, ...)`
///
/// printing is done to `stdout`
#[macro_export]
macro_rules! time_println {
    // macro time_println(unquoted label..., code()... ) -> code()::output
    {$($a:ident)*, $($b:tt)*} => {{
        let f = || { $($b)* };
        let (res, dur) = $crate::time_fn(f);
        println!("{}: {}ms", stringify!($($a)*), dur.as_millis());
        res
    }};

    // macro time_println(label: &str, code()...) -> code()::output
    {$a:expr, $($b:tt)*} => {{
        let f = || { $($b)* };
        let (res, dur) = $crate::time_fn(f);
        println!("{}: {}ms", $a, dur.as_millis());
        res
    }};

    // macro time_println(code()...) -> code()::output
    {$($a:tt)*} => {{
        let f = || { $($a)* };
        let (res, dur) = $crate::time_fn(f);
        println!("{}: {}ms", stringify!($($a)*), dur.as_millis());
        res
    }};
}

/// use for dirty debugging by printing the time it took to execute
/// the given block.
///
/// can optionally be labeled with a string: `time_println!("label", ...)`
///
/// or with an unquoted string for some reason: `time_println!(unquoted label, ...)`
///
/// printing is done to `stderr`
#[macro_export]
macro_rules! time_eprintln {
    // macro time_println(unquoted label..., code()... ) -> code()::output
    {$($a:ident)*, $($b:tt)*} => {{
        let f = || { $($b)* };
        let (res, dur) = $crate::time_fn(f);
        eprintln!("{}: {}ms", stringify!($($a)*), dur.as_millis());
        res
    }};

    // macro time_println(label: &str, code()...) -> code()::output
    {$a:expr, $($b:tt)*} => {{
        let f = || { $($b)* };
        let (res, dur) = $crate::time_fn(f);
        eprintln!("{}: {}ms", $a, dur.as_millis());
        res
    }};

    // macro time_println(code()...) -> code()::output
    {$($a:tt)*} => {{
        let f = || { $($a)* };
        let (res, dur) = $crate::time_fn(f);
        eprintln!("{}: {}ms", stringify!($($a)*), dur.as_millis());
        res
    }};
}
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use core::iter;

//     #[test]
//     fn works() {
//         let f = || 5 + 3;
//         let (res, _dur) = time_fn(f);
//         assert_eq!(res, 8);
//     }

//     #[test]
//     fn noncopy() {
//         #[derive(PartialEq, Debug)]
//         struct Noncopy;
//         let capture = Noncopy;
//         let f = || capture;

//         let (res, _dur) = time_fn(f);

//         assert_eq!(res, Noncopy);
//     }

//     #[test]
//     fn time_macro() {
//         let (res, _dur) = time! {3 + 5};
//         assert_eq!(res, 8);
//     }

//     #[test]
//     fn time_unlabeled_println_macro() {
//         let res = time_println! {3 + 5};
//         assert_eq!(res, 8);
//     }

//     #[test]
//     fn time_unlabeled_eprintln_macro() {
//         let res = time_eprintln! {3 + 5};
//         assert_eq!(res, 8);
//     }

//     #[test]
//     fn time_labeled_println_macro() {
//         let res = time_println! {
//             "Labeled stdout println",
//             3 + 5
//         };
//         assert_eq!(res, 8);
//     }

//     #[test]
//     fn time_labeled_eprintln_macro() {
//         let res = time_eprintln! {
//             "Labeled stderr println",
//             3 + 5
//         };
//         assert_eq!(res, 8);
//     }

//     #[test]
//     fn time_labeled_println_macro_no_quotes() {
//         let res = time_println! {
//             unquoted label,
//             3 + 5
//         };
//         assert_eq!(res, 8);
//     }

//     #[test]
//     fn time_labeled_eprintln_macro_no_quotes() {
//         let res = time_eprintln! {
//             unquoted label,
//             3 + 5
//         };
//         assert_eq!(res, 8);
//     }

//     #[test]
//     fn extensive_test() {
//         fn xorshift32(inp: &mut u32) -> u32 {
//             let mut x = *inp;
//             x ^= x << 13;
//             x ^= x >> 17;
//             x ^= x << 5;
//             *inp = x;
//             x
//         }
//         let mut rng = 0xdead_c0de;
//         let mut big_data = iter::from_fn(|| Some(xorshift32(&mut rng))).take(1_000_000).collect::<Vec<_>>();

//         let (_, needle_time_unsorted) = time! {
//             big_data.iter().find(|&&a| a >= 0xffff_f000)
//         };

//         let ((), sort_time) = time_fn(|| {
//             big_data.sort_unstable();
//         });

//         let (_, needle_time_sorted) = time_fn(|| big_data.binary_search_by(|a| a.cmp(&0xffff_f000)));

//         eprintln!("Unsorted: {}ms", needle_time_unsorted.as_millis());
//         eprintln!("Sort: {}ms", sort_time.as_millis());
//         eprintln!("Sorted: {}ms", needle_time_sorted.as_millis());
//     }
// }
