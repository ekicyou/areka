//! actor の子: 手動の送りの公開の口（balloon-scroll-arrows・design.md「ActorScroll」）。
//!
//! 場所（[`PlaceKey`]）ごとの手動の位置（[`PlaceScroll`]）の表を、矢印の押下・ホイール・消去・
//! 箱の登録を外す・隠した・面の切り替えから動かす。描き直しはしない（次のフレームの
//! `present_actor` が [`crate::scroll_marker::window_for`] の見える範囲で今の送りの描き直しを出す）。
//! 手動の始まり・終わりは場所と理由を `debug!`（要件 9.2）、1 段ごとの送りは `trace!`（要件 9.3）。

use areka_sakura::contract::ActorKey;
use tracing::{debug, trace};

use crate::place::PlaceKey;
use crate::scroll_marker::{ScrollChange, ScrollDir, StopReason, WheelLines};

use super::TextLayerRuntime;

impl TextLayerRuntime {
    /// 場所の手動の位置を `dir` の向きへ `lines` 行送る（矢印の押下・要件 5.1〜5.4）。
    /// まだ一度も提示していない場所は送れない（`Unchanged`）。
    pub fn scroll_manual_at(
        &mut self,
        place: &PlaceKey,
        dir: ScrollDir,
        lines: usize,
    ) -> ScrollChange {
        let Some(scroll) = self.scrolls.get_mut(place) else {
            return ScrollChange::Unchanged;
        };
        let change = scroll.step(dir, lines);
        record(place, change, "矢印");
        change
    }

    /// 場所をホイールの回転 `delta`（正＝向こう側＝古い側）で送る（要件 6.1〜6.6）。
    /// 「1 画面ずつ」は前のフレームの見えていた行の数を送る。
    pub fn wheel_at(
        &mut self,
        place: &PlaceKey,
        delta: i32,
        per_notch: WheelLines,
    ) -> ScrollChange {
        let Some(scroll) = self.scrolls.get_mut(place) else {
            return ScrollChange::Unchanged;
        };
        let page = scroll.page_lines();
        let change = scroll.wheel(delta, per_notch, page);
        record(place, change, "ホイール");
        change
    }

    /// 場所に隠れた行（押し出された古い行か、手動の位置の先の新しい行）があるか（要件 7.5・7.6）。
    pub fn has_hidden_lines_at(&self, place: &PlaceKey) -> bool {
        self.scrolls
            .get(place)
            .is_some_and(|scroll| scroll.has_hidden_lines())
    }

    /// 場所の手動の位置を捨てる（消去・隠した・箱を外した・要件 8.2・8.3）。手動でなければ何もしない。
    pub fn forget_manual_at(&mut self, place: &PlaceKey, reason: StopReason) {
        if let Some(scroll) = self.scrolls.get_mut(place) {
            record(place, scroll.forget(reason), "捨てる口");
        }
    }

    /// アクターの普通のバルーンとその箱の全部の手動の位置を捨てる（面の切り替え・要件 8.6）。
    pub fn forget_manual_of(&mut self, actor: &ActorKey, reason: StopReason) {
        for (place, scroll) in self.scrolls.iter_mut() {
            if place.actor == *actor {
                record(place, scroll.forget(reason), "捨てる口");
            }
        }
    }

    /// 消去（`ClearAll`）: 全部の場所の手動の位置を捨てる（要件 8.2）。
    pub(super) fn forget_all_manual(&mut self, reason: StopReason) {
        for (place, scroll) in self.scrolls.iter_mut() {
            record(place, scroll.forget(reason), "捨てる口");
        }
    }

    /// 箱の登録を外す: 手動を捨てて場所の状態ごと片付ける（`BoxRemoved`）。
    pub(super) fn drop_box_scroll(&mut self, place: &PlaceKey) {
        self.forget_manual_at(place, StopReason::BoxRemoved);
        self.scrolls.remove(place);
    }
}

/// 手動の位置の変わり目を記録する。始まり・終わりは `debug!`、1 段ごとの送りは `trace!`。
pub(super) fn record(place: &PlaceKey, change: ScrollChange, via: &str) {
    match change {
        ScrollChange::Started => {
            debug!(actor = %place.actor, place = ?place.place, via, "手動の送りを始めた")
        }
        ScrollChange::Stopped(reason) => debug!(
            actor = %place.actor,
            place = ?place.place,
            ?reason,
            via,
            "手動の送りを終えて自動へ戻した"
        ),
        ScrollChange::Moved => {
            trace!(actor = %place.actor, place = ?place.place, via, "手動の位置を送った")
        }
        ScrollChange::Unchanged => {}
    }
}
