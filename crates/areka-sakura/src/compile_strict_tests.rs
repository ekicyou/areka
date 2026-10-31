//! strict の走査の檻（spec: mcp-strict-errors 要件 2.5・2.7・3.6・3.9）。
//!
//! 1. 同じ命令の列から、strict かどうかに関わらず同じ合図の列ができる（見本の台本の集まり）。
//! 2. 台本の解釈で分かる誤りを、命令ごとに `check_script` の順（読みの印 → 命令の腕 → アンカー）で控える。
//! 3. 合図の順番は配られる合図だけを数え、先頭の全消しを順番 0 に含め、末尾の選択待ちの区切りは
//!    数えない。`CuePlayer` が配った順と控えの順番が一致する。
//! 4. `\e`・`\-` の後ろは控えない。

use super::test_support::cue_eq;
use super::*;
use crate::strict::{DetectionPoint, StrictJudges};
use areka_parsers::sakura::{parse, parse_noted};
use dola::cue::{CuePlayer, CueSink, TalkCue};
use std::sync::{Arc, Mutex};

/// 拾う `\!` は `open`・`set`・`move`（`move` の第 1 引数 `nosuch` だけは拾わない）。
/// `\f` は `sub` を受け取るだけ、`colour` を知らないキーとする。
struct FakeJudges;

impl StrictJudges for FakeJudges {
    fn command_handled(&self, name: &str, first: Option<&str>) -> bool {
        match name {
            "open" | "set" => true,
            "move" => first != Some("nosuch"),
            _ => false,
        }
    }
    fn font(&self, args: &[String]) -> Option<DetectionPoint> {
        match args.first().map(String::as_str) {
            Some("sub") => Some(DetectionPoint::FontIgnored),
            Some("colour") => Some(DetectionPoint::FontUnknownKey),
            _ => None,
        }
    }
}

/// どれも誤りとする判定（控えが合図の列を変えないことを確かめる）。
struct AllWrong;

impl StrictJudges for AllWrong {
    fn command_handled(&self, _: &str, _: Option<&str>) -> bool {
        false
    }
    fn font(&self, _: &[String]) -> Option<DetectionPoint> {
        Some(DetectionPoint::FontUnknownKey)
    }
}

fn strict(script: &str, judges: &dyn StrictJudges) -> (CompiledTalk, StrictTrace) {
    compile_strict(&parse_noted(script), &SystemVarSnapshot::default(), judges)
}

/// 控え 1 件の写し:（順番, 点, 綴り）。
type Found<'a> = (usize, DetectionPoint, &'a str);

/// 控えを（順番, 点, 綴り）にする。
fn pending(script: &str) -> Vec<Found<'_>> {
    let (_, trace) = strict(script, &FakeJudges);
    trace
        .pending
        .iter()
        .map(|(at, point, span)| (*at, *point, &script[span.clone()]))
        .collect()
}

/// 検出点の表の赤と緑の台本、と合図の形の違う台本の集まり。
const SAMPLES: &[&str] = &[
    "",
    "\\e",
    "\\-",
    "\\x",
    "\\s[0",
    "\\s[0]",
    "\\_w[abc]",
    "\\_w[500]",
    "\\![*]",
    "\\q[題,ID]",
    "\\![nosuch]",
    "\\![open,readme]",
    "\\![set,choicetimeout,abc]",
    "\\![set,choicetimeout,5000]",
    "\\f[sub,1]",
    "\\f[bold,1]",
    "\\f[colour,red]",
    "\\f[color,red]",
    "\\_a[x]あ\\_a[y]い\\_a",
    "\\_a[x]あ\\_a\\_a[y]い\\_a",
    "\\_a[x]",
    "\\_a[x]あ\\_a",
    "\\_a",
    "\\![move,nosuch]",
    "\\![move,--X=10]",
    "\\_u[0]",
    "\\_u[0x3000]",
    "あ\\_u[0x09]い",
    "\\h\\s[5]こんにちは\\w9\\n\\u\\s[10]\\b[2]やあ%username\\c\\_l[0,5em]",
    "あ\\![nosuch]い\\q[題,ID]\\q[次,ID2]\\![set,choicetimeout,abc]",
    "\\_w[1000]あ\\e\\![nosuch]",
    "あ\\-\\x",
];

/// 同じ命令の列から、strict かどうかに関わらず同じ合図の列と終わりの理由ができる（要件 3.9）。
#[test]
fn strict_scan_builds_the_same_cues() {
    for script in SAMPLES {
        let plain = compile(&parse(script), &SystemVarSnapshot::default());
        for judges in [&FakeJudges as &dyn StrictJudges, &AllWrong] {
            let (traced, trace) = strict(script, judges);
            let (a, b) = (plain.sheet.cues(), traced.sheet.cues());
            assert_eq!(a.len(), b.len(), "合図の数: {script}");
            assert!(
                a.iter().zip(b).all(|(x, y)| cue_eq(x, y)),
                "合図の列: {script}"
            );
            assert_eq!(plain.end, traced.end, "終わりの理由: {script}");
            let delivered = b
                .iter()
                .filter(|c| matches!(c.payload, CuePayload::Command(_)))
                .count();
            assert_eq!(trace.cue_spans.len(), delivered, "位置の数: {script}");
        }
    }
}

/// 検出点の表の赤の台本は、その点を 1 件だけ控える（全消しの後ろの順番 1、合図の無い台本は 0）。
#[test]
fn red_scripts_record_their_point() {
    use DetectionPoint::*;
    let cases: &[(&str, &[Found])] = &[
        ("\\x", &[(0, ReadUnknownTag, "\\x")]),
        ("\\s[0", &[(0, ReadUnclosed, "\\s[0")]),
        ("\\_w[abc]", &[(1, ReadDefaulted, "\\_w[abc]")]),
        ("\\![*]", &[(1, ReadMarkerIgnored, "\\![*]")]),
        ("\\![nosuch]", &[(1, CommandUnhandled, "\\![nosuch]")]),
        (
            "\\![move,nosuch]",
            &[(1, CommandUnhandled, "\\![move,nosuch]")],
        ),
        (
            "\\![set,choicetimeout,abc]",
            &[(1, ChoiceTimeoutUnreadable, "\\![set,choicetimeout,abc]")],
        ),
        ("\\f[sub,1]", &[(1, FontIgnored, "\\f[sub,1]")]),
        ("\\f[colour,red]", &[(1, FontUnknownKey, "\\f[colour,red]")]),
        // 重なり: 合図は ClearAll・開き x・あ・（足した閉じ）・開き y・い・閉じ。
        ("\\_a[x]あ\\_a[y]い\\_a", &[(3, AnchorReopened, "\\_a[y]")]),
        ("\\_a[x]", &[(1, AnchorUnclosed, "\\_a[x]")]),
        ("\\_a", &[(0, AnchorStrayClose, "\\_a")]),
        // 捨てた文字の番号（script-small-tags 要件 6.3）。合図を作らないので次に作られる合図の順番。
        ("\\_u[0]", &[(0, CharCodeDropped, "\\_u[0]")]),
        ("あ\\_m[abc]い", &[(2, CharCodeDropped, "\\_m[abc]")]),
    ];
    for (script, expected) in cases {
        assert_eq!(pending(script), expected.to_vec(), "{script}");
    }
}

/// 検出点の表の緑の台本は何も控えない。
#[test]
fn green_scripts_record_nothing() {
    for script in [
        "\\e",
        "\\s[0]",
        "\\_w[500]",
        "\\q[題,ID]",
        "\\![open,readme]",
        "\\![move,--X=10]",
        "\\![set,choicetimeout,5000]",
        "\\f[bold,1]",
        "\\f[color,red]",
        "\\_a[x]あ\\_a\\_a[y]い\\_a",
        "\\_a[x]あ\\_a",
        "\\_u[0x3000]",
        "\\_m[65]",
        "\\_u[ 0X22ee,1]",
    ] {
        assert_eq!(pending(script), [], "{script}");
    }
}

/// 同じ命令の 2 件はアンカーの判定が返す順（重なり → 閉じ無し）、命令をまたいでは台本の順。
#[test]
fn findings_follow_the_check_script_order() {
    use DetectionPoint::*;
    assert_eq!(
        pending("\\![nosuch]\\_a[x]あ\\_a[y]い\\x"),
        [
            (1, CommandUnhandled, "\\![nosuch]"),
            (4, AnchorReopened, "\\_a[y]"),
            (4, AnchorUnclosed, "\\_a[y]"),
            // 合図を作らない命令は次に作られる合図（走査の後で補った閉じ）の順番に結びつく。
            (7, ReadUnknownTag, "\\x"),
        ]
    );
}

/// `\e`・`\-` の後ろは控えない（要件 3.6）。
#[test]
fn nothing_after_end_or_quit() {
    assert_eq!(pending("\\e\\![nosuch]\\x"), []);
    assert_eq!(pending("あ\\-\\x\\_a"), []);
    assert_eq!(
        pending("\\x\\e\\x"),
        [(0, DetectionPoint::ReadUnknownTag, "\\x")]
    );
}

struct Recording(Arc<Mutex<Vec<TalkCue>>>);

impl CueSink for Recording {
    fn emit(&mut self, cue: TalkCue) {
        self.0.lock().unwrap().push(cue);
    }
}

/// `CuePlayer` が配った合図を、（合図の形, 台本の綴り）にして返す。
fn delivered(script: &str) -> (Vec<(String, Option<&str>)>, StrictTrace) {
    let (compiled, trace) = strict(script, &FakeJudges);
    let records = Arc::new(Mutex::new(Vec::new()));
    let mut player = CuePlayer::from_sheet(&compiled.sheet);
    player.register_sink(Box::new(Recording(records.clone())));
    player.tick(1.0e9);
    let cues = records.lock().unwrap().clone();
    assert_eq!(cues.len(), trace.cue_spans.len(), "配られた数と位置の数");
    let shaped = cues
        .iter()
        .zip(&trace.cue_spans)
        .map(|(cue, span)| {
            let shape = match &cue.command {
                CueCommand::ClearAll => "clear_all".to_string(),
                CueCommand::Text(s) => format!("text:{s}"),
                CueCommand::AnchorBegin { id, .. } => format!("begin:{id}"),
                CueCommand::AnchorEnd => "end".to_string(),
                CueCommand::Choice { id, .. } => format!("choice:{id}"),
                CueCommand::Custom { command, .. } => format!("custom:{command}"),
                other => format!("{other:?}"),
            };
            (shape, span.clone().map(|s| &script[s]))
        })
        .collect();
    (shaped, trace)
}

fn s(shape: &str, text: Option<&'static str>) -> (String, Option<&'static str>) {
    (shape.to_string(), text)
}

/// 先頭に全消しを持ち選択肢で終わる台本: 全消しは位置なしの順番 0、区切りは配られず数えない、
/// 補った閉じは位置なし。控えの順番は配られた列のその合図を指す（要件 2.5）。
#[test]
fn ordinals_match_delivery_with_clear_all_and_choice() {
    let script = "あ\\![nosuch]\\_a[x]い\\q[題,ID]";
    let (shapes, trace) = delivered(script);
    assert_eq!(
        shapes,
        [
            s("clear_all", None),
            s("text:あ", Some("あ")),
            s("custom:nosuch", Some("\\![nosuch]")),
            s("begin:x", Some("\\_a[x]")),
            s("text:い", Some("い")),
            s("choice:ID", Some("\\q[題,ID]")),
            s("end", None),
        ]
    );
    let at: Vec<_> = trace.pending.iter().map(|(at, p, _)| (*at, *p)).collect();
    assert_eq!(
        at,
        [
            (2, DetectionPoint::CommandUnhandled),
            (3, DetectionPoint::AnchorUnclosed),
        ]
    );
}

/// 選択肢の無い台本: 最後の合図の後ろの誤りは合図の数と同じ順番（配った後に残る）。
#[test]
fn ordinals_match_delivery_without_choice() {
    let script = "\\f[sub,1]あ\\x";
    let (shapes, trace) = delivered(script);
    assert_eq!(
        shapes,
        [
            s("clear_all", None),
            s("custom:\\f", Some("\\f[sub,1]")),
            s("text:あ", Some("あ")),
        ]
    );
    let at: Vec<_> = trace.pending.iter().map(|(at, p, _)| (*at, *p)).collect();
    assert_eq!(
        at,
        [
            (1, DetectionPoint::FontIgnored),
            (3, DetectionPoint::ReadUnknownTag),
        ]
    );
}
