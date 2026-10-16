//! helper の見出しの判定と記録の兄弟テスト（dev-helper-x64-clobber タスク 2.1・要件 3.1/3.2/3.4/3.5）。
//!
//! 判定（[`classify_helper_image`]）は合成したバイト列で、記録（[`report_helper_image`]）は
//! 結果を直接渡して確かめる。ファイルを作らない。ログは `log_capture_kit::capture` で観測する
//! （呼出スレッドで同期発火するため捕捉できる・捕捉窓は番兵で自己較正される）。

use std::path::Path;

use log_capture_kit::CapturedEvent;

use super::{
    HelperImage, SpawnError, check_helper_image, classify_helper_image, report_helper_image, spawn,
};

/// `MZ`・`e_lfanew=0x80`・`PE\0\0`・機械の種類 `machine` だけを持つ合成の見出し。
fn pe_head(machine: u16) -> Vec<u8> {
    let mut head = vec![0u8; 0x200];
    head[0..2].copy_from_slice(b"MZ");
    head[0x3C..0x40].copy_from_slice(&0x80u32.to_le_bytes());
    head[0x80..0x84].copy_from_slice(b"PE\0\0");
    head[0x84..0x86].copy_from_slice(&machine.to_le_bytes());
    head
}

/// helper の判定の記録（`target: "shiori-host32"`）だけを取り出す。
fn host32_events(events: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    events
        .iter()
        .filter(|e| e.target == "shiori-host32")
        .collect()
}

const HELPER: &str = r"C:\areka\target\debug\shiori-host32-helper.exe";

#[test]
fn classify_i386_machine_is_i686() {
    assert_eq!(classify_helper_image(&pe_head(0x014C)), HelperImage::I686);
}

#[test]
fn classify_x64_machine_is_other_machine() {
    assert_eq!(
        classify_helper_image(&pe_head(0x8664)),
        HelperImage::OtherMachine(0x8664)
    );
}

#[test]
fn classify_arm64_machine_is_other_machine() {
    assert_eq!(
        classify_helper_image(&pe_head(0xAA64)),
        HelperImage::OtherMachine(0xAA64)
    );
}

#[test]
fn classify_without_mz_is_not_pe() {
    let mut head = pe_head(0x014C);
    head[0..2].copy_from_slice(b"ZM");
    assert_eq!(classify_helper_image(&head), HelperImage::NotPe);
}

#[test]
fn classify_without_pe_signature_is_not_pe() {
    let mut head = pe_head(0x014C);
    head[0x80..0x84].copy_from_slice(b"PE\0\x01");
    assert_eq!(classify_helper_image(&head), HelperImage::NotPe);
}

/// `e_lfanew` が読んだ範囲の外（署名も機械の種類も読めない位置）なら `NotPe`。
/// 署名は読めても機械の種類の 2 バイトが欠ける位置も同じく `NotPe`。
#[test]
fn classify_with_e_lfanew_out_of_range_is_not_pe() {
    let mut head = pe_head(0x014C);
    head[0x3C..0x40].copy_from_slice(&0x1000u32.to_le_bytes());
    assert_eq!(classify_helper_image(&head), HelperImage::NotPe);

    let mut head = pe_head(0x014C);
    head.truncate(0x85);
    assert_eq!(classify_helper_image(&head), HelperImage::NotPe);

    let mut head = pe_head(0x014C);
    head[0x3C..0x40].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(classify_helper_image(&head), HelperImage::NotPe);
}

#[test]
fn classify_empty_is_not_pe() {
    assert_eq!(classify_helper_image(&[]), HelperImage::NotPe);
}

/// 要件 3.4: i686 のときは記録 0 行。
#[test]
fn report_i686_logs_nothing() {
    let ((), events) =
        log_capture_kit::capture(|| report_helper_image(Path::new(HELPER), Ok(HelperImage::I686)));
    assert_eq!(host32_events(&events).len(), 0, "{events:?}");
}

/// 要件 3.1・3.2: ほかの機械の種類はエラーの 1 行（パス・機械の種類の 16 進・直し方）。
#[test]
fn report_other_machine_logs_one_error_with_path_machine_and_fix() {
    let ((), events) = log_capture_kit::capture(|| {
        report_helper_image(Path::new(HELPER), Ok(HelperImage::OtherMachine(0x8664)))
    });
    let logged = host32_events(&events);
    assert_eq!(logged.len(), 1, "{events:?}");
    let e = logged[0];
    assert_eq!(e.level, tracing::Level::ERROR);
    assert_eq!(e.field_str("event"), Some("helper_not_i686"));
    assert_eq!(e.field("path"), Some(HELPER));
    assert_eq!(e.field("machine"), Some("0x8664"));
    let message = e.message();
    assert!(message.contains("i686"), "{message}");
    assert!(
        message.contains("shiori-host32-helper.exe"),
        "直し方が無い: {message}"
    );
}

/// 要件 3.5: PE でないときは警告の 1 行（パス・理由）。
#[test]
fn report_not_pe_logs_one_warning_with_path_and_reason() {
    let ((), events) =
        log_capture_kit::capture(|| report_helper_image(Path::new(HELPER), Ok(HelperImage::NotPe)));
    let logged = host32_events(&events);
    assert_eq!(logged.len(), 1, "{events:?}");
    let e = logged[0];
    assert_eq!(e.level, tracing::Level::WARN);
    assert_eq!(e.field_str("event"), Some("helper_image_unverified"));
    assert_eq!(e.field("path"), Some(HELPER));
    assert_eq!(e.field_str("reason"), Some("not a PE image"));
}

/// 要件 3.5: 読めないときは警告の 1 行（パス・誤り）。
#[test]
fn report_unreadable_logs_one_warning_with_path_and_error() {
    let ((), events) = log_capture_kit::capture(|| {
        report_helper_image(
            Path::new(HELPER),
            Err(std::io::Error::from(std::io::ErrorKind::NotFound)),
        )
    });
    let logged = host32_events(&events);
    assert_eq!(logged.len(), 1, "{events:?}");
    let e = logged[0];
    assert_eq!(e.level, tracing::Level::WARN);
    assert_eq!(e.field_str("event"), Some("helper_image_unverified"));
    assert_eq!(e.field("path"), Some(HELPER));
    assert!(e.field("error").is_some(), "{e:?}");
}

/// 要件 3.1: 実ファイル（テストの実行体そのもの＝x64 か arm64）を読むとエラーの 1 行。
#[test]
fn check_real_non_i686_exe_logs_one_error() {
    let exe = std::env::current_exe().expect("テストの実行体のパス");
    let ((), events) = log_capture_kit::capture(|| check_helper_image(&exe));
    let logged = host32_events(&events);
    assert_eq!(logged.len(), 1, "{events:?}");
    assert_eq!(logged[0].level, tracing::Level::ERROR);
    assert_eq!(logged[0].field_str("event"), Some("helper_not_i686"));
}

/// 要件 3.5: 無いパスを読むと警告の 1 行。
#[test]
fn check_missing_path_logs_one_warning() {
    let missing = Path::new("this_helper_exe_does_not_exist_areka_host32.exe");
    let ((), events) = log_capture_kit::capture(|| check_helper_image(missing));
    let logged = host32_events(&events);
    assert_eq!(logged.len(), 1, "{events:?}");
    assert_eq!(logged[0].level, tracing::Level::WARN);
    assert_eq!(
        logged[0].field_str("event"),
        Some("helper_image_unverified")
    );
}

/// 要件 3.1・3.3・3.5: `spawn` は起こす前に判定を 1 回呼び、起こし方（無い helper は
/// `SpawnError::Spawn`）は変えない。
#[test]
fn spawn_checks_helper_image_once_and_keeps_spawn_result() {
    let missing = Path::new("this_helper_exe_does_not_exist_areka_host32.exe");
    let load_dir = std::env::temp_dir();
    let (result, events) = log_capture_kit::capture(|| spawn(missing, &load_dir, "shiori.dll", 0));
    assert!(matches!(result, Err(SpawnError::Spawn(_))));
    let logged = host32_events(&events);
    assert_eq!(logged.len(), 1, "{events:?}");
    assert_eq!(
        logged[0].field_str("event"),
        Some("helper_image_unverified")
    );
}
