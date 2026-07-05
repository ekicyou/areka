//! 純粋コンパイル層（compile）— `Instruction` 列 → cue ドメインの発火列＋確定終端理由。
//!
//! [`compile`] は clock・sink・talk_id・アクターを知らない**純粋関数**（決定的・no I/O）。
//! 上流 [`areka_parsers::sakura::parse`] の出力 `Instruction` 列を走査し、
//!
//! - `Wait(Duration)` を `as_secs_f64()` で単調非減少に累積（`\w`×50ms を再導出しない・
//!   待ち時間の唯一の真実は上流正規化済みの `Duration`・R2.2/2.3/2.4）、
//! - 話者スコープ `SpeakerScope{n}` を現在 scope として保持し各 `Cue::actor` へ転写（R5）、
//! - `Text`/`Surface`/`NewLine`/`Clear` を対応する [`CueCommand`] へ写像（R3/R4）、
//!
//! して発火時刻付きの [`CueSheet`] を構築する。
//!
//! # 禁止事項
//!
//! `dola::cue::compile_sheet` は最小 `start_time` を 0 起点へ正規化するため、**先頭に待ち命令が
//! ある script でその待ち時間が 0 へ潰れる**。本関数は 0 起点相対秒（`offset` を 0.0 から累積）を
//! そのまま `start_time` とし、`compile_sheet` を使わない。先頭待ちは保存される（R2.4）。

use crate::contract::{ActorKey, Cue, CueCommand, CuePayload, CueSheet, TalkEndReason};
use areka_parsers::sakura::Instruction;

/// 純粋コンパイル: `Instruction` 列 → cue ドメインの発火列＋確定終端理由（決定的・no I/O）。
///
/// 事前条件: `instructions` は [`areka_parsers::sakura::parse`] の出力（再パースしない・R1.2）。
///
/// 事後条件: `sheet` 内の `start_time` は有限・非負・非減少（`Duration` 由来の構成的保証）。
/// 同一入力に対し同一出力（決定的・R2.5）。各 `Cue::actor` はその発火時点の有効 scope の転写。
pub fn compile(instructions: &[Instruction]) -> CompiledTalk {
    let mut offset: f64 = 0.0;
    let mut scope: u32 = 0;
    let mut cues: Vec<Cue> = Vec::new();

    for instruction in instructions {
        match instruction {
            // 時刻累積（cue を生成しない・R2.2/2.3）。単位換算のみ。
            Instruction::Wait(duration) => {
                offset += duration.as_secs_f64();
            }
            // scope 状態更新（cue を生成しない・R5.1）。転写のみ。
            Instruction::SpeakerScope { n } => {
                scope = *n;
            }
            // テキスト表示（→Balloon・R4.1）。
            Instruction::Text(text) => {
                cues.push(emit(scope, offset, CueCommand::Text(text.clone())));
            }
            // サーフェス切替（不透明転写・→Shell・R3.1/3.2）。引数を解釈・変換しない。
            Instruction::Surface(arg) => {
                cues.push(emit(
                    scope,
                    offset,
                    CueCommand::Emote {
                        key: arg.as_str().to_string(),
                    },
                ));
            }
            // 改行（比率・DD-9・→Balloon・R4.2）。
            Instruction::NewLine(ratio) => {
                cues.push(emit(scope, offset, CueCommand::NewLine { ratio: ratio.ratio() }));
            }
            // クリア（→Balloon・R4.3）。
            Instruction::Clear => {
                cues.push(emit(scope, offset, CueCommand::Clear));
            }
            // End/Quit の終端切詰め・M-boot 外タグの無視ログは task 3.2 で処理する。
            // `Instruction` は #[non_exhaustive] ゆえ catch-all が必須。
            _ => {}
        }
    }

    CompiledTalk {
        sheet: CueSheet::new(cues),
        end: TalkEndReason::Ended,
    }
}

/// 現在 scope・累積 offset・演出コマンドから 1 発火 [`Cue`] を構築する。
///
/// `actor` はその発火時点の有効 scope の転写（`ActorKey::from(n.to_string())`・既定 "0"・R5.2/5.3）。
fn emit(scope: u32, offset: f64, command: CueCommand) -> Cue {
    Cue {
        actor: ActorKey::from(scope.to_string()),
        start_time: offset,
        payload: CuePayload::Command(command),
    }
}

/// [`compile`] の結果。`sheet` は wintf cue パイプラインがそのまま消費できる serde 可能形
/// （ghost-setup handoff 成果物）。
pub struct CompiledTalk {
    /// 0 起点相対秒の発火列（`CuePayload::Command` のみ）。
    pub sheet: CueSheet,
    /// コンパイル時点で確定した終端理由。
    pub end: TalkEndReason,
}

#[cfg(test)]
mod tests {
    use super::*;
    use areka_parsers::sakura::{NewLineRatio, SurfaceArg};
    use std::time::Duration;

    /// `Cue::payload` から `CueCommand` を取り出すヘルパ（`Cue` は PartialEq 非導出）。
    fn command_of(cue: &Cue) -> &CueCommand {
        match &cue.payload {
            CuePayload::Command(cmd) => cmd,
            other => panic!("expected CuePayload::Command, got {other:?}"),
        }
    }

    /// サーフェス切替命令の引数を解釈・変換せず値のまま転写する（R3.1/3.2）。
    /// `"0,1,foo"` はカンマ区切りを一切パースせず `Emote{key}` へバイト完全転写される。
    #[test]
    fn surface_arg_is_transcribed_opaquely() {
        let compiled = compile(&[Instruction::Surface(SurfaceArg::new("0,1,foo".into()))]);
        let cues = compiled.sheet.cues();
        assert_eq!(cues.len(), 1);
        match command_of(&cues[0]) {
            CueCommand::Emote { key } => assert_eq!(key, "0,1,foo"),
            other => panic!("expected Emote, got {other:?}"),
        }
    }

    /// 先頭に待ち命令がある場合でもその待ち時間が 0 へ潰れず保存される（R2.4）。
    /// `compile_sheet` の 0 正規化を使っていないことの固定。
    #[test]
    fn leading_wait_is_preserved_not_collapsed() {
        let compiled = compile(&[
            Instruction::Wait(Duration::from_millis(450)),
            Instruction::Text("hi".into()),
        ]);
        let cues = compiled.sheet.cues();
        assert_eq!(cues.len(), 1);
        // 期待値は同一の as_secs_f64() で計算（10 進リテラル直書きの表現誤差を排除）。
        let expected = Duration::from_millis(450).as_secs_f64();
        assert_eq!(cues[0].start_time, expected);
        assert_ne!(cues[0].start_time, 0.0);
    }

    /// 待ち累積列に対し発火時刻が単調に累積する（R2.2/2.4）。
    /// 期待値は SAME as_secs_f64() 累積で計算（IEEE-754 加算を一致させる）。
    #[test]
    fn wait_accumulation_is_monotonic() {
        let compiled = compile(&[
            Instruction::Text("a".into()),
            Instruction::Wait(Duration::from_millis(50)),
            Instruction::Text("b".into()),
            Instruction::Wait(Duration::from_millis(100)),
            Instruction::Surface(SurfaceArg::new("1".into())),
        ]);
        let cues = compiled.sheet.cues();
        assert_eq!(cues.len(), 3);

        let t0 = 0.0_f64;
        let t1 = t0 + Duration::from_millis(50).as_secs_f64();
        let t2 = t1 + Duration::from_millis(100).as_secs_f64();

        assert_eq!(cues[0].start_time, t0);
        assert_eq!(cues[1].start_time, t1);
        assert_eq!(cues[2].start_time, t2);

        // 非減少（構成的保証の固定）。
        assert!(cues[0].start_time <= cues[1].start_time);
        assert!(cues[1].start_time <= cues[2].start_time);
    }

    /// 話者スコープ切替で actor が転写され、未指定開始は既定 "0"（R5.1/5.2/5.3）。
    #[test]
    fn speaker_scope_is_transcribed_to_actor() {
        let compiled = compile(&[
            Instruction::SpeakerScope { n: 1 },
            Instruction::Text("a".into()),
            Instruction::SpeakerScope { n: 0 },
            Instruction::Surface(SurfaceArg::new("s".into())),
        ]);
        let cues = compiled.sheet.cues();
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].actor.as_str(), "1");
        assert_eq!(cues[1].actor.as_str(), "0");

        // SpeakerScope を先行させない talk は既定 "0"。
        let compiled = compile(&[Instruction::Text("hi".into())]);
        assert_eq!(compiled.sheet.cues()[0].actor.as_str(), "0");
    }

    /// NewLine/Clear の写像（DD-9・R4.2/4.3）。
    #[test]
    fn newline_and_clear_map_to_commands() {
        let compiled = compile(&[
            Instruction::NewLine(NewLineRatio::new(1.5)),
            Instruction::Clear,
        ]);
        let cues = compiled.sheet.cues();
        assert_eq!(cues.len(), 2);
        match command_of(&cues[0]) {
            CueCommand::NewLine { ratio } => assert_eq!(*ratio, 1.5_f32),
            other => panic!("expected NewLine, got {other:?}"),
        }
        assert_eq!(command_of(&cues[1]), &CueCommand::Clear);
    }

    /// 末尾到達で end=Ended（R6.3・task 3.1 の既定）。
    #[test]
    fn end_defaults_to_ended() {
        let compiled = compile(&[Instruction::Text("hi".into())]);
        assert_eq!(compiled.end, TalkEndReason::Ended);

        let compiled = compile(&[]);
        assert_eq!(compiled.end, TalkEndReason::Ended);
        assert!(compiled.sheet.is_empty());
    }

    /// 本 task で未対応の非基本 variant は cue を生成しない（catch-all no-op・非 panic）。
    #[test]
    fn non_basic_variants_produce_no_cue() {
        let compiled = compile(&[
            Instruction::SystemVar("username".into()),
            Instruction::Raw("\\?".into()),
            Instruction::Text("hi".into()),
        ]);
        // Text のみが cue になる。
        assert_eq!(compiled.sheet.cues().len(), 1);
    }
}
