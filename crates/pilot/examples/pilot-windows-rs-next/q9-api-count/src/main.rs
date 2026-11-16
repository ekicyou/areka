//! 問い 9: `rust-analyzer scip` の索引を読み、本線が使う windows-rs の API をクレートごと・種類ごとに数える。
//! 使い方: `q9-api-count <index.scip>`（表は標準出力へ）。

mod count;

use protobuf::Message;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: q9-api-count <index.scip>")?;
    let bytes = std::fs::read(&path).map_err(|e| format!("read {path}: {e}"))?;
    let index = scip::types::Index::parse_from_bytes(&bytes)?;
    print!("{}", count::render(&count::tally(&index)));
    Ok(())
}
