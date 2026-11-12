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
/// 利用者は本関数の戻り値を `DolaRuntime::tick(time)` に渡す。
pub fn now() -> f64 {
    use std::sync::OnceLock;
    use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};

    // 周波数は起動後に変わらないので 1 度だけ読む。
    static FREQ: OnceLock<f64> = OnceLock::new();
    // SAFETY(panic 経路): QueryPerformanceCounter / QueryPerformanceFrequency は
    // Windows XP 以降のシステムでは失敗せず（Microsoft docs）、frequency が 0 に
    // なることもないため、Result は意図的に破棄している。仮に frequency = 0 でも
    // f64 除算は panic せず inf を返すのみ（0/0 でも NaN であり panic しない）。
    // 単調性は QPC のハードウェア保証に依存する（巻き戻りなし）。
    let frequency = *FREQ.get_or_init(|| {
        let mut frequency: i64 = 0;
        unsafe {
            let _ = QueryPerformanceFrequency(&mut frequency);
        }
        frequency as f64
    });
    let mut counter: i64 = 0;
    unsafe {
        let _ = QueryPerformanceCounter(&mut counter);
    }
    (counter as f64) / frequency
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
}
