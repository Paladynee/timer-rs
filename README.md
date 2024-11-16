# Voxell's Timers

```rust
let data = time_eprintln!{"Data generation", generate_data() };
// -> Data generation: 309ms
```

This library provides a simple way to time code execution.
Simply use the provided macros/functions to time your code to get a `Duration` and the result of the block/closure.

## Example 1

```rust
let (result, time) = time! {
    for _ in 0..1_000_000 {
        let _ = std::hint::black_box(3);
    }
    10
};
eprintln!("Took {}ms", time.as_millis());
assert_eq!(result, 10);
```

## Example 2

```rust
let haystack = vec![1, 2, 3, 4, 5, 6];
let result = time_println!  {
    "Finding needle in haystack",
    let needle = 4;
    haystack.iter().find(|a| **a == needle)
};

assert_eq!(result, Some(&4));
```

## Example 3

```rust
let haystack = vec![1, 2, 3, 4, 5, 6];
let result = time_fn_println("Finding needle in haystack", || {
   let needle = 4;
   haystack.iter().find(|a| **a == needle)
});
assert_eq!(result, Some(&4));
```
