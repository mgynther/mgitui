//! The logic layer of mgitui.

/// Returns the name of this layer.
#[must_use]
pub fn layer_name() -> &'static str {
    "logic"
}

#[cfg(test)]
mod tests {
    use super::layer_name;

    #[test]
    fn layer_name_is_logic() {
        assert_eq!(layer_name(), "logic");
    }
}
