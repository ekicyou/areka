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
        &crate::placement::DefaultSurfaces::builtin(),
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

/// 面が 1 つも無いシェルの失敗は、場所（シェルのフォルダ）をそのまま運び、利用者に出る文言が
/// 「シェルに面が 1 つも無い」になる（spec: areka-P0-self-alpha-declaration 要件 6.5・
/// `surfaces.txt` が無い場合があるので、そのファイルの話にしない）。
#[test]
fn shell_without_any_surface_maps_to_shell_empty_with_folder_wording() {
    use areka_emo_present::shell_target::ShellLoadError;

    let folder = std::path::PathBuf::from(r"ghost\shell\master");
    let error = BootWiringError::from(ShellLoadError::Empty {
        path: folder.clone(),
    });
    let text = error.to_string();
    assert!(
        matches!(&error, BootWiringError::ShellEmpty { path } if *path == folder),
        "Empty は ShellEmpty へ写り、場所はシェルのフォルダのまま: {error:?}"
    );
    assert!(
        text.contains("シェルに面が 1 つも無い") && text.contains(r"ghost\shell\master"),
        "文言は面が 1 つも無いことを言い、フォルダを示す: {text}"
    );
    assert!(
        !text.contains("surfaces.txt"),
        "surfaces.txt が無い場合もあるので、そのファイルの話にしない: {text}"
    );
}

/// descript-window-keys 要件 1.7・3.1: シェルの資産の各スコープの最初の面は、渡した descript の
/// 既定の面になる（本体側・相方側・スコープ 2 は書いた値、書いていないスコープ 3 は組み込みの 10）。
/// 面がシェルに在るかは問わない（在るかの判断は測り・装着の相が持つ）。
#[test]
fn shell_assets_take_initial_surfaces_from_descript_defaults() {
    use crate::placement::descript_window_keys::read_default_surfaces;
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
