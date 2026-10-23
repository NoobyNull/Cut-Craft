//! Every command the app offers. To add one: `scripts/new-command …`, add its crate to
//! `crates/app/Cargo.toml`, and add one `.add::<…>()` line here.

use aspire_core::Registry;

pub fn registry() -> Registry {
    let mut r = Registry::default();
    r.add::<cmd_align_bottom::AlignBottom>();
    r.add::<cmd_align_left::AlignLeft>();
    r.add::<cmd_align_right::AlignRight>();
    r.add::<cmd_align_top::AlignTop>();
    r.add::<cmd_center::Center>();
    r.add::<cmd_center_horizontally::CenterHorizontally>();
    r.add::<cmd_center_horizontally_in_material::CenterHorizontallyInMaterial>();
    r.add::<cmd_center_in_material::CenterInMaterial>();
    r.add::<cmd_center_vertically::CenterVertically>();
    r.add::<cmd_center_vertically_in_material::CenterVerticallyInMaterial>();
    r.add::<cmd_delete::Delete>();
    r.add::<cmd_draw_arc::DrawArc>();
    r.add_interactive::<cmd_draw_circle::DrawCircle>();
    r.add_interactive::<cmd_draw_ellipse::DrawEllipse>();
    r.add_interactive::<cmd_draw_polygon::DrawPolygon>();
    r.add::<cmd_draw_polyline::DrawPolyline>();
    r.add_interactive::<cmd_draw_rectangle::DrawRectangle>();
    r.add_interactive::<cmd_draw_star::DrawStar>();
    r.add::<cmd_mirror::Mirror>();
    r.add::<cmd_move::Move>();
    r.add::<cmd_rotate::Rotate>();
    r.add::<cmd_select_all_vectors::SelectAllVectors>();
    r.add::<cmd_set_size::SetSize>();
    r.add::<cmd_unselect_all::UnselectAll>();
    r.add::<cmd_job_setup::JobSetup>();
    // scripts/new-command adds new commands above this line
    r.add_exporter::<fmt_export_svg::ExportSvg>();
    r.add_importer::<fmt_import_svg::ImportSvg>();
    r
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn every_command_crate_is_registered() {
        let registry = super::registry();
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../commands");
        for entry in std::fs::read_dir(dir).unwrap() {
            let name = entry.unwrap().file_name().to_string_lossy().into_owned();
            let id = name.replace('-', "_");
            assert!(
                registry.get(&id).is_some(),
                "crates/commands/{name} is not registered in crates/app/src/commands.rs"
            );
        }
    }

    #[test]
    fn every_format_crate_is_registered() {
        let registry = super::registry();
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../formats");
        for entry in std::fs::read_dir(dir).unwrap() {
            let name = entry.unwrap().file_name().to_string_lossy().into_owned();
            let id = name.replace('-', "_");
            assert!(
                registry.exporter(&id).is_some() || registry.importer(&id).is_some(),
                "crates/formats/{name} is not registered in crates/app/src/commands.rs"
            );
        }
    }
}
