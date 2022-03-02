//! Convert strings to pig latin. The first consonant
//! of each word is moved to the end of the word and “ay”
//! is added, so “first” becomes “irst-fay.” Words that
//! start with a vowel have “hay” added to the end instead
//! (“apple” becomes “apple-hay”). Keep in mind the details
//! about UTF-8 encoding!

static CONSONANTS: &'static [char] = &[
    'B', 'C', 'D', 'F', 'G', 'H', 'J', 'K', 'L', 'M', 'N', 'P', 'Q', 'R', 'S', 'T', 'V', 'W', 'X',
    'Z',
];

pub fn encode(text: &str) -> String {
    let mut total_bytes = text.bytes().count();
    total_bytes += 5 * text.split_whitespace().count();
    let mut res = String::with_capacity(total_bytes);

    for word in text.split_whitespace() {
        let cap = word.bytes().count();
        let chars: Vec<char> = word.chars().collect();
        
        let s = if CONSONANTS.contains(&chars[0].to_uppercase().next().unwrap()) {
            let mut s = String::with_capacity(cap + 3);
            chars[1..].iter().for_each(|&c| s.push(c));
            s.push('-');
            s.push(chars[0]);
            s.push_str("ay");
            s
        } else {
            let mut s = String::with_capacity(cap + 3);
            s.push_str(word);
            s.push_str("-hay");
            s
        };

        res.push_str(&s);
        res.push(' ');
    }

    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_work() {
        let text: &str = "Hello, world! orange";
        let res = encode(text);
        assert_eq!(res, "ello,-Hay orld!-way orange-hay ");
    }
}
