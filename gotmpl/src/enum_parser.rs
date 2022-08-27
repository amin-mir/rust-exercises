use std::collections::HashMap;

#[derive(Debug)]
enum Token<'a> {
    String(&'a str),
    Pattern(&'a str),
}

trait StringFromTokens {
    fn build(&self, tokens: &[Token], data: &HashMap<String, String>) -> String;
}

struct SimpleStringBuilder;

impl StringFromTokens for SimpleStringBuilder {
    fn build(&self, tokens: &[Token], data: &HashMap<String, String>) -> String {
        let mut result = String::new();
        for token in tokens.iter() {
            match token {
                Token::String(s) => result.push_str(s),
                Token::Pattern(p) => {
                    let s = data.get(*p).unwrap_or_else(|| {
                        panic!("couldn't find data corresponding to key: {}", p)
                    });
                    result.push_str(s);
                }
            }
        }
        result
    }
}

struct CapacityStringBuilder;

impl CapacityStringBuilder {
    fn cap(&self, tokens: &[Token], data: &HashMap<String, String>) -> usize {
        tokens
            .iter()
            .map(|tkn| match tkn {
                Token::String(s) => s.len(),
                Token::Pattern(p) => {
                    let s = data.get(*p).unwrap_or_else(|| {
                        panic!("couldn't find data corresponding to key: {}", p)
                    });
                    s.len()
                }
            })
            .sum()
    }
}

impl StringFromTokens for CapacityStringBuilder {
    fn build(&self, tokens: &[Token], data: &HashMap<String, String>) -> String {
        let cap = self.cap(tokens, data);
        let mut result = String::with_capacity(cap);
        for token in tokens.iter() {
            match token {
                Token::String(s) => result.push_str(s),
                Token::Pattern(p) => {
                    let s = data.get(*p).unwrap_or_else(|| {
                        panic!("couldn't find data corresponding to key: {}", p)
                    });
                    result.push_str(s);
                }
            }
        }
        result
    }
}

pub fn parse(template: String, data: HashMap<String, String>) -> String {
    let mut parser = Parser::new(template, data);
    parser.parse()
}

pub fn parse_cap(template: String, data: HashMap<String, String>) -> String {
    let mut parser = Parser::with_str_builder(template, data, CapacityStringBuilder);
    parser.parse()
}

struct Parser<'a, S: StringFromTokens> {
    data: HashMap<String, String>,
    tmpl: String,
    tokens: Vec<Token<'a>>,
    str_builder: S,
}

impl<'a> Parser<'a, SimpleStringBuilder> {
    fn new(tmpl: String, data: HashMap<String, String>) -> Self {
        Parser {
            data,
            tmpl,
            tokens: vec![],
            str_builder: SimpleStringBuilder,
        }
    }
}

impl<'a, S> Parser<'a, S>
where
    S: StringFromTokens,
{
    fn with_str_builder(tmpl: String, data: HashMap<String, String>, s: S) -> Self {
        Parser {
            data,
            tmpl,
            tokens: vec![],
            str_builder: s,
        }
    }

    // TODO: extract to tokenize function for testability.
    fn parse(&'a mut self) -> String {
        let mut cur_idx = 0;
        loop {
            match self.tmpl[cur_idx..].find("{{") {
                None => {
                    let token = Token::String(&self.tmpl[cur_idx..]);
                    self.tokens.push(token);
                    break;
                }
                Some(mut idx) => {
                    // idx is relative to cur_idx because we used find
                    // on tmpl[cur_idx..] earlier.
                    idx = idx + cur_idx;
                    let mut token = Token::String(&self.tmpl[cur_idx..idx]);
                    self.tokens.push(token);

                    // Build a Token::Pattern from the scanned str and set
                    // the cur_idx to index after closing delimiters.
                    (cur_idx, token) = self.parse_pattern_at(&self.tmpl, idx);
                    self.tokens.push(token);
                }
            };
        }

        self.build()
    }

    // This function assumes that tmpl contains the opening and closing
    // delimiters: "{{" & "}}".
    // It returns the index from which we should continue the parsing.
    fn parse_pattern_at(&self, mut tmpl: &'a str, at: usize) -> (usize, Token<'a>) {
        tmpl = &tmpl[at..];

        // Find the closing delimiters and extract whatever's inside.
        let delim_end = tmpl.find("}}").expect("missing closing delimiters: }}");
        let ptrn = Token::Pattern(tmpl[2..delim_end].trim());

        // returning index of the second closing '}'.
        (at + delim_end + 2, ptrn)
    }

    fn build(&self) -> String {
        self.str_builder.build(&self.tokens, &self.data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_template_simple_builder() {
        let tmpl = String::from("Hello, {{ name }}!");
        let data = HashMap::from([("name".to_string(), "Amin".to_string())]);

        let result = parse(tmpl, data);
        assert_eq!("Hello, Amin!", result);
    }

    #[test]
    fn parse_large_template_simple_builder() {
        let tmpl = std::fs::read_to_string("templates/large.tmpl").unwrap();
        let expected = std::fs::read_to_string("templates/large.parsed").unwrap();
        let data = HashMap::from([
            ("name1".to_string(), "A1".to_string()),
            ("name2".to_string(), "A2".to_string()),
            ("name3".to_string(), "A3".to_string()),
            ("surname1".to_string(), "M1".to_string()),
            ("surname2".to_string(), "M2".to_string()),
            ("surname3".to_string(), "M3".to_string()),
        ]);

        let result = parse(tmpl, data);
        assert_eq!(expected, result);
    }

    #[test]
    fn parse_template_capacity_builder() {
        let tmpl = String::from("Hello, {{ name }}!");
        let data = HashMap::from([("name".to_string(), "Amin".to_string())]);

        let result = parse_cap(tmpl, data);
        assert_eq!("Hello, Amin!", result);
    }
}
