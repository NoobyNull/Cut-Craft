//! Tool Database name formats (Aspire's Tool Database ▸ Name Format): a tool's name made
//! from a template of `{Variable}` or `{Variable|Modifier}`, so `{Tool Type} {Diameter|F}`
//! names a 1/8 inch end mill "End Mill 1/8". Modifiers: `U` and `L` upper and lower case a
//! word, `0.00` pads and rounds a number (zeros before the point pad it, after it the
//! decimals), and `F` writes inches as a fraction. A variable that has no value here stays
//! as written, as Aspire's does.

use crate::document::Units;
use crate::toolpath::{Tool, ToolShape};

/// Every variable a format can use, for the form's list.
pub const VARIABLES: [&str; 19] = [
    "Tool Type",
    "Units",
    "Units Short",
    "Diameter",
    "Included Angle",
    "Side Angle",
    "Tip Radius",
    "Flat Diameter",
    "Num Flutes",
    "Length Units",
    "Length Units Short",
    "Pass Depth",
    "Stepover",
    "Rate Units",
    "Move Speed",
    "Plunge Rate",
    "Spindle Speed",
    "Material Name",
    "Machine Name",
];

/// The key a tool type's default name format is kept under: its saved name (`end_mill`).
pub fn type_key(shape: ToolShape) -> String {
    serde_json::to_value(shape)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// What a variable is: a word, or a number (in the tool's own units).
enum Value {
    Text(String),
    Number(f64),
}

/// `format` filled in for `tool`, cutting `material` on `machine` (either may be empty).
pub fn expand(format: &str, tool: &Tool, material: &str, machine: &str) -> String {
    let mut out = String::new();
    let mut rest = format;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let Some(close) = rest[open..].find('}') else {
            break;
        };
        let inside = &rest[open + 1..open + close];
        let (name, modifier) = inside.split_once('|').unwrap_or((inside, ""));
        match value(name.trim(), tool, material, machine) {
            Some(v) => out.push_str(&written(&v, modifier.trim(), tool.units)),
            None => out.push_str(&rest[open..=open + close]),
        }
        rest = &rest[open + close + 1..];
    }
    out.push_str(rest);
    out
}

fn value(name: &str, tool: &Tool, material: &str, machine: &str) -> Option<Value> {
    let inch = tool.units == Units::Inch;
    let length = |mm: f64| Value::Number(if inch { mm / 25.4 } else { mm });
    let text = |s: &str| Some(Value::Text(s.to_owned()));
    let rate = |mm_per_min: f64| Value::Number(mm_per_min / tool.rate_units.mm_per_min());
    match name {
        "Tool Type" => text(match tool.shape {
            ToolShape::EndMill => "End Mill",
            ToolShape::BallNose => "Ball Nose",
            ToolShape::VBit => "V-Bit",
            ToolShape::RadiusedEndMill => "Radiused End Mill",
            ToolShape::ThreadMill => "Thread Mill",
        }),
        "Units" | "Length Units" => text(if inch { "inches" } else { "mm" }),
        "Units Short" | "Length Units Short" => text(if inch { "in" } else { "mm" }),
        "Diameter" => Some(length(tool.diameter)),
        "Included Angle" => Some(Value::Number(tool.angle)),
        "Side Angle" => Some(Value::Number(tool.taper)),
        "Tip Radius" => Some(length(tool.tip_radius)),
        "Flat Diameter" => Some(length(tool.shank_diameter)),
        "Num Flutes" => Some(Value::Number(f64::from(tool.flutes))),
        "Pass Depth" => Some(length(tool.pass_depth)),
        "Stepover" => Some(length(tool.stepover * tool.diameter)),
        "Rate Units" => text(tool.rate_units.label()),
        "Move Speed" => Some(rate(tool.feed_rate)),
        "Plunge Rate" => Some(rate(tool.plunge_rate)),
        "Spindle Speed" => Some(Value::Number(tool.spindle_speed)),
        "Material Name" if !material.is_empty() => text(material),
        "Machine Name" if !machine.is_empty() => text(machine),
        _ => None,
    }
}

/// A value as the modifier asks: cased, padded and rounded, or a fraction of an inch.
fn written(v: &Value, modifier: &str, units: Units) -> String {
    match (v, modifier) {
        (Value::Text(s), "U") => s.to_uppercase(),
        (Value::Text(s), "L") => s.to_lowercase(),
        (Value::Text(s), _) => s.clone(),
        (Value::Number(n), "F") => fraction(if units == Units::Inch { *n } else { n / 25.4 }),
        (Value::Number(n), pattern) if pattern.starts_with('0') => {
            let (whole, decimals) = pattern.split_once('.').unwrap_or((pattern, ""));
            let width = whole.len()
                + if decimals.is_empty() {
                    0
                } else {
                    decimals.len() + 1
                };
            format!("{n:0width$.prec$}", prec = decimals.len())
        }
        (Value::Number(n), _) => plain(*n),
    }
}

/// A number without trailing zeros: 0.125, 6, 1.5.
fn plain(n: f64) -> String {
    let s = format!("{n:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    s.to_owned()
}

/// Inches as a fraction to the nearest 64th, whole inches before it: 1/8, 1 1/4.
fn fraction(inches: f64) -> String {
    const PARTS: i64 = 64;
    #[expect(clippy::cast_possible_truncation, reason = "tool sizes are small")]
    let sixty_fourths = (inches * 64.0).round() as i64;
    let (whole, part) = (sixty_fourths / PARTS, sixty_fourths % PARTS);
    if part == 0 {
        return whole.to_string();
    }
    let mut denominator = PARTS;
    let mut numerator = part;
    while numerator % 2 == 0 {
        numerator /= 2;
        denominator /= 2;
    }
    if whole == 0 {
        format!("{numerator}/{denominator}")
    } else {
        format!("{whole} {numerator}/{denominator}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eighth_inch_end_mill() -> Tool {
        Tool {
            shape: ToolShape::EndMill,
            diameter: 25.4 / 8.0,
            units: Units::Inch,
            flutes: 2,
            ..Tool::default()
        }
    }

    #[test]
    fn variables_and_modifiers_fill_in_as_aspire_s_do() {
        let t = eighth_inch_end_mill();
        let e = |f: &str| expand(f, &t, "Oak", "");
        assert_eq!(e("{Tool Type} {Diameter|F}"), "End Mill 1/8");
        assert_eq!(
            e("{Tool Type|U} {Diameter} {Units Short}"),
            "END MILL 0.125 in"
        );
        assert_eq!(e("{Tool Type|L} {Num Flutes}F"), "end mill 2F");
        assert_eq!(e("{Diameter|00.000}"), "00.125");
        assert_eq!(e("{Diameter|0.0} {Material Name}"), "0.1 Oak");
        // no machine here: the variable stays as written, as do unknown ones
        assert_eq!(e("{Machine Name} {Colour}"), "{Machine Name} {Colour}");
        let metric = Tool {
            diameter: 6.0,
            units: Units::Mm,
            ..t
        };
        assert_eq!(
            expand("{Diameter}{Units Short} {Diameter|F}", &metric, "", ""),
            "6mm 15/64"
        );
    }

    #[test]
    fn fractions_reduce_and_carry_whole_inches() {
        assert_eq!(fraction(0.25), "1/4");
        assert_eq!(fraction(1.25), "1 1/4");
        assert_eq!(fraction(2.0), "2");
        assert_eq!(fraction(0.015_625), "1/64");
    }
}
