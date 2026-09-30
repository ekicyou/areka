//! シェル名つきの資産づくり（`build_boot_assets_with_shell`）のテスト
//! （spec: areka-P0-shell-balloon-switch 要件 4.4・6.4・8.8）。
//!
//! `assets_tests.rs` は上限の近いファイルゆえ行を足さず、兄弟のここへ置く。

use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

use super::*;
use crate::emo2_boot::sample_test_support::emo2_balloon_root;

/// 2 つ目のシェルにだけ置く面の番号（元のシェル `master` には無い）。
const ONLY_IN_SECOND: u32 = 777;

/// 名前を渡すと、その名のシェルの絵から資産を作る。名前なし（`build_boot_assets`）は
/// 既定のシェル（`master`）のままである。
///
/// 検体は `R_POST_and_KOMAINU`（面が `surfaceNNNN.png` の画像だけで定義される）。
/// `master` を `second` へ写し、写した先にだけ `surface0777.png` を置く。名前を無視して
/// 既定のシェルを読む実装は、面 777 を持たないので赤になる。
#[test]
fn named_shell_builds_assets_from_that_shells_pictures() {
    // SAFETY: bake の WIC デコードに要る COM 初期化（既初期化の S_FALSE/RPC_E_CHANGED_MODE は無視）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let rpost =
        sample_ghost_kit::SampleRoot::acquire("R_POST_and_KOMAINU").expect("登記済みの検体");
    rpost
        .add_shell_copy("master", "second", "second")
        .expect("2 つ目のシェルを写せるはず");
    let shell_root = rpost.folder().join("shell");
    std::fs::copy(
        shell_root.join("master").join("surface0000.png"),
        shell_root
            .join("second")
            .join(format!("surface{ONLY_IN_SECOND:04}.png")),
    )
    .expect("2 つ目のシェルにだけ面を足せるはず");

    // 較正: 名前なしは既定のシェル（master）を読み、面 777 を持たない。
    let default = build_boot_assets(rpost.folder(), &emo2_balloon_root(), &[0, 1], 96, 96)
        .expect("既定のシェルの資産づくりは成功する");
    assert!(
        default.shells[0].emo_world.surface(0).is_some(),
        "較正: 既定のシェルの World は面 0 を持つ"
    );
    assert!(
        default.shells[0]
            .emo_world
            .surface(ONLY_IN_SECOND)
            .is_none(),
        "較正: 既定のシェル（master）に面 777 は無い"
    );

    let named = build_boot_assets_with_shell(
        rpost.folder(),
        &emo2_balloon_root(),
        &[0, 1],
        96,
        96,
        Some("second"),
    )
    .expect("名前の先のシェルの資産づくりは成功する");
    assert_eq!(named.shells.len(), 2, "要求 scope 集合 [0,1] に 1:1 対応");
    for scope_assets in &named.shells {
        assert!(
            scope_assets.emo_world.surface(ONLY_IN_SECOND).is_some(),
            "scope {} の World は名前の先のシェル（second）の面 777 を持つ",
            scope_assets.scope
        );
    }
    assert!(
        named
            .loop_tables
            .balloon
            .keys()
            .copied()
            .eq(named.balloons.iter().map(|b| b.scope)),
        "バルーンの側は名前に関わらず scope ごとに作られる"
    );
}
