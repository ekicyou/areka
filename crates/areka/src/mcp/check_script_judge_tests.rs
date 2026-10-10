//! 判断の決定論テスト（要件 3.2〜3.4・3.8・3.10・3.12・3.13）。事実は手書きの表で与え、
//! 台本は再生と同じ `parse_noted` で読む。種類の表（`doc/ssp-mcp/areka-tools.md` ⑶）の 17 行それぞれに「出る台本」「出ない台本」を置く。

use std::collections::BTreeMap;

use areka_parsers::sakura::parse_noted;
use areka_seriko::SurfaceResolver;

use super::*;

/// 手書きの事実の表。スコープ 0・1 に絵とバルーンが在り、スコープ 2 には何も無い。
struct Table {
    resolver: SurfaceResolver,
    /// スコープごとのシェルの生の surface ID。
    shells: &'static [(u32, &'static [u32])],
    /// スコープごとのバルーンの面の ID。
    balloons: &'static [(u32, &'static [u32])],
}

fn has(table: &[(u32, &[u32])], scope: u32, id: u32) -> Option<bool> {
    table
        .iter()
        .find(|(s, _)| *s == scope)
        .map(|(_, ids)| ids.contains(&id))
}

impl ScriptFacts for Table {
    fn resolve_surface(&self, key: &str) -> SurfaceTarget {
        self.resolver.resolve(key)
    }
    fn shell_has(&self, scope: u32, surface_id: u32) -> Option<bool> {
        has(self.shells, scope, surface_id)
    }
    fn balloon_has(&self, scope: u32, balloon_id: u32) -> Option<bool> {
        has(self.balloons, scope, balloon_id)
    }
}

/// スコープ 0 の絵は 0・3・10、スコープ 1（相方）の絵は 10・11。別名 `笑顔` は 3。
/// バルーンはスコープ 0 が 0・2、スコープ 1 が 1（相方の側はシェルもバルーンも ID 0 を持たない）。
fn ghost() -> Table {
    let mut aliases = BTreeMap::new();
    aliases.insert("笑顔".to_string(), vec![3]);
    Table {
        resolver: SurfaceResolver::new(aliases),
        shells: &[(0, &[0, 3, 10]), (1, &[10, 11])],
        balloons: &[(0, &[0, 2]), (1, &[1])],
    }
}

fn check_with(script: &str, facts: Option<&dyn ScriptFacts>) -> Vec<Diagnostic> {
    diagnose(
        script,
        &parse_noted(script),
        &ConsumerLedger::canonical(),
        facts,
    )
}

fn check(script: &str) -> Vec<Diagnostic> {
    check_with(script, Some(&ghost()))
}

/// タグ 1 つの台本が、そのタグ全体を指す診断をちょうど 1 件返す（要件 3.13）。
fn fires(script: &str, kind: Kind, message: &str) {
    let got = check(script);
    assert_eq!(got.len(), 1, "{script}: {got:?}");
    let d = &got[0];
    assert_eq!((d.kind, d.message), (kind, message), "{script}");
    assert_eq!(d.text, script, "{script}");
    assert_eq!((d.start, d.end), (0, script.chars().count()), "{script}");
}

fn silent(script: &str) {
    assert_eq!(check(script), Vec::new(), "{script}");
}

// ---- 種類の表の 14 行 ----

#[test]
fn row01_bare_unknown_tag() {
    fires(r"\x", Kind::UnknownTag, MSG_UNKNOWN_TAG);
    silent(r"\e");
}

#[test]
fn row02_bracketed_unknown_tag_including_i_and_entity() {
    fires(r"\i[5]", Kind::UnknownTag, MSG_UNKNOWN_TAG);
    fires(r"\&[amp]", Kind::UnknownTag, MSG_UNKNOWN_TAG);
    silent(r"\s[0]");
}

#[test]
fn row03_legacy_two_bracket_q() {
    fires(r"\q[ID][題]", Kind::UnknownTag, MSG_UNKNOWN_TAG);
    silent(r"\q[題,ID]");
}

#[test]
fn row04_bang_name_nobody_handles() {
    fires(r"\![nosuch]", Kind::UnknownCommand, MSG_UNKNOWN_COMMAND);
    silent(r"\![open,readme]");
    // `\j` は表の担当あり（`open` と同じ受け口）。
    silent(r"\j[http://a/]");
}

#[test]
fn row05_bang_first_argument_nobody_handles() {
    fires(r"\![set,nosuch]", Kind::UnknownCommand, MSG_UNKNOWN_COMMAND);
    silent(r"\![set,zorder,0,1]");
}

#[test]
fn row06_raw_surface_id_missing_or_out_of_range() {
    fires(r"\s[99999]", Kind::MissingSurface, MSG_MISSING_SURFACE);
    fires(r"\s[-2]", Kind::MissingSurface, MSG_MISSING_SURFACE);
    silent(r"\s[0]");
    silent(r"\s[-1]");
}

#[test]
fn row07_surface_alias_missing() {
    fires(r"\s[無い名前]", Kind::MissingSurface, MSG_MISSING_SURFACE);
    silent(r"\s[笑顔]");
}

#[test]
fn row08_balloon_id_missing_or_out_of_range() {
    fires(r"\b[99]", Kind::MissingBalloon, MSG_MISSING_BALLOON);
    fires(r"\b[-2]", Kind::MissingBalloon, MSG_MISSING_BALLOON);
    silent(r"\b[0]");
    silent(r"\b[-1]");
    // 名前の形は、そのとき表示しているサーフェスの箱に依るので診ない。
    silent(r"\b[名前]");
}

#[test]
fn row09_unclosed_bracket() {
    fires(r"\s[0", Kind::UnreadableArgument, MSG_UNCLOSED);
    silent(r"\s[0]");
}

#[test]
fn row10_argument_defaulted_by_the_reader() {
    for script in [r"\_w[abc]", r"\n[abc]", r"\p[x]", r"\q[題]"] {
        fires(script, Kind::UnreadableArgument, MSG_DEFAULTED);
    }
    for script in [r"\_w[500]", r"\n[half]", r"\p[1]", r"\q[題,ID]"] {
        silent(script);
    }
}

#[test]
fn row11_unreadable_choice_timeout() {
    fires(
        r"\![set,choicetimeout,abc]",
        Kind::UnreadableArgument,
        MSG_DEFAULTED,
    );
    silent(r"\![set,choicetimeout,5000]");
}

#[test]
fn row12_choice_marker() {
    fires(r"\![*]", Kind::Ignored, MSG_IGNORED);
    silent(r"\q[題,ID]");
}

#[test]
fn row13_font_key_accepted_without_effect() {
    for script in [r"\f[sub,1]", r"\f[align,center]", r"\f[height,large]"] {
        fires(script, Kind::Ignored, MSG_IGNORED);
    }
    silent(r"\f[bold,1]");
}

#[test]
fn row14_unknown_or_missing_font_key() {
    fires(r"\f[colour,red]", Kind::UnknownTag, MSG_UNKNOWN_TAG);
    fires(r"\f[]", Kind::UnknownTag, MSG_UNKNOWN_TAG);
    silent(r"\f[color,red]");
    // 値の誤りは受け口が読む引数なので診ない。
    silent(r"\f[bold,abc]");
}

// ---- アンカー `\_a`（anchor-tag-canon 要件 6.1〜6.3）: 表の 15〜17 行目 ----

/// 種類・位置・綴り・文言の並び（アンカーの診断の見比べ用）。
fn rows(script: &str) -> Vec<(Kind, usize, usize, String, &'static str)> {
    check(script)
        .into_iter()
        .map(|d| (d.kind, d.start, d.end, d.text, d.message))
        .collect()
}

/// 4 つの形（ID だけ・引数つき・`On` 始まり・閉じ）は、知らないタグとも誰も拾わない命令とも言わない。
#[test]
fn the_four_anchor_forms_are_not_reported() {
    silent(r"\_a[Hint]あ\_a\_a[ID,r2,r3]い\_a\_a[OnTest,r0,r1]う\_a");
}

#[test]
fn row15_anchor_without_a_close() {
    fires(r"\_a[x]", Kind::UnpairedTag, MSG_ANCHOR_UNCLOSED);
    silent(r"\_a[x]あ\_a");
}

#[test]
fn row16_anchor_opened_while_another_is_open() {
    // 診断は新しい開きに付く（直前の開きには付かない）。
    assert_eq!(
        rows(r"\_a[x]あ\_a[y]い\_a"),
        [(
            Kind::UnpairedTag,
            7,
            13,
            r"\_a[y]".to_string(),
            MSG_ANCHOR_REOPENED
        )]
    );
    silent(r"\_a[x]あ\_a\_a[y]い\_a");
}

#[test]
fn row17_close_without_an_open_anchor() {
    fires(r"\_a", Kind::UnpairedTag, MSG_ANCHOR_STRAY_CLOSE);
    silent(r"\_a[x]あ\_a");
}

/// 重なった開きが閉じられないまま終わると、その開きに 2 件（重なり → 閉じ無し の順）が付く。
#[test]
fn reopened_and_never_closed_gives_two_diagnostics_on_the_second_open() {
    let at = |message| (Kind::UnpairedTag, 7, 13, r"\_a[y]".to_string(), message);
    assert_eq!(
        rows(r"\_a[x]あ\_a[y]い"),
        [at(MSG_ANCHOR_REOPENED), at(MSG_ANCHOR_UNCLOSED)]
    );
}

/// アンカーの診断は、他の種類の診断と混ざっても台本の順のまま並ぶ。
#[test]
fn anchor_diagnostics_stay_in_script_order() {
    assert_eq!(
        rows(r"\_a\x\_a[y]")
            .iter()
            .map(|(_, start, _, _, message)| (*start, *message))
            .collect::<Vec<_>>(),
        [
            (0, MSG_ANCHOR_STRAY_CLOSE),
            (3, MSG_UNKNOWN_TAG),
            (5, MSG_ANCHOR_UNCLOSED),
        ]
    );
}

/// `\e`・`\-` の後ろの開き／閉じには出さない（再生はそこで読むのをやめる）。手前の開きは閉じ無し。
#[test]
fn anchors_after_end_or_quit_are_not_reported() {
    for stop in [r"\e", r"\-"] {
        silent(&format!(r"{stop}\_a[x]あ"));
        silent(&format!(r"{stop}\_a"));
        silent(&format!(r"{stop}\_a[x]あ\_a[y]い"));
        assert_eq!(
            rows(&format!(r"\_a[x]あ{stop}\_a")),
            [(
                Kind::UnpairedTag,
                0,
                6,
                r"\_a[x]".to_string(),
                MSG_ANCHOR_UNCLOSED
            )],
            "{stop}"
        );
    }
}

// ---- 位置・スコープ・診ない規則 ----

#[test]
fn positions_are_counted_in_characters() {
    let script = r"あいう\xえお\s[99999]か";
    let got = check(script);
    let spans: Vec<_> = got
        .iter()
        .map(|d| (d.kind, d.start, d.end, d.text.as_str()))
        .collect();
    assert_eq!(
        spans,
        vec![
            (Kind::UnknownTag, 3, 5, r"\x"),
            (Kind::MissingSurface, 7, 16, r"\s[99999]"),
        ]
    );
}

#[test]
fn two_notes_on_one_instruction_give_two_diagnostics_at_the_same_place() {
    let script = r"字\![*]\q[題]";
    let got = check(script);
    assert_eq!(got.len(), 2, "{got:?}");
    let mut kinds: Vec<_> = got.iter().map(|d| d.kind.as_str()).collect();
    kinds.sort();
    assert_eq!(kinds, ["ignored", "unreadable_argument"]);
    for d in &got {
        assert_eq!((d.start, d.end, d.text.as_str()), (1, 11, r"\![*]\q[題]"));
    }
}

#[test]
fn surface_and_balloon_are_judged_in_the_current_scope() {
    // 相方（スコープ 1）の絵は 10・11。11 はスコープ 0 には無い。
    assert_eq!(check(r"\1\s[11]"), Vec::new());
    assert_eq!(check(r"\u\s[11]\p[1]\s[10]"), Vec::new());
    assert_eq!(check(r"\1\s[3]")[0].kind, Kind::MissingSurface);
    assert_eq!(check(r"\1\s[11]\0\s[11]")[0].start, 10);
    assert_eq!(check(r"\1\s[11]\h\s[11]").len(), 1);
    // 相方のバルーンは 1 だけ。
    assert_eq!(check(r"\b[2]"), Vec::new());
    assert_eq!(check(r"\1\b[2]")[0].kind, Kind::MissingBalloon);
}

/// 解けない `\s`・不正な `\b` は、ID 0 を持たないスコープでも、絵・バルーンが在れば知らせる
/// （スコープの有無の問い合わせが「ID 0 が在るか」にすり替わっていないこと）。
#[test]
fn unresolved_keys_are_judged_in_a_scope_without_id_zero() {
    for (script, kind) in [
        (r"\1\s[-2]", Kind::MissingSurface),
        (r"\1\s[無い名前]", Kind::MissingSurface),
        (r"\1\b[-2]", Kind::MissingBalloon),
    ] {
        let got = check(script);
        assert_eq!(
            got.iter().map(|d| d.kind).collect::<Vec<_>>(),
            [kind],
            "{script}"
        );
    }
}

#[test]
fn scope_without_shell_or_balloon_is_not_judged() {
    assert_eq!(
        check(r"\p[2]\s[99999]\s[-2]\s[無い名前]\b[99]\b[-2]"),
        Vec::new()
    );
}

#[test]
fn nothing_is_judged_after_a_switch_for_what_it_switches() {
    // 切替の前は診る。
    assert_eq!(check(r"\s[99999]\![change,shell,x]\s[99999]").len(), 1);
    // シェルの切替の後は `\s` だけ診ない。
    let got = check(r"\![change,shell,x]\s[99999]\b[99]");
    assert_eq!(
        got.iter().map(|d| d.kind).collect::<Vec<_>>(),
        [Kind::MissingBalloon]
    );
    // バルーンの切替の後は `\b` だけ診ない。
    let got = check(r"\![change,balloon,x]\s[99999]\b[99]");
    assert_eq!(
        got.iter().map(|d| d.kind).collect::<Vec<_>>(),
        [Kind::MissingSurface]
    );
    // ゴーストの切替（`\+`・`\_+` を含む）の後は両方とも診ない。他の種類は診る。
    for script in [
        r"\![change,ghost,x]\s[99999]\b[99]\x",
        r"\+\s[99999]\b[99]\x",
        r"\_+\s[99999]\b[99]\x",
    ] {
        let got = check(script);
        assert_eq!(
            got.iter().map(|d| d.kind).collect::<Vec<_>>(),
            [Kind::UnknownTag],
            "{script}"
        );
    }
}

#[test]
fn no_facts_means_no_surface_and_balloon_checks() {
    let got = check_with(r"\s[99999]\s[-2]\s[無い名前]\b[99]\b[-2]\x", None);
    assert_eq!(
        got.iter().map(|d| d.kind).collect::<Vec<_>>(),
        [Kind::UnknownTag]
    );
}

#[test]
fn text_after_end_and_quit_is_still_judged() {
    assert_eq!(check(r"\e\x")[0].kind, Kind::UnknownTag);
    assert_eq!(check(r"\-\s[99999]")[0].kind, Kind::MissingSurface);
}

#[test]
fn ordinary_script_reports_only_the_raise_nobody_handles() {
    assert_eq!(
        check(r"\0\s[0]こんにちは。\w9\1\s[10]やあ。\n[half]\![raise,OnX]%username\e")
            .iter()
            .map(|d| d.kind)
            .collect::<Vec<_>>(),
        // `\![raise]` は今は誰も拾わない。
        [Kind::UnknownCommand]
    );
}
