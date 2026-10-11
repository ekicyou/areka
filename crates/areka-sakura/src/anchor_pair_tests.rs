//! `pair_anchors` の檻（anchor-tag-canon 要件 1.8〜1.10）。
//!
//! 台本の文字列だけから、開きと閉じの対応の崩れ（閉じ無し・重なり・迷子の閉じ）の位置と種類、
//! 並びの順、`\e`／`\-` の後ろを数えないことを確かめる。位置は読み手が返す命令の列の添字。

use super::*;
use AnchorIssue::{Reopened, StrayClose, Unclosed};
use areka_parsers::sakura::parse;

/// 台本を読んで対応を判定し、(位置, 種類) の列にして返す。
fn findings(script: &str) -> Vec<(usize, AnchorIssue)> {
    pair_anchors(&parse(script))
        .into_iter()
        .map(|f| (f.index, f.issue))
        .collect()
}

#[test]
fn well_paired_scripts_have_no_findings() {
    for script in [
        "",
        "あいう",
        r"\_a[x]い\_a",
        r"あ\_a[x]い\_aう\_a[OnY,1,2]え\_aお",
        // 中身が空の対・ID が空の開きも、対になっていれば崩れではない。
        r"\_a[x]\_a\_a[]い\_a",
        // あいだに改行・装飾・選択肢があっても対応は変わらない。
        r"\_a[x]い\n\f[bold,1]う\q[題,ID]\_a",
    ] {
        assert_eq!(findings(script), vec![], "script = {script:?}");
    }
}

#[test]
fn open_without_close_is_unclosed_at_the_open() {
    // 0: Text / 1: Anchor / 2: Text
    assert_eq!(findings(r"あ\_a[x]い"), vec![(1, Unclosed)]);
    // 開きだけの台本。
    assert_eq!(findings(r"\_a[x]"), vec![(0, Unclosed)]);
}

#[test]
fn open_while_open_is_reopened_at_the_new_open() {
    // 0: Anchor x / 1: Text / 2: Anchor y / 3: Text / 4: AnchorEnd
    // 最後の閉じは y を閉じるので、迷子にも閉じ無しにもならない。
    assert_eq!(findings(r"\_a[x]い\_a[y]う\_a"), vec![(2, Reopened)]);
    // 3 つ続けば、2 つ目と 3 つ目がそれぞれ重なり。
    assert_eq!(
        findings(r"\_a[x]\_a[y]\_a[z]\_a"),
        vec![(1, Reopened), (2, Reopened)]
    );
}

#[test]
fn close_while_not_open_is_stray_at_the_close() {
    // 0: Text / 1: AnchorEnd / 2: Text
    assert_eq!(findings(r"あ\_aい"), vec![(1, StrayClose)]);
    // 対を閉じた後の余分な閉じ。0: Anchor / 1: Text / 2: AnchorEnd / 3: AnchorEnd
    assert_eq!(findings(r"\_a[x]い\_a\_a"), vec![(3, StrayClose)]);
    // 迷子の閉じは後ろの開きを閉じない（後ろの対は崩れではない）。
    assert_eq!(findings(r"\_a\_a[x]い\_a"), vec![(0, StrayClose)]);
}

#[test]
fn mixed_breakages_come_in_script_order() {
    //  0: AnchorEnd（迷子）
    //  1: Anchor x / 2: Text / 3: Anchor y（重なり）/ 4: Text / 5: AnchorEnd
    //  6: AnchorEnd（迷子）/ 7: Text
    //  8: Anchor z / 9: Text / 10: Anchor w（重なり・閉じ無し）
    let got = findings(r"\_a\_a[x]あ\_a[y]い\_a\_aう\_a[z]え\_a[w]");
    assert_eq!(
        got,
        vec![
            (0, StrayClose),
            (3, Reopened),
            (6, StrayClose),
            (10, Reopened),
            (10, Unclosed),
        ]
    );
    // 位置の昇順。同じ位置に 2 件付くのは「重なりの開きが閉じられないまま終わる」ときだけで、
    // そのときも同じ種類は重ならない。
    assert!(got.windows(2).all(|w| w[0].0 <= w[1].0));
    assert!(got.windows(2).all(|w| w[0] != w[1]));
}

#[test]
fn nothing_is_counted_after_end_or_quit() {
    for stop in [r"\e", r"\-"] {
        // 止まる前に対が閉じていれば、後ろの迷子の閉じ・閉じ無しの開きは数えない。
        assert_eq!(
            findings(&format!(r"\_a[x]あ\_a{stop}\_a\_a[y]い")),
            vec![],
            "stop = {stop:?}"
        );
        // 止まる所で開いたままなら閉じ無し。後ろの閉じはこの開きを閉じない。
        // 0: Anchor / 1: Text / 2: End または Quit / 3: AnchorEnd
        assert_eq!(
            findings(&format!(r"\_a[x]あ{stop}\_a")),
            vec![(0, Unclosed)],
            "stop = {stop:?}"
        );
        // 後ろの開きは重なりにも数えない。
        assert_eq!(
            findings(&format!(r"\_a[x]あ{stop}\_a[y]")),
            vec![(0, Unclosed)],
            "stop = {stop:?}"
        );
    }
}

#[test]
fn accepts_any_iterator_of_instruction_refs() {
    // `check_script` は読み取りの列から命令への参照を取り出して渡す（スライスに限らない）。
    let instructions = parse(r"あ\_a");
    let got = pair_anchors(instructions.iter());
    assert_eq!(
        got,
        vec![AnchorFinding {
            index: 1,
            issue: StrayClose
        }]
    );
}
