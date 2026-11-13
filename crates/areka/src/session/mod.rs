//! 本体に頼る切替・読み直し・起動・終わる相の層（runtime-module-untangle）。モジュールの宣言だけ。

/// 本体の口の束の中身と据え付け（`runtime::session_hooks::SessionHooks`）。
pub(crate) mod hooks;

/// 終わる相（停止通知の受け口・`ghost_quit_system`・`wire_kanade_stop`）。
pub(crate) mod ghost_quit;

/// ゴーストの切替の入口（areka-P0-ghost-shell-balloon-switch）。
pub(crate) mod ghost_switch;

/// 読み直しの窓口（areka-P0-mcp-reload）。
pub(crate) mod reload;

/// 実行系の側の起動（受け口の送出端の束・areka-P0-mcp-reload）。
pub(crate) mod runtime_boot;

/// 同じフォルダ名のゴーストを今のゴーストとして扱う（areka-P0-baseware-root-list）。
pub(crate) mod same_folder_pin;

/// シェル・バルーンの切替の入口（areka-P0-shell-balloon-switch）。
pub(crate) mod shell_balloon_switch;

/// ゴーストの切替で同梱のゴーストを書ける根へ背景で写す（areka-P0-user-data-root）。
pub(crate) mod switch_copy;

// 偽の SHIORI を持つゴーストを同じ World で起こす統合テストの土台（areka-P0-ghost-shell-balloon-switch）。
#[cfg(test)]
pub(crate) mod ghost_switch_test_support;
