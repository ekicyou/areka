//! 本体に頼る切替・読み直し・起動・終わる相の層。モジュールの宣言だけ。

/// 本体の口の束の中身と据え付け（`areka_runtime::session_hooks::SessionHooks`）。
pub mod hooks;

/// 終わる相（停止通知の受け口・`ghost_quit_system`・`wire_kanade_stop`）。
pub mod ghost_quit;

/// ゴーストの切替の入口。
pub mod ghost_switch;

/// 読み直しの窓口。
pub mod reload;

/// 実行系の側の起動（受け口の送出端の束）。
pub mod runtime_boot;

/// 同じフォルダ名のゴーストを今のゴーストとして扱う。
pub(crate) mod same_folder_pin;

/// シェル・バルーンの切替の入口。
pub mod shell_balloon_switch;

/// ゴーストの切替で同梱のゴーストを書ける根へ背景で写す。
pub mod switch_copy;

/// 告知（起動を止める 4 場面・SHIORI が止まった・記憶を書けない）と、尋ねる口・抑止（`AREKA_NO_ALERT`）。
pub mod alert;

/// 記憶を書けない失敗を集める致命の机。
pub mod write_failure;

/// 終了の後始末が、背景で書いている仕事の終わりを上限つきで待つ口。
pub mod exit_wait;

/// 1 回の走りが読み取りだけかを 1 度だけ立てて配る口。
pub mod readonly;

/// ログの履歴。
pub mod log_history;

/// ゴーストの説明書。
pub mod readme;

/// ベースウェアの根の並び。
pub mod roots;

/// 起動の求め（`BootRequest`）と SHIORI の読み方の指定（`ModulePolicy`）。
pub mod boot_request;

/// 起動時の構成入力解決と ghost 結線ヘルパ。
pub mod boot_config;

/// 起動解決の純粋な判断（ゴースト 9 通り・バルーン 8 分岐）と既定の定数。
/// 起動前の解決は `install::startup::resolve_boot` が、記憶の書き込みは boot 成功直後の `on_boot_ok` が結線する。
pub mod boot_resolve;

/// 終了の統合操作。全窓を閉じてから終了を指示する `quit_app` と出所の語彙 `ExitOrigin` を持つ。
pub mod app_exit;

/// 自動 close の仕掛け（`AREKA_APP_SMOKE_EXIT_MS`・CI smoke）。
pub mod smoke_exit;

/// OS のセッションの終了（`WM_ENDSESSION`）を窓の手続きの中できれいな終わりにする受け手。
pub mod session_end;

/// 後始末と OS のセッションの終了が共有する起動中の印の判定。
pub mod session_mark;

/// 起こし直しの単位。系の登録の入口 `register_systems` を持つ。
pub mod ghost_session;

/// 本体だけのモジュールの系の登録と、本体へ向かう口の束。
pub mod body_systems;

/// 起動窓と起動の直後の結線のシーム。
pub mod boot_seam;

// 偽の SHIORI を持つゴーストを同じ World で起こす統合テストの土台。
#[cfg(any(test, feature = "test-support"))]
pub mod ghost_switch_test_support;

/// 別の crate のファイルを読む読み口（`crate_source!`・`crate_path`）。
#[cfg(any(test, feature = "test-support"))]
pub mod crate_source_test_support;

#[cfg(test)]
mod crash_report_tests;
