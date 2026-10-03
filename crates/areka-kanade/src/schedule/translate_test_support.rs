//! 既存テスト用の補助（テスト専用）: 翻訳の行動が出たら、結果を台詞そのままで入れ直して続きを返す。
//!
//! 出口の規則が SHIORI の台詞の再生開始を `[Action::Translate]` に替えた後も、既存の `step` の
//! テストが「`OnTranslate` が 204 を返した」ときの続き（預けた一括）を確かめられるようにする。

use super::{Action, Input, State, step};
use crate::msg::KanadeConfig;

/// `step` の返り値の一括が翻訳の行動 1 つだけなら、結果を台詞そのまま（`Ok(script)`）で入れ直して
/// 続きを返す。それ以外の一括はそのまま返す。
pub(crate) fn pass_translate(
    (state, actions): (State, Vec<Action>),
    config: &KanadeConfig,
) -> (State, Vec<Action>) {
    match actions.as_slice() {
        [Action::Translate(request)] => {
            let script = request.script.clone();
            step(state, Input::TranslateDone(Ok(script)), config)
        }
        _ => (state, actions),
    }
}
