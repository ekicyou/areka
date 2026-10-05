//! 開く処理のテストの部品（areka-P0-open-external-tags task 1.2・要件 10.1・10.6）。
//!
//! 偽の [`OsPort`]（呼ばれた [`OsCall`] を順に記録し、指定した符号を返す・環境変数は表で返す）と、
//! 一時フォルダに最小のゴーストを組む部品を置く。実際のアプリは 1 つも起こさない。

use std::collections::VecDeque;

use areka_ghost::BasewareRoot;
use temp_path_kit::TempPath;

use super::os_port::{OsCall, OsPort};

/// 偽の OS の境界。
#[derive(Default)]
pub(crate) struct FakeOs {
    /// 呼ばれた順の呼び出し。
    pub calls: Vec<OsCall>,
    /// 呼ばれるたびに先頭から 1 つずつ返す結果（尽きたら成功）。
    pub results: VecDeque<Result<(), u32>>,
    /// 環境変数の表（名前は大文字小文字を区別せずに引く）。
    pub env: Vec<(String, String)>,
}

impl FakeOs {
    /// 呼ばれるたびに `results` を順に返す偽物。
    pub(crate) fn returning(results: &[Result<(), u32>]) -> Self {
        FakeOs {
            results: results.iter().copied().collect(),
            ..FakeOs::default()
        }
    }

    /// 環境変数を 1 つ足す。
    pub(crate) fn with_env(mut self, name: &str, value: &str) -> Self {
        self.env.push((name.to_owned(), value.to_owned()));
        self
    }
}

impl OsPort for FakeOs {
    fn shell_execute(&mut self, call: &OsCall) -> Result<(), u32> {
        self.calls.push(call.clone());
        self.results.pop_front().unwrap_or(Ok(()))
    }

    fn env_var(&self, name: &str) -> Option<String> {
        let key = name.to_uppercase();
        self.env
            .iter()
            .find(|(n, _)| n.to_uppercase() == key)
            .map(|(_, v)| v.clone())
    }
}

/// `dir/descript.txt` に `name` だけの descript を書く。
fn write_descript(dir: &std::path::Path, name: &str) {
    std::fs::create_dir_all(dir).expect("フォルダを組む");
    std::fs::write(
        dir.join("descript.txt"),
        format!("charset,UTF-8\nname,{name}\n"),
    )
    .expect("descript");
}

/// 根に最小のゴーストを組む。
///
/// `ghost/<ghost.0>/ghost/master/descript.txt`（name は `ghost.1`）・
/// `ghost/<ghost.0>/shell/<フォルダ>/descript.txt`・根の `balloon/<フォルダ>/descript.txt`。
/// `shells`・`balloons` は `(フォルダ, name)` の並び。
pub(crate) fn build_ghost_root(
    tmp: &TempPath,
    ghost: (&str, &str),
    shells: &[(&str, &str)],
    balloons: &[(&str, &str)],
) -> BasewareRoot {
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    let ghost_dir = root.ghost_dir(ghost.0);
    write_descript(&ghost_dir.join("ghost").join("master"), ghost.1);
    for (folder, name) in shells {
        write_descript(&ghost_dir.join("shell").join(folder), name);
    }
    for (folder, name) in balloons {
        write_descript(&root.balloon_dir(folder), name);
    }
    root
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::super::os_port::Verb;
    use super::*;

    fn call(file: &str) -> OsCall {
        OsCall {
            verb: Verb::Open,
            file: OsString::from(file),
            params: None,
            dir: None,
        }
    }

    #[test]
    fn fake_records_calls_in_order_and_returns_given_codes() {
        let mut os = FakeOs::returning(&[Ok(()), Err(2), Err(1155)]);
        assert_eq!(os.shell_execute(&call("a")), Ok(()));
        assert_eq!(os.shell_execute(&call("b")), Err(2));
        assert_eq!(os.shell_execute(&call("c")), Err(1155));
        // 尽きたら成功。
        assert_eq!(os.shell_execute(&call("d")), Ok(()));
        let files: Vec<_> = os.calls.iter().map(|c| c.file.clone()).collect();
        assert_eq!(files, ["a", "b", "c", "d"].map(OsString::from));
    }

    #[test]
    fn fake_env_is_case_insensitive_and_missing_is_none() {
        let os = FakeOs::default().with_env("Temp", r"C:\t");
        assert_eq!(os.env_var("TEMP").as_deref(), Some(r"C:\t"));
        assert_eq!(os.env_var("temp").as_deref(), Some(r"C:\t"));
        assert_eq!(os.env_var("NOPE"), None);
    }

    #[test]
    fn build_ghost_root_lays_out_minimal_ghost() {
        let tmp = TempPath::new("open-ext-ghost-root");
        let root = build_ghost_root(&tmp, ("g", "G"), &[("s", "S")], &[("b", "B")]);
        let ghost = root.ghost_dir("g");
        assert!(ghost.join("ghost/master/descript.txt").is_file());
        assert!(ghost.join("shell/s/descript.txt").is_file());
        assert!(root.balloon_dir("b").join("descript.txt").is_file());
    }
}
