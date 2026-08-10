// ===================== broadcast per-sink relevance partition（task 8.1） =====================
//
// design.md「Testing Strategy → Integration Tests → honor 契約 ③（relevance partition）」・
// Revalidation Trigger「`cue_target_of` を relevance の単一権威とし、各表現者の action 判定が
// `cue_target_of` の分類と一致すること（partition）を再確認する」。
//
// broadcast（D4）では全 cue が登録された全 sink（seriko/emo-text）へ配られる。どの action を
// 演じるかは演者側 relevance（`cue_target_of` が単一権威）が決めるため、**変異 variant ごとに
// action する演者は高々一つ**でなければならない（二重 action / 暗黙ドロップの発散を型で塞ぐ）。
// 本モジュールは `CueCommand` の全 10 variant について、`cue_target_of` の分類が
// 「Shell→seriko だけが action／Balloon→emo-text だけが action／None→誰も action しない」の
// partition になっていることを純関数として固定する（GPU 不要・決定論）。

use areka_sakura::contract::{CueCommand, CueTarget, cue_target_of};

/// broadcast された cue に対して action する演者の同定（分類が単一権威 `cue_target_of`）。
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Performer {
    /// seriko⑤（`cue_target_of == Shell` を action ゲートにする）。
    Seriko,
    /// emo-text⑥（`cue_target_of == Balloon` を action ゲートにする）。
    EmoText,
}

/// `cue_target_of` の分類（`Option<CueTarget>`）から**action する演者**を導く（高々一つ）。
///
/// `Option<CueTarget>` が単一値ゆえ「variant あたり action する演者は高々一つ」は構造的に
/// 保証される——本関数はその分類を演者同定へ写す唯一の権威。`CueTarget` を exhaustive に
/// 網羅し（catch-all なし）、将来 `CueTarget` variant が増えたらコンパイラが再検討を強制する。
fn acting_performer(target: Option<CueTarget>) -> Option<Performer> {
    match target {
        Some(CueTarget::Shell) => Some(Performer::Seriko),
        Some(CueTarget::Balloon) => Some(Performer::EmoText),
        // Window（`\![move]` 系の窓移動先）は seriko/emo-text いずれの担当でもない——
        // 消費は areka bin 側の MoveCueSink（W5）で、この seriko/emo-text partition の
        // 圏外ゆえ None（本テストの CueCommand→cue_target_of 経路では出現しないが、
        // CueTarget の網羅性を型で保つためのアーム）。
        Some(CueTarget::Window) => None,
        None => None, // Wait（純粋な待ち）・Custom（分類不能）＝どの演者も action しない。
    }
}

/// 全 `CueCommand` variant を列挙する（catch-all なし・dola が variant を追加したら
/// コンパイラが本網羅を強制的に再検討させる＝partition の漏れを型で塞ぐ）。
fn every_cue_command() -> Vec<CueCommand> {
    // exhaustive の型檻: variant を 1 つでも落としたらここが不完全になるため、
    // `match` で全 variant を触れてから値を積む（新 variant 追加時にコンパイル停止）。
    let sample = CueCommand::Wait;
    match &sample {
        CueCommand::Text(_)
        | CueCommand::Clear
        | CueCommand::Emote { .. }
        | CueCommand::Choice { .. }
        | CueCommand::EntityRef(_)
        | CueCommand::Custom { .. }
        | CueCommand::NewLine { .. }
        | CueCommand::BalloonSurface { .. }
        | CueCommand::Cursor { .. }
        | CueCommand::Wait
        | CueCommand::ClearAll => {}
    }
    vec![
        CueCommand::Text("hi".into()),
        CueCommand::Clear,
        CueCommand::Emote { key: "0".into() },
        CueCommand::Choice {
            id: "yes".into(),
            text: "はい".into(),
            references: vec![],
        },
        CueCommand::EntityRef(42),
        CueCommand::Custom {
            command: "fade".into(),
            params: dola::DynamicValue::Null,
        },
        CueCommand::NewLine { ratio: 1.0 },
        CueCommand::BalloonSurface { key: "2".into() },
        CueCommand::Cursor {
            x: "5em".into(),
            y: "2lh".into(),
        },
        CueCommand::Wait,
        CueCommand::ClearAll,
    ]
}

/// **partition 檻**: 全 variant について、action する演者が高々一つであり、かつ
/// `cue_target_of` の分類（Shell→seriko／Balloon→emo-text／None→誰も action しない）と
/// **一致**する（各演者の action ゲートが `cue_target_of` の単一権威に従う・D4/R2.4）。
#[test]
fn every_variant_has_at_most_one_acting_performer_consistent_with_cue_target_of() {
    // 各 variant の期待 action 演者（表示系＝Shell→seriko／文字系＝Balloon→emo-text／
    // action なし＝None）。この表は `cue_target_of`（dola 単一権威）と 1:1 で対応する。
    let expected: &[(CueCommand, Option<Performer>)] = &[
        (
            CueCommand::Emote { key: "0".into() },
            Some(Performer::Seriko),
        ),
        (CueCommand::EntityRef(42), Some(Performer::Seriko)),
        (
            CueCommand::BalloonSurface { key: "2".into() },
            Some(Performer::Seriko),
        ),
        (CueCommand::Text("hi".into()), Some(Performer::EmoText)),
        (CueCommand::NewLine { ratio: 1.0 }, Some(Performer::EmoText)),
        (CueCommand::Clear, Some(Performer::EmoText)),
        (CueCommand::ClearAll, Some(Performer::EmoText)),
        (
            CueCommand::Choice {
                id: "y".into(),
                text: "はい".into(),
                references: vec![],
            },
            Some(Performer::EmoText),
        ),
        // Cursor（`\_l`）はバルーン系表現者（emo-text＝Balloon）が action する。
        (
            CueCommand::Cursor {
                x: "5em".into(),
                y: "2lh".into(),
            },
            Some(Performer::EmoText),
        ),
        (
            CueCommand::Custom {
                command: "fade".into(),
                params: dola::DynamicValue::Null,
            },
            None,
        ),
        (CueCommand::Wait, None),
    ];

    // (a) 期待表の各 variant が `cue_target_of` の分類と一致する（action 演者は高々一つ）。
    for (command, want) in expected {
        let got = acting_performer(cue_target_of(command));
        assert_eq!(
            &got, want,
            "variant {command:?} の action 演者は cue_target_of の分類と一致するはず（partition・単一権威）"
        );
    }

    // (b) 全 variant が期待表に過不足なく現れる（漏れ・重複がない＝partition が全域）。
    assert_eq!(
        expected.len(),
        every_cue_command().len(),
        "期待表は全 CueCommand variant を過不足なく網羅する（partition の全域性）"
    );
}

/// **acceptance 補完檻**: 表情切替（`Emote`）は seriko（Shell）だけが action し、
/// テキスト演者（emo-text＝Balloon）は action しない（broadcast で受信はするが動作しない・
/// duration のみ honor する・R8.4/R2.3）。`ClearAll`／`Wait` の対比も併せて固定する。
#[test]
fn emote_acts_only_on_seriko_text_performer_receives_but_does_not_act() {
    let emote = CueCommand::Emote { key: "0".into() };
    assert_eq!(
        acting_performer(cue_target_of(&emote)),
        Some(Performer::Seriko),
        "Emote（表情切替）は seriko だけが action する（Shell 分類）"
    );
    assert_ne!(
        acting_performer(cue_target_of(&emote)),
        Some(Performer::EmoText),
        "テキスト演者（emo-text）は Emote を broadcast 受信するが action しない（duration のみ honor）"
    );

    // #6 全消去はテキスト演者だけが action（seriko は受信のみ）。
    assert_eq!(
        acting_performer(cue_target_of(&CueCommand::ClearAll)),
        Some(Performer::EmoText),
        "ClearAll は emo-text だけが action する（Balloon 分類・#6）"
    );

    // 純粋な待ちはどの演者も action しない（duration だけ honor）。
    assert_eq!(
        acting_performer(cue_target_of(&CueCommand::Wait)),
        None,
        "Wait はどの演者も action しない（action なし・duration のみ）"
    );
}
