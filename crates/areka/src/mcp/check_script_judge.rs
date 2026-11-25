//! `check_script` の判断（読み取りの列と事実 → 診断の列）と診断の文言。
//!
//! 窓も World も要らない純粋な関数だけを置く。判定は自分で持たず、再生が実際に使う関数
//! （`parse_noted` の印・strict の判定 `StrictJudges`・`choice_timeout_of`・strict と同じ見分けの点
//! （`strict::read_note_point`・`command_point`）・`surface_missing`・`balloon_missing`（`areka_seriko`）・
//! `pair_anchors`）の答えを strict の点にし、点から診断の種類と文言を引くだけにする。

use std::ops::Range;

use areka_mcp::tools::check_script::{Diagnostic, Kind};
use areka_parsers::sakura::{Instruction, Read};
use areka_sakura::compile::choice_timeout_of;
use areka_sakura::strict::{self, CharCursor, DetectionPoint, StrictJudges};
use areka_sakura::{AnchorIssue, pair_anchors};
use areka_seriko::{SkinLookup, balloon_missing, surface_missing};

/// 命令を台本の順に 1 つずつ見て診断の列を作る。`facts` が None＝窓の無いゴースト（surface とバルーンを診ない）。
/// `\e`・`\-` で打ち切らない（再生はされないが、書いてある誤りは知らせる）。アンカーの対応の崩れだけは、
/// 再生と同じ判定（`pair_anchors`）がそこで数えるのをやめるので、後ろの開き／閉じには出ない。
pub(crate) fn diagnose(
    script: &str,
    reads: &[Read],
    judges: &dyn StrictJudges,
    facts: Option<&dyn SkinLookup>,
) -> Vec<Diagnostic> {
    let mut out = Out {
        cursor: CharCursor::new(script),
        list: Vec::new(),
    };
    // スコープは `compile` と同じく `SpeakerScope` だけが替える（`\0`・`\1`・`\h`・`\u`・`\p[n]` はどれもこれになる）。
    let mut scope: u32 = 0;
    // 切替の `\!` を見た後は、どの絵・バルーンになるか字面で決まらないので診ない。
    let mut shell_switched = false;
    let mut balloon_switched = false;
    // アンカーの対応の崩れ。命令の添字の昇順で来るので、前から順に取り出す。
    let mut anchor_findings = pair_anchors(reads.iter().map(|r| &r.instruction))
        .into_iter()
        .peekable();

    for (index, read) in reads.iter().enumerate() {
        let span = read.span.clone();
        for note in &read.notes {
            out.push_point(strict::read_note_point(note), &span);
        }

        match &read.instruction {
            Instruction::SpeakerScope { n } => scope = *n,
            Instruction::GenericCommand { name, raw_args } => {
                let first = raw_args.first().map(String::as_str);
                let handled = judges.command_handled(name, first);
                if let Some(point) =
                    strict::command_point(handled, choice_timeout_of(name, raw_args))
                {
                    out.push_point(point, &span);
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
            Instruction::Font { args } => {
                if let Some(point) = judges.font(args) {
                    out.push_point(point, &span);
                }
            }
            Instruction::Surface(arg) if !shell_switched => {
                if let Some(point) = facts.and_then(|f| surface_missing(f, scope, arg.as_str())) {
                    out.push_point(point, &span);
                }
            }
            Instruction::BalloonSurface(arg) if !balloon_switched => {
                if let Some(point) = facts.and_then(|f| balloon_missing(f, scope, arg.as_str())) {
                    out.push_point(point, &span);
                }
            }
            // 読み手が捨てた `\_u[…]`・`\_m[…]`（範囲は埋め込みの綴り）。出せる番号は `Text` で来るので診ない。
            Instruction::CharCodeDropped { .. } => {
                out.push_point(DetectionPoint::CharCodeDropped, &span);
            }
            // 同じ開きに 2 件（重なり → 閉じ無し）付くことがあるので、その位置の分を全部出す。
            Instruction::Anchor(_) | Instruction::AnchorEnd => {
                while let Some(finding) = anchor_findings.next_if(|f| f.index == index) {
                    let point = match finding.issue {
                        AnchorIssue::Unclosed => DetectionPoint::AnchorUnclosed,
                        AnchorIssue::Reopened => DetectionPoint::AnchorReopened,
                        AnchorIssue::StrayClose => DetectionPoint::AnchorStrayClose,
                    };
                    out.push_point(point, &span);
                }
            }
            // 診るものの無い命令（字・待ち・改行・選択肢・位置・終わり・消去・変数・知らないタグ
            // （印で診た）と、切替の後の絵・バルーン）。命令が増えたらここでコンパイルが止まる。
            Instruction::Text(_)
            | Instruction::Surface(_)
            | Instruction::BalloonSurface(_)
            | Instruction::Wait(_)
            | Instruction::NewLine(_)
            | Instruction::Choice(_)
            | Instruction::Cursor { .. }
            | Instruction::End
            | Instruction::Clear
            | Instruction::Quit
            | Instruction::SystemVar(_)
            | Instruction::Raw(_) => {}
        }
    }
    out.list
}

/// 診断の置き場。位置は strict の記録と同じ物差し（`CharCursor`）で文字の数へ直す（診断は台本の順に来る）。
struct Out<'a> {
    cursor: CharCursor<'a>,
    list: Vec<Diagnostic>,
}

impl Out<'_> {
    /// strict と同じ見分けの点を診断にする。
    fn push_point(&mut self, point: DetectionPoint, span: &Range<usize>) {
        let place = self.cursor.place(span.clone());
        self.list.push(Diagnostic {
            kind: kind_of(point),
            start: place.start,
            end: place.end,
            text: place.text,
            message: point.message(),
        });
    }
}

/// 点の種類の綴り（`DetectionPoint::kind`）を診断の種類へ。種類の決まりは strict の 1 か所だけ。
fn kind_of(point: DetectionPoint) -> Kind {
    [
        Kind::UnknownTag,
        Kind::UnknownCommand,
        Kind::MissingSurface,
        Kind::MissingBalloon,
        Kind::UnreadableArgument,
        Kind::Ignored,
        Kind::UnpairedTag,
    ]
    .into_iter()
    .find(|k| k.as_str() == point.kind())
    .expect("every DetectionPoint::kind is a check_script Kind")
}

#[cfg(test)]
#[path = "check_script_judge_tests.rs"]
pub(in crate::mcp) mod tests;
