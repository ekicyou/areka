//! 「．さくら」のシェル `master` で 8 語に由来する WARN が 0 件であることの檻
//! （spec: seriko-pattern-calls 要件 1.4・9.3）。
//!
//! 検体の `surfaces.txt` を解析を経る実経路で読み、再生表と合成の形の記録（`shape_issues`）を作る。
//! 検体は共有の受け口 `sample_test_support` 経由でだけ受ける。

use areka_emo_compose::{EmoWorld, PatternWord, ShapeIssue, pattern_word, shape_issues};
use areka_parsers::charset::{DefaultEncoding, decode};
use log_capture_kit::{LineFormat, capture_lines};

use super::FrameAct;
use crate::sample_test_support::dot_sakura_shell_root;
use crate::table::AnimationTable;

/// `check_calls` の 6 種の記録の目印（要件の番号）。
const KINDS: [&str; 6] = [
    "要件 2.5",
    "要件 5.1",
    "要件 5.3",
    "要件 6.6",
    "要件 6.5",
    "要件 6.4",
];

/// 8 語の綴り（合成の「未知の合成メソッド名」の記録から 8 語の分を見分ける）。
const WORDS: [&str; 8] = [
    "start",
    "stop",
    "alternativestart",
    "alternativestop",
    "parallelstart",
    "parallelstop",
    "insert",
    "move",
];

/// 本文から世界・再生表・形の記録を作り、面の番号と同じ走行の記録（1 イベント 1 行）を添えて返す。
fn build(text: &str) -> (AnimationTable, Vec<u32>, Vec<ShapeIssue>, Vec<String>) {
    let ((table, ids, issues), lines) = capture_lines(LineFormat::LevelTargetFields, || {
        let world = EmoWorld::build(&areka_parsers::shell::parse(text));
        let ids = world.surface_ids().collect();
        (
            AnimationTable::from_world(&world),
            ids,
            shape_issues(&world),
        )
    });
    (table, ids, issues, lines)
}

/// 8 語に由来する WARN の行（`check_calls` の 6 種と、8 語を語にした「未知の合成メソッド名」）。
fn word_warns(lines: &[String]) -> Vec<&String> {
    lines
        .iter()
        .filter(|l| l.contains("WARN"))
        .filter(|l| {
            KINDS.iter().any(|k| l.contains(k))
                || (l.contains("未知の合成メソッド名")
                    && WORDS
                        .iter()
                        .any(|w| l.to_ascii_lowercase().contains(&format!("method={w}"))))
        })
        .collect()
}

/// 「．さくら」の `surfaces.txt` 本文（`charset` の無い Shift_JIS）。
fn dot_sakura_text() -> String {
    let path = dot_sakura_shell_root().join("surfaces.txt");
    let bytes = std::fs::read(&path).expect("検体の surfaces.txt は読める");
    decode(&bytes, DefaultEncoding::Ansi)
}

/// 8 語に由来する WARN が 0 件で、形の記録にも 8 語のコマが載らない（要件 1.4・9.3）。
#[test]
fn dot_sakura_has_no_warn_from_the_eight_words() {
    let (table, ids, issues, lines) = build(&dot_sakura_text());

    // 0 件が空振りでないこと: 検体は 8 語（alternativestart・move）を実際に持つ。
    let acts: Vec<&FrameAct> = ids
        .iter()
        .flat_map(|&s| table.animations(s))
        .flat_map(|a| a.frames.iter().map(|f| &f.act))
        .collect();
    assert!(
        acts.iter().any(|a| matches!(a, FrameAct::Start { .. })),
        "検体に alternativestart がある"
    );
    assert!(
        acts.iter().any(|a| matches!(a, FrameAct::Move)),
        "検体に move がある"
    );

    let warns = word_warns(&lines);
    assert!(warns.is_empty(), "8 語に由来する WARN: {warns:#?}");
    let undrawn: Vec<&ShapeIssue> = issues
        .iter()
        .filter(|i| {
            matches!(i, ShapeIssue::UndrawnPattern { method, .. }
                if pattern_word(method) != PatternWord::Draw)
        })
        .collect();
    assert!(undrawn.is_empty(), "8 語の形の記録: {undrawn:#?}");
}

/// `surface0` の animation2 は「並び [0, 1] から 1 つを選んで開始」（要件 1.4）。
#[test]
fn dot_sakura_surface0_animation2_starts_one_of_0_and_1() {
    let (table, _, _, _) = build(&dot_sakura_text());
    let anim = table
        .animations(0)
        .iter()
        .find(|a| a.id == 2)
        .expect("surface0 の animation2 が表に載る");
    let acts: Vec<&FrameAct> = anim.frames.iter().map(|f| &f.act).collect();
    assert_eq!(
        acts,
        [&FrameAct::Start {
            targets: [0, 1].into(),
            pick_one: true,
        }]
    );
}

/// 目印の較正: 同じ見分けが 6 種の 1 つ（読めない相手）を拾う。
#[test]
fn word_warn_filter_catches_an_unreadable_target() {
    let (_, _, _, lines) = build(
        "surface10\n{\nanimation0.interval,random,2\n\
         animation0.pattern0,overlay,100,50,0,0\n\
         animation0.pattern1,start,abc\n}\n",
    );
    assert_eq!(word_warns(&lines).len(), 1, "{lines:#?}");
}
