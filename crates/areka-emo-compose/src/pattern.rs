//! `PatternState` / `PatternFrame`: pattern 進行状態の合成入力第一級表現（公開型正本）。
//!
//! seriko（⑤）が pattern タイムライン評価から生産し、emo-compose（⑥合成）／emo-present（⑥提示）が
//! 消費する。animation id → 現在コマ 1 枚（要件 4.2）の写像を `BTreeMap` で保持し、正準（昇順）順序で
//! `Eq`／ハッシュを安定させる。この順序安定性は後段で `PatternState` が `ComposeKey`（emo-present の
//! 合成メモ化キー）の一部となるため、キャッシュ等価判定の決定論に直結する（要件 5.2/5.4）。
//!
//! `Default` は空＝「pattern 寄与なし」。空の `PatternState` を渡した合成・キャッシュは従来（拡張前）と
//! 観測等価（要件 5.4）。`Send + 'static` 所有でスレッド越え受け渡しに耐える。

use std::collections::BTreeMap;

use crate::method::ComposeMethod;

/// pattern 進行状態: animation id → 現在コマ 1 枚（要件 4.2）。
///
/// 内部表現は `BTreeMap<u32, PatternFrame>`（opaque）。`BTreeMap` の正準（キー昇順）順序により、
/// 挿入順に依存せず [`Eq`] が安定する。これは `PatternState` が emo-present の `ComposeKey` に
/// 組み込まれてキャッシュ等価判定に用いられるため決定論上必須である（要件 5.2/5.4）。
///
/// [`Default`] は空マップ＝「pattern 寄与なし」で、空を渡した合成・キャッシュは拡張前と観測等価
/// （要件 5.4）。`Send + 'static` 所有。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PatternState {
    /// animation id → 現在コマ。各アニメは同時に最大 1 コマ（4.2「現在コマ 1 枚」）。
    frames: BTreeMap<u32, PatternFrame>,
}

/// pattern の現在コマ 1 枚。表示中 surface のアニメに属する transient な合成寄与。
///
/// `surface_id` は常に正値（負値センチネル `-1` 等は評価器が停止／非駆動として解決済みで、
/// コマとしてはここへ載らない）。`method` は完全語彙（要件 8.4）を保持し、合成の実駆動は
/// `Overlay` のみ（下流 plan の method ゲートが `is_implemented()` で選別する）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternFrame {
    /// コマの surface_id（正値のみ・センチネルは評価器が解決済み）。
    pub surface_id: u32,
    /// 描画メソッド（完全語彙・要件 8.4）。合成は `Overlay` のみ駆動、それ以外は非駆動シーム。
    pub method: ComposeMethod,
    /// コマの X 累積オフセット。
    pub x: i64,
    /// コマの Y 累積オフセット。
    pub y: i64,
}

impl PatternState {
    /// pattern 寄与が無い（コマを 1 枚も持たない）か。[`Default`] は真。
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// 指定 animation id の現在コマを設定する（同 id の既存コマは置換＝現在コマ 1 枚・要件 4.2）。
    pub fn set(&mut self, animation_id: u32, frame: PatternFrame) {
        self.frames.insert(animation_id, frame);
    }

    /// 指定 animation id の現在コマを除去する（停止・ベース復帰時のクリア）。
    pub fn remove(&mut self, animation_id: u32) {
        self.frames.remove(&animation_id);
    }

    /// 指定 animation id の現在コマを引く（無ければ `None`）。
    pub fn get(&self, animation_id: u32) -> Option<&PatternFrame> {
        self.frames.get(&animation_id)
    }

    /// 現在コマを animation id 昇順（正準順序）で走査する。
    pub fn iter(&self) -> impl Iterator<Item = (u32, &PatternFrame)> {
        self.frames.iter().map(|(&id, frame)| (id, frame))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// テスト用の代表コマ（`Overlay`・任意オフセット）。
    fn frame(surface_id: u32) -> PatternFrame {
        PatternFrame {
            surface_id,
            method: ComposeMethod::Overlay,
            x: 0,
            y: 0,
        }
    }

    /// 既定値は空＝「pattern 寄与なし」（完了状態の明示契約）。
    #[test]
    fn default_is_empty() {
        assert!(PatternState::default().is_empty());
    }

    /// `set` した現在コマを `get` で往復できる。
    #[test]
    fn set_then_get_round_trips() {
        let mut state = PatternState::default();
        let f = PatternFrame {
            surface_id: 1410,
            method: ComposeMethod::Overlay,
            x: 12,
            y: -34,
        };
        state.set(7, f.clone());
        assert!(!state.is_empty());
        assert_eq!(state.get(7), Some(&f));
        assert_eq!(state.get(8), None);
    }

    /// 同一 id への二度目の `set` は置換する（現在コマ 1 枚・要件 4.2）。
    #[test]
    fn set_twice_same_id_replaces() {
        let mut state = PatternState::default();
        state.set(3, frame(100));
        state.set(3, frame(200));
        assert_eq!(state.get(3).map(|f| f.surface_id), Some(200));
        // 単一エントリのまま（コマは 1 枚）。
        assert_eq!(state.iter().count(), 1);
    }

    /// `remove` は当該コマをクリアする（停止・ベース復帰）。
    #[test]
    fn remove_clears_frame() {
        let mut state = PatternState::default();
        state.set(5, frame(42));
        state.remove(5);
        assert_eq!(state.get(5), None);
        assert!(state.is_empty());
    }

    /// `iter` は animation id 昇順（正準順序）で走査する。
    #[test]
    fn iter_yields_ascending_id_order() {
        let mut state = PatternState::default();
        state.set(9, frame(1));
        state.set(2, frame(2));
        state.set(5, frame(3));
        let ids: Vec<u32> = state.iter().map(|(id, _)| id).collect();
        assert_eq!(ids, vec![2, 5, 9]);
    }

    /// 挿入順が異なっても同一コマ集合なら `Eq`（正準順序の安定性）。
    ///
    /// `PatternState` は emo-present の `ComposeKey` に組み込まれキャッシュ等価判定に用いられるため、
    /// 挿入順非依存の `Eq` が決定論上必須である（要件 5.2/5.4）。
    #[test]
    fn eq_is_insertion_order_stable() {
        let mut a = PatternState::default();
        a.set(1, frame(10));
        a.set(2, frame(20));
        a.set(3, frame(30));

        let mut b = PatternState::default();
        b.set(3, frame(30));
        b.set(1, frame(10));
        b.set(2, frame(20));

        assert_eq!(a, b);
    }

    /// `PatternState` が `Send + 'static` であることをコンパイル時に固定する（スレッド越え所有）。
    #[test]
    fn pattern_state_is_send_static() {
        fn assert_send_static<T: Send + 'static>() {}
        assert_send_static::<PatternState>();
    }
}
