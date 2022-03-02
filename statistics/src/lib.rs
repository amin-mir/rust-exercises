/// Given a list of integers, use a vector and return
/// the median (when sorted, the value in the middle position)
/// and mode (the value that occurs most often;
/// a hash map will be helpful here) of the list.
use std::collections::HashMap;

enum MiddleIndex {
    Even(usize, usize),
    Odd(usize),
}

fn mid_idx(len: usize) -> MiddleIndex {
    use MiddleIndex::*;

    if len % 2 == 0 {
        let mid = len / 2;
        Even(mid - 1, mid)
    } else {
        let mid = (len + 1) / 2;
        Odd(mid - 1)
    }
}

pub fn calc_median(numbers: &mut [u8]) -> f64 {
    use MiddleIndex::*;

    numbers.sort();

    let idx = mid_idx(numbers.len());

    match idx {
        Even(i, j) => (numbers[i] as f64 + numbers[j] as f64) / 2.0,
        Odd(i) => numbers[i] as f64,
    }
}

pub fn calc_mode(numbers: &mut [u8]) -> u8 {
    let mut counts: HashMap<u8, usize> = HashMap::new();

    numbers.sort();

    for &mut n in numbers {
        *counts.entry(n).or_insert(0) += 1;
    }

    *counts
        .iter()
        .max_by(|a, b| a.1.cmp(&b.1))
        .map(|(k, _v)| k)
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calc_median_even_number_of_input_integers() {
        let mut input = vec![1, 9, 8, 1, 5, 6];
        let median = calc_median(&mut input);
        assert_eq!(median, 5.5);
    }

    #[test]
    fn calc_median_odd_number_of_input_integers() {
        let mut input = vec![1, 9, 8, 1, 5];
        let median = calc_median(&mut input);
        assert_eq!(median, 5f64);
    }

    #[test]
    fn calc_mode_should_return_most_frequent() {
        let mut input = vec![1, 9, 2, 2, 8, 1, 5, 2];
        let mode = calc_mode(&mut input);
        assert_eq!(mode, 2);
    }
}
