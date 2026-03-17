use crate::entities::aliases::Alias;
use crate::entities::commands::Command;
use crate::entities::namespaces::{Namespace, NamespaceUpdater};
use crate::entities::vars::Var;
use std::collections::HashSet;

pub use crate::entities::identifiers::Identifier;

/// Generate synthetic discovery aliases for vars that have discover: true.
///
/// For each var with discover: true:
/// - Static vars (with choices): create alias with "printf '%s\n' value1 value2 ..." command
/// - Dynamic vars (with from_command): create alias with the original from_command
/// - Input vars: skipped (no discovery value available)
pub fn generate_discover_aliases(vars: &HashSet<Var>) -> Vec<Alias> {
    vars.iter()
        .filter(|v| v.discover())
        .filter_map(|var| {
            let discover_name = format!("discover_{}", var.name().name());
            let discover_desc = format!("discover: {}", var.name().name());

            let cmd = if !var.choices().is_empty() {
                // Static var: generate printf command
                let values: Vec<String> = var.choices().iter().map(|c| c.value().to_string()).collect();
                format!("printf '%s\\n' {}", values.join(" "))
            } else if var.is_command() {
                // Dynamic var: reuse from_command
                var.command().to_string()
            } else {
                // Input var: skip
                return None;
            };

            let mut alias = Alias::new(&discover_name, &discover_desc, &cmd);
            if let Some(ns) = var.namespace() {
                NamespaceUpdater::update(&mut alias, ns);
            }
            Some(alias)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::choices::Choice;

    // Test helper: create a discover-enabled var (simulating deserialization with discover: true)
    fn create_discoverable_static_var(
        name: &str,
        desc: &str,
        choices: Vec<Choice>,
    ) -> Var {
        // Create through serde roundtrip to properly set discover field
        let yaml = format!(
            "name: {}\ndesc: {}\ndiscover: true\nchoices:\n{}",
            name,
            desc,
            choices
                .iter()
                .map(|c| format!("  - value: {}", c.value()))
                .collect::<Vec<_>>()
                .join("\n")
        );
        serde_yaml::from_str(&yaml).expect("should deserialize discoverable var")
    }

    fn create_discoverable_dynamic_var(name: &str, desc: &str, cmd: &str) -> Var {
        let yaml = format!(
            "name: {}\ndesc: {}\ndiscover: true\nfrom_command: {}",
            name, desc, cmd
        );
        serde_yaml::from_str(&yaml).expect("should deserialize discoverable var")
    }

    fn create_discoverable_input_var(name: &str, desc: &str, prompt: &str) -> Var {
        let yaml = format!(
            "name: {}\ndesc: {}\ndiscover: true\nfrom_input: \"{}\"",
            name, desc, prompt
        );
        serde_yaml::from_str(&yaml).expect("should deserialize discoverable var")
    }

    #[test]
    fn test_discover_static_var() {
        let mut vars = HashSet::new();
        let var = create_discoverable_static_var(
            "cluster",
            "Kafka clusters",
            vec![
                Choice::new("prod", Some("production")),
                Choice::new("staging", Some("staging")),
            ],
        );
        vars.insert(var);

        let aliases = generate_discover_aliases(&vars);
        assert_eq!(aliases.len(), 1);
        assert_eq!(aliases[0].name(), "discover_cluster");
        assert!(aliases[0].alias().contains("printf"));
        assert!(aliases[0].alias().contains("prod"));
        assert!(aliases[0].alias().contains("staging"));
    }

    #[test]
    fn test_discover_dynamic_var() {
        let mut vars = HashSet::new();
        let var = create_discoverable_dynamic_var(
            "namespace",
            "K8s namespaces",
            "kubectl get namespaces -o jsonpath='{.items[*].metadata.name}'",
        );
        vars.insert(var);

        let aliases = generate_discover_aliases(&vars);
        assert_eq!(aliases.len(), 1);
        assert_eq!(aliases[0].name(), "discover_namespace");
        assert_eq!(
            aliases[0].alias(),
            "kubectl get namespaces -o jsonpath='{.items[*].metadata.name}'"
        );
    }

    #[test]
    fn test_discover_input_var_skipped() {
        let mut vars = HashSet::new();
        let var = create_discoverable_input_var("user_input", "User input", "Enter value: ");
        vars.insert(var);

        let aliases = generate_discover_aliases(&vars);
        assert_eq!(aliases.len(), 0); // input var should be skipped
    }

    #[test]
    fn test_discover_false_skipped() {
        let mut vars = HashSet::new();
        let var = Var::new("cluster", "Clusters", vec![Choice::new("prod", None)]);
        // discover defaults to false
        vars.insert(var);

        let aliases = generate_discover_aliases(&vars);
        assert_eq!(aliases.len(), 0);
    }

    #[test]
    fn test_discover_with_namespace() {
        let mut vars = HashSet::new();
        let yaml = r#"
name: cluster
namespace: kafka
desc: Clusters
discover: true
choices:
  - value: prod
"#;
        let var: Var = serde_yaml::from_str(yaml).expect("should deserialize");
        vars.insert(var);

        let aliases = generate_discover_aliases(&vars);
        assert_eq!(aliases.len(), 1);
        assert_eq!(aliases[0].namespace(), Some("kafka"));
        assert_eq!(aliases[0].name(), "discover_cluster");
    }
}
