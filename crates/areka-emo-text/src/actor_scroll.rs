//! actor の子: 手動の送りの公開の口（balloon-scroll-arrows・design.md「ActorScroll」）。
//!
//! 場所（[`PlaceKey`]）ごとの手動の位置（[`PlaceScroll`]）の表を、矢印の押下・ホイール・消去・
//! 箱の登録を外す・隠した・面の切り替えから動かす。描き直しはしない（次のフレームの
//! `present_actor` が [`crate::scroll_marker::window_for`] の見える範囲で今の送りの描き直しを出す）。
//! 手動の始まり・終わりは場所と理由を `debug!`（要件 9.2）、1 段ごとの送りは `trace!`（要件 9.3）。

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use areka_emo_compose::BoxName;
use areka_emo_present::balloon_marker::{MarkerImage, MarkerPair};
use areka_sakura::contract::ActorKey;
use tracing::{debug, trace};

use crate::place::{PlaceKey, TextPlace};
use crate::scroll_marker::{
    MarkerKind, MarkerShown, ResolvedMarkerPlacement, ScrollChange, ScrollDir, StopReason,
    WheelLines,
};
use crate::surface::TextSurface;

use super::{PlacedMarker, TextLayerRuntime};

impl TextLayerRuntime {
    /// スコープの普通のバルーンの矢印の絵を入れ替える（面の切り替えが入れる・要件 3.4・7.3）。
    /// 次の提示で出す矢印が前と違えば描き直す。
    pub fn set_balloon_markers(&mut self, actor: ActorKey, markers: MarkerPair) {
        debug!(
            actor = %actor,
            up = markers.up.is_some(),
            down = markers.down.is_some(),
            "普通のバルーンの矢印の絵を受け取った"
        );
        self.balloon_markers.insert(actor, markers);
    }

    /// 箱の名前ごとの矢印の絵を入れ替える（シェルの箱の束と一緒に入れる・要件 7.3）。
    /// 足りない番号は、提示のときにそのスコープの普通のバルーンの絵で埋める。
    pub fn set_box_markers(&mut self, markers: BTreeMap<BoxName, MarkerPair>) {
        debug!(boxes = markers.len(), "箱の矢印の絵を受け取った");
        self.box_markers = markers;
    }

    /// 点（窓のクライアント物理 px・選択肢の行と同じ空間）が、最後に提示した矢印の矩形の中で、
    /// その画素が矢印の絵の α ≥ 0.5 なら矢印の種類（要件 5.7）。押下の振り分けは送り手でなく
    /// この点の答えで決める。
    pub fn marker_at_point(&self, place: &PlaceKey, client_px: (f32, f32)) -> Option<MarkerKind> {
        self.surfaces.get(place)?.markers.as_ref()?.hit(client_px)
    }

    /// 普通のバルーンの印の面（`dump_balloon` の写し）。矢印をまだ出していなければ `None`。
    pub fn marker_surface(&self, actor: &ActorKey) -> Option<&TextSurface> {
        let render = self.surfaces.get(&PlaceKey::balloon(actor))?;
        Some(render.markers.as_ref()?.surface())
    }

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

/// 出す矢印（[`MarkerShown`]）を絵と置き場に引き当てる。普通のバルーンはスコープの絵、箱は箱の名前の
/// 絵で、箱の足りない番号はスコープの普通のバルーンの絵で埋める（要件 7.3）。絵の無い矢印は出さない。
pub(super) fn placed_markers(
    balloon: &HashMap<ActorKey, MarkerPair>,
    boxes: &BTreeMap<BoxName, MarkerPair>,
    place: &PlaceKey,
    shown: MarkerShown,
    at: ResolvedMarkerPlacement,
) -> Vec<PlacedMarker> {
    let scope = balloon.get(&place.actor);
    let own = match &place.place {
        TextPlace::Balloon => None,
        TextPlace::Box(name) => boxes.get(name),
    };
    let pick = |of: fn(&MarkerPair) -> &Option<Arc<MarkerImage>>| {
        own.and_then(|p| of(p).clone())
            .or_else(|| scope.and_then(|p| of(p).clone()))
    };
    let mut out = Vec::new();
    if let (true, Some(image)) = (shown.up, pick(|p| &p.up)) {
        out.push(PlacedMarker {
            kind: MarkerKind::Up,
            at: at.up,
            image,
        });
    }
    if let (true, Some(image)) = (shown.down, pick(|p| &p.down)) {
        out.push(PlacedMarker {
            kind: MarkerKind::Down,
            at: at.down,
            image,
        });
    }
    out
}
