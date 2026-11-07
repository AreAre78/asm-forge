use trivet::{
    Parser,
    errors::{ParseError, syntax_error},
    parse_from_string,
};

#[derive(Debug)]
pub struct Pattern {
    parts: Vec<PatternPart>,
}

impl Pattern {
    pub fn from_literal(literal: String) -> Self {
        Self {
            parts: vec![PatternPart::Literal(literal)],
        }
    }
}

#[derive(Debug)]
enum PatternPart {
    Literal(String),
    Ref(PatternBinding),
}

#[derive(Debug)]
struct PatternBinding {
    name: String,
    ty: String,
}

pub fn parse_pattern(pattern: &str) -> Result<Pattern, ParseError> {
    let mut parser = parse_from_string(pattern.trim());

    let parts = parse_pattern_parts(&mut parser)?;

    Ok(Pattern { parts })
}

fn parse_pattern_parts(parser: &mut Parser) -> Result<Vec<PatternPart>, ParseError> {
    let mut parts = Vec::new();

    while !parser.is_at_eof() {
        let literal = parse_literal(parser)?;

        if !literal.is_empty() {
            parts.push(PatternPart::Literal(literal));
        }

        if parser.is_at_eof() {
            break;
        }

        parser.expect('{')?;
        let field = parse_binding(parser)?;
        parser.expect('}')?;

        parts.push(PatternPart::Ref(field));
    }

    Ok(parts)
}

fn parse_literal(parser: &mut Parser) -> Result<String, ParseError> {
    let mut literal = String::new();
    while !parser.is_at_eof() {
        let c = parser.peek();
        match c {
            '{' => {
                if parser.peek_and_consume_str("{{") {
                    literal.push('{');
                } else {
                    break;
                }
            }
            '}' => {
                parser.expect_chars(&['}', '}'])?;
                literal.push('}');
            }
            _ => {
                parser.consume();
                literal.push(c);
            }
        }
    }
    Ok(literal)
}

fn parse_binding(parser: &mut Parser) -> Result<PatternBinding, ParseError> {
    parser.consume_ws();
    let name = take_identifier(parser)?;
    parser.consume_ws();
    parser.expect(':')?;
    parser.consume_ws();
    let ty = take_identifier(parser)?;
    parser.consume_ws();
    Ok(PatternBinding { name, ty })
}

/// An identifier is defined as any positive number of consecutive letter, number or '_'
fn take_identifier(parser: &mut Parser) -> Result<String, ParseError> {
    let loc = parser.loc();
    let identifier = parser.take_while(|c| c.is_alphanumeric() || c == '_');
    if identifier.is_empty() {
        Err(syntax_error(
            loc,
            &format!("Expected identifier, but instead found {:?}", parser.peek()),
        ))
    } else {
        Ok(identifier)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_literal_and_placeholder_parts() {
        let pattern = parse_pattern("[{reg:register} + {offset:S6}]").unwrap();

        assert_eq!(pattern.parts.len(), 5);
        assert!(matches!(&pattern.parts[0], PatternPart::Literal(value) if value == "["));
        assert!(matches!(
            &pattern.parts[1],
            PatternPart::Ref(PatternBinding { name, ty }) if name == "reg" && ty == "register"
        ));
        assert!(matches!(&pattern.parts[2], PatternPart::Literal(value) if value == " + "));
        assert!(matches!(
            &pattern.parts[3],
            PatternPart::Ref(PatternBinding { name, ty }) if name == "offset" && ty == "S6"
        ));
        assert!(matches!(&pattern.parts[4], PatternPart::Literal(value) if value == "]"));
    }

    #[test]
    fn parses_spaces_around_ref_bindings() {
        let pattern = parse_pattern("  { variable_to_bind : type }  ").unwrap();

        assert_eq!(pattern.parts.len(), 1);
        assert!(matches!(
            &pattern.parts[0],
            PatternPart::Ref(PatternBinding { name, ty })
                if name == "variable_to_bind" && ty == "type"
        ));
    }

    #[test]
    fn preserves_escaped_braces_in_literals() {
        let pattern = parse_pattern("left {{ and {name:type} and }} right").unwrap();

        assert_eq!(pattern.parts.len(), 3);
        assert!(matches!(&pattern.parts[0], PatternPart::Literal(value) if value == "left { and "));
        assert!(matches!(
            &pattern.parts[1],
            PatternPart::Ref(PatternBinding { name, ty }) if name == "name" && ty == "type"
        ));
        assert!(
            matches!(&pattern.parts[2], PatternPart::Literal(value) if value == " and } right")
        );
    }

    #[test]
    fn parses_escaped_braces_at_pattern_boundaries() {
        let pattern = parse_pattern("{{hello}}").unwrap();

        assert_eq!(pattern.parts.len(), 1);
        assert!(matches!(
            &pattern.parts[0],
            PatternPart::Literal(value) if value == "{hello}"
        ));
    }
}
