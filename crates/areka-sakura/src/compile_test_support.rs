use super::*;
use crate::sysvar::SystemVarSnapshot;

/// task 4.1 で `compile` 署名が `(instructions, vars)` へ変わった。既存檻は sysvar を
/// 観測しない（値源展開＝SystemVar アームは task 4.2 の領分）ため、空スナップショットを
/// 渡すテスト用の薄いブリッジで機械的に追随する。`use super::*` の glob import を
/// 明示定義が shadow するため、既存の `compile(&[...])` 呼び出しは無改変のまま本 1 引数
/// ヘルパへ解決される。実スナップショットを渡す新檻は `super::compile(.., &snap)` を直呼びする。
pub(super) fn compile(instructions: &[Instruction]) -> CompiledTalk {
    super::compile(instructions, &SystemVarSnapshot::default())
}

/// `Cue::payload` から `CueCommand` を取り出すヘルパ（`Cue` は PartialEq 非導出）。
pub(super) fn command_of(cue: &Cue) -> &CueCommand {
    match &cue.payload {
        CuePayload::Command(cmd) => cmd,
        other => panic!("expected CuePayload::Command, got {other:?}"),
    }
}

/// `Cue` 単位のフィールド等価（`Cue` は PartialEq 非導出のためフィールド比較）。
/// `start_time`・`duration` は決定性の観測ゆえビット同一（`==`）を要求する（compile が
/// テキスト cue へ焼き込む再生時間 D の回帰を素通しさせない・task 5.1 申し送り）。
/// `actor`（PartialEq）と `payload`（CuePayload/CueCommand は PartialEq）は等価比較。
pub(super) fn cue_eq(a: &Cue, b: &Cue) -> bool {
    a.actor == b.actor
        && a.start_time == b.start_time
        && a.payload == b.payload
        && a.duration == b.duration
}

/// 内容 cue を持つ台本は先頭へ単一 `ClearAll`（`start_time=0.0`・`duration=0.0`）を前置する
/// （#6・R6.1/6.2）。その前置を検証しつつ、後続の**内容 cue**のスライスを返すヘルパ
/// （各テストの ClearAll 前置検証を集約し、内容側の index を従来どおり保つ）。
pub(super) fn assert_clear_all_prefix_and_rest(cues: &[Cue]) -> &[Cue] {
    assert!(
        !cues.is_empty(),
        "内容 cue があるなら先頭に ClearAll が前置される"
    );
    assert_eq!(
        command_of(&cues[0]),
        &CueCommand::ClearAll,
        "先頭 cue は ClearAll"
    );
    assert_eq!(cues[0].start_time, 0.0, "ClearAll の start_time は 0.0");
    assert_eq!(cues[0].duration, 0.0, "ClearAll の duration は 0.0");
    // ClearAll は単一前置（内容側に重複しない・スコープ数に依らず 1 件）。barrier/routing の
    // 非 Command cue（choice 台本の末尾 barrier 等）は ClearAll ではあり得ないため走査から除く。
    assert!(
        cues[1..]
            .iter()
            .all(|c| !matches!(&c.payload, CuePayload::Command(CueCommand::ClearAll))),
        "ClearAll は単一前置（内容 cue 側に重複しない）"
    );
    &cues[1..]
}
