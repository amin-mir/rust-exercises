mod tokens;
use tokens::{Token, Tokens};

use std::collections::HashMap;

type Result<T> = std::result::Result<T, String>;

pub fn parse(tmpl: String, data: HashMap<String, String>) -> Result<String> {
    // let tokens = Tokens::from(tmpl);
    // let parsed = String::new();
    // tokens
    //     .into_iter()
    //     .map(|tkn| match tkn {
    //         Err(e) => Err(e),
    //         Ok(tkn) => resolve_token(&tkn, &data),
    //     })
    //     .try_fold(parsed, |mut acc, s| match s {
    //         Err(e) => Err(e),
    //         Ok(s) => {
    //             acc.push_str(&s);
    //             Ok(acc)
    //         },
    //     })
    let tokens = Tokens::from(tmpl);
    let mut parsed = String::new();

    for tkn in tokens.into_iter() {
        let tkn = tkn?;
        let resolved = resolve_token(&tkn, &data)?;
        parsed.push_str(&resolved);
    }
    Ok(parsed)
}

pub fn parse_ref(tmpl: String, data: HashMap<String, String>) -> Result<String> {
    let tokens = Tokens::from(tmpl);
    let mut parsed = String::new();

    for tkn in tokens.iter() {
        let tkn = tkn?;
        let resolved = resolve_token(&tkn, &data)?;
        parsed.push_str(&resolved);
    }
    Ok(parsed)
}

fn resolve_token<'a, T>(tkn: &'a Token<T>, data: &'a HashMap<String, String>) -> Result<&'a str>
where
    T: AsRef<str> + 'a,
    // T: Into<&'a str>,
{
    match tkn {
        Token::Text(k) => Ok(k.as_ref()),
        Token::Placeholder(k) => {
            let k = k.as_ref();
            data.get(k)
                .map(|v| v.as_str())
                .ok_or(format!("couldn't find data corresponding to key: {}", k))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_token_string_text() {
        let tkn = Token::Text("name".to_owned());
        let data = HashMap::new();

        let resolved = resolve_token(&tkn, &data);
        assert_eq!("name".to_owned(), resolved.unwrap());
    }

    #[test]
    fn resolve_token_string_placeholder() {
        let tkn = Token::Placeholder("name".to_owned());
        let data = HashMap::from([("name".to_owned(), "Amin".to_owned())]);

        let resolved = resolve_token(&tkn, &data);
        assert_eq!("Amin".to_owned(), resolved.unwrap());
    }

    #[test]
    fn resolve_token_str_text() {
        let tkn = Token::Text("name");
        let data = HashMap::new();

        let resolved = resolve_token(&tkn, &data);
        assert_eq!("name".to_owned(), resolved.unwrap());
    }

    #[test]
    fn resolve_token_str_placeholder() {
        let tkn = Token::Placeholder("name");
        let data = HashMap::from([("name".to_owned(), "Amin".to_owned())]);

        let resolved = resolve_token(&tkn, &data);
        assert_eq!("Amin".to_owned(), resolved.unwrap());
    }

    #[test]
    fn parse_small_template() {
        let tmpl = String::from("Hello, {{ name }}!");
        let data = HashMap::from([("name".to_string(), "Amin".to_string())]);

        let result = parse(tmpl, data).unwrap();
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

        let result = parse(tmpl, data).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn parse_ref_small_template() {
        let tmpl = String::from("Hello, {{ name }}!");
        let data = HashMap::from([("name".to_string(), "Amin".to_string())]);

        let result = parse_ref(tmpl, data).unwrap();
        assert_eq!("Hello, Amin!", result);
    }

    #[test]
    fn parse_ref_large_template() {
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

        let result = parse_ref(tmpl, data).unwrap();
        assert_eq!(expected, result);
    }
}
