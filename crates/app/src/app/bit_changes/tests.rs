use super::*;
use cut_craft_core::Change;
use cut_craft_core::toolpath::{Tool, Toolpath};
use cut_craft_machines::Firmware;
use files::ToolChanges;

/// A job with toolpaths of these names and bits, listed in this order.
fn job(toolpaths: &[(&str, &str)]) -> (Document, Vec<EntityId>) {
    let mut doc = Document::default();
    let mut c = Change::builder(&doc);
    let ids = toolpaths
        .iter()
        .map(|(name, bit)| {
            c.add_toolpath(Toolpath {
                name: (*name).into(),
                strategy: "profile_toolpath".into(),
                params: serde_json::json!({}),
                tool: Tool {
                    name: (*bit).into(),
                    ..Default::default()
                },
                vectors: vec![],
                moves: vec![],
                visible: true,
                power: vec![],
                stale: false,
                group: None,
                drill_bank: None,
                fallbacks: vec![],
                across: vec![],
            })
        })
        .collect();
    doc.apply(&c.finish()).unwrap();
    (doc, ids)
}

fn machine(firmware: Firmware) -> Machine {
    Machine {
        firmware: Some(firmware),
        ..Machine::default()
    }
}

fn messages(found: &[Finding]) -> Vec<&str> {
    assert!(found.iter().all(|f| f.check == Check::BitChanges));
    found.iter().map(|f| f.message.as_str()).collect()
}

#[test]
fn one_bit_or_a_tool_changer_needs_no_reminder() {
    let (doc, ids) = job(&[("Pocket", "End Mill"), ("Profile", "End Mill")]);
    let one = findings(
        &doc,
        None,
        std::slice::from_ref(&ids),
        ToolChanges::SeparateFiles,
    );
    assert!(one.is_empty(), "{one:?}");
    let (doc, ids) = job(&[("Pocket", "End Mill"), ("Text", "V-Bit")]);
    let changer = findings(&doc, None, &[ids], ToolChanges::ToolChanger);
    assert!(changer.is_empty(), "{changer:?}");
}

#[test]
fn each_bit_changed_by_hand_is_a_reminder_to_set_z_zero_again() {
    let (doc, ids) = job(&[
        ("Pocket", "End Mill"),
        ("Text", "V-Bit"),
        ("Cut Out", "End Mill"),
    ]);
    let plan = vec![vec![ids[0]], vec![ids[1]], vec![ids[2]]];
    let found = findings(&doc, None, &plan, ToolChanges::SeparateFiles);
    let said = messages(&found);
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(said[0].contains("2 times"), "{}", said[0]);
    assert!(
        said[0].contains("End Mill, then V-Bit, then End Mill"),
        "{}",
        said[0]
    );
    assert!(said[0].contains("before running each file"), "{}", said[0]);
    assert!(said[0].contains("on top of the material"), "{}", said[0]);
    let paused = findings(&doc, None, &[ids], ToolChanges::Pause);
    assert!(messages(&paused)[0].contains("at each pause"));
}

#[test]
fn pausing_on_standard_grbl_warns_it_cannot_jog_to_set_z_zero() {
    let (doc, ids) = job(&[("Pocket", "End Mill"), ("Text", "V-Bit")]);
    let plan = [ids];
    let grbl = machine(Firmware::Grbl);
    let found = findings(&doc, Some(&grbl), &plan, ToolChanges::Pause);
    assert!(
        messages(&found).iter().any(|m| m.contains("cannot jog")),
        "{found:?}"
    );
    // separate files, or a controller that can jog when paused: no such warning
    let files = findings(&doc, Some(&grbl), &plan, ToolChanges::SeparateFiles);
    assert!(!messages(&files).iter().any(|m| m.contains("cannot jog")));
    let mach = machine(Firmware::Mach);
    let found = findings(&doc, Some(&mach), &plan, ToolChanges::Pause);
    assert!(!messages(&found).iter().any(|m| m.contains("cannot jog")));
}

#[test]
fn a_cutting_order_changed_by_group_by_bit_is_shown_to_be_checked() {
    let (doc, ids) = job(&[
        ("Pocket", "End Mill"),
        ("Text", "V-Bit"),
        ("Cut Out", "End Mill"),
    ]);
    let grouped = files::grouped_by_bit(&doc, std::slice::from_ref(&ids), false);
    assert_eq!(grouped, vec![vec![ids[0], ids[2], ids[1]]]);
    let found = findings(&doc, None, &grouped, ToolChanges::ToolChanger);
    let said = messages(&found);
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(
        said[0].contains("\"Pocket\", \"Cut Out\", \"Text\""),
        "{}",
        said[0]
    );
    // in list order, nothing to say
    assert_eq!(
        findings(&doc, None, &[ids], ToolChanges::ToolChanger),
        vec![]
    );
}

#[test]
fn group_by_bit_keeps_each_bits_toolpaths_in_list_order_and_merges_files_when_asked() {
    let (doc, ids) = job(&[
        ("A", "End Mill"),
        ("B", "V-Bit"),
        ("C", "End Mill"),
        ("D", "Ball Nose"),
        ("E", "V-Bit"),
    ]);
    let each: Vec<Vec<EntityId>> = ids.iter().map(|&id| vec![id]).collect();
    let apart = files::grouped_by_bit(&doc, &each, false);
    let order = |plan: &[Vec<EntityId>]| plan.iter().flatten().copied().collect::<Vec<_>>();
    assert_eq!(order(&apart), [ids[0], ids[2], ids[1], ids[4], ids[3]]);
    assert_eq!(apart.len(), 5);
    let merged = files::grouped_by_bit(&doc, &each, true);
    assert_eq!(
        merged,
        vec![vec![ids[0], ids[2]], vec![ids[1], ids[4]], vec![ids[3]]]
    );
}
