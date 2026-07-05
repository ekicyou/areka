//! 合成メソッド写像表: `ComposeMethod` / `BlendMode` enum と式ステータス・dispatch シーム。
//!
//! ukadoc 由来の描画メソッド全量を `#[non_exhaustive]` enum として列挙し、各メソッドの
//! 実装状態（実装済み＝`overlay`・型シームのみ＝それ以外）を保持する。emo2 実測で使用される
//! `overlay`（および同義の `add`/`bind`）のみ実挙動を持ち、`overlay-fast`・`interpolate`・
//! `replace`・`asis`・`reduce`・`blend-*` 群などは未実装シームとして口だけを保つ。未実装メソッド
//! 参照時はパニックせず `warn` 以上のログで観測可能に扱う。
