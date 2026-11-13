//! The validation layer of mgitui.

/// Returns the name of this layer.
#[must_use]
pub fn layer_name() -> &'static str {
    "validation"
}

#[cfg(test)]
mod tests {
    use super::layer_name;

    #[test]
    fn layer_name_is_validation() {
        assert_eq!(layer_name(), "validation");
    }
}
