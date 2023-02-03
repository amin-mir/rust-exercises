use tokio::time::{self, Duration};

use tokio;

mod man;
use man::ManualFuture;

#[tokio::main]
async fn main() {
    let res = "Final Result".to_owned();
    let (fut, ready) = ManualFuture::new(res);

    let handle = tokio::spawn(async move {
        println!("a new task was spawned!");
        let res = fut.await;
        println!("result after awaiting the future: {}", res);
    });

    assert!(!handle.is_finished());

    ready();
    time::sleep(Duration::from_millis(50)).await;
    assert!(handle.is_finished());
}
