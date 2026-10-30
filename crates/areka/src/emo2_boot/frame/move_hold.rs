//! 位置の指令の預かり（window-work-area-clamp design「MoveHold」・要件 6.2／6.3／6.4／7.3／7.8）。
//!
//! `\![move]` は、動かす窓と基準の窓（`--base` のスコープ）の**両方が最初の面を表示して拡大率が
//! 取れる**まで預かる。表示の前の窓は仮の矩形（表示の前の大きさ）のままなので、そこを基準に当てると
//! 表示の後の照合で窓が縮んだ分だけ隙間が残る（research 12 節の測り）。GPU の装着だけを待つ形は
//! 取らない——装着の後でも最初の `\s` の前の窓が残るからである。
//!
//! 預かった指令に関わる窓への後の指令は追い越さない（同じ窓への順を保つ）。関わりの無い窓の指令は
//! 先に当てる（表示されない窓が 1 つあっても列全体を詰まらせない）。預かる間は順のまま取りこぼさない。
//! 判定はゴースト・初回起動・特定のイベントを見ない。入力は窓の在庫・表示の拡大率・窓の組の帳簿だけである。

use std::collections::{BTreeSet, VecDeque};

use areka_emo_compose::ScaleRatio;
use areka_emo_present::EmoPresenter;
use tracing::{debug, info};

use crate::placement::spawn::GhostWindows;
use crate::placement::sticky_group_ledger::{
    StickyGroup, StickyGroupLedger, StickySource, parse_sticky_tokens,
};

use super::super::move_cue::{MoveBase, MoveDirective};
use super::super::target_map::shell_target;
use super::PositionCue;

/// スコープのシェルが最初の面を表示した後の拡大率を答える読み口（本番は表示層・檻は閉包）。
///
/// `None` は「まだ最初の面を表示していない」（または表示層に登録の無いスコープ）。
pub(crate) trait ShownRatioSource {
    fn shown_ratio(&self, scope: u32) -> Option<ScaleRatio>;
}

impl ShownRatioSource for EmoPresenter {
    fn shown_ratio(&self, scope: u32) -> Option<ScaleRatio> {
        self.applied_ratio(shell_target(scope))
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
    held: &VecDeque<PositionCue>,
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
        PositionCue::SetAlignment { scope, .. } | PositionCue::BalloonMoveLock { scope, .. } => {
            Some(vec![*scope])
        }
        PositionCue::SetSticky { tokens } => Some(sticky_members(tokens)),
        PositionCue::ResetSticky => Some(
            groups
                .ledger_groups()
                .filter(|g| g.source == StickySource::Script)
                .flat_map(|g| g.members.iter().copied())
                .chain(held.iter().flat_map(|c| match c {
                    PositionCue::SetSticky { tokens } => sticky_members(tokens),
                    _ => Vec::new(),
                }))
                .collect(),
        ),
        // バルーン位置初期化は窓ごとに自分のキャラ窓の今の位置から置き直し、後の移動はバルーンを連れ
        // 動かすので、移動とは入れ替えても同じ。順を保つのは預かったバルーン側の指令とだけ。
        PositionCue::ResetBalloonPos => Some(
            held.iter()
                .filter_map(|c| match c {
                    PositionCue::BalloonMoveLock { scope, .. }
                    | PositionCue::SetAlignment { scope, .. } => Some(*scope),
                    _ => None,
                })
                .collect(),
        ),
        // 並べ直しは全部の窓を動かす。
        PositionCue::ResetWindowPos => None,
    }
}

/// 移動が表示を待つスコープのうち、まだ最初の面を表示していないもの（無ければ当ててよい）。
///
/// 窓の無いスコープは待っても表示されないので待たない（動かす窓が無ければ当てる側が `warn!` で残し、
/// 基準の窓が無ければ当てる側が `screen` へ倒す）。
fn unshown_scope<S: ShownRatioSource + ?Sized>(
    d: &MoveDirective,
    windows: &GhostWindows,
    shown: &S,
) -> Option<u32> {
    let base = match d.base {
        MoveBase::Scope(base) => Some(base),
        _ => None,
    };
    std::iter::once(d.scope)
        .chain(base)
        .filter(|&s| windows.char_window(s as usize).is_some())
        .find(|&s| shown.shown_ratio(s).is_none())
}

/// 預かった指令が関わる窓（この巡の判定の中で積む）。
#[derive(Default)]
struct Blocked {
    all: bool,
    scopes: Vec<u32>,
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
/// 預かる理由は 2 つ: 移動の対象か基準の窓がまだ最初の面を表示していない／それより前に預かった指令と
/// 同じ窓に関わる（追い越さない）。記録は届いた巡に 1 度だけ残す（預かり続ける巡ごとには出さない）。
///
/// 並べ直し（`\![execute,resetwindowpos]`）は、それより前に預かった移動を当てずに捨てる。並べ直しは
/// 全部の窓を既定の位置へ置くので、順のまま当てても移動の結果は残らない（結果は順のまま当てたのと同じ）。
/// 捨てないと、最初の面を表示しない窓への移動が並べ直しを預からせ、それに続く全部の指令が預かられ
/// 続ける（表示されない窓が 1 つで列全体が詰まる）。
pub(super) fn release_ready<S: ShownRatioSource + ?Sized>(
    held: &mut VecDeque<PositionCue>,
    arrived: impl IntoIterator<Item = PositionCue>,
    windows: &GhostWindows,
    shown: &S,
    ledger: Option<&StickyGroupLedger>,
) -> Vec<PositionCue> {
    let mut queue: VecDeque<(PositionCue, bool)> = std::mem::take(held)
        .into_iter()
        .map(|cue| (cue, false))
        .chain(arrived.into_iter().map(|cue| (cue, true)))
        .collect();
    let mut groups = Groups {
        ledger,
        added: Vec::new(),
    };
    let mut blocked = Blocked::default();
    let mut ready = Vec::new();
    while let Some((cue, just_arrived)) = queue.pop_front() {
        if matches!(cue, PositionCue::ResetWindowPos)
            && held.iter().any(|c| matches!(c, PositionCue::Move(_)))
        {
            // 預かった移動を捨て、残りの預かり（移動でない指令）を並べ直しの前へ戻して判定し直す。
            // 残りは移動に関わって預かっていただけなので、移動が無くなればこの巡で当たる。
            // 並べ直しが材料の不在で読み飛ばした窓でも、捨てた移動は後で表示しても当てない
            // （`reset_window_pos_skipped` と `move_hold_superseded` の 2 行が残る）。
            let rest: Vec<PositionCue> = std::mem::take(held)
                .into_iter()
                .filter(|c| match c {
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
            queue.push_front((cue, just_arrived));
            for earlier in rest.into_iter().rev() {
                queue.push_front((earlier, false));
            }
            blocked = Blocked::default();
            continue;
        }
        let touched = touched_scopes(&cue, &groups, held);
        let unshown = match &cue {
            PositionCue::Move(d) => unshown_scope(d, windows, shown).map(|w| (d.scope, w)),
            _ => None,
        };
        if blocked.overlaps(&touched) {
            if just_arrived {
                debug!(
                    event = "move_hold_behind",
                    ?cue,
                    "先に預かった指令と同じ窓に関わるので追い越さずに預かる"
                );
            }
        } else if let Some((scope, waiting)) = unshown {
            if just_arrived {
                // 件 5 の測り（MoveMeasure・要件 6.1）の語を残す: 拡大率が取れない（表示の前）ので預かった。
                info!(
                    event = "move_ratio_unset",
                    scope,
                    waiting,
                    "表示の前なので \\![move] を預かる（対象と基準の窓が最初の面を表示した巡で当てる）"
                );
            }
        } else {
            if let PositionCue::SetSticky { tokens } = &cue {
                groups
                    .added
                    .push(sticky_members(tokens).into_iter().collect());
            }
            ready.push(cue);
            continue;
        }
        blocked.add(touched);
        held.push_back(cue);
    }
    ready
}
