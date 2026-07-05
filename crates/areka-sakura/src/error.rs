//! sakura エンジンの失敗種別を表すエラー型。
//!
//! areka の「ログ無し失敗経路の禁止」規律（R11.4）に沿い、各種別は原因を
//! 表示メッセージへ埋め込む。回復可能失敗は上位の `run_inbox` handler が
//! `Err(SakuraError)` を受けてログ＋ループ継続へ回す（設計 Error Handling）。

/// sakura エンジンの回復可能な失敗種別。
///
/// いずれの種別も原因（オフェンダー値・下流の失敗理由）を表示メッセージへ含め、
/// ログ無し失敗経路を作らない（R11.4）。
#[derive(Debug, thiserror::Error)]
pub enum SakuraError {
    /// 非有限（`NaN`/`±inf`）な時刻前進 `Tick` を無視した回復可能失敗。
    ///
    /// dola の NaN 全量配信ハザードを遮断するためのガードで、遮断した値を保持する（R11.1）。
    #[error("non-finite tick ignored: {0}")]
    NonFiniteTick(f64),
    /// 下流 sink（seriko／emo text-layer への送出アダプタ）への送出失敗。
    ///
    /// 受信端 drop 等の理由文字列を保持して原因を握り潰さない（R11.1）。
    #[error("downstream sink send failed: {0}")]
    SinkSend(String),
    // 拡張シーム（M-dialogue 以降の失敗種別を追加可能）
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_finite_tick_display_contains_offending_value_and_phrase() {
        let err = SakuraError::NonFiniteTick(f64::NAN);
        let msg = err.to_string();
        // 非有限であることを示す語句（R11.1: NaN 全量配信ハザード遮断の文脈）
        assert!(
            msg.contains("non-finite tick"),
            "message should indicate a non-finite tick: {msg}"
        );
        // 原因となった値の描画（f64::NAN の Display は "NaN"）
        assert!(
            msg.contains("NaN"),
            "message should render the offending non-finite value: {msg}"
        );
    }

    #[test]
    fn non_finite_tick_display_contains_infinity_value() {
        let err = SakuraError::NonFiniteTick(f64::INFINITY);
        let msg = err.to_string();
        assert!(
            msg.contains("inf"),
            "message should render the offending infinite value: {msg}"
        );
    }

    #[test]
    fn sink_send_display_preserves_underlying_cause() {
        let err = SakuraError::SinkSend("channel disconnected".into());
        let msg = err.to_string();
        // 下流送出失敗であることを示す語句（R11.1）
        assert!(
            msg.contains("downstream sink send failed"),
            "message should indicate a downstream sink send failure: {msg}"
        );
        // 原因文字列が握り潰されていないこと（R11.4: ログ無し失敗経路禁止）
        assert!(
            msg.contains("channel disconnected"),
            "message should preserve the underlying cause string: {msg}"
        );
    }
}
