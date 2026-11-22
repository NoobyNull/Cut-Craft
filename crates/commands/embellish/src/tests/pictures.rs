use super::*;
use std::f64::consts::{PI, TAU};

fn shade(part: &Component, path: &std::path::Path) {
    let (w, h) = (part.columns, part.rows);
    let at = |i: usize, j: usize| {
        let v = part.heights[j.min(h - 1) * w + i.min(w - 1)];
        if v.is_nan() { 1.0 } else { v }
    };
    let (el, az) = (24_f64.to_radians(), (-135_f64).to_radians() + PI / 2.0);
    let l = [el.cos() * az.cos(), el.cos() * az.sin(), el.sin()];
    let mut img = image::RgbImage::new(w as u32, h as u32);
    for j in 0..h {
        for i in 0..w {
            let dx = (at(i + 1, j) - at(i.saturating_sub(1), j)) / (2.0 * part.cell);
            let dy = (at(i, j + 1) - at(i, j.saturating_sub(1))) / (2.0 * part.cell);
            let n = [-dx, -dy, 1.0];
            let len = (n[0] * n[0] + n[1] * n[1] + 1.0).sqrt();
            let lam = ((n[0] * l[0] + n[1] * l[1] + n[2] * l[2]) / len).max(0.0);
            let k = 0.34 + 1.1 * lam;
            let px = image::Rgb([
                (232.0 * k).min(255.0) as u8,
                (190.0 * k).min(255.0) as u8,
                (138.0 * k).min(255.0) as u8,
            ]);
            img.put_pixel(i as u32, (h - 1 - j) as u32, px);
        }
    }
    img.save(path).unwrap();
}

fn polygon(n: u32, r: f64, catenary: bool) -> Path {
    let c = Point::new(150.0, 150.0);
    let rot = if n.is_multiple_of(2) {
        PI / f64::from(n)
    } else {
        0.0
    } + PI / 2.0;
    let mut pts = vec![];
    for k in 0..n {
        let a0 = rot + TAU * f64::from(k) / f64::from(n);
        let (p0, p1) = (
            c + Point::from_angle(a0) * r,
            c + Point::from_angle(a0 + TAU / f64::from(n)) * r,
        );
        for s in 0..40 {
            let t = f64::from(s) / 40.0;
            let q = p0.lerp(p1, t);
            let sag = if catenary {
                let half = p0.distance(p1) / 2.0;
                let x = (t - 0.5) * 2.0 * half;
                let cc = half * 0.55;
                let full = (half * 0.3).min(r * (PI / f64::from(n)).cos() * 0.26);
                full * (1.0
                    - (((x / cc).cosh() - 1.0) / ((half / cc).cosh() - 1.0)).clamp(0.0, 1.0))
            } else {
                0.0
            };
            pts.push(q + (c - q).normalize() * sag);
        }
    }
    Path::polyline(&pts, true)
}

/// Every style along an open S-curve, as 3D carvings: `EMBELLISH_OUT=dir cargo nextest
/// run -p cmd-embellish line_pictures --run-ignored only`.
#[test]
#[ignore = "writes pictures to look at"]
fn line_pictures() {
    let out = std::path::PathBuf::from(std::env::var("EMBELLISH_OUT").unwrap());
    let pts: Vec<Point> = (0..=150)
        .map(|i| {
            let x = f64::from(i) * 2.0;
            Point::new(20.0 + x, 100.0 + 40.0 * (x / 300.0 * TAU).sin())
        })
        .collect();
    let line = Path::polyline(&pts, false);
    let doc = cut_craft_core::testing::document_with(vec![line]);
    for (key, style) in [
        ("folk_mix", Style::FolkMix),
        ("scrolls", Style::Scrolls),
        ("flowers", Style::Flowers),
        ("band_border", Style::BandBorder),
        ("rope", Style::Rope),
        ("chain", Style::Chain),
        ("twist", Style::Twist),
        ("braid", Style::Braid),
        ("bead_and_reel", Style::BeadAndReel),
        ("laurel", Style::Laurel),
        ("feathers", Style::Feathers),
        ("chevrons", Style::Chevrons),
    ] {
        let p = Params {
            style,
            make: Make::Carving,
            border_width: 30.0,
            ..Params::default()
        };
        let mut after = doc.clone();
        after.apply(&Embellish::run(&doc, &p).unwrap()).unwrap();
        let part = after.components.values().next().unwrap();
        shade(part, &out.join(format!("line-{key}.png")));
        let shape = cut_craft_core::testing::square(0.0, 0.0, 160.0);
        let boxed = cut_craft_core::testing::document_with(vec![shape]);
        let mut after = boxed.clone();
        after.apply(&Embellish::run(&boxed, &p).unwrap()).unwrap();
        let part = after.components.values().next().unwrap();
        shade(part, &out.join(format!("square-{key}.png")));
    }
}

#[test]
#[ignore = "writes pictures to look at"]
fn sequence_pictures() {
    let out = std::path::PathBuf::from(std::env::var("EMBELLISH_OUT").unwrap());
    let pts: Vec<Point> = (0..=100)
        .map(|i| Point::new(10.0 + f64::from(i) * 3.0, 50.0))
        .collect();
    let doc = cut_craft_core::testing::document_with(vec![Path::polyline(&pts, false)]);
    for (key, style, sequence) in [
        ("chevrons-111000222", Style::Chevrons, "111000222"),
        ("chevrons-turns", Style::Chevrons, "1,1/4,R1/4,2,2/4,R2/4"),
        ("feathers-1234", Style::Feathers, "1234"),
        ("folk-12", Style::FolkMix, "12"),
        ("folk-1", Style::FolkMix, ""),
        ("folk-1120", Style::FolkMix, "1120"),
        ("laurel-st", Style::Laurel, "ST(1,2),1,0"),
    ] {
        let p = Params {
            style,
            make: Make::Carving,
            border_width: 30.0,
            sequence: sequence.into(),
            ..Params::default()
        };
        let mut after = doc.clone();
        after.apply(&Embellish::run(&doc, &p).unwrap()).unwrap();
        let part = after.components.values().next().unwrap();
        shade(part, &out.join(format!("sequence-{key}.png")));
    }
}

#[test]
#[ignore = "writes pictures to look at"]
fn pictures() {
    let out = std::path::PathBuf::from(std::env::var("EMBELLISH_OUT").unwrap());
    let mut shapes: Vec<(String, Path)> = vec![];
    for (n, name) in [
        (3, "triangle"),
        (4, "square"),
        (6, "hexagon"),
        (8, "octagon"),
    ] {
        shapes.push((name.into(), polygon(n, 120.0, false)));
        shapes.push((format!("{name}-catenary"), polygon(n, 120.0, true)));
    }
    if let Ok(f) = std::env::var("EMBELLISH_SHAPE") {
        let pts: Vec<[f64; 2]> =
            serde_json::from_str(&std::fs::read_to_string(f).unwrap()).unwrap();
        let pts: Vec<Point> = pts.iter().map(|p| Point::new(p[0], p[1])).collect();
        shapes.push(("your-shape".into(), Path::polyline(&pts, true)));
    }
    let variants: Vec<(&str, Params)> = vec![
        ("", Params::default()),
        (
            "-low",
            Params {
                refinement: 1,
                objects: 1,
                scroll_size: 60,
                ..Params::default()
            },
        ),
        (
            "-high",
            Params {
                refinement: 10,
                objects: 10,
                scroll_size: 150,
                tightness: 10,
                ..Params::default()
            },
        ),
        (
            "-border",
            Params {
                style: Style::BandBorder,
                ..Params::default()
            },
        ),
        (
            "-rope",
            Params {
                style: Style::Rope,
                ..Params::default()
            },
        ),
        (
            "-chain",
            Params {
                style: Style::Chain,
                ..Params::default()
            },
        ),
        (
            "-twist",
            Params {
                style: Style::Twist,
                ..Params::default()
            },
        ),
        (
            "-braid",
            Params {
                style: Style::Braid,
                rails: true,
                ..Params::default()
            },
        ),
        (
            "-bead",
            Params {
                style: Style::BeadAndReel,
                ..Params::default()
            },
        ),
        (
            "-chain-fill",
            Params {
                style: Style::Chain,
                ..Params::default()
            },
        ),
        (
            "-rope-fill",
            Params {
                style: Style::Rope,
                ..Params::default()
            },
        ),
        (
            "-braid-fill",
            Params {
                style: Style::Braid,
                ..Params::default()
            },
        ),
        (
            "-laurel",
            Params {
                style: Style::Laurel,
                ..Params::default()
            },
        ),
        (
            "-feathers",
            Params {
                style: Style::Feathers,
                ..Params::default()
            },
        ),
        (
            "-chevrons",
            Params {
                style: Style::Chevrons,
                ..Params::default()
            },
        ),
        (
            "-laurel-fill",
            Params {
                style: Style::Laurel,
                ..Params::default()
            },
        ),
        (
            "-feathers-fill",
            Params {
                style: Style::Feathers,
                ..Params::default()
            },
        ),
        (
            "-chevrons-fill",
            Params {
                style: Style::Chevrons,
                ..Params::default()
            },
        ),
        (
            "-scrolls",
            Params {
                style: Style::Scrolls,
                ..Params::default()
            },
        ),
        (
            "-flowers",
            Params {
                style: Style::Flowers,
                ..Params::default()
            },
        ),
        (
            "-sunk",
            Params {
                direction: Direction::Sunk,
                ..Params::default()
            },
        ),
        (
            "-mixed",
            Params {
                direction: Direction::Mixed,
                ..Params::default()
            },
        ),
        (
            "-mixed-rope",
            Params {
                direction: Direction::Mixed,
                style: Style::Rope,
                ..Params::default()
            },
        ),
    ];
    let only = std::env::var("EMBELLISH_ONLY").unwrap_or_default();
    let only_variant = std::env::var("EMBELLISH_VARIANT").ok();
    let mut shapes: Vec<(String, Vec<Path>)> =
        shapes.into_iter().map(|(n, p)| (n, vec![p])).collect();
    // a ring: a round shape with a round hole
    let circle = |r: f64| {
        let pts: Vec<Point> = (0..240)
            .map(|k| Point::new(150.0, 150.0) + Point::from_angle(TAU * f64::from(k) / 240.0) * r)
            .collect();
        Path::polyline(&pts, true)
    };
    shapes.push(("ring".into(), vec![circle(120.0), circle(35.0)]));
    for (name, shape) in &shapes {
        if !only.is_empty() && !name.contains(&only) {
            continue;
        }
        for (suffix, p) in &variants {
            if only_variant
                .as_ref()
                .is_some_and(|v| !v.split(',').any(|v| v == *suffix))
            {
                continue;
            }
            let doc = cut_craft_core::testing::document_with(shape.clone());
            let t = std::time::Instant::now();
            let change = Embellish::run(&doc, p).unwrap();
            let ms = t.elapsed().as_millis();
            let mut after = doc.clone();
            after.apply(&change).unwrap();
            eprintln!("{name}{suffix}: {} vectors, {ms} ms", after.vectors.len());
            shade(
                after.components.values().next().unwrap(),
                &out.join(format!("{name}{suffix}.png")),
            );
        }
    }
}
