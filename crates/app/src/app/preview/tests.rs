use super::*;
use cut_craft_core::document::Job;

fn block(width: f64) -> Material {
    let job = Job {
        width,
        height: 100.0,
        ..Job::default()
    };
    Material::new(&job, 10)
}

#[test]
fn undo_last_preview_brings_back_the_material_and_side_before_it() {
    let mut preview = Preview {
        material: Some(block(100.0)),
        side: true,
        ..Preview::default()
    };
    preview.keep_for_undo(false);
    (preview.material, preview.side) = (Some(block(200.0)), false);
    assert_eq!(
        preview.undo(),
        Some(true),
        "turned to the side it was cut on"
    );
    assert_eq!(preview.material, Some(block(100.0)));
    assert_eq!(preview.undo(), None, "only the last preview undoes");
}

#[test]
fn reset_preview_leaves_none_shown_and_can_be_undone() {
    let mut preview = Preview {
        material: Some(block(100.0)),
        ..Preview::default()
    };
    preview.keep_for_undo(true);
    assert!(preview.material.is_none());
    assert_eq!(preview.undo(), Some(false));
    assert_eq!(preview.material, Some(block(100.0)));
}

#[test]
fn a_preview_of_an_old_block_goes_with_its_undo() {
    let old = |m: &Material| m != &block(100.0);
    let mut preview = Preview {
        material: Some(block(100.0)),
        ..Preview::default()
    };
    preview.keep_for_undo(false);
    assert!(!preview.drop_old(old), "both on the block as it is");
    preview.before = Some(BeforePreview(Some(block(50.0)), false));
    assert!(preview.drop_old(old), "its undo was of another block");
    assert!(preview.material.is_none() && preview.before.is_none());
}
