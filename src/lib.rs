pub mod cli;
pub mod data;
pub mod matcher;

/// Ruby's default `String#chomp` removes one record separator.
pub fn ruby_chomp(input: &str) -> String {
    if let Some(value) = input.strip_suffix("\r\n") {
        value.to_owned()
    } else if let Some(value) = input.strip_suffix(['\n', '\r']) {
        value.to_owned()
    } else {
        input.to_owned()
    }
}
