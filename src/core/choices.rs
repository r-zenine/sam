use serde::{Deserialize, Serialize};
use std::fmt::Display;
#[derive(Debug, Clone ,Default, Serialize, Deserialize, PartialEq, PartialOrd, Ord, Eq)]
pub struct Choice {
    value: String,
    desc: Option<String>,
}

impl Choice {
    pub fn new<IntoStr>(value: IntoStr, desc: Option<IntoStr>) -> Choice
    where
        String: From<IntoStr>,
    {
        Choice {
            value: value.into(),
            desc: desc.map(String::from),
        }
    }
    pub fn from_value<IntoStr>(value: IntoStr) -> Choice
    where
        String: From<IntoStr>,
    {
        Choice {
            value: value.into(),
            desc: None,
        }
    }
    pub fn value(&'_ self) -> &'_ str {
        self.value.as_str()
    }
    pub fn desc(&'_ self) -> Option<&'_ str> {
        self.desc.as_deref()
    }
}

impl Display for Choice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}
