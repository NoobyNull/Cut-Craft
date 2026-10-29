//! A 3Dconnexion SpaceMouse, through spacenavd (Linux and the BSDs): its socket sends each
//! motion and button as eight native-endian ints, the protocol libspnav's first version
//! speaks, so nothing more is needed than the socket.

use crate::options::{SpaceMouse, SpaceMouseMode};
use crate::view3d::{Framing, Orbit, Rotation};
use cut_craft_core::glam::DVec3;
use eframe::egui;
use std::sync::mpsc;

/// The cap at full deflection, as spacenavd counts it.
const FULL: f64 = 350.0;
/// How far the view turns in a second at full deflection, radians.
const TURN_PER_SECOND: f64 = std::f64::consts::PI;
/// How far the view pans in a second at full deflection, points.
const PAN_PER_SECOND: f64 = 800.0;
/// How many times the view zooms in a second at full deflection.
const ZOOM_PER_SECOND: f64 = 4.0;

/// The view after a motion of the cap held for `seconds`, as `settings` and `rotation` say:
/// in Object mode the model follows the cap (pushed in, it goes away), in Camera mode the
/// viewpoint does, so everything is the other way.
pub fn moved(
    (orbit, framing): (Orbit, Framing),
    (translate, rotate): ([i32; 3], [i32; 3]),
    seconds: f64,
    settings: &SpaceMouse,
    rotation: Rotation,
) -> (Orbit, Framing) {
    let sign = |invert: bool| if invert { -1.0 } else { 1.0 };
    let mode = match settings.mode {
        SpaceMouseMode::Object => 1.0,
        SpaceMouseMode::Camera => -1.0,
    };
    let k = seconds * settings.speed * mode / FULL;
    let [x, y, z] = translate.map(f64::from);
    let [rx, ry, rz] = rotate.map(f64::from);
    #[expect(clippy::cast_possible_truncation, reason = "a pan in points")]
    let pan = egui::vec2(
        (x * sign(settings.invert_x) * PAN_PER_SECOND * k) as f32,
        // the screen's up is egui's down
        (-y * sign(settings.invert_y) * PAN_PER_SECOND * k) as f32,
    );
    #[expect(clippy::cast_possible_truncation, reason = "a zoom factor")]
    let zoom = ZOOM_PER_SECOND.powf(-z * sign(settings.invert_zoom) * k) as f32;
    // spacenavd's z goes into the screen; the view's comes out of it
    let turn = DVec3::new(
        rx * sign(settings.invert_tilt),
        ry * sign(settings.invert_spin),
        -rz * sign(settings.invert_roll),
    ) * TURN_PER_SECOND
        * k;
    (
        orbit.turned_by(turn, rotation),
        framing.panned(pan).zoomed_about(egui::Vec2::ZERO, zoom),
    )
}

/// What one of the SpaceMouse's buttons does to the 3D view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ButtonView {
    Fit,
    /// Look from here, fitted.
    Look(Orbit),
}

/// What a SpaceMouse Pro's buttons do in the 3D view, by spacenavd's numbers: Fit and the
/// Top, Right and Front views.
pub fn button_view(number: i32) -> Option<ButtonView> {
    match number {
        1 => Some(ButtonView::Fit),
        2 => Some(ButtonView::Look(Orbit::TOP)),
        4 => Some(ButtonView::Look(Orbit::RIGHT)),
        5 => Some(ButtonView::Look(Orbit::FRONT)),
        _ => None,
    }
}

/// What the SpaceMouse did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    /// The cap's push (x right, y up, z into the screen) and twist about those axes, each
    /// about ±350 at full deflection, held for `period` milliseconds.
    Motion {
        translate: [i32; 3],
        rotate: [i32; 3],
        period: i32,
    },
    Button {
        number: i32,
        pressed: bool,
    },
}

impl Input {
    /// An event as spacenavd sends it: its type (0 motion, 1 press, 2 release), then a
    /// motion's x, y, z, rx, ry, rz and period, or a button's number.
    pub fn parse(event: [i32; 8]) -> Option<Self> {
        match event[0] {
            0 => Some(Self::Motion {
                translate: [event[1], event[2], event[3]],
                rotate: [event[4], event[5], event[6]],
                period: event[7],
            }),
            1 | 2 => Some(Self::Button {
                number: event[1],
                pressed: event[0] == 1,
            }),
            _ => None,
        }
    }
}

/// Listens for the SpaceMouse on a thread of its own, waking `ctx` for each event; while
/// spacenavd is not running it tries again every few seconds.
#[cfg(unix)]
pub fn listen(ctx: egui::Context) -> mpsc::Receiver<Input> {
    use std::io::Read;
    use std::os::unix::net::UnixStream;
    /// Where spacenavd listens.
    const SOCKET: &str = "/var/run/spnav.sock";
    /// How long to wait before looking for spacenavd again.
    const RETRY: std::time::Duration = std::time::Duration::from_secs(5);
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        loop {
            if let Ok(mut socket) = UnixStream::connect(SOCKET) {
                let mut bytes = [0u8; 32];
                while socket.read_exact(&mut bytes).is_ok() {
                    let event = std::array::from_fn(|i| {
                        i32::from_ne_bytes(bytes[i * 4..i * 4 + 4].try_into().expect("4 bytes"))
                    });
                    if let Some(input) = Input::parse(event) {
                        if tx.send(input).is_err() {
                            return; // the app has gone
                        }
                        ctx.request_repaint();
                    }
                }
            }
            std::thread::sleep(RETRY);
        }
    });
    rx
}

/// No spacenavd here: nothing ever arrives.
// ponytail: Windows and macOS need 3Dconnexion's own driver (its navlib); add it when asked
#[cfg(not(unix))]
pub fn listen(_ctx: egui::Context) -> mpsc::Receiver<Input> {
    mpsc::channel().1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spacenavd_events_are_read_as_motions_and_buttons() {
        assert_eq!(
            Input::parse([0, 10, -20, 30, 1, 2, 3, 16]),
            Some(Input::Motion {
                translate: [10, -20, 30],
                rotate: [1, 2, 3],
                period: 16
            })
        );
        assert_eq!(
            Input::parse([1, 5, 0, 0, 0, 0, 0, 0]),
            Some(Input::Button {
                number: 5,
                pressed: true
            })
        );
        assert_eq!(
            Input::parse([2, 5, 0, 0, 0, 0, 0, 0]),
            Some(Input::Button {
                number: 5,
                pressed: false
            })
        );
        assert_eq!(Input::parse([7, 0, 0, 0, 0, 0, 0, 0]), None);
    }

    fn start() -> (Orbit, Framing) {
        (Orbit::FRONT, Framing::default())
    }

    #[test]
    fn the_cap_moves_the_model_and_camera_mode_moves_the_viewpoint() {
        let object = SpaceMouse::default();
        let camera = SpaceMouse {
            mode: SpaceMouseMode::Camera,
            ..SpaceMouse::default()
        };
        let second = 1.0;
        // slid right: the model goes right, or the viewpoint does
        let (_, right) = moved(
            start(),
            ([350, 0, 0], [0; 3]),
            second,
            &object,
            Rotation::Free,
        );
        assert!(right.pan.x > 0.0);
        let (_, left) = moved(
            start(),
            ([350, 0, 0], [0; 3]),
            second,
            &camera,
            Rotation::Free,
        );
        assert!(left.pan.x < 0.0);
        // pushed in: the model goes away
        let (_, away) = moved(
            start(),
            ([0, 0, 350], [0; 3]),
            second,
            &object,
            Rotation::Free,
        );
        assert!(away.zoom < 1.0);
        // twisted about the vertical: the front turns to the right, as a drag right does
        let (spun, _) = moved(start(), ([0; 3], [0, 100, 0]), 0.1, &object, Rotation::Free);
        let front = spun.project(DVec3::NEG_Y).0;
        assert!(front.x > 0.0, "{front:?}");
        // the turntable leaves roll out; free rotation takes it
        let rolled = |rotation| moved(start(), ([0; 3], [0, 0, 200]), 0.2, &object, rotation).0;
        assert_eq!(rolled(Rotation::Turntable), Orbit::FRONT);
        assert!(rolled(Rotation::Free).roll.abs() > 0.01);
        // inverted, and faster
        let inverted = SpaceMouse {
            invert_x: true,
            speed: 2.0,
            ..SpaceMouse::default()
        };
        let (_, back) = moved(
            start(),
            ([350, 0, 0], [0; 3]),
            second,
            &inverted,
            Rotation::Free,
        );
        assert!((back.pan.x + 2.0 * right.pan.x).abs() < 1e-3);
    }
}
