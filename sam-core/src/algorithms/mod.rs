mod dependency_resolution;

pub use dependency_resolution::choice_for_var;
pub use dependency_resolution::choices_for_execution_sequence;
pub use dependency_resolution::execution_sequence_for_dependencies;
pub use dependency_resolution::ErrorDependencyResolution;
pub use dependency_resolution::VarsCollection;
pub use dependency_resolution::VarsDefaultValues;

#[cfg(test)]
pub mod mocks {
    use super::dependency_resolution;
    pub use dependency_resolution::mocks::*;
}
