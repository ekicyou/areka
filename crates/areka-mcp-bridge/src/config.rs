//! 設定: 候補の列・控えの置き場・待つ時間。
//!
//! 環境と exe の場所を読むのは [`Config::from_env`] の 1 か所だけで、判断は値だけを受け取る
//! [`Config::from_inputs`]（決めるだけの口と記録）と [`Config::from_values`]（I/O なし）が持つ
//! （`areka_mcp::candidates_from_env_value` と同じ分け方）。

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use areka_user_data::AppProfile;
use tracing::{info, warn};

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
    ///
    /// 読むだけで、判断と記録は [`Config::from_inputs`] に渡す。隣に `areka.exe` が在るときだけ、
    /// areka と同じ `probe` で判断の入力を取る（置き場を作る準備は呼ばない＝作らない・写さない）。
    pub fn from_env() -> Config {
        let areka_dir = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(PathBuf::from))
            .filter(|dir| dir.join(AREKA_EXE_FILE_NAME).is_file());
        let signals = areka_dir
            .as_deref()
            .map(|dir| areka_user_data::probe(Some(dir)));
        let profile_env = std::env::var_os(PROFILE_DIR_ENV);
        // UTF-8 でない値を「未設定」に丸めない: 読めない字を U+FFFD に替えた文は数として読めないので、
        // 共有の関数が `warn!` を 1 件残して既定の 20 個にする（要件 2.5）。
        let port_value =
            std::env::var_os(areka_mcp::PORT_ENV).map(|v| v.to_string_lossy().into_owned());
        let local_app_data = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
        let config = Config::from_inputs(
            port_value.as_deref(),
            profile_env.as_deref(),
            areka_dir.as_deref().zip(signals.as_ref()),
            local_app_data.clone(),
        );
        // 箱の見立ては、控えを決めた後に 1 回（読んだのと同じ `LOCALAPPDATA` で・要件 3.5〜3.9）。
        crate::redirect::report(&crate::redirect::observe(
            local_app_data.as_deref(),
            config.cache_dir.as_deref(),
        ));
        config
    }

    /// [`Config::from_env`] の内側（本物の環境を読まない）。`areka` は隣に areka.exe が在るときの
    /// （実行ファイルのフォルダ, `probe` の答え）。
    ///
    /// 「決めるだけ」の口（`locate_app_profile`）→ 値の判断（[`Config::from_values`]）→
    /// 控えを持たない理由の記録、の順。どれも作らない・写さない。
    pub(crate) fn from_inputs(
        port_value: Option<&str>,
        profile_env: Option<&OsStr>,
        areka: Option<(&Path, &areka_user_data::Signals)>,
        local_app_data: Option<PathBuf>,
    ) -> Config {
        let beside = areka.map(|(exe_dir, signals)| {
            areka_user_data::locate_app_profile(signals, exe_dir, profile_env)
        });
        let config = Config::from_values(
            port_value,
            profile_env.map(PathBuf::from),
            beside.clone(),
            local_app_data,
        );
        log_without_cache(beside.as_ref(), &config);
        config
    }

    /// 値 → 設定（I/O なし）。
    ///
    /// 候補の列は areka 本体と同じ関数の結果そのもの（読めない値の `warn!` もその関数が出す）。
    /// 控えの置き場は 3 段: `profile_dir` の下 → `beside`（隣の areka の記憶の置き場の答え）→
    /// `local_app_data` の `areka` の下 → なし。どれもその下の `mcp-bridge`。
    /// `beside` は `None`＝隣に areka.exe が無い、`Some(Ready(dir))`＝その下、`Some(ほか)`＝控え無し
    /// （まだ置けない・決められない根の代わりに `local_app_data` の段へ落ちない）。
    /// `local_app_data` は絶対パスのときだけ使う（空・相対の値で、起こされた場所しだいの置き場に黙って書かない）。
    pub fn from_values(
        port_value: Option<&str>,
        profile_dir: Option<PathBuf>,
        beside: Option<AppProfile>,
        local_app_data: Option<PathBuf>,
    ) -> Config {
        let base = match (profile_dir, beside) {
            (Some(dir), _) => Some(dir),
            (None, Some(AppProfile::Ready(dir))) => Some(dir),
            (None, Some(_)) => None,
            (None, None) => local_app_data
                .filter(|dir| dir.is_absolute())
                .map(|dir| dir.join("areka")),
        };
        let cache_dir = base.map(|dir| dir.join(CACHE_DIR_NAME));
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

/// 控えを持たないときの 1 件（持つときは 0 件）。文は ASCII・頭は `[bridge]`。
fn log_without_cache(beside: Option<&AppProfile>, config: &Config) {
    if config.cache_dir.is_some() {
        return;
    }
    match beside {
        // 次に起こされたとき、areka が置き場を作っていれば控えを持つ（要件 2.11）。
        Some(AppProfile::RootMissing { root }) => info!(
            reason = "user_root_missing",
            user_root = %root.display(),
            "[bridge] running without the cache until areka prepares its user root; it is used from the next start"
        ),
        Some(AppProfile::MigrationPending { root }) => info!(
            reason = "app_memory_not_migrated",
            user_root = %root.display(),
            "[bridge] running without the cache until areka prepares its user root; it is used from the next start"
        ),
        // 決められない理由は `areka-user-data` の error が先に残している（要件 3.3）。
        Some(AppProfile::Unlocatable) => {
            warn!("[bridge] the user root of areka could not be located; running without the cache")
        }
        // `Ready` は必ず控えを持つので、ここへ来るのは隣に areka が無いときだけ。
        Some(AppProfile::Ready(_)) | None => warn!(
            "[bridge] no place for the cache (AREKA_PROFILE_DIR, areka profile beside the bridge, LOCALAPPDATA: none available); running without it"
        ),
    }
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "config_prepare_tests.rs"]
mod prepare_tests;
