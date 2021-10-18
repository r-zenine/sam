mod aliases_repository;
mod vars_repository;

pub use aliases_repository::{AliasesRepository, ErrorsAliasesRepository};
pub use vars_repository::{ErrorsVarsRepository, VarsRepository};

pub mod fixtures {
    pub use super::vars_repository::fixtures as vars_repository;
}
