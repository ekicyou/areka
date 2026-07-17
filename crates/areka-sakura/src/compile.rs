//! 純粋コンパイル層（compile）— `Instruction` 列 → cue ドメインの発火列＋確定終端理由。
//!
//! [`compile`] は clock・sink・talk_id・アクターを知らない**純粋関数**（決定的・no I/O）。
//! 上流 [`areka_parsers::sakura::parse`] の出力 `Instruction` 列を走査し、
//!
//! - `Text` に対し [`crate::duration::text_playback_duration`] で暗黙 per-char 再生時間 D を
//!   算出して当該テキスト cue の envelope `duration` へ焼き込み、直後 `offset += D` で後続 cue の
//!   発火時刻をテキスト再生完了後へ確定させる（R4.1/4.2/4.3）、
//! - `Wait(Duration)` を offset へ吸収して消さず、**action を持たず duration のみを持つ第一級
//!   `CueCommand::Wait` cue** として当該 offset へ発行し、直後 `offset += d`（`as_secs_f64()`）で
//!   後続整列を進める（`\w`×50ms を再導出せず上流正規化済み `Duration` を唯一の真実とし、末尾・
//!   単独の待ちも台本に残る自己完結した楽譜・R5.1/5.3/4.4・D3）、
//! - 話者スコープ `SpeakerScope{n}` を現在 scope として保持し各 `Cue::actor` へ転写（R5）、
//! - `Text`/`Surface`/`BalloonSurface`/`NewLine`/`Clear` を対応する [`CueCommand`] へ写像（R3/R4）、
//! - 内容 cue を持つ台本の先頭へ **`ClearAll` cue を単一前置**（`start_time=0.0`・`duration=0.0`）し、
//!   新 talk が全バルーン空から始まるようにする（#6・R6.1/6.2）、
//!
//! して発火時刻付きの [`CueSheet`] を構築する。
//!
//! # 先頭待ちの保存（R2.4）
//!
//! 本関数は 0 起点相対秒（`offset` を 0.0 から累積）をそのまま `start_time` とし、min 正規化を
//! 行わない（min 正規化は**先頭に待ち命令がある script でその待ち時間を 0 へ潰す**ため）。下流の
//! canonical 変換 [`dola::cue::to_talk_schedule`] も相対 `start_time` を保存するため、先頭待ちは
//! 台本〜スケジュールを通して保存される。

use crate::contract::{ActorKey, Cue, CueCommand, CuePayload, CueSheet, TalkEndReason};
use areka_parsers::sakura::Instruction;

/// 純粋コンパイル: `Instruction` 列 → cue ドメインの発火列＋確定終端理由（決定的・no I/O）。
///
/// 事前条件: `instructions` は [`areka_parsers::sakura::parse`] の出力（再パースしない・R1.2）。
///
/// 事後条件: `sheet` 内の `start_time` は有限・非負・非減少（`Duration` 由来の構成的保証）。
/// 各テキスト cue は `duration = text_playback_duration(text)`（N>0 で正）、各 Wait cue は明示待ちの
/// `duration`、他の瞬時 cue は `duration=0.0` を持つ。内容 cue を持つ台本は先頭に単一 `ClearAll`
/// （`start_time=0.0`・`duration=0.0`）を持ち、台本のみから talk 全時間範囲
/// （`max(start_time + duration)`）が復元可能。同一入力に対し同一出力（決定的・R2.5）。
/// 各 `Cue::actor` はその発火時点の有効 scope の転写。
pub fn compile(instructions: &[Instruction]) -> CompiledTalk {
    let mut offset: f64 = 0.0;
    let mut scope: u32 = 0;
    let mut cues: Vec<Cue> = Vec::new();
    // 終端命令なしで末尾到達なら通常終了（R6.3）。End/Quit 検出で確定・走査打切り。
    let mut end = TalkEndReason::Ended;

    for instruction in instructions {
        match instruction {
            // 明示ウェイトを offset へ吸収せず、action を持たず duration のみを持つ第一級 Wait cue
            // として当該 offset へ発行し、直後 offset を進める（末尾・単独でも台本に残る・R5.1/4.4・D3）。
            // 時間は envelope duration が担い、コマンドは action の種別のみを表す。
            Instruction::Wait(duration) => {
                let d = duration.as_secs_f64();
                cues.push(emit(scope, offset, d, CueCommand::Wait));
                offset += d;
            }
            // scope 状態更新（cue を生成しない・R5.1）。転写のみ。
            Instruction::SpeakerScope { n } => {
                scope = *n;
            }
            // テキスト表示（→Balloon・R4.1）。暗黙 per-char 再生時間 D を envelope duration へ焼き込み、
            // 直後 offset += D で後続 cue をテキスト再生完了後へ整列する（R4.1/4.2/4.3）。
            Instruction::Text(text) => {
                let d = crate::duration::text_playback_duration(text);
                cues.push(emit(scope, offset, d, CueCommand::Text(text.clone())));
                offset += d;
            }
            // サーフェス切替（不透明転写・→Shell・R3.1/3.2）。引数を解釈・変換しない。瞬時（duration 0）。
            Instruction::Surface(arg) => {
                cues.push(emit(
                    scope,
                    offset,
                    0.0,
                    CueCommand::Emote {
                        key: arg.as_str().to_string(),
                    },
                ));
            }
            // バルーン面切替（不透明転写・→Shell・R3.1）。引数を解釈・変換しない（Surface と同型）。
            // 数値化・範囲展開・alias 解決はしない（バイト完全一致）。数値化は下流 seriko の責務。瞬時。
            Instruction::BalloonSurface(arg) => {
                cues.push(emit(
                    scope,
                    offset,
                    0.0,
                    CueCommand::BalloonSurface {
                        key: arg.as_str().to_string(),
                    },
                ));
            }
            // 改行（比率・DD-9・→Balloon・R4.2）。瞬時（duration 0）。
            Instruction::NewLine(ratio) => {
                cues.push(emit(
                    scope,
                    offset,
                    0.0,
                    CueCommand::NewLine {
                        ratio: ratio.ratio(),
                    },
                ));
            }
            // クリア（対象スコープのみ・→Balloon・R4.3）。瞬時（duration 0）。
            Instruction::Clear => {
                cues.push(emit(scope, offset, 0.0, CueCommand::Clear));
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

    // 冒頭 ClearAll 前置（#6・R6.1/6.2）: 内容 cue を持つ台本の先頭へ全スコープ消去 `ClearAll` を
    // 単一前置する（`start_time=0.0`・`duration=0.0`）。compile は残存スコープを列挙できないため
    // per-scope Clear でなく全消し `ClearAll`（自己完結）で表現し、書き込むスコープ数に依らず 1 件。
    // `CueSheet::new` の安定ソート＋同一 `at` FIFO により先頭配送される（index 0 挿入で 0.0 群の先頭）。
    // 内容 cue を持たない台本（リテラル空・裸終端・無視タグのみ）は空 sheet のままとし、drive の
    // `is_empty()` 即時 TalkDone 契約を保つ（消すべき前 talk のテキストも無い・ドライブ配線は task 7.x）。
    if !cues.is_empty() {
        cues.insert(0, emit(0, 0.0, 0.0, CueCommand::ClearAll));
    }

    CompiledTalk {
        sheet: CueSheet::new(cues),
        end,
    }
}

/// 現在 scope・累積 offset・再生時間 duration・演出コマンドから 1 発火 [`Cue`] を構築する。
///
/// `actor` はその発火時点の有効 scope の転写（`ActorKey::from(n.to_string())`・既定 "0"・R5.2/5.3）。
/// `duration` は当該 cue の presentation 占有時間（テキストは D、Wait は明示待ち、他の瞬時は 0.0）。
fn emit(scope: u32, offset: f64, duration: f64, command: CueCommand) -> Cue {
    Cue {
        actor: ActorKey::from(scope.to_string()),
        start_time: offset,
        payload: CuePayload::Command(command),
        duration,
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
    use crate::duration::text_playback_duration;
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
    /// `start_time`・`duration` は決定性の観測ゆえビット同一（`==`）を要求する（compile が
    /// テキスト cue へ焼き込む再生時間 D の回帰を素通しさせない・task 5.1 申し送り）。
    /// `actor`（PartialEq）と `payload`（CuePayload/CueCommand は PartialEq）は等価比較。
    fn cue_eq(a: &Cue, b: &Cue) -> bool {
        a.actor == b.actor
            && a.start_time == b.start_time
            && a.payload == b.payload
            && a.duration == b.duration
    }

    /// 内容 cue を持つ台本は先頭へ単一 `ClearAll`（`start_time=0.0`・`duration=0.0`）を前置する
    /// （#6・R6.1/6.2）。その前置を検証しつつ、後続の**内容 cue**のスライスを返すヘルパ
    /// （各テストの ClearAll 前置検証を集約し、内容側の index を従来どおり保つ）。
    fn assert_clear_all_prefix_and_rest(cues: &[Cue]) -> &[Cue] {
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
        // ClearAll は単一前置（内容側に重複しない・スコープ数に依らず 1 件）。
        assert!(
            cues[1..]
                .iter()
                .all(|c| command_of(c) != &CueCommand::ClearAll),
            "ClearAll は単一前置（内容 cue 側に重複しない）"
        );
        &cues[1..]
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
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
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
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        assert_eq!(
            cues.len(),
            1,
            "BalloonSurface が cue を生成しない（破棄された）"
        );
        match command_of(&cues[0]) {
            CueCommand::BalloonSurface { key } => assert_eq!(key, "バルーン１"),
            other => panic!("expected BalloonSurface, got {other:?}"),
        }

        // 数値形: 数値化・展開せず文字列のまま転写。
        let compiled = compile(&[Instruction::BalloonSurface(SurfaceArg::new("10".into()))]);
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        assert_eq!(cues.len(), 1);
        match command_of(&cues[0]) {
            CueCommand::BalloonSurface { key } => assert_eq!(key, "10"),
            other => panic!("expected BalloonSurface, got {other:?}"),
        }

        // 非表示センチネル `-1`: パース段階同様に数値化せず不透明転写。
        let compiled = compile(&[Instruction::BalloonSurface(SurfaceArg::new("-1".into()))]);
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
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
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].actor.as_str(), "1");
        match command_of(&cues[0]) {
            CueCommand::Emote { key } => assert_eq!(key, "0,1,foo"),
            other => panic!("expected Emote, got {other:?}"),
        }
    }

    /// 先頭に待ち命令がある場合でもその待ち時間が 0 へ潰れず保存される（R2.4）。
    /// min 正規化（先頭待ちを 0 へ食う旧実装）を使っていないことの固定。第一級化後は、先頭待ちが
    /// `duration` 付き Wait cue（`start_time=0.0`）として台本に残り、後続 Text はその分だけ遅れる。
    #[test]
    fn leading_wait_is_preserved_not_collapsed() {
        let compiled = compile(&[
            Instruction::Wait(Duration::from_millis(450)),
            Instruction::Text("hi".into()),
        ]);
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        assert_eq!(cues.len(), 2, "内容は先頭 Wait と Text の 2 件");
        // 期待値は同一の as_secs_f64() で計算（10 進リテラル直書きの表現誤差を排除）。
        let w = Duration::from_millis(450).as_secs_f64();
        // 先頭 Wait cue は 0.0 で保存され（潰れず）、待ち時間を duration に持つ。
        assert_eq!(
            command_of(&cues[0]),
            &CueCommand::Wait,
            "先頭内容は Wait cue"
        );
        assert_eq!(cues[0].start_time, 0.0);
        assert_eq!(cues[0].duration, w);
        // 後続 Text は先頭待ちが潰れず 450ms 後に発火する（0 へ正規化されない）。
        match command_of(&cues[1]) {
            CueCommand::Text(s) => assert_eq!(s, "hi"),
            other => panic!("expected Text, got {other:?}"),
        }
        assert_eq!(cues[1].start_time, w);
        assert_ne!(cues[1].start_time, 0.0);
    }

    /// 暗黙 per-char D と明示ウェイトの累積で発火時刻が単調に進む（R2.2/2.4/4.4）。
    /// テキスト D 焼き込み・Wait 第一級化の後も、明示ウェイト累積が退行しない（4.4 非退行）。
    /// 期待値は SAME `text_playback_duration`/`as_secs_f64()` 累積で計算（IEEE-754 加算を一致させる）。
    #[test]
    fn wait_accumulation_is_monotonic() {
        let compiled = compile(&[
            Instruction::Text("a".into()),
            Instruction::Wait(Duration::from_millis(50)),
            Instruction::Text("b".into()),
            Instruction::Wait(Duration::from_millis(100)),
            Instruction::Surface(SurfaceArg::new("1".into())),
        ]);
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        // 内容: Text("a") / Wait / Text("b") / Wait / Emote の 5 件。
        assert_eq!(cues.len(), 5);

        let d_a = text_playback_duration("a");
        let w50 = Duration::from_millis(50).as_secs_f64();
        let d_b = text_playback_duration("b");
        let w100 = Duration::from_millis(100).as_secs_f64();

        let t_text_a = 0.0_f64;
        let t_wait1 = t_text_a + d_a; // Text("a") の再生完了後
        let t_text_b = t_wait1 + w50; // 明示 \w[50] の累積
        let t_wait2 = t_text_b + d_b; // Text("b") の再生完了後
        let t_emote = t_wait2 + w100; // 明示 \w[100] の累積

        // Text("a")
        assert_eq!(command_of(&cues[0]), &CueCommand::Text("a".into()));
        assert_eq!(cues[0].start_time, t_text_a);
        assert_eq!(cues[0].duration, d_a);
        // Wait（\w[50]）— 第一級・duration に待ち時間
        assert_eq!(command_of(&cues[1]), &CueCommand::Wait);
        assert_eq!(cues[1].start_time, t_wait1);
        assert_eq!(cues[1].duration, w50);
        // Text("b")
        assert_eq!(command_of(&cues[2]), &CueCommand::Text("b".into()));
        assert_eq!(cues[2].start_time, t_text_b);
        assert_eq!(cues[2].duration, d_b);
        // Wait（\w[100]）
        assert_eq!(command_of(&cues[3]), &CueCommand::Wait);
        assert_eq!(cues[3].start_time, t_wait2);
        assert_eq!(cues[3].duration, w100);
        // Emote（瞬時・テキスト D と待ちの累積後に発火）
        assert_eq!(command_of(&cues[4]), &CueCommand::Emote { key: "1".into() });
        assert_eq!(cues[4].start_time, t_emote);
        assert_eq!(cues[4].duration, 0.0);

        // 非減少（構成的保証の固定）。
        for pair in cues.windows(2) {
            assert!(pair[0].start_time <= pair[1].start_time);
        }
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
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].actor.as_str(), "1");
        assert_eq!(cues[1].actor.as_str(), "0");

        // SpeakerScope を先行させない talk は既定 "0"（内容先頭 Text の actor）。
        let compiled = compile(&[Instruction::Text("hi".into())]);
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        assert_eq!(cues[0].actor.as_str(), "0");
    }

    /// NewLine/Clear の写像（DD-9・R4.2/4.3）。
    #[test]
    fn newline_and_clear_map_to_commands() {
        let compiled = compile(&[
            Instruction::NewLine(NewLineRatio::new(1.5)),
            Instruction::Clear,
        ]);
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        assert_eq!(cues.len(), 2);
        match command_of(&cues[0]) {
            CueCommand::NewLine { ratio } => assert_eq!(*ratio, 1.5_f32),
            other => panic!("expected NewLine, got {other:?}"),
        }
        // 対象スコープのみ消去の `Clear`（冒頭前置の全消去 `ClearAll` とは別コマンド）。
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
        // SystemVar/Raw は cue を生成せず、内容は Text のみ（冒頭に ClearAll が前置される）。
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        assert_eq!(cues.len(), 1);
        match command_of(&cues[0]) {
            CueCommand::Text(s) => assert_eq!(s, "hi"),
            other => panic!("expected Text, got {other:?}"),
        }
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
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        // 内容は "a" のみ（"b" は切り詰められる）。冒頭に ClearAll が前置される。
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
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
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
        // 全命令が cue になる（"a"/"b" の 2 件）。冒頭に ClearAll が前置される。
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        assert_eq!(cues.len(), 2);
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
        // 無視タグ群は 0 cue、内容は Text のみ（冒頭に ClearAll が前置される）。
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
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
        // 冒頭 ClearAll と先頭 Text は待ち無し＝同一時刻 0.0 を共有（非減少）、以降は
        // テキスト D と待ちで増加する。
        let compiled = compile(&[
            Instruction::Text("a".into()),
            Instruction::Surface(SurfaceArg::new("1".into())),
            Instruction::Wait(Duration::from_millis(50)),
            Instruction::Text("b".into()),
            Instruction::Wait(Duration::from_millis(100)),
            Instruction::Text("c".into()),
        ]);
        // 全 sheet（冒頭 ClearAll 含む）: ClearAll / Text("a") / Emote / Wait / Text("b") / Wait / Text("c")。
        let cues = compiled.sheet.cues();
        assert_eq!(cues.len(), 7);

        // 有限・非負（NaN/∞/負の放電）。
        for (i, cue) in cues.iter().enumerate() {
            assert!(
                cue.start_time.is_finite(),
                "index {i} の start_time が非有限"
            );
            assert!(cue.start_time >= 0.0, "index {i} の start_time が負");
            assert!(cue.duration.is_finite(), "index {i} の duration が非有限");
            assert!(cue.duration >= 0.0, "index {i} の duration が負");
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

        // 待ち無しの冒頭 2 cue（ClearAll@0.0 と Text("a")@0.0）は同一時刻
        // （非狭義増加＝非減少の `<=` が真に効くことの固定）。
        assert_eq!(cues[0].start_time, cues[1].start_time);
        assert_eq!(cues[0].start_time, 0.0);
        // テキスト D を挟んだ cue は狭義増加（Text("a")@0.0 < Emote@D_a）。
        assert!(cues[1].start_time < cues[2].start_time);
        // 同一時刻に並ぶ Emote と Wait（ともに D_a）は非狭義増加（`<=` の固定・FIFO で Emote が先）。
        assert_eq!(cues[2].start_time, cues[3].start_time);
    }

    // ── task 5.2: D 焼き込み・Wait 第一級化・ClearAll 前置の behavioral 檻 ──

    /// テキスト cue は算出した再生時間 D を envelope duration として保持する（N>0 で D>0・R4.1）。
    /// 期待値は同一の `text_playback_duration` で導出（10 進直書きの表現誤差を排除）。
    #[test]
    fn text_cue_carries_playback_duration() {
        let compiled = compile(&[Instruction::Text("こんにちは".into())]);
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        assert_eq!(cues.len(), 1, "内容は Text cue 1 件のみ");
        let expected = text_playback_duration("こんにちは"); // 5 char × 50ms = 0.25s
        assert!(expected > 0.0, "N>0 の再生時間は正");
        match command_of(&cues[0]) {
            CueCommand::Text(s) => assert_eq!(s, "こんにちは"),
            other => panic!("expected Text, got {other:?}"),
        }
        assert_eq!(cues[0].start_time, 0.0);
        assert_eq!(
            cues[0].duration, expected,
            "テキスト cue の duration は text_playback_duration の値（R4.1）"
        );
    }

    /// テキスト cue の直後に別の cue が続くと、後続 cue はテキスト再生完了後（`text_start + D`）へ
    /// 焼き込まれる（R4.2）。`Text("ab") Surface("7")` → Emote は 2×50ms 後に発火する。
    #[test]
    fn cue_after_text_fires_after_text_playback_completes() {
        let compiled = compile(&[
            Instruction::Text("ab".into()),
            Instruction::Surface(SurfaceArg::new("7".into())),
        ]);
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        assert_eq!(cues.len(), 2, "内容は Text と Emote の 2 件");
        let d = text_playback_duration("ab"); // 2 char × 50ms = 0.1s
        // [0] Text("ab")@0.0（duration=D）。
        match command_of(&cues[0]) {
            CueCommand::Text(s) => assert_eq!(s, "ab"),
            other => panic!("expected Text, got {other:?}"),
        }
        assert_eq!(cues[0].start_time, 0.0);
        assert_eq!(cues[0].duration, d);
        // [1] Emote@D（テキスト再生完了後に発火・R4.2）。
        match command_of(&cues[1]) {
            CueCommand::Emote { key } => assert_eq!(key, "7"),
            other => panic!("expected Emote, got {other:?}"),
        }
        assert_eq!(
            cues[1].start_time, d,
            "後続 cue はテキスト再生完了後（text_start + D）へ焼き込まれる（R4.2）"
        );
        assert_eq!(cues[1].duration, 0.0, "Emote は瞬時（duration=0）");
    }

    /// 明示ウェイトは offset へ吸収して消すのでなく、action を持たず duration のみを持つ第一級
    /// `CueCommand::Wait` cue として台本に残る（R5.1）。かつ後続 Text はその待ち時間分だけ遅れる
    /// （`D_text1 + wait`・R4.4）。
    #[test]
    fn explicit_wait_between_texts_is_first_class_and_delays_following_text() {
        let compiled = compile(&[
            Instruction::Text("aa".into()),
            Instruction::Wait(Duration::from_millis(500)),
            Instruction::Text("bb".into()),
        ]);
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        assert_eq!(cues.len(), 3, "内容は Text / Wait / Text の 3 件");
        let d1 = text_playback_duration("aa"); // 0.1s
        let w = Duration::from_millis(500).as_secs_f64(); // 0.5s
        // [1] 中間に第一級 Wait cue（吸収されず台本に残る）。
        assert_eq!(
            command_of(&cues[1]),
            &CueCommand::Wait,
            "明示ウェイトは第一級 Wait cue として残る（吸収しない・R5.1）"
        );
        assert_eq!(
            cues[1].start_time, d1,
            "Wait cue はテキスト再生完了後に置かれる"
        );
        assert_eq!(
            cues[1].duration, w,
            "待ち時間は envelope duration が担う（action なし）"
        );
        // [2] 2 つ目 Text は D_text1 + wait 分だけ遅れる（R4.4）。
        match command_of(&cues[2]) {
            CueCommand::Text(s) => assert_eq!(s, "bb"),
            other => panic!("expected Text, got {other:?}"),
        }
        let t_b = d1 + w;
        assert_eq!(
            cues[2].start_time, t_b,
            "後続 Text は暗黙 D と明示待ちの累積分だけ遅れる（R4.4）"
        );
    }

    /// 末尾（単独）の明示ウェイトも offset へ吸収されず Wait cue として台本に残り、台本のみから
    /// talk の全時間範囲（`max(start_time + duration)`）が復元可能である（自己完結した楽譜・R5.3）。
    #[test]
    fn trailing_explicit_wait_remains_in_sheet_and_extends_extent() {
        let compiled = compile(&[
            Instruction::Text("abc".into()),
            Instruction::Wait(Duration::from_millis(800)),
        ]);
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        assert_eq!(cues.len(), 2, "内容は Text と末尾 Wait の 2 件");
        let d = text_playback_duration("abc"); // 0.15s
        let w = Duration::from_millis(800).as_secs_f64(); // 0.8s
        // 末尾 Wait cue が吸収されず台本に残る（R5.3）。
        assert_eq!(
            command_of(&cues[1]),
            &CueCommand::Wait,
            "末尾の明示ウェイトも第一級 Wait cue として台本に残る（R5.3）"
        );
        assert_eq!(cues[1].start_time, d);
        assert_eq!(cues[1].duration, w);
        // 台本のみから talk 全時間範囲が復元可能（absolute_start_time 未刻印は 0.0 起点）。
        assert_eq!(
            compiled.sheet.absolute_end_time(),
            d + w,
            "末尾待ちを含めた全時間範囲が台本のみから復元可能（R5.3）"
        );
    }

    /// 内容を持つ台本の先頭へ ClearAll を単一前置する（#6・R6.1/6.2）。書き込むスコープ数に
    /// 依らず ClearAll はちょうど 1 件（compile は残存スコープを列挙できないため全消し 1 件で表現）。
    #[test]
    fn clear_all_is_prepended_once_regardless_of_scope_count() {
        // 3 スコープへ書き込む talk。
        let compiled = compile(&[
            Instruction::SpeakerScope { n: 0 },
            Instruction::Text("a".into()),
            Instruction::SpeakerScope { n: 1 },
            Instruction::Text("b".into()),
            Instruction::SpeakerScope { n: 2 },
            Instruction::Text("c".into()),
        ]);
        let cues = compiled.sheet.cues();
        // 先頭に単一 ClearAll@0.0/duration0。
        assert_eq!(
            command_of(&cues[0]),
            &CueCommand::ClearAll,
            "先頭 cue は ClearAll"
        );
        assert_eq!(cues[0].start_time, 0.0);
        assert_eq!(cues[0].duration, 0.0);
        // スコープ数（ここでは 3）に依らず ClearAll はちょうど 1 件。
        let clear_all_count = cues
            .iter()
            .filter(|c| command_of(c) == &CueCommand::ClearAll)
            .count();
        assert_eq!(
            clear_all_count, 1,
            "ClearAll はスコープ数に依らず単一前置（6.1/6.2）"
        );
    }

    /// 内容 cue を生成しない台本（リテラル空・裸の終端・無視タグのみ）は空 sheet のままで
    /// ClearAll を前置しない——drive の `is_empty()` 即時 TalkDone 契約を保ち、ドライブ配線
    /// （task 7.x）を本 task で先取りしない（消すべき前 talk のテキストも存在しない）。
    #[test]
    fn empty_content_talk_gets_no_clear_all_prepended() {
        assert!(
            compile(&[]).sheet.is_empty(),
            "リテラル空 script は空 sheet（ClearAll 前置なし）"
        );
        assert!(
            compile(&[Instruction::Quit]).sheet.is_empty(),
            "先行 cue のない裸 Quit は空 sheet（ClearAll 前置なし）"
        );
        assert!(
            compile(&[
                Instruction::SystemVar("username".into()),
                Instruction::Raw("\\0".into()),
            ])
            .sheet
            .is_empty(),
            "無視タグのみの script は空 sheet（ClearAll 前置なし）"
        );
    }
}
