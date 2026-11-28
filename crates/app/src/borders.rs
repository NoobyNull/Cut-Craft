//! The Borders add-on, seen from the app: whether it is installed, the person's own Google
//! key (kept in the computer's password store, never in a job, a setting or a crash
//! report), where generated borders are saved, and Generate Border's Generate, which runs
//! the add-on in the background and saves what comes back to the Library under
//! Generated ▸ Borders as a depth map, its photo and its recipe
//! (`cut_craft_border_tile`).

use cmd_generate_border::{DepthFrom, Params};
use cut_craft_border_tile as tile;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc;

/// The add-on's id: its folder under the add-ons folder.
pub const ADDON_ID: &str = "borders";
/// The environment variable the add-on reads the key from (never its command line, where
/// other programs could read it).
pub const KEY_VARIABLE: &str = "CUTCRAFT_GOOGLE_API_KEY";
/// Where people get a key of their own.
pub const KEY_PAGE: &str = "https://aistudio.google.com/apikey";

/// The add-on's program file name on this system.
pub fn program_name() -> String {
    format!("cutcraft-borders{}", std::env::consts::EXE_SUFFIX)
}

/// The installed add-on's program in `addons` (the add-ons folder), if it is there.
pub fn find(addons: &Path) -> Option<PathBuf> {
    let program = addons.join(ADDON_ID).join(program_name());
    program.is_file().then_some(program)
}

/// The folder added to the Library for what PowerWhittle makes: Documents/CutCraft, whose
/// Generated ▸ Borders holds the borders.
pub fn library_root() -> Option<PathBuf> {
    let home = if cfg!(windows) {
        std::env::var_os("USERPROFILE")
    } else {
        std::env::var_os("HOME")
    }?;
    Some(PathBuf::from(home).join("Documents").join("CutCraft"))
}

/// Where generated borders are saved, under `root` ([`library_root`]).
pub fn folder(root: &Path) -> PathBuf {
    tile::FOLDER
        .iter()
        .fold(root.to_path_buf(), |p, f| p.join(f))
}

/// Where the person's Google key is kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyStore {
    /// The computer's password store (Windows Credential Manager, the macOS Keychain, the
    /// Linux Secret Service), falling back to `file` when it has none.
    System { file: Option<PathBuf> },
    /// Only this file: tests and the help's pictures, never the computer's own password
    /// store.
    File(PathBuf),
}

/// Where a key was kept, for the form to say.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kept {
    PasswordStore,
    PrivateFile,
}

const SERVICE: &str = "Cut Craft";
const ACCOUNT: &str = "Google AI Studio key";

impl KeyStore {
    /// The computer's password store, with a private file in PowerWhittle's settings folder
    /// if it has none.
    pub fn system() -> Self {
        Self::System {
            file: crate::files::config_dir().map(|d| d.join("google-key")),
        }
    }

    fn entry() -> Option<keyring::Entry> {
        keyring::Entry::new(SERVICE, ACCOUNT).ok()
    }

    fn file(&self) -> Option<&Path> {
        match self {
            Self::System { file } => file.as_deref(),
            Self::File(file) => Some(file),
        }
    }

    /// The key kept, if any.
    pub fn load(&self) -> Option<String> {
        if matches!(self, Self::System { .. })
            && let Some(key) = Self::entry().and_then(|e| e.get_password().ok())
            && !key.trim().is_empty()
        {
            return Some(key.trim().to_owned());
        }
        let key = std::fs::read_to_string(self.file()?).ok()?;
        let key = key.trim();
        (!key.is_empty()).then(|| key.to_owned())
    }

    /// Keeps `key` (trimmed), in the password store when there is one.
    pub fn save(&self, key: &str) -> Result<Kept, String> {
        let key = key.trim();
        if key.is_empty() {
            return self.forget().map(|()| Kept::PasswordStore);
        }
        if matches!(self, Self::System { .. })
            && let Some(entry) = Self::entry()
            && entry.set_password(key).is_ok()
        {
            // only one copy: none left behind in a file from before
            if let Some(file) = self.file() {
                let _ = std::fs::remove_file(file);
            }
            return Ok(Kept::PasswordStore);
        }
        let file = self
            .file()
            .ok_or("this computer has nowhere to keep the key")?;
        write_private(file, key)?;
        Ok(Kept::PrivateFile)
    }

    /// Forgets the key, wherever it was kept.
    pub fn forget(&self) -> Result<(), String> {
        if matches!(self, Self::System { .. })
            && let Some(entry) = Self::entry()
        {
            let _ = entry.delete_credential();
        }
        match self.file() {
            Some(file) if file.exists() => std::fs::remove_file(file)
                .map_err(|e| format!("cannot remove {}: {e}", file.display())),
            _ => Ok(()),
        }
    }
}

/// `text` written to `file` so only this user can read it.
fn write_private(file: &Path, text: &str) -> Result<(), String> {
    use std::io::Write as _;
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("cannot make {}: {e}", dir.display()))?;
    }
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
        .open(file)
        .and_then(|mut f| f.write_all(text.as_bytes()))
        .map_err(|e| format!("cannot save the key in {}: {e}", file.display()))
}

/// What a generation needs, gathered before it starts.
#[derive(Clone, Debug)]
pub struct Job {
    pub program: PathBuf,
    pub key: String,
    /// The Depth add-on, for depth worked out on this computer.
    pub depth: Option<crate::depth::Installed>,
    pub params: Params,
    /// Where it is saved ([`folder`]).
    pub folder: PathBuf,
}

/// How far a generation has got.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Picture,
    Depth,
    Saving,
}

impl Stage {
    fn from_u8(n: u8) -> Self {
        match n {
            0 => Self::Picture,
            1 => Self::Depth,
            _ => Self::Saving,
        }
    }

    /// Its words' key (`locales/en/app.ftl`).
    pub fn key(self) -> &'static str {
        match self {
            Self::Picture => "border-asking-picture",
            Self::Depth => "border-asking-depth",
            Self::Saving => "border-saving",
        }
    }
}

/// A program run with these arguments and, when given, the key in the environment.
pub type Run<'a> = dyn Fn(&Path, &[OsString], Option<&str>) -> Result<(), String> + 'a;

/// Makes the border `job` asks for, telling `stage` how far it has got: the picture from
/// the add-on, its depth (from the add-on, or the Depth add-on), shaped and saved.
pub fn make(job: &Job, run: &Run<'_>, stage: &dyn Fn(Stage)) -> Result<tile::Files, String> {
    let p = &job.params;
    p.check().map_err(|e| e.to_string())?;
    let scratch = Scratch::new()?;
    let picture = scratch.path("picture.img");
    let depth_out = scratch.path("depth.png");
    let ratio = p.shape.ratio();
    stage(Stage::Picture);
    run(
        &job.program,
        &[
            "picture".into(),
            p.style.id().into(),
            ratio.into(),
            p.details.trim().into(),
            picture.clone().into(),
        ],
        Some(&job.key),
    )?;
    let picture_bytes =
        std::fs::read(&picture).map_err(|e| format!("the add-on gave no picture: {e}"))?;
    stage(Stage::Depth);
    match p.depth_from {
        DepthFrom::Google => run(
            &job.program,
            &[
                "depth".into(),
                picture.clone().into(),
                ratio.into(),
                depth_out.clone().into(),
            ],
            Some(&job.key),
        )?,
        DepthFrom::DepthAddOn => {
            let depth = job.depth.as_ref().ok_or(
                "the Depth add-on is not installed: install it (Gadgets ▸ Add-ons), or set \
                 Depth from to Google",
            )?;
            // the Depth add-on reads a PNG
            let png = scratch.path("picture.png");
            image::load_from_memory(&picture_bytes)
                .and_then(|i| i.save(&png))
                .map_err(|e| format!("the add-on's picture does not read: {e}"))?;
            run(
                &depth.program,
                &[
                    depth.weights.clone().into(),
                    png.into(),
                    depth_out.clone().into(),
                ],
                None,
            )?;
        }
    }
    let depth_bytes =
        std::fs::read(&depth_out).map_err(|e| format!("the add-on gave no depth map: {e}"))?;
    stage(Stage::Saving);
    let (greys, w, h) = tile::greys(&depth_bytes)?;
    let shaping = tile::Shaping {
        curve: p.depth_curve,
        smooth: p.smoothing / 100.0,
        seam: p.seam_blend / 100.0,
    };
    let shaped = tile::shape(&greys, w, h, &shaping);
    let mut recipe = tile::Recipe::new(p.style.id(), ratio, p.length, p.height(), p.depth);
    p.details.trim().clone_into(&mut recipe.details);
    recipe.shaping = shaping;
    recipe.depth_from = p.depth_from.id().into();
    recipe.made = chrono::Local::now().to_rfc3339();
    let base = if p.name.trim().is_empty() {
        "Border"
    } else {
        p.name.trim()
    };
    let name = tile::free_name(&job.folder, base);
    tile::save(&job.folder, &name, &shaped, w, h, &picture_bytes, &recipe)
}

/// A folder of its own in the temporary folder, removed with everything in it when done.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Result<Self, String> {
        let stem = format!(
            "cutcraft-border-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        )
        .replace(['(', ')'], "");
        let dir = std::env::temp_dir().join(stem);
        std::fs::create_dir_all(&dir).map_err(|e| format!("cannot make {}: {e}", dir.display()))?;
        Ok(Self(dir))
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Runs `program` with `args`, the key (if any) in its environment only, with no console
/// window flashing up; what it said on failure is the error.
pub fn run_program(program: &Path, args: &[OsString], key: Option<&str>) -> Result<(), String> {
    let mut command = std::process::Command::new(program);
    command.args(args);
    match key {
        Some(key) => command.env(KEY_VARIABLE, key),
        None => command.env_remove(KEY_VARIABLE),
    };
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let name = program
        .file_stem()
        .map_or_else(|| "the add-on".into(), |s| s.to_string_lossy().into_owned());
    let finished = command
        .output()
        .map_err(|e| format!("cannot run {name} ({}): {e}", program.display()))?;
    if finished.status.success() {
        return Ok(());
    }
    let said = String::from_utf8_lossy(&finished.stderr);
    let said = said.trim();
    Err(if said.is_empty() {
        format!("{name} stopped without saying why ({})", finished.status)
    } else {
        said.to_owned()
    })
}

/// A generation running in the background.
pub struct Generating {
    stage: Arc<AtomicU8>,
    answer: mpsc::Receiver<Result<tile::Files, String>>,
}

impl Generating {
    /// Starts `job`; `ctx` is woken as it goes.
    pub fn start(job: Job, ctx: &eframe::egui::Context) -> Result<Self, String> {
        let stage = Arc::new(AtomicU8::new(0));
        let (send, answer) = mpsc::channel();
        let (shared, ctx) = (stage.clone(), ctx.clone());
        cut_craft_core::work::background("generate border", move || {
            // the add-on's own program does the calculating; this thread only waits on it
            let waited = |program: &Path, args: &[OsString], key: Option<&str>| {
                cut_craft_core::work::waiting(|| run_program(program, args, key))
            };
            let made = make(&job, &waited, &|s| {
                shared.store(s as u8, Ordering::Relaxed);
                ctx.request_repaint();
            });
            let _ = send.send(made);
            ctx.request_repaint();
        })
        .map_err(|e| e.to_string())?;
        Ok(Self { stage, answer })
    }

    /// How far it has got.
    pub fn stage(&self) -> Stage {
        Stage::from_u8(self.stage.load(Ordering::Relaxed))
    }

    /// The answer once there is one.
    pub fn answer(&self) -> Option<Result<tile::Files, String>> {
        match self.answer.try_recv() {
            Ok(answer) => Some(answer),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err("generating the border stopped unexpectedly".into()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cutcraft-borders-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn png(w: u32, h: u32, grey: impl Fn(u32, u32) -> u8) -> Vec<u8> {
        let image = image::GrayImage::from_fn(w, h, |x, y| image::Luma([grey(x, y)]));
        let mut bytes = Vec::new();
        image
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .unwrap();
        bytes
    }

    #[test]
    fn the_add_on_is_found_once_its_program_is_there() {
        let addons = temp("find");
        assert_eq!(find(&addons), None);
        std::fs::create_dir_all(addons.join(ADDON_ID)).unwrap();
        std::fs::write(addons.join(ADDON_ID).join(program_name()), b"").unwrap();
        assert_eq!(
            find(&addons),
            Some(addons.join(ADDON_ID).join(program_name()))
        );
        std::fs::remove_dir_all(addons).unwrap();
    }

    #[test]
    fn borders_go_under_generated_borders() {
        let root = Path::new("/home/me/Documents/CutCraft");
        assert_eq!(folder(root), root.join("Generated").join("Borders"));
    }

    #[test]
    fn a_key_kept_in_a_file_is_private_and_can_be_forgotten() {
        let dir = temp("key");
        let store = KeyStore::File(dir.join("google-key"));
        assert_eq!(store.load(), None);
        assert_eq!(store.save("  my-key \n"), Ok(Kept::PrivateFile));
        assert_eq!(store.load().as_deref(), Some("my-key"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(dir.join("google-key"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o077, 0, "only its owner may read it");
        }
        store.forget().unwrap();
        assert_eq!(store.load(), None);
        std::fs::remove_dir_all(dir).unwrap();
    }

    fn job(folder: &Path, depth_from: DepthFrom) -> Job {
        Job {
            program: PathBuf::from("cutcraft-borders"),
            key: "secret".into(),
            depth: Some(crate::depth::Installed {
                program: PathBuf::from("cutcraft-depth"),
                weights: PathBuf::from("weights.safetensors"),
                detailed: None,
            }),
            params: Params {
                name: "Baroque".into(),
                details: "laurel leaves".into(),
                length: 160.0,
                depth_from,
                ..Params::default()
            },
            folder: folder.to_path_buf(),
        }
    }

    /// A stand-in for both add-ons: writes a picture or a depth map to the last argument,
    /// and notes what it was asked and whether it was given the key.
    fn fake(
        asked: &RefCell<Vec<(String, Vec<String>, bool)>>,
    ) -> impl Fn(&Path, &[OsString], Option<&str>) -> Result<(), String> + '_ {
        move |program, args, key| {
            let args: Vec<String> = args
                .iter()
                .map(|a| a.to_string_lossy().into_owned())
                .collect();
            let out = args.last().unwrap();
            let bytes = if args[0] == "picture" {
                png(32, 18, |x, _| u8::try_from(x * 8).unwrap())
            } else {
                png(32, 18, |_, y| u8::try_from(y * 14).unwrap())
            };
            std::fs::write(out, bytes).unwrap();
            asked
                .borrow_mut()
                .push((program.display().to_string(), args, key == Some("secret")));
            Ok(())
        }
    }

    #[test]
    fn generate_asks_for_the_picture_then_its_depth_and_saves_all_three() {
        let dir = temp("make");
        let asked = RefCell::new(Vec::new());
        let stages = RefCell::new(Vec::new());
        let files = make(&job(&dir, DepthFrom::Google), &fake(&asked), &|s| {
            stages.borrow_mut().push(s);
        })
        .unwrap();
        let asked = asked.into_inner();
        assert_eq!(asked.len(), 2);
        assert_eq!(
            asked[0].1[..4],
            ["picture", "classical_baroque", "16:9", "laurel leaves"]
        );
        assert_eq!(asked[1].1[0], "depth");
        assert!(
            asked.iter().all(|a| a.2),
            "the key goes to the add-on both times"
        );
        assert_eq!(
            *stages.borrow(),
            [Stage::Picture, Stage::Depth, Stage::Saving]
        );
        assert_eq!(files.depth, dir.join("Baroque.png"));
        assert!(files.picture.is_file() && files.recipe.is_file());
        let recipe = tile::read_recipe(&files.recipe).unwrap();
        assert_eq!((recipe.width, recipe.height), (160.0, 90.0));
        assert_eq!(recipe.details, "laurel leaves");
        let again = make(
            &job(&dir, DepthFrom::Google),
            &fake(&RefCell::default()),
            &|_| {},
        )
        .unwrap();
        assert_eq!(
            again.depth,
            dir.join("Baroque 2.png"),
            "never over one made before"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn depth_on_this_computer_uses_the_depth_add_on_without_the_key() {
        let dir = temp("local");
        let asked = RefCell::new(Vec::new());
        let files = make(&job(&dir, DepthFrom::DepthAddOn), &fake(&asked), &|_| {}).unwrap();
        let asked = asked.into_inner();
        assert_eq!(asked[1].0, "cutcraft-depth");
        assert_eq!(asked[1].1[0], "weights.safetensors");
        assert!(!asked[1].2, "the key never goes to the Depth add-on");
        assert_eq!(
            tile::read_recipe(&files.recipe).unwrap().depth_from,
            "depth_add_on"
        );
        let mut without = job(&dir, DepthFrom::DepthAddOn);
        without.depth = None;
        let refused = make(&without, &fake(&RefCell::default()), &|_| {}).unwrap_err();
        assert!(refused.contains("Depth add-on"), "{refused}");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn what_the_add_on_said_is_the_error() {
        let dir = temp("refused");
        let refusing = |_: &Path, _: &[OsString], _: Option<&str>| {
            Err("Google turned the key down: check it in the form's Google key box".to_owned())
        };
        let said = make(&job(&dir, DepthFrom::Google), &refusing, &|_| {}).unwrap_err();
        assert!(said.contains("Google key box"), "{said}");
        assert!(
            std::fs::read_dir(&dir).unwrap().next().is_none(),
            "nothing saved"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
