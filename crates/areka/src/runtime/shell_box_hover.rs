//! 箱の上の滞在（`ShellBoxHover`）と毎フレームの整え（`settle_box_hover`）。
//!
//! 可視性の相（`balloon_visibility_phase`）が読み、入力の側（`input_events::shell_box`）が書く。
//! 入力の側は `pub(crate) use` で出し直す（runtime-module-untangle 3.3・中身は `input_events/shell_box.rs` から移したまま）。

use std::collections::HashMap;

use areka_emo_compose::BoxName;
use areka_emo_text::actor::ShownBox;

/// 毎フレームの整え: 滞在している箱が `shown_boxes` に無ければ `None` へ戻す（要件 6.11）。
pub(crate) fn settle_box_hover(prev: Option<&BoxName>, shown: &[ShownBox]) -> Option<BoxName> {
    prev.filter(|p| shown.iter().any(|b| &b.name == *p))
        .cloned()
}

/// スコープごとの「今ポインタが居る、文字の出ている箱の名前」（NonSend・UI スレッド所有）。
///
/// 書き込むのは `input_events::shell_box::next_box_hover`（箱へ入る・移る・出る）・シェルの窓からの離脱（`None`）・
/// 毎フレームの [`settle_box_hover`] の結果だけ。時間切れの観測の滞在は `get(scope).is_some()`。
///
/// 滞在している箱へ最後に注入した選択肢の強調（ordinal）も並べて持つ（同値の注入を避けるため・
/// 普通のバルーンの `BalloonWiring::hover` と同じ役目）。強調は行の上の移動でしか載らず、その移動は
/// 箱を滞在に記録するので、強調の載った箱はいつも滞在している箱である。滞在の箱が替わる・消える
/// と記録も落ちる。
#[derive(Debug, Default)]
pub(crate) struct ShellBoxHover(HashMap<usize, (BoxName, Option<usize>)>);

impl ShellBoxHover {
    /// スコープの滞在している箱（居なければ `None`）。
    pub(crate) fn get(&self, scope: usize) -> Option<&BoxName> {
        self.0.get(&scope).map(|(name, _)| name)
    }

    /// スコープの滞在を書き換える（`None` で消す）。同じ箱のままなら強調の記録を保つ。
    pub(crate) fn set(&mut self, scope: usize, name: Option<BoxName>) {
        match name {
            Some(name) => {
                let injected = match self.0.get(&scope) {
                    Some((prev, injected)) if *prev == name => *injected,
                    _ => None,
                };
                self.0.insert(scope, (name, injected));
            }
            None => {
                self.0.remove(&scope);
            }
        }
    }

    /// 滞在している箱へ最後に注入した強調（無ければ `None`）。
    pub(crate) fn injected(&self, scope: usize) -> Option<usize> {
        self.0.get(&scope).and_then(|(_, injected)| *injected)
    }

    /// 滞在している箱へ注入した強調を記録する（滞在が無ければ何もしない）。
    pub(crate) fn set_injected(&mut self, scope: usize, ordinal: Option<usize>) {
        if let Some((_, injected)) = self.0.get_mut(&scope) {
            *injected = ordinal;
        }
    }
}
