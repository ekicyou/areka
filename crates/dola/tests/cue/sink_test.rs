//! `CueSink`（演者非依存の単一出力契約）と `cue_target_of`（relevance 単一権威）の檻。
//!
//! Task 4.1: 旧 `SurfaceSink`/`TextSink` の 2 トレイト分割を統合した単一 `CueSink` を
//! dola へ定義し、relevance 分類器 `cue_target_of` も dola が単一権威として所有することを
//! 公開 API 越しに固定する（R2.4・R11.3・D4/D7）。

use dola::DynamicValue;
use dola::cue::{
    ActorKey, CueCommand, CueFinding, CueProbe, CueSink, CueTarget, TalkCue, cue_target_of,
};
use std::sync::{Arc, Mutex};

/// 全 `CueCommand` variant を単一権威 `cue_target_of` に通し、写像表どおりの配送先へ
/// 分類されることを固定する（R11.3・sakura contract.rs から dola へ移設した網羅檻）。
///
/// `cue_target_of` の match は catch-all を置かないため、dola が将来 variant を追加すると
/// コンパイラが本関数の網羅性を強制的に再検討させる（分類漏れを型で塞ぐ）。
#[test]
fn cue_target_of_classifies_every_variant() {
    // Shell 系（サーフェス消費・seriko 行き）。
    assert_eq!(
        cue_target_of(&CueCommand::Emote {
            key: "smile".into()
        }),
        Some(CueTarget::Shell)
    );
    assert_eq!(
        cue_target_of(&CueCommand::EntityRef(42)),
        Some(CueTarget::Shell)
    );
    // バルーン面切替＝サーフェス消費系ゆえ表示系（Shell）へ。文字状態機械（Balloon）へ
    // 流すのは誤配線・配送不能 None には落ちない。
    assert_eq!(
        cue_target_of(&CueCommand::BalloonSurface { key: "2".into() }),
        Some(CueTarget::Shell)
    );

    // Balloon 系（テキスト表現者・emo-text 行き）。
    assert_eq!(
        cue_target_of(&CueCommand::Text("hello".into())),
        Some(CueTarget::Balloon)
    );
    assert_eq!(
        cue_target_of(&CueCommand::NewLine { ratio: 1.0 }),
        Some(CueTarget::Balloon)
    );
    assert_eq!(cue_target_of(&CueCommand::Clear), Some(CueTarget::Balloon));
    // 全スコープ消去もテキスト表現者の担当（対象スコープのみの Clear と同分類）。
    assert_eq!(
        cue_target_of(&CueCommand::ClearAll),
        Some(CueTarget::Balloon)
    );
    assert_eq!(
        cue_target_of(&CueCommand::Choice {
            id: "yes".into(),
            text: "はい".into(),
            references: vec![]
        }),
        Some(CueTarget::Balloon)
    );
    // アンカーの開き・閉じ（`\_a`）は文字の層（emo-text）が範囲として消費する。
    assert_eq!(
        cue_target_of(&CueCommand::AnchorBegin {
            id: "OnJump".into(),
            references: vec!["r0".into()],
        }),
        Some(CueTarget::Balloon)
    );
    assert_eq!(
        cue_target_of(&CueCommand::AnchorEnd),
        Some(CueTarget::Balloon)
    );

    // 分類不能（Custom）— どの演者も action しない。
    assert_eq!(
        cue_target_of(&CueCommand::Custom {
            command: "fade".into(),
            params: DynamicValue::Null,
        }),
        None
    );

    // カーソル位置指定（`\_l`）はバルーン系表現者（emo-text）が消費する。
    assert_eq!(
        cue_target_of(&CueCommand::Cursor {
            x: "5em".into(),
            y: "2lh".into(),
        }),
        Some(CueTarget::Balloon)
    );

    // Wait は action を持たない＝担当する演者がいない（全員が duration のみ honor）。
    assert_eq!(cue_target_of(&CueCommand::Wait), None);
}

/// 知らせの口の檻（mcp-strict-errors 1.1・要件 2.1／3.9／4.2）。
const FINDING: CueFinding = CueFinding {
    point: "seriko.surface",
    kind: "missing_surface",
    message: "サーフェスが無い",
};

/// `emit` だけを持つ小さな受け手（`emit_with` は既定のまま）。
struct Recorder(Vec<TalkCue>);

impl CueSink for Recorder {
    fn emit(&mut self, cue: TalkCue) {
        self.0.push(cue);
    }
}

/// `emit_with` を上書きしない受け手は、知らせの口を添えて渡されても `emit` をちょうど
/// 1 回だけ同じ合図で呼ぶ（今の送り出しと同じ・知らせは来ない・要件 3.9）。
#[test]
fn default_emit_with_emits_once_and_reports_nothing() {
    let reported = Arc::new(Mutex::new(Vec::<CueFinding>::new()));
    let sink_reported = reported.clone();
    let probe = CueProbe::new(move |f| sink_reported.lock().unwrap().push(f));

    let mut sink = Recorder(Vec::new());
    let cue = TalkCue {
        at: 0.5,
        actor: ActorKey::from("0"),
        command: CueCommand::Text("hello".into()),
        duration: 1.0,
    };
    sink.emit_with(cue.clone(), &probe);

    assert_eq!(sink.0, vec![cue], "emit と同じ 1 回の送り出し");
    assert!(reported.lock().unwrap().is_empty(), "既定は知らせない");
}

/// 口は知らせを登録した関数へそのまま渡し、複製しても同じ口として等しい。別に作った口は
/// 同じ関数でも等しくない。複製はスレッドをまたいで知らせられる（要件 2.1／4.2）。
#[test]
fn probe_reports_and_equality_is_identity() {
    let reported = Arc::new(Mutex::new(Vec::<CueFinding>::new()));
    let r = reported.clone();
    let probe = CueProbe::new(move |f| r.lock().unwrap().push(f));
    let copy = probe.clone();
    assert_eq!(probe, copy, "複製は同じ口");
    assert_ne!(probe, CueProbe::new(|_| {}), "別に作った口は等しくない");

    std::thread::spawn(move || copy.report(FINDING))
        .join()
        .unwrap();
    probe.report(FINDING);
    assert_eq!(*reported.lock().unwrap(), vec![FINDING, FINDING]);
}
