use crate::pattern_parser::Pattern;

// TODO: Make a module with pattern_parser and toml_parser as sub modules
struct Rule {
    options: Vec<Pattern>,
}
