use crate::core::aliases::Alias;
use crate::core::identifiers::Identifier;
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;
use std::ops::Range;
use thiserror::Error;

lazy_static! {
    // matches the following patters :
    // - [[ some_name_1 ]]
    // - [[some_name_1 ]]
    // - [[ some_name_1]]
    static ref ALIASESRE: Regex = Regex::new("(?P<alias>\\[\\[ ?[a-zA-Z0-9_:]+ ?\\]\\])").unwrap();
}

#[derive(Debug)]
pub struct AliasesRepository {
    aliases: HashMap<Identifier, Alias>,
}

impl AliasesRepository {
    pub fn new(aliases: impl Iterator<Item = Alias>) -> Result<Self, ErrorsAliasesRepository> {
        let mut mp = HashMap::new();
        for alias in aliases {
            let id = alias.identifier();
            mp.insert(id, alias);
        }
        let mut mpf = HashMap::new();
        for (key, alias) in mp.iter() {
            let t_alias = Self::substitute_alias_defs(&alias, &mp)?;
            mpf.insert(key.clone(), t_alias);
        }
        Ok(AliasesRepository { aliases: mpf })
    }

    pub fn aliases(&self) -> Vec<Alias> {
        self.aliases.values().map(Alias::clone).collect()
    }

    fn substitute_alias_defs(
        alias: &Alias,
        aliases: &HashMap<Identifier, Alias>,
    ) -> Result<Alias, ErrorsAliasesRepository> {
        let mut t_alias = alias.clone();
        let deps = Self::parse(&alias);
        if deps.len() > 0 {
            let alias_str = alias.alias();
            let mut alias_parts = vec![];
            for (range, id) in deps.iter() {
                if let Some(repl_alias) = aliases.get(id) {
                    let prefix = &alias_str[0..range.start];
                    let suffix = &alias_str[range.end..];
                    alias_parts.push(prefix.to_string());
                    alias_parts.push(repl_alias.sanitized_alias());
                    alias_parts.push(suffix.to_string());
                } else {
                    return Err(ErrorsAliasesRepository::MissingDependencies(
                        alias.identifier().clone(),
                        id.clone(),
                    ));
                }
            }
            t_alias.update(alias_parts.join(""));
        }
        Ok(t_alias)
    }

    fn parse(alias: &Alias) -> Vec<(Range<usize>, Identifier)> {
        let default_namespace = alias.identifier().namespace;
        ALIASESRE
            .captures_iter(alias.alias())
            .flat_map(|e| e.name("alias"))
            .map(|e| (e.range(), Identifier::maybe_namespace(e.as_str())))
            .map(|(r, (n, ns))| {
                (
                    r,
                    Identifier::with_namespace(n, ns.or(default_namespace.clone())),
                )
            })
            .collect()
    }
}

#[derive(Debug, Error)]
pub enum ErrorsAliasesRepository {
    #[error("Alias '{0}' has a missing dependency: '{1}'")]
    MissingDependencies(Identifier, Identifier),
}

#[cfg(test)]
mod tests {
    use super::AliasesRepository;
    use crate::core::aliases::fixtures::*;
    use crate::core::aliases::Alias;
    use crate::core::identifiers::fixtures::*;
    use maplit::hashmap;
    use std::ops::Range;
    #[test]
    fn parse_test() {
        let a = Alias::new("name", "desc", "ls -l 1| [[ toto ]] | [[ ns::toto]]");
        let parsed = AliasesRepository::parse(&a);
        assert!(parsed.len() == 2);
        assert!(parsed[0].0 == Range { start: 9, end: 19 });
        assert!(parsed[0].1.name() == "toto");
        assert!(parsed[0].1.namespace == None);
        assert!(parsed[1].0 == Range { start: 22, end: 35 });
        assert!(parsed[1].1.name() == "toto");
        assert!(parsed[1].1.namespace == Some("ns".to_string()));
    }

    #[test]
    fn substitute_alias_defs() {
        let aliases = hashmap! {
           ALIAS_LS_DIR_NAME.clone() => ALIAS_LS_DIR.clone(),
           ALIAS_GREP_DIR_NAME.clone() => ALIAS_GREP_DIR.clone(),
        };
        let a = ALIAS_GREP_DIR.clone();
        let up_alias = AliasesRepository::substitute_alias_defs(&a, &aliases);
        assert!(up_alias.is_ok());
        assert_eq!(
            "ls {{ dirs::directory }}|grep {{ pattern }}",
            up_alias.unwrap().alias()
        );
        let a_no_ns = ALIAS_GREP_DIR_NO_NS.clone();
        let up_alias_no_ns = AliasesRepository::substitute_alias_defs(&a_no_ns, &aliases);
        assert!(up_alias_no_ns.is_ok());
        assert_eq!(
            "ls {{ dirs::directory }}| grep {{ pattern }}",
            up_alias_no_ns.unwrap().alias()
        );
    }

    #[test]
    fn new() {
        let aliases = vec![
            ALIAS_LS_DIR.clone(),
            ALIAS_GREP_DIR.clone(),
            ALIAS_GREP_DIR_NO_NS.clone(),
        ];

        let ar = AliasesRepository::new(aliases.into_iter()).unwrap();
        let alias = ar.aliases.get(&ALIAS_GREP_DIR_NAME.clone());
        assert!(alias.is_some());
        assert_eq!(
            "ls {{ dirs::directory }}| grep {{ pattern }}",
            alias.unwrap().alias()
        );
    }
}
