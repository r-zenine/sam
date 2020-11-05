use crate::utils::processes::ShellCommand;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Display;
use std::hash::Hash;

lazy_static! {
    // matches the following patters :
    // - {{ some_name_1 }}
    // - {{some_name_1 }}
    // - {{ some_name_1}}
    static ref VARSRE: Regex = Regex::new("(?P<vars>\\{\\{ ?[a-zA-Z0-9_]+ ?\\}\\})").unwrap();
    static ref VARSUBSTITUTE: Regex = Regex::new("(?P<var>\\{\\{ ?(?P<varname>[a-zA-z_0-9]+) ?\\}\\})").unwrap();
}

/// parse_vars looks for patterns like '{{ var }}` in the provided str and returns a vector of VarName.
pub trait Dependencies {
    fn command(&self) -> &str;
    fn dependencies(&self) -> Vec<VarName> {
        parse_vars(self.command())
    }
    fn substitute_for_choices<'var>(
        &self,
        choices: &'var HashMap<VarName, Choice>,
    ) -> Result<String, ErrorsVarResolver> {
        let mut command = self.command().to_string();
        for dep in self.dependencies() {
            // Note , we explicitly rely on the fact that dependencies will output the dependencies as they appear in the command.
            if let Some(chce) = choices.get(&dep) {
                let re_fmt = format!(r#"(?P<var>\{{\{{ ?{} ?\}}\}})"#, dep);
                let re: Regex = Regex::new(re_fmt.as_str()).unwrap();
                command = re
                    .replace(command.as_str(), chce.value.as_str())
                    .to_string();
            } else {
                return Err(ErrorsVarResolver::NoChoiceWasAvailable(dep));
            }
        }
        Ok(command)
    }

    fn substitute_for_choices_partial<'var>(
        &self,
        choices: &'var HashMap<VarName, Choice>,
    ) -> String {
        let mut command = self.command().to_string();
        for dep in self.dependencies() {
            // Note , we explicitly rely on the fact that dependencies will output the dependencies as they appear in the command.
            if let Some(chce) = choices.get(&dep) {
                let re_fmt = format!(r#"(?P<var>\{{\{{ ?{} ?\}}\}})"#, dep);
                let re: Regex = Regex::new(re_fmt.as_str()).unwrap();
                command = re
                    .replace(command.as_str(), chce.value.as_str())
                    .to_string();
            }
        }
        command
    }
}

/// parse_vars looks for patterns like '{{ var }}` in the provided str and returns a vector of VarName.
fn parse_vars(s: &str) -> Vec<VarName> {
    VARSRE
        .captures_iter(s)
        .map(|e| e["vars"].to_owned())
        .map(VarName::new)
        .collect()
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct VarName {
    #[serde(rename(serialize = "name", deserialize = "name"))]
    inner: String,
}

impl VarName {
    /// new creates an new VarName object and it will sanitize the input.
    ///```rust
    /// use ssam::core::vars::VarName;
    /// let var = VarName::new("{{ pattern }}");
    /// assert_eq!(var.as_ref(), "pattern");
    /// let var = VarName::new("{{ pattern}}");
    /// assert_eq!(var.as_ref(), "pattern");
    /// let var = VarName::new("{{pattern }}");
    /// assert_eq!(var.as_ref(), "pattern");
    ///```
    pub fn new<IntoStr>(name: IntoStr) -> VarName
    where
        IntoStr: Into<String>,
    {
        VarName {
            inner: name
                .into()
                .replace(" ", "")
                .replace("{{", "")
                .replace("}}", ""),
        }
    }
    /// from_str builds a vector of VarName by looking at patterns like {{ var }} in the provided argument.
    ///```rust
    /// use ssam::core::vars::VarName;
    /// let vars = VarName::parse_from_str("ls -l {{location}} | grep -v {{pattern}}");
    /// assert_eq!(vars, vec![VarName::new("location"), VarName::new("pattern")]);
    ///
    ///```
    pub fn parse_from_str(s: &str) -> Vec<VarName> {
        parse_vars(s)
    }
}

impl AsRef<str> for VarName {
    fn as_ref(&self) -> &str {
        self.inner.as_str()
    }
}

impl PartialEq<&VarName> for VarName {
    fn eq(&self, other: &&VarName) -> bool {
        other.inner == self.inner
    }
}

impl Display for VarName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

// Var represent a variable with a command that can be used in an crate::core:Alias.
// Var can be static when choices is not empty or dyamic whenthe from_command is not empty
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Var {
    #[serde(flatten)]
    name: VarName,
    desc: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    choices: Vec<Choice>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    from_command: Option<String>,
}

impl Var {
    /// new creates a new var with a name a description and a static list of choices.
    pub fn new<IntoStr>(name: IntoStr, desc: IntoStr, choices: Vec<Choice>) -> Var
    where
        IntoStr: Into<String>,
    {
        Var {
            name: VarName::new(name),
            desc: desc.into(),
            choices,
            from_command: None,
        }
    }

    /// new creates a new var with a name and a description that will get it's list of choices
    /// from runing the provided command.
    pub fn from_command<IntoStr>(name: IntoStr, desc: IntoStr, from_command: IntoStr) -> Var
    where
        IntoStr: Into<String>,
    {
        Var {
            name: VarName::new(name),
            desc: desc.into(),
            choices: vec![],
            from_command: Some(from_command.into()),
        }
    }

    /// Dependencies returns the dependencies of this variable if it gets it's
    /// choices from a command.
    ///```rust
    /// use ssam::core::vars::{Var, VarName};
    /// let example = Var::from_command("name", "description", "ls -l {{ location }} | grep {{pattern}}");
    /// assert_eq!(example.dependencies(), vec![VarName::new("location"), VarName::new("pattern")]);
    ///```
    pub fn dependencies(&'_ self) -> Vec<VarName> {
        let command = self.from_command.as_deref().unwrap_or_default();
        parse_vars(command)
    }

    /// will return a valid choice for the current Var using the provided VarResolver and the
    /// HashMap of choices provided.
    /// First, this function will look into the `choices` HashMap to fill values for all the dependencies of the current
    /// `Var`and then use the resolver to get a `Choice` for the current `Var`
    pub fn resolve<'var, R>(
        &'var self,
        resolver: &'var R,
        choices: &'var HashMap<VarName, Choice>,
    ) -> Result<Choice, ErrorsVarsRepository>
    where
        R: VarResolver,
    {
        if self.from_command.is_some() {
            let command = self.substitute_for_choices(choices)?;
            resolver
                .resolve_dynamic(self.name.clone(), ShellCommand::new(command))
                .map_err(ErrorsVarsRepository::NoChoiceForVar)
        } else {
            resolver
                .resolve_static(self.name.clone(), self.choices.clone().into_iter())
                .map_err(ErrorsVarsRepository::NoChoiceForVar)
        }
    }

    fn substitute_for_choices<'var>(
        &'var self,
        choices: &'var HashMap<VarName, Choice>,
    ) -> Result<String, ErrorsVarResolver> {
        assert!(
            self.from_command.is_some(),
            "substitute for choice should only be called when self.from_command is Some(_)"
        );
        let mut command = self.from_command.clone().unwrap();
        for dep in self.dependencies() {
            // Note , we explicitly rely on the fact that dependencies will output the dependencies as they appear in the command.
            if let Some(chce) = choices.get(&dep) {
                command = VARSUBSTITUTE
                    .replace(command.as_str(), chce.value.as_str())
                    .to_string();
            } else {
                return Err(ErrorsVarResolver::NoChoiceWasAvailable(dep));
            }
        }
        Ok(command)
    }
}
impl Dependencies for Var {
    fn command(&self) -> &str {
        self.from_command.as_deref().unwrap_or("")
    }
}
impl Hash for Var {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Hash::hash(&self.name.inner, state)
    }
}

impl PartialEq for Var {
    fn eq(&self, other: &Self) -> bool {
        self.name.eq(&other.name)
    }
}

impl Borrow<VarName> for Var {
    fn borrow(&self) -> &VarName {
        &self.name
    }
}

impl Eq for Var {}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, PartialOrd, Ord, Eq)]
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

pub trait VarResolver {
    fn resolve_dynamic<CMD>(&self, var: VarName, cmd: CMD) -> Result<Choice, ErrorsVarResolver>
    where
        CMD: Into<ShellCommand<String>>;
    fn resolve_static(
        &self,
        var: VarName,
        cmd: impl Iterator<Item = Choice>,
    ) -> Result<Choice, ErrorsVarResolver>;
}

#[derive(Debug, PartialEq)]
pub enum ErrorsVarResolver {
    NoChoiceWasAvailable(VarName),
    NoChoiceWasSelected(VarName),
}

impl Display for ErrorsVarResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorsVarResolver::NoChoiceWasAvailable(name) => {
                writeln!(f, "no choice is available for var {}", name.as_ref())
            }
            ErrorsVarResolver::NoChoiceWasSelected(name) => {
                writeln!(f, "no choice was selected for var {}", name.as_ref())
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct VarsRepository {
    vars: HashSet<Var>,
}

#[derive(Debug)]
pub struct ExecutionSequence<'repository> {
    inner: Vec<&'repository VarName>,
}

impl<'repository> AsRef<[&'repository VarName]> for ExecutionSequence<'repository> {
    fn as_ref(&self) -> &[&'repository VarName] {
        self.inner.as_slice()
    }
}

// TODO tests for this.
impl VarsRepository {
    /// new creates a var Repository. this function will return an `ErrorVarRepository::ErrorMissingDependencies`
    /// if a Var provided has a dependency that is not found in the Iterator.
    pub fn new(value: impl Iterator<Item = Var>) -> Result<Self, ErrorsVarsRepository> {
        let vars: HashSet<Var> = value.collect();
        let missing: Vec<VarName> = vars
            .iter()
            .flat_map(Var::dependencies)
            .filter(|e| !vars.contains(e))
            .collect();
        if missing.is_empty() {
            Ok(VarsRepository { vars })
        } else {
            Err(ErrorsVarsRepository::MissingDependencies(missing))
        }
    }

    pub fn merge(&mut self, other: VarsRepository) {
        self.vars.extend(other.vars);
    }

    /// all_present checks whether all the provided variables in `vars`
    /// are present in the repository
    pub fn all_present(
        &'_ self,
        vars: impl Iterator<Item = VarName>,
    ) -> Result<(), ErrorsVarsRepository> {
        let missing: Vec<VarName> = vars
            .into_iter()
            .filter(|e| !self.vars.contains(e))
            .collect();
        if missing.is_empty() {
            Ok(())
        } else {
            Err(ErrorsVarsRepository::MissingDependencies(missing))
        }
    }

    /// Execution sequence returns for a given `Dep: Dependencies`
    /// an execution sequence of VARs in order to fulfill it's dependencies.
    pub fn execution_sequence<'repository, Deps>(
        &'repository self,
        dep: Deps,
    ) -> Result<ExecutionSequence<'repository>, ErrorsVarsRepository>
    where
        Deps: Dependencies,
    {
        let mut already_seen = HashSet::new();
        let mut candidates = dep.dependencies();
        let mut missing = Vec::default();
        let mut execution_seq = VecDeque::default();
        let mut push_front = 0;

        while let Some(cur) = candidates.pop() {
            if already_seen.contains(&cur) {
                continue;
            }
            if let Some(cur_var) = self.vars.get(&cur) {
                let deps = cur_var.dependencies();
                already_seen.insert(cur);
                if deps.is_empty() {
                    execution_seq.push_front(Borrow::borrow(cur_var));
                    push_front += 1;
                } else {
                    candidates.extend_from_slice(deps.as_slice());
                    execution_seq.insert(push_front, Borrow::borrow(cur_var));
                }
            } else {
                missing.push(cur);
            }
        }

        if !missing.is_empty() {
            Err(ErrorsVarsRepository::MissingDependencies(missing))
        } else {
            Ok(ExecutionSequence {
                inner: execution_seq.into_iter().collect(),
            })
        }
    }

    // choices uses the provided resolver to fetch choices for
    // the provided `ExecutionSequence`.
    pub fn choices<'repository, R>(
        &'repository self,
        resolver: &'repository R,
        vars: ExecutionSequence<'repository>,
    ) -> Result<Vec<(VarName, Choice)>, ErrorsVarsRepository>
    where
        R: VarResolver,
    {
        let mut choices: HashMap<VarName, Choice> = HashMap::new();
        for var_name in vars.inner {
            if let Some(var) = self.vars.get(var_name) {
                let choice = var.resolve(resolver, &choices)?;
                choices.insert(var_name.to_owned(), choice);
            } else {
                return Err(ErrorsVarsRepository::MissingDependencies(vec![
                    var_name.to_owned()
                ]));
            }
        }
        Ok(choices.into_iter().collect())
    }
}

#[derive(Debug, PartialEq)]
pub enum ErrorsVarsRepository {
    MissingDependencies(Vec<VarName>),
    NoChoiceForVar(ErrorsVarResolver),
}

impl Display for ErrorsVarsRepository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorsVarsRepository::MissingDependencies(vars) => {
                write!(f, "missing dependencies :")?;
                for dep in vars {
                    write!(f, " {} ", dep)?;
                }
                write!(f, "\n")
            }
            ErrorsVarsRepository::NoChoiceForVar(e) => writeln!(f, "{}", e),
        }
    }
}
impl From<ErrorsVarResolver> for ErrorsVarsRepository {
    fn from(v: ErrorsVarResolver) -> Self {
        ErrorsVarsRepository::NoChoiceForVar(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fixtures::*;
    use maplit::hashmap;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    #[test]
    fn test_varname_new() {
        let cases: Vec<(VarName, &'static str)> = vec![
            (VarName::new("{{ toto }}"), "toto"),
            (VarName::new("{{ toto}}"), "toto"),
            (VarName::new("{{toto }}"), "toto"),
            (VarName::new("{{toto}}"), "toto"),
        ];
        for (case, result) in cases {
            assert_eq!(&case.inner, result);
        }
    }

    #[test]
    fn test_hashes_are_equal() {
        let mut hasher = DefaultHasher::new();
        let mut hasher_2 = DefaultHasher::new();
        let var_name = VAR_LISTING_NAME.clone();
        let var = VAR_LISTING.clone();
        var_name.hash(&mut hasher);
        var.hash(&mut hasher_2);
        assert_eq!(hasher.finish(), hasher_2.finish());
    }

    #[test]
    fn test_parse_vars() {
        assert_eq!(
            parse_vars(VAR_LISTING_COMMAND.as_str()),
            VAR_LISTING_DEPS.clone(),
        )
    }

    #[test]
    fn test_var_dependencies() {
        assert_eq!(VAR_LISTING.dependencies(), VAR_LISTING_DEPS.clone());
    }

    #[test]
    fn test_substitute_for_choices() {
        // case 1: all is good.
        let choices = hashmap! {
            VAR_DIRECTORY_NAME.clone() => VAR_DIRECTORY_CHOICE_1.clone(),
            VAR_PATTERN_NAME.clone() => VAR_PATTERN_CHOICE_2.clone(),
        };

        let var = VAR_LISTING.clone();
        let r = var.substitute_for_choices(&choices);
        let output = format!(
            "ls -l {} |grep -v {}",
            VAR_DIRECTORY_CHOICE_1.value, VAR_PATTERN_CHOICE_2.value
        );
        assert!(r.is_ok());
        assert_eq!(r.unwrap(), output);
        // case 2: we are missing a var choice.
        let missing_choices = hashmap! {
            VAR_PATTERN_NAME.clone() => VAR_PATTERN_CHOICE_2.clone(),
        };
        let r2 = var.substitute_for_choices(&missing_choices);
        assert!(r2.is_err());
        assert_eq!(
            ErrorsVarResolver::NoChoiceWasAvailable(VAR_DIRECTORY_NAME.clone()),
            r2.unwrap_err()
        );
    }
    #[test]
    fn test_resolve() {
        let choices = hashmap! {
            VAR_DIRECTORY_NAME.clone() => VAR_DIRECTORY_CHOICE_1.clone(),
            VAR_PATTERN_NAME.clone() => VAR_PATTERN_CHOICE_2.clone(),
        };
        let command_final = format!(
            "ls -l {} |grep -v {}",
            VAR_DIRECTORY_CHOICE_1.value, VAR_PATTERN_CHOICE_2.value
        );
        let choice_final = Choice::from_value("final_value");
        let dynamic_res = hashmap![
            command_final => choice_final.clone(),
        ];
        let static_res = hashmap![
            VAR_DIRECTORY_NAME.clone() => VAR_DIRECTORY_CHOICE_1.clone(),
            VAR_PATTERN_NAME.clone() => VAR_PATTERN_CHOICE_2.clone(),
        ];
        let resolver = StaticResolver::new(dynamic_res, static_res);
        let var1 = VAR_LISTING.clone();
        let ret_var1 = var1.resolve(&resolver, &choices);
        assert!(ret_var1.is_ok());
        assert_eq!(ret_var1.unwrap(), choice_final);
        let var2 = VAR_PATTERN.clone();
        let ret_var2 = var2.resolve(&resolver, &choices);
        assert!(ret_var2.is_ok());
        assert_eq!(ret_var2.unwrap(), VAR_PATTERN_CHOICE_2.clone());
    }
    #[test]
    fn test_var_repository_new() {
        let full = vec![
            VAR_DIRECTORY.clone(),
            VAR_LISTING.clone(),
            VAR_PATTERN.clone(),
        ];
        let repo = VarsRepository::new(full.into_iter());
        assert!(repo.is_ok());
        let missing = vec![VAR_DIRECTORY.clone(), VAR_LISTING.clone()];
        let repo_err = VarsRepository::new(missing.into_iter());
        assert!(repo_err.is_err());
        assert_eq!(
            repo_err.unwrap_err(),
            ErrorsVarsRepository::MissingDependencies(vec![VAR_PATTERN_NAME.clone()])
        );
    }
    #[test]
    fn test_var_repository_all_present() {
        let full = vec![
            VAR_DIRECTORY.clone(),
            VAR_LISTING.clone(),
            VAR_PATTERN.clone(),
        ];
        let repo = VarsRepository::new(full.into_iter()).unwrap();
        let ok = repo.all_present(vec![VAR_DIRECTORY_NAME.clone()].into_iter());
        assert!(ok.is_ok());
        let ok = repo.all_present(vec![VAR_MISSING_NAME.clone()].into_iter());
        assert!(ok.is_err());
        assert_eq!(
            ok.unwrap_err(),
            ErrorsVarsRepository::MissingDependencies(vec![VAR_MISSING_NAME.clone()])
        );
    }
    #[test]
    fn test_var_repository_execution_sequence() {
        let full = vec![
            VAR_DIRECTORY.clone(),
            VAR_LISTING.clone(),
            VAR_PATTERN.clone(),
        ];
        let repo = VarsRepository::new(full.into_iter()).unwrap();
        let seq = repo.execution_sequence(VAR_LISTING.clone());
        assert!(seq.is_ok());
        let seq = repo.execution_sequence(VAR_USE_LISTING.clone());
        assert!(seq.is_ok());
        let expected = vec![
            VAR_DIRECTORY_NAME.clone(),
            VAR_PATTERN_NAME.clone(),
            VAR_LISTING_NAME.clone(),
        ];
        assert_eq!(expected.iter().as_slice(), seq.unwrap().as_ref());
    }
    #[test]
    fn test_var_repository_choices() {
        let choice_final = Choice::from_value("final_value");
        let command_final = format!(
            "ls -l {} |grep -v {}",
            VAR_DIRECTORY_CHOICE_1.value, VAR_PATTERN_CHOICE_2.value
        );
        let dynamic_res = hashmap![
            command_final => choice_final.clone(),
        ];
        let static_res = hashmap![
            VAR_DIRECTORY_NAME.clone() => VAR_DIRECTORY_CHOICE_1.clone(),
            VAR_PATTERN_NAME.clone() => VAR_PATTERN_CHOICE_2.clone(),
        ];
        let resolver = StaticResolver::new(dynamic_res, static_res);
        let full = vec![
            VAR_DIRECTORY.clone(),
            VAR_LISTING.clone(),
            VAR_PATTERN.clone(),
        ];
        let repo = VarsRepository::new(full.into_iter()).unwrap();
        let seq = repo.execution_sequence(VAR_USE_LISTING.clone()).unwrap();
        let res = repo.choices(&resolver, seq);
        assert!(res.is_ok());
        let expected = vec![
            (VAR_PATTERN_NAME.clone(), VAR_PATTERN_CHOICE_2.clone()),
            (VAR_LISTING_NAME.clone(), choice_final),
            (VAR_DIRECTORY_NAME.clone(), VAR_DIRECTORY_CHOICE_1.clone()),
        ]
        .sort();
        assert_eq!(res.unwrap().sort(), expected);
    }
    struct StaticResolver {
        dynamic_res: HashMap<String, Choice>,
        static_res: HashMap<VarName, Choice>,
    }
    impl StaticResolver {
        fn new(dynamic_res: HashMap<String, Choice>, static_res: HashMap<VarName, Choice>) -> Self {
            StaticResolver {
                dynamic_res,
                static_res,
            }
        }
    }
    impl VarResolver for StaticResolver {
        fn resolve_dynamic<CMD>(&self, var: VarName, cmd: CMD) -> Result<Choice, ErrorsVarResolver>
        where
            CMD: Into<ShellCommand<String>>,
        {
            let sh_cmd = Into::<ShellCommand<String>>::into(cmd);
            let query = sh_cmd.value();
            self.dynamic_res
                .get(query)
                .map(|e| e.to_owned())
                .ok_or(ErrorsVarResolver::NoChoiceWasAvailable(var))
        }
        fn resolve_static(
            &self,
            var: VarName,
            _cmd: impl Iterator<Item = Choice>,
        ) -> Result<Choice, ErrorsVarResolver> {
            self.static_res
                .get(&var)
                .map(|c| c.to_owned())
                .ok_or(ErrorsVarResolver::NoChoiceWasSelected(var))
        }
    }
    mod fixtures {
        use super::*;
        use lazy_static::lazy_static;
        lazy_static! {
            pub static ref VAR_USE_LISTING_NAME: VarName = VarName::new("use_listing");
            pub static ref VAR_USE_LISTING_COMMAND: String =
                String::from("cat {{listing}} |grep -v {{pattern}}");
            pub static ref VAR_USE_LISTING_DESC: String = String::from(
                "output element in {{listing}} and discards everything that matches {{pattern}}"
            );
            pub static ref VAR_USE_LISTING_CHOICES: Vec<Choice> = vec![];
            pub static ref VAR_USE_LISTING_DEPS: Vec<VarName> =
                vec![VarName::new("listing"), VarName::new("pattern")];
            pub static ref VAR_USE_LISTING: Var = Var {
                name: VAR_USE_LISTING_NAME.clone(),
                from_command: Some(VAR_USE_LISTING_COMMAND.clone()),
                desc: VAR_USE_LISTING_DESC.clone(),
                choices: VAR_USE_LISTING_CHOICES.clone(),
            };
            pub static ref VAR_LISTING_NAME: VarName = VarName::new("listing");
            pub static ref VAR_LISTING_COMMAND: String =
                String::from("ls -l {{directory}} |grep -v {{pattern}}");
            pub static ref VAR_LISTING_DESC: String = String::from(
                "list element in {{directory}} and discards everything that matches {{pattern}}"
            );
            pub static ref VAR_LISTING_CHOICES: Vec<Choice> = vec![];
            pub static ref VAR_LISTING_DEPS: Vec<VarName> =
                vec![VarName::new("directory"), VarName::new("pattern")];
            pub static ref VAR_LISTING: Var = Var {
                name: VAR_LISTING_NAME.clone(),
                from_command: Some(VAR_LISTING_COMMAND.clone()),
                desc: VAR_LISTING_DESC.clone(),
                choices: VAR_LISTING_CHOICES.clone(),
            };
            pub static ref VAR_DIRECTORY_NAME: VarName = VarName::new("directory");
            pub static ref VAR_DIRECTORY_DESC: String =
                String::from("A list of safe directory paths where to perform commands.");
            pub static ref VAR_DIRECTORY_CHOICE_1: Choice =
                Choice::new("/var/log", Some("logs directory"));
            pub static ref VAR_DIRECTORY_CHOICE_2: Choice =
                Choice::new("/home", Some("users directory"));
            pub static ref VAR_DIRECTORY_CHOICES: Vec<Choice> = vec![
                VAR_DIRECTORY_CHOICE_1.clone(),
                VAR_DIRECTORY_CHOICE_2.clone()
            ];
            pub static ref VAR_DIRECTORY: Var = Var {
                name: VAR_DIRECTORY_NAME.clone(),
                from_command: None,
                desc: VAR_DIRECTORY_DESC.clone(),
                choices: VAR_DIRECTORY_CHOICES.clone(),
            };
            pub static ref VAR_PATTERN_NAME: VarName = VarName::new("pattern");
            pub static ref VAR_PATTERN_DESC: String = String::from("A black list of patterns");
            pub static ref VAR_PATTERN_CHOICE_1: Choice =
                Choice::new("service", Some("service pattern"));
            pub static ref VAR_PATTERN_CHOICE_2: Choice =
                Choice::new("ryad", Some("users ryad pattern"));
            pub static ref VAR_PATTERN_CHOICES: Vec<Choice> =
                vec![VAR_PATTERN_CHOICE_1.clone(), VAR_PATTERN_CHOICE_2.clone()];
            pub static ref VAR_PATTERN: Var = Var {
                name: VAR_PATTERN_NAME.clone(),
                from_command: None,
                desc: VAR_PATTERN_DESC.clone(),
                choices: VAR_PATTERN_CHOICES.clone(),
            };
            pub static ref VAR_PATTERN_2_NAME: VarName = VarName::new("pattern2");
            pub static ref VAR_MISSING_NAME: VarName = VarName::new("missing");
            pub static ref VAR_MISSING_COMMAND: String =
                String::from("ls -l {{directory}} |grep -v {{pattern2}}");
            pub static ref VAR_MISSING_DESC: String = String::from(
                "list element in {{directory}} and discards everything that matches {{pattern}}"
            );
            pub static ref VAR_MISSING_CHOICES: Vec<Choice> = vec![];
            pub static ref VAR_MISSING_DEPS: Vec<VarName> =
                vec![VarName::new("directory"), VarName::new("pattern2")];
            pub static ref VAR_MISSING: Var = Var {
                name: VAR_MISSING_NAME.clone(),
                from_command: Some(VAR_MISSING_COMMAND.clone()),
                desc: VAR_MISSING_DESC.clone(),
                choices: VAR_MISSING_CHOICES.clone(),
            };
        }
    }
}
