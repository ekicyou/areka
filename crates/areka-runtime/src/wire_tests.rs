use super::*;
use crate::sample_test_support::{emo2_balloon_root, emo2_root};
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
