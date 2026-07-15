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
    // 終端命令なしで末尾到達なら通常終了（R6.3）。End/Quit 検出で確定・走査打切り。
    let mut end = TalkEndReason::Ended;

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
            // バルーン面切替（不透明転写・→Shell・R3.1）。引数を解釈・変換しない（Surface と同型）。
            // 数値化・範囲展開・alias 解決はしない（バイト完全一致）。数値化は下流 seriko の責務。
            Instruction::BalloonSurface(arg) => {
                cues.push(emit(
                    scope,
                    offset,
                    CueCommand::BalloonSurface {
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
            // 終端 `\e`（R6.1/6.5）: 終端理由 Ended を確定し以降を切り詰める。
            // ukadoc `\e` = この後に書かれたスクリプトは実行・表示されない。
            Instruction::End => {
                end = TalkEndReason::Ended;
                break;
            }
            // 終了 `\-`（R6.2/6.5）: 終端理由 Quit を確定し以降を切り詰める。
            Instruction::Quit => {
                end = TalkEndReason::Quit;
                break;
            }
            // M-boot 外タグ（Choice/Cursor/Move/SystemVar/GenericCommand/Raw）および
            // `#[non_exhaustive]` の未知 variant は無視ログを記録し cue を生成せず継続する
            // （寛容・非 panic・型シーム・R8.1/8.2/8.3/R11.2）。写像先は後続 M-dialogue。
            other => {
                tracing::debug!(instruction = ?other, "M-boot 外タグを無視");
            }
        }
    }

    CompiledTalk {
        sheet: CueSheet::new(cues),
        end,
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
        // 現状の compile は全 cue を瞬時（明示的 0）として発行する。
        duration: 0.0,
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

    /// `Cue` 単位のフィールド等価（`Cue` は PartialEq 非導出のためフィールド比較）。
    /// `start_time` は決定性の観測ゆえビット同一（`==`）を要求する。
    /// `actor`（PartialEq）と `payload`（CuePayload/CueCommand は PartialEq）は等価比較。
    fn cue_eq(a: &Cue, b: &Cue) -> bool {
        a.actor == b.actor && a.start_time == b.start_time && a.payload == b.payload
    }

    /// 多様な variant を織り交ぜた代表的な命令列（決定性・不変条件の両テストで共用）。
    /// 終端命令は含めず全命令を走査させる（末尾到達で end=Ended）。
    fn representative_instructions() -> Vec<Instruction> {
        vec![
            Instruction::SpeakerScope { n: 1 },
            Instruction::Text("a".into()),
            Instruction::Wait(Duration::from_millis(50)),
            Instruction::Surface(SurfaceArg::new("10".into())),
            Instruction::Wait(Duration::from_millis(100)),
            Instruction::NewLine(NewLineRatio::new(1.5)),
            Instruction::Clear,
            Instruction::SpeakerScope { n: 0 },
            Instruction::Text("b".into()),
        ]
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

    /// バルーン面切替命令の引数を解釈・変換せず値のまま `BalloonSurface{key}` へ
    /// バイト完全一致で転写する（不透明写像・R3.1）。名前形（非 ASCII）・数値形・非表示
    /// センチネル `-1` のいずれも数値化・alias 解決せず、`Surface`→`Emote` と完全対称に扱う。
    /// `BalloonSurface` が catch-all（M-boot 外タグの無視ログ）へ落ちて破棄されないことの固定。
    #[test]
    fn balloon_surface_arg_is_transcribed_opaquely() {
        // 名前形（非 ASCII）: 整数化せず文字列のまま転写。
        let compiled = compile(&[Instruction::BalloonSurface(SurfaceArg::new(
            "バルーン１".into(),
        ))]);
        let cues = compiled.sheet.cues();
        assert_eq!(cues.len(), 1, "BalloonSurface が cue を生成しない（破棄された）");
        match command_of(&cues[0]) {
            CueCommand::BalloonSurface { key } => assert_eq!(key, "バルーン１"),
            other => panic!("expected BalloonSurface, got {other:?}"),
        }

        // 数値形: 数値化・展開せず文字列のまま転写。
        let compiled = compile(&[Instruction::BalloonSurface(SurfaceArg::new("10".into()))]);
        let cues = compiled.sheet.cues();
        assert_eq!(cues.len(), 1);
        match command_of(&cues[0]) {
            CueCommand::BalloonSurface { key } => assert_eq!(key, "10"),
            other => panic!("expected BalloonSurface, got {other:?}"),
        }

        // 非表示センチネル `-1`: パース段階同様に数値化せず不透明転写。
        let compiled = compile(&[Instruction::BalloonSurface(SurfaceArg::new("-1".into()))]);
        let cues = compiled.sheet.cues();
        assert_eq!(cues.len(), 1);
        match command_of(&cues[0]) {
            CueCommand::BalloonSurface { key } => assert_eq!(key, "-1"),
            other => panic!("expected BalloonSurface, got {other:?}"),
        }
    }

    /// バルーン面切替命令の追加後も、既存のシェル面切替 `Surface`→`Emote` 写像は不変
    /// （additive・R3.1 既存写像非変更）。scope/start_time 転写も従来通り。
    #[test]
    fn surface_to_emote_mapping_is_unchanged_by_balloon_arm() {
        let compiled = compile(&[
            Instruction::SpeakerScope { n: 1 },
            Instruction::Surface(SurfaceArg::new("0,1,foo".into())),
        ]);
        let cues = compiled.sheet.cues();
        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].actor.as_str(), "1");
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

    /// 本 task で未対応の非基本 variant は cue を生成しない（無視ログ・非 panic）。
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

    /// `End` 検出で終端理由 `Ended` を確定し、以降の命令を発火列へ含めず破棄する
    /// （終端切詰め・R6.1/6.5）。ukadoc `\e` = この後のスクリプトは実行・表示されない。
    #[test]
    fn end_truncates_following_instructions() {
        let compiled = compile(&[
            Instruction::Text("a".into()),
            Instruction::End,
            Instruction::Text("b".into()),
        ]);
        let cues = compiled.sheet.cues();
        // "a" のみが cue になり "b" は切り詰められる。
        assert_eq!(cues.len(), 1);
        match command_of(&cues[0]) {
            CueCommand::Text(s) => assert_eq!(s, "a"),
            other => panic!("expected Text(\"a\"), got {other:?}"),
        }
        assert_eq!(compiled.end, TalkEndReason::Ended);
    }

    /// `Quit` 検出で終端理由 `Quit` を確定し、以降の命令を破棄する（終端切詰め・R6.2/6.5）。
    #[test]
    fn quit_truncates_following_instructions_and_sets_quit() {
        let compiled = compile(&[
            Instruction::Text("a".into()),
            Instruction::Quit,
            Instruction::Text("b".into()),
        ]);
        let cues = compiled.sheet.cues();
        assert_eq!(cues.len(), 1);
        match command_of(&cues[0]) {
            CueCommand::Text(s) => assert_eq!(s, "a"),
            other => panic!("expected Text(\"a\"), got {other:?}"),
        }
        assert_eq!(compiled.end, TalkEndReason::Quit);
    }

    /// 終端命令なしで末尾まで到達した列は全命令が発火し end=Ended（末尾到達・R6.3）。
    #[test]
    fn no_terminal_keeps_all_instructions_and_ends() {
        let compiled = compile(&[Instruction::Text("a".into()), Instruction::Text("b".into())]);
        assert_eq!(compiled.sheet.cues().len(), 2);
        assert_eq!(compiled.end, TalkEndReason::Ended);
        assert_ne!(compiled.end, TalkEndReason::Quit);
    }

    /// 先行 cue のない `[Quit]` でも Quit が検出され、空 sheet＋end=Quit となる
    /// （空 sheet でも Ended と判別可能なことの固定・下流 R6.2 の区別根拠）。
    #[test]
    fn bare_quit_yields_empty_sheet_with_quit_end() {
        let compiled = compile(&[Instruction::Quit]);
        assert!(compiled.sheet.is_empty());
        assert_eq!(compiled.end, TalkEndReason::Quit);
    }

    /// M-boot 外タグ（Choice/Cursor/Move/SystemVar/GenericCommand/Raw）は
    /// 無視ログのみで cue を生成せず非 panic。間に挟んだ Text だけが cue になる
    /// （R8.1/8.3/11.2）。無視のログ記録（R8.2・`tracing::debug!`）は実装済みだが
    /// 本テストは no-cue＋非 panic を主観測とし、ログ出力自体はコード検査で担保する。
    #[test]
    fn m_boot_outside_tags_are_ignored_without_cue_or_panic() {
        use areka_parsers::sakura::{Choice, MoveArgs};

        let compiled = compile(&[
            Instruction::Choice(Choice {
                disp: "はい".into(),
                target: "OnYes".into(),
                references: vec!["ref".into()],
            }),
            Instruction::Cursor {
                x: "10".into(),
                y: "20".into(),
            },
            Instruction::Move(MoveArgs {
                args: vec!["100".into(), "200".into()],
            }),
            Instruction::SystemVar("username".into()),
            Instruction::GenericCommand {
                name: "raise".into(),
                raw_args: vec!["OnBoot".into()],
            },
            Instruction::Raw("\\?".into()),
            Instruction::Text("only-me".into()),
        ]);
        let cues = compiled.sheet.cues();
        // 無視タグ群は 0 cue、Text のみが 1 cue。
        assert_eq!(cues.len(), 1);
        match command_of(&cues[0]) {
            CueCommand::Text(s) => assert_eq!(s, "only-me"),
            other => panic!("expected Text(\"only-me\"), got {other:?}"),
        }
        // 終端命令を含まないため末尾到達で Ended。
        assert_eq!(compiled.end, TalkEndReason::Ended);
    }

    /// 決定的コンパイル: 同一命令列を複数回コンパイルすると常に同一の発火列・終端理由を得る
    /// （R2.5・R9.4）。cue 数・各 index の actor/start_time（ビット同一）/payload・end が一致する。
    #[test]
    fn compile_is_deterministic_for_identical_input() {
        let instructions = representative_instructions();

        let first = compile(&instructions);
        let second = compile(&instructions);

        let a = first.sheet.cues();
        let b = second.sheet.cues();
        assert_eq!(a.len(), b.len(), "cue 数が回によって異なる");
        for (i, (ca, cb)) in a.iter().zip(b.iter()).enumerate() {
            assert!(
                cue_eq(ca, cb),
                "index {i} の cue が回によって異なる: {ca:?} != {cb:?}"
            );
        }
        assert_eq!(first.end, second.end, "終端理由が回によって異なる");
    }

    /// 不変条件（純粋コンパイル層 Postconditions/Invariants・NaN 放電）:
    /// 生成される全 `start_time` は有限・非負であり、`sheet.cues()` 順に非減少
    /// （`Duration` 由来の構成的保証）。待ちの無い連続 cue は同一時刻を共有し（非減少・非狭義増加）、
    /// 待ちを挟む cue で時刻が増加することで `<=` の固定が意味を持つ。
    #[test]
    fn compiled_start_times_are_finite_non_negative_non_decreasing() {
        // 先頭 2 cue は待ち無し＝同一時刻を共有（非減少）、以降は待ちで増加する。
        let compiled = compile(&[
            Instruction::Text("a".into()),
            Instruction::Surface(SurfaceArg::new("1".into())),
            Instruction::Wait(Duration::from_millis(50)),
            Instruction::Text("b".into()),
            Instruction::Wait(Duration::from_millis(100)),
            Instruction::Text("c".into()),
        ]);
        let cues = compiled.sheet.cues();
        assert_eq!(cues.len(), 4);

        // 有限・非負（NaN/∞/負の放電）。
        for (i, cue) in cues.iter().enumerate() {
            assert!(cue.start_time.is_finite(), "index {i} の start_time が非有限");
            assert!(cue.start_time >= 0.0, "index {i} の start_time が負");
        }

        // 非減少（構成的保証の固定）。
        for pair in cues.windows(2) {
            assert!(
                pair[0].start_time <= pair[1].start_time,
                "start_time が減少した: {} > {}",
                pair[0].start_time,
                pair[1].start_time
            );
        }

        // 待ち無しの先頭 2 cue は同一時刻（非狭義増加＝非減少の `<=` が真に効くことの固定）。
        assert_eq!(cues[0].start_time, cues[1].start_time);
        // 待ちを挟んだ cue は狭義増加。
        assert!(cues[1].start_time < cues[2].start_time);
        assert!(cues[2].start_time < cues[3].start_time);
    }
}
