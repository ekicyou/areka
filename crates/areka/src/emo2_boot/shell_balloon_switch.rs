//! シェル・バルーンの切替の入口（spec: areka-P0-shell-balloon-switch 要件 1.1〜1.16・
//! design「ShellBalloonSwitch」）。
//!
//! 今は種別 [`SkinKind`] と、台本の受け口（`switch_cue.rs` の `SwitchCueSink`）が talk スレッドから
//! 送る生の要求 [`SkinRequestRaw`] の型だけを置く。入口・名前の解決・取り出しの系はこのモジュールに
//! 後から足す（受信端を World へ据えるまでは、受け口の送出は送り失敗の `warn!` になり台本は続く）。
#![allow(dead_code)] // 受信端と取り出しの系（8.3）が結線するまで、要求の欄は読まれない

/// 切替の種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkinKind {
    /// シェル（今のゴーストの `shell/` の下）。
    Shell,
    /// バルーン（ベースウェアの根の `balloon/` の下）。
    Balloon,
}

/// 台本の `\![change,shell,名(,--option=raise-event)]`・`\![change,balloon,名]` から届く切替要求
/// （受け口 `SwitchCueSink` が talk スレッドから送る）。名前は台本の字面のまま（無変形）。
///
/// `raise_event` はシェルでだけ真になりうる（バルーンに「切り替え前」のイベントは無い・要件 1.4）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkinRequestRaw {
    pub kind: SkinKind,
    pub name: String,
    pub raise_event: bool,
}
