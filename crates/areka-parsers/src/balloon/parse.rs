//! balloon 公開 facade（KV マップ → バルーンモデル写像）。
//!
//! 後続タスク 2.2 で公開関数 `parse(...) -> BalloonModel` を実装する。
//! 2 層マージ（sXXs/kXXs 起点 ＞ descript ＞ 既定）＋整数パース＋符号保持を担い、
//! 上流 `kv::parse_kv` のフラット KV 出力を消費する寛容な写像とする。
