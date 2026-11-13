//! The git layer of mgitui.

/// Returns the name of this layer.
#[must_use]
pub fn layer_name() -> &'static str {
    "git"
}

#[cfg(test)]
mod tests {
    use super::layer_name;

    #[test]
    fn layer_name_is_git() {
        assert_eq!(layer_name(), "git");
    }
}
