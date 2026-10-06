//! 行き先の規則（純粋・areka-P0-open-external-tags task 2.1・2.2・要件 1.1・1.4・1.7・2.6・
//! 3.2・4.7・5.2・6.3・8.1〜8.5）。
//!
//! 開く系のタグ（`\j[ID]` と `\![open,file|browser|explorer|editor|mailer,…]`）から、
//! 行き先・種類・断りを決める唯一の規則 [`classify`] を置く。台本の受け口（実行時）と
//! 台本からの取り出しの両方がこの関数だけを呼ぶ（要件 8.6 を構造で保つ）。
//!
//! World・fs・OS・環境変数・記録のどれにも触れない（要件 8.4）。相対パスの解決・
//! 環境変数の展開・`mailto:` の付け足しは行わず、書かれた綴りのまま持つ（解決は opener）。
//! `\![open,readme]` は対象外（`None`）で、説明書は受け口が先に拾う（要件 8.3）。

use areka_parsers::sakura::{Instruction, JUMP_TAG_CARRIER};

/// 開く系の種類（記録の欄 `kind`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OpenKind {
    Url,
    File,
    Folder,
    Mail,
    Editor,
}

impl OpenKind {
    /// 記録の欄 `kind` の値（"url" | "file" | "folder" | "mail" | "editor"）。
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            OpenKind::Url => "url",
            OpenKind::File => "file",
            OpenKind::Folder => "folder",
            OpenKind::Mail => "mail",
            OpenKind::Editor => "editor",
        }
    }
}

/// `\![open,explorer,種類,名前]` の置き場（`headline`・`plugin` は areka に無い）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Store {
    Ghost,
    Balloon,
    Shell,
}

/// 書かれた綴りのままの行き先（解決は opener が行う）。変種は「解決の規則」ごとに分ける。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Target {
    /// そのまま OS へ。
    Url(String),
    /// `mailto:` が無ければ付けて OS へ。
    Mail(String),
    /// 絶対か ghost/master 基準・実在が要る（`\j` の `file:///` と説明書）。
    Path(String),
    /// 環境変数を展開 → Path の規則 → 名前だけならパス探索。
    Program(String),
    /// Path の規則 → フォルダは開く・ファイルは選んで示す。
    Folder(String),
    /// 目録で名前を引く。
    NamedFolder { store: Store, name: String },
    /// Path の規則 → 「編集」の動詞。
    Edit(String),
}

impl Target {
    /// 行き先の種類。
    pub(crate) fn kind(&self) -> OpenKind {
        match self {
            Target::Url(_) => OpenKind::Url,
            Target::Mail(_) => OpenKind::Mail,
            Target::Path(_) | Target::Program(_) => OpenKind::File,
            Target::Folder(_) | Target::NamedFolder { .. } => OpenKind::Folder,
            Target::Edit(_) => OpenKind::Editor,
        }
    }
}

/// 受理された行き先。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Destination {
    pub target: Target,
    /// 要件 8.1 の X（書かれた綴りのまま）。
    pub written: String,
    /// 元のタグの組み直した綴り（記録の欄 `tag`）。
    pub tag: String,
}

/// 断りの理由。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Rejection {
    /// 引数が無い・空（要件 2.6・3.2・6.3）。
    MissingArgument,
    /// `\j[ID]` の ID が `http(s)://`・`file:///`・`mailto:` のどれでもない（要件 1.7）。
    UnknownJumpId,
    /// `\![open,explorer,種類,名前]` の種類が扱えない（要件 4.7）。
    UnsupportedStore(String),
}

/// 断られた開く系のタグ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Rejected {
    /// 元のタグの組み直した綴り。
    pub tag: String,
    pub reason: Rejection,
}

/// `\!` のコマンド名 `open`。
const NAME_OPEN: &str = "open";

/// 前置きを大文字小文字を区別せずに剥がす。
fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    // 前置きは ASCII だけなので、バイト数で切ってよい（境界が文字の途中なら get が None）。
    let head = s.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then(|| &s[prefix.len()..])
}

/// 開く系でなければ None。受け口と行き先の取り出しの唯一の規則。
///
/// `name`・`args` は汎用の運び手の中身（`args[0]` が第 1 引数）。どんな入力でも panic しない。
pub(crate) fn classify<S: AsRef<str>>(
    name: &str,
    args: &[S],
) -> Option<Result<Destination, Rejected>> {
    let args: Vec<&str> = args.iter().map(AsRef::as_ref).collect();
    if name == JUMP_TAG_CARRIER {
        let tag = format!("\\j[{}]", args.join(","));
        return Some(classify_jump(args.first().copied().unwrap_or(""), tag));
    }
    if name != NAME_OPEN {
        return None;
    }
    let (&selector, rest) = args.split_first()?;
    let tag = format!("\\![{NAME_OPEN},{}]", args.join(","));
    let x = rest.first().copied().unwrap_or("");
    let accept = |target: Target, written: &str| {
        Ok(Destination {
            target,
            written: written.to_owned(),
            tag: tag.clone(),
        })
    };
    let missing = || {
        Err(Rejected {
            tag: tag.clone(),
            reason: Rejection::MissingArgument,
        })
    };
    let result = match selector {
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bopen_2cfile_2c_30d5_30a1_30a4_30eb_540d_5d:1
        "file" if !x.is_empty() => accept(Target::Program(x.to_owned()), x),
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bopen_2cbrowser_2c_30d1_30e9_30e1_30fc_30bf_5d:1
        "browser" if !x.is_empty() => accept(Target::Url(x.to_owned()), x),
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bopen_2cexplorer_2c_30d5_30a1_30a4_30eb_5d:1
        "explorer" if rest.len() >= 2 => {
            let (kind, store_name) = (rest[0], rest[1]);
            let store = match kind {
                "ghost" => Store::Ghost,
                "balloon" => Store::Balloon,
                "shell" => Store::Shell,
                // headline・plugin・知らない種類（要件 4.7）
                other => {
                    return Some(Err(Rejected {
                        tag,
                        reason: Rejection::UnsupportedStore(other.to_owned()),
                    }));
                }
            };
            if store_name.is_empty() {
                missing()
            } else {
                let target = Target::NamedFolder {
                    store,
                    name: store_name.to_owned(),
                };
                accept(target, &format!("{kind},{store_name}"))
            }
        }
        "explorer" if !x.is_empty() => accept(Target::Folder(x.to_owned()), x),
        // 表示行（第 3 引数以降）は読まない（要件 5.2）。
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bopen_2ceditor_2c_30d5_30a1_30a4_30eb_2c_8868_793a_884c_5d:1
        "editor" if !x.is_empty() => accept(Target::Edit(x.to_owned()), x),
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bopen_2cmailer_2c_30d1_30e9_30e1_30fc_30bf_5d:1
        "mailer" if !x.is_empty() => accept(Target::Mail(x.to_owned()), x),
        // X が無い・空（要件 2.6・3.2・6.3）
        "file" | "browser" | "explorer" | "editor" | "mailer" => missing(),
        // readme（受け口が先に拾う）・help・他の名前は対象外（要件 8.3）
        _ => return None,
    };
    Some(result)
}

/// `\j[ID]` の分類（要件 1.1・1.4・1.7）。前置きは大文字小文字を区別しない。
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5cj_5bID_5d:1
fn classify_jump(id: &str, tag: String) -> Result<Destination, Rejected> {
    let target =
        if strip_prefix_ci(id, "http://").is_some() || strip_prefix_ci(id, "https://").is_some() {
            Target::Url(id.to_owned())
        } else if strip_prefix_ci(id, "mailto:").is_some() {
            Target::Mail(id.to_owned())
        } else if let Some(path) = strip_prefix_ci(id, "file:///") {
            if path.is_empty() {
                return Err(Rejected {
                    tag,
                    reason: Rejection::MissingArgument,
                });
            }
            Target::Path(path.to_owned())
        } else {
            return Err(Rejected {
                tag,
                reason: Rejection::UnknownJumpId,
            });
        };
    Ok(Destination {
        target,
        written: id.to_owned(),
        tag,
    })
}

/// 台本の文字列から、開く系の行き先を現れた順に返す（断られるものは含めない・task 2.2・
/// 要件 8.1〜8.5）。汎用コマンドを順に [`classify`] へ通すだけの 1 回の線形走査。
pub(crate) fn link_destinations(script: &str) -> Vec<Destination> {
    areka_parsers::sakura::parse(script)
        .into_iter()
        .filter_map(|ins| match ins {
            Instruction::GenericCommand { name, raw_args } => classify(&name, &raw_args)?.ok(),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
#[path = "destination_tests.rs"]
mod tests;
