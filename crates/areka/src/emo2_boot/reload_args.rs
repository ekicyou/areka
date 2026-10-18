//! 読み直しの対象の振り分け（design「ReloadArgs」・要件 1.2〜1.5・7.2・7.4・7.5・8.5・10.3）。
//!
//! fs・World・時計を読まない純粋関数だけを置く。panic しない。
//!
//! 呼び手は後のタスクで付く（MCP の入口＝3.2・台本の受け口＝6.1・descript の列＝7.5）。それまでは
//! テストだけが呼ぶので、モジュールごと `dead_code` を許す。
#![allow(dead_code)]

/// 読み直しの対象（窓口の語彙）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ReloadTarget {
    Ghost,
    Shiori,
    Shell,
    Balloon,
    Descript(DescriptParts),
}

/// descript の読み直しで読み直す部分（空は無い＝語が 1 つも無ければ呼び手が断る）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DescriptParts {
    pub ghost: bool,
    pub shell: bool,
    pub balloon: bool,
}

impl DescriptParts {
    /// すべての descript.txt（`\![reload,descript]`・MCP の `descript`・要件 7.1）。
    pub(crate) const ALL: DescriptParts = DescriptParts {
        ghost: true,
        shell: true,
        balloon: true,
    };
}

/// MCP の `target` の照合結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum McpTarget {
    Target(ReloadTarget),
    Makoto,
    Unknown,
}

impl McpTarget {
    /// ASCII の大小を区別せず 6 語と照らす。前後の空白は削らない（` shell` は Unknown）。
    pub(crate) fn parse(target: &str) -> McpTarget {
        let is = |word: &str| target.eq_ignore_ascii_case(word);
        if is("makoto") {
            McpTarget::Makoto
        } else if is("descript") {
            McpTarget::Target(ReloadTarget::Descript(DescriptParts::ALL))
        } else if is("ghost") {
            McpTarget::Target(ReloadTarget::Ghost)
        } else if is("shiori") {
            McpTarget::Target(ReloadTarget::Shiori)
        } else if is("shell") {
            McpTarget::Target(ReloadTarget::Shell)
        } else if is("balloon") {
            McpTarget::Target(ReloadTarget::Balloon)
        } else {
            McpTarget::Unknown
        }
    }
}

/// 台本の綴りからの写し。`None` は「本 spec が拾わない」（名前違い・aigraph・makoto・引数なしの reload）。
pub(crate) fn parse_script(name: &str, params: &[String]) -> Option<ScriptReload> {
    let plain = |target| ScriptReload {
        target,
        notes: DescriptNotes::default(),
    };
    match name {
        // `\![reloadsurface]` は `\![reload,shell]` と同じ（要件 1.4）。
        "reloadsurface" => Some(plain(ReloadTarget::Shell)),
        "reload" => match params.first()?.as_str() {
            "ghost" => Some(plain(ReloadTarget::Ghost)),
            "shiori" => Some(plain(ReloadTarget::Shiori)),
            "shell" => Some(plain(ReloadTarget::Shell)),
            "balloon" => Some(plain(ReloadTarget::Balloon)),
            "descript" => Some(match params.get(1) {
                None => plain(ReloadTarget::Descript(DescriptParts::ALL)),
                Some(param) => {
                    let (parts, notes) = parse_descript_params(param);
                    ScriptReload {
                        target: ReloadTarget::Descript(parts),
                        notes,
                    }
                }
            }),
            // makoto・aigraph ほかは拾わない（要件 1.5・10.3）。
            _ => None,
        },
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScriptReload {
    pub target: ReloadTarget,
    pub notes: DescriptNotes,
}

/// `\![reload,descript,パラメータ]` の語の仕分け（半角スペース区切り・第 3 引数より後ろは見ない）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct DescriptNotes {
    /// areka に無い種類（`headline`・`plugin`・`language`・`calendar.skin`・`calendar.plugin`・
    /// `calendar skin`・`calendar plugin`）。
    pub unsupported: Vec<String>,
    /// 上の語でも ghost／shell／balloon でもない語。
    pub unknown: Vec<String>,
    /// 読み直せる語が 1 つも無い。
    pub empty: bool,
}

pub(crate) fn parse_descript_params(param: &str) -> (DescriptParts, DescriptNotes) {
    let mut parts = DescriptParts {
        ghost: false,
        shell: false,
        balloon: false,
    };
    let mut notes = DescriptNotes::default();
    let push_once = |list: &mut Vec<String>, word: String| {
        if !list.contains(&word) {
            list.push(word);
        }
    };
    let mut words = param.split(' ').filter(|w| !w.is_empty()).peekable();
    while let Some(word) = words.next() {
        match word {
            "ghost" => parts.ghost = true,
            "shell" => parts.shell = true,
            "balloon" => parts.balloon = true,
            "headline" | "plugin" | "language" | "calendar.skin" | "calendar.plugin" => {
                push_once(&mut notes.unsupported, word.to_string())
            }
            // `calendar skin`・`calendar plugin` の 2 語で 1 つの種類。
            "calendar" => match words.next_if(|next| matches!(*next, "skin" | "plugin")) {
                Some(next) => push_once(&mut notes.unsupported, format!("calendar {next}")),
                None => push_once(&mut notes.unknown, word.to_string()),
            },
            _ => push_once(&mut notes.unknown, word.to_string()),
        }
    }
    notes.empty = !(parts.ghost || parts.shell || parts.balloon);
    (parts, notes)
}

#[cfg(test)]
#[path = "reload_args_tests.rs"]
mod tests;
