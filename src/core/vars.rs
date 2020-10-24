use serde::{Deserialize, Serialize};
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct Var {
    name: String,
    desc: String,
    choices: Vec<Choice>,
}

impl Var {
    pub fn new<IntoStr>(name: IntoStr, desc: IntoStr, choices: Vec<Choice>) -> Var
    where
        IntoStr: Into<String>,
    {
        Var {
            name: name.into(),
            desc: desc.into(),
            choices: choices,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Choice {
    value: String,
    desc: String,
}

impl Choice {
    pub fn new<IntoStr>(value: IntoStr, desc: IntoStr) -> Choice
    where
        IntoStr: Into<String>,
    {
        Choice {
            value: value.into(),
            desc: desc.into(),
        }
    }
}
