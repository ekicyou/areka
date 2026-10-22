//! バルーン位置初期化（`\![execute,resetballoonpos]`・メニュー・近道の 3 つの入口が呼ぶ 1 つの
//! 関数・areka-P0-window-link-directives 要件 7.1〜7.4・design「reset_balloon_pos」）。中身は task 2.3 で入る。

#[cfg(test)]
#[path = "reset_balloon_pos_tests.rs"]
mod tests;
