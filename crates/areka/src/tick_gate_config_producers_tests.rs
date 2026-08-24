//! areka 側で起床の旗を立てる側（生産者）が現に在るかの字面検査。
//!
//! 判断の中身は見ない——回すか省略するかの正しさは wintf の純関数の全組合せが見ており、
//! ここが守るのは**配線の抜け**だけである。旗を立て忘れると、門は正しく判断したまま
//! 反応しない画面更新を作ってしまう（発話中に表示が固まる形）。
//!
//! wintf 側の同種の検査（`tick_gate_tests.rs` の `WINTF_PRODUCERS`）は別クレートの
//! ファイルを読めないため、areka の生産者はこの表が受け持つ。両者は同じ書き方
//! （`include_str!` ＋註釈行の除去＋期待する旗の名前）で揃えてある。
//!
//! 「立っていないはず」は**主張しない**。旗はプロセスで 1 組しか無く、本番経路が同じ
//! 旗を立てるため、そちらの主張は他の検査の巻き添えで揺れる（tasks.md「(3.4)」）。

/// 旗を立てる側（areka 内）の一覧（見出し・中身・期待する旗の名前）。並びは見出しの辞書順。
///
/// パスは本ファイル（`crates/areka/src/`）からの相対である。
const AREKA_PRODUCERS: [(&str, &str, &str); 6] = [
    (
        "emo2_boot/adapter.rs",
        include_str!("emo2_boot/adapter.rs"),
        "PRESENT",
    ),
    (
        "emo2_boot/balloon_visibility_phase.rs",
        include_str!("emo2_boot/balloon_visibility_phase.rs"),
        "REARM",
    ),
    (
        "emo2_boot/frame/scale_text.rs",
        include_str!("emo2_boot/frame/scale_text.rs"),
        "REARM",
    ),
    (
        "emo2_boot/hover_inject.rs",
        include_str!("emo2_boot/hover_inject.rs"),
        "REARM",
    ),
    (
        "emo2_boot/move_cue.rs",
        include_str!("emo2_boot/move_cue.rs"),
        "PRESENT",
    ),
    (
        "emo2_boot/talk_lifecycle.rs",
        include_str!("emo2_boot/talk_lifecycle.rs"),
        "PRESENT",
    ),
];

/// 註釈の行を落とす——説明文に書いてあるだけの綴りを「在る」と数えないため。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn every_areka_producer_marks_the_tick_wake() {
    for (label, src, flag) in AREKA_PRODUCERS {
        let code = code_only(src);
        assert!(
            code.contains("tick_wake::mark("),
            "{label}: 旗を立てる呼出（tick_wake::mark(）が無い"
        );
        assert!(
            code.contains(&format!("tick_wake::{flag}")),
            "{label}: 期待する旗 {flag} を立てていない"
        );
    }
}

/// バルーンの待ち時間は旗ではなく**期限**で預ける（設計 C16）。
///
/// 期限の枠は最も早い 1 つしか持てず到来で倒れるので、待っている限り相が走るたびに
/// 預け直す必要がある。ここでは預ける呼出が在ることだけを見る（預け直しの回数は
/// 相の側の決定論テストが `visibility_wake` の答えとして固定する）。
#[test]
fn balloon_visibility_phase_arms_the_deadline() {
    let code = code_only(include_str!("emo2_boot/balloon_visibility_phase.rs"));
    assert!(
        code.contains("tick_wake::arm_deadline("),
        "emo2_boot/balloon_visibility_phase.rs: 待ち時間を預ける呼出（tick_wake::arm_deadline(）が無い"
    );
}
