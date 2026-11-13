//! The mgitui binary. Terminal-only code, excluded from coverage.

fn main() {
    let names: Vec<&'static str> = wiring::layer_names();
    let _ = names;
}
