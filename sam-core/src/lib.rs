pub mod aliases;
pub mod aliases_repository;
pub mod choices;
pub mod commands;
pub mod dependencies;
pub mod identifiers;
pub mod namespaces;
pub mod processes;
pub mod vars;
pub mod vars_repository;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
