//! 引数の列 → 起動オプションの中身と記録の材料（要件 1・2.1・2.3・2.4・4.5〜4.7・6.1・6.4〜6.7・8.4・9.4・11.1）。
//!
//! 最後のまとめ（`Summary`）の中身は `summary_table` で見る（1.12）。

use super::*;
use crate::cli::listen::ListenNote;
use crate::cli::{BootRequest, ListenPlan, ModulePolicy, StartupOptions};
use std::path::PathBuf;

/// `args[0]` に exe を足して読む。
fn run(args: &[&str]) -> Parsed {
    let mut all = vec!["areka".to_owned()];
    all.extend(args.iter().map(|a| (*a).to_owned()));
    parse(&all)
}

/// まとめが最後に 1 件だけあることを確かめ、まとめの前の材料を返す。
fn notes_before_summary(parsed: &Parsed) -> &[ParseNote] {
    let (last, rest) = parsed.notes.split_last().expect("まとめが 1 件はある");
    assert!(matches!(last, ParseNote::Summary { .. }), "最後はまとめ");
    assert!(
        !rest.iter().any(|n| matches!(n, ParseNote::Summary { .. })),
        "まとめは 1 件だけ"
    );
    rest
}

fn s(v: &str) -> String {
    v.to_owned()
}

fn unknown(arg: &str) -> ParseNote {
    ParseNote::Unknown { arg: s(arg) }
}

fn unsupported(name: &str) -> ParseNote {
    ParseNote::Unsupported { name: s(name) }
}

fn repeated(name: &str) -> ParseNote {
    ParseNote::Repeated { name: s(name) }
}

fn boot(
    positional_ghost: Option<&str>,
    positional_balloon: Option<&str>,
    ghost: Option<&str>,
    random: bool,
) -> BootRequest {
    BootRequest {
        positional_ghost: positional_ghost.map(PathBuf::from),
        positional_balloon: positional_balloon.map(PathBuf::from),
        ghost: ghost.map(s),
        random,
        no_boot_check: false,
    }
}

/// 起動の求めと記録の材料（まとめの前まで）の表。
#[test]
fn boot_request_and_notes_table() {
    let rows: Vec<(Vec<&str>, BootRequest, Vec<ParseNote>)> = vec![
        // 1.1・1.2: `--名前 値` と `--名前=値`
        (
            vec!["--ghost", "a"],
            boot(None, None, Some("a"), false),
            vec![],
        ),
        (
            vec!["--ghost=a"],
            boot(None, None, Some("a"), false),
            vec![],
        ),
        // 1.2: 最初の `=` で割る
        (
            vec!["--ghost=a=b"],
            boot(None, None, Some("a=b"), false),
            vec![],
        ),
        // 1.3・1.5: 短い形と大文字小文字
        (vec!["/g", "a"], boot(None, None, Some("a"), false), vec![]),
        (vec!["-g", "a"], boot(None, None, Some("a"), false), vec![]),
        (vec!["/G", "a"], boot(None, None, Some("a"), false), vec![]),
        (
            vec!["--GHOST", "a"],
            boot(None, None, Some("a"), false),
            vec![],
        ),
        (vec!["/r"], boot(None, None, None, true), vec![]),
        (vec!["-R"], boot(None, None, None, true), vec![]),
        (vec!["--Random"], boot(None, None, None, true), vec![]),
        // 1.4: 短い形の `=` は知らない名前（値は飲まない＝`a` は位置の引数）
        (
            vec!["/g=a"],
            boot(None, None, None, false),
            vec![unknown("/g=a")],
        ),
        (
            vec!["-g=a", "b"],
            boot(Some("b"), None, None, false),
            vec![unknown("-g=a")],
        ),
        // `-` ＋ 2 字以上は知らない名前
        (
            vec!["-ghost", "a"],
            boot(Some("a"), None, None, false),
            vec![unknown("-ghost")],
        ),
        // 1.6: 扱わない名前は値の取り方どおりに値も飲む
        (
            vec!["--install", "x", "y", "--random"],
            boot(None, None, None, true),
            vec![unsupported("install")],
        ),
        (
            vec!["/i", "x", "--random"],
            boot(None, None, None, true),
            vec![unsupported("install")],
        ),
        (
            vec!["--fmo-name", "f", "g"],
            boot(Some("g"), None, None, false),
            vec![unsupported("fmo-name")],
        ),
        (
            vec!["/s", "x.ssf"],
            boot(None, None, None, false),
            vec![unsupported("ssf")],
        ),
        (
            vec!["--offline-dump", "o"],
            boot(None, None, None, false),
            vec![unsupported("offline-dump")],
        ),
        (
            vec!["--dump-surface", "d"],
            boot(None, None, None, false),
            vec![unsupported("dump-surface")],
        ),
        // 省略できる値: 次が名前なら飲まない
        (
            vec!["--windowmode", "--random"],
            boot(None, None, None, true),
            vec![unsupported("windowmode")],
        ),
        (
            vec!["--password=p", "g"],
            boot(Some("g"), None, None, false),
            vec![unsupported("password")],
        ),
        // 1 つの値の扱わない名前が最後（値なし）でも warn は 1 件だけ
        (
            vec!["--guifont"],
            boot(None, None, None, false),
            vec![unsupported("guifont")],
        ),
        // 1.7: 知らない名前は値を飲まない
        (
            vec!["--foo", "bar"],
            boot(Some("bar"), None, None, false),
            vec![unknown("--foo")],
        ),
        (
            vec!["--"],
            boot(None, None, None, false),
            vec![unknown("--")],
        ),
        (vec!["-"], boot(None, None, None, false), vec![unknown("-")]),
        // 1.10: `--ghost` は書いた順に足す（最初の 1 つを起こし、残りは 4.5）
        (
            vec!["--ghost", "a", "--ghost", "b"],
            boot(None, None, Some("a"), false),
            vec![ParseNote::ExtraGhosts { rest: vec![s("b")] }],
        ),
        // 1.11: ほかは後勝ち＋warn
        (
            vec!["--random", "/r"],
            boot(None, None, None, true),
            vec![repeated("random")],
        ),
        // 値を取らない名前の `=` は効かせて値を使わない
        (
            vec!["--random=x"],
            boot(None, None, None, true),
            vec![ParseNote::ValueIgnored { name: s("random") }],
        ),
        // 2.1: 位置の引数の 1 つ目＝ゴースト・2 つ目＝バルーン
        (
            vec!["C:\\g\\emily"],
            boot(Some("C:\\g\\emily"), None, None, false),
            vec![],
        ),
        // 2.3: 3 つ目以降は warn
        (
            vec!["a", "b", "c", "d"],
            boot(Some("a"), Some("b"), None, false),
            vec![ParseNote::ExtraPositional {
                args: vec![s("c"), s("d")],
            }],
        ),
        // 2.4: `--ghost` があれば位置のゴーストを捨てる（バルーンは使う＝2.5）
        (
            vec!["C:\\g\\emily", "C:\\b\\x", "--ghost", "emo2"],
            boot(None, Some("C:\\b\\x"), Some("emo2"), false),
            vec![ParseNote::PositionalGhostIgnored {
                arg: s("C:\\g\\emily"),
            }],
        ),
        // `--ghost` が値なしでも位置のゴーストは使わない（2.4・4.7）
        (
            vec!["C:\\g\\emily", "--ghost"],
            boot(None, None, None, false),
            vec![
                ParseNote::PositionalGhostIgnored {
                    arg: s("C:\\g\\emily"),
                },
                ParseNote::GhostEmpty,
            ],
        ),
        // 4.5・4.6: 次の起動オプションの前までが全部 `--ghost` の値
        (
            vec!["--ghost", "A", "B", "--random"],
            boot(None, None, Some("A"), true),
            vec![ParseNote::ExtraGhosts { rest: vec![s("B")] }],
        ),
        // 複数の名前の `=` は続く引数も読む
        (
            vec!["--ghost=A", "B"],
            boot(None, None, Some("A"), false),
            vec![ParseNote::ExtraGhosts { rest: vec![s("B")] }],
        ),
        // 複数の名前は `=` の後ろが空でも続く引数を読む（`--ghost= A`＝`--ghost A`）
        (
            vec!["--ghost=", "A"],
            boot(None, None, Some("A"), false),
            vec![],
        ),
        // 1.2: 1 つ・省略できる 1 つの名前の `=` は値を同じ引数に持つ。
        // `=` の後ろが空でも次の引数は飲まない（次は位置の引数のまま）
        (
            vec!["--sstp-listen=", "C:\\g\\emily"],
            boot(Some("C:\\g\\emily"), None, None, false),
            vec![],
        ),
        (
            vec!["--option=", "a"],
            boot(Some("a"), None, None, false),
            vec![ParseNote::WordEmpty],
        ),
        (
            vec!["--help=", "x"],
            boot(Some("x"), None, None, false),
            vec![],
        ),
        // 4.7: 値なし
        (
            vec!["--ghost", "--random"],
            boot(None, None, None, true),
            vec![ParseNote::GhostEmpty],
        ),
        (
            vec!["--ghost="],
            boot(None, None, None, false),
            vec![ParseNote::GhostEmpty],
        ),
        // 1.2: 空白の形でも空の値は値なし（`--ghost=` と同じ）
        (
            vec!["--ghost", ""],
            boot(None, None, None, false),
            vec![ParseNote::GhostEmpty],
        ),
        // 9.4: `--help` の値は使わない
        (
            vec!["--help", "x"],
            boot(None, None, None, false),
            vec![ParseNote::HelpValueIgnored { value: s("x") }],
        ),
        // 8.4・設計の「値なし」: 省略できる値は次の引数が名前でなければ飲む。
        // 説明書の「位置の引数はすべての起動オプションより前に」の根拠になるふるまい。
        (
            vec!["--sstp-listen", "C:\\g\\emily", "C:\\b\\x"],
            boot(Some("C:\\b\\x"), None, None, false),
            vec![ParseNote::Listen(ListenNote::Rejected {
                item: s("C:\\g\\emily"),
                reason: "format",
            })],
        ),
        // 1.13: 書式の誤りが並んでも読み続ける
        (
            vec!["/g=x", "--foo", "--install", "--random", "a"],
            boot(Some("a"), None, None, true),
            vec![unknown("/g=x"), unknown("--foo"), unsupported("install")],
        ),
    ];
    for (args, want_boot, want_notes) in rows {
        let parsed = run(&args);
        assert_eq!(parsed.options.boot, want_boot, "起動の求め: {args:?}");
        assert_eq!(
            notes_before_summary(&parsed),
            want_notes.as_slice(),
            "記録の材料: {args:?}"
        );
        // 位置のゴーストと `--ghost` が両方 Some になることはない
        assert!(
            parsed.options.boot.positional_ghost.is_none() || parsed.options.boot.ghost.is_none(),
            "{args:?}"
        );
    }
}

/// `--sstp-listen` の値は 2.1 の計画へ渡る（8.4・1.11）。
#[test]
fn sstp_listen_table() {
    use ListenPlan::*;
    let rows: Vec<(Vec<&str>, ListenPlan, Vec<ParseNote>)> = vec![
        (vec![], NotGiven, vec![]),
        (vec!["--sstp-listen", "9900"], Ports(vec![9900]), vec![]),
        (
            vec!["--sstp-listen=9900,9901"],
            Ports(vec![9900, 9901]),
            vec![],
        ),
        (vec!["--SSTP-LISTEN", "none"], Off, vec![]),
        // 8.4: 値なし（最後・次が名前・`=` の後ろが空）は待ち受けない。
        // `=` の後ろが空なら次の引数を値に取らない（1.2）
        (vec!["--sstp-listen"], Off, vec![]),
        (vec!["--sstp-listen", "--random"], Off, vec![]),
        (vec!["--sstp-listen="], Off, vec![]),
        (vec!["--sstp-listen=", "C:\\g\\emily"], Off, vec![]),
        // 1.2: 空白の形でも空の値は値なし（`--sstp-listen=` と同じ）
        (vec!["--sstp-listen", ""], Off, vec![]),
        // 1.11: 後勝ち＋warn
        (
            vec!["--sstp-listen", "1", "--sstp-listen", "2"],
            Ports(vec![2]),
            vec![repeated("sstp-listen")],
        ),
        // 2.1 の注記は記録の材料へ流れる
        (
            vec!["--sstp-listen", "0.0.0.0:9900,9901"],
            Ports(vec![9901]),
            vec![ParseNote::Listen(ListenNote::NotLoopback {
                item: s("0.0.0.0:9900"),
            })],
        ),
        (
            vec!["--sstp-listen", "abc"],
            AllRejected,
            vec![ParseNote::Listen(ListenNote::Rejected {
                item: s("abc"),
                reason: "port",
            })],
        ),
    ];
    for (args, want_plan, want_notes) in rows {
        let parsed = run(&args);
        assert_eq!(parsed.options.listen, want_plan, "{args:?}");
        assert_eq!(
            notes_before_summary(&parsed),
            want_notes.as_slice(),
            "{args:?}"
        );
    }
}

/// `--help` の有無（9.1・9.4・1.3）。
#[test]
fn help_table() {
    let rows: Vec<(Vec<&str>, bool, Vec<ParseNote>)> = vec![
        (vec![], false, vec![]),
        (vec!["--help"], true, vec![]),
        (vec!["/?"], true, vec![]),
        (vec!["-?"], true, vec![]),
        (vec!["--help="], true, vec![]),
        // `=` の後ろが空なら次の引数を値に取らない（1.2）
        (vec!["--help=", "x"], true, vec![]),
        // 1.2: 空白の形でも空の値は値なし（`--help=` と同じ・空の引数は位置の引数にしない）
        (vec!["--help", ""], true, vec![]),
        (
            vec!["--help=x"],
            true,
            vec![ParseNote::HelpValueIgnored { value: s("x") }],
        ),
        (vec!["--help", "--random"], true, vec![]),
        (vec!["--help", "/?"], true, vec![repeated("help")]),
    ];
    for (args, want_help, want_notes) in rows {
        let parsed = run(&args);
        assert_eq!(parsed.options.help, want_help, "{args:?}");
        assert_eq!(
            notes_before_summary(&parsed),
            want_notes.as_slice(),
            "{args:?}"
        );
    }
}

fn noop(w: &str) -> ParseNote {
    ParseNote::WordNoop { word: s(w) }
}

fn pending(w: &str) -> ParseNote {
    ParseNote::WordPending { word: s(w) }
}

fn word_unknown(w: &str) -> ParseNote {
    ParseNote::WordUnknown { word: s(w) }
}

/// `--option` の語（1.5・1.8・1.9・6.1・6.2・6.4〜6.7・7.1 の読み方側）。
#[test]
fn option_words_table() {
    use ModulePolicy::*;
    let rows: Vec<(Vec<&str>, bool, ModulePolicy, Vec<ParseNote>)> = vec![
        (vec![], false, Load, vec![]),
        // 6.1・6.4〜6.7: 語ごとに 1 件（空の語も 1 つずつ）
        (
            vec!["--option", "nobootcheck,bootunlock,readonly,xyz,,"],
            true,
            Load,
            vec![
                noop("bootunlock"),
                pending("readonly"),
                word_unknown("xyz"),
                ParseNote::WordEmpty,
                ParseNote::WordEmpty,
            ],
        ),
        // 7.1・1.5: 大文字小文字を問わない
        (vec!["--option", "DontLoadModule"], false, Skip, vec![]),
        (
            vec!["--option=standalone,layered,region,dpiaware"],
            false,
            Load,
            vec![
                noop("standalone"),
                noop("layered"),
                noop("region"),
                noop("dpiaware"),
            ],
        ),
        (
            vec!["/o", "kiosk,callghost,deletesource"],
            false,
            Load,
            vec![
                pending("kiosk"),
                pending("callghost"),
                pending("deletesource"),
            ],
        ),
        // 1.9: 重ねは語を合わせ、後勝ちの warn は出ない
        (
            vec!["--option", "nobootcheck", "-O", "dontloadmodule"],
            true,
            Skip,
            vec![],
        ),
        // 1.4: 短い形の `=` は知らない名前
        (
            vec!["/o", "x", "-O=c"],
            false,
            Load,
            vec![word_unknown("x"), unknown("-O=c")],
        ),
        // 1.8・6.7: 値なしは空の語 1 つ（1.8 の warn は出さない）。
        // `=` の後ろが空なら次の引数を語に取らない（1.2・次は位置の引数）
        (vec!["--option"], false, Load, vec![ParseNote::WordEmpty]),
        (
            vec!["--option", "--random"],
            false,
            Load,
            vec![ParseNote::WordEmpty],
        ),
        (vec!["--option="], false, Load, vec![ParseNote::WordEmpty]),
        // 1.2: 空白の形でも空の値は値なし（`--option=` と同じ）
        (
            vec!["--option", ""],
            false,
            Load,
            vec![ParseNote::WordEmpty],
        ),
        (
            vec!["--option=", "a"],
            false,
            Load,
            vec![ParseNote::WordEmpty],
        ),
    ];
    for (args, want_nbc, want_modules, want_notes) in rows {
        let parsed = run(&args);
        assert_eq!(parsed.options.boot.no_boot_check, want_nbc, "{args:?}");
        assert_eq!(parsed.options.modules, want_modules, "{args:?}");
        assert_eq!(
            notes_before_summary(&parsed),
            want_notes.as_slice(),
            "{args:?}"
        );
    }
}

/// 1.12: まとめは最後に 1 件で、効かせるもの・受けて何もしないもの・無視したものを並べる。
#[test]
fn summary_table() {
    let rows: Vec<(Vec<&str>, Vec<&str>, Vec<&str>, Vec<&str>)> = vec![
        (vec![], vec![], vec![], vec![]),
        // 位置の引数は起動オプションではないので並ばない
        (vec![r"C:\g\emily", r"C:\b\x"], vec![], vec![], vec![]),
        (
            vec![
                "--ghost",
                "a",
                "--random",
                "--option",
                "nobootcheck,bootunlock,readonly,xyz,,",
                "--foo",
                "--install",
                "x",
                "--sstp-listen",
                "9900",
                "/o",
                "DONTLOADMODULE",
            ],
            vec![
                "ghost",
                "random",
                "nobootcheck",
                "sstp-listen",
                "dontloadmodule",
            ],
            vec!["bootunlock"],
            vec!["readonly", "xyz", "--foo", "install"],
        ),
        // 重ねても 1 つ
        (vec!["--help", "/?"], vec!["help"], vec![], vec![]),
    ];
    for (args, effective, noop, ignored) in rows {
        let parsed = run(&args);
        notes_before_summary(&parsed);
        let want = ParseNote::Summary {
            effective: effective.into_iter().map(s).collect(),
            noop: noop.into_iter().map(s).collect(),
            ignored: ignored.into_iter().map(s).collect(),
        };
        assert_eq!(parsed.notes.last(), Some(&want), "{args:?}");
    }
}

/// 1.14: 読めない文字を置き換えた引数ごとに 1 件。読める引数はそのまま。
#[cfg(windows)]
#[test]
fn from_os_replaces_non_utf8_per_arg() {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    let bad = OsString::from_wide(&[0x61, 0xD800]);
    let args = vec![
        OsString::from("areka"),
        bad.clone(),
        OsString::from("--random"),
        bad,
    ];
    let (strings, notes) = from_os(&args);
    assert_eq!(
        strings,
        vec![s("areka"), s("a\u{FFFD}"), s("--random"), s("a\u{FFFD}")]
    );
    let lossy = || ParseNote::NonUtf8 {
        lossy: s("a\u{FFFD}"),
    };
    assert_eq!(notes, vec![lossy(), lossy()]);
    let (strings, notes) = from_os(&[OsString::from("areka"), OsString::from("名前")]);
    assert_eq!(strings, vec![s("areka"), s("名前")]);
    assert!(notes.is_empty());
}

/// 記録の形: 材料ごとに 1 件、設計の `event` の名前と重さ、前置き `[cli]`。
#[test]
fn notes_log_one_event_each_with_cli_prefix() {
    use tracing::Level;
    let rows: Vec<(ParseNote, Level, &str)> = vec![
        (unknown("--foo"), Level::WARN, "startup_option_unknown"),
        (
            unsupported("install"),
            Level::WARN,
            "startup_option_unsupported",
        ),
        (
            ParseNote::MissingValue { name: s("x") },
            Level::WARN,
            "startup_option_missing_value",
        ),
        (repeated("random"), Level::WARN, "startup_option_repeated"),
        (
            ParseNote::ValueIgnored { name: s("random") },
            Level::WARN,
            "option_value_ignored",
        ),
        (
            ParseNote::ExtraPositional { args: vec![s("c")] },
            Level::WARN,
            "positional_extra",
        ),
        (
            ParseNote::PositionalGhostIgnored { arg: s("a") },
            Level::WARN,
            "positional_ghost_ignored",
        ),
        (
            ParseNote::ExtraGhosts { rest: vec![s("b")] },
            Level::WARN,
            "ghost_option_extra",
        ),
        (ParseNote::GhostEmpty, Level::WARN, "ghost_option_empty"),
        (noop("bootunlock"), Level::INFO, "option_word_noop"),
        (pending("readonly"), Level::WARN, "option_word_pending"),
        (word_unknown("xyz"), Level::WARN, "option_word_unknown"),
        (ParseNote::WordEmpty, Level::WARN, "option_word_empty"),
        (
            ParseNote::NonUtf8 { lossy: s("a") },
            Level::WARN,
            "startup_arg_not_utf8",
        ),
        (
            ParseNote::HelpValueIgnored { value: s("x") },
            Level::WARN,
            "help_value_ignored",
        ),
        (
            ParseNote::Listen(ListenNote::NotLoopback {
                item: s("0.0.0.0:1"),
            }),
            Level::WARN,
            "mcp_listen_not_loopback",
        ),
        (
            ParseNote::Summary {
                effective: vec![s("random")],
                noop: vec![],
                ignored: vec![],
            },
            Level::INFO,
            "startup_options",
        ),
    ];
    for (note, level, event) in rows {
        let ((), events) = log_capture_kit::capture(|| note.log());
        assert_eq!(events.len(), 1, "{note:?}: {events:?}");
        assert_eq!(events[0].level, level, "{note:?}");
        assert_eq!(events[0].field_str("event"), Some(event), "{note:?}");
        assert!(events[0].message().starts_with("[cli] "), "{note:?}");
    }
}

/// 実走の 3 本と同じ形（絶対パス 2 つ）は位置の引数だけになり、記録の材料はまとめ 1 件だけ（2.6・2.1）。
#[test]
fn real_run_shape_is_positional_only() {
    let ghost = r"C:\home\maz\git\areka\target\emo2_real_run\ghost\emo2";
    let balloon = r"C:\home\maz\git\areka\target\emo2_real_run\balloon\emo2";
    let parsed = run(&[ghost, balloon]);
    assert_eq!(
        parsed.options.boot,
        boot(Some(ghost), Some(balloon), None, false)
    );
    assert_eq!(parsed.options.listen, ListenPlan::NotGiven);
    assert_eq!(parsed.options.modules, ModulePolicy::Load);
    assert!(!parsed.options.help);
    assert_eq!(parsed.notes.len(), 1);
    assert!(matches!(parsed.notes[0], ParseNote::Summary { .. }));
}

/// 1.13: 引数が空でも、exe だけでも読める。まとめは必ず最後に 1 件。
#[test]
fn empty_args_still_end_with_summary() {
    for all in [vec![], vec![s("areka")]] {
        let parsed = parse(&all);
        assert_eq!(parsed.options, StartupOptions::default());
        assert_eq!(notes_before_summary(&parsed), &[]);
    }
}
