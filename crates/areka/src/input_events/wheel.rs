//! 窓ごとのホイールの溜めと、100 ミリ秒の間引きの純関数（mouse-click-wheel-events 要件 6・design「WheelThrottle」）。
//!
//! 状態の置き場所は `MouseWiring.wheel`（鍵＝窓の entity）。結線は `char_watch`（task 3.5）。

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "char_watch（task 3.5）が結線するまで本番の呼び手が居ない"
    )
)]

/// 間引きの幅（要件 6.3 で決まるホイール自身の値。移動の間引きの 100 ms とたまたま同じ）。
pub(super) const WHEEL_MIN_INTERVAL_MS: u64 = 100;

/// 溜まった回転（和と、最後の回転の位置〔縮約後サーフェス px〕と当たり判定）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PendingWheel {
    pub sum: i32,
    pub surface: (i64, i64),
    pub region: Option<String>,
}

/// 窓 1 つの溜め。鍵は窓の entity（`MouseWiring.wheel`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WheelAccum {
    pub scope: u32,
    /// 前に `OnMouseWheel` を送った時刻（注入の時計の ms）。送らなかった（和 0）ときは変えない。
    pub last_sent_ms: Option<u64>,
    pub pending: Option<PendingWheel>,
}

/// 1 回の縦の回転（120＝1 目盛り・上が正）と、その時点の位置と当たり判定。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WheelRotation {
    pub delta: i32,
    pub surface: (i64, i64),
    pub region: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum WheelPlan {
    /// 送る（和・最後の回転の位置と当たり判定）。
    Send(PendingWheel),
    /// 和が 0 になったので送らずに溜めを空にした（要件 6.6）。
    DropZero,
    /// 溜めが残る。`until_ms` に期限が来る（前の送出＋100 ms ちょうど）。
    Wait { until_ms: u64 },
    /// 何も無い。
    Idle,
}

/// 回転を足し（あれば・位置と当たり判定は最後の回転で上書き）、期限が来ていれば吐き出す。
pub(super) fn plan_wheel(
    acc: &mut WheelAccum,
    now_ms: u64,
    rotation: Option<WheelRotation>,
) -> WheelPlan {
    if let Some(r) = rotation {
        let sum = acc
            .pending
            .as_ref()
            .map_or(r.delta, |p| p.sum.saturating_add(r.delta));
        acc.pending = Some(PendingWheel {
            sum,
            surface: r.surface,
            region: r.region,
        });
    }
    let Some(pending) = acc.pending.take() else {
        return WheelPlan::Idle;
    };
    if let Some(last) = acc.last_sent_ms {
        let until_ms = last.saturating_add(WHEEL_MIN_INTERVAL_MS);
        if now_ms < until_ms {
            acc.pending = Some(pending);
            return WheelPlan::Wait { until_ms };
        }
    }
    if pending.sum == 0 {
        return WheelPlan::DropZero;
    }
    acc.last_sent_ms = Some(now_ms);
    WheelPlan::Send(pending)
}

#[cfg(test)]
#[path = "wheel_tests.rs"]
mod tests;
