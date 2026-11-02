//! `cutcraft-depth WEIGHTS PHOTO OUT.png`: the photo's estimated depth as a 16-bit grey
//! picture the photo's size, white nearest. Cut Craft runs it; it can be run by hand too.
//! `cutcraft-depth convert depth_anything_v2_vits.pth OUT.safetensors` makes the shipped
//! weights from the official file.

use cut_craft_depth::{Estimator, to_grey16};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    let done = match args.as_slice() {
        [command, pth, out] if command.as_os_str() == "convert" => {
            cut_craft_depth::convert(pth, out).map_err(|e| format!("conversion failed: {e}"))
        }
        [weights, photo, out] => run(weights, photo, out),
        _ => {
            eprintln!(
                "usage: cutcraft-depth WEIGHTS PHOTO OUT.png\n       cutcraft-depth convert IN.pth OUT.safetensors"
            );
            return ExitCode::from(2);
        }
    };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run(weights: &Path, photo: &Path, out: &Path) -> Result<(), String> {
    let estimator = Estimator::load(weights).map_err(|e| {
        format!(
            "could not load the depth model from {}: {e}. Reinstall the Depth add-on.",
            weights.display()
        )
    })?;
    let picture = image::open(photo)
        .map_err(|e| format!("could not open the photo {}: {e}", photo.display()))?
        .to_rgb8();
    let (columns, rows) = picture.dimensions();
    let depth = estimator
        .estimate(picture.as_raw(), columns, rows)
        .map_err(|e| format!("depth estimation failed: {e}"))?;
    to_grey16(&depth, columns, rows)
        .save(out)
        .map_err(|e| format!("could not write {}: {e}", out.display()))
}
