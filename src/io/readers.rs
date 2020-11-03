use crate::core::aliases::Alias;
use crate::core::scripts::Script;
use crate::core::vars::{Choice, ErrorsVarsRepository, Var, VarsRepository};
use std::fmt::Display;
use std::fs::read_dir;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

pub fn read_aliases_from_file(path: &'_ Path) -> Result<Vec<Alias>, ErrorsAliasRead> {
    let f = File::open(path)?;
    let buf = BufReader::new(f);
    read_aliases(buf)
}

fn read_aliases<T>(r: T) -> Result<Vec<Alias>, ErrorsAliasRead>
where
    T: Read,
{
    serde_yaml::from_reader(r).map_err(ErrorsAliasRead::from)
}

pub fn read_choices<T>(r: T) -> Result<Vec<Choice>, ErrorsChoiceRead>
where
    T: BufRead,
{
    let mut out = vec![];
    for line_r in r.lines() {
        let line = line_r?;
        let splits: Vec<&str> = line.split('\t').collect();
        let value_o = splits.get(0).map(|e| e.to_string());
        let desc = splits.get(1).map(|e| e.to_string());
        if let Some(value) = value_o {
            out.push(Choice::new(value, desc));
        }
    }
    Ok(out)
}

pub fn read_vars_repository(path: &'_ Path) -> Result<VarsRepository, ErrorsVarRead> {
    let f = File::open(path)?;
    let buf = BufReader::new(f);
    let vars = read_vars(buf)?;
    VarsRepository::new(vars.into_iter()).map_err(|e| e.into())
}

fn read_vars<T>(r: T) -> Result<Vec<Var>, ErrorsVarRead>
where
    T: Read,
{
    serde_yaml::from_reader(r).map_err(ErrorsVarRead::from)
}

pub fn read_scripts<'a>(path: &'a Path) -> Result<Vec<Script>, ErrorScriptRead> {
    if path.is_dir() {
        let mut out = vec![];
        for entry in read_dir(path)? {
            let current_path = entry?.path();
            if current_path.is_file() {
                if let Ok(script) = read_script(current_path) {
                    out.push(script);
                }
            }
        }
        Ok(out)
    } else {
        Err(ErrorScriptRead::ScriptDirNotDirectory(
            path.display().to_string(),
        ))
    }
}

fn read_script(path: PathBuf) -> Result<Script, ErrorScriptRead> {
    let r = File::open(&path)?;
    let description = BufReader::new(r)
        .lines()
        .take(2)
        .skip(1)
        .next()
        .transpose()?;

    let name = path
        .file_name()
        .and_then(|e| e.to_str())
        .map(|e| e.to_string())
        .ok_or(ErrorScriptRead::ReadScriptName(format!(
            "could not extract file name from path {}",
            path.display()
        )))?;

    Ok(Script::new(name, description, path))
}
#[derive(Debug)]
pub enum ErrorsAliasRead {
    AliasSerde(serde_yaml::Error),
    AliasIO(std::io::Error),
}

impl From<std::io::Error> for ErrorsAliasRead {
    fn from(v: std::io::Error) -> Self {
        ErrorsAliasRead::AliasIO(v)
    }
}

impl From<serde_yaml::Error> for ErrorsAliasRead {
    fn from(v: serde_yaml::Error) -> Self {
        ErrorsAliasRead::AliasSerde(v)
    }
}

impl Display for ErrorsAliasRead {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorsAliasRead::AliasIO(err) => {
                writeln!(f, "while trying to read aliases got error {}", err)
            }
            ErrorsAliasRead::AliasSerde(err) => {
                writeln!(f, "while trying to deserialize aliases got error {}", err)
            }
        }
    }
}
#[derive(Debug)]
pub enum ErrorsVarRead {
    VarsSerde(serde_yaml::Error),
    VarIO(std::io::Error),
    VarsRepositoryInitialisation(ErrorsVarsRepository),
}

#[derive(Debug)]
pub enum ErrorsChoiceRead {
    ChoiceIO(std::io::Error),
}

impl From<std::io::Error> for ErrorsChoiceRead {
    fn from(v: std::io::Error) -> Self {
        ErrorsChoiceRead::ChoiceIO(v)
    }
}

impl From<ErrorsVarsRepository> for ErrorsVarRead {
    fn from(v: ErrorsVarsRepository) -> Self {
        ErrorsVarRead::VarsRepositoryInitialisation(v)
    }
}

impl From<std::io::Error> for ErrorsVarRead {
    fn from(v: std::io::Error) -> Self {
        ErrorsVarRead::VarIO(v)
    }
}

impl From<serde_yaml::Error> for ErrorsVarRead {
    fn from(v: serde_yaml::Error) -> Self {
        ErrorsVarRead::VarsSerde(v)
    }
}

impl Display for ErrorsVarRead {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorsVarRead::VarsSerde(e) => writeln!(f, "parsing error for vars file\n -> {}", e),
            ErrorsVarRead::VarIO(e) => writeln!(f, "while reading the vars file got error {}", e),
            ErrorsVarRead::VarsRepositoryInitialisation(e) => {
                writeln!(f, "while validating the vars file got error {}", e)
            }
        }
    }
}

#[derive(Debug)]
pub enum ErrorScriptRead {
    ReadScriptName(String),
    ScriptDirNotDirectory(String),
    ReadScriptContent(std::io::Error),
}

impl Display for ErrorScriptRead {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorScriptRead::ReadScriptName(err) => {
                writeln!(f, "while reading script name got error {}", err)
            }
            ErrorScriptRead::ReadScriptContent(err) => {
                writeln!(f, "while reading script content got error {}", err)
            }
            ErrorScriptRead::ScriptDirNotDirectory(path) => writeln!(
                f,
                "the path provided to read scripts in not a directory. path was : {}",
                path
            ),
        }
    }
}
impl From<std::io::Error> for ErrorScriptRead {
    fn from(v: std::io::Error) -> Self {
        ErrorScriptRead::ReadScriptContent(v)
    }
}

#[cfg(test)]
mod tests {
    use super::{read_aliases, read_scripts, read_vars};
    use crate::core::aliases::Alias;
    use crate::core::scripts::Script;
    use crate::core::vars::{Choice, Var};
    use std::env;
    use std::fs::File;
    use std::io;
    use std::io::BufReader;
    use std::io::Write;
    use std::panic;
    use std::path::{Path, PathBuf};
    use tempdir::TempDir;
    #[test]
    fn test_read_vars() {
        let vars_str = "
            - desc: 'desc1'
              name: 'name1'
              choices:
              - value: 'val1'
                desc: val1 description
            - desc: 'desc2'
              name: 'name2'
              choices:
              - value: 'val2'
                desc: val2 description
              - value: 'val1'
                desc: val1 description"
            .as_bytes();

        let r = BufReader::new(vars_str);
        let vars_r = read_vars(r);
        assert!(vars_r.is_ok());
        let vars = vars_r.unwrap();
        assert_eq!(vars.len(), 2);
        let exp_choices_1 = vec![Choice::new("val1", Some("val1 description"))];
        let exp_choices_2 = vec![
            Choice::new("val2", Some("val2 description")),
            Choice::new("val1", Some("val1 description")),
        ];
        let exp_var_listing = Var::new("name1", "desc1", exp_choices_1);
        let exp_var_2 = Var::new("name2", "desc2", exp_choices_2);
        assert_eq!(vars, vec![exp_var_listing, exp_var_2]);
    }
    #[test]
    fn test_read_aliases() {
        let aliase_str = "
            - desc: 'desc1'
              name: 'name1'
              alias: 'alias1'
            - desc: 'desc2'
              name: 'name2'
              alias: 'alias2'"
            .as_bytes();
        let r = BufReader::new(aliase_str);
        let aliases_r = read_aliases(r);
        assert!(aliases_r.is_ok());
        let aliases = aliases_r.unwrap();
        assert_eq!(aliases.len(), 2);
        assert_eq!(aliases[0], Alias::new("name1", "desc1", "alias1"));
        assert_eq!(aliases[1], Alias::new("name2", "desc2", "alias2"));

        let aliase_str = "
            - desc: 'desc1'
              alias: 'alias1'
            - desc: 'desc2'
              alias: 'alias2'"
            .as_bytes();
        let r = BufReader::new(aliase_str);
        let aliases_r = read_aliases(r);
        assert!(aliases_r.is_err());
    }

    #[test]
    fn test_read_scripts() {
        let temp_dir_r =
            TempDir::new_in(env::temp_dir(), "saam_tests").expect("Can't create temp directory");
        let script_content = ["#!/bin/sh", "# some description of the script."];
        let script_path = prepare_mock_script(temp_dir_r.as_ref(), &script_content[..])
            .expect("Can't prepare test environment");

        let scripts =
            read_scripts(temp_dir_r.as_ref()).expect("read scripts failed in an expected way");

        assert_eq!(scripts.len(), 1);
        let file_name = script_path
            .file_name()
            .and_then(|e| e.to_os_string().into_string().ok())
            .expect("something weird happened");

        assert_eq!(
            scripts[0],
            Script::new(file_name, Some(script_content[1]), script_path.clone())
        )
    }

    fn prepare_mock_script(temp_dir: &'_ Path, content: &[&'_ str]) -> io::Result<PathBuf> {
        let rnd: u16 = rand::random();
        let temp_file = temp_dir.join(format!("script_{}", rnd));
        let mut f = File::create(&temp_file)?;

        for line in content {
            writeln!(f, "{}", *line)?;
        }
        Ok(temp_file)
    }
}
