//! `transition_judge` のテーマ別テストが共有する**観測行の組立**（テスト専用）。
//!
//! 判定器のテストは「行の字面」を入力に取るので、各テーマが自前で行を組むと
//! フィールドの並びが少しずつ食い違い、片方だけが語彙の変更に追随しなくなる。
//! 組立をここ 1 箇所に集約し、フィールド名は**すべて発行側の `pub const` を参照**する
//! （リテラルを書かない＝判定語の二重定義をしない）。
//!
//! 各ヘルパは当該種別の必須フィールド（`*_FIELDS`）を漏れなく載せる。載せ漏れは
//! [`super::parse_transition_line`] が [`super::RecordDefect::MissingField`] を立てるので、
//! `transition_judge_tests::builders_produce_well_formed_records` が赤にする。

use areka_emo_present::presenter::{
    KIND_SURFACE, SURFACE_FIELD_H, SURFACE_FIELD_REASON, SURFACE_FIELD_RESIZED,
    SURFACE_FIELD_TARGET_ID, SURFACE_FIELD_W,
};
use wintf::ecs::window::transition_diag::{
    FIELD_AH, FIELD_AW, FIELD_AX, FIELD_AY, FIELD_CALL_US, FIELD_COUNT, FIELD_CX, FIELD_CY,
    FIELD_ENTITY, FIELD_FLAGS, FIELD_FRAME, FIELD_HWND, FIELD_IN_SWP, FIELD_KIND,
    FIELD_MERGED_INTO_SEQ, FIELD_MSG, FIELD_NEW_DPI, FIELD_NEW_WA, FIELD_OK, FIELD_OLD_DPI,
    FIELD_OLD_WA, FIELD_ORIGIN, FIELD_SCOPE, FIELD_SEQ, FIELD_SINCE_FLUSH_US, FIELD_SINCE_TICK_US,
    FIELD_STAGE, FIELD_T_US, FIELD_TOTAL_US, FIELD_WIN_KIND, FIELD_X, FIELD_Y, KIND_ENQUEUE,
    KIND_FLUSH, KIND_MONITOR, KIND_MSG, KIND_WRITE, MISSING, RECORD_PREFIX_TAG,
};

use super::super::diag::WindowKind;
use super::super::transition_diag::{
    FIELD_DECISION, FIELD_DIFF, FIELD_GROUND_Y, FIELD_MOVED, FIELD_REASON, FIELD_ROUTE,
    FIELD_SCOPES, FIELD_SINCE_FRAME, FIELD_SITE, FIELD_TABLE_DPI, FIELD_WA_BOTTOM,
    FIELD_WINDOW_DPI, KIND_CHAIN, KIND_GROUND, KIND_HOLD,
};
use super::{
    TransitionRecord, TransitionSummary, parse_transition_line, parse_transition_log,
    split_transitions, summarize,
};

/// 全レコード共通の接頭語＋本体。
pub fn record(frame: u32, t_us: u64, kind: &str, body: &str) -> String {
    format!(
        "{RECORD_PREFIX_TAG} {FIELD_FRAME}={frame} {FIELD_T_US}={t_us} {FIELD_KIND}={kind} {body}"
    )
}

/// モニタ表更新（`old_wa`／`new_wa` は下端だけ指定できれば足りる）。
pub fn monitor(frame: u32, old_dpi: u32, new_dpi: u32, old_bottom: i32, new_bottom: i32) -> String {
    record(
        frame,
        0,
        KIND_MONITOR,
        &format!(
            "{FIELD_ENTITY}=2v0 {FIELD_OLD_DPI}={old_dpi} {FIELD_NEW_DPI}={new_dpi} \
             {FIELD_OLD_WA}=0,0,2880,{old_bottom} {FIELD_NEW_WA}=0,0,2880,{new_bottom}"
        ),
    )
}

/// 窓書込 1 回。`scope`／`win_kind` は番兵を渡せるよう字面で受け取る。
// 上と同じ理由（引数＝必須フィールドなので束ねない）。
#[allow(clippy::too_many_arguments)]
pub fn write(
    frame: u32,
    t_us: u64,
    stage: &str,
    seq: u32,
    hwnd: &str,
    origin: &str,
    scope: &str,
    win_kind: &str,
    call_us: u64,
) -> String {
    record(
        frame,
        t_us,
        KIND_WRITE,
        &format!(
            "{FIELD_STAGE}={stage} {FIELD_SEQ}={seq} {FIELD_HWND}={hwnd} {FIELD_ORIGIN}={origin} \
             {FIELD_SCOPE}={scope} {FIELD_WIN_KIND}={win_kind} {FIELD_X}=10 {FIELD_Y}=20 \
             {FIELD_CX}=30 {FIELD_CY}=40 {FIELD_FLAGS}=0x14 {FIELD_AX}=10 {FIELD_AY}=20 \
             {FIELD_AW}=30 {FIELD_AH}=40 {FIELD_CALL_US}={call_us} {FIELD_OK}=true"
        ),
    )
}

/// 一括書込の区間端。`total_us` は開始行で番兵になるため字面で受け取る。
pub fn flush(frame: u32, t_us: u64, stage: &str, count: u32, total_us: &str) -> String {
    record(
        frame,
        t_us,
        KIND_FLUSH,
        &format!(
            "{FIELD_STAGE}={stage} {FIELD_COUNT}={count} {FIELD_SINCE_TICK_US}=100 \
             {FIELD_TOTAL_US}={total_us}"
        ),
    )
}

/// ウィンドウメッセージの受理。
pub fn msg(frame: u32, t_us: u64, name: &str, hwnd: &str) -> String {
    record(
        frame,
        t_us,
        KIND_MSG,
        &format!(
            "{FIELD_MSG}={name} {FIELD_HWND}={hwnd} {FIELD_IN_SWP}=true \
             {FIELD_SINCE_FLUSH_US}=1000"
        ),
    )
}

/// 窓書込指令の積み上げ。
pub fn enqueue(
    frame: u32,
    t_us: u64,
    hwnd: &str,
    origin: &str,
    scope: &str,
    win_kind: &str,
) -> String {
    record(
        frame,
        t_us,
        KIND_ENQUEUE,
        &format!(
            "{FIELD_HWND}={hwnd} {FIELD_ORIGIN}={origin} {FIELD_SCOPE}={scope} \
             {FIELD_WIN_KIND}={win_kind} {FIELD_MERGED_INTO_SEQ}={MISSING}"
        ),
    )
}

/// サーフェス更新（`w`／`h`／`resized`／`reason` は字面で受け取る＝番兵を渡せる）。
// 引数はそのまま `surface` レコードの必須フィールドであり、束ねると呼出側で
// どのフィールドを番兵にしたのかが読めなくなる。
#[allow(clippy::too_many_arguments)]
pub fn surface(
    frame: u32,
    t_us: u64,
    stage: &str,
    target_id: u32,
    w: &str,
    h: &str,
    resized: &str,
    reason: &str,
) -> String {
    record(
        frame,
        t_us,
        KIND_SURFACE,
        &format!(
            "{FIELD_STAGE}={stage} {SURFACE_FIELD_TARGET_ID}={target_id} {SURFACE_FIELD_W}={w} \
             {SURFACE_FIELD_H}={h} {SURFACE_FIELD_RESIZED}={resized} \
             {SURFACE_FIELD_REASON}={reason}"
        ),
    )
}

/// 接地点（差は発行側と同じ `ground_y − wa_bottom`）。
pub fn ground(frame: u32, scope: u32, ground_y: i32, wa_bottom: i32, route: &str) -> String {
    let diff = ground_y - wa_bottom;
    record(
        frame,
        0,
        KIND_GROUND,
        &format!(
            "{FIELD_SCOPE}={scope} {FIELD_GROUND_Y}={ground_y} {FIELD_WA_BOTTOM}={wa_bottom} \
             {FIELD_DIFF}={diff} {FIELD_ROUTE}={route}"
        ),
    )
}

/// 整合待ちの判定 1 件。
pub fn hold(frame: u32, scope: u32, win_kind: &str, decision: &str, site: &str) -> String {
    record(
        frame,
        0,
        KIND_HOLD,
        &format!(
            "{FIELD_ENTITY}=6v0 {FIELD_SCOPE}={scope} {FIELD_WIN_KIND}={win_kind} \
             {FIELD_WINDOW_DPI}=192 {FIELD_TABLE_DPI}=96 {FIELD_SINCE_FRAME}={frame} \
             {FIELD_DECISION}={decision} {FIELD_SITE}={site}"
        ),
    )
}

/// 連鎖再解決 1 件。
pub fn chain(frame: u32, stage: &str, scopes: u32, moved: u32) -> String {
    record(
        frame,
        0,
        KIND_CHAIN,
        &format!(
            "{FIELD_STAGE}={stage} {FIELD_SCOPES}={scopes} {FIELD_MOVED}={moved} \
             {FIELD_REASON}={MISSING}"
        ),
    )
}

// ---------------------------------------------------------------------------
// テーマ間で共有する助走（構造規約: ヘルパは 1 件でも本ファイルへ集約する）
// ---------------------------------------------------------------------------

/// キャラ窓の種別語（[`WindowKind::as_str`] が唯一の産出元）。
pub fn char_kind() -> &'static str {
    WindowKind::Char.as_str()
}

/// バルーン窓の種別語。
pub fn balloon_kind() -> &'static str {
    WindowKind::Balloon.as_str()
}

/// 1 行を解析して健全なレコードを得る（欠陥があれば落とす）。
pub fn parse_ok(line: &str) -> TransitionRecord {
    let record = parse_transition_line(line).expect("観測行として解析できるはず");
    assert!(
        record.is_well_formed(),
        "行が語彙の規約を破っている: {:?} / {line}",
        record.defects
    );
    record
}

/// 行の列を解析して遷移 1 本へ切り出し、集計まで通す。
pub fn summarize_lines(lines: &[String]) -> TransitionSummary {
    let records = parse_transition_log(&lines.join("\n"));
    let transitions = split_transitions(&records);
    assert_eq!(transitions.len(), 1, "遷移が 1 本に切り出せるはず");
    summarize(&transitions[0])
}
