# Voxell's Timers

```rust,ignore
let data = time_eprintln!{hello mom, generate_data() };
// -> hello mom: 309ms
```

This library provides a simple way to time code execution.
Simply use the provided macros/functions to time your code to get a `Duration` and the result of the block/closure.

## Simple example

```rust
use voxell_timer::*;
use core::time::Duration;

let (result, time): (&str, Duration) = time! {
    for _ in 0..1_000_000 {
        // expensive operation ...
    }
    "hello, timers!"
};
eprintln!("Took {}ms", time.as_millis());
assert_eq!(result, "hello, timers!");
```

## Complicated `ScopedTimer` example

This example uses `ScopedTimer` to profile nested loops. In the code below, the outer loop creates its own scope while the inner loop creates nested scopes. You can view the timings associated with each scope.

```rust
use voxell_timer::power_toys::ScopedTimer;
use std::thread;
use std::time::Duration;

fn sleep(ms: u64) {
    thread::sleep(Duration::from_millis(ms / 10));
}

// create a performance session with identifier type &str
let mut session = ScopedTimer::<&str>::new("total");

let mut outer = session.fork("outer loop"); // <- give a unique name!
for _ in 0..3 {
    //          VVVVV you can nest scopes!
    let mut inner = outer.fork("inner loop"); // <- give a unique name!

    // expensive work...
    sleep(200);

    for _ in 0..4 {
        //           VVVVV so many nests...
        let innest = inner.fork("innest loop"); // <- give a unique name!

        // more work ...
        sleep(100);

        innest.join(); // <- times the innest scope.
    }
    inner.join(); // <- times the inner scope.
}
outer.join(); // <- times the outer scope

let results = session.join_and_finish();
println!("{:#?}", results);
// inner loop: 600ms
// innest loop: 1.2s
// outer loop: 0ms
//             ^ scopes only time their own!
```

## Example

```rust
use voxell_timer::*;

let haystack = vec![1, 2, 3, 4, 5, 6];
let result = time_println! {
    "Finding needle in haystack",
    let needle = 4;
    haystack.iter().find(|a| **a == needle)
};

assert_eq!(result, Some(&4));
```

## Example

```rust
use voxell_timer::*;

let haystack = vec![1, 2, 3, 4, 5, 6];
let result = time_fn_println("Finding needle in haystack", || {
    let needle = 4;
    haystack.iter().find(|a| **a == needle)
});
assert_eq!(result, Some(&4));
```
