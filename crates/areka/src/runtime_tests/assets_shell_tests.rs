//! シェル名つきの資産づくり（`build_boot_assets_with_shell`）のテスト
//! （spec: areka-P0-shell-balloon-switch 要件 4.4・6.4・8.8）。
//!
//! `assets_tests.rs` は上限の近いファイルゆえ行を足さず、兄弟のここへ置く。

use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

use super::*;

/// descript-window-keys 要件 1.7・3.1: シェルの資産の各スコープの最初の面は、渡した descript の
/// 既定の面になる（本体側・相方側・スコープ 2 は書いた値、書いていないスコープ 3 は組み込みの 10）。
/// 面がシェルに在るかは問わない（在るかの判断は測り・装着の相が持つ）。
#[test]
fn shell_assets_take_initial_surfaces_from_descript_defaults() {
    use areka_placement::descript_window_keys::read_default_surfaces;
    // SAFETY: bake の WIC デコードに要る COM 初期化（既初期化の S_FALSE/RPC_E_CHANGED_MODE は無視）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let ghost = crate::scope_bind_fixture_test_support::scope_bind_ghost();
    let decoder = WicDecoderArm::new().expect("WIC デコーダ");
    let ghost_kv = [
        ("sakura.seriko.defaultsurface", "3"),
        ("kero.seriko.defaultsurface", "12"),
        ("char2.seriko.defaultsurface", "7"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();
    let defaults = read_default_surfaces(&ghost_kv);
    let shell = match build_shell_assets(ghost.path(), None, &[0, 1, 2, 3], 96, &defaults, &decoder)
    {
        Ok(shell) => shell,
        Err(e) => panic!("シェルの資産づくりは成功する: {e:?}"),
    };
    let initial: Vec<(u32, u32)> = shell
        .shells
        .iter()
        .map(|s| (s.scope, s.initial_surface_id))
        .collect();
    assert_eq!(initial, vec![(0, 3), (1, 12), (2, 7), (3, 10)]);
}
