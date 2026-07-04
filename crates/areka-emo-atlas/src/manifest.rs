//! マニフェスト導出（列挙・間接参照解決・重複排除）。
//!
//! 設計決定 **D6**（要件 **R1.1–1.6, 5.6**）。
//!
//! shell モデル（`areka-parsers::shell::Shell`）と surface 表現された balloon から、
//! 全 surface が参照する element 画像パス集合を列挙する。`Pattern.surface_id` の
//! 間接 bind 参照を surface id 索引で辿って参照先 surface の element を展開し、
//! 負値センチネル・不在 id・`Range`/alias（画像を持たない）は除外する。
//! 訪問済み集合による循環検出を含む。列挙結果は正規化パスで重複排除する。
//!
//! （本タスクは雛形。実装は後続タスクで追加する。）
