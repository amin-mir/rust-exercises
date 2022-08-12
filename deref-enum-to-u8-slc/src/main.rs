use core::ops::Deref;
use std::sync::Arc;

fn main() {
    let b1 = Block::Owned(vec![1, 3, 4]);
    let a1 = &*b1;
    println!("{:?}", a1);

    let b2 = Block::Cached(Arc::new(vec![5, 6, 7]));
    let a2 = &b2[..];
    println!("{:?}", a2);

    // We can also call len due to automatic deref.
    println!("{}", b1.len());
}

enum Block {
    Owned(Vec<u8>),
    Cached(Arc<Vec<u8>>),
}

impl Deref for Block {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        match self {
            Self::Owned(v) => {
                println!("Yea it is a vector");
                v.as_ref()
            }
            Self::Cached(a) => {
                println!("Wow it was an Arc this time");
                a.as_ref()
            }
        }
    }
}
