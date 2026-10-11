//! アンカー `\_a` の台本写像の檻（anchor-tag-canon 要件 1.7〜1.10・2.5・2.10）。
//!
//! 台本の文字列だけから、次を確かめる:
//!
//! 1. 開きと閉じが **0 秒**の合図になり、あいだの文字・改行・装飾は今までどおり流れる
//!    （`\_a` の有無で他の合図の時刻と長さが変わらない）。
//! 2. 崩れた形の補い: 重なりは新しい開きの**前**に閉じを足す／閉じ無しは走査の終わり
//!    （`\e`・`\-`・末尾）に閉じを足す／迷子の閉じは合図を出さない。
//! 3. 警告は崩れ 1 件につき **1 件**（`anchor_reopened`／`anchor_unclosed`／
//!    `anchor_stray_close`・位置と ID 付き）。崩れの無い台本には警告が無い。
//! 4. アンカーだけの台本は選択肢の答えを待つ柵を出さない。
//!
//! 位置（`index`）は読み手が返す命令の列の添字（0 始まり）。

use super::test_support::{assert_clear_all_prefix_and_rest, compile, cue_eq};
use super::*;
use areka_parsers::sakura::parse;

/// 警告 1 件の写し: (event の名前, 位置, ID)。ID の欄が無い警告は `None`。
type Warned = (String, String, Option<String>);

fn warned(event: &str, index: usize, id: Option<&str>) -> Warned {
    (event.into(), index.to_string(), id.map(str::to_string))
}

/// 台本を読んで compile し、その間に出た警告（水準 WARN の全件）と一緒に返す。
fn compile_script(script: &str) -> (CompiledTalk, Vec<Warned>) {
    let (compiled, events) = log_capture_kit::capture(|| compile(&parse(script)));
    let warnings = events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .map(|e| {
            (
                e.field_str("event").unwrap_or("").to_string(),
                e.field("index").unwrap_or("").to_string(),
                e.field("id").map(str::to_string),
            )
        })
        .collect();
    (compiled, warnings)
}

/// 先頭の `ClearAll` を除いた合図の列を、種類と中身の短い綴りにする。
fn shape(compiled: &CompiledTalk) -> Vec<String> {
    assert_clear_all_prefix_and_rest(compiled.sheet.cues())
        .iter()
        .map(|cue| match &cue.payload {
            CuePayload::Command(CueCommand::AnchorBegin { id, references }) => {
                format!("begin:{id}:{}", references.join("|"))
            }
            CuePayload::Command(CueCommand::AnchorEnd) => "end".to_string(),
            CuePayload::Command(CueCommand::Text(s)) => format!("text:{s}"),
            CuePayload::Command(CueCommand::NewLine { .. }) => "newline".to_string(),
            CuePayload::Command(CueCommand::Choice { id, .. }) => format!("choice:{id}"),
            CuePayload::Command(CueCommand::Custom { command, .. }) => format!("custom:{command}"),
            CuePayload::Barrier(_) => "barrier".to_string(),
            other => format!("{other:?}"),
        })
        .collect()
}

/// 合図がアンカーの開きか閉じか。
fn is_anchor(cue: &Cue) -> bool {
    matches!(
        &cue.payload,
        CuePayload::Command(CueCommand::AnchorBegin { .. } | CueCommand::AnchorEnd)
    )
}

/// 台本の占有の終わり（`max(start_time + duration)`）。
fn horizon(compiled: &CompiledTalk) -> f64 {
    compiled
        .sheet
        .cues()
        .iter()
        .map(|cue| cue.start_time + cue.duration)
        .fold(0.0, f64::max)
}

/// 4 つの形（ID だけ・引数付き・`On` 始まり・閉じ）と空の ID が、記述順のまま開きと閉じの
/// 合図になる。引数は空のトークンも潰さない（要件 1.7）。
#[test]
fn four_forms_become_anchor_cues_in_script_order() {
    let (compiled, warnings) =
        compile_script(r"\_a[x]あ\_a\_a[y,r2,r3]い\_a\_a[OnJump,r0,,r1]う\_a\_a[]え\_a");
    assert_eq!(
        shape(&compiled),
        [
            "begin:x:",
            "text:あ",
            "end",
            "begin:y:r2|r3",
            "text:い",
            "end",
            "begin:OnJump:r0||r1",
            "text:う",
            "end",
            "begin::",
            "text:え",
            "end",
        ]
    );
    assert_eq!(warnings, [], "対になっている台本に警告は無い");
}

/// 開きと閉じは 0 秒で、`\_a` の有無で他の合図の時刻と長さが変わらない。開きは直後の文字の
/// 書き出しの時刻、閉じは直前の文字の出終わりの時刻に並ぶ。
#[test]
fn anchor_cues_take_no_time_and_do_not_shift_the_others() {
    let (with_anchor, _) = compile_script(r"あ\_a[x]い\_aう");
    // 比べる相手: 同じ所で文字が切れるよう、持ち主の無いタグ `\_n`（合図にならない）を挟む。
    let (without_anchor, _) = compile_script(r"あ\_nい\_nう");
    let cues = assert_clear_all_prefix_and_rest(with_anchor.sheet.cues());
    // 0: あ / 1: 開き / 2: い / 3: 閉じ / 4: う
    assert_eq!(
        shape(&with_anchor),
        ["text:あ", "begin:x:", "text:い", "end", "text:う"]
    );
    for cue in cues.iter().filter(|cue| is_anchor(cue)) {
        assert_eq!(cue.duration, 0.0, "アンカーの合図は 0 秒");
    }
    assert_eq!(cues[1].start_time, cues[2].start_time, "開きは「い」の頭");
    assert_eq!(
        cues[3].start_time,
        cues[2].start_time + cues[2].duration,
        "閉じは「い」の出終わり"
    );
    assert_eq!(cues[3].start_time, cues[4].start_time, "閉じは「う」の頭");
    let others: Vec<&Cue> = cues.iter().filter(|cue| !is_anchor(cue)).collect();
    let control = assert_clear_all_prefix_and_rest(without_anchor.sheet.cues());
    assert_eq!(shape(&without_anchor), ["text:あ", "text:い", "text:う"]);
    assert!(
        others.len() == control.len() && others.iter().zip(control).all(|(a, b)| cue_eq(a, b)),
        "アンカーの有無で他の合図の時刻と長さは変わらない"
    );
}

/// あいだの文字・改行・装飾の合図は今までどおり流れ、合図は書いたスコープの宛先になる
/// （要件 1.7）。
#[test]
fn text_newline_and_decoration_flow_between_open_and_close() {
    let (compiled, warnings) = compile_script(r"\1\_a[x]あ\n\f[bold,1]い\_a");
    assert_eq!(
        shape(&compiled),
        [
            "begin:x:",
            "text:あ",
            "newline",
            r"custom:\f",
            "text:い",
            "end"
        ]
    );
    for cue in assert_clear_all_prefix_and_rest(compiled.sheet.cues()) {
        assert_eq!(cue.actor.as_str(), "1");
    }
    assert_eq!(warnings, []);
}

/// 開いている間の新たな開き: 新しい開きの前に閉じを足し、警告を 1 件残す（要件 1.9）。
#[test]
fn reopened_anchor_is_closed_before_the_new_open() {
    // 0: 開き x / 1: あ / 2: 開き y（重なり）/ 3: い / 4: 閉じ
    let (compiled, warnings) = compile_script(r"\_a[x]あ\_a[y]い\_a");
    assert_eq!(
        shape(&compiled),
        ["begin:x:", "text:あ", "end", "begin:y:", "text:い", "end"]
    );
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(cues[2].duration, 0.0, "足した閉じも 0 秒");
    assert_eq!(
        cues[2].start_time, cues[3].start_time,
        "足した閉じは新しい開きと同じ時刻（並びは開きの前）"
    );
    assert_eq!(warnings, [warned("anchor_reopened", 2, Some("y"))]);
}

/// 閉じの無い開き: 走査の終わり（末尾・`\e`・`\-`）に閉じを足し、警告を 1 件残す
/// （要件 1.8）。`\e`・`\-` の後ろは読まない（後ろの閉じでは閉じない）。
#[test]
fn unclosed_anchor_is_closed_at_the_end_of_the_scan() {
    for (script, end) in [
        (r"\_a[x]あ", TalkEndReason::Ended),
        (r"\_a[x]あ\eい\_a", TalkEndReason::Ended),
        (r"\_a[x]あ\-い\_a", TalkEndReason::Quit),
    ] {
        let (compiled, warnings) = compile_script(script);
        assert_eq!(
            shape(&compiled),
            ["begin:x:", "text:あ", "end"],
            "script = {script:?}"
        );
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        assert_eq!(cues[2].duration, 0.0, "足した閉じも 0 秒（{script:?}）");
        assert_eq!(
            cues[2].start_time,
            horizon(&compiled),
            "足した閉じは表示の終わり（{script:?}）"
        );
        assert_eq!(compiled.end, end, "script = {script:?}");
        assert_eq!(
            warnings,
            [warned("anchor_unclosed", 0, Some("x"))],
            "script = {script:?}"
        );
    }
}

/// 重なりの開きが閉じられないまま終わる: その開きの位置に重なりと閉じ無しの 2 件が付く。
/// 閉じは新しい開きの前と走査の終わりの 2 か所に足し、警告はそれぞれ 1 件（計 2 件）。
#[test]
fn reopened_and_never_closed_gets_both_supplied_closes_and_two_warnings() {
    // 0: 開き x / 1: あ / 2: 開き y（重なり・閉じ無し）/ 3: い
    let (compiled, warnings) = compile_script(r"\_a[x]あ\_a[y]い");
    assert_eq!(
        shape(&compiled),
        ["begin:x:", "text:あ", "end", "begin:y:", "text:い", "end"]
    );
    assert_eq!(
        warnings,
        [
            warned("anchor_reopened", 2, Some("y")),
            warned("anchor_unclosed", 2, Some("y")),
        ]
    );
}

/// 開いていないのに閉じ: 合図を出さずに表示を続け、警告を 1 件残す（要件 1.10）。
#[test]
fn stray_close_emits_nothing() {
    // 0: あ / 1: 閉じ（迷子）/ 2: い
    let (compiled, warnings) = compile_script(r"あ\_aい");
    assert_eq!(shape(&compiled), ["text:あ", "text:い"]);
    assert_eq!(warnings, [warned("anchor_stray_close", 1, None)]);

    // 対を閉じた後の余分な閉じも同じ。0: 開き / 1: あ / 2: 閉じ / 3: 閉じ（迷子）
    let (compiled, warnings) = compile_script(r"\_a[x]あ\_a\_a");
    assert_eq!(shape(&compiled), ["begin:x:", "text:あ", "end"]);
    assert_eq!(warnings, [warned("anchor_stray_close", 3, None)]);

    // 迷子の閉じだけの台本は合図が 1 つも無い（空の台本と同じ）。
    let (compiled, warnings) = compile_script(r"\_a");
    assert!(compiled.sheet.is_empty());
    assert_eq!(warnings, [warned("anchor_stray_close", 0, None)]);
}

/// 複数の崩れが混ざっても、警告は崩れ 1 件につき 1 件で、台本の順に並ぶ。
#[test]
fn mixed_breakages_warn_once_each_in_script_order() {
    // 0: 閉じ（迷子）/ 1: 開き x / 2: あ / 3: 開き y（重なり）/ 4: い / 5: 閉じ
    // 6: 閉じ（迷子）/ 7: 開き z（閉じ無し）/ 8: う
    let (compiled, warnings) = compile_script(r"\_a\_a[x]あ\_a[y]い\_a\_a\_a[z]う");
    assert_eq!(
        shape(&compiled),
        [
            "begin:x:", "text:あ", "end", "begin:y:", "text:い", "end", "begin:z:", "text:う",
            "end",
        ]
    );
    assert_eq!(
        warnings,
        [
            warned("anchor_stray_close", 0, None),
            warned("anchor_reopened", 3, Some("y")),
            warned("anchor_stray_close", 6, None),
            warned("anchor_unclosed", 7, Some("z")),
        ]
    );
}

/// 崩れの無い台本には警告が 1 件も無い（`\e` の後ろの開き・閉じは読まないので数えない）。
/// 捕まえる道具が働いていることは、同じ道具で警告を数える上の檻が示す。
#[test]
fn well_formed_scripts_have_no_warning() {
    for script in [
        "あいう",
        r"\_a[x]い\_a",
        r"あ\_a[x]い\_aう\_a[OnY,1,2]え\_aお",
        r"\_a[x]\_a\_a[]い\_a",
        r"\_a[x]い\n\f[bold,1]う\q[題,ID]\_a",
        r"\_a[x]い\_a\e\_a\_a[y]う",
    ] {
        let (_, warnings) = compile_script(script);
        assert_eq!(warnings, [], "script = {script:?}");
    }
}

/// アンカーだけの台本は選択肢の答えを待つ柵を出さない（要件 2.5）。選択肢がある台本の柵は
/// 今までどおり 1 つで、足した閉じより後ろ（列の最後）に並ぶ。
#[test]
fn anchors_alone_do_not_raise_the_choice_fence() {
    for script in [r"\_a[x]あ\_a", r"\_a[x]あ", r"\_a[x]あ\_a[y]い"] {
        let (compiled, _) = compile_script(script);
        assert!(
            !shape(&compiled).contains(&"barrier".to_string()),
            "アンカーだけの台本に柵は無い: {script:?}"
        );
    }

    let (compiled, _) = compile_script(r"\_a[x]\q[はい,OnYes]\_a");
    assert_eq!(
        shape(&compiled),
        ["begin:x:", "choice:OnYes", "end", "barrier"]
    );
    // 閉じ無しでも、足した閉じは柵の前。
    let (compiled, _) = compile_script(r"\_a[x]\q[はい,OnYes]");
    assert_eq!(
        shape(&compiled),
        ["begin:x:", "choice:OnYes", "end", "barrier"]
    );
}
