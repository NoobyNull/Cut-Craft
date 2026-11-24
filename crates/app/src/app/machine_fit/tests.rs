use super::*;

fn machine(laser: bool, rotary: bool) -> Machine {
    Machine {
        controller: "grblHAL".into(),
        laser,
        rotary,
        ..Machine::default()
    }
}

#[test]
fn toolpaths_that_need_nothing_are_always_offered() {
    assert!(fits(None, Needs::Nothing));
    assert!(fits(Some(&machine(false, false)), Needs::Nothing));
    assert_eq!(lacks(None, Needs::Nothing), None);
}

#[test]
fn with_no_machine_set_up_there_is_no_laser_or_4th_axis() {
    assert!(!fits(None, Needs::Laser));
    assert!(!fits(None, Needs::Rotary));
}

#[test]
fn a_laser_toolpath_is_offered_only_with_a_laser_fitted() {
    assert!(!fits(Some(&machine(false, true)), Needs::Laser));
    assert!(fits(Some(&machine(true, false)), Needs::Laser));
    let said = lacks(Some(&machine(false, false)), Needs::Laser).unwrap();
    assert!(said.contains("Laser fitted") && said.contains("Machine Configuration"));
}

#[test]
fn a_rotary_toolpath_is_offered_only_with_a_4th_axis() {
    assert!(!fits(Some(&machine(true, false)), Needs::Rotary));
    assert!(fits(Some(&machine(false, true)), Needs::Rotary));
    let said = lacks(Some(&machine(false, false)), Needs::Rotary).unwrap();
    assert!(said.contains("4th axis") && said.contains("Machine Configuration"));
}
