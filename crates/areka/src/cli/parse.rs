//! 起動オプションの名前と値を読む表と読み方。
//!
//! 純粋: ファイル・環境変数・時計に触らない。記録を出すのは `ParseNote::log` だけ。
//! 書式の誤りでも `Err` を返さない。

use std::ffi::OsString;
use std::path::PathBuf;

use super::listen::{self, ListenNote};
use super::{ModulePolicy, OfflineDumpRequest, StartupOptions};

/// 値の取り方。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Arity {
    /// 値なし。
    None,
    /// 1 つ。
    One,
    /// 省略できる 1 つ。
    Optional,
    /// 空白区切りで複数（次の起動オプションまで）。
    Many,
}

/// 名前の種類。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Ghost,
    Random,
    Option,
    SstpListen,
    Help,
    /// `--offline-dump`（値はゴーストのフォルダ）。
    OfflineDump,
    /// `--dump-*` の値の欄。
    DumpValue(DumpField),
    /// `--dump-surface-option`・`--dump-animation`（まだ扱わない）。
    DumpPending,
    /// `--install`（値は入れる書庫）。
    Install,
    /// areka が扱わない名前（表の中のものと、表に無い `--dump-*`）。
    Unsupported,
}

/// `--dump-*` の値の欄（`OfflineDumpRequest` の欄）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DumpField {
    Shell,
    Scope,
    SurfaceList,
    OutputDir,
    OutputPrefix,
    ErrorLog,
    Bind,
}

impl DumpField {
    fn slot(self, req: &mut OfflineDumpRequest) -> &mut Option<String> {
        match self {
            Self::Shell => &mut req.shell,
            Self::Scope => &mut req.scope,
            Self::SurfaceList => &mut req.surface_list,
            Self::OutputDir => &mut req.output_dir,
            Self::OutputPrefix => &mut req.output_prefix,
            Self::ErrorLog => &mut req.error_log,
            Self::Bind => &mut req.bind,
        }
    }
}

/// 表の 1 行: 名前・短い形・値の取り方・種類。
struct Spec {
    long: &'static str,
    short: Option<char>,
    arity: Arity,
    kind: Kind,
}

const fn spec(long: &'static str, short: Option<char>, arity: Arity, kind: Kind) -> Spec {
    Spec {
        long,
        short,
        arity,
        kind,
    }
}

/// 起動オプションの表。表に無い `dump-` で始まる名前は `lookup_long` が読む。
const TABLE: &[Spec] = &[
    spec("ghost", Some('g'), Arity::Many, Kind::Ghost),
    spec("random", Some('r'), Arity::None, Kind::Random),
    spec("option", Some('o'), Arity::One, Kind::Option),
    spec("sstp-listen", None, Arity::Optional, Kind::SstpListen),
    spec("help", Some('?'), Arity::Optional, Kind::Help),
    spec("install", Some('i'), Arity::Many, Kind::Install),
    spec("fmo-name", Some('f'), Arity::One, Kind::Unsupported),
    spec("ukagaka-link", Some('m'), Arity::One, Kind::Unsupported),
    spec("ssf", Some('s'), Arity::One, Kind::Unsupported),
    spec("offline-dump", None, Arity::One, Kind::OfflineDump),
    spec(
        "dump-shell",
        None,
        Arity::Optional,
        Kind::DumpValue(DumpField::Shell),
    ),
    spec(
        "dump-scope",
        None,
        Arity::Optional,
        Kind::DumpValue(DumpField::Scope),
    ),
    spec(
        "dump-surface-list",
        None,
        Arity::Optional,
        Kind::DumpValue(DumpField::SurfaceList),
    ),
    spec(
        "dump-output-dir",
        None,
        Arity::Optional,
        Kind::DumpValue(DumpField::OutputDir),
    ),
    spec(
        "dump-output-prefix",
        None,
        Arity::Optional,
        Kind::DumpValue(DumpField::OutputPrefix),
    ),
    spec(
        "dump-error-log",
        None,
        Arity::Optional,
        Kind::DumpValue(DumpField::ErrorLog),
    ),
    spec(
        "dump-bind",
        None,
        Arity::Optional,
        Kind::DumpValue(DumpField::Bind),
    ),
    spec(
        "dump-surface-option",
        None,
        Arity::Optional,
        Kind::DumpPending,
    ),
    spec("dump-animation", None, Arity::Optional, Kind::DumpPending),
    spec("offline-tool", None, Arity::One, Kind::Unsupported),
    spec("target-dir", None, Arity::One, Kind::Unsupported),
    spec("output", None, Arity::One, Kind::Unsupported),
    spec("guifont", None, Arity::One, Kind::Unsupported),
    spec("windowmode", None, Arity::Optional, Kind::Unsupported),
    spec("password", None, Arity::Optional, Kind::Unsupported),
];

/// 長い形の名前（小文字にしたもの）→ 表の名前・値の取り方・種類。
fn lookup_long(name: &str) -> Option<(String, Arity, Kind)> {
    if let Some(s) = TABLE.iter().find(|s| s.long == name) {
        return Some((s.long.to_owned(), s.arity, s.kind));
    }
    name.starts_with("dump-")
        .then(|| (name.to_owned(), Arity::Optional, Kind::Unsupported))
}

/// 短い形の 1 字（小文字にしたもの）→ 表の名前・値の取り方・種類。
fn lookup_short(c: char) -> Option<(String, Arity, Kind)> {
    TABLE
        .iter()
        .find(|s| s.short == Some(c))
        .map(|s| (s.long.to_owned(), s.arity, s.kind))
}

/// `--`・`/`・`-` で始まる引数は起動オプションの名前（用語「起動オプション」）。
fn is_option(arg: &str) -> bool {
    arg.starts_with('-') || arg.starts_with('/')
}

/// 名前の引数 → 表の名前・値の取り方・種類と `=` の後ろ（`=` が無ければ None・空でも `Some("")`）。
/// 知らない名前は None。
fn read_name(arg: &str) -> Option<(String, Arity, Kind, Option<String>)> {
    if let Some(rest) = arg.strip_prefix("--") {
        let (name, value) = match rest.split_once('=') {
            Some((n, v)) => (n, Some(v.to_owned())),
            None => (rest, None),
        };
        let (long, arity, kind) = lookup_long(&name.to_ascii_lowercase())?;
        return Some((long, arity, kind, value));
    }
    // 短い形（`/x`・`-x`）は 1 字だけ。`=` 付きと `-` ＋ 2 字以上は知らない名前。
    let rest = &arg[1..];
    let mut chars = rest.chars();
    let (Some(c), None) = (chars.next(), chars.next()) else {
        return None;
    };
    let (long, arity, kind) = lookup_short(c.to_ascii_lowercase())?;
    Some((long, arity, kind, None))
}

/// 記録の材料（記録を出すのは `ParseNote::log`）。
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ParseNote {
    /// 知らない名前。warn
    Unknown { arg: String },
    /// 扱わない名前（表の `Kind::Unsupported` と、表に無い `--dump-*`）。warn
    Unsupported { name: String },
    /// 値を取る名前の値が無く、意味も定まっていない。`--offline-dump` があるときの `--dump-*` の 7 つ。warn
    MissingValue { name: String },
    /// `--random`・`--sstp-listen`・`--help`・`--offline-dump`・値を取る `--dump-*` の 7 つ（`--dump-shell`・`--dump-scope`・`--dump-surface-list`・`--dump-output-dir`・`--dump-output-prefix`・`--dump-error-log`・`--dump-bind`）の重ね（`--option`・`--ghost`・`--install` は値を足すので出ない）。warn
    Repeated { name: String },
    /// 値を取らない名前の `=` の値（使わない）。warn
    ValueIgnored { name: String },
    /// 位置の引数の 3 つ目以降。warn
    ExtraPositional { args: Vec<String> },
    /// `--ghost` があるので使わない位置のゴースト。warn
    PositionalGhostIgnored { arg: String },
    /// 起こさない `--ghost` の残りの値。warn
    ExtraGhosts { rest: Vec<String> },
    /// `--ghost` に値が無い。warn
    GhostEmpty,
    /// 受けて何もしない語。info
    WordNoop { word: String },
    /// まだ扱わない語。warn
    WordPending { word: String },
    /// 正典に無い語。warn
    WordUnknown { word: String },
    /// 空の語。warn
    WordEmpty,
    /// UTF-8 として読めない引数。warn
    NonUtf8 { lossy: String },
    /// `--help` の値（使わない）。warn
    HelpValueIgnored { value: String },
    /// `--offline-dump` があるので効かせない起動オプション（`positional` は位置の引数）。warn
    OfflineIgnored { name: String },
    /// `--offline-dump` と一緒の、まだ扱わない `--dump-*`。warn
    DumpPending { name: String },
    /// `--offline-dump` なしの `--dump-*`。warn
    DumpWithoutOffline { name: String },
    /// `--sstp-listen` の待ち受け先の注記。warn
    Listen(ListenNote),
    /// `--install` に値が無い。warn
    InstallEmpty,
    /// `--install` の値なしの `callghost`・`deletesource`。warn
    InstallWordWithoutInstall { word: String },
    /// 読み終えたまとめ（最後に 1 件）。info
    Summary {
        effective: Vec<String>,
        noop: Vec<String>,
        ignored: Vec<String>,
    },
}

/// 読んだ中身と記録の材料。
#[derive(Debug)]
pub(crate) struct Parsed {
    pub options: StartupOptions,
    /// 記録の材料。最後は必ず `Summary` が 1 件。
    pub notes: Vec<ParseNote>,
}

/// 引数の列を読む。`args[0]` は exe（読まない）。純粋で、`Err` を返さない。
pub(crate) fn parse(args: &[String]) -> Parsed {
    let mut options = StartupOptions::default();
    let mut notes = Vec::new();
    let mut effective: Vec<String> = Vec::new();
    let mut noop: Vec<String> = Vec::new();
    let mut ignored: Vec<String> = Vec::new();

    let mut positional: Vec<String> = Vec::new();
    let mut ghost_given = false;
    let mut ghosts: Vec<String> = Vec::new();
    let mut listen_value: Option<Option<String>> = None;
    let mut help_value: Option<String> = None;
    let mut offline_given = false;
    let mut offline = OfflineDumpRequest::default();
    // `--offline-dump` の有無で記録が変わる名前（出てきた順・重ねは 1 つ）。
    let mut others: Vec<String> = Vec::new();
    let mut dumps: Vec<String> = Vec::new();
    let seen = |list: &mut Vec<String>, name: &str| {
        if !list.iter().any(|n| n == name) {
            list.push(name.to_owned());
        }
    };

    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        i += 1;
        if !is_option(arg) {
            seen(&mut others, "positional");
            positional.push(arg.clone());
            continue;
        }
        let Some((name, arity, kind, eq_value)) = read_name(arg) else {
            notes.push(ParseNote::Unknown { arg: arg.clone() });
            ignored.push(arg.clone());
            continue;
        };
        // 値を読む。`=` の後ろが空なら値なし（`--名前=` の形）。
        let had_eq = eq_value.is_some();
        let eq_value = eq_value.filter(|v| !v.is_empty());
        let mut values: Vec<String> = Vec::new();
        match arity {
            Arity::None => {
                if eq_value.is_some() {
                    notes.push(ParseNote::ValueIgnored { name: name.clone() });
                }
            }
            Arity::One | Arity::Optional => match eq_value {
                Some(v) => values.push(v),
                // `=` の形は値を同じ引数に持つので、空でも次の引数は読まない。
                None if had_eq => {}
                // 次の引数が名前でなければ値として読む。空の値は `=` の形と同じく値なし。
                None if i < args.len() && !is_option(&args[i]) => {
                    values.extend(Some(args[i].clone()).filter(|v| !v.is_empty()));
                    i += 1;
                }
                None => {}
            },
            Arity::Many => {
                // 「複数」は `=` の値を 1 つ目として続く引数も読む（`--ghost=A B`＝`--ghost A B`）。
                values.extend(eq_value);
                // 空の引数は値なし（`--ghost ""`＝`--ghost=`）。読み飛ばすだけで位置の引数にもしない。
                while i < args.len() && !is_option(&args[i]) {
                    values.extend(Some(args[i].clone()).filter(|v| !v.is_empty()));
                    i += 1;
                }
            }
        }
        let first = !effective.contains(&name);
        match kind {
            Kind::Ghost | Kind::Random | Kind::Option | Kind::SstpListen | Kind::Install => {
                seen(&mut others, &name)
            }
            Kind::DumpValue(_) | Kind::DumpPending => seen(&mut dumps, &name),
            _ => {}
        }
        match kind {
            Kind::Unsupported => {
                notes.push(ParseNote::Unsupported { name: name.clone() });
                ignored.push(name);
                continue;
            }
            Kind::Ghost => {
                ghost_given = true;
                ghosts.extend(values);
            }
            Kind::Install => options
                .install
                .archives
                .extend(values.into_iter().map(PathBuf::from)),
            Kind::Option => {
                // 値なし（`--option` だけ）は空の語 1 つ。重ねは語を合わせる。
                let value = values.pop().unwrap_or_default();
                for word in value.split(',') {
                    let lower = word.to_ascii_lowercase();
                    match lower.as_str() {
                        "" => notes.push(ParseNote::WordEmpty),
                        "nobootcheck" | "dontloadmodule" | "readonly" | "callghost"
                        | "deletesource" => {
                            match lower.as_str() {
                                "nobootcheck" => options.boot.no_boot_check = true,
                                "dontloadmodule" => options.modules = ModulePolicy::Skip,
                                "callghost" => options.install.call_ghost = true,
                                "deletesource" => options.install.delete_source = true,
                                _ => options.readonly = true,
                            }
                            if !effective.contains(&lower) {
                                effective.push(lower);
                            }
                        }
                        "bootunlock" | "standalone" | "layered" | "region" | "dpiaware" => {
                            notes.push(ParseNote::WordNoop {
                                word: lower.clone(),
                            });
                            noop.push(lower);
                        }
                        "kiosk" => {
                            notes.push(ParseNote::WordPending {
                                word: lower.clone(),
                            });
                            ignored.push(lower);
                        }
                        _ => {
                            notes.push(ParseNote::WordUnknown {
                                word: word.to_owned(),
                            });
                            ignored.push(word.to_owned());
                        }
                    }
                }
                continue;
            }
            Kind::DumpPending => {
                if !ignored.contains(&name) {
                    ignored.push(name);
                }
                continue;
            }
            Kind::OfflineDump | Kind::DumpValue(_) => {
                if !first {
                    notes.push(ParseNote::Repeated { name: name.clone() });
                }
                // 後勝ち。後が値なしなら値なし。
                let value = values.pop();
                match kind {
                    Kind::DumpValue(field) => *field.slot(&mut offline) = value,
                    _ => {
                        offline_given = true;
                        offline.ghost_dir = value;
                    }
                }
            }
            Kind::Random | Kind::SstpListen | Kind::Help => {
                if !first {
                    notes.push(ParseNote::Repeated { name: name.clone() });
                }
                match kind {
                    Kind::Random => options.boot.random = true,
                    Kind::SstpListen => listen_value = Some(values.pop()),
                    _ => {
                        options.help = true;
                        help_value = values.pop();
                    }
                }
            }
        }
        if first {
            effective.push(name);
        }
    }

    let mut positional = positional.into_iter();
    if let Some(g) = positional.next() {
        if ghost_given {
            notes.push(ParseNote::PositionalGhostIgnored { arg: g });
        } else {
            options.boot.positional_ghost = Some(PathBuf::from(g));
        }
    }
    options.boot.positional_balloon = positional.next().map(PathBuf::from);
    let extra: Vec<String> = positional.collect();
    if !extra.is_empty() {
        notes.push(ParseNote::ExtraPositional { args: extra });
    }

    if ghost_given {
        let mut ghosts = ghosts.into_iter();
        options.boot.ghost = ghosts.next();
        let rest: Vec<String> = ghosts.collect();
        if options.boot.ghost.is_none() {
            notes.push(ParseNote::GhostEmpty);
        } else if !rest.is_empty() {
            notes.push(ParseNote::ExtraGhosts { rest });
        }
    }

    // `--offline-dump` の有無は読み終えて決まる。値なしの `--dump-*` は値なしの記録（`--offline-dump` の値なしは記録しない）。
    if offline_given {
        for name in &effective {
            if let Some((_, _, Kind::DumpValue(field))) = lookup_long(name)
                && field.slot(&mut offline).is_none()
            {
                notes.push(ParseNote::MissingValue { name: name.clone() });
            }
        }
        for name in dumps {
            if lookup_long(&name).is_some_and(|(_, _, k)| k == Kind::DumpPending) {
                notes.push(ParseNote::DumpPending { name });
            }
        }
        notes.extend(
            others
                .into_iter()
                .map(|name| ParseNote::OfflineIgnored { name }),
        );
        options.offline = Some(offline);
        // まとめの効く側はオフラインの名前と `--help` だけ（ほかは無視の側）。
        let (keep, moved): (Vec<String>, Vec<String>) = effective.into_iter().partition(|n| {
            matches!(
                lookup_long(n),
                Some((_, _, Kind::OfflineDump | Kind::DumpValue(_) | Kind::Help))
            )
        });
        effective = keep;
        ignored.extend(moved);
    } else {
        // 値が 0 本の `--install` と、それと組む語は効かない。
        if options.install.archives.is_empty() {
            let (moved, keep): (Vec<String>, Vec<String>) = effective
                .into_iter()
                .partition(|n| matches!(n.as_str(), "install" | "callghost" | "deletesource"));
            effective = keep;
            for name in moved {
                notes.push(match name.as_str() {
                    "install" => ParseNote::InstallEmpty,
                    _ => ParseNote::InstallWordWithoutInstall { word: name.clone() },
                });
                ignored.push(name);
            }
            options.install.call_ghost = false;
            options.install.delete_source = false;
        }
        effective.retain(|n| !dumps.contains(n));
        for name in dumps {
            if !ignored.contains(&name) {
                ignored.push(name.clone());
            }
            notes.push(ParseNote::DumpWithoutOffline { name });
        }
    }

    if let Some(value) = help_value {
        notes.push(ParseNote::HelpValueIgnored { value });
    }

    if let Some(value) = listen_value {
        let (plan, listen_notes) = listen::plan(value.as_deref());
        options.listen = plan;
        notes.extend(listen_notes.into_iter().map(ParseNote::Listen));
    }
    notes.push(ParseNote::Summary {
        effective,
        noop,
        ignored,
    });
    Parsed { options, notes }
}

/// OS の引数を文字列へ。UTF-8 として読めない文字は置き換え、その引数ごとに `NonUtf8`。
pub(crate) fn from_os(args: &[OsString]) -> (Vec<String>, Vec<ParseNote>) {
    let mut notes = Vec::new();
    let strings = args
        .iter()
        .map(|a| match a.to_str() {
            Some(s) => s.to_owned(),
            None => {
                let lossy = a.to_string_lossy().into_owned();
                notes.push(ParseNote::NonUtf8 {
                    lossy: lossy.clone(),
                });
                lossy
            }
        })
        .collect();
    (strings, notes)
}

impl ParseNote {
    /// 記録を 1 件出す（前置きは `[cli]`）。
    pub(crate) fn log(&self) {
        use tracing::{info, warn};
        match self {
            Self::Unknown { arg } => warn!(
                event = "startup_option_unknown",
                arg = %arg,
                "[cli] 知らない起動オプションなので、この引数を無視する"
            ),
            Self::Unsupported { name } => warn!(
                event = "startup_option_unsupported",
                name = %name,
                "[cli] areka が扱わない起動オプションなので、値ごと無視する"
            ),
            Self::MissingValue { name } => warn!(
                event = "startup_option_missing_value",
                name = %name,
                "[cli] 起動オプションに値が無いので無視する"
            ),
            Self::Repeated { name } => warn!(
                event = "startup_option_repeated",
                name = %name,
                "[cli] 起動オプションが重なったので、後に書かれた値を使う"
            ),
            Self::ValueIgnored { name } => warn!(
                event = "option_value_ignored",
                name = %name,
                "[cli] 値を取らない起動オプションに値が付いていたので、値を使わない"
            ),
            Self::ExtraPositional { args } => warn!(
                event = "positional_extra",
                args = ?args,
                "[cli] 3 つ目からの位置の引数は使わない"
            ),
            Self::PositionalGhostIgnored { arg } => warn!(
                event = "positional_ghost_ignored",
                arg = %arg,
                "[cli] --ghost があるので、位置の引数のゴーストを使わない"
            ),
            Self::ExtraGhosts { rest } => warn!(
                event = "ghost_option_extra",
                rest = ?rest,
                "[cli] --ghost の最初の 1 つだけを起こし、残りは使わない"
            ),
            Self::GhostEmpty => warn!(
                event = "ghost_option_empty",
                "[cli] --ghost に値が無いので、いつもの選び方で起こす"
            ),
            Self::WordNoop { word } => info!(
                event = "option_word_noop",
                word = %word,
                "[cli] --option の語を受け取ったが、今の areka では振る舞いが変わらない"
            ),
            Self::WordPending { word } => warn!(
                event = "option_word_pending",
                word = %word,
                "[cli] --option の語をまだ扱わないので、働きは変わらない"
            ),
            Self::WordUnknown { word } => warn!(
                event = "option_word_unknown",
                word = %word,
                "[cli] --option の知らない語なので、この語を無視する"
            ),
            Self::WordEmpty => warn!(
                event = "option_word_empty",
                "[cli] --option の空の語を無視する"
            ),
            Self::NonUtf8 { lossy } => warn!(
                event = "startup_arg_not_utf8",
                arg = %lossy,
                "[cli] 引数に読めない文字があったので、置き換えて読む"
            ),
            Self::HelpValueIgnored { value } => warn!(
                event = "help_value_ignored",
                value = %value,
                "[cli] --help の値は使わない"
            ),
            Self::OfflineIgnored { name } => warn!(
                event = "offline_option_ignored",
                name = %name,
                "[cli] --offline-dump があるので、この起動オプションを効かせない"
            ),
            Self::DumpPending { name } => warn!(
                event = "dump_option_pending",
                name = %name,
                "[cli] この --dump-* の起動オプションはまだ扱わないので、値を使わない"
            ),
            Self::DumpWithoutOffline { name } => warn!(
                event = "dump_option_without_offline",
                name = %name,
                "[cli] --dump-* の起動オプションは --offline-dump なしでは効かないので、使わない"
            ),
            Self::Listen(note) => note.log(),
            Self::InstallEmpty => warn!(
                event = "install_option_empty",
                "[cli] --install に値が無いので、入れる物が無い"
            ),
            Self::InstallWordWithoutInstall { word } => warn!(
                event = "install_word_without_install",
                word = %word,
                "[cli] --install の値が無いので、この --option の語は効かない"
            ),
            Self::Summary {
                effective,
                noop,
                ignored,
            } => info!(
                event = "startup_options",
                effective = ?effective,
                noop = ?noop,
                ignored = ?ignored,
                "[cli] 起動オプションを読み終えた"
            ),
        }
    }
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod parse_tests;

#[cfg(test)]
#[path = "parse_offline_tests.rs"]
mod parse_offline_tests;

#[cfg(test)]
#[path = "parse_readonly_tests.rs"]
mod parse_readonly_tests;

#[cfg(test)]
#[path = "parse_install_tests.rs"]
mod parse_install_tests;
