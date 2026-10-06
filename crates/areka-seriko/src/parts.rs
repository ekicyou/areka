//! parts: 部品の時計（spec: areka-P0-surface-element-nesting 要件 5.1〜5.8・5.11〜5.14・8.3）。
//!
//! 部品＝element定義で置かれた子と、pattern定義が指して今の絵に出ているサーフェス。部品の
//! animation は、そのサーフェスを一番上に表示したときと同じ列（[`AnimationTable::animations`]）を
//! 同じ規則（境界の抽選・[`frame_at`] の進行・`-1` の停止・末尾の保持）で動かす。
//!
//! 時計の鍵は (スコープ, 部品の番号, animation の番号)。element定義の子も pattern定義の先も同じ鍵を
//! 使うので、同じ番号は 1 つの時計で揃って動く（要件 5.3・5.4・5.14）。見えない部品には触らない
//! （抽選・進行・欄への書き込みのどれもしない・要件 5.11）。`Playing` は開始の時刻を持ち続けるので、
//! 見えなかった間も進んでいた扱いになり、戻った刻みに経過から今のコマが決まる（要件 5.7）。
//!
//! 時計を動かすのは刻みの [`PartClocks::advance`] だけ。面の切り替え・着せ替えの変化の直後は
//! [`PartClocks::peek`] が時計を書き換えず抽選もせずに今のコマを求める（要件 5.6）。時計は
//! [`PartClocks::clear`]（シェルの表の差し替え）でだけ捨てる（要件 5.8）。発火・停止・末尾での保持の
//! 記録は `advance` だけが出す（`peek` は状態を変えないので記録しない）。

use std::collections::{BTreeMap, HashMap, HashSet};

use areka_emo_compose::{BindSet, PatternFrame, PatternState};
use areka_sakura::ActorKey;

use crate::looper::pattern_frame;
use crate::table::{AnimationTable, LoopAnimation, LoopTrigger};
use crate::timeline::{FrameStatus, LoopRng, current_frame_index, frame_at, should_fire};

/// animation 1 本の時計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PartAnim {
    /// 再生中（開始の時刻）。今のコマは `frame_at(経過)` で求める。
    Playing { started_at_ms: u64 },
    /// `-1` の無い末尾まで進んで、最後のコマを保っている（抽選の対象）。
    Residual { frame_index: usize },
}

/// スコープ × 部品の番号 × animation の番号ごとの時計（持ち主は `LoopRuntime`・シェル面だけ）。
#[derive(Default)]
pub(crate) struct PartClocks {
    /// スコープ → (部品の番号, animation の番号) → 時計。
    clocks: HashMap<ActorKey, BTreeMap<(u32, u32), PartAnim>>,
    /// `-1` 以外の負の番号の `warn!` を (スコープ, 部品の番号, animation の番号) ごとに初回だけ出した記録。
    warned_negative: HashSet<(ActorKey, u32, u32)>,
    /// `visible_parts` の作業用の列（刻みごとの確保を避けて使い回す）。
    visible: Vec<u32>,
    /// その刻みで評価した部品の番号（昇順・使い回す）。
    evaluated: Vec<u32>,
    /// 部品 1 つの animation を番号の昇順に回すための添字の列（使い回す）。
    order: Vec<usize>,
}

/// 時計 1 本を `now_ms` で見た答え（`advance` と `peek` で共有）。
enum Look {
    /// 時計が無いか、先頭の待ちの前（コマ無し）。
    Nothing,
    /// このコマを出す（再生中か、保っているコマ）。
    Frame(usize),
    /// `-1` の無い末尾に着いた（このコマを出す。`advance` は `Residual` へ移す）。
    Finished(usize),
    /// 負の番号のコマに着いた（`advance` は時計を消す）。
    Stopped { surface_id: i64 },
}

fn look(anim: &LoopAnimation, clock: Option<PartAnim>, now_ms: u64) -> Look {
    match clock {
        None => Look::Nothing,
        Some(PartAnim::Residual { frame_index }) => Look::Frame(frame_index),
        Some(PartAnim::Playing { started_at_ms }) => {
            let elapsed = now_ms.saturating_sub(started_at_ms);
            match frame_at(&anim.frames, elapsed) {
                FrameStatus::Pending => Look::Nothing,
                FrameStatus::Active(i) => Look::Frame(i),
                FrameStatus::FinishedResidual(i) => Look::Finished(i),
                FrameStatus::Stopped => Look::Stopped {
                    surface_id: current_frame_index(&anim.frames, elapsed)
                        .map_or(-1, |i| anim.frames[i].surface_id),
                },
            }
        }
    }
}

/// 着せ替えの番人: 抽選の頻度 K を返す。`bind+random` で `binds` に無いものは `None`（要件 5.12）。
fn gate(anim: &LoopAnimation, binds: &BindSet) -> Option<u32> {
    match anim.trigger {
        LoopTrigger::Random { k } => Some(k),
        LoopTrigger::BindRandom { k } => binds.contains(anim.id).then_some(k),
    }
}

/// `anims` を animation の番号の昇順に回す添字を `order` に入れる。
fn sort_by_id(anims: &[LoopAnimation], order: &mut Vec<usize>) {
    order.clear();
    order.extend(0..anims.len());
    order.sort_unstable_by_key(|&i| anims[i].id);
}

/// 部品の欄を空にし、見える部品が増えなくなるまで［求める → 未評価を番号の昇順に `evaluate`］を回す。
/// 評価済みの集合は増えるだけなので必ず止まる。最後に、同じ刻みの後の回で見えなくなった部品
/// （着せ替えのコマが pattern0 を置き換えた先など）のコマを欄から外す。見えない部品の辺は
/// `visible_parts` がたどらないので、外しても `visible` は変わらない（部品の欄の番号は `visible` に
/// 含まれる・design.md「Data Models」）。
fn rebuild(
    table: &AnimationTable,
    surface_id: u32,
    binds: &BindSet,
    pattern: &mut PatternState,
    visible: &mut Vec<u32>,
    evaluated: &mut Vec<u32>,
    mut evaluate: impl FnMut(u32, &mut PatternState),
) {
    pattern.clear_parts();
    evaluated.clear();
    let nest = table.nest_table();
    loop {
        nest.visible_parts(surface_id, binds, pattern, visible);
        let mut added = false;
        for &part in visible.iter() {
            let Err(at) = evaluated.binary_search(&part) else {
                continue;
            };
            evaluated.insert(at, part);
            added = true;
            evaluate(part, pattern);
        }
        if !added {
            break;
        }
    }

    if evaluated.len() > visible.len() {
        let kept: Vec<(u32, u32, PatternFrame)> = visible
            .iter()
            .flat_map(|&p| pattern.part(p).map(move |(id, f)| (p, id, f.clone())))
            .collect();
        pattern.clear_parts();
        for (p, id, f) in kept {
            pattern.set_part(p, id, f);
        }
    }
}

/// コマ `i` を部品の欄へ書く。
fn write(pattern: &mut PatternState, part: u32, anim: &LoopAnimation, i: usize) {
    if let Some(f) = anim.frames.get(i) {
        pattern.set_part(part, anim.id, pattern_frame(f));
    }
}

impl PartClocks {
    /// 刻み 1 回。見える部品を抽選（境界を跨いだ刻みだけ）・進行させ、
    /// `pattern` の部品の欄を今の絵の分で作り直す。
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn advance(
        &mut self,
        scope: &ActorKey,
        surface_id: u32,
        binds: &BindSet,
        table: &AnimationTable,
        now_ms: u64,
        crossed: bool,
        rng: &mut LoopRng,
        pattern: &mut PatternState,
    ) {
        let PartClocks {
            clocks,
            warned_negative,
            visible,
            evaluated,
            order,
        } = self;
        if !clocks.contains_key(scope) {
            clocks.insert(scope.clone(), BTreeMap::new());
        }
        let Some(clocks) = clocks.get_mut(scope) else {
            return;
        };

        // 部品 1 つの評価（animation の番号の昇順・design.md「部品 1 つの評価」）。
        let evaluate = |part: u32, pattern: &mut PatternState| {
            let anims = table.animations(part);
            sort_by_id(anims, order);
            for &i in order.iter() {
                let anim = &anims[i];
                let key = (part, anim.id);
                // 着せ替えの種類で有効でないものは抽選せず、時計を消す（要件 5.12）。
                let Some(k) = gate(anim, binds) else {
                    if clocks.remove(&key).is_some() {
                        tracing::info!(
                            scope = scope.as_str(),
                            part,
                            animation_id = anim.id,
                            "seriko: part bind から外れた ID の再生を停止（保持コマ除去・要件 5.12）"
                        );
                    }
                    continue;
                };
                // 境界を跨いだ刻みで再生中でなければ抽選（保っているコマは当たれば捨てる）。
                let playing = matches!(clocks.get(&key), Some(PartAnim::Playing { .. }));
                if crossed && !playing && should_fire(k, rng) {
                    clocks.insert(
                        key,
                        PartAnim::Playing {
                            started_at_ms: now_ms,
                        },
                    );
                    tracing::info!(
                        scope = scope.as_str(),
                        part,
                        animation_id = anim.id,
                        k,
                        "seriko: part 抽選発火（再生開始・先頭コマから・要件 5.1）"
                    );
                }
                // 進行: 開始からの経過で今のコマを決める（見えなかった間も進んでいた扱い・要件 5.7）。
                match look(anim, clocks.get(&key).copied(), now_ms) {
                    Look::Nothing => {}
                    Look::Frame(i) => write(pattern, part, anim, i),
                    Look::Finished(i) => {
                        clocks.insert(key, PartAnim::Residual { frame_index: i });
                        tracing::info!(
                            scope = scope.as_str(),
                            part,
                            animation_id = anim.id,
                            "seriko: part 末尾残留（最終コマ保持・再抽選対象へ・要件 5.1）"
                        );
                        write(pattern, part, anim, i);
                    }
                    Look::Stopped { surface_id } => {
                        // `-1` は正典駆動、それ以外の負値は初回だけ warn!（自アニメ停止扱い・他アニメ停止は非駆動）。
                        if surface_id != -1
                            && warned_negative.insert((scope.clone(), part, anim.id))
                        {
                            tracing::warn!(
                                scope = scope.as_str(),
                                part,
                                animation_id = anim.id,
                                surface_id,
                                "seriko: part `-1` 以外の負 surface（自アニメ停止扱い・他アニメ停止 `-2` は非駆動）"
                            );
                        }
                        clocks.remove(&key);
                        tracing::info!(
                            scope = scope.as_str(),
                            part,
                            animation_id = anim.id,
                            "seriko: part 停止（負 surface でベース復帰・要件 5.1）"
                        );
                    }
                }
            }
        };

        rebuild(
            table, surface_id, binds, pattern, visible, evaluated, evaluate,
        );
    }

    /// 時計を書き換えず、抽選もせずに、`now_ms` の時点の部品のコマで `pattern` の部品の欄を作り直す
    /// （面の切り替え・着せ替えの変化の直後・要件 5.6）。見える部品の求め方は `advance` と同じ。
    /// `bind+random` で `binds` に無い animation のコマは書かない（時計を消すのは次の `advance`・要件 5.12）。
    /// 末尾に着いたコマ・負の番号のコマは、時計を移さず・消さずに、出す・出さないだけを決める。
    pub(crate) fn peek(
        &self,
        scope: &ActorKey,
        surface_id: u32,
        binds: &BindSet,
        table: &AnimationTable,
        now_ms: u64,
        pattern: &mut PatternState,
    ) {
        let clocks = self.clocks.get(scope);
        let mut order = Vec::new();
        let evaluate = |part: u32, pattern: &mut PatternState| {
            let anims = table.animations(part);
            sort_by_id(anims, &mut order);
            for &i in order.iter() {
                let anim = &anims[i];
                if gate(anim, binds).is_none() {
                    continue;
                }
                let clock = clocks.and_then(|c| c.get(&(part, anim.id))).copied();
                if let Look::Frame(i) | Look::Finished(i) = look(anim, clock, now_ms) {
                    write(pattern, part, anim, i);
                }
            }
        };
        rebuild(
            table,
            surface_id,
            binds,
            pattern,
            &mut Vec::new(),
            &mut Vec::new(),
            evaluate,
        );
    }

    /// 一番上の再生が無い刻みで部品の経路を通すか: `pattern` の絵に見える部品に、動く animation を
    /// 持つものが在るか（作業用の列を使い回す・design.md「Performance & Scalability」）。
    ///
    /// 保持している部品の欄に載る番号は見える部品に含まれ（design.md「Data Models」の不変条件）、
    /// 欄のコマはその部品の animation のコマなので、「部品の欄が空でない」もこの判定に含まれる。
    pub(crate) fn moving_visible(
        &mut self,
        surface_id: u32,
        binds: &BindSet,
        table: &AnimationTable,
        pattern: &PatternState,
    ) -> bool {
        table
            .nest_table()
            .visible_parts(surface_id, binds, pattern, &mut self.visible);
        self.visible
            .iter()
            .any(|&p| !table.animations(p).is_empty())
    }

    /// 全スコープの時計と、負の番号の `warn!` の記録を捨てる（シェルの表の差し替え・要件 5.8）。
    pub(crate) fn clear(&mut self) {
        self.clocks.clear();
        self.warned_negative.clear();
    }

    /// `scope` の (部品の番号, animation の番号) の時計を引く（テストの観測用）。
    #[cfg(test)]
    pub(crate) fn clock(&self, scope: &ActorKey, part: u32, animation_id: u32) -> Option<PartAnim> {
        self.clocks.get(scope)?.get(&(part, animation_id)).copied()
    }

    /// 最後の `clear` の後（または作ってから）`advance` がどのスコープでも 1 度も走っていないか
    /// （テストの観測用・`advance` はスコープの入れ物を先に作る）。
    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.clocks.is_empty()
    }
}

#[cfg(test)]
#[path = "parts_tests.rs"]
mod tests;
