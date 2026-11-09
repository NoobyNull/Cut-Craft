//! Previews of 3D clipart for the Clipart tab: a model seen from its largest side (from above,
//! or face on for one saved standing up), shaded as if lit from
//! the top left, so its shape reads at a glance. A picture beside the model with its name
//! (`mydesign.png` for `mydesign.stl`) is shown instead. Each preview is made once and kept
//! in the cache folder, until the file it was made from changes.

use cut_craft_core::component::Component;
use cut_craft_core::glam::DVec3;
use eframe::egui;
use image::{Rgba, RgbaImage};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Condvar, Mutex};

/// A preview's edge, in pixels.
pub const SIZE: u32 = 128;
/// The clear border round a model, in pixels.
const MARGIN: f64 = 4.0;
/// Files previewed as models.
pub const MODELS: [&str; 3] = ["stl", "obj", "ccclip"];
/// Pictures that stand in for a model with the same name.
pub const PICTURES: [&str; 4] = ["png", "jpg", "jpeg", "bmp"];
/// The colour of a lit model: a light wood.
const WOOD: [f64; 3] = [222.0, 184.0, 135.0];
/// A model shown as at least this deep, for its slopes' shading, as a share of its width.
const DEEPEST: f64 = 0.25;
/// Bump when previews are drawn differently, so the cached ones are made again.
const RENDERER: u32 = 2;

fn name(path: &Path) -> String {
    path.file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
}

/// What a file's extension says it is, lower case.
pub fn extension(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// `model` seen from its best side.
#[cfg(test)]
pub fn render(model: &Path) -> Result<RgbaImage, String> {
    render_at(model, SIZE)
}

/// `model` seen from its best side, `size` pixels square.
pub fn render_at(model: &Path, size: u32) -> Result<RgbaImage, String> {
    let bytes = std::fs::read(model).map_err(|e| format!("Couldn't read {}: {e}", name(model)))?;
    let image = if extension(model) == "ccclip" {
        let k = fmt_export_as_3d_clipart::read_clip(&bytes)
            .map_err(|e| format!("{}: {e}", name(model)))?;
        shade(&from_component(&k, size as usize))
    } else {
        let triangles =
            fmt_import_stl::triangles(&bytes).map_err(|e| format!("{}: {e}", name(model)))?;
        best_view(&triangles, size as usize).and_then(|h| shade(&h))
    };
    image.ok_or_else(|| format!("{} has nothing in it to show", name(model)))
}

/// The preview of `model` for a tile, [`SIZE`] pixels square at most.
#[cfg(test)]
pub fn preview(
    model: &Path,
    picture: Option<&Path>,
    cache: Option<&Path>,
) -> Result<RgbaImage, String> {
    preview_at(model, picture, cache, SIZE)
}

/// The preview of `model`, `size` pixels square at most: `picture` scaled to fit if there is
/// one, else the model rendered. Kept in `cache` (when there is one) and read from there
/// while the file is unchanged.
pub fn preview_at(
    model: &Path,
    picture: Option<&Path>,
    cache: Option<&Path>,
    size: u32,
) -> Result<RgbaImage, String> {
    let source = picture.unwrap_or(model);
    let kept = cache.and_then(|dir| Some(dir.join(cache_name(source, size)?)));
    if let Some((at, image)) = kept
        .as_deref()
        .and_then(|p| Some((p, image::open(p).ok()?)))
    {
        // marked as used, so [`prune`] keeps it
        let _ = std::fs::File::options()
            .write(true)
            .open(at)
            .and_then(|f| f.set_modified(std::time::SystemTime::now()));
        return Ok(image.into_rgba8());
    }
    let image = match picture {
        Some(p) => image::open(p)
            .map_err(|e| format!("Couldn't read {}: {e}", name(p)))?
            .thumbnail(size, size)
            .into_rgba8(),
        None => render_at(model, size)?,
    };
    if let Some(kept) = kept {
        // a preview that can't be kept is made again next time: nothing is lost
        let _ = std::fs::create_dir_all(kept.parent().unwrap_or(Path::new(".")))
            .and_then(|()| image.save(&kept).map_err(std::io::Error::other));
    }
    Ok(image)
}

/// How long a cached preview is kept unused: one of a file deleted, moved or changed since
/// is never used again.
pub const UNUSED: std::time::Duration = std::time::Duration::from_hours(1440);

/// Deletes the previews in `cache` unused for longer than `unused`.
pub fn prune(cache: &Path, unused: std::time::Duration) {
    let Ok(read) = std::fs::read_dir(cache) else {
        return;
    };
    for entry in read.flatten() {
        let path = entry.path();
        let old = (entry.metadata().and_then(|m| m.modified()))
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age > unused);
        if old && extension(&path) == "png" {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// The cached preview's file name for `source` at `size`: from its [`stamp`] (and the size,
/// when it isn't a tile's).
fn cache_name(source: &Path, size: u32) -> Option<String> {
    stamp(source).map(|hash| {
        if size == SIZE {
            format!("{hash:016x}.png")
        } else {
            format!("{hash:016x}-{size}.png")
        }
    })
}

/// What a preview of `source` is made from: its path, size and time changed, so a changed
/// file gets a new one. `None` when the file can't be looked at.
pub fn stamp(source: &Path) -> Option<u64> {
    let meta = std::fs::metadata(source).ok()?;
    let changed = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos());
    // FNV-1a: the same on every run and platform (std's hasher may change between releases)
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let path = source.to_string_lossy();
    let parts: [&[u8]; 4] = [
        &RENDERER.to_le_bytes(),
        path.as_bytes(),
        &meta.len().to_le_bytes(),
        &changed.to_le_bytes(),
    ];
    for byte in parts.into_iter().flatten() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    Some(hash)
}

/// Heights in pixels, row by row from the top, a square this many pixels across; NaN
/// where there is nothing.
struct Heights(Vec<f64>, usize);

/// How a model's `width` x `depth` box is fitted into the picture: the scale, and where its
/// lower left corner goes (from the picture's lower left).
#[expect(clippy::cast_precision_loss, reason = "the picture is small")]
fn fit(width: f64, depth: f64, pixels: usize) -> Option<(f64, f64, f64)> {
    let span = width.max(depth);
    if span.is_nan() || span <= 0.0 {
        return None;
    }
    let room = pixels as f64 - 2.0 * MARGIN;
    let scale = room / span;
    Some((
        scale,
        (pixels as f64 - width * scale) / 2.0,
        (pixels as f64 - depth * scale) / 2.0,
    ))
}

/// The ways a model can be looked at: (right, up, towards the viewer). From above first, so
/// it wins a tie (clipart is mostly modelled lying down), then from the front.
const VIEWS: [[DVec3; 3]; 6] = [
    [DVec3::X, DVec3::Y, DVec3::Z],
    [DVec3::X, DVec3::Z, DVec3::NEG_Y],
    [DVec3::NEG_Y, DVec3::Z, DVec3::NEG_X],
    [DVec3::X, DVec3::NEG_Y, DVec3::NEG_Z],
    [DVec3::NEG_X, DVec3::Z, DVec3::Y],
    [DVec3::Y, DVec3::Z, DVec3::X],
];

/// `triangles` seen from the side that shows them best.
#[cfg(test)]
fn shade_triangles(triangles: &[[DVec3; 3]]) -> Option<RgbaImage> {
    shade(&best_view(triangles, SIZE as usize)?)
}

/// The view of `triangles` that shows the most of them: the side with the largest outline
/// (a relief saved standing up is seen face on, a statue from the front), and of it and
/// the side opposite, the front (from above, or -Y) unless the other has far more relief
/// (a relief's flat back is the wrong way round).
#[expect(clippy::cast_precision_loss, reason = "pixel counts are small")]
fn best_view(triangles: &[[DVec3; 3]], pixels: usize) -> Option<Heights> {
    let outline = |(Heights(z, _), scale): &(Heights, f64)| {
        z.iter().filter(|h| !h.is_nan()).count() as f64 / (scale * scale)
    };
    let relief = |Heights(z, _): &Heights| {
        let shown: Vec<f64> = z.iter().copied().filter(|h| !h.is_nan()).collect();
        let mean = shown.iter().sum::<f64>() / shown.len().max(1) as f64;
        shown.iter().map(|h| (h - mean).powi(2)).sum::<f64>() / shown.len().max(1) as f64
    };
    // the first three views and their opposites (the last three) have the same outline
    let mut best: Option<(usize, (Heights, f64))> = None;
    for (i, view) in VIEWS[..3].iter().enumerate() {
        let seen = from_triangles(triangles, *view, pixels)?;
        // a little larger to win, so near ties go to the view listed first
        if best
            .as_ref()
            .is_none_or(|(_, b)| outline(&seen) > outline(b) * 1.05)
        {
            best = Some((i, seen));
        }
    }
    let (i, (front, _)) = best?;
    let (back, _) = from_triangles(triangles, VIEWS[i + 3], pixels)?;
    // well ahead to win: a figure carved all round keeps its front
    Some(if relief(&back) > relief(&front) * 2.0 {
        back
    } else {
        front
    })
}

/// The highest of `triangles`, seen along `view` (right, up, towards the viewer), over
/// each pixel's centre; and how many pixels a model unit is.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "pixel indices are small and clamped non-negative"
)]
#[expect(
    clippy::many_single_char_names,
    reason = "the triangle's corners and barycentric u, v"
)]
fn from_triangles(
    triangles: &[[DVec3; 3]],
    view: [DVec3; 3],
    pixels: usize,
) -> Option<(Heights, f64)> {
    let [right, up, out] = view;
    let seen = |p: &DVec3| DVec3::new(p.dot(right), p.dot(up), p.dot(out));
    let points = || triangles.iter().flatten().map(seen);
    let lo = points().fold(DVec3::INFINITY, DVec3::min);
    let hi = points().fold(DVec3::NEG_INFINITY, DVec3::max);
    let (scale, left, bottom) = fit(hi.x - lo.x, hi.y - lo.y, pixels)?;
    let top = pixels as f64;
    // in pixels, y down the picture, z from the model's lowest point
    let to_pixels = |p: DVec3| {
        DVec3::new(
            left + (p.x - lo.x) * scale,
            top - bottom - (p.y - lo.y) * scale,
            (p.z - lo.z) * scale,
        )
    };
    let mut heights = vec![f64::NAN; pixels * pixels];
    let index = |v: f64| (v.max(0.0) as usize).min(pixels - 1);
    for t in triangles {
        let [a, b, c] = t.map(|p| to_pixels(seen(&p)));
        let (ab, ac) = (b - a, c - a);
        let det = ab.x * ac.y - ab.y * ac.x;
        if det.abs() < 1e-12 {
            continue; // edge-on from above
        }
        let (min, max) = (a.min(b).min(c), a.max(b).max(c));
        for row in index(min.y - 0.5)..=index(max.y + 0.5) {
            for col in index(min.x - 0.5)..=index(max.x + 0.5) {
                let (dx, dy) = (col as f64 + 0.5 - a.x, row as f64 + 0.5 - a.y);
                let u = (dx * ac.y - dy * ac.x) / det;
                let v = (ab.x * dy - ab.y * dx) / det;
                if u < -1e-9 || v < -1e-9 || u + v > 1.0 + 1e-9 {
                    continue;
                }
                let z = a.z + u * ab.z + v * ac.z;
                let h = &mut heights[row * pixels + col];
                if h.is_nan() || z > *h {
                    *h = z;
                }
            }
        }
    }
    Some((Heights(heights, pixels), scale))
}

/// A clipart component's heights, each pixel taking the cell under its centre.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "cell and pixel indices are small and non-negative"
)]
fn from_component(k: &Component, pixels: usize) -> Heights {
    let (width, depth) = (k.columns as f64 * k.cell, k.rows as f64 * k.cell);
    let mut heights = vec![f64::NAN; pixels * pixels];
    let Some((scale, left, bottom)) = fit(width, depth, pixels) else {
        return Heights(heights, pixels);
    };
    let lowest = k
        .heights
        .iter()
        .copied()
        .filter(|h| !h.is_nan())
        .fold(f64::INFINITY, f64::min);
    for row in 0..pixels {
        for col in 0..pixels {
            let x = (col as f64 + 0.5 - left) / scale;
            let y = (pixels as f64 - row as f64 - 0.5 - bottom) / scale;
            if x < 0.0 || y < 0.0 || x >= width || y >= depth {
                continue;
            }
            let at = ((y / k.cell) as usize).min(k.rows - 1) * k.columns
                + ((x / k.cell) as usize).min(k.columns - 1);
            if let Some(h) = k.heights.get(at).filter(|h| !h.is_nan()) {
                heights[row * pixels + col] = (h - lowest) * scale;
            }
        }
    }
    Heights(heights, pixels)
}

/// `heights` lit from the top left; clear where there is nothing. `None` when empty.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "colour channels are clamped to 0..=255"
)]
fn shade(heights: &Heights) -> Option<RgbaImage> {
    let &Heights(ref z, pixels) = heights;
    let side = u32::try_from(pixels).ok()?;
    let highest = z
        .iter()
        .copied()
        .filter(|h| !h.is_nan())
        .fold(f64::NAN, f64::max);
    if highest.is_nan() {
        return None;
    }
    // carvings are shallow: steepen a low model's slopes so its detail still shows
    let deepen = (DEEPEST * f64::from(side) / highest).clamp(1.0, 6.0);
    // towards the light: left, up the picture, and above
    let light = DVec3::new(-1.0, 1.0, 1.5).normalize();
    let at = |row: usize, col: usize, here: f64| {
        let h = z[row * pixels + col];
        if h.is_nan() { here } else { h }
    };
    Some(RgbaImage::from_fn(side, side, |x, y| {
        let (col, row) = (x as usize, y as usize);
        let here = z[row * pixels + col];
        if here.is_nan() {
            return Rgba([0, 0, 0, 0]);
        }
        let left = at(row, col.saturating_sub(1), here);
        let right = at(row, (col + 1).min(pixels - 1), here);
        let up = at(row.saturating_sub(1), col, here);
        let down = at((row + 1).min(pixels - 1), col, here);
        // slopes per pixel; the picture's y runs down, the model's up
        let normal = DVec3::new(
            -(right - left) * deepen / 2.0,
            (down - up) * deepen / 2.0,
            1.0,
        )
        .normalize();
        let lit = normal.dot(light).max(0.0);
        // higher parts a touch lighter, so a flat-topped model still shows its levels
        let rise = if highest > 0.0 { here / highest } else { 1.0 };
        let brightness = (0.25 + 0.8 * lit) * (0.85 + 0.15 * rise);
        let [r, g, b] = WOOD.map(|c| (c * brightness).round().clamp(0.0, 255.0) as u8);
        Rgba([r, g, b, 255])
    }))
}

/// Previews made on background threads, for the tiles on screen, newest asked first.
pub struct Previews {
    /// Each model's preview, and the [`stamp`] it was asked for with.
    shown: HashMap<PathBuf, (u64, Shown)>,
    jobs: Arc<Jobs>,
    done: Receiver<Made>,
    send: Sender<Made>,
    /// Workers started so far (started as they are first needed).
    workers: usize,
    /// Where previews are kept between runs; `None` makes them again each run.
    cache: Option<PathBuf>,
    /// Counts the frames drawn, so the least recently shown pictures are let go first.
    frame: u64,
    /// Each preview's edge, in pixels.
    size: u32,
    /// Pictures kept on the graphics card at most.
    keep: usize,
    /// Threads making previews at most.
    threads: usize,
}

/// A model, the picture beside it if it has one, and the [`stamp`] asked for.
type Job = (PathBuf, Option<PathBuf>, u64);
/// A model, the [`stamp`] asked for, and its preview or why it has none.
type Made = (PathBuf, u64, Result<RgbaImage, String>);

/// Previews waiting to be made, and whether the workers should stop.
#[derive(Default)]
struct Jobs {
    waiting: Mutex<Waiting>,
    ready: Condvar,
}

#[derive(Default)]
struct Waiting {
    jobs: Vec<Job>,
    stop: bool,
}

enum Shown {
    Waiting,
    Ready(egui::TextureHandle, u64),
    Failed(String),
}

/// What a tile shows for its model.
pub enum Preview<'a> {
    Waiting,
    Ready(&'a egui::TextureHandle),
    Failed(&'a str),
}

/// Pictures kept on the graphics card at most; ones scrolled past come back from the cache.
const KEEP: usize = 400;

impl Default for Previews {
    fn default() -> Self {
        Self::new(None)
    }
}

impl Previews {
    /// Tiles' previews, [`SIZE`] pixels across.
    pub fn new(cache: Option<PathBuf>) -> Self {
        let threads = cut_craft_core::simulation::cores().saturating_sub(1).max(1);
        Self::sized(cache, SIZE, KEEP, threads)
    }

    /// Previews `size` pixels across, at most `keep` of them on the graphics card, made by
    /// at most `threads` threads.
    pub fn sized(cache: Option<PathBuf>, size: u32, keep: usize, threads: usize) -> Self {
        let (send, done) = channel();
        Self {
            shown: HashMap::new(),
            jobs: Arc::default(),
            done,
            send,
            workers: 0,
            cache,
            frame: 0,
            size,
            keep,
            threads,
        }
    }

    /// The preview of `model` (made from `picture` if it has one), asking for it if new or
    /// if `stamp` (its source's [`stamp`]) says the file has changed since.
    pub fn get(&mut self, model: &Path, picture: Option<&Path>, stamp: u64) -> Preview<'_> {
        if self.shown.get(model).is_none_or(|(had, _)| *had != stamp) {
            self.ask(model, picture, stamp);
        }
        match self.shown.get_mut(model).map(|(_, shown)| shown) {
            Some(Shown::Ready(texture, seen)) => {
                *seen = self.frame;
                Preview::Ready(texture)
            }
            Some(Shown::Failed(e)) => Preview::Failed(e),
            Some(Shown::Waiting) | None => Preview::Waiting,
        }
    }

    fn ask(&mut self, model: &Path, picture: Option<&Path>, stamp: u64) {
        self.shown.insert(model.to_owned(), (stamp, Shown::Waiting));
        let mut waiting = self
            .jobs
            .waiting
            .lock()
            .expect("no worker panics holding it");
        waiting
            .jobs
            .push((model.to_owned(), picture.map(Path::to_owned), stamp));
        drop(waiting);
        self.jobs.ready.notify_one();
        if self.workers < self.threads {
            self.workers += 1;
            let (jobs, send, cache) = (self.jobs.clone(), self.send.clone(), self.cache.clone());
            let size = self.size;
            let spawned = std::thread::Builder::new()
                .name("clipart previews".into())
                .spawn(move || work(&jobs, &send, cache.as_deref(), size));
            if spawned.is_err() {
                self.workers -= 1;
            }
        }
    }

    /// Takes in the previews made since the last frame; repaints until all asked for are
    /// in. Called once a frame, before the tiles.
    pub fn update(&mut self, ctx: &egui::Context) {
        self.frame += 1;
        for (model, stamp, made) in self.done.try_iter() {
            if self
                .shown
                .get(&model)
                .is_none_or(|(asked, _)| *asked != stamp)
            {
                continue; // made from a file that has changed since, or no longer wanted
            }
            let shown = match made {
                Ok(image) => {
                    let size = [image.width() as usize, image.height() as usize];
                    let picture = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
                    let name = format!("clipart {}", model.display());
                    let texture = ctx.load_texture(name, picture, egui::TextureOptions::LINEAR);
                    Shown::Ready(texture, self.frame)
                }
                Err(e) => Shown::Failed(e),
            };
            self.shown.insert(model, (stamp, shown));
        }
        let ready = self
            .shown
            .values()
            .filter(|(_, s)| matches!(s, Shown::Ready(..)));
        if ready.count() > self.keep {
            let mut seen: Vec<(u64, PathBuf)> = (self.shown.iter())
                .filter_map(|(p, (_, s))| match s {
                    Shown::Ready(_, seen) => Some((*seen, p.clone())),
                    _ => None,
                })
                .collect();
            seen.sort();
            for (_, p) in &seen[..seen.len() - self.keep] {
                self.shown.remove(p);
            }
        }
        if self
            .shown
            .values()
            .any(|(_, s)| matches!(s, Shown::Waiting))
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }

    /// How many previews have been asked for.
    #[cfg(test)]
    pub fn asked(&self) -> usize {
        self.shown.len()
    }

    /// How many previews are made and on the graphics card.
    #[cfg(test)]
    pub fn ready(&self) -> usize {
        (self.shown.values())
            .filter(|(_, s)| matches!(s, Shown::Ready(..)))
            .count()
    }

    /// Drops the previews still waiting (another folder is being looked in).
    pub fn forget_waiting(&mut self) {
        let mut waiting = self
            .jobs
            .waiting
            .lock()
            .expect("no worker panics holding it");
        for (model, ..) in waiting.jobs.drain(..) {
            self.shown.remove(&model);
        }
    }
}

/// A larger preview's edge, in pixels: the pop-up over a tile the pointer rests on.
pub const LARGE: u32 = 512;

/// Larger previews, for the pop-up over the tile the pointer rests on: one made at a time,
/// the newest hovered first, the few last seen kept.
pub struct Enlarged {
    previews: Previews,
    /// The model asked for last.
    last: Option<PathBuf>,
}

impl Enlarged {
    /// Kept on the graphics card at most: a few, to look back at.
    const KEEP: usize = 8;

    pub fn new(cache: Option<PathBuf>) -> Self {
        Self {
            previews: Previews::sized(cache, LARGE, Self::KEEP, 1),
            last: None,
        }
    }

    /// As [`Previews::get`]; those asked for before and not yet started are dropped, so
    /// passing the pointer over many tiles doesn't leave a queue of them.
    pub fn get(&mut self, model: &Path, picture: Option<&Path>, stamp: u64) -> Preview<'_> {
        if self.last.as_deref() != Some(model) {
            self.previews.forget_waiting();
            self.last = Some(model.to_owned());
        }
        self.previews.get(model, picture, stamp)
    }

    pub fn update(&mut self, ctx: &egui::Context) {
        self.previews.update(ctx);
    }

    #[cfg(test)]
    pub fn asked(&self) -> usize {
        self.previews.asked()
    }

    #[cfg(test)]
    pub fn ready(&self) -> usize {
        self.previews.ready()
    }
}

impl Default for Enlarged {
    fn default() -> Self {
        Self::new(None)
    }
}

impl Drop for Previews {
    fn drop(&mut self) {
        if let Ok(mut waiting) = self.jobs.waiting.lock() {
            waiting.stop = true;
        }
        self.jobs.ready.notify_all();
    }
}

/// A worker: makes the newest asked-for preview until told to stop.
fn work(jobs: &Jobs, send: &Sender<Made>, cache: Option<&Path>, size: u32) {
    loop {
        let Ok(mut waiting) = jobs.waiting.lock() else {
            return;
        };
        let (model, picture, stamp) = loop {
            if waiting.stop {
                return;
            }
            if let Some(job) = waiting.jobs.pop() {
                break job;
            }
            waiting = match jobs.ready.wait(waiting) {
                Ok(w) => w,
                Err(_) => return,
            };
        };
        drop(waiting);
        let made = preview_at(&model, picture.as_deref(), cache, size);
        if send.send((model, stamp, made)).is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cut-craft-preview-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A square pyramid 10 wide and 5 high, its peak at the centre, closed underneath.
    fn pyramid() -> Vec<[DVec3; 3]> {
        let peak = DVec3::new(5.0, 5.0, 5.0);
        let c = [
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(10.0, 0.0, 0.0),
            DVec3::new(10.0, 10.0, 0.0),
            DVec3::new(0.0, 10.0, 0.0),
        ];
        // its sides, and its base closing it as a model's is
        (0..4)
            .map(|i| [c[i], c[(i + 1) % 4], peak])
            .chain([[c[0], c[2], c[1]], [c[0], c[3], c[2]]])
            .collect()
    }

    fn alpha(image: &RgbaImage, x: u32, y: u32) -> u8 {
        image.get_pixel(x, y).0[3]
    }

    fn brightness(image: &RgbaImage, x: u32, y: u32) -> u32 {
        image.get_pixel(x, y).0[..3]
            .iter()
            .map(|&c| u32::from(c))
            .sum()
    }

    /// A model lying down is seen from above, filling the picture but for a margin: solid where it is,
    /// clear round it.
    #[test]
    fn model_seen_from_above() {
        let image = shade_triangles(&pyramid()).unwrap();
        assert_eq!(image.dimensions(), (SIZE, SIZE));
        assert_eq!(alpha(&image, SIZE / 2, SIZE / 2), 255);
        assert_eq!(alpha(&image, 1, 1), 0, "the margin is clear");
    }

    /// Light comes from the top left, so the faces turned that way are brighter: the shape
    /// reads as raised, not as a flat silhouette.
    #[test]
    fn lit_from_top_left() {
        let image = shade_triangles(&pyramid()).unwrap();
        let (q, h) = (SIZE / 4, SIZE / 2);
        let left = brightness(&image, q, h);
        let right = brightness(&image, SIZE - q, h);
        let top = brightness(&image, h, q);
        let bottom = brightness(&image, h, SIZE - q);
        assert!(left > right, "left {left} right {right}");
        assert!(top > bottom, "top {top} bottom {bottom}");
    }

    /// A long model keeps its proportions: it is not stretched to the square.
    #[test]
    fn keeps_proportions() {
        let long: Vec<_> = pyramid()
            .into_iter()
            .map(|t| t.map(|p| DVec3::new(p.x * 4.0, p.y, p.z)))
            .collect();
        let image = shade_triangles(&long).unwrap();
        assert_eq!(alpha(&image, SIZE / 2, SIZE / 2), 255);
        assert_eq!(
            alpha(&image, SIZE / 2, SIZE / 5),
            0,
            "above a flat model is clear"
        );
    }

    #[test]
    fn nothing_to_see() {
        assert!(shade_triangles(&[]).is_none());
    }

    #[test]
    fn renders_stl_and_clipart_files() {
        use cut_craft_core::component::{Combine, Component};
        use cut_craft_core::geometry::{Point, Rect};
        let dir = temp("files");
        let stl = dir.join("pyramid.STL");
        std::fs::write(&stl, fmt_export_as_stl::binary_stl(&pyramid())).unwrap();
        assert_eq!(alpha(&render(&stl).unwrap(), SIZE / 2, SIZE / 2), 255);

        // a 4 x 2 block, but for its top left cell
        let area = Rect {
            min: Point::ZERO,
            max: Point::new(4.0, 2.0),
        };
        let k = Component::from_fn("blocks", Combine::Add, area, 1.0, |p| {
            (p.x > 1.0 || p.y < 1.0).then_some(p.x)
        });
        let clip = dir.join("blocks.ccclip");
        std::fs::write(&clip, fmt_export_as_3d_clipart::clip_bytes(&k).unwrap()).unwrap();
        let image = render(&clip).unwrap();
        assert_eq!(alpha(&image, SIZE / 2 + 2, SIZE / 2 + 2), 255);
        assert_eq!(alpha(&image, SIZE / 2, 2), 0);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A file that is not a model says so, naming it, rather than showing a blank tile.
    #[test]
    fn a_broken_file_says_so() {
        let dir = temp("broken");
        let bad = dir.join("broken.stl");
        std::fs::write(&bad, "not a model").unwrap();
        let e = render(&bad).unwrap_err();
        assert!(e.contains("broken.stl"), "{e}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A model is rendered once: the preview is kept in the cache and read from there
    /// until the model changes.
    #[test]
    fn rendered_once() {
        let dir = temp("once");
        let cache = dir.join("cache");
        let stl = dir.join("pyramid.stl");
        std::fs::write(&stl, fmt_export_as_stl::binary_stl(&pyramid())).unwrap();
        let first = preview(&stl, None, Some(&cache)).unwrap();
        let kept: Vec<_> = std::fs::read_dir(&cache).unwrap().flatten().collect();
        assert_eq!(kept.len(), 1);
        // the cached picture is what is shown, even when the renderer would now fail
        std::fs::write(kept[0].path(), {
            let mut marked = first.clone();
            marked.put_pixel(0, 0, image::Rgba([1, 2, 3, 255]));
            let mut png = vec![];
            marked
                .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
                .unwrap();
            png
        })
        .unwrap();
        let again = preview(&stl, None, Some(&cache)).unwrap();
        assert_eq!(again.get_pixel(0, 0).0, [1, 2, 3, 255]);
        // a changed model is rendered again
        let bigger: Vec<_> = pyramid()
            .into_iter()
            .map(|t| t.map(|p| p * 2.0))
            .chain(pyramid())
            .collect();
        std::fs::write(&stl, fmt_export_as_stl::binary_stl(&bigger)).unwrap();
        let changed = preview(&stl, None, Some(&cache)).unwrap();
        assert_eq!(changed.get_pixel(0, 0).0[3], 0);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A picture beside the model (mydesign.png for mydesign.stl) is its preview: the model
    /// is not read at all.
    #[test]
    fn picture_beside_the_model() {
        let dir = temp("beside");
        let stl = dir.join("mydesign.stl");
        std::fs::write(&stl, "not even a model").unwrap();
        let png = dir.join("mydesign.png");
        RgbaImage::from_pixel(400, 200, image::Rgba([10, 200, 30, 255]))
            .save(&png)
            .unwrap();
        let image = preview(&stl, Some(&png), None).unwrap();
        assert!(image.width() <= SIZE && image.height() <= SIZE);
        assert_eq!(image.width(), SIZE, "scaled down to fit, keeping its shape");
        assert_eq!(image.get_pixel(5, 5).0, [10, 200, 30, 255]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Previews are made off the drawing thread and turn up a frame or so later.
    #[test]
    fn made_in_the_background() {
        let dir = temp("background");
        let stl = dir.join("pyramid.stl");
        std::fs::write(&stl, fmt_export_as_stl::binary_stl(&pyramid())).unwrap();
        let bad = dir.join("bad.stl");
        std::fs::write(&bad, "nope").unwrap();
        let mut previews = Previews::new(None);
        let ctx = egui::Context::default();
        assert!(matches!(previews.get(&stl, None, 1), Preview::Waiting));
        let _ = previews.get(&bad, None, 1);
        let start = std::time::Instant::now();
        loop {
            previews.update(&ctx);
            let ready = matches!(previews.get(&stl, None, 1), Preview::Ready(_));
            if ready && matches!(previews.get(&bad, None, 1), Preview::Failed(_)) {
                break;
            }
            assert!(start.elapsed().as_secs() < 10, "no preview after 10 s");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A model changed while the tab is open (its stamp changes) is previewed again, and a
    /// preview made from the old file is not shown for the new one.
    /// A larger preview is drawn at its own size, not the tile's stretched, and kept apart
    /// from the tile's in the cache.
    #[test]
    fn larger_previews_are_drawn_larger() {
        let dir = temp("larger");
        let cache = dir.join("cache");
        let stl = dir.join("pyramid.stl");
        std::fs::write(&stl, fmt_export_as_stl::binary_stl(&pyramid())).unwrap();
        let small = preview(&stl, None, Some(&cache)).unwrap();
        let large = preview_at(&stl, None, Some(&cache), LARGE).unwrap();
        assert_eq!(small.dimensions(), (SIZE, SIZE));
        assert_eq!(large.dimensions(), (LARGE, LARGE));
        assert_eq!(alpha(&large, LARGE / 2, LARGE / 2), 255);
        assert_eq!(
            std::fs::read_dir(&cache).unwrap().count(),
            2,
            "one of each kept"
        );
        // and read back from the cache at the size asked for
        let again = preview_at(&stl, None, Some(&cache), LARGE).unwrap();
        assert_eq!(again.dimensions(), (LARGE, LARGE));
        let png = dir.join("wide.png");
        image::RgbImage::new(2000, 1000).save(&png).unwrap();
        let picture = preview_at(&stl, Some(&png), None, LARGE).unwrap();
        assert_eq!(picture.dimensions(), (LARGE, LARGE / 2));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Passing the pointer over many tiles doesn't queue a larger preview for each: only
    /// the one hovered last is wanted.
    #[test]
    fn only_the_last_hovered_is_enlarged() {
        let dir = temp("enlarged");
        let mut enlarged = Enlarged::new(None);
        // no worker has started on them yet: all three are still waiting
        let mut waiting = enlarged.previews.jobs.waiting.lock().unwrap();
        waiting.stop = true;
        drop(waiting);
        for name in ["a.stl", "b.stl", "c.stl"] {
            let _ = enlarged.get(&dir.join(name), None, 1);
        }
        assert_eq!(enlarged.asked(), 1);
        assert!(matches!(
            enlarged.get(&dir.join("c.stl"), None, 1),
            Preview::Waiting
        ));
        assert_eq!(
            enlarged.asked(),
            1,
            "asking again for the same one keeps it"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_changed_model_is_previewed_again() {
        let dir = temp("changed");
        let stl = dir.join("pyramid.stl");
        std::fs::write(&stl, fmt_export_as_stl::binary_stl(&pyramid())).unwrap();
        let before = stamp(&stl).unwrap();
        let mut previews = Previews::new(None);
        let ctx = egui::Context::default();
        let start = std::time::Instant::now();
        while !matches!(previews.get(&stl, None, before), Preview::Ready(_)) {
            previews.update(&ctx);
            assert!(start.elapsed().as_secs() < 10, "no preview after 10 s");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let twice: Vec<_> = pyramid().into_iter().chain(pyramid()).collect();
        std::fs::write(&stl, fmt_export_as_stl::binary_stl(&twice)).unwrap();
        let after = stamp(&stl).unwrap();
        assert_ne!(before, after);
        assert!(matches!(previews.get(&stl, None, after), Preview::Waiting));
        // the old preview, coming in late, is not taken for the new file's
        previews
            .send
            .send((stl.clone(), before, Ok(RgbaImage::new(1, 1))))
            .unwrap();
        previews.forget_waiting();
        let _ = previews.get(&stl, None, after);
        previews.update(&ctx);
        let shown = previews.shown.get(&stl).map(|(s, _)| *s);
        assert_eq!(shown, Some(after));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A pyramid stood up, its base in the X-Z plane and its point towards -Y, as a relief
    /// saved standing up is.
    fn standing() -> Vec<[DVec3; 3]> {
        pyramid()
            .into_iter()
            .map(|t| t.map(|p| DVec3::new(p.x, -p.z, p.y)))
            .collect()
    }

    /// A model saved standing up is seen face on: along its thinnest side, from the side
    /// with its shape (its point), not from its top edge.
    #[test]
    fn a_standing_model_is_seen_face_on() {
        let image = shade_triangles(&standing()).unwrap();
        let (q, h) = (SIZE / 4, SIZE / 2);
        // seen from above it would be a thin strip across the middle
        assert_eq!(alpha(&image, h, q), 255, "not just its top edge");
        assert_eq!(alpha(&image, h, SIZE - q), 255);
        // and from the front: lit like the pyramid lying down
        assert!(brightness(&image, q, h) > brightness(&image, SIZE - q, h));
        assert!(brightness(&image, h, q) > brightness(&image, h, SIZE - q));
    }

    /// A model with a flat back is seen from its front, whichever way it faces.
    #[test]
    fn seen_from_the_side_with_its_shape() {
        let backwards: Vec<_> = standing()
            .into_iter()
            .map(|t| t.map(|p| DVec3::new(-p.x, -p.y, p.z)))
            .collect();
        let image = shade_triangles(&backwards).unwrap();
        let (q, h) = (SIZE / 4, SIZE / 2);
        assert!(brightness(&image, q, h) > brightness(&image, SIZE - q, h));
        assert!(brightness(&image, h, q) > brightness(&image, h, SIZE - q));
    }

    /// Previews unused for a long time (their files deleted, moved or changed) are cleared
    /// out; ones in use are kept, as reading one marks it used.
    #[test]
    fn unused_previews_cleared() {
        let dir = temp("prune");
        let cache = dir.join("cache");
        let stl = dir.join("pyramid.stl");
        std::fs::write(&stl, fmt_export_as_stl::binary_stl(&pyramid())).unwrap();
        preview(&stl, None, Some(&cache)).unwrap();
        let old = cache.join("0123456789abcdef.png");
        std::fs::write(&old, "").unwrap();
        let long_ago = std::time::SystemTime::now() - 2 * UNUSED;
        for f in std::fs::read_dir(&cache).unwrap().flatten() {
            let file = std::fs::File::options().write(true).open(f.path()).unwrap();
            file.set_modified(long_ago).unwrap();
        }
        // the model's preview is used again, the other isn't
        preview(&stl, None, Some(&cache)).unwrap();
        prune(&cache, UNUSED);
        let left: Vec<_> = std::fs::read_dir(&cache).unwrap().flatten().collect();
        assert_eq!(left.len(), 1);
        assert!(!old.exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
