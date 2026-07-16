//! `CueSink`（演者非依存の単一出力契約）と `cue_target_of`（relevance 単一権威）の檻。
//!
//! Task 4.1: 旧 `SurfaceSink`/`TextSink` の 2 トレイト分割を統合した単一 `CueSink` を
//! dola へ定義し、relevance 分類器 `cue_target_of` も dola が単一権威として所有することを
//! 公開 API 越しに固定する（R2.4・R11.3・D4/D7）。

use dola::DynamicValue;
use dola::cue::{ActorKey, CueCommand, CueSink, CueTarget, TalkCue, cue_target_of};

/// テスト用ダミー演者。**単一の `CueSink` 契約のみ**を実装し、broadcast で受け取った
/// 全 cue のうち **relevance 判定（単一権威 `cue_target_of`）が自分宛て**の cue だけを
/// action 対象として選別する。担当外 cue でも duration は必ず記録する（honor 契約——
/// action を無視しても duration は落とさない・R2.3）。
struct RelevancePresenter {
    /// この演者の担当スロット（自分宛て判定の基準）。
    my_target: CueTarget,
    /// relevance 判定で自分の担当だった cue の command 列（action 対象）。
    acted: Vec<CueCommand>,
    /// 受け取った**全** cue の duration 列（担当外を含めて honor した証跡）。
    honored_durations: Vec<f64>,
}

impl RelevancePresenter {
    fn new(my_target: CueTarget) -> Self {
        Self {
            my_target,
            acted: Vec::new(),
            honored_durations: Vec::new(),
        }
    }
}

impl CueSink for RelevancePresenter {
    fn emit(&mut self, cue: TalkCue) {
        // honor 契約: action 可否に関わらず duration は必ず honor（記録）する。
        self.honored_durations.push(cue.duration);
        // relevance は演者側で判定する（中央 router に依存しない・R2.4）。
        // 判定権威は単一の `cue_target_of`——全演者が同じ分類器を共有する（R11.3）。
        if cue_target_of(&cue.command) == Some(self.my_target.clone()) {
            self.acted.push(cue.command);
        }
    }
}

/// 配送する共通の cue ストリーム（Shell 系・Balloon 系・担当なし Wait を混在させる）。
fn broadcast_stream() -> Vec<TalkCue> {
    let actor = ActorKey::from("0");
    vec![
        // Shell 系（Emote）— duration 占有あり。
        TalkCue {
            at: 0.0,
            actor: actor.clone(),
            command: CueCommand::Emote {
                key: "smile".into(),
            },
            duration: 0.5,
        },
        // Balloon 系（Text）— duration 占有あり。
        TalkCue {
            at: 0.5,
            actor: actor.clone(),
            command: CueCommand::Text("hello".into()),
            duration: 1.0,
        },
        // どの演者の担当でもない純粋な待ち（Wait）— duration のみ。
        TalkCue {
            at: 1.5,
            actor: actor.clone(),
            command: CueCommand::Wait,
            duration: 0.25,
        },
        // Shell 系（BalloonSurface＝サーフェス消費）— 瞬時（明示 0）。
        TalkCue {
            at: 1.75,
            actor: actor.clone(),
            command: CueCommand::BalloonSurface { key: "2".into() },
            duration: 0.0,
        },
        // Balloon 系（NewLine）— 瞬時（明示 0）。
        TalkCue {
            at: 1.75,
            actor: actor.clone(),
            command: CueCommand::NewLine { ratio: 1.0 },
            duration: 0.0,
        },
        // Balloon 系（ClearAll＝全スコープ消去）— 瞬時（明示 0）。
        TalkCue {
            at: 1.75,
            actor,
            command: CueCommand::ClearAll,
            duration: 0.0,
        },
    ]
}

/// **観測可能な完了条件（Task 4.1）**: 単一 `CueSink` を実装したダミー演者が、
/// broadcast された同一ストリームから **relevance 判定で自分の担当 cue のみ**を選別し、
/// 担当外 cue には action しないことを示す。同時に担当外 cue の duration も honor する
/// （action を無視しても duration は落とさない・R2.3/R2.4）。
#[test]
fn dummy_presenter_selects_only_its_relevant_cues_by_relevance() {
    let stream = broadcast_stream();
    // 全 cue の duration（honor 証跡の期待値）。broadcast ゆえ両演者が同一列を受ける。
    let all_durations: Vec<f64> = stream.iter().map(|c| c.duration).collect();

    // Balloon 演者と Shell 演者へ**同一ストリームを broadcast**（両者が全 cue を受ける）。
    let mut balloon = RelevancePresenter::new(CueTarget::Balloon);
    let mut shell = RelevancePresenter::new(CueTarget::Shell);
    for cue in &stream {
        balloon.emit(cue.clone());
        shell.emit(cue.clone());
    }

    // Balloon 演者は Balloon 系（Text/NewLine/ClearAll）だけを action 対象に選ぶ。
    assert_eq!(
        balloon.acted,
        vec![
            CueCommand::Text("hello".into()),
            CueCommand::NewLine { ratio: 1.0 },
            CueCommand::ClearAll,
        ],
        "Balloon 演者は relevance==Balloon の cue のみ action する"
    );
    // Shell 系（Emote/BalloonSurface）へは action しない（担当外）。
    assert!(
        !balloon.acted.iter().any(|c| matches!(
            c,
            CueCommand::Emote { .. } | CueCommand::BalloonSurface { .. }
        )),
        "Balloon 演者は Shell 系 cue へ action しない"
    );

    // Shell 演者は Shell 系（Emote/BalloonSurface）だけを action 対象に選ぶ。
    assert_eq!(
        shell.acted,
        vec![
            CueCommand::Emote {
                key: "smile".into()
            },
            CueCommand::BalloonSurface { key: "2".into() },
        ],
        "Shell 演者は relevance==Shell の cue のみ action する"
    );
    // Balloon 系（Text/NewLine/ClearAll）へは action しない（担当外）。
    assert!(
        !shell.acted.iter().any(|c| matches!(
            c,
            CueCommand::Text(_) | CueCommand::NewLine { .. } | CueCommand::ClearAll
        )),
        "Shell 演者は Balloon 系 cue へ action しない"
    );

    // Wait（relevance==None）はどちらの演者も action しない（担当する演者がいない）。
    assert!(
        !balloon.acted.contains(&CueCommand::Wait) && !shell.acted.contains(&CueCommand::Wait),
        "Wait は担当演者が無く、どの演者も action しない"
    );

    // honor 契約: 両演者とも**全** cue の duration を honor する（担当外でも落とさない）。
    assert_eq!(
        balloon.honored_durations, all_durations,
        "Balloon 演者は担当外 cue の duration も honor する（Wait/Emote/BalloonSurface 含む）"
    );
    assert_eq!(
        shell.honored_durations, all_durations,
        "Shell 演者は担当外 cue の duration も honor する（Wait/Text/NewLine/ClearAll 含む）"
    );
}

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
            text: "はい".into()
        }),
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

    // Wait は action を持たない＝担当する演者がいない（全員が duration のみ honor）。
    assert_eq!(cue_target_of(&CueCommand::Wait), None);
}
