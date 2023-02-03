use pin_utils::pin_mut;
use std::io;
use std::time::Instant;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

mod slow_reader;
use slow_reader::SlowReader;

#[tokio::main]
async fn main() -> io::Result<()> {
    let now = Instant::now();

    let f = File::open("/dev/urandom").await?;
    let sr = SlowReader::new(f);
    pin_mut!(sr);

    let mut buf = [0; 256 * 1024]; // 256KiB
    let n = sr.read_exact(&mut buf).await?;

    println!("read byte count: {}, in {:?}", n, now.elapsed());
    Ok(())
}
