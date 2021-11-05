use std::collections::HashMap;
use std::fmt::Write;
use std::path::PathBuf;

use sam_core::{choices::Choice, identifiers::Identifier};

pub struct PreviewSkim<'a> {
    pub choices: &'a HashMap<Identifier, Choice>,
    pub preview_prefix: PathBuf,
    pub directory: PathBuf,
}

impl<'a> PreviewSkim<'a> {
    pub fn new(choices: &'a HashMap<Identifier, Choice>) -> Self {
        let preview_prefix = std::env::current_exe().expect("toto");
        let directory = std::env::current_dir().expect("toto");
        PreviewSkim {
            choices,
            preview_prefix,
            directory,
        }
    }
    pub fn preview_for_identifier(&self, identifier: &Identifier) -> String {
        let mut preview_string = String::with_capacity(50);
        write!(
            preview_string,
            "cd {} && ",
            self.directory.to_string_lossy()
        )
        .expect("Should not fail, please open a bug!:");
        write!(
            preview_string,
            "{} preview '{}' ",
            self.preview_prefix.to_string_lossy(),
            identifier
        )
        .expect("Should not fail, please open a bug!:");
        for (id, choice) in self.choices {
            write!(preview_string, " -c '{}={}' ", id, choice)
                .expect("Should not fail, please open a bug!:");
        }
        //write!(preview_string, " {{}}").expect("Should not fail, please open a bug!:");

        preview_string
    }

    pub fn preview(&self) -> String {
        let mut preview_string = String::with_capacity(50);
        write!(
            preview_string,
            "cd {} && ",
            self.directory.to_string_lossy()
        )
        .expect("Should not fail, please open a bug!:");
        write!(
            preview_string,
            "{} preview '{{}}' ",
            self.preview_prefix.to_string_lossy()
        )
        .expect("Should not fail, please open a bug!:");

        println!("Preview string: {}", preview_string);
        preview_string
    }
}
