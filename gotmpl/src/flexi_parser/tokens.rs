mod iter;
use iter::Iter;

mod into_iter;
use into_iter::IntoIter;

use super::Result;

#[derive(Debug, PartialEq)]
pub enum Token<T> {
    Text(T),
    Placeholder(T),
}

pub struct Tokens {
    tmpl: String,
}

impl Tokens {
    pub fn from(tmpl: String) -> Self {
        Tokens { tmpl }
    }

    pub fn iter(&self) -> Iter {
        Iter::new(&self.tmpl)
    }

    pub fn into_iter(&self) -> IntoIter {
        IntoIter::new(self.tmpl.clone())
    }
}

impl IntoIterator for Tokens {
    type Item = Result<Token<String>>;
    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        Tokens::into_iter(&self)
    }
}

impl<'a> IntoIterator for &'a Tokens {
    type Item = Result<Token<&'a str>>;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iter() {
        let tmpl = String::from("Hello {{ name }} {{surname}}, Welcome!");

        let tokens = Tokens::from(tmpl);

        let mut actual = Vec::new();
        for t in &tokens {
            println!("{:?}", t);
            actual.push(t.unwrap());
        }

        let expected = vec![
            Token::Text("Hello "),
            Token::Placeholder("name"),
            Token::Text(" "),
            Token::Placeholder("surname"),
            Token::Text(", Welcome!"),
        ];

        assert_eq!(expected, actual);
    }

    #[test]
    fn into_iter() {
        let tmpl = String::from("Hello {{ name }} {{surname}}, Welcome!");

        let tokens = Tokens::from(tmpl);

        let mut actual = Vec::new();
        for t in tokens {
            println!("{:?}", t);
            actual.push(t.unwrap());
        }

        let expected = vec![
            Token::Text("Hello ".to_owned()),
            Token::Placeholder("name".to_owned()),
            Token::Text(" ".to_owned()),
            Token::Placeholder("surname".to_owned()),
            Token::Text(", Welcome!".to_owned()),
        ];

        assert_eq!(expected, actual);
    }
}
