//! 位置の指令の預かり。
//!
//! `\![move]` は、動かす窓と基準の窓（`--base` のスコープ）の**両方が最初の面を表示して拡大率が
//! 取れる**まで預かる。表示の前の窓は仮の矩形（表示の前の大きさ）のままなので、そこを基準に当てると
//! 表示の後の照合で窓が縮んだ分だけ隙間が残る。GPU の装着だけを待つ形は
//! 取らない——装着の後でも最初の `\s` の前の窓が残るからである。
//!
//! 預かった指令に関わる窓への後の指令は追い越さない（同じ窓への順を保つ）。関わりの無い窓の指令は
//! 先に当てる（表示されない窓が 1 つあっても列全体を詰まらせない）。預かる間は順のまま取りこぼさない。
//! 並べ直しは、窓のある全部のスコープの表示が決まる（表示した、または表示しないまま決まった）まで
//! 預かる。仮の大きさで並べると、表示の後の照合で縮んだ分だけ先頭の窓が作業領域の端から離れる。
//!
//! 面の指令をまだ受けていない窓は、その指令を運んだトークが終わるまで待ち、終わったら表示しないまま
//! 決まった（隠れたまま）と読む。
//!
//! 判定はゴースト・初回起動・特定のイベントを見ない。入力は窓の在庫・表示の拡大率・窓の組の帳簿と、
//! トークの終わりの通し番号だけである。

use std::collections::{BTreeSet, VecDeque};

use areka_emo_compose::ScaleRatio;
use areka_emo_present::{EmoPresenter, NeverShown};
use tracing::{debug, info};

use areka_placement::spawn::GhostWindows;
use areka_placement::sticky_group_ledger::{
    StickyGroup, StickyGroupLedger, StickySource, parse_sticky_tokens,
};

use super::super::move_cue::{MoveBase, MoveDirective};
use super::super::target_map::shell_target;
use super::PositionCue;

/// スコープのシェルが最初の面を表示した後の拡大率を答える読み口（本番は表示層・檻は閉包）。
///
/// `None` は「まだ最初の面を表示していない」（または表示層に登録の無いスコープ）。
// 公開の `run_move_drain_with` が名指すので開ける
pub trait ShownRatioSource {
    fn shown_ratio(&self, scope: u32) -> Option<ScaleRatio>;

    /// 表示しないまま決まった理由（まだ決まっていなければ `None`）。
    fn never_shown(&self, _scope: u32) -> Option<NeverShown> {
        None
    }

    /// 実際に表示した窓の物理寸（まだ表示していなければ `None`）。
    fn physical_size(&self, _scope: u32) -> Option<(u32, u32)> {
        None
    }
}

impl ShownRatioSource for EmoPresenter {
    fn shown_ratio(&self, scope: u32) -> Option<ScaleRatio> {
        self.applied_ratio(shell_target(scope))
    }

    fn never_shown(&self, scope: u32) -> Option<NeverShown> {
        EmoPresenter::never_shown(self, shell_target(scope))
    }

    fn physical_size(&self, scope: u32) -> Option<(u32, u32)> {
        self.target_physical_size(shell_target(scope))
    }
}

impl<F: Fn(u32) -> Option<ScaleRatio>> ShownRatioSource for F {
    fn shown_ratio(&self, scope: u32) -> Option<ScaleRatio> {
        self(scope)
    }
}

/// 窓の組の見え方（帳簿の今と、この巡で当てると決めた組の作成）。移動が連れ動かす相手を引く。
///
/// 当てる指令はこの巡の判定がすべて済んでから当たるので、同じ巡で先に当てると決めた組の作成は
/// 帳簿にまだ無い。その分を `added` で足す（足りない側に外すと関わる窓を見落として追い越すため）。
struct Groups<'a> {
    ledger: Option<&'a StickyGroupLedger>,
    added: Vec<BTreeSet<u32>>,
}

impl Groups<'_> {
    fn ledger_groups(&self) -> impl Iterator<Item = &StickyGroup> {
        self.ledger.into_iter().flat_map(|l| l.groups())
    }

    /// `scope` と同じ組の窓（`scope` 自身を含む）。
    fn members_with(&self, scope: u32) -> impl Iterator<Item = u32> + '_ {
        self.ledger_groups()
            .map(|g| &g.members)
            .chain(&self.added)
            .filter(move |m| m.contains(&scope))
            .flatten()
            .copied()
    }
}

/// 預かった位置の指令と、届いたときのトークの終わりの通し番号。
pub(super) struct HeldCue {
    cue: PositionCue,
    talk_ends: u64,
    /// 並べ直しの `reset_window_pos_held` を残したか（待つ間に 1 度だけ残す）。
    held_logged: bool,
}

/// `\![set,sticky-window,…]` が組にする窓（読めない並びは何も組まないので空）。
fn sticky_members(tokens: &[String]) -> Vec<u32> {
    parse_sticky_tokens(tokens)
        .map(|m| m.into_iter().collect())
        .unwrap_or_default()
}

/// 指令が関わる窓のスコープ。`None` はすべての窓に関わる指令（並べ直し）。
///
/// 移動は対象・基準と、連れ動く組の相手に関わる。組の指令は連れ動く相手が変わる窓だけに関わる:
/// 組の作成は名指しの窓（重なる組は拒否されるので他の組は変えない）、台本の組を解く指令は台本の組の
/// 窓（帳簿の今と、預かっている組の作成の分）。
fn touched_scopes(
    cue: &PositionCue,
    groups: &Groups,
    held: &VecDeque<HeldCue>,
) -> Option<Vec<u32>> {
    match cue {
        PositionCue::Move(d) => {
            let mut scopes = vec![d.scope];
            if let MoveBase::Scope(base) = d.base {
                scopes.push(base);
            }
            if !d.ignore_sticky_window {
                scopes.extend(groups.members_with(d.scope));
            }
            Some(scopes)
        }
        PositionCue::SetAlignment { scope, .. }
        | PositionCue::BalloonMoveLock { scope, .. }
        | PositionCue::SetBalloonOffset { scope, .. }
        | PositionCue::SetBalloonAlign { scope, .. } => Some(vec![*scope]),
        PositionCue::SetSticky { tokens } => Some(sticky_members(tokens)),
        PositionCue::ResetSticky => Some(
            groups
                .ledger_groups()
                .filter(|g| g.source == StickySource::Script)
                .flat_map(|g| g.members.iter().copied())
                .chain(held.iter().flat_map(|h| match &h.cue {
                    PositionCue::SetSticky { tokens } => sticky_members(tokens),
                    _ => Vec::new(),
                }))
                .collect(),
        ),
        // バルーン位置初期化は窓ごとに自分のキャラ窓の今の位置から置き直し、後の移動はバルーンを連れ
        // 動かすので、移動とは入れ替えても同じ。順を保つのは預かったバルーン側の指令とだけ。
        PositionCue::ResetBalloonPos => Some(
            held.iter()
                .filter_map(|h| match h.cue {
                    PositionCue::BalloonMoveLock { scope, .. }
                    | PositionCue::SetAlignment { scope, .. }
                    | PositionCue::SetBalloonOffset { scope, .. }
                    | PositionCue::SetBalloonAlign { scope, .. } => Some(scope),
                    _ => None,
                })
                .collect(),
        ),
        // 並べ直しは全部の窓を動かす。
        PositionCue::ResetWindowPos => None,
    }
}

/// スコープの表示の決まり（窓のあるスコープだけを判定する）。
enum Settle {
    Shown,
    NeverShown(NeverShown),
    Pending,
}

/// `talk_ended` は、指令を運んだトークが終わったか（終わる前は面の指令の来ていない窓にも面が来うる）。
fn settle<S: ShownRatioSource + ?Sized>(scope: u32, shown: &S, talk_ended: bool) -> Settle {
    if shown.shown_ratio(scope).is_some() {
        return Settle::Shown;
    }
    match shown.never_shown(scope) {
        None => Settle::Pending,
        Some(NeverShown::NotRequested) if !talk_ended => Settle::Pending,
        Some(reason) => Settle::NeverShown(reason),
    }
}

fn reason_word(reason: NeverShown) -> &'static str {
    match reason {
        NeverShown::Hidden => "hidden",
        NeverShown::Failed => "failed",
        NeverShown::NotRequested => "not_requested",
    }
}

fn window_scopes(windows: &GhostWindows) -> impl Iterator<Item = u32> + '_ {
    windows.scopes().map(|s| s as u32)
}

/// 窓があり、表示の決まっていないスコープ（昇順）。空なら並べ直しを当ててよい。
fn pending_scopes<S: ShownRatioSource + ?Sized>(
    windows: &GhostWindows,
    shown: &S,
    talk_ended: bool,
) -> Vec<u32> {
    window_scopes(windows)
        .filter(|&s| matches!(settle(s, shown, talk_ended), Settle::Pending))
        .collect()
}

/// 移動が表示を待つスコープ（対象・基準）の決まり。
///
/// 窓の無いスコープは待っても表示されないので待たない（動かす窓が無ければ当てる側が `warn!` で残し、
/// 基準の窓が無ければ当てる側が `screen` へ倒す）。
fn move_waits<S: ShownRatioSource + ?Sized>(
    d: &MoveDirective,
    windows: &GhostWindows,
    shown: &S,
    talk_ended: bool,
) -> Vec<(u32, Settle)> {
    let base = match d.base {
        MoveBase::Scope(base) => Some(base),
        _ => None,
    };
    std::iter::once(d.scope)
        .chain(base)
        .filter(|&s| windows.char_window(s as usize).is_some())
        .map(|s| (s, settle(s, shown, talk_ended)))
        .collect()
}

/// 預かった指令が関わる窓（この巡の判定の中で積む）。
#[derive(Default)]
struct Blocked {
    all: bool,
    scopes: BTreeSet<u32>,
}

impl Blocked {
    fn is_empty(&self) -> bool {
        !self.all && self.scopes.is_empty()
    }

    fn overlaps(&self, touched: &Option<Vec<u32>>) -> bool {
        match touched {
            _ if self.all => true,
            None => !self.is_empty(),
            Some(ss) => ss.iter().any(|s| self.scopes.contains(s)),
        }
    }

    fn add(&mut self, touched: Option<Vec<u32>>) {
        match touched {
            None => self.all = true,
            Some(ss) => self.scopes.extend(ss),
        }
    }
}

/// 前の巡から預かっている指令（先）と、この巡に届いた指令（後）を順に判定し、今当てる指令を順のまま
/// 返す。当てない指令は `held` へ順のまま戻す（取りこぼさない）。
///
/// 預かる理由は 3 つ: 移動の対象か基準の窓の表示がまだ決まっていない（表示しないまま決まった窓への
/// 移動は預からずに位置だけ置く）／並べ直しのとき窓のある
/// どれかのスコープの表示がまだ決まっていない／それより前に預かった指令と同じ窓に関わる（追い越さない）。
/// 記録は届いた巡に 1 度だけ残す（預かり続ける巡ごとには出さない）。塞がれて届いた並べ直しは、自分が
/// 表示を待ち始めた巡に 1 度残す。
///
/// 届いた指令には `arrived_talk_ends`（前に線を取り出したときの番号）を添え、`talk_ends`（今の番号）が
/// それより進んだ指令だけ、運んだトークが終わったと読む。今の番号を添えると、トークの終わりと同じ巡に
/// 届いた指令が次のトークの終わりまで待つ。
///
/// 並べ直し（`\![execute,resetwindowpos]`）は、それより前に預かった移動を当てずに捨てる。並べ直しは
/// 全部の窓を既定の位置へ置くので、順のまま当てても移動の結果は残らない（結果は順のまま当てたのと同じ）。
/// 捨てないと、最初の面を表示しない窓への移動が並べ直しを預からせ、それに続く全部の指令が預かられ
/// 続ける（表示されない窓が 1 つで列全体が詰まる）。
pub(super) fn release_ready<S: ShownRatioSource + ?Sized>(
    held: &mut VecDeque<HeldCue>,
    arrived: impl IntoIterator<Item = PositionCue>,
    arrived_talk_ends: u64,
    talk_ends: u64,
    windows: &GhostWindows,
    shown: &S,
    ledger: Option<&StickyGroupLedger>,
) -> Vec<PositionCue> {
    let mut queue: VecDeque<(HeldCue, bool)> = std::mem::take(held)
        .into_iter()
        .map(|h| (h, false))
        .chain(arrived.into_iter().map(|cue| {
            let h = HeldCue {
                cue,
                talk_ends: arrived_talk_ends,
                held_logged: false,
            };
            (h, true)
        }))
        .collect();
    let mut groups = Groups {
        ledger,
        added: Vec::new(),
    };
    let mut blocked = Blocked::default();
    let mut ready = Vec::new();
    while let Some((mut h, just_arrived)) = queue.pop_front() {
        let talk_ended = talk_ends > h.talk_ends;
        if matches!(h.cue, PositionCue::ResetWindowPos)
            && held.iter().any(|c| matches!(c.cue, PositionCue::Move(_)))
        {
            // 預かった移動を捨て、残りの預かり（移動でない指令）を並べ直しの前へ戻して判定し直す。
            // 残りは移動に関わって預かっていただけなので、移動が無くなればこの巡で当たる。
            // 並べ直しが材料の不在で読み飛ばした窓でも、捨てた移動は後で表示しても当てない
            // （`reset_window_pos_skipped` と `move_hold_superseded` の 2 行が残る）。
            let rest: Vec<HeldCue> = std::mem::take(held)
                .into_iter()
                .filter(|c| match &c.cue {
                    PositionCue::Move(d) => {
                        info!(
                            event = "move_hold_superseded",
                            scope = d.scope,
                            "預かった \\![move] は後の並べ直しで上書きされるので当てずに捨てる"
                        );
                        false
                    }
                    _ => true,
                })
                .collect();
            queue.push_front((h, just_arrived));
            for earlier in rest.into_iter().rev() {
                queue.push_front((earlier, false));
            }
            blocked = Blocked::default();
            continue;
        }
        let touched = touched_scopes(&h.cue, &groups, held);
        let waits = match &h.cue {
            PositionCue::Move(d) => move_waits(d, windows, shown, talk_ended),
            _ => Vec::new(),
        };
        let unshown = match &h.cue {
            PositionCue::Move(d) => waits
                .iter()
                .find(|(_, w)| matches!(w, Settle::Pending))
                .map(|&(w, _)| (d.scope, w)),
            _ => None,
        };
        let reset_waiting = match &h.cue {
            PositionCue::ResetWindowPos => pending_scopes(windows, shown, talk_ended),
            _ => Vec::new(),
        };
        if blocked.overlaps(&touched) {
            if just_arrived {
                debug!(
                    event = "move_hold_behind",
                    cue = ?h.cue,
                    "先に預かった指令と同じ窓に関わるので追い越さずに預かる"
                );
            }
        } else if let Some((scope, waiting)) = unshown {
            if just_arrived {
                // 拡大率が取れない（表示の前）ので預かったことを、測りの語で残す。
                info!(
                    event = "move_ratio_unset",
                    scope,
                    waiting,
                    "表示の前なので \\![move] を預かる（対象と基準の窓が最初の面を表示した巡で当てる）"
                );
            }
        } else if !reset_waiting.is_empty() {
            if !h.held_logged {
                h.held_logged = true;
                info!(
                    event = "reset_window_pos_held",
                    waiting = ?reset_waiting,
                    "表示の決まっていない窓があるので並べ直しを預かる（窓のある全部のスコープの表示が決まった巡で当てる）"
                );
            }
        } else {
            if matches!(h.cue, PositionCue::ResetWindowPos) {
                for scope in window_scopes(windows) {
                    if let Settle::NeverShown(reason) = settle(scope, shown, talk_ended) {
                        info!(
                            event = "reset_window_pos_unshown",
                            scope,
                            reason = reason_word(reason),
                            "最初の面を表示しないまま決まった窓は待たずに並べ直す（今の大きさで入れる）"
                        );
                    }
                }
            }
            if let PositionCue::Move(d) = &h.cue
                && let Some(&(waiting, Settle::NeverShown(reason))) = waits
                    .iter()
                    .find(|(_, w)| matches!(w, Settle::NeverShown(_)))
            {
                info!(
                    event = "move_hold_unshown",
                    scope = d.scope,
                    waiting,
                    reason = reason_word(reason),
                    handling = "placed",
                    "最初の面を表示しないまま決まった窓への \\![move] は預からずに位置だけ置く（今の大きさで当てる）"
                );
            }
            if let PositionCue::SetSticky { tokens } = &h.cue {
                groups
                    .added
                    .push(sticky_members(tokens).into_iter().collect());
            }
            ready.push(h.cue);
            continue;
        }
        blocked.add(touched);
        held.push_back(h);
    }
    ready
}

#[cfg(test)]
#[path = "move_hold_tests.rs"]
mod tests;
