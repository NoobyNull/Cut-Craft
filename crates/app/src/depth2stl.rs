//! The Depth to STL add-on (depth2stl), seen from the app: Model ▸ Depth to STL runs it on
//! the chosen picture through its plugin API (`depth2stl api describe` and `depth2stl api
//! run`: JSON over the standard streams), in the background, and the height map it answers
//! with becomes a picture and a relief component. Its settings form is drawn from the
//! schema the add-on describes, so a newer add-on brings its own settings and tooltips.

use cut_craft_core::bitmap::Bitmap;
use cut_craft_core::document::EntityId;
use cut_craft_core::geometry::{Point, Rect};
use serde_json::{Value, json};
use std::io::{BufRead as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;

/// The add-on's id: its folder under the add-ons folder.
pub const ADDON_ID: &str = cut_craft_updater::addons::DEPTH2STL.id;
/// The plugin API this Cut Craft speaks.
pub const API: u64 = 1;

/// The installed add-on's program, when its manifest says it speaks [`API`].
pub fn find(addons: &Path) -> Option<PathBuf> {
    let manifest = cut_craft_updater::addons::installed(addons, ADDON_ID)?;
    (manifest.api == Some(1)).then(|| addons.join(ADDON_ID).join(manifest.program()))
}

/// What `depth2stl api describe` says about itself.
#[derive(Clone, Debug, PartialEq)]
pub struct Description {
    pub version: String,
    /// A JSON Schema of its settings, for [`crate::form`].
    pub settings: Value,
}

impl Description {
    /// Reads the describe answer.
    pub fn parse(text: &str) -> Result<Self, String> {
        let v: Value = serde_json::from_str(text.trim()).map_err(|e| {
            format!("Depth to STL described itself in a way Cut Craft cannot read ({e})")
        })?;
        if v["api"].as_u64() != Some(API) {
            return Err(format!(
                "this Depth to STL speaks plugin API {}, and this Cut Craft speaks {API}: update \
                 both from Help ▸ Add-ons and Help ▸ Check for Updates",
                v["api"]
            ));
        }
        if !v["settings"]["properties"].is_object() {
            return Err("Depth to STL did not list its settings".into());
        }
        Ok(Self {
            version: v["version"].as_str().unwrap_or("?").to_owned(),
            settings: v["settings"].clone(),
        })
    }

    /// Every setting at its default.
    pub fn defaults(&self) -> Value {
        let props = self.settings["properties"]
            .as_object()
            .into_iter()
            .flatten();
        Value::Object(
            props
                .filter_map(|(k, s)| Some((k.clone(), s.get("default")?.clone())))
                .collect(),
        )
    }
}

fn command(program: &Path) -> Command {
    #[cfg_attr(not(windows), expect(unused_mut, reason = "changed on Windows only"))]
    let mut command = Command::new(program);
    #[cfg(windows)]
    {
        // no console window flashing up
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

/// Asks the add-on to describe itself (a second or two: its first run unpacks itself).
pub fn describe(program: &Path) -> Result<Description, String> {
    let out = command(program)
        .args(["api", "describe"])
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("cannot run Depth to STL ({}): {e}", program.display()))?;
    if !out.status.success() {
        return Err(format!(
            "Depth to STL would not describe itself: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Description::parse(&String::from_utf8_lossy(&out.stdout))
}

/// The picture as the add-on's input: a 16-bit grey PNG of its depth when it has one (a
/// depth map, or a photo whose depth was estimated), else of its greys; and the photo's
/// colours too when it has depth and colours, for the add-on to lift fine detail from.
pub fn inputs(bitmap: &Bitmap) -> Result<(Vec<u8>, Option<Vec<u8>>), String> {
    let size = |n: usize| u32::try_from(n).map_err(|_| "the picture is too big".to_string());
    let (w, h) = (size(bitmap.columns)?, size(bitmap.rows)?);
    let deep = bitmap.depth.len() == bitmap.grey.len() && !bitmap.grey.is_empty();
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped into 0..=65535"
    )]
    let levels: Vec<u16> = if deep {
        bitmap
            .depth
            .iter()
            .map(|d| (d.clamp(0.0, 1.0) * 65_535.0).round() as u16)
            .collect()
    } else {
        bitmap.grey.iter().map(|&g| u16::from(g) * 257).collect()
    };
    let grey = image::ImageBuffer::<image::Luma<u16>, _>::from_raw(w, h, levels)
        .ok_or("the picture's pixels are short")?;
    let png = |image: image::DynamicImage| -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        image
            .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        Ok(out)
    };
    let render = if deep && bitmap.rgb.len() == 3 * bitmap.grey.len() {
        let rgb = image::RgbImage::from_raw(w, h, bitmap.rgb.clone())
            .ok_or("the picture's colours are short")?;
        Some(png(rgb.into())?)
    } else {
        None
    };
    Ok((png(grey.into())?, render))
}

/// What the add-on made.
#[derive(Clone, Debug, PartialEq)]
pub struct Made {
    /// The finished model's height at each pixel, 0 to 1 of [`Made::top`], rows from the top.
    pub heights: Vec<f32>,
    pub columns: usize,
    pub rows: usize,
    /// Its size in millimetres.
    pub width: f64,
    pub height: f64,
    /// The height a value of 1 stands for, in millimetres.
    pub top: f64,
    /// What the add-on said about detail finer than the bit, when it simulated it.
    pub bit: Option<String>,
}

impl Made {
    /// As a picture placed with its centre on `centre`, its heights kept at full precision.
    pub fn bitmap(&self, name: &str, centre: Point) -> Bitmap {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped into 0..=255"
        )]
        let grey = self
            .heights
            .iter()
            .map(|h| (h.clamp(0.0, 1.0) * 255.0).round() as u8)
            .collect();
        let half = Point::new(self.width / 2.0, self.height / 2.0);
        Bitmap {
            name: name.to_owned(),
            columns: self.columns,
            rows: self.rows,
            grey,
            rect: Rect::new(centre - half, centre + half),
            depth: self.heights.clone(),
            rgb: Vec::new(),
        }
    }
}

/// Reads the run's final report and its height map.
pub fn read_made(report: &Value, heightmap: &[u8]) -> Result<Made, String> {
    let map = image::load_from_memory(heightmap)
        .map_err(|e| format!("Depth to STL's height map does not read: {e}"))?
        .into_luma16();
    let (columns, rows) = (map.width() as usize, map.height() as usize);
    let number = |key: &str| {
        report[key]
            .as_f64()
            .filter(|v| v.is_finite() && *v > 0.0)
            .ok_or_else(|| format!("Depth to STL's answer has no {key}"))
    };
    if columns < 2 || rows < 2 {
        return Err("Depth to STL made an empty height map".into());
    }
    Ok(Made {
        heights: map
            .into_raw()
            .into_iter()
            .map(|v| f32::from(v) / 65_535.0)
            .collect(),
        columns,
        rows,
        width: number("width_mm")?,
        height: number("height_mm")?,
        top: number("top_mm")?,
        bit: report["bit"].as_str().map(str::to_owned),
    })
}

/// What a run says as it goes.
#[derive(Clone, Debug, PartialEq)]
pub enum Said {
    /// A step done: its name, and how many of how many.
    Step(String, u64, u64),
    Done(Result<Made, String>),
}

/// A run in the background, for the picture `bitmap`.
pub struct Running {
    pub bitmap: EntityId,
    /// The picture as it was sent, to place the answer on.
    pub photo: Bitmap,
    /// The newest step it reported.
    pub step: Option<(String, u64, u64)>,
    said: mpsc::Receiver<Said>,
}

impl Running {
    /// Starts the add-on `program` on `bitmap` (`id`) with `settings`; `ctx` is woken as it
    /// reports.
    pub fn start(
        program: &Path,
        id: EntityId,
        bitmap: &Bitmap,
        settings: &Value,
        ctx: &eframe::egui::Context,
    ) -> Result<Self, String> {
        let (input, render) = inputs(bitmap)?;
        let (send, said) = mpsc::channel();
        let (program, settings, ctx) = (program.to_path_buf(), settings.clone(), ctx.clone());
        std::thread::Builder::new()
            .name("depth to stl".into())
            .spawn(move || {
                let tell = |s: Said| {
                    let _ = send.send(s);
                    ctx.request_repaint();
                };
                let made = run(&program, &input, render.as_deref(), &settings, &|s| tell(s));
                tell(Said::Done(made));
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            bitmap: id,
            photo: bitmap.clone(),
            step: None,
            said,
        })
    }

    /// The answer once there is one; keeps [`Running::step`] up to date meanwhile.
    pub fn answer(&mut self) -> Option<Result<Made, String>> {
        loop {
            match self.said.try_recv() {
                Ok(Said::Step(name, at, of)) => self.step = Some((name, at, of)),
                Ok(Said::Done(made)) => return Some(made),
                Err(mpsc::TryRecvError::Empty) => return None,
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Some(Err("Depth to STL stopped unexpectedly".into()));
                }
            }
        }
    }
}

/// Runs the add-on on `input` (a PNG), through files in the temporary folder, telling
/// `said` each step.
pub fn run(
    program: &Path,
    input: &[u8],
    render: Option<&[u8]>,
    settings: &Value,
    said: &dyn Fn(Said),
) -> Result<Made, String> {
    let stem = format!(
        "cutcraft-depth2stl-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    )
    .replace(['(', ')'], "");
    let dir = std::env::temp_dir();
    let file = |end: &str| dir.join(format!("{stem}-{end}"));
    let (input_path, render_path, out) = (file("in.png"), file("photo.png"), file("height.png"));
    let result = (|| {
        std::fs::write(&input_path, input)
            .map_err(|e| format!("cannot write {}: {e}", input_path.display()))?;
        let mut job = json!({
            "api": API,
            "input": input_path,
            "heightmap": out,
            "settings": settings,
        });
        if let Some(render) = render {
            std::fs::write(&render_path, render)
                .map_err(|e| format!("cannot write {}: {e}", render_path.display()))?;
            job["render"] = json!(render_path);
        }
        let report = talk(program, &job, said)?;
        let map =
            std::fs::read(&out).map_err(|e| format!("Depth to STL gave no height map: {e}"))?;
        read_made(&report, &map)
    })();
    for path in [&input_path, &render_path, &out] {
        let _ = std::fs::remove_file(path);
    }
    result
}

/// Sends `job` to `program api run` and reads its steps and final report.
fn talk(program: &Path, job: &Value, said: &dyn Fn(Said)) -> Result<Value, String> {
    let mut child = command(program)
        .args(["api", "run"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot run Depth to STL ({}): {e}", program.display()))?;
    {
        let mut stdin = child.stdin.take().expect("piped");
        stdin
            .write_all(job.to_string().as_bytes())
            .map_err(|e| format!("cannot send Depth to STL its job: {e}"))?;
    }
    let mut last = None;
    for line in std::io::BufReader::new(child.stdout.take().expect("piped")).lines() {
        let Ok(line) = line else { break };
        let Ok(v) = serde_json::from_str::<Value>(&line) else {
            continue; // anything else it prints is not for us
        };
        if let (Some(stage), Some(at), Some(of)) =
            (v["stage"].as_str(), v["step"].as_u64(), v["of"].as_u64())
        {
            said(Said::Step(stage.to_owned(), at, of));
        } else if v.get("ok").is_some() {
            last = Some(v);
        }
    }
    let finished = child
        .wait_with_output()
        .map_err(|e| format!("Depth to STL did not finish: {e}"))?;
    match last {
        Some(v) if v["ok"] == true => Ok(v),
        Some(v) => Err(format!(
            "Depth to STL: {}",
            v["error"].as_str().unwrap_or("it failed")
        )),
        None => Err(format!(
            "Depth to STL stopped without an answer ({}): {}",
            finished.status,
            String::from_utf8_lossy(&finished.stderr).trim()
        )),
    }
}

/// The Model ▸ Depth to STL window's state.
#[derive(Default)]
pub struct Window {
    pub open: bool,
    /// The add-on's program, found when the window opened.
    pub program: Option<PathBuf>,
    /// Its description, once it has given it.
    pub described: Option<Result<Description, String>>,
    describing: Option<mpsc::Receiver<Result<Description, String>>>,
    /// The settings as the form shows them.
    pub settings: Value,
    /// The picture the width was last taken from.
    pub sized_for: Option<EntityId>,
    pub running: Option<Running>,
    /// What the last run made or said.
    pub message: Option<String>,
}

impl Window {
    /// Opens it for the add-on in `addons`, asking the add-on to describe itself afresh
    /// (it may have been updated since).
    pub fn show(&mut self, addons: Option<&Path>, ctx: &eframe::egui::Context) {
        self.open = true;
        self.program = addons.and_then(find);
        self.described = None;
        self.describing = None;
        let Some(program) = self.program.clone() else {
            return;
        };
        let (send, answer) = mpsc::channel();
        let ctx = ctx.clone();
        let spawned = std::thread::Builder::new()
            .name("depth to stl describe".into())
            .spawn(move || {
                let _ = send.send(describe(&program));
                ctx.request_repaint();
            });
        match spawned {
            Ok(_) => self.describing = Some(answer),
            Err(e) => self.described = Some(Err(e.to_string())),
        }
    }

    /// Takes in the description once it comes, keeping settings already chosen that it
    /// still has.
    pub fn take_description(&mut self) {
        let Some(answer) = self.describing.as_ref().and_then(|rx| rx.try_recv().ok()) else {
            return;
        };
        self.describing = None;
        if let Ok(d) = &answer {
            let mut settings = d.defaults();
            if let (Some(now), Some(kept)) = (settings.as_object_mut(), self.settings.as_object()) {
                for (k, v) in kept {
                    if let Some(slot) = now.get_mut(k) {
                        *slot = v.clone();
                    }
                }
            }
            self.settings = settings;
        }
        self.described = Some(answer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DESCRIBED: &str = r#"{"api": 1, "id": "depth2stl", "version": "1.1.0",
        "settings": {"type": "object", "properties": {
            "width": {"type": "number", "default": 300.0, "x-unit": "mm"},
            "invert": {"type": "boolean", "default": false}}}}"#;

    #[test]
    fn reads_a_description_and_its_defaults() {
        let d = Description::parse(DESCRIBED).unwrap();
        assert_eq!(d.version, "1.1.0");
        assert_eq!(d.defaults(), json!({"width": 300.0, "invert": false}));
        assert_eq!(crate::form::fields(&d.settings).len(), 2, "a form draws it");
        let newer = DESCRIBED.replace(r#""api": 1"#, r#""api": 2"#);
        assert!(Description::parse(&newer).unwrap_err().contains("API 2"));
        assert!(Description::parse("not json").is_err());
    }

    fn picture(depth: Vec<f32>, rgb: Vec<u8>) -> Bitmap {
        Bitmap {
            name: "p".into(),
            columns: 2,
            rows: 1,
            grey: vec![0, 255],
            rect: Rect::new(Point::ZERO, Point::new(20.0, 10.0)),
            depth,
            rgb,
        }
    }

    #[test]
    fn sends_depth_at_full_precision_and_the_photo_beside_it() {
        let read = |png: &[u8]| {
            image::load_from_memory(png)
                .unwrap()
                .into_luma16()
                .into_raw()
        };
        let (grey, render) = inputs(&picture(vec![], vec![])).unwrap();
        assert_eq!((read(&grey), render), (vec![0, 65_535], None));
        let fine = 0.5 + 1.0 / 65_535.0;
        let (deep, render) = inputs(&picture(vec![0.5, fine], vec![9; 6])).unwrap();
        let levels = read(&deep);
        assert_eq!(levels[1] - levels[0], 1, "finer than a grey level");
        assert!(render.is_some(), "the photo's colours for fine detail");
    }

    #[test]
    fn its_answer_becomes_a_picture_centred_on_the_source() {
        let mut png = Vec::new();
        image::ImageBuffer::<image::Luma<u16>, _>::from_raw(2, 2, vec![0u16, 65_535, 32_768, 1])
            .unwrap()
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let report = json!({"ok": true, "width_mm": 40.0, "height_mm": 20.0, "top_mm": 11.0,
                            "bit": "all detail is carvable with this bit"});
        let made = read_made(&report, &png).unwrap();
        assert_eq!((made.columns, made.rows, made.top), (2, 2, 11.0));
        let b = made.bitmap("relief", Point::new(10.0, 5.0));
        assert_eq!(
            b.rect,
            Rect::new(Point::new(-10.0, -5.0), Point::new(30.0, 15.0))
        );
        assert_eq!(b.grey, [0, 255, 128, 0]);
        assert!(
            (b.tone(Point::new(-9.9, 14.9)).unwrap()).abs() < 1e-3,
            "top left low"
        );
        assert!(
            read_made(&json!({"ok": true}), &png)
                .unwrap_err()
                .contains("width_mm")
        );
    }

    /// The real add-on, built from github.com/NoobyNull/depth2stl, named by
    /// `CUTCRAFT_DEPTH2STL`: describes itself and makes a relief from a round blob.
    #[test]
    #[ignore = "needs a built depth2stl; set CUTCRAFT_DEPTH2STL to it"]
    fn runs_the_real_add_on() {
        let program = PathBuf::from(std::env::var("CUTCRAFT_DEPTH2STL").unwrap());
        let described = describe(&program).unwrap();
        let fields = crate::form::fields(&described.settings);
        assert!(
            fields.iter().any(|f| f.key == "tip" && f.help.is_some()),
            "{fields:?}"
        );
        let (columns, rows) = (120u16, 80u16);
        let depth = (0..rows)
            .flat_map(|r| (0..columns).map(move |c| (c, r)))
            .map(|(c, r)| {
                let (x, y) = (f32::from(c) - 60.0, f32::from(r) - 40.0);
                if x * x + y * y < 30.0 * 30.0 {
                    0.6 + x / 200.0
                } else {
                    0.0
                }
            })
            .collect::<Vec<f32>>();
        let picture = Bitmap {
            name: "blob".into(),
            columns: columns.into(),
            rows: rows.into(),
            grey: vec![0; usize::from(columns) * usize::from(rows)],
            rect: Rect::new(Point::ZERO, Point::new(60.0, 40.0)),
            depth,
            rgb: Vec::new(),
        };
        let (input, render) = inputs(&picture).unwrap();
        let mut settings = described.defaults();
        settings["width"] = json!(30.0);
        let steps = std::cell::RefCell::new(0);
        let made = run(&program, &input, render.as_deref(), &settings, &|_| {
            *steps.borrow_mut() += 1;
        })
        .unwrap();
        assert!(*steps.borrow() > 3, "it reports as it goes");
        assert!(
            (made.width - 30.0).abs() < 1e-9 && made.columns > 30,
            "{made:?}"
        );
        assert!(made.heights.iter().copied().fold(0.0, f32::max) > 0.5);
        assert_eq!(made.heights[0], 0.0, "the background cut away");
        settings["tip"] = json!(-1);
        let refused = run(&program, &input, None, &settings, &|_| {}).unwrap_err();
        assert!(refused.contains("tip"), "{refused}");
    }

    /// A stand-in add-on: a shell script answering as depth2stl does.
    #[cfg(unix)]
    #[test]
    fn runs_the_add_on_and_reads_its_steps() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = std::env::temp_dir().join(format!("cutcraft-d2s-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // copies the input over the height map named in the job
        let script = r#"#!/bin/sh
[ "$1 $2" = "api run" ] || exit 2
job=$(cat)
input=$(printf '%s' "$job" | sed 's/.*"input":"\([^"]*\)".*/\1/')
out=$(printf '%s' "$job" | sed 's/.*"heightmap":"\([^"]*\)".*/\1/')
echo "loading stuff"
echo '{"stage": "floor cut", "step": 1, "of": 2}'
cp "$input" "$out"
echo '{"stage": "written", "step": 2, "of": 2}'
echo '{"ok": true, "width_mm": 20, "height_mm": 10, "top_mm": 5}'
"#;
        let program = dir.join("depth2stl");
        std::fs::write(&program, script).unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
        let square = Bitmap {
            rows: 2,
            grey: vec![0, 255, 0, 255],
            ..picture(vec![], vec![])
        };
        let (input, _) = inputs(&square).unwrap();
        let steps = std::cell::RefCell::new(vec![]);
        let made = run(&program, &input, None, &json!({}), &|s| {
            steps.borrow_mut().push(s);
        })
        .unwrap();
        assert_eq!(made.heights, [0.0, 1.0, 0.0, 1.0]);
        assert_eq!(steps.borrow()[1], Said::Step("written".into(), 2, 2));
        std::fs::write(
            &program,
            "#!/bin/sh\ncat >/dev/null\necho '{\"ok\": false, \"error\": \"no luck\"}'\nexit 1\n",
        )
        .unwrap();
        let e = run(&program, &input, None, &json!({}), &|_| {}).unwrap_err();
        assert_eq!(e, "Depth to STL: no luck");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
