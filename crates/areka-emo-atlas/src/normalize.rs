//! 透過正規化（`use_self_alpha` 解釈・premultiplied BGRA 統一）。
//!
//! 設計決定 **D5 / D8**（要件 **R3**）。
//!
//! 伺かの透過規則（優先順位 α ＞ `.pna` ＞ キーカラー）を契約として定義する。
//! ukadoc 2×2（`use_self_alpha` × `.pna` 有無）動作表のうち emo2 実装腕＝
//! `use_self_alpha=1` かつ `.pna` 無し（α チャンネル採用）のみを実装し、他はシーム。
//! 契約は「Normalizer 出力は常に premultiplied BGRA」で、premultiplied 統一点は
//! デコード腕出力（WIC PBGRA）で既に成立、シーム腕実装時は本段末尾で premultiply する。
//!
//! （本タスクは雛形。実装は後続タスクで追加する。）
