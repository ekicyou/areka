//! 純粋コンパイル層（compile）— `Instruction` 列 → cue ドメインの発火列＋確定終端理由。
//!
//! [`compile`] は clock・sink・talk_id・アクターを知らない純粋関数（決定的・no I/O）。
//! 上流 [`areka_parsers::sakura::parse`] の出力 `Instruction` 列を走査し、発火時刻付きの
//! [`CueSheet`] を構築する。
//!
//! - `Text` は [`crate::duration::text_playback_duration`] の再生時間 D を cue の `duration` へ
//!   入れ、直後 offset を D だけ進める。
//! - `Wait(Duration)` は offset へ吸収せず、duration だけを持つ `CueCommand::Wait` cue として
//!   発行し、直後 offset を進める。末尾・単独の待ちも台本に残る。
//! - 話者スコープ `SpeakerScope{n}` を現在 scope として持ち、各 `Cue::actor` へ転写する。
//! - 内容 cue を持つ台本の先頭へ `ClearAll` cue を 1 つ前置し（`start_time=0`・`duration=0`）、
//!   新しい talk が全バルーン空から始まるようにする。
//!
//! # 先頭待ちの保存
//!
//! 台本の起点からの隔たりをそのまま `start_time` とし、min 正規化を行わない（min 正規化は先頭の待ちを 0 へ
//! 潰す）。下流の [`dola::cue::to_talk_schedule`] も相対 `start_time` を保存するので、先頭待ちは
//! 台本からスケジュールまで保存される。

use crate::anchor_pair::{AnchorIssue, pair_anchors};
use crate::contract::{
    ActorKey, BarrierKind, Cue, CueCommand, CuePayload, CueSheet, FONT_TAG_CARRIER, TalkEndReason,
    TimeDelta, TimeSpan,
};
use crate::strict::{self, DetectionPoint, StrictJudges};
use crate::strict_tap::StrictTrace;
use crate::sysvar::SystemVarSnapshot;
use areka_parsers::sakura::{Instruction, Read};
use areka_talk::EpilogueCommand;
use std::ops::Range;

/// 純粋コンパイル: `Instruction` 列 → cue ドメインの発火列＋確定終端理由（決定的・no I/O）。
///
/// 事前条件: `instructions` は [`areka_parsers::sakura::parse`] の出力（再パースしない）。
///
/// 事後条件: `sheet` 内の `start_time` は非減少。
/// 各テキスト cue は `duration = text_playback_duration(text)`（N>0 で正）、各 Wait cue は明示待ちの
/// `duration`、他の瞬時 cue は `duration=0` を持つ。内容 cue を持つ台本は先頭に単一 `ClearAll`
/// （`start_time=0`・`duration=0`）を持ち、台本のみから talk 全時間範囲
/// （`max(start_time + duration)`）が復元可能。同一入力に対し同一出力（決定的）。
/// 各 `Cue::actor` はその発火時点の有効 scope の転写。
///
/// `vars` は ghost が talk 開始時に手渡す名前→値の凍結スナップショット（`%username` 等の
/// システム変数展開の値源）。本関数は参照するのみで OS 環境・SHIORI・永続化層を直接
/// 読まない（純粋・決定論を保つ）。SystemVar アームが `resolve_system_var` で
/// これを純粋展開し、展開文字列を Text cue へ写像する。
pub fn compile(instructions: &[Instruction], vars: &SystemVarSnapshot) -> CompiledTalk {
    scan(instructions.iter(), vars, None)
}

/// strict の台本の [`compile`]。`compile` と同じ走査で
/// 同じ [`CompiledTalk`] を作り、台本の解釈で分かる誤りを命令ごとに控える。
///
/// 控えの順は `check_script` の診断と同じ（命令の順に、読みの印 → 命令の腕 → アンカー）。
/// 合図の順番は sink へ配られる合図だけを数え、先頭の全消しを順番 0 に含め、末尾の選択待ちの
/// 区切りは数えない。`\e`・`\-` で走査が止まるので、その後ろは控えない。
pub(crate) fn compile_strict(
    reads: &[Read],
    vars: &SystemVarSnapshot,
    judges: &dyn StrictJudges,
) -> (CompiledTalk, StrictTrace) {
    let mut tracer = Tracer {
        reads,
        judges,
        index: 0,
        at: 0,
        trace: StrictTrace::default(),
    };
    let compiled = scan(
        reads.iter().map(|read| &read.instruction),
        vars,
        Some(&mut tracer),
    );
    (compiled, tracer.trace)
}

/// strict の控えを取る走査の連れ。`compile` では居ない（控えを取らない）。
struct Tracer<'a> {
    reads: &'a [Read],
    judges: &'a dyn StrictJudges,
    /// 今の命令の添字。
    index: usize,
    /// 今の命令の頭で控えた「次に作られる合図」の順番（先頭の全消しを差し込む前の数え方）。
    at: usize,
    trace: StrictTrace,
}

impl Tracer<'_> {
    fn span(&self) -> Range<usize> {
        self.reads[self.index].span.clone()
    }

    fn find(&mut self, point: DetectionPoint) {
        let span = self.span();
        self.trace.pending.push((self.at, point, span));
    }

    /// 命令の頭: 順番を控え、読みの印を控える。
    fn head(&mut self, index: usize, next_cue: usize) {
        (self.index, self.at) = (index, next_cue);
        for note in &self.reads[index].notes {
            self.find(strict::read_note_point(note));
        }
    }

    /// `\!` の腕: 誰も拾わない組と、読めない `choicetimeout`（`choice_timeout` は 1 回だけ読んだ答え）。
    fn command(
        &mut self,
        name: &str,
        raw_args: &[String],
        choice_timeout: Option<ChoiceTimeoutDirective>,
    ) {
        let handled = self
            .judges
            .command_handled(name, raw_args.first().map(String::as_str));
        if let Some(point) = strict::command_point(handled, choice_timeout) {
            self.find(point);
        }
    }

    /// ここまでに作られた合図の位置を `span` で埋める（命令の合図は命令の範囲・補いは `None`）。
    fn spans_up_to(&mut self, cues: usize, span: Option<Range<usize>>) {
        self.trace.cue_spans.resize(cues, span);
    }
}

/// `compile` と `compile_strict` が共有する走査。`tracer` が在るときだけ控えを取る（合図の列は同じ）。
fn scan<'a>(
    instructions: impl Iterator<Item = &'a Instruction> + Clone,
    vars: &SystemVarSnapshot,
    mut tracer: Option<&mut Tracer<'_>>,
) -> CompiledTalk {
    let mut offset = TimeSpan::ZERO;
    let mut scope: u32 = 0;
    let mut cues: Vec<Cue> = Vec::new();
    // 終端命令なしで末尾に着いたら通常終了。End/Quit で確定して走査を打ち切る。
    let mut end = TalkEndReason::Ended;
    // `\![set,choicetimeout,時間]` の最後の値（`None`＝未指定）。読むたびに上書きし、走査後の
    // 選択待ちの区切りへ 1 回だけ入れる。
    let mut last_choice_timeout: Option<TimeDelta> = None;
    // アンカーの開き `\_a[…]` と閉じ `\_a` の対応の崩れ（位置の昇順）。走査の前に 1 回だけ判定し、
    // 開きと閉じの腕が自分の位置の分を先頭から順に取り出す。
    // 判定も走査も `\e`・`\-` で止まるので、崩れの位置は必ずどちらかの腕を通る。
    let anchor_findings = pair_anchors(instructions.clone());
    let mut anchor_findings = anchor_findings.iter().peekable();
    let mut anchor_issue_at = |index: usize, issue: AnchorIssue| {
        anchor_findings
            .next_if(|f| f.index == index && f.issue == issue)
            .is_some()
    };
    // 閉じの無い開きがある（走査の終わりで閉じを補う）。
    let mut anchor_unclosed = false;
    // 選択肢の合図を 1 つでも出した（走査の後で選択待ちの区切りを出す）。
    let mut has_choice = false;

    for (index, instruction) in instructions.enumerate() {
        if let Some(t) = tracer.as_deref_mut() {
            t.head(index, cues.len());
        }
        match instruction {
            // 明示ウェイトは offset へ吸収せず、duration だけを持つ Wait cue として発行する
            // （末尾・単独でも台本に残る）。時間は envelope duration が担い、コマンドは種別のみを表す。
            Instruction::Wait(duration) => {
                // 表せない長さは上限に張り付く（offset の足し算と同じ）。
                let d = TimeSpan::try_from(*duration).unwrap_or(TimeSpan::MAX);
                cues.push(emit(scope, offset, d, CueCommand::Wait));
                offset = offset + d;
            }
            // scope の更新だけで cue は作らない。
            Instruction::SpeakerScope { n } => {
                scope = *n;
            }
            // 暗黙の再生時間 D を duration へ入れ、直後 offset を D だけ進めて、
            // 後続 cue をテキスト再生完了後へ整列する。
            Instruction::Text(text) => {
                let d = crate::duration::text_playback_duration(text);
                cues.push(emit(scope, offset, d, CueCommand::Text(text.clone())));
                offset = offset + d;
            }
            // サーフェス切替。引数は解釈・変換せず不透明に転写する。瞬時（duration 0）。
            Instruction::Surface(arg) => {
                cues.push(emit(
                    scope,
                    offset,
                    TimeSpan::ZERO,
                    CueCommand::Emote {
                        key: arg.as_str().to_string(),
                    },
                ));
            }
            // バルーン面切替。引数は Surface と同じく不透明に転写する（バイト完全一致。数値化・範囲展開・
            // alias 解決はしない。数値化は下流 seriko の責務）。瞬時。
            Instruction::BalloonSurface(arg) => {
                cues.push(emit(
                    scope,
                    offset,
                    TimeSpan::ZERO,
                    CueCommand::BalloonSurface {
                        key: arg.as_str().to_string(),
                    },
                ));
            }
            // 改行（比率つき）。瞬時（duration 0）。
            Instruction::NewLine(ratio) => {
                cues.push(emit(
                    scope,
                    offset,
                    TimeSpan::ZERO,
                    CueCommand::NewLine {
                        ratio: ratio.ratio(),
                    },
                ));
            }
            // 対象スコープのみのクリア。瞬時（duration 0）。
            Instruction::Clear => {
                cues.push(emit(scope, offset, TimeSpan::ZERO, CueCommand::Clear));
            }
            // 選択肢 `\q[disp,target,refs...]`。`id = target`（選択 ID・第 2 引数）・`text = disp`
            // （表示ラベル・第 1 引数）・`references`（第 3 引数以降）を欠落なく不透明に転写する
            // （ID の解釈・整数化なし）。瞬時。選択待ちの区切りは走査の後で足す（下の `has_choice`）。
            Instruction::Choice(choice) => {
                has_choice = true;
                cues.push(emit(
                    scope,
                    offset,
                    TimeSpan::ZERO,
                    CueCommand::Choice {
                        id: choice.target.clone(),
                        text: choice.disp.clone(),
                        references: choice.references.clone(),
                    },
                ));
            }
            // カーソル絶対位置 `\_l[x,y]`。x・y は記述通りの不透明文字列で転写する（単位付き
            // `5em`/`2lh`・裸数値・`@` 相対・空の区別を失わない。単位換算・座標解決は消費側の責務）。
            // 双方が空でも無条件に発行する（記述の存在を台本から失わせない）。瞬時。
            Instruction::Cursor { x, y } => {
                cues.push(emit(
                    scope,
                    offset,
                    TimeSpan::ZERO,
                    CueCommand::Cursor {
                        x: x.clone(),
                        y: y.clone(),
                    },
                ));
            }
            // 終端 `\e`: 終端理由 Ended を確定し以降を切り詰める。
            // ukadoc `\e` = この後に書かれたスクリプトは実行・表示されない。
            Instruction::End => {
                end = TalkEndReason::Ended;
                break;
            }
            // 終了 `\-`: 終端理由 Quit を確定し以降を切り詰める。
            Instruction::Quit => {
                end = TalkEndReason::Quit;
                break;
            }
            // 汎用 `\!` コマンド（`move` も含む）。name と raw_args を解釈せず同一キャリアへ載せる
            // （`\![*]` 単独形＝raw_args 空も台本に載る。空トークン・`--key=value` 形も素通し）。
            // 消費側はコマンド名で選別する。瞬時。
            //
            // 転記の**後**に、`set,choicetimeout`（綴りは逐語）だけは時間の欄を読み、最後の値を
            // 上書きする（転記はそのまま・offset も cue も増やさない・他の名前は読まない）。
            // 読めない値は既定として扱い、警告を 1 行残す。
            Instruction::GenericCommand { name, raw_args } => {
                cues.push(emit(
                    scope,
                    offset,
                    TimeSpan::ZERO,
                    CueCommand::command_carrier(name.clone(), raw_args.clone()),
                ));
                let choice_timeout = choice_timeout_of(name, raw_args);
                if let Some(t) = tracer.as_deref_mut() {
                    t.command(name, raw_args, choice_timeout);
                }
                if let Some(directive) = choice_timeout {
                    last_choice_timeout = match directive {
                        ChoiceTimeoutDirective::Default => None,
                        ChoiceTimeoutDirective::Delta(delta) => Some(delta),
                        ChoiceTimeoutDirective::Unreadable => {
                            tracing::warn!(
                                event = "choice_timeout_unreadable",
                                raw = %raw_args[1],
                                "[compile] set,choicetimeout の時間が整数として読めないので既定として扱う"
                            );
                            None
                        }
                    };
                }
            }
            // システム変数 `%username` 等。値源は所有せず、talk 起動時に手渡された凍結スナップショット
            // `vars` を純粋展開する（`resolve_system_var`・no I/O）。展開値（スナップショット値／
            // 既定値／未対応名の素通し `%名前`）はいずれも通常テキストと同格の Text cue にし、
            // 再生時間を焼き込んで offset を進める。独立 cue とし、隣接 Text と併合しない。
            Instruction::SystemVar(name) => {
                let expanded = match crate::sysvar::resolve_system_var(name, vars) {
                    crate::sysvar::ResolvedVar::Text(s) => s,
                    crate::sysvar::ResolvedVar::PassThrough(s) => s,
                };
                let d = crate::duration::text_playback_duration(&expanded);
                cues.push(emit(scope, offset, d, CueCommand::Text(expanded)));
                offset = offset + d;
            }
            // 文字装飾 `\f[key,args...]`。キーごとに typed variant を
            // 新設せず、`\!` と同じ 1 本の汎用キャリア（名前 `FONT_TAG_CARRIER`）へ載せる。引数
            // （空トークン含む）は記述順のまま無変形で運び、意味付けは消費側（文字レンダリング層）
            // がキーで自己選別して行う。**再生時間 0**（`\f` の有無で台本の時刻列を変えない）で、
            // 前後の文字との並び順は書き出し位置 `offset` の転写だけで保たれる（offset は進めない）。
            Instruction::Font { args } => {
                cues.push(emit(
                    scope,
                    offset,
                    TimeSpan::ZERO,
                    CueCommand::command_carrier(FONT_TAG_CARRIER, args.clone()),
                ));
                if let Some(t) = tracer.as_deref_mut()
                    && let Some(point) = t.judges.font(args)
                {
                    t.find(point);
                }
            }
            // アンカーの開き `\_a[ID,…]`。ID と引数の列を
            // 解釈せずに運ぶ 0 秒の合図（`Choice` と同じ）。ここから閉じまでに流れる文字が押せる範囲に
            // なる。同じ位置に重なりと閉じ無しの 2 件が付くことがあるので、両方を見る（判定が返す順）。
            Instruction::Anchor(anchor) => {
                // 開いている間の新たな開き: 直前のアンカーをここで閉じてから始める。
                if anchor_issue_at(index, AnchorIssue::Reopened) {
                    tracing::warn!(
                        event = "anchor_reopened",
                        index,
                        id = %anchor.id,
                        "[compile] アンカーが開いている間に新しい開きがあるので、直前のアンカーをここで閉じる"
                    );
                    cues.push(emit(scope, offset, TimeSpan::ZERO, CueCommand::AnchorEnd));
                    if let Some(t) = tracer.as_deref_mut() {
                        t.find(DetectionPoint::AnchorReopened);
                    }
                }
                // 閉じの無い開き: 範囲は表示の終わりまで（閉じは走査の終わりで補う）。
                if anchor_issue_at(index, AnchorIssue::Unclosed) {
                    tracing::warn!(
                        event = "anchor_unclosed",
                        index,
                        id = %anchor.id,
                        "[compile] アンカーの開きに閉じが無いので、表示の終わりまでを範囲にする"
                    );
                    anchor_unclosed = true;
                    if let Some(t) = tracer.as_deref_mut() {
                        t.find(DetectionPoint::AnchorUnclosed);
                    }
                }
                cues.push(emit(
                    scope,
                    offset,
                    TimeSpan::ZERO,
                    CueCommand::AnchorBegin {
                        id: anchor.id.clone(),
                        references: anchor.references.clone(),
                    },
                ));
            }
            // アンカーの閉じ（角括弧の無い `\_a`）。0 秒の合図。開いて
            // いないのに閉じなら合図を出さず、警告を残して表示を続ける。
            Instruction::AnchorEnd => {
                if anchor_issue_at(index, AnchorIssue::StrayClose) {
                    tracing::warn!(
                        event = "anchor_stray_close",
                        index,
                        "[compile] アンカーが開いていないのに閉じがあるので無視する"
                    );
                    if let Some(t) = tracer.as_deref_mut() {
                        t.find(DetectionPoint::AnchorStrayClose);
                    }
                } else {
                    cues.push(emit(scope, offset, TimeSpan::ZERO, CueCommand::AnchorEnd));
                }
            }
            // 出せない・読めない文字の番号（`\_u[…]`・`\_m[…]`）。合図を作らず
            // 警告を 1 行残して続ける（代わりの字は出さない）。strict では「読めない引数」。
            Instruction::CharCodeDropped { raw, reason } => {
                tracing::warn!(
                    event = "char_code_dropped",
                    raw = %raw,
                    reason = ?reason,
                    "[compile] 文字の番号を出せないので捨てる"
                );
                if let Some(t) = tracer.as_deref_mut() {
                    t.find(DetectionPoint::CharCodeDropped);
                }
            }
            // 知らないタグ・閉じの無い `[` の残り（`Raw`）は合図を作らず、警告を 1 行残して続ける
            // （寛容・panic しない）。綴りは長いことがあるので先頭 64 字とバイト数だけ。
            Instruction::Raw(raw) => {
                let head: String = raw.chars().take(64).collect();
                tracing::warn!(
                    event = "raw_ignored",
                    head = %head,
                    bytes = raw.len(),
                    "[compile] 知らないタグを捨てる"
                );
            }
        }
        // この命令から作った合図（重なりで足す閉じを含む）は、この命令の範囲を持つ。
        if let Some(t) = tracer.as_deref_mut() {
            let span = t.span();
            t.spans_up_to(cues.len(), Some(span));
        }
    }

    // 閉じの無いアンカーは走査の終わり（`\e`・`\-`・末尾）で閉じる。
    // 選択待ちの区切りを出す台本でも、閉じはその前に並ぶ。アンカーは下の区切りの判定に関わらない
    // （`Choice` だけを数える）。
    if anchor_unclosed {
        cues.push(emit(scope, offset, TimeSpan::ZERO, CueCommand::AnchorEnd));
    }
    // 走査の後で補った閉じは台本に元が無い。区切りは配られないので数えない（下で積む前に締める）。
    if let Some(t) = tracer.as_deref_mut() {
        t.spans_up_to(cues.len(), None);
    }

    // 選択待ちの区切り: choice cue が 1 個以上あれば `WaitForChoice{timeout:last_choice_timeout}`
    // （走査で最後に読んだ `\![set,choicetimeout,時間]` の差。指定なし・省略・空欄・読めない値は
    // `None`＝未指定＝下流の既定値へ委譲し、値の解釈は kanade だけが行う）を最終 offset へちょうど
    // 1 個足す。選択肢が無ければ出さず、読んだ値は捨てる。同一 at の FIFO 挿入で全 cue より後に
    // 配送される。presentation でないので duration は 0。
    if has_choice {
        cues.push(emit(
            scope,
            offset,
            TimeSpan::ZERO,
            BarrierKind::WaitForChoice {
                timeout: last_choice_timeout,
            },
        ));
    }

    // 内容 cue を持つ台本の先頭へ全スコープ消去 `ClearAll` を 1 つ前置する。残存スコープを列挙
    // できないので per-scope Clear でなく全消しで表し、書き込むスコープ数に依らず 1 件。
    // `CueSheet::new` の安定ソートと同一 `at` の FIFO で先頭に配送される。内容 cue を持たない台本
    // （リテラル空・裸終端・無視タグのみ）は空 sheet のままにし、drive の `is_empty()` 即時
    // TalkDone を保つ。
    if !cues.is_empty() {
        cues.insert(
            0,
            emit(0, TimeSpan::ZERO, TimeSpan::ZERO, CueCommand::ClearAll),
        );
        // 全消しは位置なしの順番 0。命令の合図と控えの順番はみな 1 つ後ろへずれる。
        if let Some(t) = tracer {
            t.trace.cue_spans.insert(0, None);
            for (at, _, _) in &mut t.trace.pending {
                *at += 1;
            }
        }
    }

    CompiledTalk {
        sheet: CueSheet::new(cues),
        end,
    }
}

/// 現在 scope・発火時刻・再生時間 duration・中身（演出コマンドか区切り）から 1 発火 [`Cue`] を
/// 構築する。合図を組むのはここ 1 か所だけ。
///
/// `actor` はその発火時点の有効 scope の転写（`ActorKey::from(n.to_string())`・既定 "0"）。
/// `duration` は当該 cue の presentation 占有時間（テキストは D、Wait は明示待ち、ほかの瞬時の
/// 合図と区切りは 0）。
fn emit(scope: u32, start: TimeSpan, duration: TimeSpan, payload: impl Into<CuePayload>) -> Cue {
    Cue {
        actor: ActorKey::from(scope.to_string()),
        start_time: start,
        payload: payload.into(),
        duration,
    }
}

/// `\![set,choicetimeout,…]` のときだけ時間の欄を 1 回読む（compile と strict と `check_script` が使う）。
pub fn choice_timeout_of(name: &str, raw_args: &[String]) -> Option<ChoiceTimeoutDirective> {
    (name == "set" && raw_args.first().is_some_and(|a| a == "choicetimeout"))
        .then(|| parse_choice_timeout(raw_args))
}

/// `\![set,choicetimeout,時間]` の時間の欄の読み取りの結果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChoiceTimeoutDirective {
    /// 欄なし・空欄 → 既定へ戻す（区切りには `None` を書く）。
    Default,
    /// 整数で読めた → 符号つきの差（区切りには `Some(delta)` を書く）。`0`・負は無期限、正は期限。
    Delta(TimeDelta),
    /// 整数として読めない → 呼び手が警告を出し、既定として扱う（区切りには `None` を書く）。
    Unreadable,
}

/// `set,choicetimeout` の `raw_args` から、選択待ちの区切りへ入れる指令を決める純関数
/// （no I/O・記録を出さない）。
///
/// `raw_args[0] == "choicetimeout"` は呼び手が確かめる。時間の欄は `raw_args[1]` だけを読み、
/// 3 欄目以降は見ない。前後の空白は落としてから `i64` として読む（`+500` も受ける）。
/// 桁あふれは正負それぞれ `i64::MAX`／`i64::MIN` ミリ秒へ飽和し、差に表せない値も上下限へ張り付く。
/// 値の正規化（`-1` を `0` にそろえる等）はしない。
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bset_2cchoicetimeout_2c_6642_9593_5d:1
pub fn parse_choice_timeout(raw_args: &[String]) -> ChoiceTimeoutDirective {
    use std::num::IntErrorKind;
    // 欄なし・空欄（空白だけを含む）は省略と同じ＝既定へ戻す。
    let field = match raw_args.get(1).map(|f| f.trim()) {
        None | Some("") => return ChoiceTimeoutDirective::Default,
        Some(field) => field,
    };
    let ms = match field.parse::<i64>() {
        Ok(ms) => ms,
        Err(e) => match e.kind() {
            IntErrorKind::PosOverflow => i64::MAX,
            IntErrorKind::NegOverflow => i64::MIN,
            _ => return ChoiceTimeoutDirective::Unreadable,
        },
    };
    ChoiceTimeoutDirective::Delta(TimeDelta::from_millis_saturating(ms))
}

/// [`compile`] 済み [`CueSheet`] の末尾へ epilogue を汎用キャリア cue として付加する
/// （決定論・no I/O）。
///
/// 各 `EpilogueCommand` は 1 個の carrier cue（`actor="0"`・`duration=0`・
/// `payload=command_carrier(name, tokens)`）へ写像され、`start_time` は既存 cues の
/// `max(start_time + duration)`（空 sheet は 0）＝台本の占有 horizon に一致する。zero-duration
/// ゆえ horizon は延長されない（`TalkDone` を遅らせない）。
///
/// [`CueSheet::new`] の**安定ソート**で再構築するため、同一 `start_time` に既存の末尾要素
/// （選択待ち barrier 等）があっても epilogue cue は**その後ろ**へ並ぶ＝barrier 解決後・占有
/// horizon 到達 tick（`TalkDone` 送出前）に発火する。
///
/// `epilogue` が空なら**恒等**（`sheet` をそのまま返す）＝既存経路は完全に不変。
pub(crate) fn append_epilogue(sheet: CueSheet, epilogue: &[EpilogueCommand]) -> CueSheet {
    // 空 epilogue は恒等（アンカー刻印状態も含め sheet を変えない）。
    if epilogue.is_empty() {
        return sheet;
    }

    let horizon = sheet.relative_end();

    let mut cues: Vec<Cue> = sheet.cues().to_vec();
    for command in epilogue {
        cues.push(emit(
            0,
            horizon,
            TimeSpan::ZERO,
            CueCommand::command_carrier(command.name.clone(), command.tokens.clone()),
        ));
    }
    CueSheet::new(cues)
}

/// [`compile`] の結果。`sheet` は wintf cue パイプラインがそのまま消費できる serde 可能形
/// （ghost-setup handoff 成果物）。
pub struct CompiledTalk {
    /// 台本の起点からの隔たりの発火列（`CuePayload::Command`。選択肢を持つ台本では、その後ろに選択待ちの
    /// 区切り `CuePayload::Barrier`（`WaitForChoice`）が高々 1 つ並ぶ）。
    pub sheet: CueSheet,
    /// コンパイル時点で確定した終端理由。
    pub end: TalkEndReason,
}

#[cfg(test)]
#[path = "compile_anchor_tests.rs"]
mod anchor_tests;
#[cfg(test)]
#[path = "compile_arm_tests.rs"]
mod arm_tests;
#[cfg(test)]
#[path = "compile_char_code_tests.rs"]
mod char_code_tests;
#[cfg(test)]
#[path = "compile_choice_timeout_tests.rs"]
mod choice_timeout_tests;
#[cfg(test)]
#[path = "compile_font_tests.rs"]
mod font_tests;
#[cfg(test)]
#[path = "compile_sheet_tests.rs"]
mod sheet_tests;
#[cfg(test)]
#[path = "compile_strict_tests.rs"]
mod strict_tests;
#[cfg(test)]
#[path = "compile_test_support.rs"]
mod test_support;
