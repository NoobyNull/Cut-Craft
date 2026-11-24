//! The one way heavy calculations get threads: every simulation, preview, import and
//! toolpath that spreads over the processor's cores starts them here, and nowhere else (a
//! test in the app fails on a thread started anywhere else).
//!
//! Here they get:
//! - **How many cores**: Options ▸ Performance's limit ([`Limit`]), by default 80% of them
//!   rounded down, so the computer keeps some for everything else.
//! - **Their priority**: while the limit is on, each thread lowers its own priority as it
//!   starts ([`cut_craft_priority`]), so a video, the browser and Cut Craft's own window come
//!   first. It still runs flat out when nothing else wants the processor.
//! - **A count of what is running** ([`running`]), which the status bar's [`Gauge`] turns into
//!   how briskly the calculation is going: Crawl, Walk, Run or Sprint ([`Pace`]).
//!
//! A new limit applies from the next calculation; one already running keeps its threads.

use std::cell::Cell;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use std::thread::{JoinHandle, Scope, ScopedJoinHandle};
use std::time::{Duration, Instant};

/// How much of the computer calculations may use: Options ▸ Performance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limit {
    /// Whether there is a limit at all: off, calculations use every core at normal priority.
    pub on: bool,
    /// The share of the cores calculations may use while the limit is on, percent.
    pub share: u8,
}

/// When the graphics card, which draws the desktop too, is free for calculations again:
/// after each piece of work it rests as the limit asks ([`Limit::rest_after`]). The rest is
/// owed to whoever hands it work next, so a calculation that sends one short job after
/// another rests between the jobs as well, and nothing waits once the last one is done.
#[derive(Debug, Default)]
pub struct Rests {
    until: Option<Instant>,
}

impl Rests {
    /// How long work arriving at `now` waits for the card's rest to end.
    pub fn owed(&self, now: Instant) -> Duration {
        self.until
            .map_or(Duration::ZERO, |until| until.saturating_duration_since(now))
    }

    /// Notes a piece that kept the card busy from `start` to `end`, under `limit`.
    pub fn worked(&mut self, start: Instant, end: Instant, limit: Limit) {
        self.until = Some(end + limit.rest_after(end.saturating_duration_since(start)));
    }

    /// Runs `each` on every piece in turn, each after the rest owed before it.
    pub fn paced<I: IntoIterator>(&mut self, pieces: I, mut each: impl FnMut(I::Item)) {
        let limit = limit();
        for piece in pieces {
            std::thread::sleep(self.owed(Instant::now()));
            let start = Instant::now();
            each(piece);
            self.worked(start, Instant::now(), limit);
        }
    }
}

/// The share of the cores calculations use unless Options says otherwise, percent.
pub const DEFAULT_SHARE: u8 = 80;

/// The least share of the time work on the graphics card is given, percent.
const CARD_LEAST: u8 = 10;

impl Default for Limit {
    fn default() -> Self {
        Self {
            on: true,
            share: DEFAULT_SHARE,
        }
    }
}

impl Limit {
    /// No limit: every core at normal priority.
    pub const OFF: Self = Self {
        on: false,
        share: 100,
    };

    /// How many of `all` cores calculations may use: the share of them rounded down, at
    /// least one, at most all of them; all of them with no limit.
    pub fn cores_of(self, all: usize) -> usize {
        let all = all.max(1);
        if !self.on {
            return all;
        }
        (all * usize::from(self.share.min(100)) / 100).clamp(1, all)
    }

    /// How long work on the graphics card rests after a piece that kept it `busy`, so the
    /// card (which also draws the desktop and your videos) works only the limit's share of
    /// the time: a quarter of the piece at 80%, as long again at 50%, none with no limit.
    /// The card is given at least a tenth of the time, so a tiny share does not leave a
    /// preview waiting minutes.
    pub fn rest_after(self, busy: Duration) -> Duration {
        if !self.on {
            return Duration::ZERO;
        }
        let share = u32::from(self.share.clamp(CARD_LEAST, 100));
        busy * (100 - share) / share
    }

    /// The limit that gives the cores the old Advanced setting "Processor cores to use
    /// (0 = all)" asked for, on a computer with `all` of them: 0 was the default, every core,
    /// and becomes the new default.
    pub fn from_old_cores(cores: usize, all: usize) -> Self {
        if cores == 0 {
            return Self::default();
        }
        let all = all.max(1);
        let share = (cores.min(all) * 100).div_ceil(all);
        Self {
            on: true,
            share: u8::try_from(share).unwrap_or(100),
        }
    }
}

static ON: AtomicBool = AtomicBool::new(true);
static SHARE: AtomicU8 = AtomicU8::new(DEFAULT_SHARE);

/// Sets the limit calculations started from now on keep to.
pub fn set_limit(limit: Limit) {
    SHARE.store(limit.share, Ordering::Relaxed);
    ON.store(limit.on, Ordering::Relaxed);
}

/// The limit calculations keep to.
pub fn limit() -> Limit {
    Limit {
        on: ON.load(Ordering::Relaxed),
        share: SHARE.load(Ordering::Relaxed),
    }
}

/// Every core the computer has.
pub fn all_cores() -> usize {
    std::thread::available_parallelism().map_or(1, std::num::NonZero::get)
}

/// The cores a calculation may spread over now.
pub fn cores() -> usize {
    limit().cores_of(all_cores())
}

/// Calculation threads running now, not counting one waiting on its own workers.
static RUNNING: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    /// Whether this thread is a calculation thread counted in [`RUNNING`].
    static COUNTED: Cell<bool> = const { Cell::new(false) };
}

/// How many calculation threads are working now: none means nothing is calculating.
pub fn running() -> usize {
    RUNNING.load(Ordering::Relaxed)
}

/// This thread counted as a calculation thread (and lowered, while the limit is on) until
/// dropped.
struct Counted;

impl Counted {
    fn start() -> Self {
        if limit().on {
            cut_craft_priority::lower_this_thread();
        }
        COUNTED.set(true);
        RUNNING.fetch_add(1, Ordering::Relaxed);
        Self
    }
}

impl Drop for Counted {
    fn drop(&mut self) {
        RUNNING.fetch_sub(1, Ordering::Relaxed);
        COUNTED.set(false);
    }
}

/// While held, a counted thread waiting on its own workers is not counted as working.
struct Waiting(bool);

impl Waiting {
    fn start() -> Self {
        let counted = COUNTED.get();
        if counted {
            RUNNING.fetch_sub(1, Ordering::Relaxed);
        }
        Self(counted)
    }
}

impl Drop for Waiting {
    fn drop(&mut self) {
        if self.0 {
            RUNNING.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// `wait` run by a calculation thread that is only waiting (for work to come, or on another
/// program that does the calculating), not counted as working meanwhile: a thread kept for
/// work that has none is not a calculation running.
pub fn waiting<T>(wait: impl FnOnce() -> T) -> T {
    let _waiting = Waiting::start();
    wait()
}

/// Starts one of a calculation's threads in `scope`: lowered while the limit is on, and
/// counted as running until it ends. How many to start is [`cores`]'s to say.
pub fn spawn<'scope, T: Send + 'scope>(
    scope: &'scope Scope<'scope, '_>,
    work: impl FnOnce() -> T + Send + 'scope,
) -> ScopedJoinHandle<'scope, T> {
    scope.spawn(move || {
        let _counted = Counted::start();
        work()
    })
}

/// `work` with its own threads started by [`spawn`] in a scope, which it waits for: the same
/// as [`std::thread::scope`], with the waiting thread not counted as working meanwhile.
pub fn scope<'env, T>(work: impl for<'scope> FnOnce(&'scope Scope<'scope, 'env>) -> T) -> T {
    let _waiting = Waiting::start();
    std::thread::scope(work)
}

/// Starts a calculation on a thread of its own, named `name`, that outlives the caller (a
/// toolpath's calculation behind the progress window): lowered while the limit is on, and
/// counted as running until it ends.
pub fn background<T: Send + 'static>(
    name: &str,
    work: impl FnOnce() -> T + Send + 'static,
) -> std::io::Result<JoinHandle<T>> {
    std::thread::Builder::new()
        .name(name.to_owned())
        .spawn(move || {
            let _counted = Counted::start();
            work()
        })
}

/// `f` of each of `0..count`, in order, worked out across the cores: each thread takes every
/// `threads`th one.
pub fn parallel<T: Send>(count: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    let threads = cores().min(count).max(1);
    if threads == 1 {
        return (0..count).map(f).collect();
    }
    let f = &f;
    let parts: Vec<Vec<(usize, T)>> = scope(|s| {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                spawn(s, move || {
                    (t..count)
                        .step_by(threads)
                        .map(|i| (i, f(i)))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap_or_else(|e| std::panic::resume_unwind(e)))
            .collect()
    });
    let mut all: Vec<(usize, T)> = parts.into_iter().flatten().collect();
    all.sort_by_key(|(i, _)| *i);
    all.into_iter().map(|(_, t)| t).collect()
}

/// Runs `work` on each of `jobs`, on `threads` threads each taking the next as it comes free.
pub fn queued<J: Send>(jobs: Vec<J>, threads: usize, work: impl Fn(J) + Sync) {
    if threads <= 1 {
        jobs.into_iter().rev().for_each(work);
        return;
    }
    let queue = Mutex::new(jobs);
    let (queue, work) = (&queue, &work);
    scope(|s| {
        for _ in 0..threads {
            spawn(s, move || {
                loop {
                    let next = queue.lock().expect("a job panicked").pop();
                    let Some(job) = next else { break };
                    work(job);
                }
            });
        }
    });
}

/// Grids under this many cells are worked on the calling thread alone: threads cost more than
/// they save there.
const PARALLEL_CELLS: usize = 4096;

/// Bands handed out per core: smaller bands than a core's share, taken as cores come free,
/// so work bunched in part of the grid does not leave the other cores waiting.
const BANDS_PER_CORE: usize = 4;

/// Fills `values`, a grid `columns` wide, in bands of rows across the cores: `fill(first, band)`
/// gets each band's first row and its cells. A grid under a few thousand cells is one band.
pub fn by_rows<T: Send>(values: &mut [T], columns: usize, fill: impl Fn(usize, &mut [T]) + Sync) {
    let rows = values.len() / columns.max(1);
    let threads = if values.len() < PARALLEL_CELLS {
        1
    } else {
        cores().min(rows)
    };
    if threads <= 1 {
        fill(0, values);
        return;
    }
    let band = rows.div_ceil(threads * BANDS_PER_CORE).max(1);
    let bands = values.chunks_mut(band * columns).enumerate().collect();
    queued(bands, threads, |(i, chunk)| fill(i * band, chunk));
}

/// `f` of each item, in order, worked out across the cores.
pub fn each<T: Sync, U: Send>(items: &[T], f: impl Fn(&T) -> U + Sync) -> Vec<U> {
    let threads = cores().min(items.len());
    if threads <= 1 {
        return items.iter().map(f).collect();
    }
    let share = items.len().div_ceil(threads * BANDS_PER_CORE).max(1);
    let mut out: Vec<Vec<U>> = (0..items.len().div_ceil(share)).map(|_| vec![]).collect();
    let shares = items.chunks(share).zip(&mut out).collect();
    queued(shares, threads, |(chunk, slot): (&[T], &mut Vec<U>)| {
        *slot = chunk.iter().map(&f).collect();
    });
    out.into_iter().flatten().collect()
}

/// The processor time all of Cut Craft's threads have used so far, for [`Gauge::tick`]; none
/// where the system will not say.
pub fn process_time() -> Option<Duration> {
    cut_craft_priority::process_time()
}

/// How briskly a calculation is going, as the status bar shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pace {
    /// Getting under [`WALK`] of the processor time its threads asked for: other programs
    /// are taking most of it.
    Crawl,
    /// Getting [`WALK`] to [`RUN`] of it: sharing with other programs.
    Walk,
    /// Getting [`RUN`] or more of it, within the limit set.
    Run,
    /// No limit set: every core, at normal priority.
    Sprint,
}

/// The share of the processor time asked for at and above which a calculation runs.
pub const RUN: f64 = 0.75;
/// The share at and above which it walks; below, it crawls.
pub const WALK: f64 = 0.35;

impl Pace {
    /// The pace of a calculation whose `threads` threads (on average) got `cpu` of processor
    /// time over `wall` of clock time, under `limit`.
    pub fn of(cpu: Duration, wall: Duration, threads: f64, limit: Limit) -> Self {
        if !limit.on {
            return Self::Sprint;
        }
        let asked = wall.as_secs_f64() * threads;
        if asked <= 0.0 {
            return Self::Run;
        }
        let got = cpu.as_secs_f64() / asked;
        if got >= RUN {
            Self::Run
        } else if got >= WALK {
            Self::Walk
        } else {
            Self::Crawl
        }
    }
}

/// Measures a calculation's [`Pace`] once a second while one runs: fed every frame with the
/// time, Cut Craft's processor time so far and the threads [`running`].
#[derive(Debug, Default)]
pub struct Gauge {
    /// When the second being measured began, and the processor time then.
    since: Option<(Instant, Duration)>,
    /// The threads running at each tick of it, added up, and how many ticks.
    threads: (usize, u32),
    pace: Option<Pace>,
}

impl Gauge {
    /// How often the pace is measured.
    pub const EVERY: Duration = Duration::from_secs(1);

    /// One frame's look: `now`, the processor time used so far (`None` where the system will
    /// not say) and the threads `running`. The new pace once a second has passed; none
    /// otherwise, and none (and nothing shown) once nothing runs.
    pub fn tick(
        &mut self,
        now: Instant,
        cpu: Option<Duration>,
        running: usize,
        limit: Limit,
    ) -> Option<Pace> {
        let (Some(cpu), true) = (cpu, running > 0) else {
            *self = Self::default();
            return None;
        };
        let Some((start, cpu_then)) = self.since else {
            self.since = Some((now, cpu));
            self.threads = (running, 1);
            return None;
        };
        self.threads = (self.threads.0 + running, self.threads.1 + 1);
        let wall = now.saturating_duration_since(start);
        if wall < Self::EVERY {
            return None;
        }
        #[expect(clippy::cast_precision_loss, reason = "a few hundred threads at most")]
        let threads = self.threads.0 as f64 / f64::from(self.threads.1);
        let pace = Pace::of(cpu.saturating_sub(cpu_then), wall, threads, limit);
        self.pace = Some(pace);
        self.since = Some((now, cpu));
        self.threads = (running, 1);
        Some(pace)
    }

    /// The pace measured last; none while nothing runs, or for the first second.
    pub fn pace(&self) -> Option<Pace> {
        self.pace
    }
}
