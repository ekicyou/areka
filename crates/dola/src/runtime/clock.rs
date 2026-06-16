//! # Clock — 高精度時刻取得ユーティリティ
//!
//! OS 起動時からの経過秒数をマイクロ秒級精度で取得する。
//! `QueryPerformanceCounter` / `QueryPerformanceFrequency` ベース。

/// OS 起動時からの現在時刻（f64 秒）を高精度で取得する。
///
/// `QueryPerformanceCounter` の戻り値（カウント）を
/// `QueryPerformanceFrequency` で除算して秒に変換。
/// ハードウェアレベルの高精度タイマーを使用し、マイクロ秒級の精度を提供。
///
/// アニメーションエンジンのフレーム間時間差計測に使用する想定。
/// 利用者は本関数の戻り値を `runtime.update(subscriber_id, time)` に渡す。
pub fn now() -> f64 {
    use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};

    let mut counter: i64 = 0;
    let mut frequency: i64 = 0;
    // SAFETY(panic 経路): QueryPerformanceCounter / QueryPerformanceFrequency は
    // Windows XP 以降のシステムでは失敗せず（Microsoft docs）、frequency が 0 に
    // なることもないため、Result は意図的に破棄している。仮に frequency = 0 でも
    // f64 除算は panic せず inf を返すのみ（0/0 でも NaN であり panic しない）。
    // 単調性は QPC のハードウェア保証に依存する（巻き戻りなし）。
    unsafe {
        let _ = QueryPerformanceCounter(&mut counter);
        let _ = QueryPerformanceFrequency(&mut frequency);
    }
    (counter as f64) / (frequency as f64)
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;
    use std::time::Duration;

    /// 単調増加テスト: 連続2回の now() 呼び出しで t2 >= t1 (Req 6.1)
    #[test]
    fn now_is_monotonically_increasing() {
        let t1 = now();
        let t2 = now();
        assert!(
            t2 >= t1,
            "now() must be monotonically increasing: t1={t1}, t2={t2}"
        );
    }

    /// 正の有限値テスト: 戻り値が 0.0 より大きく有限であること (Req 6.2)
    #[test]
    fn now_returns_positive_finite_value() {
        let value = now();
        assert!(value > 0.0, "now() must return a positive value: {value}");
        assert!(
            value.is_finite(),
            "now() must return a finite value: {value}"
        );
    }

    /// ms 精度テスト: 1ms sleep 後の差分が 0 より大きいこと (Req 6.3)
    #[test]
    fn now_has_millisecond_precision() {
        let t1 = now();
        std::thread::sleep(Duration::from_millis(1));
        let t2 = now();
        let diff = t2 - t1;
        assert!(diff > 0.0, "now() must detect 1ms difference: diff={diff}");
    }

    /// ベンチマーク: now() 1回の呼び出しコストがナノ秒オーダーであることを検証 (Task 4)
    ///
    /// 10,000回呼び出しの平均時間を計測し、60FPS フレーム間隔 (16.67ms) に対して
    /// 無視できるオーバーヘッドであることを確認する。
    #[test]
    fn now_call_overhead_is_negligible() {
        use std::time::Instant;

        const ITERATIONS: u32 = 10_000;
        const WARMUP: u32 = 1_000;

        // ウォームアップ
        for _ in 0..WARMUP {
            let _ = now();
        }

        // 計測
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            let _ = std::hint::black_box(now());
        }
        let elapsed = start.elapsed();

        let avg_ns = elapsed.as_nanos() as f64 / ITERATIONS as f64;
        let current_value = now();

        // 結果を表示（cargo test -- --nocapture で確認可能）
        eprintln!("=== clock::now() ベンチマーク結果 ===");
        eprintln!("  反復回数: {ITERATIONS}");
        eprintln!("  合計時間: {elapsed:?}");
        eprintln!("  平均呼出時間: {avg_ns:.1} ns/call");
        eprintln!(
            "  現在の now() 値: {current_value:.6} 秒 (OS起動後 {:.1} 時間)",
            current_value / 3600.0
        );
        eprintln!(
            "  60FPS フレーム間隔比: {:.6}%",
            (avg_ns / 16_670_000.0) * 100.0
        );

        // 1回の now() が 1μs (1000ns) 未満であること（余裕を持った閾値）
        assert!(
            avg_ns < 1_000.0,
            "now() average call time {avg_ns:.1}ns exceeds 1μs threshold"
        );

        // 60FPS フレーム間隔 (16.67ms) の 0.01% 未満であること
        let frame_ratio = avg_ns / 16_670_000.0;
        assert!(
            frame_ratio < 0.0001,
            "now() overhead {frame_ratio:.6} exceeds 0.01% of 60fps frame interval"
        );
    }
}
