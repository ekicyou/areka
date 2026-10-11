//! 設定: 候補の列・控えの置き場・待つ時間。
//!
//! 環境と exe の場所を読むのは [`Config::from_env`] の 1 か所だけで、判断は値だけを受け取る
//! [`Config::from_values`] が持つ（`areka_mcp::candidates_from_env_value` と同じ分け方）。

use std::path::PathBuf;
use std::time::Duration;

use tracing::warn;

/// 候補へ並べて開く TCP の接続を待つ上限。
pub const CONNECT_WAIT: Duration = Duration::from_millis(300);
/// 相手を確かめる `initialize` の応答を待つ上限。
pub const VERIFY_WAIT: Duration = Duration::from_secs(3);
/// 届けた要求の応答を待つ上限。areka のツールの上限（`areka_mcp::tools::REPLY_WAIT`）より長い。
pub const REPLY_WAIT: Duration = Duration::from_secs(30);

/// 記憶の置き場を変える環境変数（areka 本体と同じ名前。空の値も「設定あり」として扱うのも同じ）。
const PROFILE_DIR_ENV: &str = "AREKA_PROFILE_DIR";
/// 中継の隣に居れば配布 zip の形と見なす、areka 本体の exe の名前。
const AREKA_EXE_FILE_NAME: &str = "areka.exe";
/// 控えのフォルダの名前。
const CACHE_DIR_NAME: &str = "mcp-bridge";

/// 待つ時間の 3 つ。既定は上の 3 つの定数で、テストは短い値に差し替える。
#[derive(Debug, Clone)]
pub struct Timeouts {
    pub connect: Duration,
    pub verify: Duration,
    pub reply: Duration,
}

impl Default for Timeouts {
    fn default() -> Self {
        Timeouts {
            connect: CONNECT_WAIT,
            verify: VERIFY_WAIT,
            reply: REPLY_WAIT,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    /// 試す番号の列。空は `AREKA_MCP_PORT=0`（待ち受けない設定）。
    pub candidates: Vec<u16>,
    /// 控えの置き場。`None` は控えを持たない。
    pub cache_dir: Option<PathBuf>,
    pub timeouts: Timeouts,
}

impl Config {
    /// 環境（`AREKA_MCP_PORT`・`AREKA_PROFILE_DIR`・exe の場所と隣の `areka.exe` の有無・
    /// `LOCALAPPDATA`）を読む唯一の口。exe の場所は `current_exe` のまま（リンクは辿らない）。
    pub fn from_env() -> Config {
        let exe = std::env::current_exe().ok();
        let areka_beside = exe
            .as_deref()
            .is_some_and(|exe| exe.with_file_name(AREKA_EXE_FILE_NAME).is_file());
        // UTF-8 でない値を「未設定」に丸めない: 読めない字を U+FFFD に替えた文は数として読めないので、
        // 共有の関数が `warn!` を 1 件残して既定の 20 個にする（要件 2.5）。
        let port_value =
            std::env::var_os(areka_mcp::PORT_ENV).map(|v| v.to_string_lossy().into_owned());
        let config = Config::from_values(
            port_value.as_deref(),
            std::env::var_os(PROFILE_DIR_ENV).map(PathBuf::from),
            exe,
            areka_beside,
            std::env::var_os("LOCALAPPDATA").map(PathBuf::from),
        );
        if config.cache_dir.is_none() {
            warn!(
                "[bridge] no place for the cache (AREKA_PROFILE_DIR, areka.exe beside the bridge, LOCALAPPDATA: none available); running without it"
            );
        }
        config
    }

    /// 値 → 設定（I/O なし）。
    ///
    /// 候補の列は areka 本体と同じ関数の結果そのもの（読めない値の `warn!` もその関数が出す）。
    /// 控えの置き場は 3 段: `profile_dir` の下 → `areka_beside` のときだけ `exe` の隣の
    /// `profile\areka` の下 → `local_app_data` の `areka` の下 → なし。`local_app_data` は絶対パスの
    /// ときだけ使う（空・相対の値で、起こされた場所しだいの置き場に黙って書かない）。
    pub fn from_values(
        port_value: Option<&str>,
        profile_dir: Option<PathBuf>,
        exe: Option<PathBuf>,
        areka_beside: bool,
        local_app_data: Option<PathBuf>,
    ) -> Config {
        let beside = exe
            .filter(|_| areka_beside)
            .and_then(|exe| Some(exe.parent()?.join("profile").join("areka")));
        let local = local_app_data
            .filter(|dir| dir.is_absolute())
            .map(|dir| dir.join("areka"));
        let cache_dir = profile_dir
            .or(beside)
            .or(local)
            .map(|dir| dir.join(CACHE_DIR_NAME));
        Config::from_parts(
            areka_mcp::candidates_from_env_value(port_value),
            cache_dir,
            Timeouts::default(),
        )
    }

    /// 部品をそのまま持つ（テストが候補の列と時間を差し込む口）。
    pub fn from_parts(
        candidates: Vec<u16>,
        cache_dir: Option<PathBuf>,
        timeouts: Timeouts,
    ) -> Config {
        Config {
            candidates,
            cache_dir,
            timeouts,
        }
    }
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
