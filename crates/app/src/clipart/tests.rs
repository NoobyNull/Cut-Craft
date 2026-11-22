use super::*;

fn importable(extensions: &[&str]) -> Importable {
    extensions.iter().map(|&e| e.to_owned()).collect()
}

/// A folder's clipart wraps onto rows in a narrow panel, not one long line (#34).
#[test]
fn thumbnails_wrap() {
    let dir = std::env::temp_dir().join(format!("cut-craft-clipart-wrap-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for i in 0..8 {
        std::fs::write(dir.join(format!("{i}.stl")), "").unwrap();
    }
    let stl = importable(&["stl"]);
    let mut clipart = Clipart {
        folders: vec![dir.clone()],
        ..Clipart::default()
    };
    clipart.read_now(&stl);
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(300.0, 2000.0),
        )),
        ..Default::default()
    };
    let mut used = egui::Rect::NOTHING;
    let mut out = ctx.run_ui(input, |ui| {
        egui::CentralPanel::default().show(ui, |ui| {
            panel(ui, &mut clipart, &stl, &[]);
            used = ui.min_rect();
        });
    });
    // no renderer takes a headless frame's texture changes
    out.textures_delta.clear();
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(used.width() <= 300.0, "{used:?} is wider than the panel");
}

/// Every piece of text painted in `shapes`, and where.
fn texts<'a>(shapes: impl Iterator<Item = &'a egui::Shape>) -> Vec<(String, egui::Rect, bool)> {
    let mut found = Vec::new();
    for shape in shapes {
        match shape {
            egui::Shape::Text(text) => {
                let galley = &text.galley;
                found.push((
                    galley.text().to_owned(),
                    galley.rect.translate(text.pos.to_vec2()),
                    galley.elided,
                ));
            }
            egui::Shape::Vec(inner) => found.extend(texts(inner.iter())),
            _ => {}
        }
    }
    found
}

/// A name too long for its tile ends in "..." with a gap before the next tile's name,
/// not run into it: a tile with a picture is wider than its picture by the button's
/// padding, and the name was cut to that.
#[test]
fn long_names_are_cut_short() {
    let dir = folder("long-names");
    let long = "0355. UK-1-final with a very long name indeed.png";
    for name in [long, "0356. UK-2-final.png"] {
        image::RgbImage::new(64, 64).save(dir.join(name)).unwrap();
    }
    let png = importable(&["png"]);
    let mut clipart = Clipart {
        folders: vec![dir.clone()],
        ..Clipart::default()
    };
    clipart.read_now(&png);
    let ctx = egui::Context::default();
    let (mut edge, mut gap) = (0.0, 0.0);
    let mut frame = |clipart: &mut Clipart| {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(600.0, 2000.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                edge = ui.spacing().interact_size.y * THUMBNAIL_ROWS;
                gap = ui.spacing().item_spacing.x;
                panel(ui, clipart, &png, &[]);
            });
        });
        out.textures_delta.clear();
        out
    };
    // until both pictures' previews are in, so the tiles are drawn with them
    let started = Instant::now();
    while clipart.previews.ready() < 2 && started.elapsed() < Duration::from_secs(10) {
        frame(&mut clipart);
        std::thread::sleep(Duration::from_millis(10));
    }
    let out = frame(&mut clipart);
    std::fs::remove_dir_all(&dir).unwrap();
    let captions: Vec<_> = texts(out.shapes.iter().map(|c| &c.shape))
        .into_iter()
        .filter(|(text, ..)| text.starts_with("035"))
        .collect();
    assert_eq!(captions.len(), 2, "{captions:?}");
    let (text, first, elided) = &captions[0];
    assert!(elided, "{text:?} isn't cut short");
    assert!(
        first.width() <= edge,
        "{first:?} is wider than a tile ({edge})"
    );
    let next = captions[1].1;
    assert!(
        next.left() - first.right() >= gap,
        "{first:?} runs into {next:?}"
    );
}

/// Resting the pointer on a tile pops up a larger preview with the whole name; only
/// the tile under the pointer asks for one.
#[test]
fn hovering_shows_it_larger() {
    let dir = folder("hover");
    let long = "0355. UK-1-final with a very long name indeed.png";
    for name in [long, "0356. UK-2-final.png"] {
        image::RgbImage::new(600, 400).save(dir.join(name)).unwrap();
    }
    let png = importable(&["png"]);
    let mut clipart = Clipart {
        folders: vec![dir.clone()],
        ..Clipart::default()
    };
    clipart.read_now(&png);
    let ctx = egui::Context::default();
    ctx.global_style_mut(|s| s.interaction.tooltip_delay = 0.0);
    let frame = |clipart: &mut Clipart, at: Option<egui::Pos2>| {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(600.0, 2000.0),
            )),
            events: at.map(egui::Event::PointerMoved).into_iter().collect(),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                panel(ui, clipart, &png, &[]);
            });
        });
        out.textures_delta.clear();
        out
    };
    let out = frame(&mut clipart, None);
    assert_eq!(clipart.large.asked(), 0, "nothing hovered, nothing asked");
    // the first tile: where its name is painted, and the picture above it
    let (_, name, _) = texts(out.shapes.iter().map(|c| &c.shape))
        .into_iter()
        .find(|(t, ..)| t == long)
        .unwrap();
    let over = egui::pos2(name.left() + 20.0, name.top() - 20.0);
    let mut out = frame(&mut clipart, Some(over));
    let started = Instant::now();
    while clipart.large.ready() < 1 && started.elapsed() < Duration::from_secs(10) {
        std::thread::sleep(Duration::from_millis(10));
        out = frame(&mut clipart, None);
    }
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(clipart.large.asked(), 1, "only the tile hovered");
    assert_eq!(clipart.large.ready(), 1);
    let whole = texts(out.shapes.iter().map(|c| &c.shape))
        .into_iter()
        .filter(|(t, _, elided)| t == long && !elided)
        .count();
    assert_eq!(whole, 1, "the whole name, in the pop-up");
}

/// A model's picture beside it (same name, any case) is its preview, not a tile of its
/// own; a picture of nothing in particular is still a tile; Vectric's locked clipart is
/// counted, to say why it isn't shown.
#[test]
fn pictures_go_with_their_models() {
    let dir = std::env::temp_dir().join(format!("cut-craft-clipart-list-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for f in [
        "mydesign.stl",
        "mydesign.png",
        "Eagle.STL",
        "eagle.JPG",
        "lonely.obj",
        "sunset.png",
        "rose.3dclip",
        "tulip.V3M",
        "notes.txt",
    ] {
        std::fs::write(dir.join(f), "").unwrap();
    }
    let importable = importable(&["stl", "obj", "png", "jpg"]);
    let listing = list(&dir, &importable, &mut HashMap::new());
    let entry = |file: &str, picture: Option<&str>| {
        let picture = picture.map(|p| dir.join(p));
        let source = picture.clone().unwrap_or_else(|| dir.join(file));
        Entry {
            file: dir.join(file),
            stamp: preview::stamp(&source).unwrap(),
            picture,
            kind: if file == "sunset.png" {
                Kind::Picture
            } else {
                Kind::Model
            },
        }
    };
    assert_eq!(
        listing,
        Listing {
            entries: vec![
                entry("Eagle.STL", Some("eagle.JPG")),
                entry("lonely.obj", None),
                entry("mydesign.stl", Some("mydesign.png")),
                entry("sunset.png", None),
            ],
            locked: vec![dir.join("rose.3dclip"), dir.join("tulip.V3M")],
            ..Listing::default()
        }
    );
    // a picture put beside a model later becomes its preview: the model's stamp changes
    let stamp = |listing: &Listing, file: &str| {
        let at = listing.entries.iter().find(|e| e.file == dir.join(file));
        at.unwrap().stamp
    };
    let before = stamp(&listing, "lonely.obj");
    std::fs::write(dir.join("lonely.png"), "").unwrap();
    assert_ne!(
        stamp(&list(&dir, &importable, &mut HashMap::new()), "lonely.obj"),
        before
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A picture's tile is a preview shrunk from it, not the picture itself: a scan 16 604
/// pixels wide went past the graphics card's largest texture and stopped the app.
#[test]
fn pictures_are_shrunk() {
    let dir = folder("huge");
    image::RgbImage::new(9000, 2)
        .save(dir.join("scan.png"))
        .unwrap();
    image::RgbImage::new(9000, 2)
        .save(dir.join("banner.gif"))
        .unwrap();
    let pictures = importable(&["png", "gif"]);
    let mut clipart = Clipart {
        folders: vec![dir.clone()],
        ..Clipart::default()
    };
    clipart.read_now(&pictures);
    let ctx = egui::Context::default();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        egui::CentralPanel::default().show(ui, |ui| {
            panel(ui, &mut clipart, &pictures, &[]);
        });
    });
    out.textures_delta.clear();
    assert_eq!(clipart.previews.asked(), 2, "both made into previews");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Only the tiles on screen ask for previews: a folder of a thousand models opens at
/// once, and the rest are made as they are scrolled to.
#[test]
fn previews_only_on_screen() {
    let dir = std::env::temp_dir().join(format!("cut-craft-clipart-lazy-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for i in 0..200 {
        std::fs::write(dir.join(format!("{i:03}.stl")), "").unwrap();
    }
    let stl = importable(&["stl"]);
    let mut clipart = Clipart {
        folders: vec![dir.clone()],
        ..Clipart::default()
    };
    clipart.read_now(&stl);
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(300.0, 400.0),
        )),
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |ui| {
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                panel(ui, &mut clipart, &stl, &[]);
            });
        });
    });
    out.textures_delta.clear();
    std::fs::remove_dir_all(&dir).unwrap();
    let asked = clipart.previews.asked();
    assert!(asked > 0 && asked < 50, "{asked} previews asked for");
}

fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("cut-craft-library-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Add to Level is offered for 3D clipart only: a drawing or picture adds no component,
/// so the new level would stay empty.
#[test]
fn only_3d_clipart_goes_onto_a_level() {
    let files = |names: &[&str]| names.iter().map(PathBuf::from).collect::<Vec<_>>();
    assert!(takes_a_level(&files(&[
        "eagle.stl",
        "rose.ccclip",
        "cup.OBJ"
    ])));
    assert!(!takes_a_level(&files(&["logo.svg"])));
    assert!(!takes_a_level(&files(&["relief.png"])));
    assert!(!takes_a_level(&files(&["eagle.stl", "plan.dxf"])));
}

/// A generated border is one 3D tile, its picture the one saved with it; the picture and
/// recipe are not tiles of their own.
#[test]
fn a_generated_border_is_one_model_with_its_own_picture() {
    let dir = folder("border");
    let mut photo = Vec::new();
    image::RgbImage::new(4, 2)
        .write_to(
            &mut std::io::Cursor::new(&mut photo),
            image::ImageFormat::Png,
        )
        .unwrap();
    let recipe = cut_craft_border_tile::Recipe::new("gothic_pewter", "2:1", 80.0, 40.0, 5.0);
    let f = cut_craft_border_tile::save(&dir, "Gothic", &[0.5; 8], 4, 2, &photo, &recipe).unwrap();
    let mut clipart = Clipart::default();
    let all = importable(&["stl", "png", "jpg"]);
    let listing = list(&dir, &all, &mut clipart.greys);
    assert_eq!(listing.entries.len(), 1, "{:?}", listing.entries);
    let e = &listing.entries[0];
    assert_eq!((&e.file, e.picture.as_ref()), (&f.depth, Some(&f.picture)));
    assert_eq!(clipart.kind(e), Kind::Model);
    assert!(takes_a_level(std::slice::from_ref(&f.depth)));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Each file is a kind the Show filter picks: a greyscale picture, or one tagged so, is
/// a depth map.
#[test]
fn kinds_of_file() {
    let dir = folder("kinds");
    std::fs::write(dir.join("eagle.stl"), "").unwrap();
    std::fs::write(dir.join("logo.svg"), "<svg/>").unwrap();
    std::fs::write(dir.join("plan.dxf"), "").unwrap();
    image::RgbImage::new(4, 4)
        .save(dir.join("photo.png"))
        .unwrap();
    image::RgbImage::new(4, 4)
        .save(dir.join("tagged.png"))
        .unwrap();
    image::GrayImage::new(4, 4)
        .save(dir.join("relief.png"))
        .unwrap();
    let mut clipart = Clipart::default();
    clipart
        .tags
        .add(&[dir.join("tagged.png")], "Depth Map")
        .unwrap();
    let all = importable(&["stl", "svg", "dxf", "png"]);
    let listing = list(&dir, &all, &mut clipart.greys);
    let kinds: Vec<(String, Kind)> = (listing.entries.iter())
        .map(|e| (name(&e.file), clipart.kind(e)))
        .collect();
    assert_eq!(
        kinds,
        [
            ("eagle.stl".into(), Kind::Model),
            ("logo.svg".into(), Kind::Drawing),
            ("photo.png".into(), Kind::Picture),
            ("plan.dxf".into(), Kind::Drawing),
            ("relief.png".into(), Kind::DepthMap),
            ("tagged.png".into(), Kind::DepthMap),
        ]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A click picks one; Ctrl+click adds or takes one away; Shift+click picks everything
/// from the last one clicked, as file managers do.
#[test]
fn picking() {
    let files: Vec<PathBuf> = ["a", "b", "c", "d", "e"].map(PathBuf::from).into();
    let order: Vec<&Path> = files.iter().map(PathBuf::as_path).collect();
    let mut clipart = Clipart::default();
    let picked = |c: &Clipart| {
        let mut p: Vec<String> = c.selected.iter().map(|f| name(f)).collect();
        p.sort();
        p.join("")
    };
    clipart.click(&files[1], false, false, &order);
    assert_eq!(picked(&clipart), "b");
    clipart.click(&files[3], false, true, &order);
    assert_eq!(picked(&clipart), "bcd");
    clipart.click(&files[0], false, true, &order);
    assert_eq!(picked(&clipart), "ab", "Shift+click again goes from b");
    clipart.click(&files[4], true, false, &order);
    assert_eq!(picked(&clipart), "abe");
    clipart.click(&files[0], true, false, &order);
    assert_eq!(picked(&clipart), "be");
    clipart.click(&files[2], false, false, &order);
    assert_eq!(picked(&clipart), "c");
}

/// Delete from Disk takes the files away for good, and with them their tags and
/// their place among the picked; one already gone is no trouble.
#[test]
fn deleting() {
    let dir = folder("delete");
    let (stl, png) = (dir.join("eagle.stl"), dir.join("eagle.png"));
    std::fs::write(&stl, "").unwrap();
    std::fs::write(&png, "").unwrap();
    let mut clipart = Clipart::default();
    clipart
        .tags
        .add(std::slice::from_ref(&stl), "birds")
        .unwrap();
    clipart.selected = vec![stl.clone(), dir.join("other.stl")];
    clipart
        .delete(&[stl.clone(), png.clone(), dir.join("never-there.stl")])
        .unwrap();
    assert!(!stl.exists() && !png.exists());
    assert_eq!(clipart.selected, [dir.join("other.stl")]);
    assert_eq!(clipart.tags.of(&stl).count(), 0);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The Library opens on its folders as they were: kept between runs, looking in the
/// folder looked in last; in all of them if that was all, or it's no longer added.
#[test]
fn opens_where_it_was() {
    let dir = folder("kept");
    let store = dir.join("clipart.json");
    let (a, b) = (dir.join("a"), dir.join("b"));
    let mut library = Clipart::kept(Some(store.clone()));
    assert_eq!(library.open, None, "nothing to look in yet");
    library.add(a.clone()).unwrap();
    library.add(b.clone()).unwrap();
    let mut again = Clipart::kept(Some(store.clone()));
    assert_eq!(again.folders, [a.clone(), b.clone()]);
    assert_eq!(
        again.open,
        Some(b.clone()),
        "the one added last is looked in"
    );
    again.choose(Some(a.join("birds")));
    assert_eq!(
        Clipart::kept(Some(store.clone())).open,
        Some(a.join("birds"))
    );
    again.choose(None);
    assert_eq!(Clipart::kept(Some(store.clone())).open, None, "all of them");
    again.choose(Some(a.join("birds")));
    again.remove(&a).unwrap();
    assert_eq!(
        again.open, None,
        "looking in a folder taken away: all of them"
    );
    assert_eq!(Clipart::kept(Some(store.clone())).folders, [b]);
    assert_eq!(Clipart::kept(Some(store)).open, None);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A folder is in the library once: one added inside another already there is looked
/// in, not added; one added round others takes their place; and folders kept from
/// before this (Matthew's had both) are tidied as they are loaded.
#[test]
fn no_folder_twice() {
    let dir = folder("once");
    let store = dir.join("clipart.json");
    let (team, club, other) = (
        dir.join("team"),
        dir.join("team").join("club"),
        dir.join("other"),
    );
    crate::files::save_recent(&[other.clone(), team.clone(), club.clone()], &store).unwrap();
    let mut library = Clipart::kept(Some(store.clone()));
    assert_eq!(
        library.folders,
        [other.clone(), team.clone()],
        "tidied as loaded"
    );
    assert_eq!(
        Clipart::kept(Some(store.clone())).folders,
        [other.clone(), team.clone()],
        "and kept tidy"
    );
    library.add(club.clone()).unwrap();
    assert_eq!(
        library.folders,
        [other.clone(), team.clone()],
        "inside one already added"
    );
    assert_eq!(
        library.open,
        Some(club.clone()),
        "so looked in there instead"
    );
    library.add(team.clone()).unwrap();
    assert_eq!(
        library.folders,
        [other.clone(), team.clone()],
        "the same one again"
    );
    library.add(dir.clone()).unwrap();
    assert_eq!(
        library.folders,
        std::slice::from_ref(&dir),
        "round the others: they're in it"
    );
    assert_eq!(
        Clipart::kept(Some(store)).folders,
        std::slice::from_ref(&dir)
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Where `text` is drawn in `shapes`, if it is.
fn text_at(shapes: &[egui::epaint::ClippedShape], text: &str) -> Option<egui::Pos2> {
    shapes.iter().find_map(|c| match &c.shape {
        egui::Shape::Text(t) if t.galley.text() == text => Some(t.pos + t.galley.size() / 2.0),
        _ => None,
    })
}

/// Right-click ▸ Properties, at the bottom of a tile's menu, opens a window about the
/// file: its kind, size, tags and where it is. Escape closes it.
#[test]
fn properties_from_the_menu() {
    let dir = folder("properties");
    let file = dir.join("eagle.stl");
    std::fs::write(&file, "").unwrap();
    let stl = importable(&["stl"]);
    let mut clipart = Clipart {
        folders: vec![dir.clone()],
        ..Clipart::default()
    };
    clipart
        .tags
        .add(std::slice::from_ref(&file), "birds")
        .unwrap();
    clipart.read_now(&stl);
    let ctx = egui::Context::default();
    let frame = |clipart: &mut Clipart, events: Vec<egui::Event>| {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 800.0),
            )),
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                panel(ui, clipart, &stl, &[]);
            });
        });
        out.textures_delta.clear();
        out.shapes
    };
    let click = |at: egui::Pos2, button: egui::PointerButton| {
        [true, false].map(|pressed| egui::Event::PointerButton {
            pos: at,
            button,
            pressed,
            modifiers: egui::Modifiers::NONE,
        })
    };
    let shapes = frame(&mut clipart, vec![]);
    let tile = text_at(&shapes, "eagle.stl").expect("the tile's name");
    frame(&mut clipart, vec![egui::Event::PointerMoved(tile)]);
    for e in click(tile, egui::PointerButton::Secondary) {
        frame(&mut clipart, vec![e]);
    }
    // a new menu is drawn unseen for a frame while egui sizes it
    frame(&mut clipart, vec![]);
    let shapes = frame(&mut clipart, vec![]);
    let item = text_at(&shapes, "Properties").expect("Properties in the menu");
    let report = text_at(&shapes, "Report a Bug Here…").expect("the bug report item");
    assert!(item.y < report.y, "Properties comes before Report a Bug");
    frame(&mut clipart, vec![egui::Event::PointerMoved(item)]);
    for e in click(item, egui::PointerButton::Primary) {
        frame(&mut clipart, vec![e]);
    }
    clipart
        .properties
        .as_mut()
        .expect("the window is open")
        .wait();
    frame(&mut clipart, vec![]);
    let shapes = frame(&mut clipart, vec![]);
    for text in ["3D model (STL file)", "birds", "0 bytes"] {
        assert!(text_at(&shapes, text).is_some(), "{text} is shown");
    }
    assert!(
        text_at(&shapes, &file.display().to_string()).is_some(),
        "its path"
    );
    let escape = egui::Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    frame(&mut clipart, vec![escape]);
    assert!(clipart.properties.is_none(), "Escape closes it");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A folder added from a network share's FUSE mount (as KDE's file dialog gives it) is
/// kept by its share's location, not by the mount, whose name changes every session; a
/// share that can't be reached stays in the Library, and comes back where it's mounted
/// now when it can be.
#[test]
fn network_folders_last() {
    let store = folder("network").join("clipart.json");
    let monday = PathBuf::from("/run/user/1000/kio-fuse-UrUqhD/smb/synology.local/STL/A-Team");
    let mut library = Clipart::kept(Some(store.clone()));
    library.add(monday.join("02_CLUB A")).unwrap();
    assert_eq!(library.folders, [monday.join("02_CLUB A")]);
    let url = PathBuf::from("smb://synology.local/STL/A-Team/02_CLUB A");
    assert_eq!(
        crate::files::load_recent(&store),
        std::slice::from_ref(&url)
    );
    // the same folder from tomorrow's mount isn't added again
    let tuesday = PathBuf::from("/run/user/1000/kio-fuse-NeWtOk/smb/synology.local/STL/A-Team");
    library.add(tuesday.join("02_CLUB A")).unwrap();
    assert_eq!(library.folders.len(), 1);

    // next run: kept, and looked in, while the share isn't reached
    let mut again = Clipart::kept(Some(store.clone()));
    assert_eq!(again.folders, std::slice::from_ref(&url));
    assert_eq!(again.open, Some(url.clone()));
    assert_eq!(again.unreachable(&url), Some(false), "connecting at first");
    again.found(vec![(url.clone(), None)]);
    assert_eq!(again.folders, std::slice::from_ref(&url), "not dropped");
    assert_eq!(again.unreachable(&url), Some(true));
    // ... and found mounted somewhere new
    let now = folder("network-mount");
    again.found(vec![(url.clone(), Some(now.clone()))]);
    assert_eq!(again.folders, std::slice::from_ref(&now));
    assert_eq!(again.open, Some(now.clone()));
    assert_eq!(again.unreachable(&now), None);
    // a local folder that's gone is only not found
    assert_eq!(again.unreachable(Path::new("/media/usb/STL")), None);
}

/// The folders in the folders added are read too, however deep, and counted for the
/// tree; hidden ones aren't, and a folder added inside another is read once.
#[test]
fn folders_in_folders() {
    let dir = folder("deep");
    let birds = dir.join("animals").join("birds");
    let hidden = dir.join(".thumbnails");
    for d in [&birds, &hidden, &dir.join("empty")] {
        std::fs::create_dir_all(d).unwrap();
    }
    for f in [
        dir.join("top.stl"),
        dir.join("animals").join("cat.stl"),
        birds.join("eagle.stl"),
        birds.join("owl.stl"),
        hidden.join("secret.stl"),
    ] {
        std::fs::write(f, "").unwrap();
    }
    // a link back up, which would go round for ever if followed
    #[cfg(unix)]
    std::os::unix::fs::symlink(&dir, birds.join("loop")).unwrap();
    let stl = importable(&["stl"]);
    let read = read_all(&[dir.clone(), birds.clone()], &stl, &mut HashMap::new());
    let names: Vec<String> = read.entries.iter().map(|e| name(&e.file)).collect();
    // by folder, then name
    assert_eq!(names, ["eagle.stl", "owl.stl", "cat.stl", "top.stl"]);
    let count = |f: &Path| read.counts.get(f).copied();
    assert_eq!(count(&dir), Some(4));
    assert_eq!(count(&dir.join("animals")), Some(3));
    assert_eq!(count(&birds), Some(2));
    assert_eq!(
        count(&dir.join("empty")),
        None,
        "nothing in it: not in the tree"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Listing stops at the most files Options ▸ Advanced allows, and says so.
#[test]
fn listing_stops_at_the_most_files_allowed() {
    let dir = folder("most");
    for sub in ["a", "b", "c"] {
        std::fs::create_dir_all(dir.join(sub)).unwrap();
        std::fs::write(dir.join(sub).join("part.stl"), "").unwrap();
    }
    let stl = importable(&["stl"]);
    let read = read_all(std::slice::from_ref(&dir), &stl, &mut HashMap::new());
    assert!(!read.full && read.entries.len() == 3);
    // the most allowed is passed in, not set, since tests beside this one read it
    let read = read_most(std::slice::from_ref(&dir), &stl, &mut HashMap::new(), 1);
    assert!(read.full, "stopped");
    assert!(read.entries.len() < 3, "{}", read.entries.len());
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Thousands of files open at once: only the tiles on screen are drawn. Looking in a
/// folder shows what is in it and the folders in it; looking in none, everything.
#[test]
fn thousands_of_files() {
    let root = folder("thousands");
    std::fs::create_dir_all(root.join("few")).unwrap();
    let entry = |file: PathBuf| Entry {
        file,
        picture: None,
        stamp: 0,
        kind: Kind::Drawing,
    };
    let mut entries: Vec<Entry> = (0..3000)
        .map(|i| entry(root.join(format!("{i:04}.dxf"))))
        .collect();
    entries.extend((0..5).map(|i| entry(root.join("few").join(format!("{i}.dxf")))));
    let mut clipart = Clipart {
        folders: vec![root.clone()],
        listing: Some((
            vec![root.clone()],
            Instant::now(),
            Listing {
                entries,
                ..Listing::default()
            },
        )),
        ..Clipart::default()
    };
    let ctx = egui::Context::default();
    let draw = |clipart: &mut Clipart| {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(300.0, 600.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    panel(ui, clipart, &importable(&["dxf"]), &[]);
                });
            });
        });
        out.textures_delta.clear();
        clipart.drawn
    };
    let drawn = draw(&mut clipart);
    assert!(drawn > 0 && drawn < 60, "{drawn} tiles drawn");
    clipart.open = Some(root.join("few"));
    assert_eq!(draw(&mut clipart), 5);
    std::fs::remove_dir_all(&root).unwrap();
}

/// Tiles fill the rows across, and the rows in view are the ones drawn.
#[test]
fn rows_in_view() {
    let step = egui::vec2(60.0, 80.0);
    let seen = |from: f32, to: f32| egui::Rangef::new(from, to);
    // 300 wide fits 5 tiles 56 across with a 4 gap between (the last needs none)
    assert_eq!(
        layout(12, 296.0, step, 4.0, seen(0.0, 100.0), 0.0),
        (5, 3, 0, 10)
    );
    assert_eq!(
        layout(12, 296.0, step, 4.0, seen(170.0, 400.0), 0.0),
        (5, 3, 10, 12)
    );
    assert_eq!(
        layout(12, 10.0, step, 4.0, seen(0.0, 10.0), 50.0),
        (1, 12, 0, 0)
    );
    assert_eq!(
        layout(0, 300.0, step, 4.0, seen(0.0, 100.0), 0.0),
        (5, 0, 0, 0)
    );
}
