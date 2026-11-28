use super::*;
use crate::commands::registry;
use crate::form::{arrange, fields};

fn form(id: &str) -> Vec<Field> {
    arrange(fields(&registry().get(id).unwrap().schema()))
}

fn tab(fields: &[Field], key: &str) -> FormTab {
    tab_of(fields.iter().find(|f| f.key == key).unwrap())
}

#[test]
fn every_toolpath_form_is_tabbed_with_the_bit_first_and_defaults_last() {
    let mut checked = 0;
    for command in registry().iter() {
        let f = arrange(fields(&command.schema()));
        if !tabbed(&f) {
            continue;
        }
        checked += 1;
        let tabs = tabs_of(&f);
        assert_eq!(tabs.first(), Some(&FormTab::Cutting), "{}", command.id());
        assert_eq!(tabs.last(), Some(&FormTab::Defaults), "{}", command.id());
        assert!(tabs.len() >= 3, "{}: {tabs:?}", command.id());
        assert!(tabs.windows(2).all(|w| w[0] < w[1]), "{}", command.id());
        // every setting stands on one of the form's tabs
        let shown: usize = tabs.iter().map(|&t| on_tab(&f, t).len()).sum();
        assert_eq!(shown, f.len(), "{}", command.id());
    }
    assert!(checked > 25, "only {checked} toolpath forms");
}

#[test]
fn a_pockets_settings_go_where_a_beginner_looks_for_them() {
    let pocket = form("pocket_toolpath");
    assert_eq!(tab(&pocket, "tool"), FormTab::Cutting);
    assert_eq!(tab(&pocket, "clearance_tools"), FormTab::Cutting);
    assert_eq!(tab(&pocket, "depth"), FormTab::Cutting);
    assert_eq!(tab(&pocket, AFTER_TOOL), FormTab::Where);
    assert_eq!(tab(&pocket, "strategy"), FormTab::Own);
    assert_eq!(tab(&pocket, "allowance"), FormTab::Own);
    assert_eq!(tab(&pocket, "project_onto_model"), FormTab::Where);
    assert_eq!(tab(&pocket, "name"), FormTab::Defaults);
}

#[test]
fn profiles_side_is_with_its_depth_and_its_leads_and_tabs_are_movement() {
    let profile = form("profile_toolpath");
    assert_eq!(tab(&profile, "depth"), FormTab::Cutting);
    assert_eq!(tab(&profile, "side"), FormTab::Cutting);
    assert_eq!(tab(&profile, "lead"), FormTab::Movement);
    assert_eq!(tab(&profile, "order"), FormTab::Movement);
    assert_eq!(tab(&profile, "start_at"), FormTab::Movement);
    assert_eq!(tab(&profile, "sharp_corners"), FormTab::Movement);
    assert_eq!(tab(&profile, "tabs"), FormTab::Movement);
    assert_eq!(tab(&profile, "allowance"), FormTab::Finishing);
}

#[test]
fn a_tab_with_nothing_on_it_is_left_out() {
    let pocket = form("pocket_toolpath");
    let tabs = tabs_of(&pocket);
    for t in ALL {
        let has = !on_tab(&pocket, t).is_empty();
        assert_eq!(
            tabs.contains(&t),
            has || t == FormTab::Defaults,
            "{t:?}: {tabs:?}"
        );
    }
}

#[test]
fn a_changed_setting_marks_its_tab_but_the_bit_and_name_never_do() {
    let pocket = form("pocket_toolpath");
    let defaults = registry().get("pocket_toolpath").unwrap().default_params();
    let mut params = defaults.clone();
    assert_eq!(changed_tabs(&pocket, &params, &defaults), []);
    params["name"] = "Mine".into();
    params["tool"]["name"] = "Another bit".into();
    assert_eq!(changed_tabs(&pocket, &params, &defaults), []);
    params["allowance"] = 0.5.into();
    assert_eq!(changed_tabs(&pocket, &params, &defaults), [FormTab::Own]);
    assert_eq!(changed_keys(&pocket, &params, &defaults), ["allowance"]);
    params["depth"] = 12.0.into();
    assert_eq!(
        changed_tabs(&pocket, &params, &defaults),
        [FormTab::Cutting, FormTab::Own]
    );
}

#[test]
fn a_warning_marks_the_tab_of_the_setting_it_names() {
    let pocket = form("pocket_toolpath");
    let depth = pocket
        .iter()
        .find(|f| f.key == "depth")
        .unwrap()
        .label
        .clone();
    let said = format!("The {depth} is deeper than the material");
    assert_eq!(warned_tabs(&pocket, &[&said], false), [FormTab::Cutting]);
    assert_eq!(warned_tabs(&pocket, &["Nothing to cut"], false), []);
    assert_eq!(
        warned_tabs(&pocket, &["Nothing to cut"], true),
        [FormTab::Cutting],
        "a warning about the bit marks the bit's tab"
    );
}

#[test]
fn a_tour_pointing_at_a_setting_finds_its_tab() {
    let profile = form("profile_toolpath");
    assert_eq!(tab_of_mark(&profile, "field:lead"), Some(FormTab::Movement));
    assert_eq!(tab_of_mark(&profile, "field:depth"), Some(FormTab::Cutting));
    assert_eq!(tab_of_mark(&profile, "button:Calculate"), None);
}

#[test]
fn every_toolpath_forms_own_tab_has_a_short_name() {
    let english = cut_craft_i18n::english();
    let missing: Vec<String> = registry()
        .iter()
        .filter(|c| tabbed(&fields(&c.schema())))
        .map(|c| own_key(c.id()))
        .filter(|k| !english.has(k))
        .collect();
    assert!(missing.is_empty(), "add to locales/en/app.ftl: {missing:?}");
}

#[test]
fn the_cutting_order_is_movement_and_stock_left_on_is_finishing() {
    let drilling = form("drilling_toolpath");
    assert_eq!(
        tab(&drilling, "use_vector_selection_order"),
        FormTab::Movement
    );
    let rotary = form("rotary_roughing_toolpath");
    assert_eq!(tab(&rotary, "allowance"), FormTab::Finishing);
}

#[test]
fn a_warning_names_a_setting_only_as_whole_words() {
    let profile = form("profile_toolpath");
    // Profile's side is called Cut: "cuts" is not it, "tabs" is the Tabs block
    let said = "This cuts the piece right out with nothing holding it. Add tabs.";
    assert_eq!(warned_tabs(&profile, &[said], false), [FormTab::Movement]);
}
