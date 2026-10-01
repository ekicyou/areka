//! シェル・バルーンの切替の入口（spec: areka-P0-shell-balloon-switch 要件 1.1〜1.16・
//! design「ShellBalloonSwitch」）。
//!
//! 今は種別 [`SkinKind`]・台本の受け口（`switch_cue.rs` の `SwitchCueSink`）が talk スレッドから
//! 送る生の要求 [`SkinRequestRaw`]・入口の要求 [`SkinRequest`] と判定 [`SkinVerdict`] の型を置く。
//! 名前の解決とインストールの控えは隣の `shell_balloon_resolve.rs`。入口と取り出しの系はこのモジュールに
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

/// 切替先の指し方。台本は名前（`descript.txt` の `name` → フォルダ名の順）、メニューはフォルダ名で指す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SkinSpec {
    Name(String),
    Folder(String),
}

/// 要求の出どころ（`OnShellChanging` を送るかを決める＝台本の `raise-event` とメニューのシェル）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkinOrigin {
    Script { raise_event: bool },
    Menu,
}

/// 入口へ渡す切替要求（要件 1.1: 台本もメニューもこの 1 本を通る）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkinRequest {
    pub kind: SkinKind,
    pub target: SkinSpec,
    pub origin: SkinOrigin,
}

/// 入口の判定（受理以外はどれも `warn!` を 1 件残して何もしない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkinVerdict {
    Accepted,
    /// シェル・バルーンの切替が進行中（重ねない・要件 1.12）。
    Busy,
    /// ゴースト切替が進行中（要件 1.12）。
    GhostSwitching,
    /// ネットワーク更新の実行中（要件 1.13）。
    Updating,
    /// 切替先が決まらない（要件 1.7）。
    NotFound,
    /// 起動の文脈・置き場のゴースト・結線・seriko の送り手のどれかが無い。
    NoContext,
}
