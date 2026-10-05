//! 出す番の状態機械（純粋）。
//!
//! 時刻・マウスの位置・押下・足元・追っている範囲の様子を引数で受けて 1 回進め、
//! 「すること」を返す。自分では時計も OS も読まない（OS の設定も引数の関数で受ける）。

use super::geometry::{PointPx, RectPx, in_safe_zone};
use super::ranges::{TooltipArea, TooltipRangeId};
use crate::ecs::PointF;
use bevy_ecs::prelude::*;
use std::time::{Duration, Instant};
use tracing::trace;

/// OS の待ち時間の設定を読めないときに使う値（OS の既定と同じ）。
pub(crate) const FALLBACK_HOVER_TIME: Duration = Duration::from_millis(400);
/// 出す番の間の見回りの間隔。
const PATROL: Duration = Duration::from_millis(100);
/// 直前のツールチップが消えてからこの間に入れば、待ち時間を設定の 1 倍にする。
const RESHOW: Duration = Duration::from_millis(200);

/// 出す番の印（通し番号・使い回さない・中身は非公開）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TooltipTurnToken(u64);

/// 出す番が終わった・ツールチップが消えた理由。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TooltipEndReason {
    LeftSafeZone,
    ButtonPressed,
    WindowHidden,
    WindowDestroyed,
    RangeUnregistered,
}

/// 足元（どの窓のどの範囲に入っているか）。入っていなければ None を渡す。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Over {
    pub window: Entity,
    pub range: TooltipRangeId,
    pub area_logical: TooltipArea,
    pub has_text: bool,
    pub pos_logical: PointF,
}

/// 今追っている範囲の様子（追っていなければ渡さない）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Tracked {
    // receives＝その窓の配下に PointerState がある かつ マウスの位置が hit_test_in_window に当たる
    // かつ 窓が見えている（足元の候補 Over と同じ 3 つの条件）。
    Present { range_px: RectPx, receives: bool },
    RangeUnregistered,
    WindowDestroyed,
    WindowHidden,
}

#[derive(Debug)]
pub(crate) struct StepInput {
    pub now: Instant,
    /// 画面の位置。読めなければ None（待ちを進めず、出す番も終わらせない）。
    pub cursor: Option<PointPx>,
    /// 押している、または前回の判定の後に押された。
    pub button_down: bool,
    pub over: Option<Over>,
    pub tracked: Option<Tracked>,
}

/// 状態機械が殻に頼む「すること」。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Effect {
    /// 期限を預ける（待ちの期限・出す番の間の見回り）。
    ArmDeadline(Instant),
    /// 来た。stored が真なら預けた文字を出す。
    TurnStarted {
        token: TooltipTurnToken,
        window: Entity,
        range: TooltipRangeId,
        pos_logical: PointF,
        stored: bool,
    },
    /// 出ているツールチップを消す。
    Hide {
        token: TooltipTurnToken,
        reason: HideReason,
    },
    /// 終わった。
    TurnEnded {
        token: TooltipTurnToken,
        window: Entity,
        range: TooltipRangeId,
        reason: TooltipEndReason,
    },
}

/// 消した理由（記録用）。利用側の求め・空の文字は印の照合（supply・dismiss）と一緒に足す。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum HideReason {
    End(TooltipEndReason),
}

/// 出す番の状態機械。
#[derive(Debug, Default)]
pub(crate) struct TurnMachine {
    state: State,
    /// 最後に発行した印の番号（0 はまだ無い）。
    last_token: u64,
    /// 出ているツールチップの画面の矩形。
    tip: Option<RectPx>,
    /// 最後にツールチップが消えた時刻（出し直しの 0.2 秒の起点）。
    last_hidden: Option<Instant>,
    /// 前回の判定のマウスの位置（動いたかを見る）。
    last_cursor: Option<PointPx>,
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Idle,
    Waiting {
        window: Entity,
        range: TooltipRangeId,
        deadline: Instant,
    },
    Active(Turn),
    /// 押したので、この範囲から出るまで出さない。
    Suppressed {
        range: TooltipRangeId,
    },
}

/// 続いている出す番。範囲の矩形は始まったときのものを持つ（途中の差し替えは次から効く）。
#[derive(Debug)]
struct Turn {
    token: TooltipTurnToken,
    window: Entity,
    range: TooltipRangeId,
    area: TooltipArea,
}

impl TurnMachine {
    /// 1 回進めて「すること」を返す。hover_time は待ちに入るときだけ呼ぶ。
    pub(crate) fn step(
        &mut self,
        input: &StepInput,
        hover_time: &mut dyn FnMut() -> Duration,
    ) -> Vec<Effect> {
        let mut fx = Vec::new();
        let moved = input.cursor.is_some() && input.cursor != self.last_cursor;
        // ① 追っている範囲を照らす（終われば何もない・押したので休み、へ）。
        self.state = match std::mem::take(&mut self.state) {
            State::Waiting {
                window,
                range,
                deadline,
            } => self.check_waiting(input, moved, window, range, deadline, &mut fx),
            State::Active(turn) => self.check_active(input, moved, turn, &mut fx),
            other => other,
        };
        // ② 押して休んでいる範囲から出たら、休みを解く（位置が分からない回は出たとみなさない）。
        if let State::Suppressed { range } = self.state
            && input.cursor.is_some()
            && input.over.map(|o| o.range) != Some(range)
        {
            trace!(range = ?range, "[tooltip_turn] 押した範囲から出た");
            self.state = State::Idle;
        }
        // ③ 何もなければ足元の範囲に入る（終わったその回のうちに別の範囲の待ちまで進める）。
        if let (State::Idle, Some(over)) = (&self.state, input.over) {
            self.state = if input.button_down {
                trace!(range = ?over.range, "[tooltip_turn] 押したまま入った（数えない）");
                State::Suppressed { range: over.range }
            } else {
                let deadline = input.now + self.wait_for(over.range, input.now, hover_time());
                fx.push(Effect::ArmDeadline(deadline));
                State::Waiting {
                    window: over.window,
                    range: over.range,
                    deadline,
                }
            };
        }
        if input.cursor.is_some() {
            self.last_cursor = input.cursor;
        }
        fx
    }

    /// 待ち時間＝設定の 2 倍。直前のツールチップが消えてから 0.2 秒以内なら 1 倍。
    fn wait_for(&self, range: TooltipRangeId, now: Instant, setting: Duration) -> Duration {
        let reshow = self
            .last_hidden
            .is_some_and(|t| now.saturating_duration_since(t) <= RESHOW);
        let wait = if reshow { setting } else { setting * 2 };
        trace!(
            range = ?range,
            wait_ms = wait.as_millis() as u64,
            reshow, "[tooltip_turn] 範囲に入った・数え始め"
        );
        wait
    }

    fn check_waiting(
        &mut self,
        input: &StepInput,
        moved: bool,
        window: Entity,
        range: TooltipRangeId,
        deadline: Instant,
        fx: &mut Vec<Effect>,
    ) -> State {
        let reason = lost(input).or_else(|| match (input.cursor, input.over) {
            // 位置が分からない回は待ちを進めない。
            (None, _) => None,
            (Some(_), Some(o)) if o.range == range => None,
            (Some(_), _) => Some(left(input, moved)),
        });
        if let Some(reason) = reason {
            // 待ちから外れるだけなら知らせは出さない。
            trace!(range = ?range, reason = ?reason, "[tooltip_turn] 来なかった");
            return match reason {
                TooltipEndReason::ButtonPressed => State::Suppressed { range },
                _ => State::Idle,
            };
        }
        let over = input.over.filter(|_| input.cursor.is_some());
        let Some(over) = over.filter(|_| input.now >= deadline) else {
            fx.push(Effect::ArmDeadline(deadline));
            return State::Waiting {
                window,
                range,
                deadline,
            };
        };
        self.last_token += 1;
        let token = TooltipTurnToken(self.last_token);
        trace!(range = ?range, token = token.0, stored = over.has_text, "[tooltip_turn] 来た");
        fx.push(Effect::TurnStarted {
            token,
            window,
            range,
            pos_logical: over.pos_logical,
            stored: over.has_text,
        });
        fx.push(Effect::ArmDeadline(input.now + PATROL));
        State::Active(Turn {
            token,
            window,
            range,
            area: over.area_logical,
        })
    }

    fn check_active(
        &mut self,
        input: &StepInput,
        moved: bool,
        turn: Turn,
        fx: &mut Vec<Effect>,
    ) -> State {
        let reason = lost(input).or_else(|| {
            // 位置が分からない回・様子が無い回は終わらせない。
            let p = input.cursor?;
            let Some(Tracked::Present { range_px, receives }) = input.tracked else {
                return None;
            };
            // 足元を ツールチップ → 範囲 → 通り道 → それ以外 の順に見る。
            // （tip を渡さない in_safe_zone は「矩形の中か」と同じ。）
            if self.tip.is_some_and(|tip| in_safe_zone(p, tip, None)) {
                None
            } else if in_safe_zone(p, range_px, None) {
                (!receives).then(|| left(input, moved))
            } else {
                (!in_safe_zone(p, range_px, self.tip)).then_some(TooltipEndReason::LeftSafeZone)
            }
        });
        let Some(reason) = reason else {
            fx.push(Effect::ArmDeadline(input.now + PATROL));
            return State::Active(turn);
        };
        if self.tip.take().is_some() {
            fx.push(Effect::Hide {
                token: turn.token,
                reason: HideReason::End(reason),
            });
            self.last_hidden = Some(input.now);
        }
        trace!(range = ?turn.range, token = turn.token.0, reason = ?reason, "[tooltip_turn] 終わり");
        fx.push(Effect::TurnEnded {
            token: turn.token,
            window: turn.window,
            range: turn.range,
            reason,
        });
        match reason {
            TooltipEndReason::ButtonPressed => State::Suppressed { range: turn.range },
            _ => State::Idle,
        }
    }

    /// 今追っている範囲（待ち・出す番のとき）。殻が Tracked を作るために読む。
    pub(crate) fn tracked_range(&self) -> Option<(Entity, TooltipRangeId)> {
        match &self.state {
            State::Waiting { window, range, .. } => Some((*window, *range)),
            State::Active(t) => Some((t.window, t.range)),
            _ => None,
        }
    }
}

/// 位置に依らず、追っている範囲を続けられない理由（消えた・隠れた・押した）。
fn lost(input: &StepInput) -> Option<TooltipEndReason> {
    match input.tracked {
        Some(Tracked::RangeUnregistered) => Some(TooltipEndReason::RangeUnregistered),
        Some(Tracked::WindowDestroyed) => Some(TooltipEndReason::WindowDestroyed),
        Some(Tracked::WindowHidden) => Some(TooltipEndReason::WindowHidden),
        _ => input.button_down.then_some(TooltipEndReason::ButtonPressed),
    }
}

/// 範囲の外へ外れた理由。範囲の矩形の中で窓が受けなくなったとき、動いていなければ
/// 「窓が隠れた」（絵や当たり判定が消えた）、動いていれば「安全地帯から出た」（透過の穴や
/// 上に重なった別の窓へ移った）。
fn left(input: &StepInput, moved: bool) -> TooltipEndReason {
    match input.tracked {
        Some(Tracked::Present {
            receives: false, ..
        }) if !moved => TooltipEndReason::WindowHidden,
        _ => TooltipEndReason::LeftSafeZone,
    }
}

#[cfg(test)]
#[path = "turn_tests.rs"]
mod turn_tests;
