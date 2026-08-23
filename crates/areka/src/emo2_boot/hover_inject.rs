//! 実機サインオフ用 hover 注入デバッグ導線（design.md「結線層 / HoverInjectConduit」・R8.2/8.4/8.6）。
//!
//! 実ポインタに依存せず、env ゲート駆動で「選択肢表示中」actor のハイライトを周期巡回させる有界導線。
//! `AREKA_CHOICE_HOVER_INJECT` が **未設定/空/不正なら完全 no-op（本番既定）**。`cycle`＝既定周期
//! 700ms・`cycle:<ms>`＝指定周期。text phase の frame clock（`TalkClock::talk_time` と同じ時刻源）で
//! 駆動し、注入時刻の商（time÷period の整数商）で `None→0→1→…→None→…` を巡回する——**実時間 sleep を
//! 使わず**決定論的な ordinal 系列を再現する。各注入は `info!`（`RUST_LOG` grep サインオフ材料・8.2/12.1）。
//!
//! 本導線はライブラリ公開 API（[`TextLayerRuntime::inject_choice_hover`]／[`TextLayerRuntime::choice_active`]／
//! [`TextLayerRuntime::choice_hit_rows`]）の一消費者に過ぎず、emo-text 本体・本番描画経路・決定論資産に
//! 変更を加えない（8.6）。frame.rs の text phase から present_frame の**後**に駆動され
//! （`choice_active`／`choice_hit_rows` が当該フレームを反映する）、env 無効ならば存在が消える。
//!
//! # 純関数分割（決定論単体テスト）
//! - [`parse_hover_inject`]: env 文字列（`Option<&str>`）→ [`HoverInjectConfig`]（env 非依存）。
//! - [`cycle_ordinal`]: (frame_time, period_secs, ordinal_count) → `Option<usize>`（純関数・時刻商で巡回）。
//! - [`run_hover_inject_with`]: config＋actor スナップショット＋注入クロージャ→巡回駆動（runtime 非依存）。
//!
//! [`drive`]（frame.rs 結線口）は env を一度だけ読んで cache した config で [`run_hover_inject`] を呼ぶ。

use std::sync::OnceLock;

use areka_emo_text::actor::TextLayerRuntime;
use areka_sakura::ActorKey;
use tracing::{info, warn};
use wintf::ecs::world::tick_wake;

/// hover 注入導線を駆動する env 変数（`AREKA_` 名前空間規約）。
const ENV_KEY: &str = "AREKA_CHOICE_HOVER_INJECT";

/// `cycle`（周期未指定）時の既定周期ミリ秒（design.md「Batch / Job Contract」）。
const DEFAULT_PERIOD_MS: u64 = 700;

/// env ゲートの解決形（`parse_hover_inject` の戻り値・純粋・env 非依存）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoverInjectConfig {
    /// 未設定/空/不正＝完全無効（本番既定・[`run_hover_inject_with`] は注入を一切呼ばない）。
    Disabled,
    /// 周期巡回モード（`cycle`＝既定 700ms・`cycle:<ms>`＝指定周期）。
    Cycle {
        /// 巡回周期（ミリ秒・正）。
        period_ms: u64,
    },
}

/// env 文字列から [`HoverInjectConfig`] を導出する純関数（env アクセスなし・単体テスト可能）。
///
/// - `None`／空／空白のみ → [`HoverInjectConfig::Disabled`]。
/// - `cycle`（周辺空白トリム後） → [`HoverInjectConfig::Cycle`]（既定周期 [`DEFAULT_PERIOD_MS`]）。
/// - `cycle:<ms>`（`<ms>` は正の `u64`） → [`HoverInjectConfig::Cycle`]（指定周期）。
/// - 上記以外（未知トークン・`cycle:` の非数値/0/負値/溢れ） → `warn!`＋[`HoverInjectConfig::Disabled`]。
pub fn parse_hover_inject(value: Option<&str>) -> HoverInjectConfig {
    // 未設定/空/空白のみは無効（本番既定・完全 no-op）。
    let Some(raw) = value.map(str::trim) else {
        return HoverInjectConfig::Disabled;
    };
    if raw.is_empty() {
        return HoverInjectConfig::Disabled;
    }
    // `cycle`（周期未指定）→ 既定周期。
    if raw == "cycle" {
        return HoverInjectConfig::Cycle {
            period_ms: DEFAULT_PERIOD_MS,
        };
    }
    // `cycle:<ms>`（`<ms>` は正の u64・`u64::from_str` が負号・非数字・溢れを弾く）。
    if let Some(rest) = raw.strip_prefix("cycle:") {
        match rest.trim().parse::<u64>() {
            Ok(period_ms) if period_ms > 0 => {
                return HoverInjectConfig::Cycle { period_ms };
            }
            // 0・負値・非数値・溢れ・欠落は不正値として warn＋無効へ倒す。
            _ => {
                warn!(
                    env = ENV_KEY,
                    value = raw,
                    "AREKA_CHOICE_HOVER_INJECT: 不正な cycle 周期指定（正の整数 ms を要する）→ 無効化"
                );
                return HoverInjectConfig::Disabled;
            }
        }
    }
    // 未知トークンは warn＋無効。
    warn!(
        env = ENV_KEY,
        value = raw,
        "AREKA_CHOICE_HOVER_INJECT: 未知の指定（`cycle` または `cycle:<ms>`）→ 無効化"
    );
    HoverInjectConfig::Disabled
}

/// 巡回 ordinal の純判定（design.md「Batch / Job Contract」・実時間 sleep なし・時刻商で決定論巡回）。
///
/// `frame_time`（talk 起点相対秒・非負想定）を `period_secs` で割った整数商を、`ordinal_count + 1`
/// スロット（スロット0＝`None`・スロット`k`(1..=count)＝`Some(k-1)`）で巡回させる。
/// `ordinal_count == 0`（ヒット行未 population 等）はスロット数 1＝常に `None`。防御的に非正/非有限
/// `period_secs` は `None`（parse 段で弾かれるが純関数を total に保つ）・`frame_time` は 0.0 clamp。
pub fn cycle_ordinal(frame_time: f64, period_secs: f64, ordinal_count: usize) -> Option<usize> {
    // 防御: 非正/非有限周期は注入なし（parse 段で弾かれるが純関数を total に保つ）。
    if !(period_secs > 0.0) || !period_secs.is_finite() {
        return None;
    }
    // 負の frame_time は 0.0 clamp（talk_time は 0.0 clamp 済みだが二重防波堤）。
    let t = frame_time.max(0.0);
    // 時刻商（実時間 sleep なし・注入時刻÷周期の整数商）。非負有限ゆえ u64 へ truncate（飽和）。
    let quotient = (t / period_secs).floor() as u64;
    // スロット数＝ordinal_count + 1（スロット0＝None・スロット k(1..=count)＝Some(k-1)）。
    // ordinal_count == 0 はスロット数 1＝常に None（ヒット行未 population 等の縮退）。
    let slots = ordinal_count as u64 + 1;
    let step = quotient % slots;
    if step == 0 {
        None
    } else {
        Some((step - 1) as usize)
    }
}

/// config＋actor スナップショット（actor・choice_active・hit_row 数）＋注入クロージャで巡回駆動する
/// 純ドライバ（runtime 非依存・呼び出し回数を檻に入れられる）。
///
/// [`HoverInjectConfig::Disabled`] は即 return し **`inject` を一度も呼ばない**（本番既定 no-op・8.6）。
/// [`HoverInjectConfig::Cycle`] は `active` な各 actor について [`cycle_ordinal`] の結果を `inject` へ渡し
/// `info!` する（非 active はスキップ）。
pub fn run_hover_inject_with(
    config: &HoverInjectConfig,
    frame_time: f64,
    actors: &[(ActorKey, bool, usize)],
    mut inject: impl FnMut(&ActorKey, Option<usize>),
) {
    // Disabled は即 return（inject を一度も呼ばない・本番既定 no-op・8.6）。
    let HoverInjectConfig::Cycle { period_ms } = config else {
        return;
    };
    let period_secs = *period_ms as f64 / 1000.0;
    for (actor, active, hit_rows) in actors {
        // 「選択肢表示中」でない actor はスキップ（照会のみ・8.4 の実ポインタ非依存）。
        if !active {
            continue;
        }
        let ordinal = cycle_ordinal(frame_time, period_secs, *hit_rows);
        inject(actor, ordinal);
        // 各注入ステップを info!（RUST_LOG grep でのサインオフ判定材料・8.2/12.1）。
        info!(
            actor = %actor.as_str(),
            frame_time,
            period_ms = *period_ms,
            ordinal = ?ordinal,
            hit_rows = *hit_rows,
            "choice hover inject: 周期巡回による ordinal 注入（実機サインオフ導線・8.2）"
        );
    }
}

/// runtime から actor スナップショットを取り [`run_hover_inject_with`] へ委譲する結線（借用規律）。
///
/// `runtime.state().actors()`（BTreeMap 決定論順）で actor 集合を列挙し、各 actor の
/// `choice_active`／`choice_hit_rows().len()` を不変借用でスナップショットしてから（借用を解いて）
/// `inject_choice_hover` の可変呼び出しへ渡す。
pub fn run_hover_inject(
    config: &HoverInjectConfig,
    runtime: &mut TextLayerRuntime,
    frame_time: f64,
) {
    // Disabled は actor 走査すら行わず即 return（本番既定・完全 no-op・8.6）。
    if matches!(config, HoverInjectConfig::Disabled) {
        return;
    }
    // 不変借用でスナップショット（actor key clone・active・hit_row 数）を取り切ってから可変注入へ移る。
    let snapshot: Vec<(ActorKey, bool, usize)> = runtime
        .state()
        .actors()
        .map(|(actor, _)| {
            let active = runtime.choice_active(actor);
            let hit_rows = runtime.choice_hit_rows(actor).len();
            (actor.clone(), active, hit_rows)
        })
        .collect();
    run_hover_inject_with(config, frame_time, &snapshot, |actor, ordinal| {
        runtime.inject_choice_hover(actor, ordinal);
    });
}

/// env を一度だけ読んで cache する解決済み config（プロセス寿命で不変・OnceLock）。
fn config() -> &'static HoverInjectConfig {
    static CONFIG: OnceLock<HoverInjectConfig> = OnceLock::new();
    CONFIG.get_or_init(|| parse_hover_inject(std::env::var(ENV_KEY).ok().as_deref()))
}

/// frame.rs の text phase（present_frame の後）から駆動する結線口。
///
/// env 未設定/無効なら [`run_hover_inject`] は [`HoverInjectConfig::Disabled`] で完全 no-op
/// （`inject_choice_hover` を一度も呼ばない・本番既定）。`frame_time` は text phase が解決した
/// `talk_time`（`TalkClock::talk_time` と同源）。
pub fn drive(runtime: &mut TextLayerRuntime, frame_time: f64) {
    let config = config();
    // 注入が有効な間は周期巡回を進めるために毎画面更新で回る必要があるので、次の画面更新を
    // 予約する（設計 C16 の `REARM`）。無効（本番既定）なら旗も立てない＝完全 no-op のまま。
    if !matches!(config, HoverInjectConfig::Disabled) {
        tick_wake::mark(tick_wake::REARM);
    }
    run_hover_inject(config, runtime, frame_time);
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::*;

    // ── parse_hover_inject（env 語彙・純粋・env 非依存）─────────────────────────────

    /// 未設定/空/空白のみ → Disabled（本番既定・完全無効）。
    #[test]
    fn parse_unset_or_blank_is_disabled() {
        assert_eq!(
            parse_hover_inject(None),
            HoverInjectConfig::Disabled,
            "未設定は無効"
        );
        assert_eq!(
            parse_hover_inject(Some("")),
            HoverInjectConfig::Disabled,
            "空は無効"
        );
        assert_eq!(
            parse_hover_inject(Some("   ")),
            HoverInjectConfig::Disabled,
            "空白のみは無効"
        );
    }

    /// `cycle`（周辺空白トリム込み）→ 既定周期 700ms の Cycle。
    #[test]
    fn parse_cycle_uses_default_period() {
        assert_eq!(
            parse_hover_inject(Some("cycle")),
            HoverInjectConfig::Cycle {
                period_ms: DEFAULT_PERIOD_MS
            },
            "cycle は既定 700ms"
        );
        assert_eq!(
            parse_hover_inject(Some("  cycle  ")),
            HoverInjectConfig::Cycle {
                period_ms: DEFAULT_PERIOD_MS
            },
            "周辺空白はトリムして cycle"
        );
    }

    /// `cycle:<ms>`（正の u64）→ 指定周期の Cycle。
    #[test]
    fn parse_cycle_with_explicit_period() {
        assert_eq!(
            parse_hover_inject(Some("cycle:500")),
            HoverInjectConfig::Cycle { period_ms: 500 },
            "cycle:500 は 500ms"
        );
        assert_eq!(
            parse_hover_inject(Some("cycle:1200")),
            HoverInjectConfig::Cycle { period_ms: 1200 },
            "cycle:1200 は 1200ms"
        );
    }

    /// 不正値（未知トークン・非数値/0/負値の周期）→ Disabled（warn＋無効）。
    #[test]
    fn parse_invalid_values_are_disabled() {
        assert_eq!(
            parse_hover_inject(Some("bogus")),
            HoverInjectConfig::Disabled,
            "未知トークンは無効"
        );
        assert_eq!(
            parse_hover_inject(Some("cycle:")),
            HoverInjectConfig::Disabled,
            "周期欠落は無効"
        );
        assert_eq!(
            parse_hover_inject(Some("cycle:abc")),
            HoverInjectConfig::Disabled,
            "非数値周期は無効"
        );
        assert_eq!(
            parse_hover_inject(Some("cycle:0")),
            HoverInjectConfig::Disabled,
            "0 周期は無効"
        );
        assert_eq!(
            parse_hover_inject(Some("cycle:-5")),
            HoverInjectConfig::Disabled,
            "負値周期は無効"
        );
    }

    // ── cycle_ordinal（時刻商→ordinal 系列・純粋・決定論）──────────────────────────

    /// 決定論系列（3 ヒット行・周期 1.0s）: None→0→1→2→None→0→…（スロット数＝count+1＝4）。
    #[test]
    fn cycle_ordinal_series_none_then_each_ordinal_then_wrap() {
        let period = 1.0; // 1.0 秒周期（商＝floor(frame_time)）。
        let count = 3;
        // frame_time を 0.0,1.0,2.0,… と進め、各周期スロットの注入 ordinal を確認する。
        let expected: [Option<usize>; 8] = [
            None,    // 商0 → スロット0 = None
            Some(0), // 商1 → スロット1 = ordinal0
            Some(1), // 商2 → スロット2 = ordinal1
            Some(2), // 商3 → スロット3 = ordinal2
            None,    // 商4 → 巡回して None
            Some(0), // 商5
            Some(1), // 商6
            Some(2), // 商7
        ];
        for (q, want) in expected.iter().enumerate() {
            // 周期内の任意点で同一（商が同じ）: 中央 +0.5 でも境界と同じ結果になることも確認。
            let t = q as f64 + 0.5;
            assert_eq!(
                cycle_ordinal(t, period, count),
                *want,
                "frame_time={t} 商={q} の注入 ordinal"
            );
        }
    }

    /// ヒット行 0 件（未 population）→ スロット数 1＝常に None（周期に依らず）。
    #[test]
    fn cycle_ordinal_zero_rows_is_always_none() {
        for q in 0..5u32 {
            assert_eq!(
                cycle_ordinal(q as f64 * 0.7 + 0.35, 0.7, 0),
                None,
                "0 行は常に None"
            );
        }
    }

    /// 負の frame_time は 0.0 clamp（スロット0＝None）・非正/非有限 period は None（防御）。
    #[test]
    fn cycle_ordinal_defensive_clamps_and_guards() {
        assert_eq!(
            cycle_ordinal(-10.0, 1.0, 3),
            None,
            "負 frame_time は 0.0 clamp → 商0 → None"
        );
        assert_eq!(cycle_ordinal(2.0, 0.0, 3), None, "period 0 は防御的に None");
        assert_eq!(
            cycle_ordinal(2.0, -1.0, 3),
            None,
            "負 period は防御的に None"
        );
        assert_eq!(
            cycle_ordinal(2.0, f64::NAN, 3),
            None,
            "非有限 period は防御的に None"
        );
    }

    /// 同一 frame 時刻→同一注入（純関数駆動の冪等・design Idempotency）。
    #[test]
    fn cycle_ordinal_is_deterministic_for_same_time() {
        let (t, period, count) = (3.5, 0.7, 2);
        let a = cycle_ordinal(t, period, count);
        let b = cycle_ordinal(t, period, count);
        assert_eq!(a, b, "同一 (frame_time, period, count) は同一注入");
    }

    // ── run_hover_inject_with（driver・disabled は注入ゼロ／cycle は active のみ）──────

    /// **観測可能な完了条件（8.6）**: Disabled では active な actor があっても `inject` が一度も呼ばれない。
    #[test]
    fn driver_disabled_never_injects() {
        let calls = Cell::new(0usize);
        let actors = vec![
            (ActorKey::from("0"), true, 3), // active・ヒット行あり
            (ActorKey::from("1"), true, 2),
        ];
        run_hover_inject_with(
            &HoverInjectConfig::Disabled,
            1.5,
            &actors,
            |_actor, _ord| {
                calls.set(calls.get() + 1);
            },
        );
        assert_eq!(
            calls.get(),
            0,
            "Disabled は inject を一度も呼ばない（本番既定 no-op・8.6）"
        );
    }

    /// Cycle は `choice_active` な actor のみに注入し、非 active はスキップする。注入 ordinal は
    /// 各 actor の hit_row 数で巡回する（frame_time=商1 相当 → スロット1＝ordinal0）。
    #[test]
    fn driver_cycle_injects_active_actors_only() {
        let recorded: RefCell<Vec<(String, Option<usize>)>> = RefCell::new(Vec::new());
        let actors = vec![
            (ActorKey::from("0"), true, 3),  // active
            (ActorKey::from("1"), false, 2), // 非 active → skip
            (ActorKey::from("2"), true, 1),  // active（1 行）
        ];
        // period 1.0s・frame_time 1.5 → 商1 → スロット1 = ordinal0（両 active actor とも）。
        run_hover_inject_with(
            &HoverInjectConfig::Cycle { period_ms: 1000 },
            1.5,
            &actors,
            |actor, ord| {
                recorded
                    .borrow_mut()
                    .push((actor.as_str().to_string(), ord))
            },
        );
        let got = recorded.into_inner();
        assert_eq!(
            got,
            vec![("0".to_string(), Some(0)), ("2".to_string(), Some(0))],
            "active な 0,2 のみに ordinal0 注入・非 active 1 は skip"
        );
    }

    /// Cycle・商0 相当（frame_time < period）→ 全 active actor へ None 注入（ハイライト無し先頭）。
    #[test]
    fn driver_cycle_injects_none_at_cycle_head() {
        let recorded: RefCell<Vec<Option<usize>>> = RefCell::new(Vec::new());
        let actors = vec![(ActorKey::from("0"), true, 3)];
        // period 700ms・frame_time 0.35s（<0.7）→ 商0 → None。
        run_hover_inject_with(
            &HoverInjectConfig::Cycle { period_ms: 700 },
            0.35,
            &actors,
            |_actor, ord| recorded.borrow_mut().push(ord),
        );
        assert_eq!(
            recorded.into_inner(),
            vec![None],
            "商0 は None 注入（巡回の先頭）"
        );
    }
}
