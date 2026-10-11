//! 控え（サーバーの情報・ツールの一覧）の読み書き。
//!
//! areka が居ないときに答えるための、最後に areka から受け取った答え 2 つ（要件 3.1〜3.3）。
//! 形は rmcp の型そのものの JSON で、鍵や版の欄は持たない。読めない・書けないときは標準エラーに
//! 理由を残し、無いものとして続ける（要件 3.10。失敗を呼び手へ返さない）。

use std::io;
use std::path::{Path, PathBuf};

use rmcp::model::{ListToolsResult, ServerPeerInfo};
use tracing::warn;

/// サーバーの情報の控えのファイル名。
const SERVER_INFO_FILE: &str = "server-info.json";
/// ツールの一覧の控えのファイル名。
const TOOLS_FILE: &str = "tools.json";

/// 控えの置き場。`dir` が `None` なら読まず書かず（その旨は `Config::from_env` が 1 度記録する）。
pub struct Cache {
    dir: Option<PathBuf>,
}

impl Cache {
    pub fn new(dir: Option<PathBuf>) -> Cache {
        Cache { dir }
    }

    /// サーバーの情報の控え。無い・読めないは `None`。
    pub fn load_server_info(&self) -> Option<ServerPeerInfo> {
        let (path, bytes) = self.read(SERVER_INFO_FILE)?;
        parsed(&path, serde_json::from_slice(&bytes))
    }

    /// サーバーの情報の控えを置き換える。
    pub fn save_server_info(&self, info: &ServerPeerInfo) {
        self.write(SERVER_INFO_FILE, serde_json::to_vec(info));
    }

    /// ツールの一覧の控え。無い・読めないは `None`。
    pub fn load_tools(&self) -> Option<ListToolsResult> {
        let (path, bytes) = self.read(TOOLS_FILE)?;
        parsed(&path, serde_json::from_slice(&bytes))
    }

    /// ツールの一覧の控えを置き換える。
    pub fn save_tools(&self, tools: &ListToolsResult) {
        self.write(TOOLS_FILE, serde_json::to_vec(tools));
    }

    /// 控えを丸ごと読む。まだ無いのは普通のこと（最初の起動）なので記録しない。
    fn read(&self, name: &str) -> Option<(PathBuf, Vec<u8>)> {
        let path = self.dir.as_ref()?.join(name);
        match std::fs::read(&path) {
            Ok(bytes) => Some((path, bytes)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => {
                warn!(path = %path.display(), %error, "[bridge] cannot read the cache; treating it as absent");
                None
            }
        }
    }

    fn write(&self, name: &str, json: serde_json::Result<Vec<u8>>) {
        let Some(dir) = &self.dir else { return };
        let written = json
            .map_err(io::Error::from)
            .and_then(|bytes| replace(dir, name, &bytes, std::process::id()));
        if let Err(error) = written {
            warn!(path = %dir.join(name).display(), %error, "[bridge] cannot write the cache; continuing without it");
        }
    }
}

/// 読んだ中身を解いた結果。壊れている・途中まで・形が違う控えは無いものとして扱う。
fn parsed<T>(path: &Path, result: serde_json::Result<T>) -> Option<T> {
    result
        .inspect_err(|error| {
            warn!(path = %path.display(), %error, "[bridge] the cache is not readable JSON; treating it as absent");
        })
        .ok()
}

/// 同じフォルダの一時の名前（`<名前>.<プロセス番号>.tmp`）へ全部書いてから、名前の付け替えで置き換える。
///
/// 置き場にある控えは、いつもどれか 1 つの答えの全体（要件 3.11）。同時に動く中継は別のプロセスで
/// 一時の名前が違うので、互いの書きかけを踏まない。プロセス番号を引数で受けるのは、テストが
/// 1 つのプロセスの中で 2 つの中継の形を作るため。
fn replace(dir: &Path, name: &str, bytes: &[u8], pid: u32) -> io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let tmp = dir.join(format!("{name}.{pid}.tmp"));
    let replaced = std::fs::write(&tmp, bytes).and_then(|()| std::fs::rename(&tmp, dir.join(name)));
    if replaced.is_err() {
        // 書きかけを残さない。消せなくても元の失敗を返す（次の書きが同じ名前を上書きする）。
        let _ = std::fs::remove_file(&tmp);
    }
    replaced
}

#[cfg(test)]
#[path = "cache_tests.rs"]
mod tests;
