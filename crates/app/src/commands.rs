//! Every command the app offers. To add one: `scripts/new-command …`, add its crate to
//! `crates/app/Cargo.toml`, and add one `.add::<…>()` line here.

use cut_craft_core::Registry;

#[expect(clippy::too_many_lines, reason = "one line per command")]
pub fn registry() -> Registry {
    let mut r = Registry::default();
    r.add::<cmd_align::AlignBottom>();
    r.add::<cmd_align::AlignLeft>();
    r.add::<cmd_align::AlignRight>();
    r.add::<cmd_align::AlignTop>();
    r.add::<cmd_align::Center>();
    r.add::<cmd_align::CenterHorizontally>();
    r.add::<cmd_align::CenterHorizontallyInMaterial>();
    r.add::<cmd_align::CenterInMaterial>();
    r.add::<cmd_align::CenterVertically>();
    r.add::<cmd_align::CenterVerticallyInMaterial>();
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
    r.add::<cmd_select::SelectAllVectors>();
    r.add::<cmd_set_size::SetSize>();
    r.add::<cmd_select::UnselectAll>();
    r.add::<cmd_job_setup::JobSetup>();
    r.add::<cmd_weld_vectors::WeldVectors>();
    r.add::<cmd_subtract_vectors::SubtractVectors>();
    r.add::<cmd_overlap_intersect::OverlapIntersect>();
    r.add::<cmd_offset_vectors::OffsetVectors>();
    r.add::<cmd_join_vectors::JoinVectors>();
    r.add::<cmd_profile_toolpath::ProfileToolpath>();
    r.add::<cmd_pocket_toolpath::PocketToolpath>();
    r.add::<cmd_drilling_toolpath::DrillingToolpath>();
    r.add::<cmd_add_layer::AddLayer>();
    r.add::<cmd_delete_layer::DeleteLayer>();
    r.add::<cmd_move_to_layer::MoveToLayer>();
    r.add::<cmd_select_layer_vectors::SelectLayerVectors>();
    r.add::<cmd_layer_properties::LayerProperties>();
    r.add::<cmd_set_current_layer::SetCurrentLayer>();
    r.add::<cmd_create_shape::CreateShape>();
    r.add::<cmd_vcarve_engrave_toolpath::VcarveEngraveToolpath>();
    r.add::<cmd_3d_finishing_toolpath::FinishingToolpath3d>();
    r.add::<cmd_3d_roughing_toolpath::RoughingToolpath3d>();
    r.add::<cmd_toolpath_properties::ToolpathProperties>();
    r.add::<cmd_component_properties::ComponentProperties>();
    r.add::<cmd_delete_toolpath::DeleteToolpath>();
    r.add::<cmd_delete_component::DeleteComponent>();
    r.add::<cmd_show_hide::ShowHide>();
    r.add::<cmd_delete_items::DeleteItems>();
    r.add::<cmd_fluting_toolpath::FlutingToolpath>();
    r.add::<cmd_vcarve_inlay_toolpath::VcarveInlayToolpath>();
    r.add::<cmd_prism_toolpath::PrismToolpath>();
    r.add::<cmd_texture_toolpath::TextureToolpath>();
    r.add::<cmd_quick_engrave_toolpath::QuickEngraveToolpath>();
    r.add::<cmd_keyhole_toolpath::KeyholeToolpath>();
    r.add::<cmd_drag_knife_toolpath::DragKnifeToolpath>();
    r.add::<cmd_chamfer_toolpath::ChamferToolpath>();
    r.add::<cmd_thread_milling_toolpath::ThreadMillingToolpath>();
    r.add::<cmd_moulding_toolpath::MouldingToolpath>();
    r.add::<cmd_rounding_toolpath::RoundingToolpath>();
    r.add::<cmd_photo_vcarve_toolpath::PhotoVcarveToolpath>();
    r.add::<cmd_sketch_carving_toolpath::SketchCarvingToolpath>();
    r.add::<cmd_laser_cut_fill_toolpath::LaserCutFillToolpath>();
    r.add::<cmd_laser_picture_toolpath::LaserPictureToolpath>();
    r.add::<cmd_array_copy_toolpath::ArrayCopyToolpath>();
    r.add::<cmd_merged_toolpath::MergedToolpath>();
    r.add::<cmd_inlay_toolpath::InlayToolpath>();
    r.add::<cmd_space_horizontally::SpaceHorizontally>();
    r.add::<cmd_space_vertically::SpaceVertically>();
    r.add::<cmd_select::SelectAllOpenVectors>();
    r.add::<cmd_select::SelectAllDuplicateVectors>();
    r.add::<cmd_add_zero_plane::AddZeroPlane>();
    r.add::<cmd_scale_model_z_height::ScaleModelZHeight>();
    r.add::<cmd_create_component_from_bitmap::CreateComponentFromBitmap>();
    r.add::<cmd_bake_components::BakeComponents>();
    r.add::<cmd_create_component_from_visible_model::CreateComponentFromVisibleModel>();
    r.add::<cmd_smooth_selected_components::SmoothSelectedComponents>();
    r.add::<cmd_reset_inside_selected_vectors::ResetInsideSelectedVectors>();
    r.add::<cmd_reset_outside_selected_vectors::ResetOutsideSelectedVectors>();
    r.add::<cmd_bitmap_properties::BitmapProperties>();
    r.add::<cmd_group::Group>();
    r.add::<cmd_ungroup::Ungroup>();
    r.add_interactive::<cmd_draw_text::DrawText>();
    r.add::<cmd_fit_curves_to_vectors::FitCurvesToVectors>();
    r.add::<cmd_component_boundary::ComponentBoundary>();
    r.add::<cmd_trace_bitmap::TraceBitmap>();
    r.add::<cmd_replace_below::ReplaceBelow>();
    r.add::<cmd_offset_model::OffsetModel>();
    r.add::<cmd_slice::Slice>();
    r.add::<cmd_split::Split>();
    r.add::<cmd_emboss::Emboss>();
    r.add::<cmd_add_draft::AddDraft>();
    r.add_interactive::<cmd_cross_section::CrossSection>();
    r.add::<cmd_notes::Notes>();
    r.add::<cmd_texture_area::TextureArea>();
    r.add::<cmd_array_copy::ArrayCopy>();
    r.add::<cmd_circular_copy::CircularCopy>();
    r.add::<cmd_copy_along_vectors::CopyAlongVectors>();
    r.add::<cmd_vector_validator::VectorValidator>();
    r.add::<cmd_close_extend_vector::CloseExtendVector>();
    r.add::<cmd_create_fillets::CreateFillets>();
    r.add_interactive::<cmd_dimensions::Dimensions>();
    r.add::<cmd_turn_and_spin::TurnAndSpin>();
    r.add::<cmd_extrude_and_weave::ExtrudeAndWeave>();
    r.add::<cmd_two_rail_sweep::TwoRailSweep>();
    r.add::<cmd_nest_parts::NestParts>();
    r.add::<cmd_text_on_a_curve::TextOnACurve>();
    r.add::<cmd_move_node::MoveNode>();
    r.add::<cmd_move_node::SmoothPoint>();
    r.add::<cmd_insert_point::InsertPoint>();
    r.add::<cmd_delete_point::DeletePoint>();
    r.add::<cmd_cut_vector::CutVector>();
    r.add::<cmd_make_start_point::MakeStartPoint>();
    r.add::<cmd_convert_span::SpanToLine>();
    r.add::<cmd_convert_span::SpanToBezier>();
    r.add::<cmd_convert_span::SpanToArc>();
    r.add::<cmd_reverse_direction::ReverseDirection>();
    r.add::<cmd_close_with_line::CloseWithLine>();
    r.add_interactive::<cmd_interactive_vector_trim::InteractiveVectorTrim>();
    r.add::<cmd_vector_texture::VectorTexture>();
    r.add_interactive::<cmd_draw_freehand::DrawFreehand>();
    r.add::<cmd_draw_curve::DrawCurve>();
    r.add::<cmd_vector_selector::VectorSelector>();
    r.add::<cmd_orientate_3d_model::Orientate3dModel>();
    r.add_interactive::<cmd_sculpt::Sculpt>();
    r.add::<cmd_distort_object::DistortObject>();
    r.add::<cmd_level_properties::LevelProperties>();
    r.add::<cmd_insert_level::InsertLevel>();
    r.add::<cmd_delete_level::DeleteLevel>();
    r.add::<cmd_move_to_level::MoveToLevel>();
    r.add::<cmd_level_clipping::LevelClipping>();
    r.add::<cmd_vector_unwrapper::VectorUnwrapper>();
    r.add::<cmd_reorder_component::ReorderComponent>();
    r.add::<cmd_reorder_level::ReorderLevel>();
    r.add::<cmd_duplicate_component::DuplicateComponent>();
    r.add::<cmd_segment_3d_model::Segment3dModel>();
    r.add::<cmd_material_setup::MaterialSetup>();
    r.add::<cmd_group_components::GroupComponents>();
    r.add::<cmd_ungroup_components::UngroupComponents>();
    r.add::<cmd_group_properties::GroupProperties>();
    r.add::<cmd_add_sheet::AddSheet>();
    r.add::<cmd_delete_sheet::DeleteSheet>();
    r.add::<cmd_activate_sheet::ActivateSheet>();
    r.add::<cmd_sheet_properties::SheetProperties>();
    r.add::<cmd_duplicate_toolpath::DuplicateToolpath>();
    r.add::<cmd_crop_bitmap::CropBitmap>();
    r.add::<cmd_edit_picture::EditPicture>();
    r.add::<cmd_document_variables::DocumentVariables>();
    r.add::<cmd_materials::Materials>();
    r.add::<cmd_keep_out_zones::KeepOutZones>();
    r.add::<cmd_swap_sides::SwapSides>();
    r.add::<cmd_copy_to_other_side::CopyToOtherSide>();
    r.add::<cmd_plate_production::PlateProduction>();
    r.add::<cmd_toolpath_tiling::ToolpathTiling>();
    r.add::<cmd_wrapped_fluting_layout::WrappedFlutingLayout>();
    r.add::<cmd_wrapped_spiral_layout::WrappedSpiralLayout>();
    r.add::<cmd_celtic_weave::CelticWeave>();
    r.add::<cmd_join_close_vectors::JoinCloseVectors>();
    r.add::<cmd_convert_text_to_curves::ConvertTextToCurves>();
    r.add::<cmd_vector_boundary::VectorBoundary>();
    r.add::<cmd_auto_layout_text::AutoLayoutText>();
    // scripts/new-command adds new commands above this line
    r.add_exporter::<fmt_export_svg::ExportSvg>();
    r.add_exporter::<fmt_export_pdf::ExportPdf>();
    r.add_exporter::<fmt_export_pdf::ExportEps>();
    r.add_exporter::<fmt_export_pdf::ExportAi>();
    r.add_exporter::<fmt_create_job_sheet::JobSheet>();
    r.add_importer::<fmt_import_svg::ImportSvg>();
    r.add_importer::<fmt_import_bitmap::ImportBitmap>();
    r.add_importer::<fmt_import_stl::ImportStl>();
    r.add_importer::<fmt_export_as_3d_clipart::ImportClipart>();
    r.add_exporter::<fmt_export_as_3d_clipart::ExportClipart>();
    r.add_importer::<fmt_import_crv3d::ImportCrv3d>();
    r.add_exporter::<fmt_export_as_grayscale_bitmap::ExportGrayscaleBitmap>();
    r.add_exporter::<fmt_export_dxf::ExportDxf>();
    r.add_exporter::<fmt_export_as_stl::ExportStl>();
    r.add_importer::<fmt_import_dxf::ImportDxf>();
    r
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    /// A crate is one command named after it, or a family (`align`, `select`,
    /// `convert-span`) whose commands the registry lists by the crate's name.
    #[test]
    fn every_command_crate_is_registered() {
        let registry = super::registry();
        let source = include_str!("commands.rs");
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../commands");
        for entry in std::fs::read_dir(dir).unwrap() {
            let name = entry.unwrap().file_name().to_string_lossy().into_owned();
            let id = name.replace('-', "_");
            assert!(
                registry.get(&id).is_some() || source.contains(&format!("cmd_{id}::")),
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
