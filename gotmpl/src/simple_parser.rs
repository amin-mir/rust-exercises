use std::collections::HashMap;

pub fn parse(template: String, data: HashMap<String, String>) -> String {
    let parser = Parser::new(template, data);
    parser.parse()
}

struct Parser {
    data: HashMap<String, String>,
    tmpl: String,
    result: String,
}

// Can the parser be extracted as a general algorithm implementation.
impl Parser {
    fn new(tmpl: String, data: HashMap<String, String>) -> Self {
        // Capacity here is simply an estimation. We predict that the result
        // string is equal or greater in length than the template itself.
        let result_cap = tmpl.len();
        Parser {
            data,
            tmpl,
            result: String::with_capacity(result_cap),
        }
    }

    // TODO: extract to tokenize function for testability.
    fn parse(mut self) -> String {
        let mut cur_idx = 0;
        loop {
            match self.tmpl[cur_idx..].find("{{") {
                None => {
                    self.result.push_str(&self.tmpl[cur_idx..]);
                    break;
                }
                Some(mut idx) => {
                    // idx is relative to cur_idx because we used find
                    // on tmpl[cur_idx..] earlier.
                    idx = idx + cur_idx;
                    self.result.push_str(&self.tmpl[cur_idx..idx]);

                    // Build a Token::Pattern from the scanned str and set
                    // the cur_idx to index after closing delimiters.
                    cur_idx = self.parse_pattern_at(idx);
                }
            };
        }

        self.result
    }

    // This function assumes that tmpl contains the opening and closing
    // delimiters: "{{" & "}}".
    // It returns the index from which we should continue the parsing.
    fn parse_pattern_at(&mut self, at: usize) -> usize {
        let tmpl = &self.tmpl[at..];

        // Find the closing delimiters and extract whatever's inside.
        let delim_end = tmpl.find("}}").expect("missing closing delimiters: }}");
        let key = tmpl[2..delim_end].trim();

        let val = self.data.get(key).unwrap_or_else(|| {
            panic!("couldn't find data corresponding to key: {}", key)
        });
        self.result.push_str(val);

        // returning index after the second closing '}'.
        at + delim_end + 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_small_template() {
        let tmpl = String::from("Hello, {{ name }}!");
        let data = HashMap::from([("name".to_string(), "Amin".to_string())]);

        let result = parse(tmpl, data);
        assert_eq!("Hello, Amin!", result);
    }

    #[test]
    fn parse_large_template() {
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
}
