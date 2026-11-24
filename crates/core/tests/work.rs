//! The one way heavy calculations get their threads: how many, at what priority, and how
//! briskly the calculation is running.

use cut_craft_core::work::{self, Gauge, Limit, Pace};
use std::time::{Duration, Instant};

#[test]
fn the_limit_takes_its_share_of_the_cores_rounded_down_never_none() {
    let at = |share| Limit { on: true, share };
    assert_eq!(at(80).cores_of(32), 25, "80% of 32 is 25.6");
    assert_eq!(at(80).cores_of(4), 3, "80% of 4 is 3.2");
    assert_eq!(at(80).cores_of(2), 1, "a 2-core laptop keeps one free");
    assert_eq!(at(80).cores_of(1), 1, "never no cores at all");
    assert_eq!(at(10).cores_of(4), 1);
    assert_eq!(at(100).cores_of(8), 8);
    assert_eq!(
        at(0).cores_of(8),
        1,
        "below the slider's range still calculates"
    );
    assert_eq!(at(250).cores_of(8), 8, "never more than the computer has");
    let off = Limit {
        on: false,
        share: 30,
    };
    assert_eq!(off.cores_of(32), 32, "no limit: every core");
}

#[test]
fn the_default_limit_is_on_at_80_percent() {
    assert_eq!(
        Limit::default(),
        Limit {
            on: true,
            share: 80
        }
    );
}

#[test]
fn the_old_cores_setting_becomes_a_share() {
    assert_eq!(
        Limit::from_old_cores(0, 32),
        Limit::default(),
        "0 (all) is the new default"
    );
    assert_eq!(
        Limit::from_old_cores(8, 32),
        Limit {
            on: true,
            share: 25
        }
    );
    assert_eq!(
        Limit::from_old_cores(3, 4),
        Limit {
            on: true,
            share: 75
        }
    );
    assert_eq!(
        Limit::from_old_cores(64, 32),
        Limit {
            on: true,
            share: 100
        },
        "more than the computer has is all of it"
    );
    // the share it became gives back the cores asked for
    for (cores, all) in [(1, 32), (8, 32), (25, 32), (3, 4), (5, 12), (7, 24)] {
        assert_eq!(
            Limit::from_old_cores(cores, all).cores_of(all),
            cores,
            "{cores} of {all}"
        );
    }
}

#[test]
fn parallel_answers_in_order_on_no_more_threads_than_allowed() {
    let made = work::parallel(1000, |i| (i * 3, std::thread::current().id()));
    assert_eq!(
        made.iter().map(|m| m.0).collect::<Vec<_>>(),
        (0..1000).map(|i| i * 3).collect::<Vec<_>>()
    );
    let mut threads: Vec<_> = made.iter().map(|m| m.1).collect();
    threads.sort_by_key(|t| format!("{t:?}"));
    threads.dedup();
    assert!(
        threads.len() <= work::cores(),
        "{} threads for {} cores",
        threads.len(),
        work::cores()
    );
}

#[test]
fn bands_of_rows_and_shares_of_items_cover_everything_once_in_order() {
    // a grid 7 wide and big enough to be split across the cores
    let mut cells = vec![0_usize; 7 * 900];
    work::by_rows(&mut cells, 7, |first, band| {
        for (i, cell) in band.iter_mut().enumerate() {
            *cell = (first * 7 + i) * 3;
        }
    });
    assert!(cells.iter().enumerate().all(|(i, &c)| c == i * 3));
    let items: Vec<usize> = (0..1000).collect();
    assert_eq!(
        work::each(&items, |&i| i * 2),
        items.iter().map(|i| i * 2).collect::<Vec<_>>()
    );
    // a small grid, one band on this thread, and nothing at all
    let mut few = vec![0_usize; 10];
    work::by_rows(&mut few, 5, |first, band| {
        for (i, cell) in band.iter_mut().enumerate() {
            *cell = first * 5 + i;
        }
    });
    assert_eq!(few, (0..10).collect::<Vec<_>>());
    assert_eq!(
        work::each(&Vec::<usize>::new(), |&i| i),
        Vec::<usize>::new()
    );
}

#[test]
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
fn calculation_threads_give_way_to_other_programs_and_the_caller_does_not() {
    let caller = cut_craft_priority::this_thread_is_lowered();
    let workers = std::thread::scope(|s| {
        let handles: Vec<_> = (0..3)
            .map(|_| work::spawn(s, cut_craft_priority::this_thread_is_lowered))
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(workers, vec![true; 3], "every worker lowered");
    let job = work::background("a test job", cut_craft_priority::this_thread_is_lowered)
        .unwrap()
        .join()
        .unwrap();
    assert!(job, "a background job's own thread is lowered too");
    assert_eq!(
        cut_craft_priority::this_thread_is_lowered(),
        caller,
        "the caller is left alone"
    );
}

#[test]
fn a_calculation_thread_counts_as_running_until_it_ends() {
    let inside = std::thread::scope(|s| work::spawn(s, work::running).join().unwrap());
    assert!(inside >= 1, "a worker sees itself running");
    let job = work::background("a test job", || {
        // waiting on its own workers, a job thread is not busy: only they are
        let waiting = std::thread::scope(|s| {
            let worker = work::spawn(s, work::running);
            std::thread::sleep(Duration::from_millis(5));
            worker.join().unwrap()
        });
        (work::running(), waiting)
    });
    let (alone, waiting) = job.unwrap().join().unwrap();
    assert!(alone >= 1 && waiting >= 1);
}

#[test]
fn the_pace_is_how_much_of_what_it_asked_for_the_calculation_got() {
    let second = Duration::from_secs(1);
    let ms = Duration::from_millis;
    let on = Limit::default();
    // 4 threads busy for a second each would be 4 s of processor time
    assert_eq!(
        Pace::of(ms(4000), second, 4.0, on),
        Pace::Run,
        "all it asked for"
    );
    assert_eq!(Pace::of(ms(3000), second, 4.0, on), Pace::Run, "75%");
    assert_eq!(
        Pace::of(ms(2000), second, 4.0, on),
        Pace::Walk,
        "sharing with other programs"
    );
    assert_eq!(
        Pace::of(ms(1000), second, 4.0, on),
        Pace::Crawl,
        "other programs come first"
    );
    assert_eq!(
        Pace::of(ms(5000), second, 4.0, on),
        Pace::Run,
        "the window's own time on top"
    );
    let off = Limit { on: false, ..on };
    assert_eq!(
        Pace::of(ms(1000), second, 4.0, off),
        Pace::Sprint,
        "no limit set"
    );
    assert_eq!(
        Pace::of(ms(0), Duration::ZERO, 0.0, on),
        Pace::Run,
        "nothing measured yet"
    );
}

#[test]
fn the_gauge_says_the_pace_once_a_second_while_calculating() {
    let start = Instant::now();
    let at = |ms| start + Duration::from_millis(ms);
    let cpu = Duration::from_millis;
    let on = Limit::default();
    let mut gauge = Gauge::default();
    assert_eq!(
        gauge.tick(at(0), Some(cpu(0)), 2, on),
        None,
        "nothing running yet to judge"
    );
    assert_eq!(
        gauge.tick(at(500), Some(cpu(1000)), 2, on),
        None,
        "under a second"
    );
    assert_eq!(gauge.pace(), None);
    // 2 threads for a second got 0.6 s: 30%
    assert_eq!(
        gauge.tick(at(1000), Some(cpu(600)), 2, on),
        Some(Pace::Crawl)
    );
    assert_eq!(
        gauge.pace(),
        Some(Pace::Crawl),
        "kept until the next second"
    );
    assert_eq!(
        gauge.tick(at(2000), Some(cpu(2600)), 2, on),
        Some(Pace::Run)
    );
    assert_eq!(
        gauge.tick(at(2100), None, 0, on),
        None,
        "the calculation ended"
    );
    assert_eq!(gauge.pace(), None, "nothing shown once nothing runs");
}

/// Code allowed to start its own threads, and why: each waits on the network, a disk or a
/// device rather than working the processor, so it neither needs a core limit nor counts as
/// a calculation.
const OWN_THREADS: [(&str, &str); 10] = [
    (
        "core/src/work.rs",
        "the one place calculations' threads start",
    ),
    ("app/src/addons.rs", "downloads add-ons"),
    ("app/src/update.rs", "checks for and downloads updates"),
    (
        "app/src/depth.rs",
        "downloads the depth add-on's detail (its estimate is a calculation)",
    ),
    ("app/src/spacemouse.rs", "waits on the SpaceMouse"),
    (
        "app/src/app/job_files.rs",
        "writes the recovery copy to disk",
    ),
    ("app/src/clipart.rs", "reads the Library's folders"),
    (
        "app/src/clipart/properties.rs",
        "reads one Library file's details",
    ),
    (
        "bug-report/src",
        "sends reports and reads what computer this is",
    ),
    ("web-api/src/lib.rs", "answers the Web API"),
];

/// Every thread a calculation uses starts in [`work`], so every calculation keeps to the limit
/// and gives way to other programs: code anywhere else that starts threads, or counts the
/// cores, fails here unless it is listed above with its reason. Tests may start their own.
#[test]
fn calculations_start_their_threads_in_one_place() {
    const STARTS: [&str; 4] = [
        "thread::spawn(",
        "thread::scope(",
        "thread::Builder::new(",
        "available_parallelism(",
    ];
    let crates = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .unwrap();
    let mut found = vec![];
    let mut files = vec![crates.clone()];
    while let Some(path) = files.pop() {
        if path.is_dir() {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if name == "tests" || name == "target" || name.starts_with('.') {
                continue;
            }
            files.extend(
                std::fs::read_dir(&path)
                    .unwrap()
                    .flatten()
                    .map(|e| e.path()),
            );
            continue;
        }
        let name = path.to_string_lossy().replace('\\', "/");
        let rust = path.extension().is_some_and(|e| e == "rs");
        if !rust || name.ends_with("tests.rs") || !name.contains("/src/") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        // a file's own tests, at its foot, may start threads as they like
        let code = text
            .split("#[cfg(test)]\nmod tests")
            .next()
            .unwrap_or_default();
        let code = code
            .split("#[cfg(test)]\npub(crate) mod tests")
            .next()
            .unwrap_or_default();
        let at = name
            .split("/crates/")
            .last()
            .unwrap_or(&name)
            .trim_start_matches("../");
        if STARTS.iter().any(|s| code.contains(s))
            && !OWN_THREADS
                .iter()
                .any(|(allowed, _)| at.starts_with(allowed))
        {
            found.push(at.to_owned());
        }
    }
    assert!(
        found.is_empty(),
        "start calculation threads with cut_craft_core::work (spawn, background, parallel, \
         by_rows, each), or list the file in OWN_THREADS with why it may not: {found:?}"
    );
}

#[test]
fn a_thread_only_waiting_is_not_counted_as_running() {
    let (during, after) = work::background("a kept worker", || {
        let mine = work::running();
        let during = work::waiting(work::running);
        (mine - during, work::running() + 1 - mine)
    })
    .unwrap()
    .join()
    .unwrap();
    assert_eq!(
        (during, after),
        (1, 1),
        "uncounted while waiting, counted again after"
    );
}
