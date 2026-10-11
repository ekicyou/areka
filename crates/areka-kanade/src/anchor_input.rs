//! アンカー（`\_a`）の選択の知らせの型（UI 配線層 → kanade）。

/// アンカーの選択の知らせ（[`crate::msg::ChoiceInput`] と同型の値オブジェクト）。
///
/// UI 配線層が、押されたアンカーの範囲の中身を詰めて送る。kanade は `id`・`text`・`references` を
/// 解釈せず、そのまま SHIORI のイベントへ写す（引数は記述順のまま）。選択肢と違い、選択待ちの
/// 帳簿との照合はしない（届いた知らせはそのまま送る・areka-P0-anchor-tag-canon 要件 4.12）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnchorInput {
    /// アンカーの ID（不透明）。`On` で始まれば、その名前のイベントを送る。
    pub id: String,
    /// アンカーの範囲に表示された文字（`OnAnchorSelectEx` の Reference0 へ写す）。
    pub text: String,
    /// 押されたバルーンのスコープ番号。記録にだけ使い、Reference には載せない。
    pub scope: u32,
    /// ID に続く引数（不透明・記述順のまま）。
    pub references: Vec<String>,
}
