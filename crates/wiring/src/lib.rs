//! The wiring layer of mgitui: connects the other layers and runs the app.

/// Returns the names of all layers, which proves that wiring can use every other layer.
#[must_use]
pub fn layer_names() -> Vec<&'static str> {
    vec![
        git::layer_name(),
        logic::layer_name(),
        ui::layer_name(),
        validation::layer_name(),
    ]
}

#[cfg(test)]
mod tests {
    use super::layer_names;

    #[test]
    fn all_layers_are_reachable() {
        assert_eq!(layer_names(), vec!["git", "logic", "ui", "validation"]);
    }
}
