//! 開く処理のテストの部品。
//!
//! 偽の [`OsPort`]（呼ばれた [`OsCall`] を順に記録し、指定した符号を返す・環境変数は表で返す）と、
//! 台本のタグの形から行き先を作る部品、一時フォルダに最小のゴーストを組む部品を置く。実際のアプリは 1 つも起こさない。

use std::collections::VecDeque;

use areka_ghost::BasewareRoot;
use areka_ghost::sink::{GhostPlace, TalkRunner};
use temp_path_kit::TempPath;

use super::destination::{Destination, classify};
use super::opener::OpenContext;
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

/// 断る口が受けた呼び出しの `lpFile`（プロセス共有・テストのビルドの開く専用のスレッドが積む）。
static REFUSED: std::sync::Mutex<Vec<std::ffi::OsString>> = std::sync::Mutex::new(Vec::new());

/// テストのビルドの開く専用のスレッドが持つ口: OS を呼ばずに記録して断る。
pub(crate) struct RefusingOs;

/// 断る口が返す符号（ERROR_ACCESS_DENIED）。
const REFUSED_CODE: u32 = 5;

impl OsPort for RefusingOs {
    fn shell_execute(&mut self, call: &OsCall) -> Result<(), u32> {
        REFUSED
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(call.file.clone());
        Err(REFUSED_CODE)
    }

    fn env_var(&self, _name: &str) -> Option<String> {
        None
    }
}

/// 断る口の記録の写し（他のテストも積みうるので、件数や位置でなく中身で判定する）。
pub(crate) fn refused_files() -> Vec<std::ffi::OsString> {
    REFUSED.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// `dir/descript.txt` に `name` だけの descript を書く。
pub(crate) fn write_descript(dir: &std::path::Path, name: &str) {
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

/// 台本のタグの形から本物の規則で行き先を作る。
pub(crate) fn dest_of(name: &str, args: &[&str]) -> Destination {
    classify(name, args).expect("開く系").expect("受理される")
}

/// `\j[url]` から本物の規則で行き先を作る。
pub(crate) fn dest_url(url: &str) -> Destination {
    dest_of(areka_parsers::sakura::JUMP_TAG_CARRIER, &[url])
}

/// `build_ghost_root(.., ("g", "G"), ..)` で組んだ根のゴーストの開く文脈。
pub(crate) fn open_context(root: &BasewareRoot) -> OpenContext {
    let ghost_dir = root.ghost_dir("g");
    OpenContext {
        runner: TalkRunner {
            name: "G".to_owned(),
            place: Some(GhostPlace {
                master: ghost_dir.join("ghost").join("master"),
                root: ghost_dir,
            }),
        },
        baseware: Some(areka_ghost::BasewareRoots::single(root.dir().to_path_buf())),
    }
}

/// 文脈の実行するゴーストの最上位（置き場の無い文脈では落ちる）。
pub(crate) fn ghost_root(ctx: &OpenContext) -> &std::path::Path {
    &ctx.runner.place.as_ref().expect("置き場").root
}
