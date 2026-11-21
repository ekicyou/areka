//! 本体に頼る切替・読み直し・起動・終わる相の層。モジュールの宣言だけ。

/// 本体の口の束の中身と据え付け（`areka_runtime::session_hooks::SessionHooks`）。
pub(crate) mod hooks;

/// 終わる相（停止通知の受け口・`ghost_quit_system`・`wire_kanade_stop`）。
pub(crate) mod ghost_quit;

/// ゴーストの切替の入口。
pub(crate) mod ghost_switch;

/// 読み直しの窓口。
pub(crate) mod reload;

/// 実行系の側の起動（受け口の送出端の束）。
pub(crate) mod runtime_boot;

/// 同じフォルダ名のゴーストを今のゴーストとして扱う。
pub(crate) mod same_folder_pin;

/// シェル・バルーンの切替の入口。
pub(crate) mod shell_balloon_switch;

/// ゴーストの切替で同梱のゴーストを書ける根へ背景で写す。
pub(crate) mod switch_copy;

/// 無いときの告知（根なし／ゴーストなし／バルーンなし／起動窓を開けない）と
/// 告知の抑止（`AREKA_NO_ALERT`）。
pub(crate) mod alert;

/// 記憶を書けない失敗を集める致命の机。
pub(crate) mod write_failure;

/// 終了の後始末が、背景で書いている仕事の終わりを上限つきで待つ口。
pub(crate) mod exit_wait;

/// 1 回の走りが読み取りだけかを 1 度だけ立てて配る口。
pub(crate) mod readonly;

/// ログの履歴。
pub(crate) mod log_history;

/// ゴーストの説明書。
pub(crate) mod readme;

/// ベースウェアの根の並び。
pub(crate) mod roots;

// 偽の SHIORI を持つゴーストを同じ World で起こす統合テストの土台。
#[cfg(test)]
pub(crate) mod ghost_switch_test_support;
