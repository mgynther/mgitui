//! The ui layer of mgitui.

/// Returns the name of this layer.
#[must_use]
pub fn layer_name() -> &'static str {
    "ui"
}

#[cfg(test)]
mod tests {
    use super::layer_name;

    #[test]
    fn layer_name_is_ui() {
        assert_eq!(layer_name(), "ui");
    }
}
