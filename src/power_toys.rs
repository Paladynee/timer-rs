use core::fmt::{self, Debug, Write};
use std::{
    fmt::Display,
    time::{Duration, Instant},
};

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
    // this also doubles as an infinite-size
    // protector since it is heap allocated.
    children: Vec<ScopedTimer<I>>,
}

impl<I: Eq + Debug> fmt::Debug for ScopedTimer<I> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScopedTimer")
            .field("ident", &self.ident)
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
    /// a list of `(identifier, duration)` pairs.
    ///
    /// Scopes **subtract** time of other scopes forked from it. So you can rest assured the value
    /// with the highest time is the hottest path.
    #[inline]
    pub fn join_and_finish(mut self) -> Vec<(I, Duration)> {
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
        let mut timings = self.join_and_finish();

        timings.sort_unstable_by(|a, b| b.1.cmp(&a.1));

        let strings = timings
            .into_iter()
            .map(|res| {
                (res.0.to_string(), {
                    let mut f = String::new();
                    // string guarantees fmt writes never fail, though the debug impl could, i honestly dont know.
                    // i dont want random panics from happening, so lets just ignore the result.
                    let _ = write!(f, "{:?}", res.1);
                    f
                })
            })
            .collect::<Vec<_>>();

        let (mut longest_ident, mut longest_dur) = strings.iter().fold((0, 0), |(mut longest_ident, mut longest_dur), (ident, dur)| {
            let il = ident.len();
            let dl = dur.len();
            if il > longest_ident {
                longest_ident = il;
            }
            if dl > longest_dur {
                longest_dur = dl;
            }
            (longest_ident, longest_dur)
        });
        longest_ident = longest_ident.max("Identifier".len());
        longest_dur = longest_dur.max("Duration".len());

        let mut buf = String::new();

        // we now have stringified pairs of identifiers and durations along with
        // the longest identifier and duration lengths. the resulting table should look like this:
        /*
           +----------------------+----------------+
           | Identifier           | Duration       |
           +----------------------+----------------+
           | scope 1              | 16.485ms       |
           | scope sdfkljsdfsdf   | 0.00000000001s |
           | hot loop             | 5.34h          |
           +----------------------+----------------+
        */
        // key aspects:
        // - Every textual value is left aligned.
        // - Things represented with strings arent quoted.
        // - At least 1 space before and after any pipe "|".

        // +----------------------+----------------+
        let hline = format!("+{}+{}+", "-".repeat(longest_ident + 2), "-".repeat(longest_dur + 2));

        buf.push_str(&hline);
        buf.push('\n');

        // | Identifier           | Duration       |
        let _ = writeln!(
            buf,
            "| {:<width_id$} | {:<width_dur$} |",
            "Identifier",
            "Duration",
            width_id = longest_ident,
            width_dur = longest_dur
        );

        // +----------------------+----------------+
        buf.push_str(&hline);
        buf.push('\n');

        for (ident, dur) in strings {
            // | scope 1              | 16.485ms       |
            let _ = writeln!(
                buf,
                "| {:<width_id$} | {:<width_dur$} |",
                ident,
                dur,
                width_id = longest_ident,
                width_dur = longest_dur
            );
        }

        // +----------------------+----------------+
        buf.push_str(&hline);

        buf
    }

    // private api because of the recursive nature for children,
    // while also requiring the root also `join`s the horde.
    #[inline]
    fn finish(self, v: &mut Vec<(I, Duration)>) {
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
        v.push((self.ident, horde));
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
        // FIXME: after polonius gets released, change this to
        // ```
        // if let Some (res) = v.iter_mut().find(...) { ... return fjh; }`
        // ```
        // we need to drop the reference so that the iterator over the vector is no longer valid,
        // and we can mutably reference the vector again.

        // Safety: the index is returned by the `.iter().position()`, which guarantees
        // things exist when the vector couldn't possibly have changed after returning `Some`.
        let entry = unsafe { v.get_unchecked_mut(index) };
        entry.start = Instant::now();

        let cjh = ScopeJoinHandle { inner: entry };
        return cjh;
    }

    let timer = ScopedTimer::new(ident);
    v.push(timer);

    let cjh = ScopeJoinHandle {
        // Safety: we pushed 1 line before this, the vector can't possibly be empty.
        // in order for the vec to be empty it needs to fail while pushing the element.
        // it panics when it fails, so its impossible to reach here.
        inner: unsafe { v.last_mut().unwrap_unchecked() },
    };
    cjh
}

#[inline]
#[test]
pub fn test() {
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

    let results = tlt.join_and_finish_pretty();
    eprintln!("{}", results);
}
