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
use crate::nesting::{FilmId, PartKey};

/// pattern 進行状態: animation id → 現在コマ 1 枚（要件 4.2）。
///
/// 欄の意味は 3 つ（要件 1.10, 4.5, 7.4）: **載っていない＝経過 0**／**コマ**（サーフェスを指す
/// [`PatternFrame`]、または動く絵の子の絵の番号）／**消えている**（`always` の途中の終わりのコマの後）。
/// 読みは [`cell`](Self::cell) が [`Cell`] で返す。seriko は経過 0 と同じコマを載せないので、空の
/// `PatternState` は「全部が経過 0」と同じ意味になる。
///
/// 内部表現は昇順の表（opaque）。`BTreeMap` の正準（キー昇順）順序により、
/// 挿入順に依存せず [`Eq`] が安定する。これは `PatternState` が emo-present の `ComposeKey` に
/// 組み込まれてキャッシュ等価判定に用いられるため決定論上必須である（要件 5.2/5.4）。
///
/// [`Default`] は空マップ＝「pattern 寄与なし」で、空を渡した合成・キャッシュは拡張前と観測等価
/// （要件 5.4）。`Send + 'static` 所有。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PatternState {
    /// animation id → 現在コマか「消えている」。各アニメは同時に最大 1 コマ（4.2「現在コマ 1 枚」）。
    frames: BTreeMap<u32, Slot>,
    /// 部品のサーフェス番号 → animation id → 現在コマか「消えている」（要件 5.3, 5.4, 5.10）。
    /// 一番上の欄とは別。
    ///
    /// どれも昇順の表で、空の内側の表は持たない（入れた順・消した後に依らず [`Eq`] を安定させる）。
    parts: BTreeMap<u32, BTreeMap<u32, Slot>>,
    /// 動く絵の子 → 今のコマ（絵の番号）。子は animation を 1 本（番号 0）しか持たないので 1 子 1 欄。
    films: BTreeMap<FilmId, u32>,
}

/// 欄 1 つに載るもの（載っていない＝経過 0 は「表に無い」で表す）。コマと「消えている」は同じ欄で
/// 入れ替わり、両方が残ることは無い。
#[derive(Clone, Debug, PartialEq, Eq)]
enum Slot {
    Frame(PatternFrame),
    Blank,
}

impl Slot {
    fn frame(&self) -> Option<&PatternFrame> {
        match self {
            Slot::Frame(frame) => Some(frame),
            Slot::Blank => None,
        }
    }

    fn cell(&self) -> Cell<'_> {
        match self {
            Slot::Frame(frame) => Cell::Frame(frame),
            Slot::Blank => Cell::Blank,
        }
    }
}

/// 欄 1 つの読み（要件 1.10, 4.5）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell<'a> {
    /// 載っていない（`always` なら経過 0 のコマを定義から描く）。
    Rest,
    /// サーフェスを指すコマ（今までのコマ）。
    Frame(&'a PatternFrame),
    /// 絵を直接指すコマ（動く絵の子）。絵の番号。
    Picture(u32),
    /// 消えている（`always` の途中の終わりのコマの後）。
    Blank,
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
    /// 全部の欄が「載っていない」（＝全部が経過 0・コマも「消えている」も子のコマも持たない）か。
    /// [`Default`] は真。
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty() && self.parts.is_empty() && self.films.is_empty()
    }

    /// 指定 animation id の現在コマを設定する（同 id の既存コマ・「消えている」は置換＝現在コマ 1 枚・
    /// 要件 4.2）。
    pub fn set(&mut self, animation_id: u32, frame: PatternFrame) {
        self.frames.insert(animation_id, Slot::Frame(frame));
    }

    /// 一番上の animation を「消えている」にする（同 id のコマは置換・要件 4.5）。
    pub fn set_blank(&mut self, animation_id: u32) {
        self.frames.insert(animation_id, Slot::Blank);
    }

    /// 指定 animation id の欄を「載っていない」へ戻す（停止・ベース復帰時のクリア。コマも
    /// 「消えている」も消える）。
    pub fn remove(&mut self, animation_id: u32) {
        self.frames.remove(&animation_id);
    }

    /// 指定 animation id の現在コマを引く（コマでなければ `None`。「消えている」もコマではない）。
    pub fn get(&self, animation_id: u32) -> Option<&PatternFrame> {
        self.frames.get(&animation_id)?.frame()
    }

    /// 現在コマを animation id 昇順（正準順序）で走査する（「消えている」は現れない）。
    pub fn iter(&self) -> impl Iterator<Item = (u32, &PatternFrame)> {
        self.frames
            .iter()
            .filter_map(|(&id, slot)| Some((id, slot.frame()?)))
    }

    /// 部品 `surface_id` の animation `animation_id` の今のコマを置く（同じ鍵の既存コマは置換）。
    ///
    /// 一番上の欄（[`set`](Self::set)）とは別の欄なので、同じ番号のサーフェスが一番上と部品の両方で
    /// 出ても混ざらない（要件 5.10）。同じ部品を何か所に置いても欄は 1 つ（要件 5.4）。
    pub fn set_part(&mut self, surface_id: u32, animation_id: u32, frame: PatternFrame) {
        self.parts
            .entry(surface_id)
            .or_default()
            .insert(animation_id, Slot::Frame(frame));
    }

    /// 部品 `part` の animation を「消えている」にする（同じ鍵のコマは置換・要件 4.5）。
    pub fn set_part_blank(&mut self, part: u32, animation_id: u32) {
        self.parts
            .entry(part)
            .or_default()
            .insert(animation_id, Slot::Blank);
    }

    /// 部品の欄（コマと「消えている」）と動く絵の子の欄を全部消す（一番上の欄は残す）。空の内側の表は
    /// 残らない。
    pub fn clear_parts(&mut self) {
        self.parts.clear();
        self.films.clear();
    }

    /// 部品 `surface_id` の animation `animation_id` の今のコマを引く（コマでなければ `None`）。
    pub fn part_get(&self, surface_id: u32, animation_id: u32) -> Option<&PatternFrame> {
        self.parts.get(&surface_id)?.get(&animation_id)?.frame()
    }

    /// 部品 `surface_id` のコマを animation の番号の昇順に走査する（無ければ空・「消えている」は
    /// 現れない）。
    pub fn part(&self, surface_id: u32) -> impl Iterator<Item = (u32, &PatternFrame)> {
        self.parts
            .get(&surface_id)
            .into_iter()
            .flat_map(|slots| slots.iter())
            .filter_map(|(&id, slot)| Some((id, slot.frame()?)))
    }

    /// 動く絵の子 `film` の今のコマ（絵の番号）を置く（同じ子の既存コマは置換）。
    pub fn set_film(&mut self, film: FilmId, picture: u32) {
        self.films.insert(film, picture);
    }

    /// 動く絵の子 `film` の欄を「載っていない」（経過 0）へ戻す。
    pub fn remove_film(&mut self, film: FilmId) {
        self.films.remove(&film);
    }

    /// 欄の読み。`part` が `None` なら一番上の欄。動く絵の子は animation 0 だけを持つので、他の番号は
    /// [`Cell::Rest`]。
    pub fn cell(&self, part: Option<PartKey>, animation_id: u32) -> Cell<'_> {
        let slot = match part {
            None => self.frames.get(&animation_id),
            Some(PartKey::Surface(surface_id)) => self
                .parts
                .get(&surface_id)
                .and_then(|slots| slots.get(&animation_id)),
            Some(PartKey::Film(film)) => {
                return match self.films.get(&film) {
                    Some(&picture) if animation_id == 0 => Cell::Picture(picture),
                    _ => Cell::Rest,
                };
            }
        };
        slot.map_or(Cell::Rest, Slot::cell)
    }

    /// 「載っていない」以外の欄を全部、読むだけで走査する（表示層が回数つきの子の欄を外す・perf の
    /// 鍵へ混ぜるのに使う）。順は一番上 → 部品（番号の昇順）→ 動く絵の子（番号の昇順）、各々
    /// animation の番号の昇順。子の animation の番号は 0。
    pub fn cells(&self) -> impl Iterator<Item = (Option<PartKey>, u32, Cell<'_>)> {
        let top = self
            .frames
            .iter()
            .map(|(&id, slot)| (None, id, slot.cell()));
        let parts = self.parts.iter().flat_map(|(&surface_id, slots)| {
            slots
                .iter()
                .map(move |(&id, slot)| (Some(PartKey::Surface(surface_id)), id, slot.cell()))
        });
        let films = self
            .films
            .iter()
            .map(|(&film, &picture)| (Some(PartKey::Film(film)), 0, Cell::Picture(picture)));
        top.chain(parts).chain(films)
    }
}

#[cfg(test)]
#[path = "pattern_parts_tests.rs"]
mod parts_tests;

#[cfg(test)]
#[path = "pattern_cell_tests.rs"]
mod cell_tests;

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
