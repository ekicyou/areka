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
use areka_talk::EpilogueCommand;

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
    // cue が 1 個以上あれば選択待ち barrier `WaitForChoice{timeout:None}`（`None`＝未指定＝下流の
    // 既定値へ委譲する・DD-8。台本からの時間指定は追跡 spec の領分ゆえ本層は値を供給しない）を最終 offset へ
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

/// [`compile`] 済み [`CueSheet`] の末尾へ epilogue を汎用キャリア cue として付加する
/// （決定論・no I/O・design C12・R3.4）。
///
/// 各 `EpilogueCommand` は 1 個の carrier cue（`actor="0"`・`duration=0.0`・
/// `payload=command_carrier(name, tokens)`）へ写像され、`start_time` は既存 cues の
/// `max(start_time + duration)`（空 sheet は 0.0）＝台本の占有 horizon に一致する。zero-duration
/// ゆえ horizon は延長されない（`TalkDone` を遅らせない）。
///
/// [`CueSheet::new`] の**安定ソート**で再構築するため、同一 `start_time` に既存の末尾要素
/// （選択待ち barrier 等）があっても epilogue cue は**その後ろ**へ並ぶ＝barrier 解決後・占有
/// horizon 到達 tick（`TalkDone` 送出前）に発火する。
///
/// `epilogue` が空なら**恒等**（`sheet` をそのまま返す）＝既存経路は完全に不変。
pub fn append_epilogue(sheet: CueSheet, epilogue: &[EpilogueCommand]) -> CueSheet {
    // 空 epilogue は恒等（アンカー刻印状態も含め sheet を一切変えない・既存経路完全不変）。
    if epilogue.is_empty() {
        return sheet;
    }

    // 既存 cues の占有 horizon（max(start_time + duration)・空 sheet は 0.0）を末尾 offset とする。
    let horizon = sheet
        .cues()
        .iter()
        .map(|cue| cue.start_time + cue.duration)
        .fold(0.0_f64, f64::max);

    // 既存 cues を保ったまま epilogue を末尾へ push し、CueSheet::new の安定ソートで再構築する
    // （同一 at の既存末尾要素より後に並ぶ＝FIFO）。zero-duration ゆえ horizon は不変。
    let mut cues: Vec<Cue> = sheet.cues().to_vec();
    for command in epilogue {
        cues.push(Cue {
            actor: ActorKey::from("0".to_string()),
            start_time: horizon,
            payload: CuePayload::Command(CueCommand::command_carrier(
                command.name.clone(),
                command.tokens.clone(),
            )),
            duration: 0.0,
        });
    }
    CueSheet::new(cues)
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
#[path = "compile_test_support.rs"]
mod test_support;
#[cfg(test)]
#[path = "compile_arm_tests.rs"]
mod arm_tests;
#[cfg(test)]
#[path = "compile_sheet_tests.rs"]
mod sheet_tests;
