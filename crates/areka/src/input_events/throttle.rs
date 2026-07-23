//! OnMouseMove 送出間引きの純粋・決定的判定（areka-P0-input-events task 2.4・DD-IE-5）。
//!
//! 生のマウス移動イベントごとに OnMouseMove を送出せず、機械的な間引き規則で送出を絞る
//! （要件 5.1）。判定は純粋・決定的で、`now_ms`（注入時刻）と位置・region のみに依存し
//! （要件 5.2）、撫での意味論（連打・滞留の解釈）を送出側で発明しない——その解釈は SHIORI 側
//! の領分（要件 5.3・意味論は touch_detect.lua）。
//!
//! 間引き状態（[`MouseMoveThrottle`]）は値型で per-scope 独立に保持される（実際の per-scope
//! 保持は task 2.6 の `MouseWiring` が `HashMap` で行う。本モジュールは状態型＋純関数のみ）。

/// per-scope の間引き状態（値のみ・純粋更新）。
///
/// `#[allow(dead_code)]`: 状態型＋純関数のみを提供する本 task（2.4）の時点では非テストコードから
/// 未消費。task 2.6 の `MouseWiring`（per-scope `HashMap` 保持）／2.7 のポインタハンドラ結線が
/// 消費する（collision-geometry の `resolve_hit_region` が最初の消費者まで `#[allow(dead_code)]` を
/// 携えたのと同型）。
#[allow(dead_code)]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct MouseMoveThrottle {
    /// 前回観測位置（移動検出・毎観測更新）。
    last_pos: Option<(i64, i64)>,
    /// 前回送出時の region（`None`=未送出）。
    last_sent_region: Option<Option<String>>,
    /// 前回送出時刻（ミリ秒・`None`=未送出）。
    last_sent_ms: Option<u64>,
}

/// 送出上限間隔（10Hz）。touch_detect.lua の 2 秒規律に対し 20 サンプル/2s の余裕。
///
/// `#[allow(dead_code)]`: task 2.6／2.7 が消費するまで非テストコードから未参照。
#[allow(dead_code)]
pub(crate) const MOUSE_MOVE_MIN_INTERVAL_MS: u64 = 100;

/// 純関数: (現状態, 観測) → (次状態, 送出可否)。
///
/// 送出 ⇔ 位置が変化 かつ（初回送出 or region が前回送出時から変化 or 間隔経過）（DD-IE-5）。
///
/// - `moved = state.last_pos != Some(pos)`（前回**観測**から位置が変化・初回観測は `last_pos=None`
///   ゆえ `moved=true`）。
/// - `next.last_pos = Some(pos)` は**常に**更新する（送出可否に関わらず毎観測で位置を記録）。
/// - 位置不変（`!moved`）なら常に送出しない（hover は移動でない・要件 5.1）。`last_sent_*` は不変。
/// - 移動時のみ以下で送出可否を決める:
///   - `first_send = state.last_sent_ms.is_none()`（一度も送出していない）
///   - `region_changed`（前回送出時の region 値と異なる・`None↔Some`・`Some↔別 Some` の双方を数える）
///   - `interval_elapsed`（前回送出から `MOUSE_MOVE_MIN_INTERVAL_MS` 以上経過・未送出は常に真）
///   - `send = first_send || region_changed || interval_elapsed`
/// - 送出時（`send=true`）のみ `last_sent_region`／`last_sent_ms` を今回値へ更新する。
/// - 移動したが抑制（同一 region かつ間隔未経過）のときは `last_sent_*` を持ち越す。
///
/// # Preconditions
/// `now_ms` は単調（呼び手が保証・注入可能）。本関数は I/O もクロック読取もしない純関数。
///
/// # Invariants
/// 判定は位置・region・時刻のみ＝撫で意味論（連打・滞留の解釈）を持たない（要件 5.3）。
///
/// `#[allow(dead_code)]`: task 2.6／2.7 のポインタハンドラが消費するまで非テストコードから未呼出。
#[allow(dead_code)]
pub(crate) fn plan_mouse_move(
    state: &MouseMoveThrottle,
    pos: (i64, i64),
    region: &Option<String>,
    now_ms: u64,
) -> (MouseMoveThrottle, bool) {
    let moved = state.last_pos != Some(pos);

    // 位置不変（hover）: 常に送出しない。位置は毎観測で更新し、送出関連状態は持ち越す。
    if !moved {
        return (
            MouseMoveThrottle {
                last_pos: Some(pos),
                last_sent_region: state.last_sent_region.clone(),
                last_sent_ms: state.last_sent_ms,
            },
            false,
        );
    }

    let first_send = state.last_sent_ms.is_none();
    let region_changed = match &state.last_sent_region {
        None => true,
        Some(prev) => prev != region,
    };
    let interval_elapsed = match state.last_sent_ms {
        Some(t) => now_ms.saturating_sub(t) >= MOUSE_MOVE_MIN_INTERVAL_MS,
        None => true,
    };
    let send = first_send || region_changed || interval_elapsed;

    if send {
        (
            MouseMoveThrottle {
                last_pos: Some(pos),
                last_sent_region: Some(region.clone()),
                last_sent_ms: Some(now_ms),
            },
            true,
        )
    } else {
        // 移動したが抑制（同一 region・間隔未経過）: 位置のみ更新し送出状態は持ち越す。
        (
            MouseMoveThrottle {
                last_pos: Some(pos),
                last_sent_region: state.last_sent_region.clone(),
                last_sent_ms: state.last_sent_ms,
            },
            false,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(s: &str) -> Option<String> {
        Some(s.to_string())
    }

    /// 初回観測（`last_pos=None`）は移動扱いで送出し、状態に pos/region/ms を記録する。
    #[test]
    fn first_observation_sends_and_records_state() {
        let state = MouseMoveThrottle::default();
        let (next, send) = plan_mouse_move(&state, (10, 20), &region("Head"), 1000);

        assert!(send, "初回観測は送出すべき");
        assert_eq!(next.last_pos, Some((10, 20)));
        assert_eq!(next.last_sent_region, Some(region("Head")));
        assert_eq!(next.last_sent_ms, Some(1000));
    }

    /// 位置不変（前回観測と同一 pos）は送出しない（hover 抑制）。間隔が経過していても抑制する。
    #[test]
    fn position_unchanged_suppresses_even_if_interval_elapsed() {
        // 前回送出済み: pos=(10,20)・region=Head・ms=1000。
        let state = MouseMoveThrottle {
            last_pos: Some((10, 20)),
            last_sent_region: Some(region("Head")),
            last_sent_ms: Some(1000),
        };
        // 同一 pos・間隔十分経過（+5000ms）でも位置不変ゆえ送出しない。
        let (next, send) = plan_mouse_move(&state, (10, 20), &region("Head"), 6000);

        assert!(!send, "位置不変（hover）は間隔経過でも送出しない");
        // 位置は更新されるが送出状態（region/ms）は持ち越す。
        assert_eq!(next.last_pos, Some((10, 20)));
        assert_eq!(next.last_sent_region, Some(region("Head")));
        assert_eq!(next.last_sent_ms, Some(1000));
    }

    /// 移動＋region 変化（None→Some）は間隔未経過でも即時送出する。
    #[test]
    fn moved_region_changed_none_to_some_sends_immediately() {
        // 前回送出時 region=None（当たり判定外）・ms=1000。
        let state = MouseMoveThrottle {
            last_pos: Some((10, 20)),
            last_sent_region: Some(None),
            last_sent_ms: Some(1000),
        };
        // 移動し region が Some("Head") へ変化・間隔未経過（+1ms）。
        let (next, send) = plan_mouse_move(&state, (11, 20), &region("Head"), 1001);

        assert!(send, "region 変化（None→Some）は間隔未経過でも送出");
        assert_eq!(next.last_sent_region, Some(region("Head")));
        assert_eq!(next.last_sent_ms, Some(1001));
    }

    /// 移動＋region 変化（Some("A")→Some("B")）は間隔未経過でも即時送出する。
    #[test]
    fn moved_region_changed_some_to_other_some_sends_immediately() {
        let state = MouseMoveThrottle {
            last_pos: Some((10, 20)),
            last_sent_region: Some(region("A")),
            last_sent_ms: Some(1000),
        };
        let (next, send) = plan_mouse_move(&state, (11, 20), &region("B"), 1001);

        assert!(send, "region 変化（Some→別 Some）は間隔未経過でも送出");
        assert_eq!(next.last_sent_region, Some(region("B")));
        assert_eq!(next.last_sent_ms, Some(1001));
    }

    /// 移動＋region 変化（Some→None・当たり判定から外れた）は間隔未経過でも即時送出する。
    #[test]
    fn moved_region_changed_some_to_none_sends_immediately() {
        let state = MouseMoveThrottle {
            last_pos: Some((10, 20)),
            last_sent_region: Some(region("Head")),
            last_sent_ms: Some(1000),
        };
        let none_region: Option<String> = None;
        let (next, send) = plan_mouse_move(&state, (11, 20), &none_region, 1001);

        assert!(send, "region 変化（Some→None）は間隔未経過でも送出");
        assert_eq!(next.last_sent_region, Some(None));
        assert_eq!(next.last_sent_ms, Some(1001));
    }

    /// 移動＋同一 region＋間隔未経過（delta < 100ms）は抑制する。状態は持ち越す（位置のみ更新）。
    #[test]
    fn moved_same_region_within_interval_is_throttled() {
        let state = MouseMoveThrottle {
            last_pos: Some((10, 20)),
            last_sent_region: Some(region("Head")),
            last_sent_ms: Some(1000),
        };
        // 移動・同一 region・間隔未経過（+99ms < 100ms）。
        let (next, send) = plan_mouse_move(&state, (11, 20), &region("Head"), 1099);

        assert!(!send, "同一 region・間隔未経過は抑制");
        // 位置は更新、送出状態（region/ms）は持ち越し。
        assert_eq!(next.last_pos, Some((11, 20)));
        assert_eq!(next.last_sent_region, Some(region("Head")));
        assert_eq!(next.last_sent_ms, Some(1000));
    }

    /// 移動＋同一 region＋間隔経過（delta >= 100ms）は送出する（境界値 100ms を含む）。
    #[test]
    fn moved_same_region_interval_elapsed_sends() {
        let state = MouseMoveThrottle {
            last_pos: Some((10, 20)),
            last_sent_region: Some(region("Head")),
            last_sent_ms: Some(1000),
        };
        // 境界値ちょうど 100ms 経過で送出（>= 判定）。
        let (next, send) = plan_mouse_move(&state, (11, 20), &region("Head"), 1100);

        assert!(send, "間隔経過（境界 100ms）は送出");
        assert_eq!(next.last_pos, Some((11, 20)));
        assert_eq!(next.last_sent_region, Some(region("Head")));
        assert_eq!(next.last_sent_ms, Some(1100));
    }

    /// per-scope 独立: 2 つの状態値は互いに影響せず独立に進化する（HashMap per-scope の縮図）。
    #[test]
    fn per_scope_states_evolve_independently() {
        // scope A: 送出済み（ms=1000・Head）。scope B: 未送出（default）。
        let scope_a = MouseMoveThrottle {
            last_pos: Some((10, 20)),
            last_sent_region: Some(region("Head")),
            last_sent_ms: Some(1000),
        };
        let scope_b = MouseMoveThrottle::default();

        // A は同一 region・間隔未経過で抑制。
        let (next_a, send_a) = plan_mouse_move(&scope_a, (11, 20), &region("Head"), 1050);
        // B は初回観測ゆえ送出。
        let (next_b, send_b) = plan_mouse_move(&scope_b, (11, 20), &region("Head"), 1050);

        assert!(!send_a, "scope A は抑制されるべき");
        assert!(send_b, "scope B は初回送出されるべき");
        // A の抑制は B の送出判定に影響しない（独立）。
        assert_eq!(next_a.last_sent_ms, Some(1000), "A は送出状態を持ち越す");
        assert_eq!(next_b.last_sent_ms, Some(1050), "B は今回送出時刻を記録");
    }

    /// 抑制時の状態持ち越し正確性: 抑制連続時も `last_sent_*` は不変、`last_pos` は毎回更新される。
    #[test]
    fn suppressed_sends_carry_state_and_update_last_pos() {
        let state = MouseMoveThrottle {
            last_pos: Some((10, 20)),
            last_sent_region: Some(region("Head")),
            last_sent_ms: Some(1000),
        };

        // 1 回目の抑制（移動・同一 region・間隔未経過）。
        let (next1, send1) = plan_mouse_move(&state, (11, 20), &region("Head"), 1030);
        assert!(!send1);
        assert_eq!(next1.last_pos, Some((11, 20)), "位置は更新");
        assert_eq!(next1.last_sent_ms, Some(1000), "送出時刻は持ち越し");

        // 2 回目の抑制（さらに移動・依然間隔未経過）。last_sent_* は 1000 のまま持ち越す。
        let (next2, send2) = plan_mouse_move(&next1, (12, 20), &region("Head"), 1060);
        assert!(!send2);
        assert_eq!(next2.last_pos, Some((12, 20)), "位置は再度更新");
        assert_eq!(next2.last_sent_region, Some(region("Head")));
        assert_eq!(next2.last_sent_ms, Some(1000), "送出時刻は 1000 のまま");

        // 3 回目でようやく間隔経過（1000→1100 で 100ms）＝送出。基準時刻が 1000 のままだった証拠。
        let (next3, send3) = plan_mouse_move(&next2, (13, 20), &region("Head"), 1100);
        assert!(send3, "抑制中は基準時刻が動かないので 1100 で間隔経過し送出");
        assert_eq!(next3.last_sent_ms, Some(1100));
    }
}
