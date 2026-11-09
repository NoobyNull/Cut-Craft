//! The Staunton chess set content pack: a full set modelled from scratch on the classic
//! 1849 Staunton pattern (long out of copyright) and given away under CC0, so anyone may
//! carve and sell it.
//!
//! The turned pieces come as halves to cut from 3/4″ wood and glue face to face. The
//! knight also comes whole, for cutting from both sides of thicker wood with alignment
//! pins: the pack's tutorial job.

pub mod mesh;
pub mod profile;
pub mod shape;
pub mod tutorial;

pub use mesh::{Cut, mesh};
pub use profile::Piece;
pub use shape::Shape;

use cut_craft_content_pack::{Item, Licence, Pack};

/// The pack's id: the folder it installs into.
pub const ID: &str = "staunton-chess";
/// How finely the pieces are sampled, mm: well under a 1/16″ ball nose's radius.
pub const STEP: f64 = 0.3;

/// A file of the pack: where it goes, its Library item, and its bytes.
pub struct File {
    pub item: Item,
    pub bytes: Vec<u8>,
}

fn model(piece: Piece, cut: Cut) -> File {
    let (r, h) = piece.size();
    let name = piece.name();
    let (file, title, description) = match cut {
        Cut::Half => (
            format!("{name} half.stl"),
            format!("{name} (half)"),
            format!(
                "Half a {h:.0} mm {lower}, lying flat. Cut two from {thick:.0} mm (3/4″) wood \
                 and glue them face to face; {count} make one colour's {lower}s.",
                lower = name.to_lowercase(),
                thick = r.max(19.0),
                count = 2 * piece.count(),
            ),
        ),
        Cut::Whole => (
            format!("{name} two-sided.stl"),
            format!("{name} (whole, two-sided)"),
            format!(
                "The whole {h:.0} mm {lower}, lying on its side, {t:.0} mm thick: cut from both \
                 sides of 1½″ (38 mm) wood. Open the tutorial job to see how.",
                lower = name.to_lowercase(),
                t = 2.0 * r,
            ),
        ),
    };
    let bytes = fmt_export_as_stl::binary_stl(&mesh(&Shape::new(piece), cut, STEP));
    let mut item = Item::new(&format!("Pieces/{file}"), &title, &bytes);
    item.description = description;
    item.tags = vec![
        "chess".to_owned(),
        "staunton".to_owned(),
        name.to_lowercase(),
        match cut {
            Cut::Half => "glue-up",
            Cut::Whole => "two-sided",
        }
        .to_owned(),
    ];
    File { item, bytes }
}

/// The tutorial job, saved.
fn tutorial_job() -> Result<File, String> {
    let bytes = cut_craft_core::file::save_job(&tutorial::job()?);
    let mut item = Item::new(
        "Tutorial - Two-sided knight.ccjob",
        "Tutorial: cut a knight from both sides",
        &bytes,
    );
    "A ready-made two-sided job: the knight cut from both faces of 1½″ \
        wood, lined up with two dowel pins. The steps open with the job."
        .clone_into(&mut item.description);
    item.tags = ["chess", "knight", "two-sided", "tutorial"]
        .map(str::to_owned)
        .to_vec();
    Ok(File { item, bytes })
}

/// Every file of the pack, with the pack's `pack.json` to list them.
pub fn files() -> Result<(Pack, Vec<File>), String> {
    let mut files = vec![tutorial_job()?];
    files.extend(Piece::ALL.iter().map(|&p| model(p, Cut::Half)));
    files.push(model(Piece::Knight, Cut::Whole));
    let mut pack = Pack::new(ID, "Staunton Chess Set", Licence::Cc0, "Cut Craft");
    "A full chess set in the classic Staunton pattern: halves to cut from \
        3/4″ wood and glue, and a knight to cut from both sides as a first two-sided job. \
        Free to carve and sell (CC0)."
        .clone_into(&mut pack.description);
    pack.homepage = Some("https://github.com/NoobyNull/Cut-Craft".to_owned());
    pack.items = files.iter().map(|f| f.item.clone()).collect();
    Ok((pack, files))
}

/// Writes the pack into `dir`: its files, `pack.json`, and the `addon.toml` the add-on
/// installer installs it by.
pub fn write(dir: &std::path::Path) -> Result<Pack, String> {
    let (pack, files) = files()?;
    let io = |e: std::io::Error| e.to_string();
    for file in &files {
        let path = file.item.path(dir);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(io)?;
        }
        std::fs::write(path, &file.bytes).map_err(io)?;
    }
    std::fs::write(dir.join(cut_craft_content_pack::MANIFEST), pack.to_json()).map_err(io)?;
    std::fs::write(
        dir.join(cut_craft_content_pack::ADDON_MANIFEST),
        pack.addon_toml(),
    )
    .map_err(io)?;
    Ok(pack)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pack_is_written_whole_and_reads_back() {
        let dir = std::env::temp_dir().join(format!("cc-staunton-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let pack = write(&dir).unwrap();
        let json = std::fs::read(dir.join(cut_craft_content_pack::MANIFEST)).unwrap();
        assert_eq!(Pack::parse(&json).unwrap(), pack);
        assert_eq!(pack.verify(&dir), []);
        let addon = std::fs::read_to_string(dir.join("addon.toml")).unwrap();
        assert!(
            addon.contains("kind = \"content\"") && addon.contains(ID),
            "{addon}"
        );
        assert_eq!(pack.licence, Licence::Cc0);
        assert_eq!(pack.credits(), Vec::<String>::new(), "CC0 needs no credits");
        // a tutorial, a half of each piece, and the whole knight
        assert_eq!(pack.items.len(), 1 + Piece::ALL.len() + 1);
        assert!(
            pack.items
                .iter()
                .all(|i| i.tags.contains(&"chess".to_owned()))
        );
        let job = std::fs::read(dir.join("Tutorial - Two-sided knight.ccjob")).unwrap();
        assert!(cut_craft_core::file::load_job(&job).is_ok());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
