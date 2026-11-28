//! Border tiles: a generated length of moulding, kept in the Library as three files with
//! one name. `Name.png` is its depth map (16-bit greys: black the lowest, white the
//! highest), `Name.jpg` its picture (the Library tile's preview), and `Name.border.json` its
//! recipe: what made it and its size in millimetres, so it comes into a job at that size,
//! the way a 3D model from the Library does. A tile repeats left to right, so its two ends
//! are made to meet.

#![expect(
    clippy::many_single_char_names,
    reason = "a picture's width and height are w and h, its pixels x and y"
)]
use cut_craft_core::glam::DVec3;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The Library folder generated borders are saved in, under the person's PowerWhittle folder.
pub const FOLDER: [&str; 2] = ["Generated", "Borders"];

/// What a recipe file's `format` always says.
const FORMAT_NAME: &str = "cut-craft-border-tile";
const VERSION: u32 = 1;
/// The recipe's name ends with this, after the tile's name.
pub const RECIPE_SUFFIX: &str = ".border.json";
/// The most points a tile's surface has along its longer side when it is made into a 3D
/// model: finer than a carving needs, coarse enough to stay quick.
const MOST_POINTS: usize = 600;

/// What made a tile, and its size.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Recipe {
    /// Always "cut-craft-border-tile": says what the file is.
    pub format: String,
    pub version: u32,
    /// The style picked (its id, as `classical_baroque`).
    pub style: String,
    /// The extra details typed.
    #[serde(default)]
    pub details: String,
    /// The tile's shape, width to height, as "16:9".
    pub ratio: String,
    /// The tile's size, left to right (one repeat), in mm.
    pub width: f64,
    /// Top to bottom, in mm.
    pub height: f64,
    /// How far its highest point stands above its lowest, in mm.
    pub depth: f64,
    /// How it was shaped after it came back (see [`Shaping`]).
    pub shaping: Shaping,
    /// Where its depth came from: `google` (asked of the picture) or `depth_add_on`.
    pub depth_from: String,
    /// When it was made, as the computer's clock said (RFC 3339).
    #[serde(default)]
    pub made: String,
}

impl Recipe {
    /// A recipe with the format and version filled in.
    pub fn new(style: &str, ratio: &str, width: f64, height: f64, depth: f64) -> Self {
        Self {
            format: FORMAT_NAME.into(),
            version: VERSION,
            style: style.into(),
            details: String::new(),
            ratio: ratio.into(),
            width,
            height,
            depth,
            shaping: Shaping::default(),
            depth_from: "google".into(),
            made: String::new(),
        }
    }
}

/// How a depth map is shaped after it comes back: its depth curve, how much it is smoothed
/// (so the 256 greys of a picture don't carve as stairs) and how far in from each end the
/// two ends are blended to meet.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Shaping {
    /// Below 1 the middle greys rise (a fuller relief); above 1 they fall (crisp tops). 1
    /// leaves it as it is.
    pub curve: f64,
    /// How far it is smoothed, as a share of the tile's height (0: not at all).
    pub smooth: f64,
    /// How far in from each end the ends are blended, as a share of the tile's width (0:
    /// not at all).
    pub seam: f64,
}

impl Default for Shaping {
    fn default() -> Self {
        Self {
            curve: 1.0,
            smooth: 0.004,
            seam: 0.06,
        }
    }
}

/// A tile's three files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Files {
    pub depth: PathBuf,
    pub picture: PathBuf,
    pub recipe: PathBuf,
}

/// The files of the tile `name` in `folder`.
pub fn files(folder: &Path, name: &str) -> Files {
    Files {
        depth: folder.join(format!("{name}.png")),
        picture: folder.join(format!("{name}.jpg")),
        recipe: folder.join(format!("{name}{RECIPE_SUFFIX}")),
    }
}

/// The tile whose depth map is `depth`, when its recipe is beside it.
pub fn of_depth_map(depth: &Path) -> Option<Files> {
    let ext = depth.extension()?.to_str()?;
    if !ext.eq_ignore_ascii_case("png") {
        return None;
    }
    let name = depth.file_stem()?.to_str()?;
    let f = files(depth.parent()?, name);
    f.recipe.is_file().then_some(f)
}

/// Whether `file` is part of a tile but not its depth map: its picture or its recipe, which
/// the Library leaves out (the depth map stands for the tile).
pub fn is_companion(file: &Path) -> bool {
    let Some(name) = file.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    if name.to_ascii_lowercase().ends_with(RECIPE_SUFFIX) {
        return true;
    }
    let is_jpg = file
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("jpg"));
    let Some(stem) = file.file_stem().and_then(|s| s.to_str()) else {
        return false;
    };
    is_jpg
        && file
            .parent()
            .is_some_and(|dir| dir.join(format!("{stem}{RECIPE_SUFFIX}")).is_file())
}

/// A name for a new tile in `folder` that no tile there has: `base`, or `base 2`, `base 3`...
/// Characters a file name can't hold on some computer are left out.
pub fn free_name(folder: &Path, base: &str) -> String {
    let clean: String = base
        .chars()
        .filter(|c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
        .filter(|c| !c.is_control())
        .collect();
    let clean = clean.trim().trim_end_matches('.');
    let clean = if clean.is_empty() { "Border" } else { clean };
    let taken = |n: &str| {
        let f = files(folder, n);
        f.depth.exists() || f.picture.exists() || f.recipe.exists()
    };
    if !taken(clean) {
        return clean.to_owned();
    }
    let mut k = 2_u64;
    loop {
        let name = format!("{clean} {k}");
        if !taken(&name) {
            return name;
        }
        k += 1;
    }
}

/// The depth map `depth` (`w` by `h`, rows from the top, 0 lowest to 1 highest) shaped as
/// `s` says: smoothed (round, as the tile repeats left to right), its depth curve applied,
/// then its two ends blended to meet, and stretched to fill 0 to 1.
pub fn shape(depth: &[f32], w: usize, h: usize, s: &Shaping) -> Vec<f32> {
    assert_eq!(depth.len(), w * h, "a depth map {w} by {h}");
    let mut d = depth.to_vec();
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "a blur radius of a picture's pixels"
    )]
    let radius = (s.smooth.max(0.0) * h as f64).round() as usize;
    if radius > 0 {
        // three box blurs make a soft, round one
        for _ in 0..3 {
            d = box_blur(&d, w, h, radius);
        }
    }
    let curve = s.curve.clamp(0.2, 5.0);
    if (curve - 1.0).abs() > 1e-9 {
        #[expect(clippy::cast_possible_truncation, reason = "a depth curve")]
        let c = curve as f32;
        for v in &mut d {
            *v = v.clamp(0.0, 1.0).powf(c);
        }
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "a band of a picture's pixels"
    )]
    let band = ((s.seam.clamp(0.0, 0.5) * w as f64).round() as usize).min(w / 2);
    if band > 0 && w > 1 {
        for y in 0..h {
            let row = &mut d[y * w..(y + 1) * w];
            // both ends meet halfway, the change fading out over the band
            let meet = f32::midpoint(row[0], row[w - 1]);
            let (left, right) = (meet - row[0], meet - row[w - 1]);
            for x in 0..band {
                #[expect(clippy::cast_precision_loss, reason = "a share of the band")]
                let t = 1.0 - x as f32 / band as f32;
                let fade = t * t * (3.0 - 2.0 * t);
                row[x] += left * fade;
                row[w - 1 - x] += right * fade;
            }
        }
    }
    // stretched to fill 0 to 1, so the depth asked for is the depth made
    let (lo, hi) = d
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), &v| {
            (lo.min(v), hi.max(v))
        });
    if hi - lo > 1e-6 {
        for v in &mut d {
            *v = (*v - lo) / (hi - lo);
        }
    }
    d
}

/// A box blur `r` pixels either side: round left to right (the tile repeats), held at the
/// top and bottom edges.
fn box_blur(d: &[f32], w: usize, h: usize, r: usize) -> Vec<f32> {
    #[expect(clippy::cast_precision_loss, reason = "a count of pixels")]
    let n = (2 * r + 1) as f32;
    let mut across = vec![0.0; w * h];
    for y in 0..h {
        let row = &d[y * w..(y + 1) * w];
        let at = |x: isize| {
            #[expect(clippy::cast_possible_wrap, reason = "a pixel")]
            let i = x.rem_euclid(w as isize) as usize;
            row[i]
        };
        #[expect(clippy::cast_possible_wrap, reason = "a pixel")]
        let ri = r as isize;
        let mut sum: f32 = (-ri..=ri).map(at).sum();
        for x in 0..w {
            across[y * w + x] = sum / n;
            #[expect(clippy::cast_possible_wrap, reason = "a pixel")]
            let xi = x as isize;
            sum += at(xi + ri + 1) - at(xi - ri);
        }
    }
    let mut out = vec![0.0; w * h];
    for x in 0..w {
        let at = |y: isize| {
            #[expect(clippy::cast_possible_wrap, clippy::cast_sign_loss, reason = "a pixel")]
            let j = y.clamp(0, h as isize - 1) as usize;
            across[j * w + x]
        };
        #[expect(clippy::cast_possible_wrap, reason = "a pixel")]
        let ri = r as isize;
        let mut sum: f32 = (-ri..=ri).map(at).sum();
        for y in 0..h {
            out[y * w + x] = sum / n;
            #[expect(clippy::cast_possible_wrap, reason = "a pixel")]
            let yi = y as isize;
            sum += at(yi + ri + 1) - at(yi - ri);
        }
    }
    out
}

/// The greys of the picture `bytes` (any format the app reads), 0 to 1, with its size.
pub fn greys(bytes: &[u8]) -> Result<(Vec<f32>, usize, usize), String> {
    let img = image::load_from_memory(bytes).map_err(|e| format!("not a picture: {e}"))?;
    let g = img.to_luma16();
    let (w, h) = (g.width() as usize, g.height() as usize);
    let d = g
        .into_raw()
        .into_iter()
        .map(|v| f32::from(v) / 65535.0)
        .collect();
    Ok((d, w, h))
}

/// Saves the tile `name` in `folder` (made if missing): the depth map `depth` (`w` by `h`,
/// 0 to 1) as 16-bit greys, the picture `picture` (any format the app reads) as a JPEG, and
/// `recipe`. Its files, once all three are written.
pub fn save(
    folder: &Path,
    name: &str,
    depth: &[f32],
    w: usize,
    h: usize,
    picture: &[u8],
    recipe: &Recipe,
) -> Result<Files, String> {
    let f = files(folder, name);
    std::fs::create_dir_all(folder)
        .map_err(|e| format!("cannot make {}: {e}", folder.display()))?;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a share as 16-bit grey"
    )]
    let raw: Vec<u16> = depth
        .iter()
        .map(|v| (v.clamp(0.0, 1.0) * 65535.0).round() as u16)
        .collect();
    let size = |n: usize| u32::try_from(n).map_err(|_| "the depth map is too big".to_owned());
    let grey = image::ImageBuffer::<image::Luma<u16>, _>::from_raw(size(w)?, size(h)?, raw)
        .ok_or("the depth map's size does not match its pixels")?;
    grey.save(&f.depth)
        .map_err(|e| format!("cannot save {}: {e}", f.depth.display()))?;
    let photo = image::load_from_memory(picture).map_err(|e| format!("the picture: {e}"))?;
    photo
        .to_rgb8()
        .save(&f.picture)
        .map_err(|e| format!("cannot save {}: {e}", f.picture.display()))?;
    let json = serde_json::to_string_pretty(recipe).map_err(|e| e.to_string())?;
    std::fs::write(&f.recipe, json)
        .map_err(|e| format!("cannot save {}: {e}", f.recipe.display()))?;
    Ok(f)
}

/// The recipe in `path`.
pub fn read_recipe(path: &Path) -> Result<Recipe, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let r: Recipe = serde_json::from_str(&text)
        .map_err(|e| format!("{}: not a border tile's recipe: {e}", path.display()))?;
    if r.format != FORMAT_NAME || r.version > VERSION {
        return Err(format!(
            "{}: not a border tile this version of PowerWhittle reads",
            path.display()
        ));
    }
    if !(r.width > 0.0 && r.height > 0.0 && r.depth >= 0.0) {
        return Err(format!("{}: the tile has no size", path.display()));
    }
    Ok(r)
}

/// The tile whose depth map is `depth` as a 3D surface at its recipe's size, in mm: its
/// bottom left corner at the origin, its lowest point at 0, facing up. Read through the
/// same steps as an STL from the Library.
pub fn triangles(depth: &Path) -> Result<Vec<[DVec3; 3]>, String> {
    let f = of_depth_map(depth).ok_or_else(|| {
        format!(
            "{}: its recipe ({RECIPE_SUFFIX}) is missing",
            depth.display()
        )
    })?;
    let recipe = read_recipe(&f.recipe)?;
    let bytes = std::fs::read(depth).map_err(|e| format!("{}: {e}", depth.display()))?;
    let (d, w, h) = greys(&bytes)?;
    Ok(surface(&d, w, h, &recipe))
}

/// The depth map `d` (`w` by `h`, rows from the top) as a surface `recipe` sizes: no more
/// than [`MOST_POINTS`] along its longer side.
pub fn surface(d: &[f32], w: usize, h: usize, recipe: &Recipe) -> Vec<[DVec3; 3]> {
    if w < 2 || h < 2 {
        return vec![];
    }
    let step = w.max(h).div_ceil(MOST_POINTS).max(1);
    let (cols, rows) = ((w - 1).div_ceil(step) + 1, (h - 1).div_ceil(step) + 1);
    #[expect(clippy::cast_precision_loss, reason = "pixels of a picture")]
    let point = |i: usize, j: usize| {
        let (x, y) = ((i * step).min(w - 1), (j * step).min(h - 1));
        DVec3::new(
            x as f64 / (w - 1) as f64 * recipe.width,
            (1.0 - y as f64 / (h - 1) as f64) * recipe.height,
            f64::from(d[y * w + x]) * recipe.depth,
        )
    };
    let mut out = Vec::with_capacity(2 * cols * rows);
    for j in 0..rows - 1 {
        for i in 0..cols - 1 {
            // rows run down the picture, so each cell's corners go round anticlockwise
            // seen from above as (i, j+1), (i+1, j+1), (i+1, j), (i, j)
            let (a, b, c, e) = (
                point(i, j + 1),
                point(i + 1, j + 1),
                point(i + 1, j),
                point(i, j),
            );
            out.push([a, b, c]);
            out.push([a, c, e]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("cc-border-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn ramp(w: usize, h: usize) -> Vec<f32> {
        (0..w * h)
            .map(|k| {
                #[expect(clippy::cast_precision_loss, reason = "a test ramp")]
                let x = (k % w) as f32 / (w - 1) as f32;
                x
            })
            .collect()
    }

    #[test]
    fn the_ends_are_blended_to_meet_and_the_depth_fills_0_to_1() {
        let (w, h) = (100, 10);
        let s = Shaping {
            curve: 1.0,
            smooth: 0.0,
            seam: 0.1,
        };
        let d = shape(&ramp(w, h), w, h, &s);
        for y in 0..h {
            let row = &d[y * w..(y + 1) * w];
            assert!(
                (row[0] - row[w - 1]).abs() < 1e-5,
                "{} {}",
                row[0],
                row[w - 1]
            );
            // the middle is left as it was, only stretched
            assert!(row[50] > 0.3 && row[50] < 0.7);
        }
        let (lo, hi) = d
            .iter()
            .fold((1.0f32, 0.0f32), |(l, h), &v| (l.min(v), h.max(v)));
        assert!(lo.abs() < 1e-6 && (hi - 1.0).abs() < 1e-6);
    }

    #[test]
    fn smoothing_takes_the_steps_out_and_wraps_round_the_ends() {
        let (w, h) = (64, 64);
        // 8 steps of grey, as a coarse picture would carve
        let stairs: Vec<f32> = (0..w * h)
            .map(|k| {
                #[expect(clippy::cast_precision_loss, reason = "a test")]
                let y = (k / w) as f32;
                (y / 8.0).floor() / 8.0
            })
            .collect();
        let s = Shaping {
            curve: 1.0,
            smooth: 0.05,
            seam: 0.0,
        };
        let d = shape(&stairs, w, h, &s);
        let col: Vec<f32> = (0..h).map(|y| d[y * w + 5]).collect();
        let biggest_jump = col
            .windows(2)
            .map(|p| (p[1] - p[0]).abs())
            .fold(0.0, f32::max);
        assert!(biggest_jump < 0.05, "{biggest_jump}");
    }

    #[test]
    fn the_depth_curve_lifts_or_lowers_the_middle_greys() {
        let mid = |curve| {
            let d = vec![0.0, 0.5, 1.0];
            shape(
                &d,
                3,
                1,
                &Shaping {
                    curve,
                    smooth: 0.0,
                    seam: 0.0,
                },
            )[1]
        };
        assert!(mid(0.5) > 0.6 && mid(2.0) < 0.3 && (mid(1.0) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn a_saved_tile_comes_back_at_its_size_and_its_companions_are_known() {
        let dir = temp("save");
        let (w, h) = (40, 20);
        let mut photo = Vec::new();
        image::RgbImage::from_pixel(8, 4, image::Rgb([200, 150, 90]))
            .write_to(
                &mut std::io::Cursor::new(&mut photo),
                image::ImageFormat::Png,
            )
            .unwrap();
        let recipe = Recipe::new("victorian_carved", "2:1", 100.0, 50.0, 6.0);
        let name = free_name(&dir, "Victorian: carved?");
        assert_eq!(name, "Victorian carved");
        let f = save(&dir, &name, &ramp(w, h), w, h, &photo, &recipe).unwrap();
        assert_eq!(free_name(&dir, &name), "Victorian carved 2");
        assert_eq!(of_depth_map(&f.depth), Some(f.clone()));
        assert!(is_companion(&f.picture) && is_companion(&f.recipe) && !is_companion(&f.depth));
        assert_eq!(read_recipe(&f.recipe).unwrap(), recipe);
        let tris = triangles(&f.depth).unwrap();
        assert_eq!(tris.len(), 2 * (w - 1) * (h - 1));
        let (mut lo, mut hi) = (DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY));
        for p in tris.iter().flatten() {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
        assert!(lo.abs().max_element() < 1e-6, "{lo}");
        assert!(
            (hi - DVec3::new(100.0, 50.0, 6.0)).abs().max_element() < 1e-3,
            "{hi}"
        );
        // facing up
        let [a, b, c] = tris[0];
        assert!((b - a).cross(c - a).z > 0.0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_big_depth_map_is_thinned_to_a_quick_surface() {
        let (w, h) = (2400, 600);
        let r = Recipe::new("x", "4:1", 400.0, 100.0, 5.0);
        let tris = surface(&vec![0.5; w * h], w, h, &r);
        assert!(tris.len() < 2 * 601 * 151, "{}", tris.len());
        let far = tris.iter().flatten().fold(0.0f64, |m, p| m.max(p.x));
        assert!((far - 400.0).abs() < 1e-9);
    }

    #[test]
    fn a_picture_without_a_recipe_is_not_a_tile() {
        let dir = temp("plain");
        let p = dir.join("photo.png");
        std::fs::write(&p, b"x").unwrap();
        assert_eq!(of_depth_map(&p), None);
        assert!(triangles(&p).unwrap_err().contains("recipe"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
