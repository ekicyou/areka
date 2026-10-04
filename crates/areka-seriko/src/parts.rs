//! parts: 部品の時計（spec: areka-P0-surface-element-nesting 要件 5.1〜5.5・5.7・5.11〜5.14）。
//!
//! 部品＝element定義で置かれた子と、pattern定義が指して今の絵に出ているサーフェス。部品の
//! animation は、そのサーフェスを一番上に表示したときと同じ列（[`AnimationTable::animations`]）を
//! 同じ規則（境界の抽選・[`frame_at`] の進行・`-1` の停止・末尾の保持）で動かす。
//!
//! 時計の鍵は (スコープ, 部品の番号, animation の番号)。element定義の子も pattern定義の先も同じ鍵を
//! 使うので、同じ番号は 1 つの時計で揃って動く（要件 5.3・5.4・5.14）。見えない部品には触らない
//! （抽選・進行・欄への書き込みのどれもしない・要件 5.11）。`Playing` は開始の時刻を持ち続けるので、
//! 見えなかった間も進んでいた扱いになり、戻った刻みに経過から今のコマが決まる（要件 5.7）。

use std::collections::{BTreeMap, HashMap};

use areka_emo_compose::{BindSet, PatternFrame, PatternState};
use areka_sakura::ActorKey;

use crate::looper::pattern_frame;
use crate::table::{AnimationTable, LoopTrigger};
use crate::timeline::{FrameStatus, LoopRng, frame_at, should_fire};

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
    /// `visible_parts` の作業用の列（刻みごとの確保を避けて使い回す）。
    visible: Vec<u32>,
    /// その刻みで評価した部品の番号（昇順・使い回す）。
    evaluated: Vec<u32>,
    /// 部品 1 つの animation を番号の昇順に回すための添字の列（使い回す）。
    order: Vec<usize>,
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
        let mut evaluate = |part: u32, pattern: &mut PatternState| {
            let anims = table.animations(part);
            order.clear();
            order.extend(0..anims.len());
            order.sort_unstable_by_key(|&i| anims[i].id);
            for &i in order.iter() {
                let anim = &anims[i];
                let key = (part, anim.id);
                // 着せ替えの種類で有効でないものは抽選せず、時計を消す（要件 5.12）。
                let k = match anim.trigger {
                    LoopTrigger::Random { k } => k,
                    LoopTrigger::BindRandom { k } => {
                        if !binds.contains(anim.id) {
                            clocks.remove(&key);
                            continue;
                        }
                        k
                    }
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
                }
                // 進行: 開始からの経過で今のコマを決める（見えなかった間も進んでいた扱い・要件 5.7）。
                let frame_index = match clocks.get(&key).copied() {
                    None => continue,
                    Some(PartAnim::Residual { frame_index }) => frame_index,
                    Some(PartAnim::Playing { started_at_ms }) => {
                        match frame_at(&anim.frames, now_ms.saturating_sub(started_at_ms)) {
                            FrameStatus::Pending => continue,
                            FrameStatus::Active(i) => i,
                            FrameStatus::FinishedResidual(i) => {
                                clocks.insert(key, PartAnim::Residual { frame_index: i });
                                i
                            }
                            FrameStatus::Stopped => {
                                clocks.remove(&key);
                                continue;
                            }
                        }
                    }
                };
                if let Some(f) = anim.frames.get(frame_index) {
                    pattern.set_part(part, anim.id, pattern_frame(f));
                }
            }
        };

        // 部品の欄を空にし、見える部品が増えなくなるまで［求める → 未評価を番号の昇順に評価］を回す。
        // 評価済みの集合は増えるだけなので必ず止まる。
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

        // 同じ刻みの後の回で見えなくなった部品（着せ替えのコマが pattern0 を置き換えた先など）の
        // コマを欄から外す。見えない部品の辺は `visible_parts` がたどらないので、外しても `visible`
        // は変わらない（部品の欄の番号は `visible` に含まれる・design.md「Data Models」）。
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

    /// `scope` の (部品の番号, animation の番号) の時計を引く（テストの観測用）。
    #[cfg(test)]
    pub(crate) fn clock(&self, scope: &ActorKey, part: u32, animation_id: u32) -> Option<PartAnim> {
        self.clocks.get(scope)?.get(&(part, animation_id)).copied()
    }
}

#[cfg(test)]
#[path = "parts_tests.rs"]
mod tests;
