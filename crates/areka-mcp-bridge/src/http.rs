//! 自前の HTTP/1.1 の送り手（接続ごとに相手を確かめてから送る）。

#[cfg(test)]
#[path = "http_tests.rs"]
mod tests;
