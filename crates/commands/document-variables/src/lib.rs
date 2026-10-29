//! Document Variables: named values kept with the job, used in number fields' sums as
//! `{name}` (Aspire's help: Document Variables). A name starts with a letter and then has
//! letters, digits and underscores.

use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub struct DocumentVariables;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Variable {
    pub name: String,
    pub value: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// Every variable; one taken off the list is deleted
    pub variables: Vec<Variable>,
}

impl Command for DocumentVariables {
    const ID: &'static str = "document_variables";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Document Variables",
            group: "Edit",
            icon: "",
            shortcut: "",
        }
    }

    fn initial_params(doc: &Document) -> Params {
        let variables = doc
            .variables
            .iter()
            .map(|(name, &value)| Variable {
                name: name.clone(),
                value,
            })
            .collect();
        Params { variables }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let mut variables = BTreeMap::new();
        for v in &p.variables {
            let name = v.name.trim();
            let mut chars = name.chars();
            let named = chars.next().is_some_and(|c| c.is_ascii_alphabetic())
                && chars.all(|c| c.is_ascii_alphanumeric() || c == '_');
            if !named {
                return Err(CommandError::InvalidParams(format!(
                    "\"{name}\": a name starts with a letter, then letters, digits or _"
                )));
            }
            if variables.insert(name.to_owned(), v.value).is_some() {
                return Err(CommandError::InvalidParams(format!(
                    "{name} is named twice"
                )));
            }
        }
        let mut c = Change::builder(doc);
        c.set_variables(variables);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{after, rejection};

    fn var(name: &str, value: f64) -> Variable {
        Variable {
            name: name.into(),
            value,
        }
    }

    fn with_depth() -> Document {
        after::<DocumentVariables>(&Params {
            variables: vec![var("depth", 6.0)],
        })
    }

    cut_craft_core::contract_tests!(DocumentVariables, with_depth);

    #[test]
    fn keeps_the_variables_with_the_job() {
        let doc = after::<DocumentVariables>(&Params {
            variables: vec![var(" depth ", 6.0), var("pass_2", 1.5)],
        });
        assert_eq!(doc.variables["depth"], 6.0);
        assert_eq!(doc.variables["pass_2"], 1.5);
        assert_eq!(DocumentVariables::initial_params(&doc).variables.len(), 2);
    }

    #[test]
    fn names_start_with_a_letter_and_are_not_repeated() {
        for bad in ["2nd", "a b", "", "x-y"] {
            let why = rejection::<DocumentVariables>(&Params {
                variables: vec![var(bad, 1.0)],
            });
            assert!(why.contains("starts with a letter"), "{bad}: {why}");
        }
        let twice = rejection::<DocumentVariables>(&Params {
            variables: vec![var("d", 1.0), var("d", 2.0)],
        });
        assert!(twice.contains("named twice"));
    }
}
