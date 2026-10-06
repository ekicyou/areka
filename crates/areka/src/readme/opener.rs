//! 開く処理の 1 か所（areka-P0-open-external-tags task 3.1・要件 1.1〜1.4・2.1〜2.4・3.1・
//! 4.1〜4.3・5.1・5.3・6.1・6.2・10.2）。
//!
//! 行き先（[`Destination`]）と文脈（[`OpenContext`]）から OS への 1 回分の呼び出し（[`OsCall`]）を
//! 作る [`resolve`] を置く。fs は読むが OS は呼ばない（呼ぶのは開く専用のスレッドの実行だけ）。

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::destination::{Destination, Store, Target};
use super::os_port::{OsCall, OsPort, Verb};

/// 開く文脈（UI スレッドで World から写す）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OpenContext {
    /// get_active_ghost_list と同じ名（記録の `ghost`）。
    pub ghost: String,
    /// ゴーストのフォルダ（`ghost/<フォルダ>`）の絶対パス。
    pub ghost_dir: PathBuf,
    /// ベースウェアの根（BootContext が無ければ None）。
    pub baseware: Option<areka_ghost::BasewareRoot>,
}

/// 解決の失敗。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OpenFailure {
    /// 解いたパスに何も無い。
    NotFound(PathBuf),
    /// `\![open,explorer,種類,名前]` の名前に当たらない。
    NoMatch { store: Store, name: String },
}

/// `mailto:` の前置き（大文字小文字を区別しない）。
const MAILTO: &str = "mailto:";

/// 解決（fs を読む・OS は呼ばない）。
pub(crate) fn resolve(
    dest: &Destination,
    ctx: &OpenContext,
    port: &dyn OsPort,
) -> Result<OsCall, OpenFailure> {
    match &dest.target {
        Target::Url(s) => Ok(call(Verb::Open, s, None, None)),
        Target::Mail(s) => {
            let has_mailto = s
                .get(..MAILTO.len())
                .is_some_and(|h| h.eq_ignore_ascii_case(MAILTO));
            let to = if has_mailto {
                s.clone()
            } else {
                format!("{MAILTO}{s}")
            };
            Ok(call(Verb::Open, to, None, None))
        }
        Target::Path(s) => {
            let path = existing(ctx, s)?;
            Ok(open_file(path))
        }
        Target::Program(s) => {
            let expanded = expand_env(s, port);
            match existing(ctx, &expanded) {
                Ok(path) => Ok(open_file(path)),
                // 名前だけ（区切りを含まない）なら OS のパス探索に任せる（要件 2.4）。
                Err(_) if !expanded.contains(['\\', '/', ':']) => {
                    Ok(call(Verb::Open, expanded, None, None))
                }
                Err(e) => Err(e),
            }
        }
        Target::Folder(s) => {
            let path = existing(ctx, s)?;
            if path.is_dir() {
                Ok(call(Verb::Open, path, None, None))
            } else {
                // ファイルは選んだ状態でフォルダを開く（要件 4.2）。
                let mut params = OsString::from("/select,\"");
                params.push(&path);
                params.push("\"");
                Ok(call(Verb::Open, "explorer.exe", Some(params), None))
            }
        }
        Target::Edit(s) => {
            let path = existing(ctx, s)?;
            Ok(call(Verb::Edit, path, None, None))
        }
        // 目録で名前を引くのは task 3.2。それまでは当たらない扱い。
        Target::NamedFolder { store, name } => Err(OpenFailure::NoMatch {
            store: *store,
            name: name.clone(),
        }),
    }
}

/// 1 回分の呼び出しを組む。
fn call(
    verb: Verb,
    file: impl Into<OsString>,
    params: Option<OsString>,
    dir: Option<PathBuf>,
) -> OsCall {
    OsCall {
        verb,
        file: file.into(),
        params,
        dir,
    }
}

/// ファイルを開く（作業フォルダはそのファイルのあるフォルダ＝エクスプローラーのダブルクリックと同じ）。
fn open_file(path: PathBuf) -> OsCall {
    let dir = path.parent().map(Path::to_path_buf);
    call(Verb::Open, path, None, dir)
}

/// 絶対ならそのまま、相対なら `ghost/master` に繋ぐ。無ければ [`OpenFailure::NotFound`]。
fn existing(ctx: &OpenContext, written: &str) -> Result<PathBuf, OpenFailure> {
    let p = Path::new(written);
    let path = if p.is_absolute() {
        p.to_path_buf()
    } else {
        ctx.ghost_dir.join("ghost").join("master").join(p)
    };
    if path.exists() {
        Ok(path)
    } else {
        Err(OpenFailure::NotFound(path))
    }
}

/// `%名前%` を境界の環境変数で置き換える。未定義はそのまま残す（OS の展開と同じ）。
fn expand_env(s: &str, port: &dyn OsPort) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(open) = rest.find('%') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let Some(close) = after.find('%') else {
            // 閉じの無い `%` は字面のまま。
            out.push_str(&rest[open..]);
            return out;
        };
        let name = &after[..close];
        match (!name.is_empty()).then(|| port.env_var(name)).flatten() {
            Some(value) => {
                out.push_str(&value);
                rest = &after[close + 1..];
            }
            None => {
                // 未定義は `%名前` を残し、閉じの `%` を次の開きとして読み直す。
                out.push('%');
                out.push_str(name);
                rest = &after[close..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
#[path = "opener_tests.rs"]
mod tests;
