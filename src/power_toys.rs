use core::fmt::{self, Debug, Write};
use std::{
    fmt::Display,
    time::{Duration, Instant},
};

// not public intentionally, we forward the method names from the implementations instead
trait Forkable<I: Eq + Clone> {
    /// do not use zero sized types or non-easily-comparable identifiers as identifiers
    /// for your top level timer. it will mess up with the result, since we have no
    /// way of determining the scope at which you probe the timer. it is used
    /// as a means of discern 2 different scopes of the same lifetime.
    ///
    /// take this code as an example:
    /// ```
    /// # use voxell_timer::power_toys::ScopedTimer;
    /// # let mut timer: ScopedTimer<usize> = ScopedTimer::new(0);
    /// {
    ///     let tomfoolery = timer.fork(1); // <- DO NOT DO THIS
    ///     // expensive work ...
    ///     tomfoolery.join();
    /// }
    ///
    /// let not_the_same_scope = timer.fork(1); // <- DO NOT DO THIS
    /// // expensive work ...
    /// not_the_same_scope.join();
    /// ```
    ///
    /// we cant discern between the two, since they share the `timer` parent and have
    /// no other means to be created. we also have to this to allow this code to
    /// be accepted under the same "scope" instead of creating multiple different ones:
    /// ```
    /// # use voxell_timer::power_toys::ScopedTimer;
    /// # let mut timer: ScopedTimer<&str> = ScopedTimer::new(" ");
    /// # let iterable = (0..2);
    /// for _ in iterable.into_iter() {
    ///     let scope = timer.fork("hot loop");
    ///     // expensive operation ...
    ///     scope.join();
    /// }
    /// ```
    ///
    /// instead, please use the same identifier for scopes you wish to be merged,
    /// and different identifiers for scopes you wish to be separate:
    /// ```
    /// # use voxell_timer::power_toys::ScopedTimer;
    /// # let mut timer: ScopedTimer<usize> = ScopedTimer::new(0);
    /// {
    ///     let srs_scope = timer.fork(1); // do this
    ///     // expensive work...
    ///     srs_scope.join();
    /// }
    ///
    /// let not_the_same_scope = timer.fork(2); //
    /// // other work..
    /// not_the_same_scope.join();
    /// ```
    fn fork<'prt>(&'prt mut self, ident: I) -> ScopeJoinHandle<'prt, I>;
}

/// A slightly complex perf tool that you can implement manual flamegraphs with.
///
/// ## The Generics
///
/// The generic parameter `T` is used for identifying different scopes. Please choose
/// an appropiate type that can discern between the different scopes you desire,
/// and isn't zero sized.
///
/// ## `ScopedTimer`
///
/// Each [`ScopedTimer<T>`] represents a perf session for a specific scope. You make a session
/// using the  [`ScopedTimer::new`] function, or use [`ScopedTimer::fork`] to nest another scope
/// by giving it a unique identifier.
///
/// ## `ScopeJoinHandle`
///
/// The returned [`ScopeJoinHandle`] can also be forked, allowing you to nest
/// your scopes. Whenever you are done with the scope you want to perf, either call [`ScopeJoinHandle::join`]
/// or drop the [`ScopeJoinHandle`]. When you are done, call [`ScopedTimer::join_and_finish`]
/// to get your timings, or [`ScopedTimer::join_and_finish_pretty`] for a pretty table.
///
/// ## Getting your timings
///
/// Scopes **subtract** time of other scopes forked from it. So you can rest assured the value
/// with the highest time is the hottest path.
#[derive(Clone)]
pub struct ScopedTimer<I>
where
    I: Eq,
{
    ident: I,
    start: Instant,
    accumulated: Duration,
    times_forked: u32,
    // this also doubles as an infinite-size
    // protector since it is heap allocated.
    children: Vec<ScopedTimer<I>>,
}

impl<I: Eq + Debug> fmt::Debug for ScopedTimer<I> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScopedTimer")
            .field("ident", &self.ident)
            .field("times_forked", &self.times_forked)
            .field("start", &"{ some point in time }")
            .field("accumulated", &self.accumulated)
            .field("children", &self.children)
            .finish()
    }
}

impl<I: Eq + Clone> Forkable<I> for ScopedTimer<I> {
    /// do not use zero sized types or non-easily-comparable identifiers as identifiers
    /// for your top level timer. it will mess up with the result, since we have no
    /// way of determining the scope at which you probe the timer. it is used
    /// as a means of discern 2 different scopes of the same lifetime.
    ///
    /// take this code as an example:
    /// ```
    /// # use voxell_timer::power_toys::ScopedTimer;
    /// # let mut timer: ScopedTimer<usize> = ScopedTimer::new(0);
    /// {
    ///     let tomfoolery = timer.fork(1); // <- DO NOT DO THIS
    ///     // expensive work ...
    ///     tomfoolery.join();
    /// }
    ///
    /// let not_the_same_scope = timer.fork(1); // <- DO NOT DO THIS
    /// // expensive work ...
    /// not_the_same_scope.join();
    /// ```
    ///
    /// we cant discern between the two, since they share the `timer` parent and have
    /// no other means to be created. we also have to this to allow this code to
    /// be accepted under the same "scope" instead of creating multiple different ones:
    /// ```
    /// # use voxell_timer::power_toys::ScopedTimer;
    /// # let mut timer: ScopedTimer<&str> = ScopedTimer::new(" ");
    /// # let iterable = (0..2);
    /// for _ in iterable.into_iter() {
    ///     let scope = timer.fork("hot loop");
    ///     // expensive operation ...
    ///     scope.join();
    /// }
    /// ```
    ///
    /// instead, please use the same identifier for scopes you wish to be merged,
    /// and different identifiers for scopes you wish to be separate:
    /// ```
    /// # use voxell_timer::power_toys::ScopedTimer;
    /// # let mut timer: ScopedTimer<usize> = ScopedTimer::new(0);
    /// {
    ///     let srs_scope = timer.fork(1); // do this
    ///     // expensive work...
    ///     srs_scope.join();
    /// }
    ///
    /// let not_the_same_scope = timer.fork(2); //
    /// // other work..
    /// not_the_same_scope.join();
    /// ```
    #[inline]
    fn fork<'prt>(&'prt mut self, ident: I) -> ScopeJoinHandle<'prt, I> {
        self.times_forked += 1;
        search_and_push(&mut self.children, ident)
    }
}

impl<I: Eq + Clone> ScopedTimer<I> {
    /// Make a new perf session for a specific scope. Note that
    /// nesting scopes together is a better idea than creating constantly
    /// using `new`.
    #[inline]
    pub fn new(ident: I) -> Self {
        Self {
            ident,
            start: Instant::now(),
            accumulated: Duration::ZERO,
            times_forked: 0,
            children: Vec::new(),
        }
    }

    /// Fork the `ScopedTimer` and return the handle to the child scope. The child
    /// scope holds a mutable reference to the parent scope, and thus the parent scope
    /// can't have 2 children at once. `ScopeJoinHandle` implement `Forkable`, so you
    /// can fork that to have nested children.
    #[inline]
    pub fn fork<'prt>(&'prt mut self, ident: I) -> ScopeJoinHandle<'prt, I> {
        <Self as Forkable<I>>::fork(self, ident)
    }

    /// Collect all the timed values from all child scopes and returns
    /// a list of `(identifier, duration, times_forked)` tuples.
    ///
    /// Scopes **subtract** time of other scopes forked from it. So you can rest assured the value
    /// with the highest time is the hottest path.
    #[inline]
    pub fn join_and_finish(mut self) -> Vec<(I, Duration, u32)> {
        self.join();

        let mut vec = vec![];
        self.finish(&mut vec);
        vec
    }

    /// Collect all the timed values from all child scopes and returns
    /// a pretty table of the hottest paths.
    ///
    /// Scopes **subtract** time of other scopes forked from it. So you can rest assured the value
    /// with the highest time is the hottest path.
    #[inline]
    pub fn join_and_finish_pretty(self) -> String
    where
        I: Display,
    {
        const IDENT: &str = "Identifier";
        const DURAT: &str = "Duration";
        const TIMESF: &str = "Times Forked";

        let mut timings = self.join_and_finish();

        timings.sort_unstable_by(|a, b| b.1.cmp(&a.1));

        let strings = timings
            .into_iter()
            .map(|res| {
                (
                    res.0.to_string(),
                    {
                        let mut f = String::new();
                        // string guarantees fmt writes never fail. even though,
                        // i dont want random panics, so lets just ignore the result.
                        // as per the standard library, string formatting is an infallible operation.
                        let _ = write!(f, "{:?}", res.1);
                        f
                    },
                    res.2.to_string(),
                )
            })
            .collect::<Vec<_>>();

        let (mut longest_ident, mut longest_dur, mut longest_fork) =
            strings
                .iter()
                .fold((0, 0, 0), |(mut longest_ident, mut longest_dur, mut longest_fork), (ident, dur, fork)| {
                    longest_ident = longest_ident.max(ident.len());
                    longest_dur = longest_dur.max(dur.len());
                    longest_fork = longest_fork.max(fork.len());
                    (longest_ident, longest_dur, longest_fork)
                });

        longest_ident = longest_ident.max(IDENT.len());
        longest_dur = longest_dur.max(DURAT.len());
        longest_fork = longest_fork.max(TIMESF.len());

        // we now have stringified pairs of identifiers and durations along with
        // the longest identifier and duration lengths. the resulting table should look like this:
        /*
           +----------------------+----------------+--------------+
           | Identifier           | Duration       | Times Forked |
           +----------------------+----------------+--------------+
           | scope 1              | 16.485ms       | 15           |
           | scope sdfkljsdfsdf   | 0.00000000001s | 1            |
           | hot loop             | 5.34h          | 3651343      |
           +----------------------+----------------+--------------+
        */
        // key aspects:
        // - Every textual value is left aligned.
        // - Things represented with strings arent quoted.
        // - At least 1 space before and after any pipe "|".

        let mut buf = String::new();

        // +----------------------+----------------+--------------+
        let hline = format!(
            "+{}+{}+{}+",
            "-".repeat(longest_ident + 2),
            "-".repeat(longest_dur + 2),
            "-".repeat(longest_fork + 2)
        );

        buf.push_str(&hline);
        buf.push('\n');

        // string guarantees fmt writes never fail. even though,
        // i dont want random panics, so lets just ignore the result.
        // as per the standard library, string formatting is an infallible operation.
        // | Identifier           | Duration       | Times Forked |
        let _ = writeln!(
            buf,
            "| {:<width_id$} | {:<width_dur$} | {:<width_fork$} |",
            IDENT,
            DURAT,
            TIMESF,
            width_id = longest_ident,
            width_dur = longest_dur,
            width_fork = longest_fork
        );

        // +----------------------+----------------+--------------+
        buf.push_str(&hline);
        buf.push('\n');

        for (ident, dur, fork) in strings {
            // string guarantees fmt writes never fail. even though,
            // i dont want random panics, so lets just ignore the result.
            // as per the standard library, string formatting is an infallible operation.
            // | scope 1              | 16.485ms       | 15           |
            let _ = writeln!(
                buf,
                "| {:<width_id$} | {:<width_dur$} | {:<width_fork$} |",
                ident,
                dur,
                fork,
                width_id = longest_ident,
                width_dur = longest_dur,
                width_fork = longest_fork
            );
        }

        // +----------------------+----------------+--------------+
        buf.push_str(&hline);

        buf
    }

    // private api because of the recursive nature for children,
    // while also requiring the root also `join`s the horde.
    #[inline]
    fn finish(self, v: &mut Vec<(I, Duration, u32)>) {
        let mut horde = self.accumulated;
        let mut chillated = Duration::ZERO;

        for child in self.children {
            chillated += child.accumulated;
            child.finish(v);
        }

        // prevent underflow when subtracting child durations
        if chillated <= horde {
            horde -= chillated;
        } else {
            // saturate on underflow
            horde = Duration::ZERO;
        }
        v.push((self.ident, horde, self.times_forked));
    }

    // private api
    #[inline]
    fn join(&mut self) {
        self.accumulated += self.start.elapsed();
    }
}

/// A join handle forked from a [`ScopedTimer`] or another [`ScopeJoinHandle`].
///
/// It holds a mutable reference to its parent, so you can't fork from its parent
/// without joining it first. If you're looking to nest scopes, [`ScopeJoinHandle::fork`] from this instead.
///
/// [`ScopeJoinHandle::join`] to time the scope and destroy this handle.
#[repr(transparent)]
pub struct ScopeJoinHandle<'a, T>
where
    T: Eq + Clone,
{
    inner: &'a mut ScopedTimer<T>,
}

impl<'bef, I: Eq + Clone> Forkable<I> for ScopeJoinHandle<'bef, I> {
    #[inline]
    fn fork<'prt>(&'prt mut self, ident: I) -> ScopeJoinHandle<'prt, I> {
        search_and_push(&mut self.inner.children, ident)
    }
}

impl<'bef, T: Eq + Clone> ScopeJoinHandle<'bef, T> {
    /// do not use zero sized types or non-easily-comparable identifiers as identifiers
    /// for your top level timer. it will mess up with the result, since we have no
    /// way of determining the scope at which you probe the timer. it is used
    /// as a means of discern 2 different scopes of the same lifetime.
    ///
    /// take this code as an example:
    /// ```
    /// # use voxell_timer::power_toys::ScopedTimer;
    /// # let mut timer: ScopedTimer<usize> = ScopedTimer::new(0);
    /// {
    ///     let tomfoolery = timer.fork(1); // <- DO NOT DO THIS
    ///     // expensive work ...
    ///     tomfoolery.join();
    /// }
    ///
    /// let not_the_same_scope = timer.fork(1); // <- DO NOT DO THIS
    /// // expensive work ...
    /// not_the_same_scope.join();
    /// ```
    ///
    /// we cant discern between the two, since they share the `timer` parent and have
    /// no other means to be created. we also have to this to allow this code to
    /// be accepted under the same "scope" instead of creating multiple different ones:
    /// ```
    /// # use voxell_timer::power_toys::ScopedTimer;
    /// # let mut timer: ScopedTimer<&str> = ScopedTimer::new(" ");
    /// # let iterable = (0..2);
    /// for _ in iterable.into_iter() {
    ///     let scope = timer.fork("hot loop");
    ///     // expensive operation ...
    ///     scope.join();
    /// }
    /// ```
    ///
    /// instead, please use the same identifier for scopes you wish to be merged,
    /// and different identifiers for scopes you wish to be separate:
    /// ```
    /// # use voxell_timer::power_toys::ScopedTimer;
    /// # let mut timer: ScopedTimer<usize> = ScopedTimer::new(0);
    /// {
    ///     let srs_scope = timer.fork(1); // do this
    ///     // expensive work...
    ///     srs_scope.join();
    /// }
    ///
    /// let not_the_same_scope = timer.fork(2); //
    /// // other work..
    /// not_the_same_scope.join();
    /// ```
    #[inline]
    pub fn fork<'prt>(&'prt mut self, ident: T) -> ScopeJoinHandle<'prt, T> {
        <Self as Forkable<T>>::fork(self, ident)
    }

    /// joins the scope to the parent, saving how much time has passed
    /// since its forked and until its joined.
    #[inline]
    pub fn join(self) {
        // join logic handled in the Drop impl
    }
}

impl<I: Eq + Clone> Drop for ScopeJoinHandle<'_, I> {
    #[inline]
    fn drop(&mut self) {
        self.inner.join();
    }
}

#[inline]
fn search_and_push<'vec, I: Eq + Clone>(v: &'vec mut Vec<ScopedTimer<I>>, ident: I) -> ScopeJoinHandle<'vec, I> {
    let find = v.iter().position(|child| child.ident == ident);
    if let Some(index) = find {
        // FIXME: when the borrow checker is replaced with Polonius replace this part with
        // ```
        // if let Some (res) = v.iter_mut().find(...) { ... return fjh; }`
        // ```
        // we need to drop the reference so that the iterator over the vector is no longer valid,
        // and we can mutably reference the vector again. this is always safe to do, and it's
        // a current limitation of the borrow checker that rejects sound code.

        // Safety: the index is returned by the `.iter().position()`, which guarantees
        // things exist when the vector couldn't possibly have changed after returning `Some`.
        let entry = unsafe { v.get_unchecked_mut(index) };
        entry.times_forked += 1;

        let cjh = ScopeJoinHandle { inner: entry };
        // do not account for addassign
        cjh.inner.start = Instant::now();
        return cjh;
    }

    let mut timer = ScopedTimer::new(ident);
    timer.times_forked = 1;
    v.push(timer);

    let cjh = ScopeJoinHandle {
        // Safety: Vec::push panics if the push wasn't succesful,
        // it is guaranteed that there is a last element.
        inner: unsafe { v.last_mut().unwrap_unchecked() },
    };
    // do not account for potential vec growth in the output
    cjh.inner.start = Instant::now();
    cjh
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test() {
        use std::thread;
        fn sleep(x: u64) {
            thread::sleep(Duration::from_millis(x / 10));
        }

        let mut tlt: ScopedTimer<usize> = ScopedTimer::new(0);
        // ensure tlt can be forked.
        let fork1 = tlt.fork(1);
        sleep(333);
        fork1.join();

        let mut fork2 = tlt.fork(2);
        // ensure fork2 can be forked.
        // (`impl Forkable for T` error otherwise, lifetime comes from callsite of `fork`)
        let mut fork3 = fork2.fork(3);
        let mut fork4 = fork3.fork(4);
        let fork5 = fork4.fork(5);
        sleep(1000);
        // ensure lifetimes work
        fork5.join();
        fork4.join();
        fork3.join();
        // ensure fork 2, 3, and 4 are all relatively close to 0. fork 5 did all the work.
        fork2.join();

        let mut fork6 = tlt.fork(6);
        for _ in 0..6 {
            // ensure the same scope gets reused in `search_and_push` (scope with ident 7),
            // such that we dont get duplicate scopes.
            let fork7 = fork6.fork(7);
            sleep(666);
            fork7.join();
        }
        // ensure fork6 is relatively close to 0. fork7 did all the hard work.
        fork6.join();

        let mut fork8 = tlt.fork(8);
        for _ in 0..100000 {
            let fork9 = fork8.fork(9);
            // measures how much overhead making a fork and joining it has.
            fork9.join();
        }
        // measures how much overhead making a fork and joining it has.
        fork8.join();

        let results = tlt.join_and_finish_pretty();
        eprintln!("{}", results);
    }
}
