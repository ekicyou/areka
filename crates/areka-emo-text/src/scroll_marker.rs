//! # scroll_marker — 場所ごとの手動の位置と送りの規則（純粋層）
//!
//! バルーン（箱）の文字を利用者が矢印やホイールで読み返す「手動の位置」を、場所ごとに
//! [`PlaceScroll`] 1 つで持つ。手動の位置は「先頭に見せる行の番号」で、いつも
//! `0 ≤ line < auto_first`（`auto_first`＝前のフレームの自動の送りの先頭行）に収まる。
//!
//! - 矢印のクリック 1 回で [`ARROW_STEP_LINES`] 行送る（SSP 2.9.26 で測った量・要件 5.1・5.2）。
//! - 古い側は行 0 で止まり（行き過ぎは 0 に詰める）、新しい側で `auto_first` に達したら
//!   手動を捨てて自動へ戻る（要件 5.3・5.4）。
//! - ホイールは 120 ごとに 1 目盛り、1 目盛りで OS の行数（「1 画面ずつ」なら見えている行の数）
//!   を送り、端数は送れる間だけ持ち越す（要件 6.3〜6.6）。
//! - 見える字の数が前より増えたら手動を捨てる（要件 8.1）。
//!
//! - 見える範囲（[`window_for`]）・矢印の出る条件（[`marker_visibility`]）・置き場
//!   （[`ResolvedMarkerPlacement`]）もここで純粋に決める（要件 3.1〜3.4・3.7・4.1・4.2・7.1・7.2・7.7）。
//!
//! **層規律**: 純粋層。時計も窓も `World` も見ない（`PURE_SOURCES` に載せて走査する）。

use areka_parsers::balloon::{ArrowKeys, MarkerPoint};

use crate::layout::{LayoutEngine, PositionedLine, VisibleWindow};
use crate::region::TextRegion;
use crate::writing::WritingMode;

/// 矢印のクリック 1 回で送る行数（SSP 2.9.26 で測った量・要件 5.1・5.2）。
pub const ARROW_STEP_LINES: usize = 2;

/// ホイールの 1 目盛り（OS の `WHEEL_DELTA`）。
const WHEEL_NOTCH: i32 = 120;

/// 送る向き。`Older`＝古い側（`arrow0`・ホイールの向こう側）、`Newer`＝新しい側（`arrow1`・手前）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDir {
    /// 古い側へ（押し出された行を見せる）。
    Older,
    /// 新しい側へ（自動の位置へ近づく）。
    Newer,
}

/// 1 回の操作で手動の位置がどう変わったか（呼び手が記録の段を選ぶ）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollChange {
    /// 自動から手動に入った。
    Started,
    /// 手動を捨てて自動へ戻った（理由つき）。
    Stopped(StopReason),
    /// 手動のまま位置が動いた。
    Moved,
    /// 何も変わらなかった。
    Unchanged,
}

/// 手動の位置を捨てた理由（要件 5.4・8.1〜8.3・8.6）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// 新しい側への送りが自動の位置に追い付いた。
    ReachedAuto,
    /// 見える字が増えた。
    NewText,
    /// 文字が消去された。
    Cleared,
    /// バルーン（箱）が消えた。
    Hidden,
    /// バルーンの面が替わった。
    FaceChanged,
    /// 箱の登録が外れた。
    BoxRemoved,
}

/// ホイールの 1 目盛りで送る量（OS の `SPI_GETWHEELSCROLLLINES` の値）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WheelLines {
    /// 1 目盛りでこの行数を送る。
    Lines(u32),
    /// OS が「1 画面ずつ」（`WHEEL_PAGESCROLL`）＝見えている行の数を送る。
    Page,
}

/// 場所ごとの手動の位置の状態（要件 8.8: 場所ごとに 1 つ）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PlaceScroll {
    /// 先頭に見せる行の番号（`None`＝自動）。
    manual: Option<usize>,
    /// 120 に満たない回転。
    wheel_remainder: i32,
    /// 前のフレームの見える字の数。
    last_visible: usize,
    /// 前のフレームの自動の先頭行（押下・ホイールの限り）。
    auto_first: usize,
    /// 前のフレームの自動の範囲に見えていた行の数（ホイールの「1 画面ずつ」の量）。
    page_lines: usize,
}

impl PlaceScroll {
    /// `dir` の向きへ `lines` 行送る。古い側は行 0 に詰め、新しい側で `auto_first` に達したら
    /// 手動を捨てて自動へ戻る。隠れた古い行が無ければ何もしない（要件 5.3・5.4・6.5）。
    pub fn step(&mut self, dir: ScrollDir, lines: usize) -> ScrollChange {
        if lines == 0 {
            return ScrollChange::Unchanged;
        }
        match (dir, self.manual) {
            (ScrollDir::Older, None) => {
                if self.auto_first == 0 {
                    return ScrollChange::Unchanged;
                }
                self.manual = Some(self.auto_first.saturating_sub(lines));
                ScrollChange::Started
            }
            (ScrollDir::Older, Some(0)) => ScrollChange::Unchanged,
            (ScrollDir::Older, Some(line)) => {
                self.manual = Some(line.saturating_sub(lines));
                ScrollChange::Moved
            }
            (ScrollDir::Newer, None) => ScrollChange::Unchanged,
            (ScrollDir::Newer, Some(line)) => {
                let next = line.saturating_add(lines);
                if next >= self.auto_first {
                    self.manual = None;
                    ScrollChange::Stopped(StopReason::ReachedAuto)
                } else {
                    self.manual = Some(next);
                    ScrollChange::Moved
                }
            }
        }
    }

    /// ホイールの回転 `delta`（正＝向こう側＝古い側）を足し、120 ごとの目盛りで送る。
    /// 1 目盛りの行数は `per_notch`（`Page` なら `visible_lines`）。送れない間は端数を 0 に戻す
    /// （持ち越しは送れる間だけ・要件 6.3〜6.5）。絵の有無とは無関係（要件 6.6）。
    pub fn wheel(
        &mut self,
        delta: i32,
        per_notch: WheelLines,
        visible_lines: usize,
    ) -> ScrollChange {
        if !self.has_hidden_lines() {
            self.wheel_remainder = 0;
            return ScrollChange::Unchanged;
        }
        let total = self.wheel_remainder.saturating_add(delta);
        let notches = total / WHEEL_NOTCH;
        self.wheel_remainder = total % WHEEL_NOTCH;
        if notches == 0 {
            return ScrollChange::Unchanged;
        }
        let per = match per_notch {
            WheelLines::Lines(n) => n as usize,
            WheelLines::Page => visible_lines,
        };
        let dir = if notches > 0 {
            ScrollDir::Older
        } else {
            ScrollDir::Newer
        };
        let change = self.step(dir, per.saturating_mul(notches.unsigned_abs() as usize));
        if change == ScrollChange::Unchanged {
            self.wheel_remainder = 0;
        }
        change
    }

    /// このフレームの見える字の数を覚える。前より増えていて手動の位置にいたら手動を捨て、
    /// [`StopReason::NewText`] を返す。減った（消去）ときは覚え直すだけ（要件 8.1）。
    pub fn observe_visible(&mut self, count: usize) -> Option<StopReason> {
        let grew = count > self.last_visible;
        self.last_visible = count;
        if grew && self.forget(StopReason::NewText) != ScrollChange::Unchanged {
            Some(StopReason::NewText)
        } else {
            None
        }
    }

    /// 手動の位置を捨てる。手動にいなければ何もしない。
    pub fn forget(&mut self, reason: StopReason) -> ScrollChange {
        if self.manual.take().is_some() {
            self.wheel_remainder = 0;
            ScrollChange::Stopped(reason)
        } else {
            ScrollChange::Unchanged
        }
    }

    /// 押し出された行か、見えていない新しい行があるか（ホイールを受けるかの判断）。
    pub fn has_hidden_lines(&self) -> bool {
        self.auto_first > 0 || self.manual.is_some()
    }

    /// 前のフレームの自動の範囲に見えていた行の数（[`WheelLines::Page`] の 1 目盛りの量）。
    pub fn page_lines(&self) -> usize {
        self.page_lines
    }
}

/// 矢印の種類。`Up`＝`arrow0`（古い側へ送る）・`Down`＝`arrow1`（新しい側へ送る）。
/// 古い側は書字方向の正準表で決まる（横書き＝上・`vertical_rl`＝右・`vertical_lr`＝左）が、
/// 矢印の役目と座標の意味は書字方向で入れ替えない（要件 4.1・4.2）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerKind {
    /// `arrow0`。
    Up,
    /// `arrow1`。
    Down,
}

impl MarkerKind {
    /// この矢印の押下で送る向き。
    pub fn dir(self) -> ScrollDir {
        match self {
            MarkerKind::Up => ScrollDir::Older,
            MarkerKind::Down => ScrollDir::Newer,
        }
    }
}

/// このフレームの見える範囲。自動の先頭行を覚え直し、手動の位置が自動の位置に追い付いて
/// いたら捨てる（捨てた理由を返す＝呼び手が `debug!` する）。手動の位置が在れば、その行から
/// 自動と同じ式で範囲を作る（行 0 なら一番古い行が描画範囲の始まりに来る・要件 5.3・8.5・8.7）。
/// 同じフレームの [`PlaceScroll::observe_visible`] の**後**に呼ぶ（字が増えたフレームを自動で描く）。
pub fn window_for(
    lines: &[PositionedLine],
    region: &TextRegion,
    mode: WritingMode,
    scroll: &mut PlaceScroll,
) -> (VisibleWindow, Option<StopReason>) {
    let auto = LayoutEngine::visible_window(lines, region, mode);
    scroll.auto_first = auto.first_visible_line;
    // shortcut: 自動の範囲（最新の行から収まる分）の行数を 1 画面とみなす。行の丈が揃わない台本では手動の位置の見えている数とずれうる。
    scroll.page_lines = lines.len() - auto.first_visible_line;
    match scroll.manual {
        Some(line) if line >= scroll.auto_first => {
            scroll.forget(StopReason::ReachedAuto);
            (auto, Some(StopReason::ReachedAuto))
        }
        // line < auto_first ≤ lines.len() − 1 なので添字は範囲内。
        Some(line) => (
            VisibleWindow {
                first_visible_line: line,
                block_offset: LayoutEngine::block_offset_at(&lines[line], region, mode),
            },
            None,
        ),
        None => (auto, None),
    }
}

/// どの矢印を出すか。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MarkerShown {
    /// `arrow0`（古い側に隠れた行が在る）。
    pub up: bool,
    /// `arrow1`（手動の位置に居る＝新しい側に隠れた行が在る）。
    pub down: bool,
}

/// 出る条件: 見えている先頭の行 > 0 で `arrow0`、手動の位置が在れば `arrow1`。
/// どちらも無ければ出さない（要件 3.1〜3.3・4.1・7.2。書字方向に依らず同じ）。
pub fn marker_visibility(window: &VisibleWindow, scroll: &PlaceScroll) -> MarkerShown {
    MarkerShown {
        up: window.first_visible_line > 0,
        down: scroll.manual.is_some(),
    }
}

/// 座標が無いときの置き場（左上・image px）。SSP 2.9.26 で測った値:
/// `arrow0`＝`(幅 − 20, 15)`・`arrow1`＝`(幅 − 20, 高さ − 15)`（要件 3.7・7.7）。
pub fn default_marker_position(kind: MarkerKind, extent: (i32, i32)) -> (i32, i32) {
    let (w, h) = extent;
    match kind {
        MarkerKind::Up => (w - 20, 15),
        MarkerKind::Down => (w - 20, h - 15),
    }
}

/// 2 つの矢印の左上（image px）。広がりは普通のバルーン＝面の画像、箱＝箱の大きさ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedMarkerPlacement {
    /// `arrow0` の左上。
    pub up: (i32, i32),
    /// `arrow1` の左上。
    pub down: (i32, i32),
}

impl ResolvedMarkerPlacement {
    /// x・y がそろった座標は広がりに解き、無ければ（片方だけも）既定の置き場にする
    /// （要件 3.4・3.7・4.2・7.1・7.7）。
    pub fn resolve(keys: &ArrowKeys, extent: (i32, i32)) -> Self {
        let at = |kind, point: &MarkerPoint| match point.both() {
            Some((x, y)) => (x.resolve(extent.0), y.resolve(extent.1)),
            _ => default_marker_position(kind, extent),
        };
        Self {
            up: at(MarkerKind::Up, &keys.arrow0),
            down: at(MarkerKind::Down, &keys.arrow1),
        }
    }
}

#[cfg(test)]
#[path = "scroll_marker_tests.rs"]
mod tests;
