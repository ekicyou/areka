use crate::sample_test_support::{emo2_balloon_root, emo2_root};
use super::*;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

/// 選びの種は抽選の種と別に引く（spec: seriko-pattern-calls 要件 3.6）: 続けて引いた 2 つの種が
/// 違う（同じ種だと選びと抽選が同じ列を引き、別にした意味が無い）。
#[test]
fn lottery_and_pick_seeds_are_drawn_separately() {
    assert_ne!(entropy_seed(), entropy_seed());
}

/// 取り違え防止の檻（task 4.3・要件 1.1）: `build_boot_assets` への隣接 `u16` 2 引数
/// （`shell_author_dpi`／`balloon_author_dpi`）が [`AuthorDpi`] の各フィールドと
/// **正しく対応**していることを、意図的に**異なる 2 値**で固定する。
///
/// 入れ替えは型検査を通り抜けるため、この檻だけが誤りを検出できる
/// （192 と 144 を入れ替えると両アサーションが落ちる）。
/// `wire_runtime` 本体は生の `u16` を触らず、この関数だけが位置引数へ落とす。
#[test]
fn build_boot_assets_for_pairs_shell_and_balloon_author_dpi() {
    // SAFETY: bake の WIC デコードに要る COM 初期化（既初期化の S_FALSE 等は無視・assets.rs 同流儀）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }

    let assets = build_boot_assets_for(
        &emo2_root(),
        &emo2_balloon_root(),
        &[0, 1],
        AuthorDpi {
            shell: 192,
            balloon: 144,
        },
        None,
        &DefaultSurfaces::builtin(),
    )
    .expect("emo2 fixture の BootAssets 組立は成功する");

    assert_eq!(
        assets.shell_author_dpi, 192,
        "shell 引数には AuthorDpi::shell が渡る（balloon の 144 ではない）"
    );
    assert_eq!(
        assets.balloon_author_dpi, 144,
        "balloon 引数には AuthorDpi::balloon が渡る（shell の 192 ではない）"
    );
}

/// 観測可能な完了条件（tasks.md task 5.1）: 存在しない ghost_root に対し
/// `wire_runtime` は `wired=false`（帰結は呼び手が決める・R7.3）を返し、
/// ghost／seriko ハンドルは `None` となる（実 sink 結線を成立させない決定分岐の檻）。
///
/// 素の `World::new()` で足りる。`build_boot_assets` が `resolve` 起点不在で早期 `Err` を返すため、
/// World には一切触れず、spawn_emo_text／spawn_seriko／boot へも到達しない（＝アクタースレッドを
/// 起こさない）ので決定論的に判定できる。
#[test]
fn wire_runtime_falls_back_to_unwired_on_missing_ghost_root() {
    let mut world = World::new();
    let inputs = RuntimeInputs {
        ghost_root: PathBuf::from("C:/areka-nonexistent-ghost-root-runtime-5v1"),
        balloon_root: PathBuf::from("C:/areka-nonexistent-balloon-root-runtime-5v1"),
        shiori: ShioriWiring::Helper {
            helper_exe: PathBuf::from("shiori-host32-helper.exe"),
        },
        ticker: TickerMode::Disabled,
        app_profile_dir: None,
        boot_origin: BootOrigin::Plain,
    };

    let outcome = wire_runtime(
        &mut world,
        inputs,
        AuthorDpi::DEFAULT,
        None,
        std::sync::mpsc::channel().0,
        None,
        &DefaultSurfaces::builtin(),
        crate::session::runtime_boot::boot_runtime,
    );

    assert!(
        !outcome.wired,
        "存在しない ghost_root では実 sink 結線は成立せず wired=false を返す（R7.3）"
    );
    assert!(
        outcome.ghost.is_none(),
        "不成立時は ghost ランタイムを起動しない（None）"
    );
    assert!(
        outcome.seriko.is_none(),
        "不成立時は seriko アクターを起こさない（None）"
    );
}
