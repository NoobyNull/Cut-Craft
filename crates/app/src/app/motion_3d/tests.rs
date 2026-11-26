use super::{Frame, Motion3d};
use crate::options::Detail;

fn frame(pass: u64, now: f64, seconds: f64) -> Frame {
    Frame {
        pass,
        now,
        seconds,
        busy: false,
        settle: 0.5,
        still: Detail::VeryHigh,
        fps: 60.0,
    }
}

#[test]
fn the_view_moves_for_the_settle_time_after_it_last_moved() {
    let mut motion = Motion3d::default();
    motion.moved(10.0);
    motion.judge(&frame(1, 10.2, 0.016));
    assert_eq!(motion.detail(), (Detail::VeryHigh, true));
    motion.judge(&frame(2, 10.6, 0.016));
    assert_eq!(motion.detail(), (Detail::VeryHigh, false));
    assert!((motion.since_moved(10.6) - 0.6).abs() < 1e-9);
}

#[test]
fn something_else_moving_counts_as_moving() {
    let mut motion = Motion3d::default();
    let busy = Frame {
        busy: true,
        ..frame(1, 100.0, 0.016)
    };
    motion.judge(&busy);
    assert!(motion.detail().1);
}

#[test]
fn each_pass_is_judged_once() {
    let mut motion = Motion3d::default();
    motion.moved(10.0);
    motion.judge(&frame(1, 10.1, 0.016));
    // the same pass again, though the view has settled by now
    motion.judge(&frame(1, 20.0, 0.016));
    assert!(motion.detail().1, "the first judgement stands");
}

#[test]
fn slow_frames_drop_the_detail_and_ask_to_draw_finely_once_settled() {
    let mut motion = Motion3d::default();
    let mut asked = None;
    for pass in 0..40 {
        let now = 10.0 + f64::from(u32::try_from(pass).unwrap()) * 0.1;
        motion.moved(now);
        asked = motion.judge(&frame(pass, now, 0.1)).or(asked);
    }
    let (level, moving) = motion.detail();
    assert!(moving && level < Detail::VeryHigh, "{level:?}");
    let wait = asked.expect("a fine frame is asked for");
    assert!(wait > 0.5 && wait < 0.6, "{wait}");
    assert!(motion.fps().is_some_and(|fps| (fps - 10.0).abs() < 1e-6));
}

#[test]
fn still_shows_no_frame_rate() {
    let mut motion = Motion3d::default();
    motion.judge(&frame(1, 100.0, 0.016));
    assert_eq!(motion.fps(), None);
}

#[test]
fn auto_rotate_turns_by_the_time_since_it_last_did() {
    let mut motion = Motion3d::default();
    assert_eq!(motion.spun(5.0), None, "not spinning");
    assert!(!motion.spinning(), "asking does not start it");
    motion.set_spin(Some(5.0));
    assert!(motion.spinning());
    assert_eq!(motion.spun(5.5), Some(0.5));
    assert_eq!(motion.spin(), Some(5.5));
    motion.stop_spin();
    assert!(!motion.spinning());
}
