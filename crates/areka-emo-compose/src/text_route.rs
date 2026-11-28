//! スコープ 1 つの字の行き先の遷移と箱の表の導き方（純粋層・記録なし）。
//!
//! 「箱の列」はサーフェス番号に対する element番号の昇順の箱の名前の列で、「既定」は列が空なら
//! 普通のバルーン、空でなければ列の先頭の箱である。
//!
//! 不変条件: 行き先が `Box(n)` のとき、`n` は今のサーフェスの箱の列に在る。箱の表が空なら、
//! 行き先は普通のバルーンから動かない。

use std::collections::{BTreeMap, BTreeSet};

use crate::boxes::{BoxLayout, BoxName};

/// スコープの中の文字の場所。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextPlace {
    /// バルーン専用の窓に出る普通のバルーン。
    Balloon,
    /// element定義の描画メソッド `balloon` で置かれた箱。
    Box(BoxName),
}

/// `\s` の鍵を今のシェルで解決した結果。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceKeyOutcome {
    /// そのサーフェス番号を表示する。
    Show(u32),
    /// `\s[-1]`（非表示）。
    Hide,
    /// 解決できない（今のシェルにそのサーフェスが無い場合を含む）。何も変えない。
    Unresolved,
}

/// 名前の形の `\b[名前]` の結果。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoxSelect {
    /// 行き先がその箱へ替わった。
    Changed,
    /// もうその箱が行き先だった。
    Same,
    /// 今の面の箱の列にその名前が無い（行き先は変えない）。
    Missing,
}

/// スコープ 1 つの字の行き先（既定は非表示・普通のバルーン）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextRoute {
    surface: Option<u32>,
    dest: TextPlace,
}

impl Default for TextRoute {
    fn default() -> Self {
        TextRoute {
            surface: None,
            dest: TextPlace::Balloon,
        }
    }
}

impl TextRoute {
    /// 今のサーフェス番号（非表示・まだ `\s` が届いていなければ `None`）。
    pub fn surface(&self) -> Option<u32> {
        self.surface
    }

    /// 今の文字の行き先。
    pub fn dest(&self) -> &TextPlace {
        &self.dest
    }

    /// `\s` の答えで番号と行き先を決める。行き先が替わったら真。
    pub fn follow_surface(
        &mut self,
        boxes: &BTreeMap<u32, Vec<BoxName>>,
        outcome: SurfaceKeyOutcome,
    ) -> bool {
        self.surface = match outcome {
            SurfaceKeyOutcome::Show(id) => Some(id),
            SurfaceKeyOutcome::Hide => None,
            SurfaceKeyOutcome::Unresolved => return false,
        };
        let keep = match &self.dest {
            TextPlace::Box(name) => boxes_of(boxes, self.surface).contains(name),
            TextPlace::Balloon => false,
        };
        !keep && self.set_dest(default_dest(boxes, self.surface))
    }

    /// 名前の形の `\b[名前]` で行き先を今の面の箱へ切り替える。
    pub fn select_box(&mut self, boxes: &BTreeMap<u32, Vec<BoxName>>, name: &str) -> BoxSelect {
        match boxes_of(boxes, self.surface)
            .iter()
            .find(|n| n.as_str() == name)
        {
            Some(found) if self.set_dest(TextPlace::Box(found.clone())) => BoxSelect::Changed,
            Some(_) => BoxSelect::Same,
            None => BoxSelect::Missing,
        }
    }

    /// 台詞の頭・箱の表の差し替え: 行き先を今の番号の既定へ戻す（番号は変えない）。
    pub fn reset(&mut self, boxes: &BTreeMap<u32, Vec<BoxName>>) {
        self.dest = default_dest(boxes, self.surface);
    }

    /// シェルの切替の戻り先: 今の番号が新しいシェルに無いときだけ、候補の最初に在る番号（無ければ非表示）の答え。
    ///
    /// 番号の集合が空（材料なし）・非表示・在る番号なら `None`。
    pub fn fallback_outcome(
        &self,
        surface_ids: &BTreeSet<u32>,
        candidates: &[u32],
    ) -> Option<SurfaceKeyOutcome> {
        let current = self.surface?;
        if surface_ids.is_empty() || surface_ids.contains(&current) {
            return None;
        }
        let to = candidates
            .iter()
            .copied()
            .find(|id| surface_ids.contains(id));
        Some(to.map_or(SurfaceKeyOutcome::Hide, SurfaceKeyOutcome::Show))
    }

    /// `\b` の鍵が名前の形か（整数として読めない）。
    pub fn is_box_key(key: &str) -> bool {
        key.parse::<i64>().is_err()
    }

    fn set_dest(&mut self, dest: TextPlace) -> bool {
        if self.dest == dest {
            return false;
        }
        self.dest = dest;
        true
    }
}

/// 箱の表（サーフェス番号 → element番号の昇順の箱の名前）を置き場所の表から導く。
pub fn box_index(layout: &BoxLayout) -> BTreeMap<u32, Vec<BoxName>> {
    layout
        .surfaces()
        .map(|(id, placements)| (id, placements.iter().map(|p| p.name.clone()).collect()))
        .collect()
}

fn boxes_of(boxes: &BTreeMap<u32, Vec<BoxName>>, surface: Option<u32>) -> &[BoxName] {
    surface
        .and_then(|id| boxes.get(&id))
        .map_or(&[], Vec::as_slice)
}

fn default_dest(boxes: &BTreeMap<u32, Vec<BoxName>>, surface: Option<u32>) -> TextPlace {
    boxes_of(boxes, surface)
        .first()
        .map_or(TextPlace::Balloon, |name| TextPlace::Box(name.clone()))
}

#[cfg(test)]
#[path = "text_route_tests.rs"]
mod tests;
