//! 起動時の構成入力解決と ghost 結線ヘルパ（`main.rs` から切り出し）。
//!
//! `main.rs` が 1,000 行規約（`.kiro/steering/structure.md`）を超えたため、
//! 相互に凝集した「起動引数・既定パスの解決」と「`GhostBootOptions` の組み立て」を
//! 本モジュールへ移した。**挙動は 1 ビットも変えていない**（可視性を `pub(crate)` へ
//! 広げただけで、式・分岐・doc の主張はすべて移設前と逐語同一）。
//!
//! 消費者は `main.rs`（`pub(crate) use` で crate 直下へ再輸出）と `emo2_boot`
//! （`crate::default_app_profile_dir` / `crate::is_benign_boot_error`）、および
//! 既存の檻 `main_config_input_tests.rs` / `main_ghost_wiring_tests.rs`。

// ---------------------------------------------------------------------------
// Config Inputs (task 2.1)
// ---------------------------------------------------------------------------

/// 構成入力（解決済みルートパス）。
///
/// ゴースト／バルーンのルートパスを保持する。決定のみで実在は保証しない
/// （マウント・descript.txt 読取・`areka-parsers` 呼び出しは一切行わない・R6.1）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfigInputs {
    pub(crate) ghost_root: std::path::PathBuf,
    pub(crate) balloon_root: std::path::PathBuf,
}

/// ゴーストルートの既定パス（`CARGO_MANIFEST_DIR` 相対・DD1）。
///
/// `crates/areka` 配下には現状ゴースト fixture が無いため（emo2 fixture は別クレート
/// `crates/pilot/...` にありクロスクレート `../` 参照は脆いので採らない）、ukadoc 標準の
/// ルート配置 `ghost/master` を **プレースホルダ subpath** として採用する。実在は検証せず、
/// 実マウント対象の確定は下流 ghost-setup の領分（本仕様スコープ外）。
pub(crate) fn default_ghost_root() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/ghost/master"))
}

/// バルーンルートの既定パス（`CARGO_MANIFEST_DIR` 相対・DD1）。
///
/// ゴースト既定と同じく `env!("CARGO_MANIFEST_DIR")` 相対のプレースホルダ subpath
/// `balloon/master` を採用する（実在検証なし・下流 ghost-setup が実体を確定）。
pub(crate) fn default_balloon_root() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/balloon/master"))
}

/// 起動引数（位置引数）と既定パスから構成入力を決定する。純粋・副作用なし。
///
/// - `args[0]` は実行ファイル名。`args[1]` = ghost root、`args[2]` = balloon root。
/// - 位置引数が与えられていれば採用し（R3.3）、欠落時は `CARGO_MANIFEST_DIR` 相対の
///   既定へフォールバックする（R3.4・DD1）。
/// - `args` を入力に取ることで `std::env::args()` を内部で呼ばず、実プロセス引数に触れずに
///   単体テスト可能な純粋関数に保つ。std（`std::path`・`env!`）のみに依存し、マウントも
///   descript.txt 読取も行わない（R6.1）。
pub(crate) fn resolve_config_inputs(args: &[String]) -> ConfigInputs {
    let ghost_root = args
        .get(1)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(default_ghost_root);
    let balloon_root = args
        .get(2)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(default_balloon_root);
    ConfigInputs {
        ghost_root,
        balloon_root,
    }
}

// ---------------------------------------------------------------------------
// Ghost Wiring (task 3.3)
// ---------------------------------------------------------------------------

/// 実行ファイル隣接の 32bit SHIORI helper 実行ファイルパスを解決する（純粋・DD 準拠）。
///
/// `std::env::current_exe()` の親ディレクトリへ `shiori-host32-helper.exe` を結合する。
/// `current_exe()` が失敗した場合（環境依存の稀な事象）は、この骨格の既存の寛容な
/// （panic しない）流儀に倣い `"."` を親ディレクトリ扱いにフォールバックする——`boot` 呼び出し
/// 自体はどのみち非致命として扱われるため、ここで panic/Err 伝播する必要はない。
pub(crate) fn default_helper_exe_path() -> std::path::PathBuf {
    let dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    dir.join("shiori-host32-helper.exe")
}

/// App スコープの sylphya profile root を解決する（task 8.2・R8.2 の `AREKA_` 冠準拠）。
///
/// - 環境変数 `AREKA_PROFILE_DIR` が設定されていればそのパスを採用する（本番 env は `AREKA_`
///   名前空間・記憶 areka-runtime-env-naming）。
/// - 未設定なら実行ファイル隣接の `profile/areka/`（`current_exe()` の親ディレクトリ／`current_exe()`
///   失敗時は `"."` へ寛容フォールバック——boot 呼び出し自体が非致命ゆえ panic/Err 伝播は不要）。
///
/// App スコープはマウント解決に現れない（ghost/shell スコープは `<shiori.dir>`／`<shell.dir>` から
/// ghost が導く）ため、bin が本関数で供給して `GhostBootOptions.app_profile_dir` へ渡す。
pub(crate) fn default_app_profile_dir() -> std::path::PathBuf {
    if let Some(dir) = std::env::var_os("AREKA_PROFILE_DIR") {
        return std::path::PathBuf::from(dir);
    }
    let base = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    base.join("profile").join("areka")
}

/// `ghost_root`／helper パスから `GhostBootOptions` を組み立てる純粋ヘルパ
/// （design.md「main の ghost boot／shutdown 結線」）。
///
/// - `shiori`: `ShioriWiring::Helper { helper_exe }`（実行ファイル隣接の 32bit helper・本番結線）。
/// - `default_encoding`: `DefaultEncoding::Ansi`（charset 未宣言時の SSP 既定・記憶
///   areka-descript-encoding-ishiori-utf8）。
/// - `sinks`: 可変長 sink 列（S-3）を `vec![LogSink, DiscardSink]` で埋める。broadcast（D4）で
///   全 cue は登録された全 sink へ配られるため、両スロットを `LogSink` にすると 1 cue が 2 回ログ
///   される（二重ログ）。記録 sink を **1 本（`LogSink`）だけ**にし、もう一方を破棄専用の
///   `DiscardSink` で埋めることで cue ごと 1 回ログへ正す（設計 D4 Topic 2）。
/// - `system_vars`: 本番 provider（`SystemVarWiring::FromSylphya`＝boot が据えた sylphya reader
///   由来のスナップショット・R7.1）。
/// - `app_profile_dir`: App スコープの sylphya profile root（`default_app_profile_dir()`＝env
///   `AREKA_PROFILE_DIR` 優先・既定は実行ファイル隣接 `profile/areka/`・R8.2）。
/// - `ticker`: `TickerMode::Real` を既定 `TickerConfig`（`base_interval=50ms`／
///   `kanade_interval=1000ms`／実クロック `GetTickCount64`）で駆動する。
///
/// `app_profile_dir` の解決は env（`AREKA_PROFILE_DIR`）・`current_exe()` を読むため厳密には純粋
/// ではない（副作用のない read のみ）。他フィールドの決定は従来どおり引数からの写しに留まる。
pub(crate) fn ghost_boot_options(
    ghost_root: std::path::PathBuf,
    helper_exe: std::path::PathBuf,
) -> areka_ghost::GhostBootOptions {
    areka_ghost::GhostBootOptions {
        ghost_root,
        default_encoding: areka_parsers::charset::DefaultEncoding::Ansi,
        shiori: areka_ghost::ShioriWiring::Helper { helper_exe },
        sinks: vec![
            Box::new(areka_ghost::sink::LogSink::new()),
            Box::new(areka_ghost::sink::DiscardSink::new()),
        ],
        system_vars: areka_ghost::SystemVarWiring::FromSylphya,
        app_profile_dir: Some(default_app_profile_dir()),
        ticker: areka_ghost::TickerMode::Real(Default::default()),
    }
}

/// `GhostBootError` を「起点不在（良性・`warn!` どまり）」と「それ以外（予期しない・`error!`）」
/// へ分類する純粋関数（design.md「main の ghost boot／shutdown 結線」・要件 8.2）。
///
/// `default_ghost_root()` はプレースホルダ subpath であり、この開発サンドボックスでは
/// 実在しないのが常態（＝`MountError::StartPointMissing` は想定内の事象）。読取不能
/// （`StartPointUnreadable`）・shell 不在（`ShellDirMissing`）・将来追加される
/// `#[non_exhaustive]` variant は、真に予期しない I/O 問題として区別する。
///
/// `pub(crate)`: `emo2_boot::wire_emo2_boot`（task 5.1）が boot 失敗（`GhostBootError`）を
/// 同一方針（起点不在＝良性 `warn!`・他＝`error!`・R7.4）で分類するため再利用する。
pub(crate) fn is_benign_boot_error(err: &areka_ghost::GhostBootError) -> bool {
    match err {
        areka_ghost::GhostBootError::Mount(
            areka_parsers::package::MountError::StartPointMissing { .. },
        ) => true,
        _ => false,
    }
}

