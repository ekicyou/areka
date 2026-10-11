//! `check_script` の判断（読み取りの列と事実 → 診断の列）と診断の文言（spec: areka-P0-mcp-author-tools）。
//!
//! 窓も World も要らない純粋な関数だけを置く。判定は自分で持たず、再生が実際に使う関数
//! （`parse_noted` の印・`ConsumerLedger::consumer_of`・`parse_choice_timeout`・`apply_font_tag`・
//! `SurfaceResolver::resolve`・`resolve_balloon_key`・`pair_anchors`）の答えを診断の種類へ写すだけにする。

use std::ops::Range;

use areka_emo_text::look::{FontTagFailure, LookLayers, Note, apply_font_tag};
use areka_mcp::tools::check_script::{Diagnostic, Kind};
use areka_parsers::sakura::{Instruction, Read, ReadNote};
use areka_sakura::{AnchorIssue, ChoiceTimeoutDirective, pair_anchors, parse_choice_timeout};
use areka_seriko::{BalloonResolve, SurfaceTarget, resolve_balloon_key};

use crate::emo2_boot::consumer_ledger::ConsumerLedger;

// 文言（正本は `doc/ssp-mcp/areka-tools.md`。形は「何が起きているか; 再生でどうなるか」の英文 1 文）。
pub(in crate::mcp) const MSG_UNKNOWN_TAG: &str = "areka does not know this tag; playback drops it";
pub(in crate::mcp) const MSG_UNKNOWN_COMMAND: &str =
    "no part of areka handles this \\! command; playback ignores it";
pub(in crate::mcp) const MSG_MISSING_SURFACE: &str =
    "no such surface in the current shell; playback does not change the surface";
pub(in crate::mcp) const MSG_MISSING_BALLOON: &str =
    "no such balloon ID in the current balloon; playback does not change the balloon";
pub(in crate::mcp) const MSG_UNCLOSED: &str =
    "the bracket or quote is not closed; playback drops everything from here to the end";
pub(in crate::mcp) const MSG_DEFAULTED: &str =
    "the argument is missing or unreadable; playback uses the default value";
pub(in crate::mcp) const MSG_IGNORED: &str = "areka accepts this but it has no effect yet";
pub(in crate::mcp) const MSG_ANCHOR_UNCLOSED: &str =
    "the anchor is not closed; playback extends it to the end of the script";
pub(in crate::mcp) const MSG_ANCHOR_REOPENED: &str =
    "a new anchor opens before the previous one closes; playback closes the previous one here";
pub(in crate::mcp) const MSG_ANCHOR_STRAY_CLOSE: &str =
    "no anchor is open; playback ignores this close";

/// 判断に要る事実。本番は UI スレッドで写し取った値が答え、テストは手書きの表で答える。
pub(in crate::mcp) trait ScriptFacts {
    /// `\s` の引数を、再生と同じ解決器で解いた結果。
    fn resolve_surface(&self, key: &str) -> SurfaceTarget;
    /// そのスコープのシェルに生の surface ID が在るか。スコープに絵が無ければ None。
    fn shell_has(&self, scope: u32, surface_id: u32) -> Option<bool>;
    /// そのスコープのバルーンに面の ID が在るか。バルーンが無ければ None。
    fn balloon_has(&self, scope: u32, balloon_id: u32) -> Option<bool>;
}

/// 命令を台本の順に 1 つずつ見て診断の列を作る。`facts` が None＝窓の無いゴースト（surface とバルーンを診ない）。
/// `\e`・`\-` で打ち切らない（再生はされないが、書いてある誤りは知らせる）。アンカーの対応の崩れだけは、
/// 再生と同じ判定（`pair_anchors`）がそこで数えるのをやめるので、後ろの開き／閉じには出ない。
pub(in crate::mcp) fn diagnose(
    script: &str,
    reads: &[Read],
    ledger: &ConsumerLedger,
    facts: Option<&dyn ScriptFacts>,
) -> Vec<Diagnostic> {
    let mut out = Out {
        script,
        byte: 0,
        chars: 0,
        list: Vec::new(),
    };
    // スコープは `compile` と同じく `SpeakerScope` だけが替える（`\0`・`\1`・`\h`・`\u`・`\p[n]` はどれもこれになる）。
    let mut scope: u32 = 0;
    // 切替の `\!` を見た後は、どの絵・バルーンになるか字面で決まらないので診ない（要件 3.8）。
    let mut shell_switched = false;
    let mut balloon_switched = false;
    // アンカーの対応の崩れ。命令の添字の昇順で来るので、前から順に取り出す。
    let mut anchor_findings = pair_anchors(reads.iter().map(|r| &r.instruction))
        .into_iter()
        .peekable();

    for (index, read) in reads.iter().enumerate() {
        let span = read.span.clone();
        for note in &read.notes {
            let (kind, message) = match note {
                ReadNote::UnknownTag => (Kind::UnknownTag, MSG_UNKNOWN_TAG),
                ReadNote::Unclosed => (Kind::UnreadableArgument, MSG_UNCLOSED),
                ReadNote::ArgumentDefaulted => (Kind::UnreadableArgument, MSG_DEFAULTED),
                ReadNote::MarkerIgnored => (Kind::Ignored, MSG_IGNORED),
                // 読む段が後から足す印は、その意味が決まるまで診ない（誤って知らせる枝を作らない）。
                _ => continue,
            };
            out.push(kind, &span, message);
        }

        match &read.instruction {
            Instruction::SpeakerScope { n } => scope = *n,
            // 単独のマーカー（名前 `*`）は印の `ignored` だけで、表は引かない。
            Instruction::GenericCommand { name, .. }
                if read.notes.contains(&ReadNote::MarkerIgnored) && name == "*" => {}
            Instruction::GenericCommand { name, raw_args } => {
                let first = raw_args.first().map(String::as_str);
                if ledger.consumer_of(name, first).is_none() {
                    out.push(Kind::UnknownCommand, &span, MSG_UNKNOWN_COMMAND);
                } else if name == "set"
                    && first == Some("choicetimeout")
                    && parse_choice_timeout(raw_args) == ChoiceTimeoutDirective::Unreadable
                {
                    out.push(Kind::UnreadableArgument, &span, MSG_DEFAULTED);
                }
                // `\+`・`\_+` も `change,ghost` の `GenericCommand` として来る。
                if name == "change" {
                    match first {
                        Some("ghost") => (shell_switched, balloon_switched) = (true, true),
                        Some("shell") => shell_switched = true,
                        Some("balloon") => balloon_switched = true,
                        _ => {}
                    }
                }
            }
            Instruction::Move(args) => {
                let first = args.args.first().map(String::as_str);
                if ledger.consumer_of("move", first).is_none() {
                    out.push(Kind::UnknownCommand, &span, MSG_UNKNOWN_COMMAND);
                }
            }
            Instruction::Font { args } => {
                if let Some((kind, message)) = judge_font(args) {
                    out.push(kind, &span, message);
                }
            }
            Instruction::Surface(arg) if !shell_switched => {
                if let Some(facts) = facts
                    && surface_missing(facts, scope, arg.as_str())
                {
                    out.push(Kind::MissingSurface, &span, MSG_MISSING_SURFACE);
                }
            }
            Instruction::BalloonSurface(arg) if !balloon_switched => {
                if let Some(facts) = facts
                    && balloon_missing(facts, scope, arg.as_str())
                {
                    out.push(Kind::MissingBalloon, &span, MSG_MISSING_BALLOON);
                }
            }
            // 同じ開きに 2 件（重なり → 閉じ無し）付くことがあるので、その位置の分を全部出す。
            Instruction::Anchor(_) | Instruction::AnchorEnd => {
                while let Some(finding) = anchor_findings.next_if(|f| f.index == index) {
                    let message = match finding.issue {
                        AnchorIssue::Unclosed => MSG_ANCHOR_UNCLOSED,
                        AnchorIssue::Reopened => MSG_ANCHOR_REOPENED,
                        AnchorIssue::StrayClose => MSG_ANCHOR_STRAY_CLOSE,
                    };
                    out.push(Kind::UnpairedTag, &span, message);
                }
            }
            _ => {}
        }
    }
    out.list
}

/// `\f` を使い捨ての見た目に適用し、受け取るだけのキーは `ignored`、知らないキー・キーなしは `unknown_tag`。
/// 値の誤りと、他の `Ok`（`AnchorColorAsDefault` を含む）は診ない。
fn judge_font(args: &[String]) -> Option<(Kind, &'static str)> {
    let layers = LookLayers::default();
    let mut look = layers.default.clone();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match apply_font_tag(&mut look, &layers, &args) {
        Ok(Some(Note::VocabularyOnly { .. } | Note::Unowned | Note::StylesheetKeyword)) => {
            Some((Kind::Ignored, MSG_IGNORED))
        }
        Ok(_) => None,
        Err(issue) => match issue.failure() {
            FontTagFailure::UnknownKey | FontTagFailure::NoKey => {
                Some((Kind::UnknownTag, MSG_UNKNOWN_TAG))
            }
            FontTagFailure::BadValue => None,
        },
    }
}

/// スコープに絵が無ければ診ない（解けない名前・範囲外の数も、絵の在るスコープでだけ知らせる）。
fn surface_missing(facts: &dyn ScriptFacts, scope: u32, key: &str) -> bool {
    match facts.resolve_surface(key) {
        SurfaceTarget::Show(id) => facts.shell_has(scope, id) == Some(false),
        SurfaceTarget::Unresolved => facts.shell_has(scope, 0).is_some(),
        SurfaceTarget::Hide => false,
    }
}

/// 名前の形の `\b[名前]` は、そのとき表示しているサーフェスの箱に依るので診ない。
fn balloon_missing(facts: &dyn ScriptFacts, scope: u32, key: &str) -> bool {
    match resolve_balloon_key(key) {
        BalloonResolve::Show(id) => facts.balloon_has(scope, id) == Some(false),
        BalloonResolve::Invalid => facts.balloon_has(scope, 0).is_some(),
        BalloonResolve::Hide | BalloonResolve::NameForm => false,
    }
}

/// 診断の置き場。位置はバイトの範囲を前から 1 回だけ数えて文字の数へ直す（診断は台本の順に来る）。
struct Out<'a> {
    script: &'a str,
    /// ここまで数えたバイトの位置と、そこまでの文字の数。
    byte: usize,
    chars: usize,
    list: Vec<Diagnostic>,
}

impl Out<'_> {
    fn push(&mut self, kind: Kind, span: &Range<usize>, message: &'static str) {
        // 同じ命令に 2 件目が来たときは `byte == span.start` のままなので数え直さない。
        self.chars += self.script[self.byte..span.start].chars().count();
        self.byte = span.start;
        let text = &self.script[span.clone()];
        self.list.push(Diagnostic {
            kind,
            start: self.chars,
            end: self.chars + text.chars().count(),
            text: text.to_string(),
            message,
        });
    }
}

#[cfg(test)]
#[path = "check_script_judge_tests.rs"]
mod tests;
