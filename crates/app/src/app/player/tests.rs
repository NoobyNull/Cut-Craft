use super::*;
use cut_craft_core::document::Job;
use cut_craft_core::glam::DVec3;
use cut_craft_core::toolpath::{Move, MoveKind, Tool, Toolpath};

/// A preview of an 80 mm cut across a 100 × 100 job at a millimetre a second, rapids too, so
/// seconds along read as millimetres: 87 + 7 + 80 + 7 long.
fn preview() -> Playback {
    let job = Job {
        width: 100.0,
        height: 100.0,
        ..Job::default()
    };
    let m = |kind, x: f64, z: f64| Move {
        kind,
        to: DVec3::new(job.area().min.x + x, job.area().min.y + 50.0, z),
    };
    let toolpath = Toolpath {
        name: "line".into(),
        strategy: String::new(),
        params: serde_json::Value::Null,
        tool: Tool {
            feed_rate: 60.0,
            plunge_rate: 60.0,
            ..Tool::default()
        },
        vectors: vec![],
        moves: vec![
            m(MoveKind::Rapid, 10.0, 5.0),
            m(MoveKind::Plunge, 10.0, -2.0),
            m(MoveKind::Cut, 90.0, -2.0),
            m(MoveKind::Rapid, 90.0, 5.0),
        ],
        visible: true,
        power: vec![],
        stale: false,
        group: None,
        drill_bank: None,
        fallbacks: vec![],
        across: vec![],
    };
    Playback::new(Material::new(&job, 100), &[&toolpath], 60.0)
}

fn playing() -> Player {
    let mut player = Player {
        budget: Duration::from_secs(5),
        ..Player::default()
    };
    player.start(preview(), 3);
    player
}

#[test]
fn a_started_preview_plays_from_the_start_at_the_jobs_revision() {
    let player = playing();
    assert!(player.playing);
    assert_eq!(player.current().map(Playback::at), Some(0.0));
    assert!(!player.stale(3));
    assert!(player.stale(4), "the job changed since it was laid out");
}

#[test]
fn a_frame_moves_the_tool_on_at_the_players_speed() {
    let mut player = Player {
        speed: 10.0,
        ..playing()
    };
    let frame = player.advance(0.05, Instant::now());
    assert!(frame.repaint);
    let at = player.current().unwrap().at();
    assert!(
        (at - 0.5).abs() < 1e-9,
        "10 × the machine's speed for 0.05 s: {at}"
    );
}

#[test]
fn a_slow_frame_does_not_send_the_tool_further() {
    let mut player = Player {
        speed: 1.0,
        ..playing()
    };
    player.advance(3.0, Instant::now());
    let at = player.current().unwrap().at();
    assert!(
        (at - 0.1).abs() < 1e-9,
        "a frame counts as a tenth of a second at most: {at}"
    );
}

#[test]
fn a_paused_preview_goes_to_where_it_was_sent_and_stops_there() {
    let mut player = playing();
    (player.playing, player.goal) = (false, Some(20.0));
    player.advance(0.01, Instant::now());
    assert_eq!(player.goal, None, "it got there");
    assert!((player.current().unwrap().at() - 20.0).abs() < 1e-9);
    assert!(
        !player.advance(0.01, Instant::now()).repaint,
        "paused: nothing moves"
    );
}

#[test]
fn a_preview_played_to_its_end_says_so() {
    let mut player = Player {
        speed: 1e6,
        ..playing()
    };
    assert!(player.advance(0.1, Instant::now()).finished);
}

#[test]
fn a_finished_preview_is_kept_with_what_it_cut() {
    let mut player = playing();
    player.goal = Some(50.0);
    player.advance(0.01, Instant::now());
    let cut = player.finish().expect("it was playing");
    assert!(!player.playing && player.playback.is_none());
    assert_eq!(
        player.current().map(Playback::at),
        Some(50.0),
        "kept to scrub"
    );
    assert_eq!(&cut, player.kept.as_ref().unwrap().material());
    assert!(player.finish().is_none(), "nothing playing any more");
    assert!(player.stale(4), "a kept preview goes stale too");
}

#[test]
fn the_view_shows_the_cut_again_once_it_is_due_or_settled() {
    let mut player = Player {
        redraw: Duration::from_secs(60),
        speed: 1.0,
        ..playing()
    };
    let now = Instant::now();
    player.advance(0.05, now);
    let first = player.shown.expect("the first frame is shown");
    let frame = player.advance(0.05, now);
    assert!(frame.repaint_after.is_some(), "playing: not yet, but later");
    assert_eq!(player.shown, Some(first));
    player.playing = false;
    player.advance(0.05, now);
    assert_ne!(player.shown, Some(first), "stopped: shown at once");
}

#[test]
fn nothing_playing_drops_the_goal() {
    let mut player = Player {
        goal: Some(5.0),
        ..Player::default()
    };
    assert_eq!(player.advance(0.05, Instant::now()), Frame::default());
    assert_eq!(player.goal, None);
}

#[test]
fn a_jump_gets_more_time_each_frame_than_playing_does() {
    assert!(jump_budget(PLAYBACK_BUDGET) > PLAYBACK_BUDGET);
    assert_eq!(jump_budget(Duration::ZERO), Duration::ZERO);
}

#[test]
fn a_kept_preview_can_be_taken_up_again_until_it_is_closed() {
    let mut player = Player {
        speed: 3.0,
        ..playing()
    };
    player.finish();
    assert!(player.resume(), "the kept one plays again");
    assert!(player.playback.is_some() && player.kept.is_none());
    assert!(!player.resume(), "nothing kept now");
    player.close();
    assert!(player.current().is_none() && !player.playing);
    assert!(
        (player.speed - 3.0).abs() < 1e-9,
        "how it plays stays for the next"
    );
}
