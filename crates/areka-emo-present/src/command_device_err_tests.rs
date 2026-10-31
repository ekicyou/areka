//! [`device_err`] の「前に記録した原因」: 同じ文脈・同じ HRESULT なら `debug!`、それ以外は今と同じ
//! `error!`（要件 1.6・4.3）。文言・欄・target は水準によらず同じ。

use super::*;

use log_capture_kit::capture;

const E_FAIL: i32 = 0x8000_4005u32 as i32;

/// `device_err(context, repeat)` に E_FAIL を渡し、出た行（ちょうど 1 行）を返す。
fn log_of(
    context: &'static str,
    repeat: Option<(&'static str, i32)>,
) -> log_capture_kit::CapturedEvent {
    let e = windows::core::Error::from_hresult(windows::core::HRESULT(E_FAIL));
    let (err, mut events) = capture(|| device_err(context, repeat)(e));
    assert!(matches!(
        err,
        PresentError::Device { hresult: E_FAIL, context: c } if c == context
    ));
    assert_eq!(events.len(), 1, "失敗 1 回につき行はちょうど 1 行");
    let line = events.remove(0);
    assert_eq!(line.target, "areka_emo_present::command");
    assert_eq!(line.message(), "D3D/DXGI 呼び出しが失敗");
    assert_eq!(line.field("hresult"), Some(E_FAIL.to_string().as_str()));
    assert_eq!(line.field_str("context"), Some(context));
    line
}

#[test]
fn same_context_and_hresult_logs_debug() {
    assert_eq!(
        log_of("EndDraw", Some(("EndDraw", E_FAIL))).level,
        tracing::Level::DEBUG
    );
}

#[test]
fn no_previous_cause_or_a_different_one_logs_error() {
    assert_eq!(log_of("EndDraw", None).level, tracing::Level::ERROR);
    assert_eq!(
        log_of("EndDraw", Some(("Close", E_FAIL))).level,
        tracing::Level::ERROR
    );
    assert_eq!(
        log_of("EndDraw", Some(("EndDraw", 0))).level,
        tracing::Level::ERROR
    );
}
