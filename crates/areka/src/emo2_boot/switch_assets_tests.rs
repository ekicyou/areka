//! 背景の資産づくりと荷物の置き場のテスト（spec: areka-P0-shell-balloon-switch 要件 4.4・5.5・5.6）。
//!
//! - 荷物・結果・置き場が線を越えて送れること（コンパイル時の確かめ）
//! - 背景のスレッドが COM（MTA）の上で名前の先のシェル・バルーンの資産を作って線で返すこと
//! - 読めないフォルダ・復号できない画像で失敗の型が返り、`switch_assets_failed` の `error!` が 1 件
//! - スレッドが倒れたら、受け手に「送り手が落ちた」が見えること

use std::sync::mpsc::RecvError;

use log_capture_kit::{LineFormat, capture_lines};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

use super::*;
use crate::emo2_boot::sample_test_support::emo2_balloon_root;

/// 失敗の記録の目印（`event = "switch_assets_failed"`）。
const FAILED_MARK: &str = "switch_assets_failed";

/// 送れること（design「SwitchAssets」の送れること）: 背景のスレッドから UI へ渡す結果と、
/// seriko のスレッドの橋渡しが取り出す荷物・置き場が、線を越えて送れる形であることを
/// コンパイル時に固定する（どれかが `Send` でなくなるとここでビルドが止まる）。
#[test]
fn payload_and_results_can_cross_threads() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    assert_send::<SwapBuilt>();
    assert_send::<SwitchBuildError>();
    assert_send::<Result<SwapBuilt, SwitchBuildError>>();
    assert_send::<SwapPayload>();
    assert_send::<SwapSlot>();
    assert_sync::<SwapSlot>();
}

/// シェル: 背景のスレッドが名前の先のシェルの資産・配置の値・位置の記憶を作って線で返す。
///
/// 検体は `R_POST_and_KOMAINU` に 2 つ目のシェル `second` を写したもの。写した先にだけ面 777 を
/// 置くので、名前を無視して既定のシェルを読む実装は赤になる。このテストのスレッドは COM を
/// 初期化しないので、背景のスレッドが初期化しなければ WIC のデコーダが作れず `Err` になる
/// （ただし同じプロセスの別スレッドが MTA を初期化していると暗黙の MTA で通り得る）。
#[test]
fn shell_build_runs_on_worker_and_returns_named_shell() {
    let rpost =
        sample_ghost_kit::SampleRoot::acquire("R_POST_and_KOMAINU").expect("登記済みの検体");
    rpost
        .add_shell_copy("master", "second", "second")
        .expect("2 つ目のシェルを写せるはず");
    let second_dir = rpost.folder().join("shell").join("second");
    std::fs::copy(
        second_dir.join("surface0000.png"),
        second_dir.join("surface0777.png"),
    )
    .expect("2 つ目のシェルにだけ面を足せるはず");

    let rx = spawn_switch_build(SwitchBuildRequest::Shell {
        ghost_root: rpost.folder().to_path_buf(),
        folder: "second".to_string(),
    });
    let built = rx
        .recv()
        .expect("背景のスレッドは結果を 1 件送る")
        .expect("名前の先のシェルの資産づくりは成功する");

    let SwapBuilt::Shell { assets, source, .. } = built else {
        panic!("シェルの依頼にはシェルの結果が返る");
    };
    assert_eq!(
        assets.shells.iter().map(|s| s.scope).collect::<Vec<_>>(),
        vec![0, 1],
        "scope の集合は起動の結線（derive_scopes）と同じ"
    );
    for scope_assets in &assets.shells {
        assert!(
            scope_assets.emo_world.surface(777).is_some(),
            "scope {} の World は名前の先のシェル（second）の面 777 を持つ",
            scope_assets.scope
        );
    }
    assert!(
        source.shell_dir.ends_with("second"),
        "配置の値は名前の先のシェルの descript.txt から読む: {:?}",
        source.shell_dir
    );
    assert_eq!(
        assets.author_dpi,
        source.shell_author_dpi(),
        "作者の DPI は新しいシェルの seriko.dpi（配置の値と同じ読み）"
    );
}

/// バルーン: 背景のスレッドがバルーンの資産（scope ごとの文字の模型・背景色・アニメ表）を作る。
/// アニメ表は装着の全 scope ぶん作る（4.2 の合図の `shows` がこれを前提にする）。
#[test]
fn balloon_build_runs_on_worker_and_builds_every_scope() {
    let rx = spawn_switch_build(SwitchBuildRequest::Balloon {
        dir: emo2_balloon_root(),
    });
    let built = rx
        .recv()
        .expect("背景のスレッドは結果を 1 件送る")
        .expect("emo2 のバルーンの資産づくりは成功する");

    let SwapBuilt::Balloon { assets } = built else {
        panic!("バルーンの依頼にはバルーンの結果が返る");
    };
    assert_eq!(
        assets.balloons.iter().map(|b| b.scope).collect::<Vec<_>>(),
        vec![0, 1],
        "scope の集合は起動の結線（derive_scopes）と同じ"
    );
    assert_eq!(
        assets.loop_tables.keys().copied().collect::<Vec<_>>(),
        vec![0, 1],
        "アニメ表は装着の全 scope ぶん作る"
    );
}

/// 失敗の記録（`switch_assets_failed` の `error!`）の件数を数える。
fn count_failed(lines: &[String]) -> usize {
    lines
        .iter()
        .filter(|l| l.contains("level=ERROR") && l.contains(FAILED_MARK))
        .count()
}

/// 読めないフォルダ（名前の先のシェルが無い）: 失敗の型が返り、`switch_assets_failed` が 1 件。
///
/// 捕捉はテストのスレッドだけに効くので、背景のスレッドが走らせる本体 [`build_swap`] を
/// テストのスレッドで直に呼んで数える（線で返ることは下の `spawn` のテストが見る）。
#[test]
fn missing_shell_folder_fails_with_one_error() {
    let rpost =
        sample_ghost_kit::SampleRoot::acquire("R_POST_and_KOMAINU").expect("登記済みの検体");
    let request = SwitchBuildRequest::Shell {
        ghost_root: rpost.folder().to_path_buf(),
        folder: "no-such-shell".to_string(),
    };

    let (result, lines) = capture_lines(LineFormat::LevelTargetFields, || build_swap(&request));
    assert!(
        matches!(result, Err(SwitchBuildError::Placement(_))),
        "名前の先が無いシェルは配置の値の読みで失敗する: {:?}",
        result.as_ref().err()
    );
    assert_eq!(count_failed(&lines), 1, "失敗の記録は 1 件: {lines:?}");

    let rx = spawn_switch_build(request);
    assert!(
        matches!(rx.recv(), Ok(Err(SwitchBuildError::Placement(_)))),
        "背景のスレッドからも失敗の型が線で返る"
    );
}

/// 復号できない画像（バルーンの面 0 が画像でない）: 失敗の型が返り、`switch_assets_failed` が 1 件。
#[test]
fn undecodable_balloon_image_fails_with_one_error() {
    // SAFETY: bake の WIC デコードに要る COM 初期化（既初期化の S_FALSE/RPC_E_CHANGED_MODE は無視）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    // 壊れたバルーンは検体の複製の中に置く（複製と一緒に消える・`C:\` 直下に作らない）。
    let rpost =
        sample_ghost_kit::SampleRoot::acquire("R_POST_and_KOMAINU").expect("登記済みの検体");
    let broken = rpost.folder().join("broken-balloon");
    std::fs::create_dir(&broken).expect("壊れたバルーンのフォルダを作れるはず");
    std::fs::write(broken.join("balloons0.png"), b"not a png").expect("壊れた面を書けるはず");
    std::fs::write(broken.join("balloonk0.png"), b"not a png").expect("壊れた面を書けるはず");
    let request = SwitchBuildRequest::Balloon { dir: broken };

    let (result, lines) = capture_lines(LineFormat::LevelTargetFields, || build_swap(&request));
    assert!(
        matches!(
            result,
            Err(SwitchBuildError::Assets(BootWiringError::Balloon(_)))
        ),
        "復号できない面はバルーンの資産づくりで失敗する: {:?}",
        result.as_ref().err()
    );
    assert_eq!(count_failed(&lines), 1, "失敗の記録は 1 件: {lines:?}");

    let rx = spawn_switch_build(request);
    assert!(
        matches!(
            rx.recv(),
            Ok(Err(SwitchBuildError::Assets(BootWiringError::Balloon(_))))
        ),
        "背景のスレッドからも失敗の型が線で返る"
    );
}

/// 復号できない画像（シェルの面 0 の画像が画像でない）: 起動なら読み飛ばして続けるところを、
/// 切替では差し替えの前の失敗として返し（要件 5.5・5.6）、`switch_assets_failed` が 1 件。
#[test]
fn undecodable_shell_image_fails_with_one_error() {
    // SAFETY: bake の WIC デコードに要る COM 初期化（既初期化の S_FALSE/RPC_E_CHANGED_MODE は無視）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let rpost =
        sample_ghost_kit::SampleRoot::acquire("R_POST_and_KOMAINU").expect("登記済みの検体");
    rpost
        .add_shell_copy("master", "second", "second")
        .expect("2 つ目のシェルを写せるはず");
    std::fs::write(
        rpost
            .folder()
            .join("shell")
            .join("second")
            .join("surface0000.png"),
        b"not a png",
    )
    .expect("壊れた面を書けるはず");
    let request = SwitchBuildRequest::Shell {
        ghost_root: rpost.folder().to_path_buf(),
        folder: "second".to_string(),
    };

    let (result, lines) = capture_lines(LineFormat::LevelTargetFields, || build_swap(&request));
    assert!(
        matches!(result, Err(SwitchBuildError::ShellUndecodable { .. })),
        "復号できない面を持つシェルは資産づくりで失敗する: {:?}",
        result.as_ref().err()
    );
    assert_eq!(count_failed(&lines), 1, "失敗の記録は 1 件: {lines:?}");

    let rx = spawn_switch_build(request);
    assert!(
        matches!(
            rx.recv(),
            Ok(Err(SwitchBuildError::ShellUndecodable { .. }))
        ),
        "背景のスレッドからも失敗の型が線で返る"
    );
}

/// スレッドが倒れたら（結果を送らずに送り手が落ちたら）、受け手に「送り手が落ちた」が見える。
#[test]
fn worker_panic_is_seen_as_sender_gone() {
    let rx = spawn_build_worker(|| panic!("資産づくりのスレッドが倒れた（テストの注入）"));
    assert_eq!(
        rx.recv().err(),
        Some(RecvError),
        "倒れたスレッドは結果を送らず、受け手は送り手が落ちたことを見る"
    );
}
