//! Every command the app offers. To add one: `scripts/new-command …`, add its crate to
//! `crates/app/Cargo.toml`, and add one `.add::<…>()` line here.

use aspire_core::Registry;

pub fn registry() -> Registry {
    let mut r = Registry::default();
    r.add::<cmd_draw_polyline::DrawPolyline>()
        .add::<cmd_draw_rectangle::DrawRectangle>()
        .add::<cmd_draw_circle::DrawCircle>()
        .add::<cmd_draw_ellipse::DrawEllipse>()
        .add::<cmd_draw_polygon::DrawPolygon>()
        .add::<cmd_draw_star::DrawStar>()
        .add::<cmd_draw_arc::DrawArc>();
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
}
