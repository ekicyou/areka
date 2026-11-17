// =============================================================================
// 読み直しの対象の振り分けの決定論檻（design.md Testing Strategy「reload_args_tests.rs」・
// 要件 12.1 ⑴⑵⑸・1.2〜1.5・7.2・7.4・7.5・8.5・10.3）
// =============================================================================

use super::*;

fn toks(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

const ALL: DescriptParts = DescriptParts {
    ghost: true,
    shell: true,
    balloon: true,
};

fn parts(ghost: bool, shell: bool, balloon: bool) -> DescriptParts {
    DescriptParts {
        ghost,
        shell,
        balloon,
    }
}

fn notes(unsupported: &[&str], unknown: &[&str], empty: bool) -> DescriptNotes {
    DescriptNotes {
        unsupported: toks(unsupported),
        unknown: toks(unknown),
        empty,
    }
}

// ── ⑴ MCP の `target`（要件 1.2・1.3・12.1 ⑴） ──

#[test]
fn mcp_target_six_words() {
    assert_eq!(
        McpTarget::parse("ghost"),
        McpTarget::Target(ReloadTarget::Ghost)
    );
    assert_eq!(
        McpTarget::parse("shiori"),
        McpTarget::Target(ReloadTarget::Shiori)
    );
    assert_eq!(
        McpTarget::parse("shell"),
        McpTarget::Target(ReloadTarget::Shell)
    );
    assert_eq!(
        McpTarget::parse("balloon"),
        McpTarget::Target(ReloadTarget::Balloon)
    );
    assert_eq!(
        McpTarget::parse("descript"),
        McpTarget::Target(ReloadTarget::Descript(ALL))
    );
    assert_eq!(McpTarget::parse("makoto"), McpTarget::Makoto);
}

#[test]
fn mcp_target_ignores_ascii_case() {
    assert_eq!(
        McpTarget::parse("Shell"),
        McpTarget::Target(ReloadTarget::Shell)
    );
    assert_eq!(
        McpTarget::parse("GHOST"),
        McpTarget::Target(ReloadTarget::Ghost)
    );
    assert_eq!(
        McpTarget::parse("DeScRiPt"),
        McpTarget::Target(ReloadTarget::Descript(ALL))
    );
    assert_eq!(McpTarget::parse("MAKOTO"), McpTarget::Makoto);
}

#[test]
fn mcp_target_does_not_trim_spaces() {
    assert_eq!(McpTarget::parse(" shell"), McpTarget::Unknown);
    assert_eq!(McpTarget::parse("shell "), McpTarget::Unknown);
    assert_eq!(McpTarget::parse("\tghost"), McpTarget::Unknown);
}

#[test]
fn mcp_target_empty_and_unknown() {
    assert_eq!(McpTarget::parse(""), McpTarget::Unknown);
    assert_eq!(McpTarget::parse("aigraph"), McpTarget::Unknown);
    assert_eq!(McpTarget::parse("reloadsurface"), McpTarget::Unknown);
    // 全角（ASCII の外）は大小の同一視に入れない。
    assert_eq!(McpTarget::parse("ｓｈｅｌｌ"), McpTarget::Unknown);
}

// ── ⑵ 台本の 7 綴り（要件 1.4・1.5・10.3・12.1 ⑵） ──

fn script(name: &str, params: &[&str]) -> Option<ScriptReload> {
    parse_script(name, &toks(params))
}

fn plain(target: ReloadTarget) -> Option<ScriptReload> {
    Some(ScriptReload {
        target,
        notes: DescriptNotes::default(),
    })
}

#[test]
fn script_seven_spellings() {
    assert_eq!(script("reload", &["ghost"]), plain(ReloadTarget::Ghost));
    assert_eq!(script("reload", &["shiori"]), plain(ReloadTarget::Shiori));
    assert_eq!(script("reload", &["shell"]), plain(ReloadTarget::Shell));
    assert_eq!(script("reload", &["balloon"]), plain(ReloadTarget::Balloon));
    assert_eq!(
        script("reload", &["descript"]),
        plain(ReloadTarget::Descript(ALL))
    );
    assert_eq!(
        script("reload", &["descript", "shell balloon"]),
        plain(ReloadTarget::Descript(parts(false, true, true)))
    );
    assert_eq!(script("reloadsurface", &[]), plain(ReloadTarget::Shell));
}

#[test]
fn script_and_mcp_descript_agree() {
    let Some(ScriptReload {
        target: ReloadTarget::Descript(from_script),
        ..
    }) = script("reload", &["descript"])
    else {
        panic!("descript が写らない");
    };
    assert_eq!(
        McpTarget::parse("descript"),
        McpTarget::Target(ReloadTarget::Descript(from_script))
    );
}

#[test]
fn script_not_ours() {
    assert_eq!(script("reload", &[]), None);
    assert_eq!(script("reload", &["makoto"]), None);
    assert_eq!(script("reload", &["aigraph"]), None);
    assert_eq!(script("reload", &[""]), None);
    assert_eq!(script("reload", &["Shell"]), None);
    assert_eq!(script("change", &["shell"]), None);
    assert_eq!(script("change", &["ghost", "emo2"]), None);
    assert_eq!(script("Reload", &["ghost"]), None);
    assert_eq!(script("", &["ghost"]), None);
}

#[test]
fn script_extra_args_after_target_are_ignored() {
    assert_eq!(
        script("reload", &["shell", "whatever"]),
        plain(ReloadTarget::Shell)
    );
    assert_eq!(
        script("reloadsurface", &["ghost"]),
        plain(ReloadTarget::Shell)
    );
}

#[test]
fn script_descript_param_notes_are_carried() {
    assert_eq!(
        script("reload", &["descript", "shell headline nosuchword"]),
        Some(ScriptReload {
            target: ReloadTarget::Descript(parts(false, true, false)),
            notes: notes(&["headline"], &["nosuchword"], false),
        })
    );
}

#[test]
fn script_descript_ignores_third_param_and_later() {
    // `\![reload,descript,shell,balloon]` の `balloon` は見ない（要件 7.2）。
    assert_eq!(
        script("reload", &["descript", "shell", "balloon"]),
        plain(ReloadTarget::Descript(parts(false, true, false)))
    );
}

// ── ⑸ descript のパラメータの語（要件 7.2・7.4・7.5・12.1 ⑸） ──

fn desc(param: &str) -> (DescriptParts, DescriptNotes) {
    parse_descript_params(param)
}

#[test]
fn descript_single_and_pairs() {
    assert_eq!(
        desc("shell"),
        (parts(false, true, false), notes(&[], &[], false))
    );
    assert_eq!(
        desc("ghost"),
        (parts(true, false, false), notes(&[], &[], false))
    );
    assert_eq!(
        desc("balloon"),
        (parts(false, false, true), notes(&[], &[], false))
    );
    assert_eq!(
        desc("shell balloon"),
        (parts(false, true, true), notes(&[], &[], false))
    );
    assert_eq!(desc("ghost shell balloon"), (ALL, notes(&[], &[], false)));
}

#[test]
fn descript_duplicates_once() {
    assert_eq!(
        desc("ghost ghost"),
        (parts(true, false, false), notes(&[], &[], false))
    );
    assert_eq!(
        desc("headline headline nosuch nosuch shell"),
        (
            parts(false, true, false),
            notes(&["headline"], &["nosuch"], false)
        )
    );
}

#[test]
fn descript_unsupported_kinds() {
    for word in [
        "headline",
        "plugin",
        "language",
        "calendar.skin",
        "calendar.plugin",
    ] {
        assert_eq!(
            desc(word),
            (parts(false, false, false), notes(&[word], &[], true)),
            "{word}"
        );
    }
}

#[test]
fn descript_calendar_two_words_are_one() {
    assert_eq!(
        desc("calendar skin"),
        (
            parts(false, false, false),
            notes(&["calendar skin"], &[], true)
        )
    );
    assert_eq!(
        desc("shell calendar plugin balloon"),
        (
            parts(false, true, true),
            notes(&["calendar plugin"], &[], false)
        )
    );
    // 後ろが skin／plugin でない `calendar` は知らない語。
    assert_eq!(
        desc("calendar shell"),
        (parts(false, true, false), notes(&[], &["calendar"], false))
    );
    assert_eq!(
        desc("calendar"),
        (parts(false, false, false), notes(&[], &["calendar"], true))
    );
}

#[test]
fn descript_unknown_and_empty() {
    assert_eq!(
        desc("nosuchword"),
        (
            parts(false, false, false),
            notes(&[], &["nosuchword"], true)
        )
    );
    assert_eq!(
        desc(""),
        (parts(false, false, false), notes(&[], &[], true))
    );
    assert_eq!(
        desc("   "),
        (parts(false, false, false), notes(&[], &[], true))
    );
    // カンマは区切りでない＝第 2 引数の全体が 1 語。
    assert_eq!(
        desc("shell,balloon"),
        (
            parts(false, false, false),
            notes(&[], &["shell,balloon"], true)
        )
    );
    // 大小は区別する（台本の綴りどおり）。
    assert_eq!(
        desc("Shell"),
        (parts(false, false, false), notes(&[], &["Shell"], true))
    );
}

#[test]
fn descript_extra_spaces_are_separators() {
    assert_eq!(
        desc("  shell   balloon "),
        (parts(false, true, true), notes(&[], &[], false))
    );
}

// ── どの入力でも panic しない（要件 8.5） ──

#[test]
fn never_panics_on_tricky_inputs() {
    let long = "x".repeat(100_000);
    let many_words = "shell calendar ".repeat(10_000);
    let words: Vec<&str> = vec![
        "",
        " ",
        "\t\n\r",
        "\0",
        "ghost",
        "Ghost",
        " ghost ",
        "ｇｈｏｓｔ",
        "ゴースト",
        "\u{1F388}",
        "calendar",
        "calendar ",
        " calendar",
        "calendar\u{3000}skin",
        "shell,balloon",
        "descript",
        "reloadsurface",
        long.as_str(),
        many_words.as_str(),
    ];
    for w in &words {
        let _ = McpTarget::parse(w);
        let _ = parse_descript_params(w);
        for name in ["reload", "reloadsurface", "change", "", w] {
            let _ = parse_script(name, &[]);
            let _ = parse_script(name, &toks(&[w]));
            let _ = parse_script(name, &toks(&["descript", w]));
            let _ = parse_script(name, &toks(&[w, w, w, w]));
        }
    }
    let many_args: Vec<String> = (0..10_000).map(|i| i.to_string()).collect();
    let _ = parse_script("reload", &many_args);
    let mut descript_many = vec!["descript".to_string()];
    descript_many.extend(many_args);
    let _ = parse_script("reload", &descript_many);
}
