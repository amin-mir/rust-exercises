use super::Token;

pub struct IntoIter {
    cur_idx: usize,
    next: Option<Result<Token<String>, String>>,
    tmpl: String,
}

impl IntoIter {
    pub fn new(tmpl: String) -> IntoIter {
        IntoIter {
            cur_idx: 0,
            next: None,
            tmpl,
        }
    }

    fn set_next_placeholder(&mut self, at: usize) -> Result<(), String> {
        let tmpl = &self.tmpl[at..];

        let delim_end = match tmpl.find("}}") {
            None => {
                // There is a problem with template, therefore should stop iterating.
                self.stop_iter();
                return Err("missing closing delimiter: }}".to_owned());
            }
            Some(idx) => idx,
        };

        self.next = Some(Ok(Token::Placeholder(tmpl[2..delim_end].trim().to_owned())));
        // Setting current to index after the second closing '}'.
        self.cur_idx = at + delim_end + 2;
        Ok(())
    }

    fn stop_iter(&mut self) {
        // Setting current index to the end of template, so that
        // there is nothing left to iterate through.
        self.cur_idx = self.tmpl.len();
    }
}

impl Iterator for IntoIter {
    type Item = Result<Token<String>, String>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next.is_some() {
            return self.next.take();
        }

        if self.cur_idx >= self.tmpl.len() {
            return None;
        }

        match self.tmpl[self.cur_idx..].find("{{") {
            None => {
                let next = Ok(Token::Text(self.tmpl[self.cur_idx..].to_owned()));

                // No more to iterate through after this. Calling stop_iter
                // has a side-effect of setting cur_idx to a new value, thus
                // it's important to first extract the next and then
                // stop the iterator.
                self.stop_iter();

                Some(next)
            }
            Some(mut idx) => {
                // idx is relative to cur_idx because we used find
                // on tmpl[cur_idx..] earlier.
                idx = idx + self.cur_idx;
                let cur = Token::Text(self.tmpl[self.cur_idx..idx].to_owned());

                if let Err(e) = self.set_next_placeholder(idx) {
                    return Some(Err(e));
                }

                Some(Ok(cur))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_iterate_correctly() {
        let tmpl = String::from("Hello {{ name }} {{surname}}, Welcome!");

        let tokens = IntoIter::new(tmpl);
        let actual: Vec<Result<Token<String>, String>> = tokens.collect();

        let expected = vec![
            Ok(Token::Text("Hello ".to_owned())),
            Ok(Token::Placeholder("name".to_owned())),
            Ok(Token::Text(" ".to_owned())),
            Ok(Token::Placeholder("surname".to_owned())),
            Ok(Token::Text(", Welcome!".to_owned())),
        ];

        assert_eq!(expected, actual);
    }

    #[test]
    fn error_when_no_closing_delim() {
        let tmpl = String::from("Hello {{ name }} {{ surnamne  Welcome!");

        let mut tokens = IntoIter::new(tmpl);

        assert_eq!(tokens.next(), Some(Ok(Token::Text("Hello ".to_owned()))));
        assert_eq!(
            tokens.next(),
            Some(Ok(Token::Placeholder("name".to_owned())))
        );
        assert_eq!(
            tokens.next(),
            Some(Err("missing closing delimiter: }}".to_owned()))
        );
        assert_eq!(tokens.next(), None);
    }
}
