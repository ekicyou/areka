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

use crate::contract::{ActorKey, BarrierKind, Cue, CueCommand, CuePayload, CueSheet, TalkEndReason};
use crate::sysvar::SystemVarSnapshot;
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
///
/// `vars` は ⓪ghost が talk 開始時に手渡す名前→値の凍結スナップショット（`%username` 等の
/// システム変数展開の値源・R7）。本関数は参照するのみで OS 環境・SHIORI・永続化層を直接
/// 読まない（純粋・決定論を保つ）。SystemVar アーム（task 4.2）が `resolve_system_var` で
/// これを純粋展開し、展開文字列を Text cue へ写像する。
pub fn compile(instructions: &[Instruction], vars: &SystemVarSnapshot) -> CompiledTalk {
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
            // 選択肢 `\q[disp,target,refs...]`（→Balloon・R1.1/1.2/1.4/1.5/1.6）。
            // `id = target`（選択 ID・第 2 引数）・`text = disp`（表示ラベル・第 1 引数）・
            // `references`（第 3 引数以降）を欠落なく不透明転写する（ID 解釈・整数化なし）。
            // 台本内順序＝記述順（emit が現在 scope・offset を転写・瞬時 duration 0）。
            // 選択待ち barrier（choice ⩾1 の台本へ最終 offset に 1 個）は走査終了後に append する
            // （下記 `has_choice` 分岐・R2.1/2.2）。
            Instruction::Choice(choice) => {
                cues.push(emit(
                    scope,
                    offset,
                    0.0,
                    CueCommand::Choice {
                        id: choice.target.clone(),
                        text: choice.disp.clone(),
                        references: choice.references.clone(),
                    },
                ));
            }
            // カーソル絶対位置 `\_l[x,y]`（→Balloon・R3.1/3.3/3.4/3.5）。x・y は記述通りの
            // 不透明文字列で転写する（単位付き `5em`/`2lh`・裸数値・`@` 相対・空の区別を失わない・
            // 単位換算/座標解決は消費側の責務）。双方が空でも無条件に発行する（記述の存在を台本から
            // 失わせない・R3.5）。emit が現在 scope・offset を転写・瞬時 duration 0。
            Instruction::Cursor { x, y } => {
                cues.push(emit(
                    scope,
                    offset,
                    0.0,
                    CueCommand::Cursor {
                        x: x.clone(),
                        y: y.clone(),
                    },
                ));
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
            // キャラ移動 `\![move,...]`（→汎用キャリア・R4.1/4.2）。`\!` 名前空間全体を単一の
            // 不透明汎用 cue へ写像し typed variant を新設しない（name は暗黙 "move"＝parser が
            // 種別で分離済みの転記を戻すだけ）。引数（空トークン＝省略スロット含む）を欠落なく
            // 記述順のまま保持する。意味割当（座標・基準点）は消費側 ghost の責務。瞬時（duration 0）。
            Instruction::Move(move_args) => {
                cues.push(emit(
                    scope,
                    offset,
                    0.0,
                    CueCommand::command_carrier("move", move_args.args.clone()),
                ));
            }
            // 汎用 `\!` コマンド（move 以外・→汎用キャリア・R4.1/4.2/8.2）。name＋raw_args を
            // 解釈せず同一キャリアへ載せる（`\![*]` 単独形＝raw_args 空も第一級で台本に載る＝
            // R8.2 卒業）。空トークン・`--key=value` 形も素通し。消費はコマンド名レベル選別（R4.5）。瞬時。
            Instruction::GenericCommand { name, raw_args } => {
                cues.push(emit(
                    scope,
                    offset,
                    0.0,
                    CueCommand::command_carrier(name.clone(), raw_args.clone()),
                ));
            }
            // システム変数 `%username` 等（→Text・R7.1/7.2/7.4/7.5）。値源は所有せず、talk 起動時
            // 手渡しの凍結スナップショット `vars` を純粋展開（`resolve_system_var`・no I/O）。展開値
            // （スナップショット値／既定値／未対応名の素通し `%名前`）はいずれも通常テキストと同格の
            // Text cue へ写像し、`duration = text_playback_duration(展開文字列)` を焼き込み offset += D
            // で後続を整列する。独立 cue とし隣接 Text と併合しない（純粋走査・観測同一）。
            Instruction::SystemVar(name) => {
                let expanded = match crate::sysvar::resolve_system_var(name, vars) {
                    crate::sysvar::ResolvedVar::Text(s) => s,
                    crate::sysvar::ResolvedVar::PassThrough(s) => s,
                };
                let d = crate::duration::text_playback_duration(&expanded);
                cues.push(emit(scope, offset, d, CueCommand::Text(expanded)));
                offset += d;
            }
            // 残る除外（`Raw` および `#[non_exhaustive]` の未知 variant）は無視ログを記録し cue を
            // 生成せず継続する（寛容・非 panic・型シーム・R8.2/8.3/R11.2）。Choice/Cursor は task 4.1、
            // Move/GenericCommand/SystemVar は task 4.2 で専用アームへ卒業したため、catch-all が無視
            // する除外集合は Raw＋未知 variant のみへ縮小済み（`Instruction` は別 crate の
            // `#[non_exhaustive]` ゆえ本 catch-all は構造上必須・未知 variant は防御経路）。Raw-only
            // 化の明文檻は `catch_all_ignored_set_is_raw_only`（task 4.3・R8.3）。
            other => {
                tracing::debug!(instruction = ?other, "M-boot 外タグを無視");
            }
        }
    }

    // 選択待ち barrier 発行（R2.1/2.2/2.5/2.6）: 走査終了（End/Quit 切詰め後の出力）に対し、choice
    // cue が 1 個以上あれば選択待ち barrier `WaitForChoice{timeout:None}`（M1 無期限）を最終 offset へ
    // ちょうど 1 個 append する。同一 at の FIFO 挿入により全 cue より後に配送される（全 choice cue の
    // 後・R2.2）。`\q` の無い台本は barrier を発行せず既存完了挙動を変えない（R2.5）。barrier は
    // presentation でなく `emit`（CueCommand 専用）とは別の Barrier 用発行ヘルパで組む。
    let has_choice = cues.iter().any(|cue| {
        matches!(
            &cue.payload,
            CuePayload::Command(CueCommand::Choice { .. })
        )
    });
    if has_choice {
        cues.push(emit_barrier(
            scope,
            offset,
            BarrierKind::WaitForChoice { timeout: None },
        ));
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

/// 現在 scope・発火 offset・バリア種別から 1 発火 [`Cue`]（`CuePayload::Barrier`）を構築する。
///
/// `emit`（`CueCommand` 専用）とは別の Barrier 用発行口。barrier は presentation でなく占有時間を
/// 持たないため `duration=0.0` 固定（envelope としては一律にフィールドを持ち値は 0）。`start_time`
/// は呼び出し側が最終 offset を渡す（全 choice cue より後・同一 at の FIFO で末尾配送・R2.2）。
/// `actor` は発行時点の scope の転写（barrier は演者振分の対象外だが emit と同じ scope 規律に従う）。
fn emit_barrier(scope: u32, offset: f64, kind: BarrierKind) -> Cue {
    Cue {
        actor: ActorKey::from(scope.to_string()),
        start_time: offset,
        payload: CuePayload::Barrier(kind),
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
    use crate::duration::text_playback_duration;
    use crate::sysvar::SystemVarSnapshot;
    use areka_parsers::sakura::{NewLineRatio, SurfaceArg};
    use std::time::Duration;

    /// task 4.1 で `compile` 署名が `(instructions, vars)` へ変わった。既存檻は sysvar を
    /// 観測しない（値源展開＝SystemVar アームは task 4.2 の領分）ため、空スナップショットを
    /// 渡すテスト用の薄いブリッジで機械的に追随する。`use super::*` の glob import を
    /// 明示定義が shadow するため、既存の `compile(&[...])` 呼び出しは無改変のまま本 1 引数
    /// ヘルパへ解決される。実スナップショットを渡す新檻は `super::compile(.., &snap)` を直呼びする。
    fn compile(instructions: &[Instruction]) -> CompiledTalk {
        super::compile(instructions, &SystemVarSnapshot::default())
    }

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
        // ClearAll は単一前置（内容側に重複しない・スコープ数に依らず 1 件）。barrier/routing の
        // 非 Command cue（choice 台本の末尾 barrier 等）は ClearAll ではあり得ないため走査から除く。
        assert!(
            cues[1..].iter().all(|c| !matches!(
                &c.payload,
                CuePayload::Command(CueCommand::ClearAll)
            )),
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

    /// 除外集合の Raw-only 対置換（catch-all の意図的更新の檻・R8.2/8.3・Testing Strategy 項目 6）。
    ///
    /// task 4.1（Choice/Cursor）・task 4.2（Move/GenericCommand/SystemVar）で 5 語彙が専用アームへ
    /// **卒業**した結果、compile の catch-all が無視する除外集合は `Raw`＋`#[non_exhaustive]` の未知
    /// variant のみへ縮小された（`Instruction` は別 crate の `#[non_exhaustive]` ゆえ catch-all は
    /// 構造上必須・未知 variant は防御経路だが workspace 内に合成手段がなく直接は載せられない）。
    /// 本檻はその縮小後の除外集合を **Raw-only** で固定し、4.1/4.2 の卒業と決して矛盾させない:
    /// (1) `Raw` は cue を 0 個生成する（無視ログ・非 panic・従来挙動維持・R8.2）、
    /// (2) 卒業した 5 語彙（Choice / Cursor / Move / GenericCommand / SystemVar）は除外集合に
    ///     **含まれない**＝各々 cue を生成する（本檻は決してこれらが「無視される」とは主張しない・
    ///     R8.3）。各語彙の写像詳細は個別 behavioral 檻が担い、ここでは除外集合の境界のみを固定する。
    #[test]
    fn catch_all_ignored_set_is_raw_only() {
        use areka_parsers::sakura::{Choice, MoveArgs};

        // (1) Raw は除外集合ゆえ 0 cue。中途に Raw を挟んでも内容は後続 Text のみ（ClearAll 前置）。
        let compiled = compile(&[
            Instruction::Raw("\\?".into()),
            Instruction::Text("hi".into()),
        ]);
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        assert_eq!(cues.len(), 1, "Raw は cue を生成しない（除外集合は Raw のみ）");
        match command_of(&cues[0]) {
            CueCommand::Text(s) => assert_eq!(s, "hi"),
            other => panic!("expected Text, got {other:?}"),
        }
        // Raw 単独（後続なし）は空 sheet（内容 cue ゼロゆえ ClearAll 前置もなし・R8.2）。
        assert!(
            compile(&[Instruction::Raw("\\0".into())]).sheet.is_empty(),
            "Raw のみの台本は空 sheet（除外集合の 0 cue を単独でも固定）"
        );

        // (2) 卒業した 5 語彙は除外集合の外＝各々 cue を生成する（sheet 非空）。ここでこれらが
        //     「無視される」と主張することは 4.1/4.2 と矛盾するため、本檻は生成側のみを固定する
        //     （R8.3 対置換）。詳細写像は個別 behavioral 檻（`*_maps_to_*` 等）が担う。
        let graduated: [Instruction; 5] = [
            Instruction::Choice(Choice {
                disp: "はい".into(),
                target: "OnYes".into(),
                references: vec![],
            }),
            Instruction::Cursor {
                x: "5em".into(),
                y: "2lh".into(),
            },
            Instruction::Move(MoveArgs {
                args: vec!["100".into()],
            }),
            Instruction::GenericCommand {
                name: "raise".into(),
                raw_args: vec!["OnBoot".into()],
            },
            Instruction::SystemVar("username".into()),
        ];
        for instruction in graduated {
            assert!(
                !compile(std::slice::from_ref(&instruction)).sheet.is_empty(),
                "卒業語彙は除外集合に含まれず cue を生成する（4.1/4.2 の卒業を明示）: {instruction:?}"
            );
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

    /// task 4.2 で Move/SystemVar/GenericCommand は専用アームへ卒業し cue を発行する（R4.1/7.1/8.2）。
    /// catch-all に残る除外は `Raw`（＋未知 variant）のみで、これは従来どおり 0 cue・非 panic
    /// （R8.2/8.3/11.2）。卒業アームと Raw が交錯しても各々の写像・破棄が一貫することを固定する。
    /// 各アームの詳細写像は個別の behavioral 檻（`move_maps_to_command_carrier_*` 他）が担う。
    #[test]
    fn graduated_arms_emit_while_raw_remains_ignored() {
        use areka_parsers::sakura::MoveArgs;

        let compiled = compile(&[
            Instruction::Move(MoveArgs {
                args: vec!["100".into(), "200".into()],
            }),
            Instruction::SystemVar("username".into()),
            Instruction::GenericCommand {
                name: "raise".into(),
                raw_args: vec!["OnBoot".into()],
            },
            Instruction::Raw("\\?".into()),
            Instruction::Text("tail".into()),
        ]);
        // 内容: carrier(move) / Text(username 既定値) / carrier(raise) / Text(tail) の 4 件。
        // Raw のみ破棄される（冒頭に ClearAll が前置される）。
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        assert_eq!(cues.len(), 4, "卒業アーム 3 種＋末尾 Text の 4 cue（Raw のみ破棄）");
        assert_eq!(
            command_of(&cues[0]).as_command_carrier(),
            Some(("move", vec!["100", "200"])),
            "Move はキャリア cue へ卒業（R4.1）"
        );
        match command_of(&cues[1]) {
            CueCommand::Text(s) => assert_eq!(s, "ユーザーさん", "SystemVar は Text cue へ卒業（R7.1/7.4）"),
            other => panic!("expected Text, got {other:?}"),
        }
        assert_eq!(
            command_of(&cues[2]).as_command_carrier(),
            Some(("raise", vec!["OnBoot"])),
            "GenericCommand はキャリア cue へ卒業（R4.1/8.2）"
        );
        match command_of(&cues[3]) {
            CueCommand::Text(s) => assert_eq!(s, "tail", "Raw は破棄され末尾 Text のみ残る"),
            other => panic!("expected Text(\"tail\"), got {other:?}"),
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
                Instruction::Raw("\\0".into()),
                Instruction::Raw("\\1".into()),
            ])
            .sheet
            .is_empty(),
            "破棄される Raw のみの script は空 sheet（ClearAll 前置なし）"
        );
    }

    // ── task 4.1: Choice/Cursor アーム写像の behavioral 檻 ──

    /// fixture メインメニュー script 断片（`menu.pasta:15` 相当）を直入力し、`\q`/`\_l` が
    /// 期待どおりの Choice/Cursor cue へ写像されることを固定する（R1.1/1.2/1.5/1.6・R3.1/3.4）。
    ///
    /// 断片（さくらスクリプト）:
    /// `\q[おしゃべり頻度,Onおしゃべり頻度メニュー]\n\q[エモの位置調整,Onエモの位置調整メニュー]\_l[5em,2lh]\q[閉じる,Onメニュー閉じる]`
    /// を parse 済み `Instruction` 列として直入力する（scope="1"＝エモ）。
    ///
    /// 検証: (1) Choice cue は `id=target`（第 2 引数）・`text=disp`（第 1 引数）を欠落なく持つ、
    /// (2) Cursor cue が `\_l[5em,2lh]` から `x="5em"`/`y="2lh"` で発行される、(3) 記述順が
    /// 台本内順序として保存される（Choice/NewLine/Choice/Cursor/Choice が交互のまま並ぶ）、
    /// (4) 各 cue が現在 scope "1" へ帰属し瞬時（duration 0）。barrier（task 4.2）・完全 at 檻
    /// （task 4.4）はここでは主張しない。
    #[test]
    fn choice_and_cursor_arms_map_menu_fragment_in_description_order() {
        use areka_parsers::sakura::Choice;

        let compiled = compile(&[
            Instruction::SpeakerScope { n: 1 },
            Instruction::Choice(Choice {
                disp: "おしゃべり頻度".into(),
                target: "Onおしゃべり頻度メニュー".into(),
                references: vec![],
            }),
            Instruction::NewLine(NewLineRatio::new(1.0)),
            Instruction::Choice(Choice {
                disp: "エモの位置調整".into(),
                target: "Onエモの位置調整メニュー".into(),
                references: vec![],
            }),
            Instruction::Cursor {
                x: "5em".into(),
                y: "2lh".into(),
            },
            Instruction::Choice(Choice {
                disp: "閉じる".into(),
                target: "Onメニュー閉じる".into(),
                references: vec![],
            }),
        ]);
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        // 内容は Choice / NewLine / Choice / Cursor / Choice の 5 件（記述順保存）＋末尾に選択待ち
        // barrier 1 件（`\q` を含む台本ゆえ task 4.2 で発行・R2.1/2.2）。
        assert_eq!(cues.len(), 6, "内容 cue 5 件＋末尾 barrier 1 件");

        // [0] Choice(頻度): id=target・text=disp。
        assert_eq!(
            command_of(&cues[0]),
            &CueCommand::Choice {
                id: "Onおしゃべり頻度メニュー".into(),
                text: "おしゃべり頻度".into(),
                references: vec![],
            },
            "Choice は id=target・text=disp で写像される（R1.2）"
        );
        // [1] NewLine（既存写像・記述順の中間に保存）。
        match command_of(&cues[1]) {
            CueCommand::NewLine { ratio } => assert_eq!(*ratio, 1.0_f32),
            other => panic!("expected NewLine, got {other:?}"),
        }
        // [2] Choice(位置調整)。
        assert_eq!(
            command_of(&cues[2]),
            &CueCommand::Choice {
                id: "Onエモの位置調整メニュー".into(),
                text: "エモの位置調整".into(),
                references: vec![],
            }
        );
        // [3] Cursor(5em,2lh): 不透明転写（単位付きの区別を失わない・R3.1）。
        assert_eq!(
            command_of(&cues[3]),
            &CueCommand::Cursor {
                x: "5em".into(),
                y: "2lh".into(),
            },
            "Cursor は x/y を不透明転写する（R3.1）"
        );
        // [4] Choice(閉じる)。
        assert_eq!(
            command_of(&cues[4]),
            &CueCommand::Choice {
                id: "Onメニュー閉じる".into(),
                text: "閉じる".into(),
                references: vec![],
            }
        );
        // [5] 末尾に選択待ち barrier（全 choice cue より後・R2.2）。
        assert_eq!(
            barrier_of(&cues[5]),
            &BarrierKind::WaitForChoice { timeout: None },
            "`\\q` を含む台本の末尾に選択待ち barrier（R2.1/2.2）"
        );

        // 全 cue が現在 scope "1" へ帰属し瞬時（duration 0）。barrier も末尾 scope 1・duration 0。
        for (i, cue) in cues.iter().enumerate() {
            assert_eq!(cue.actor.as_str(), "1", "index {i} は scope 1 帰属（R3.4）");
            assert_eq!(cue.duration, 0.0, "index {i} は瞬時（duration 0）");
        }
    }

    /// `\_l[,]` 相当（x・y 双方空）でも Cursor cue が無条件に発行される（記述の存在を台本から
    /// 失わせない・R3.5）。空は代表形と別物（区別保持）。
    #[test]
    fn cursor_double_empty_still_emits() {
        let compiled = compile(&[Instruction::Cursor {
            x: "".into(),
            y: "".into(),
        }]);
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        assert_eq!(cues.len(), 1, "双方空でも Cursor cue は発行される（R3.5）");
        assert_eq!(
            command_of(&cues[0]),
            &CueCommand::Cursor {
                x: "".into(),
                y: "".into(),
            },
            "双方空の Cursor も欠落させず発行（区別保持・R3.5）"
        );
        assert_eq!(cues[0].duration, 0.0, "Cursor は瞬時（duration 0）");
    }

    // ── task 4.2: Move/GenericCommand/SystemVar アーム＋barrier 発行の behavioral 檻 ──

    /// `Cue::payload` から `BarrierKind` を取り出すヘルパ（barrier 檻用）。
    fn barrier_of(cue: &Cue) -> &BarrierKind {
        match &cue.payload {
            CuePayload::Barrier(kind) => kind,
            other => panic!("expected CuePayload::Barrier, got {other:?}"),
        }
    }

    /// `Move(MoveArgs)` は `command_carrier("move", args)` へ写像される（`\!` 全体が第一級で
    /// 台本に載る・R4.1/4.2）。空トークン（省略スロット）も欠落なく保持される。瞬時（duration 0）。
    #[test]
    fn move_maps_to_command_carrier_preserving_empty_tokens() {
        use areka_parsers::sakura::MoveArgs;

        // fixture の `\![move,-353,,,0,base,base]` 相当（空トークン 2 個を保つ 6 トークン）。
        let compiled = compile(&[Instruction::Move(MoveArgs {
            args: vec![
                "-353".into(),
                "".into(),
                "".into(),
                "0".into(),
                "base".into(),
                "base".into(),
            ],
        })]);
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        assert_eq!(cues.len(), 1, "move は単一の汎用キャリア cue を生成する");
        assert_eq!(
            command_of(&cues[0]).as_command_carrier(),
            Some((
                "move",
                vec!["-353", "", "", "0", "base", "base"]
            )),
            "Move → command_carrier(\"move\", args)（空トークン保持・R4.1/4.2）"
        );
        assert_eq!(cues[0].duration, 0.0, "汎用キャリアは瞬時（duration 0）");
    }

    /// `GenericCommand{name,raw_args}` は `command_carrier(name, raw_args)` へ写像される（R4.1/4.2）。
    /// 引数を持つ形も、`\![*]` 単独形（raw_args 空）も台本に第一級で載る（R8.2 卒業）。
    #[test]
    fn generic_command_maps_to_command_carrier_including_bare_form() {
        // 引数付き `\![raise,OnBoot]`。
        let compiled = compile(&[Instruction::GenericCommand {
            name: "raise".into(),
            raw_args: vec!["OnBoot".into()],
        }]);
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        assert_eq!(cues.len(), 1);
        assert_eq!(
            command_of(&cues[0]).as_command_carrier(),
            Some(("raise", vec!["OnBoot"])),
            "GenericCommand → command_carrier(name, raw_args)（R4.1）"
        );
        assert_eq!(cues[0].duration, 0.0, "瞬時（duration 0）");

        // 単独形 `\![vanish]`（raw_args 空）でもキャリア cue が発行される（R8.2 卒業）。
        let compiled = compile(&[Instruction::GenericCommand {
            name: "vanish".into(),
            raw_args: vec![],
        }]);
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        assert_eq!(cues.len(), 1, "単独形 `\\![*]` もキャリア cue を生成する");
        assert_eq!(
            command_of(&cues[0]).as_command_carrier(),
            Some(("vanish", vec![])),
            "単独形は空トークンのキャリアとして載る（R8.2）"
        );
    }

    /// `SystemVar(name)` は値ありスナップショットの値を `Text` cue へ写像する（R7.1/7.2）。
    /// `duration = text_playback_duration(展開文字列)`・直後 offset += D で後続 cue を整列する。
    /// 展開結果は通常テキストと同格（独立 cue・隣接 Text と併合しない）。
    #[test]
    fn system_var_present_maps_to_text_cue_with_playback_duration() {
        let mut vars = SystemVarSnapshot::default();
        vars.insert("username", "アヒル");
        // `[SystemVar("username"), Surface("1")]`: 展開 Text 後に Emote が D 分遅れて発火する。
        let compiled = super::compile(
            &[
                Instruction::SystemVar("username".into()),
                Instruction::Surface(SurfaceArg::new("1".into())),
            ],
            &vars,
        );
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        assert_eq!(cues.len(), 2, "展開 Text と Emote の 2 件");
        let d = text_playback_duration("アヒル"); // 3 char × 50ms
        // [0] 展開値の Text cue（通常テキスト同格・R7.2）。
        match command_of(&cues[0]) {
            CueCommand::Text(s) => assert_eq!(s, "アヒル", "スナップショット値へ展開（R7.1）"),
            other => panic!("expected Text, got {other:?}"),
        }
        assert_eq!(cues[0].start_time, 0.0);
        assert_eq!(
            cues[0].duration, d,
            "duration = text_playback_duration(展開文字列)（R7.2）"
        );
        // [1] 後続 Emote は展開テキスト再生完了後（offset += D）へ整列する。
        assert_eq!(
            command_of(&cues[1]),
            &CueCommand::Emote { key: "1".into() }
        );
        assert_eq!(cues[1].start_time, d, "SystemVar 由来 D の分だけ後続が遅れる（R7.2）");
    }

    /// `username` 欠落スナップショット → 既定値 `ユーザーさん` の Text cue（R7.4・生の `%username`
    /// を露出しない）。既定 snapshot（値なし）を渡す薄いブリッジ経由で確認する。
    #[test]
    fn system_var_missing_username_expands_to_default_text() {
        let compiled = compile(&[Instruction::SystemVar("username".into())]);
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        assert_eq!(cues.len(), 1);
        match command_of(&cues[0]) {
            CueCommand::Text(s) => assert_eq!(s, "ユーザーさん", "既定値へ展開（R7.4）"),
            other => panic!("expected Text, got {other:?}"),
        }
        assert_eq!(
            cues[0].duration,
            text_playback_duration("ユーザーさん"),
            "既定値も通常テキストと同一の再生時間規則（R7.2）"
        );
    }

    /// M1 未対応のシステム変数名 → 元の `%名前` を Text として素通し出力（R7.5・情報を失わない縮退）。
    #[test]
    fn system_var_unsupported_passes_through_as_text() {
        let compiled = compile(&[Instruction::SystemVar("selfname".into())]);
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        assert_eq!(cues.len(), 1);
        match command_of(&cues[0]) {
            CueCommand::Text(s) => assert_eq!(s, "%selfname", "元の `%名前` を素通し（R7.5）"),
            other => panic!("expected Text, got {other:?}"),
        }
    }

    /// `\q` を 1 個以上含む台本には選択待ち barrier をちょうど 1 個、全 choice cue より後（最終
    /// offset）へ発行する（R2.1/2.2/2.6）。タイムアウトは指定しない（`timeout:None`・M1 無期限）。
    #[test]
    fn talk_with_choice_appends_single_barrier_after_all_choices() {
        use areka_parsers::sakura::Choice;

        let compiled = compile(&[
            Instruction::Choice(Choice {
                disp: "はい".into(),
                target: "OnYes".into(),
                references: vec![],
            }),
            Instruction::Choice(Choice {
                disp: "いいえ".into(),
                target: "OnNo".into(),
                references: vec![],
            }),
        ]);
        // 内容: Choice / Choice / Barrier の 3 件（barrier は末尾）。
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        assert_eq!(cues.len(), 3, "choice 2 個＋barrier 1 個");
        // barrier はちょうど 1 個（R2.1）。
        let barrier_count = cues
            .iter()
            .filter(|c| matches!(c.payload, CuePayload::Barrier(_)))
            .count();
        assert_eq!(barrier_count, 1, "選択待ち barrier はちょうど 1 個（R2.1）");
        // barrier は全 choice cue より後（R2.2）。
        assert!(
            matches!(command_of(&cues[0]), CueCommand::Choice { .. }),
            "[0] は choice"
        );
        assert!(
            matches!(command_of(&cues[1]), CueCommand::Choice { .. }),
            "[1] は choice"
        );
        assert_eq!(
            barrier_of(&cues[2]),
            &BarrierKind::WaitForChoice { timeout: None },
            "barrier は末尾・WaitForChoice{{timeout:None}}（R2.2/2.6）"
        );
        // barrier は最終 offset（choice が瞬時なので 0.0）・duration 0.0。
        assert_eq!(cues[2].start_time, 0.0, "barrier は最終 offset（R2.2）");
        assert_eq!(cues[2].duration, 0.0, "barrier の duration は 0.0");
    }

    /// テキスト先行のメニューでも barrier は全 choice cue より後（テキスト再生完了後の最終 offset）へ
    /// 置かれる（R2.2）。`Text("ab") \q[..]` → barrier は text D の位置。
    #[test]
    fn barrier_is_placed_at_final_offset_after_leading_text() {
        use areka_parsers::sakura::Choice;

        let compiled = compile(&[
            Instruction::Text("ab".into()),
            Instruction::Choice(Choice {
                disp: "はい".into(),
                target: "OnYes".into(),
                references: vec![],
            }),
        ]);
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        // Text / Choice / Barrier の 3 件。
        assert_eq!(cues.len(), 3);
        let d = text_playback_duration("ab"); // 0.1s
        // choice は text 再生完了後に発行され（start_time=D）、barrier はさらにその後（最終 offset=D）。
        assert!(matches!(command_of(&cues[0]), CueCommand::Text(_)));
        assert!(matches!(command_of(&cues[1]), CueCommand::Choice { .. }));
        assert_eq!(cues[1].start_time, d, "choice は text 再生完了後");
        assert_eq!(
            barrier_of(&cues[2]),
            &BarrierKind::WaitForChoice { timeout: None }
        );
        assert_eq!(
            cues[2].start_time, d,
            "barrier は最終 offset（choice と同一 at・FIFO で後）"
        );
    }

    /// `\q` を 1 つも含まない台本には barrier を発行しない（R2.5・既存完了挙動を変えない）。
    #[test]
    fn talk_without_choice_appends_no_barrier() {
        let compiled = compile(&[
            Instruction::Text("hi".into()),
            Instruction::Surface(SurfaceArg::new("1".into())),
        ]);
        let cues = compiled.sheet.cues();
        assert!(
            cues.iter()
                .all(|c| !matches!(c.payload, CuePayload::Barrier(_))),
            "`\\q` の無い台本は barrier を発行しない（R2.5）"
        );
    }

    /// `\q` の第 3 引数以降（references）を記述順を保って欠落なく運ぶ（R1.4）。
    #[test]
    fn choice_references_are_preserved_in_order() {
        use areka_parsers::sakura::Choice;

        let compiled = compile(&[Instruction::Choice(Choice {
            disp: "はい".into(),
            target: "OnYes".into(),
            references: vec!["r0".into(), "r1".into(), "".into()],
        })]);
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        // choice cue 1 件＋末尾に選択待ち barrier 1 件（`\q` を含むため・R2.1）。
        assert_eq!(cues.len(), 2, "choice cue 1 件＋末尾 barrier 1 件");
        assert_eq!(
            command_of(&cues[0]),
            &CueCommand::Choice {
                id: "OnYes".into(),
                text: "はい".into(),
                // 記述順・空トークンを含め欠落なく運ぶ（不透明・ID 解釈なし・R1.4）。
                references: vec!["r0".into(), "r1".into(), "".into()],
            },
            "references は記述順を保って欠落なく運ばれる（R1.4）"
        );
        assert_eq!(
            barrier_of(&cues[1]),
            &BarrierKind::WaitForChoice { timeout: None },
            "`\\q` を含む台本の末尾に選択待ち barrier（R2.1）"
        );
    }

    // ── task 4.4: compile 決定論檻（メニュー・キャリア・sysvar 展開の包括固定・R9.1/9.2/9.3/9.4）──

    /// fixture `menu.pasta:15` メインメニュー script を parse 済み `Instruction` 列として直入力し、
    /// **完全な順序付き cue 列**（冒頭 ClearAll 前置＋末尾 barrier を含む全 sheet）を期待ベクタと
    /// index ごとに突合する決定論檻（R9.1/9.2）。個別 behavioral 檻（task 4.1/4.2）が写像の各アームを
    /// 担うのに対し、本檻は「実 fixture 断片 → 全 sheet」の end-to-end 固定であり、
    /// - 順序（記述順＋冒頭 ClearAll＋末尾 barrier）、
    /// - `at`（全命令が瞬時ゆえ全 start_time が 0.0＝完全整列）、
    /// - `duration`（全 cue が瞬時 0.0）、
    /// - `scope`（内容は エモ scope "1" 帰属・冒頭 ClearAll のみ scope "0"）、
    /// - barrier の**唯一性**（ちょうど 1 個）と**最終位置**（末尾 index）、
    /// を一括で固定する。期待ベクタは production の `emit`/`emit_barrier` で組み、actor/duration の
    /// 転写規律を実装と同一に保つ（10 進直書きの表現差を排除）。
    #[test]
    fn menu_script_compiles_to_exact_ordered_cue_sheet() {
        use areka_parsers::sakura::Choice;

        // `\q[おしゃべり頻度,Onおしゃべり頻度メニュー]\n\q[エモの位置調整,Onエモの位置調整メニュー]`
        // `\_l[5em,2lh]\q[閉じる,Onメニュー閉じる]`（エモ発話＝scope "1"）を parse 済みとして直入力。
        let compiled = compile(&[
            Instruction::SpeakerScope { n: 1 },
            Instruction::Choice(Choice {
                disp: "おしゃべり頻度".into(),
                target: "Onおしゃべり頻度メニュー".into(),
                references: vec![],
            }),
            Instruction::NewLine(NewLineRatio::new(1.0)),
            Instruction::Choice(Choice {
                disp: "エモの位置調整".into(),
                target: "Onエモの位置調整メニュー".into(),
                references: vec![],
            }),
            Instruction::Cursor {
                x: "5em".into(),
                y: "2lh".into(),
            },
            Instruction::Choice(Choice {
                disp: "閉じる".into(),
                target: "Onメニュー閉じる".into(),
                references: vec![],
            }),
        ]);

        // 期待は全 sheet（冒頭 ClearAll＋内容 5 件＋末尾 barrier ＝ 7 件）。ClearAll のみ scope "0"
        // （production は `emit(0, ..)` で前置）、内容と barrier は現在 scope "1"。全 at=0.0/duration=0.0。
        let expected: Vec<Cue> = vec![
            emit(0, 0.0, 0.0, CueCommand::ClearAll),
            emit(
                1,
                0.0,
                0.0,
                CueCommand::Choice {
                    id: "Onおしゃべり頻度メニュー".into(),
                    text: "おしゃべり頻度".into(),
                    references: vec![],
                },
            ),
            emit(1, 0.0, 0.0, CueCommand::NewLine { ratio: 1.0 }),
            emit(
                1,
                0.0,
                0.0,
                CueCommand::Choice {
                    id: "Onエモの位置調整メニュー".into(),
                    text: "エモの位置調整".into(),
                    references: vec![],
                },
            ),
            emit(
                1,
                0.0,
                0.0,
                CueCommand::Cursor {
                    x: "5em".into(),
                    y: "2lh".into(),
                },
            ),
            emit(
                1,
                0.0,
                0.0,
                CueCommand::Choice {
                    id: "Onメニュー閉じる".into(),
                    text: "閉じる".into(),
                    references: vec![],
                },
            ),
            emit_barrier(1, 0.0, BarrierKind::WaitForChoice { timeout: None }),
        ];

        let cues = compiled.sheet.cues();
        assert_eq!(
            cues.len(),
            expected.len(),
            "全 sheet は ClearAll＋内容 5＋barrier ＝ 7 件（順序完全一致）"
        );
        for (i, (got, want)) in cues.iter().zip(expected.iter()).enumerate() {
            assert!(
                cue_eq(got, want),
                "index {i} の cue が期待と異なる: {got:?} != {want:?}"
            );
        }

        // at 整列: 全命令が瞬時ゆえ全 start_time は 0.0（完全整列・単調非減少の退化形）。
        for (i, cue) in cues.iter().enumerate() {
            assert_eq!(cue.start_time, 0.0, "index {i} の start_time は 0.0（at 整列）");
            assert_eq!(cue.duration, 0.0, "index {i} は瞬時（duration 0）");
        }

        // scope 帰属: 冒頭 ClearAll のみ "0"、内容＋barrier は エモ "1"。
        assert_eq!(cues[0].actor.as_str(), "0", "冒頭 ClearAll は scope 0");
        for (i, cue) in cues.iter().enumerate().skip(1) {
            assert_eq!(cue.actor.as_str(), "1", "index {i} は エモ scope 1 帰属（R3.4）");
        }

        // barrier の唯一性（ちょうど 1 個）と最終位置（末尾 index）。
        let barrier_positions: Vec<usize> = cues
            .iter()
            .enumerate()
            .filter(|(_, c)| matches!(c.payload, CuePayload::Barrier(_)))
            .map(|(i, _)| i)
            .collect();
        assert_eq!(
            barrier_positions,
            vec![cues.len() - 1],
            "選択待ち barrier はちょうど 1 個・末尾 index（唯一性＋最終位置・R2.1/2.2）"
        );
        assert_eq!(
            barrier_of(&cues[cues.len() - 1]),
            &BarrierKind::WaitForChoice { timeout: None },
            "末尾 barrier は WaitForChoice{{timeout:None}}（M1 無期限）"
        );
    }

    /// `\q` を 1 つも含まないメニュー系 script（NewLine/Cursor/Text の混在）には barrier を
    /// 一切発行しない（R2.5・既存完了挙動を変えない）。Cursor を含んでも「選択メニュー」でなければ
    /// barrier は出ないことを決定論的に固定する（Cursor 単独では barrier 条件を満たさない）。
    #[test]
    fn no_choice_menu_script_emits_no_barrier() {
        let compiled = compile(&[
            Instruction::SpeakerScope { n: 1 },
            Instruction::Text("メニューやで".into()),
            Instruction::NewLine(NewLineRatio::new(1.0)),
            Instruction::Cursor {
                x: "5em".into(),
                y: "2lh".into(),
            },
            Instruction::Text("また来てな".into()),
        ]);
        let cues = compiled.sheet.cues();
        assert!(
            cues.iter()
                .all(|c| !matches!(c.payload, CuePayload::Barrier(_))),
            "`\\q` の無い台本は Cursor を含んでも barrier を発行しない（R2.5）"
        );
    }

    /// fixture `menu.pasta:65` の `\1\![move,-353,,,0,base,base]` を parse 済みとして直入力し、
    /// `\1` の話者スコープが汎用キャリア cue へ actor "1" として転写されること、`\![move,..]` が
    /// **6 トークン（空 2 個保持）**の `move` キャリアへ写像されることを一括固定する（R9.3・R4.1/4.2/5）。
    /// 既存の `move_maps_to_command_carrier_preserving_empty_tokens` は既定 scope 0 での写像を固定する
    /// のに対し、本檻は `\1` プレフィックスの scope 帰属を明示する（fixture 忠実な end-to-end）。
    #[test]
    fn scoped_move_carrier_transcribes_scope_and_preserves_empty_tokens() {
        use areka_parsers::sakura::MoveArgs;

        let compiled = compile(&[
            Instruction::SpeakerScope { n: 1 },
            Instruction::Move(MoveArgs {
                args: vec![
                    "-353".into(),
                    "".into(),
                    "".into(),
                    "0".into(),
                    "base".into(),
                    "base".into(),
                ],
            }),
        ]);
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        assert_eq!(cues.len(), 1, "内容は move キャリア cue 1 件のみ（barrier なし）");
        assert_eq!(
            command_of(&cues[0]).as_command_carrier(),
            Some(("move", vec!["-353", "", "", "0", "base", "base"])),
            "`\\![move,..]` → command_carrier(\"move\", 6 トークン・空 2 個保持・R9.3/4.1/4.2)"
        );
        assert_eq!(
            cues[0].actor.as_str(),
            "1",
            "`\\1` の話者スコープがキャリア cue へ actor \"1\" として転写される（R5）"
        );
        assert_eq!(cues[0].duration, 0.0, "汎用キャリアは瞬時（duration 0）");
        // move は `\q` でないため barrier を伴わない（R2.5）。
        assert!(
            compiled
                .sheet
                .cues()
                .iter()
                .all(|c| !matches!(c.payload, CuePayload::Barrier(_))),
            "キャリアのみの台本は barrier を発行しない（R2.5）"
        );
    }

    /// 未知名 `\![raise,OnBoot]`・単独形 `\![*]`（bare・raw_args 空）のいずれも汎用キャリア cue として
    /// 発行され**無音落ちしない**ことを一括固定する（R9.3・R8.2 卒業）。compile は `\!` 名前空間を
    /// typed variant 化せず単一の不透明キャリアへ載せるため、未知名でも既知名でも扱いは同型
    /// （name 選別は消費側の責務）。
    #[test]
    fn unknown_and_bare_carrier_forms_are_emitted_not_dropped() {
        let compiled = compile(&[
            // 未知名＋引数（消費側に既知の消費者が無くてもキャリアとして載る）。
            Instruction::GenericCommand {
                name: "raise".into(),
                raw_args: vec!["OnBoot".into()],
            },
            // 単独形 `\![*]`（raw_args 空）＝R8.2 卒業（無音落ちしない）。
            Instruction::GenericCommand {
                name: "vanish".into(),
                raw_args: vec![],
            },
        ]);
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        assert_eq!(cues.len(), 2, "未知名＋単独形の 2 キャリアが載る（無音落ちなし）");
        assert_eq!(
            command_of(&cues[0]).as_command_carrier(),
            Some(("raise", vec!["OnBoot"])),
            "未知名 `\\![raise,OnBoot]` もキャリア cue を発行（R9.3/8.2）"
        );
        assert_eq!(
            command_of(&cues[1]).as_command_carrier(),
            Some(("vanish", vec![])),
            "単独形 `\\![*]`（raw_args 空）もキャリア cue を発行（無音落ちしない・R8.2 卒業）"
        );
        for (i, cue) in cues.iter().enumerate() {
            assert_eq!(cue.duration, 0.0, "index {i} のキャリアは瞬時（duration 0）");
        }
    }

    /// sysvar スナップショット展開の決定論檻（R9.4・R7.1/7.4/7.5）。実 2 引数 `compile(&instr, &snapshot)`
    /// を直呼びし、スナップショット値がテキスト再生層まで実際に流れることを検証する（値あり／値なし
    /// 既定／未対応名の 3 経路を script 直入力から一括固定）。task 4.2 の実装が本檻で初めて
    /// **実時間非依存・script 直入力から**検証可能になる（値源は凍結スナップショット・no I/O）。
    #[test]
    fn sysvar_expansion_from_snapshot_is_deterministic() {
        // (1) 値ありスナップショット → その値の Text cue（duration は展開文字列の再生時間）。
        let mut vars = SystemVarSnapshot::default();
        vars.insert("username", "アヒル");
        let compiled = super::compile(&[Instruction::SystemVar("username".into())], &vars);
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        assert_eq!(cues.len(), 1, "展開 Text cue 1 件");
        assert_eq!(
            command_of(&cues[0]),
            &CueCommand::Text("アヒル".into()),
            "値ありスナップショットは当該値の Text へ展開（R7.1）"
        );
        assert_eq!(
            cues[0].duration,
            text_playback_duration("アヒル"),
            "duration = text_playback_duration(展開文字列)（R7.2）"
        );

        // (2) 値なしスナップショット（`username` 欠落）→ 既定値「ユーザーさん」の Text cue（R7.4）。
        let compiled = super::compile(
            &[Instruction::SystemVar("username".into())],
            &SystemVarSnapshot::default(),
        );
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        assert_eq!(
            command_of(&cues[0]),
            &CueCommand::Text("ユーザーさん".into()),
            "値なしスナップショットは既定値へ展開（生の `%username` を露出しない・R7.4）"
        );

        // (3) M1 未対応名 `%foo` → 元の `%foo` を Text として素通し（情報を失わない縮退・R7.5）。
        let compiled = super::compile(
            &[Instruction::SystemVar("foo".into())],
            &SystemVarSnapshot::default(),
        );
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        assert_eq!(
            command_of(&cues[0]),
            &CueCommand::Text("%foo".into()),
            "未対応名は元の `%名前` を素通し出力（R7.5）"
        );
    }
}
