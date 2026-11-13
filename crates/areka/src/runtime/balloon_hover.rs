//! バルーンの上の滞在の集合（`BalloonHover`・UI スレッドの NonSend）。
//!
//! 可視性の相（`balloon_visibility_phase`）が読み、入力の側（`input_events::balloon` の移動・離脱）が書く。
//! 旧 `BalloonWiring::balloon_hover` を分けたもの（runtime-module-untangle 3.3）——`BalloonWiring` ごと
//! 下ろすと選択の送り口まで連れて来るため、集合だけを runtime の型にする。置くのは `BalloonWiring` と
//! 同じ所・同じ時（本番は `input_events::balloon::wire_balloon_choice`）。

use std::collections::HashSet;

/// ポインタがバルーン窓の上に居る scope の集合（areka-P0-balloon-visibility 5.2）。
///
/// 選択肢行の追跡（`BalloonWiring::hover`）とは**別概念の独立した軸**——こちらは「バルーンの上に
/// 居るか」だけを表す（選択肢の有無・行ヒットの有無に依存しない）。
#[derive(Debug, Default)]
pub(crate) struct BalloonHover(HashSet<usize>);

impl BalloonHover {
    /// scope のバルーン窓上にポインタが居るかを照会する（5.2）。未観測の scope は偽。
    ///
    /// バルーン可視性の相が毎フレームの観測収集で読む（`balloon_visibility_phase` の `collect_observations`）。
    pub(crate) fn is_balloon_hovered(&self, scope: usize) -> bool {
        self.0.contains(&scope)
    }

    /// scope のバルーン窓上へポインタが入った（居る）ことを記録する（5.2）。
    ///
    /// 選択肢行の追跡（`BalloonWiring::set_hover`）とは独立——行に当たっていなくても記録する。
    pub(crate) fn set_balloon_hover(&mut self, scope: usize) {
        self.0.insert(scope);
    }

    /// scope のバルーン滞在の記録を落とす（離脱・非表示遷移時の掃除・5.2/5.5）。
    ///
    /// 窓外離脱（`PointerLeave`）のほか、可視性コントローラが非表示遷移で呼ぶ掃除口でもある——
    /// 不可視の間は `PointerLeave` が届かず、放置すると滞在が真のまま固着して恒久抑止になる
    /// （Requirement 5.5 が禁じる側）。未記録 scope への呼出は no-op（冪等）。
    pub(crate) fn clear_balloon_hover(&mut self, scope: usize) {
        self.0.remove(&scope);
    }
}
