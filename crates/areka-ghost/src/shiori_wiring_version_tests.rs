//! 読み込み直後の版の確かめ（`record_version`／`ghost_label`／`real_connect` の結線）の
//! 決定論テスト（タスク 4.2・要件 2.1／2.3〜2.6／7.3〜7.5）。
//!
//! 窓も 32bit 成果物も実 SHIORI も要さない。記録は `capture_events` の観測で固定する
//! （捕捉窓は番兵で生存を確かめるので「1 件」の主張は恒真にならない）。

use std::path::Path;

use log_capture_kit::CapturedEvent;
use shiori_host32_host::{ShioriVersion, VersionVerdict};
use tracing::Level;

use super::{ghost_label, helper_load_dir, record_version};
use crate::test_log_capture::capture_events;

/// 起動ログの宛先（`shiori_wiring` の `LOG_TARGET` と同じ綴り・design §Monitoring）。
const BOOT_TARGET: &str = "ghost-boot";

/// 宛先 `ghost-boot` の版の記録（決めた INFO／決められない WARN）を全部取り出す。
fn version_events(events: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    events
        .iter()
        .filter(|e| {
            e.target == BOOT_TARGET
                && matches!(
                    e.field_str("event"),
                    Some("shiori_version_decided" | "shiori_version_undecided")
                )
        })
        .collect()
}

/// 決めたら INFO を 1 行（ゴースト・版・根拠）出し、その版を返す（要件 2.4）。
#[test]
fn decided_records_one_info_line_and_returns_the_version() {
    for (version, evidence) in [
        (ShioriVersion::Shiori2, "SHIORI/2.6 200 OK"),
        (ShioriVersion::Shiori3, "SHIORI/3.0 400 Bad Request"),
    ] {
        let mut got = None;
        let events = capture_events(|| {
            got = Some(record_version(
                "dot_sakura",
                VersionVerdict::Decided {
                    version,
                    evidence: evidence.to_string(),
                },
            ));
        });

        assert_eq!(got, Some(version), "決めた版をそのまま返す");
        let hits = version_events(&events);
        assert_eq!(hits.len(), 1, "記録はちょうど 1 行（要件 2.4）");
        let hit = hits[0];
        assert_eq!(hit.level, Level::INFO);
        assert_eq!(hit.field_str("event"), Some("shiori_version_decided"));
        assert_eq!(hit.field_str("ghost"), Some("dot_sakura"));
        assert_eq!(hit.field_str("version"), Some(version.as_str()));
        assert_eq!(hit.field_str("evidence"), Some(evidence));
    }
}

/// 決められなければ WARN を 1 行（理由・`assumed = "SHIORI/3.0"`）出し、3.0 を返す
/// （要件 2.5・7.3）。
#[test]
fn undecided_records_one_warn_line_and_assumes_shiori3() {
    let mut got = None;
    let events = capture_events(|| {
        got = Some(record_version(
            "dot_sakura",
            VersionVerdict::Undecided {
                reason: "empty_status_line".to_string(),
            },
        ));
    });

    assert_eq!(got, Some(ShioriVersion::Shiori3), "決められなければ 3.0");
    let hits = version_events(&events);
    assert_eq!(hits.len(), 1, "記録はちょうど 1 行（要件 2.5）");
    let hit = hits[0];
    assert_eq!(hit.level, Level::WARN);
    assert_eq!(hit.field_str("event"), Some("shiori_version_undecided"));
    assert_eq!(hit.field_str("ghost"), Some("dot_sakura"));
    assert_eq!(hit.field_str("reason"), Some("empty_status_line"));
    assert_eq!(hit.field_str("assumed"), Some("SHIORI/3.0"));
}

/// ゴーストの名前は `ghost\master` の 2 つ上のフォルダ名。
#[test]
fn ghost_label_takes_the_folder_two_levels_above_master() {
    assert_eq!(
        ghost_label(Path::new(r"C:\ghosts\dot_sakura\ghost\master")),
        "dot_sakura"
    );
    // 2 つ上が無ければフォルダの表示へ後退する。
    assert_eq!(ghost_label(Path::new("master")), "master");
}

/// helper の `load` へ渡すパスは絶対パス・`\` の区切り・末尾に `\` が 1 つ（要件 6.4・
/// `shiori4-api` の決定 6）。華和梨はこれにファイル名をそのままつなげて辞書を開く。
#[test]
fn helper_load_dir_is_absolute_backslashed_with_one_trailing_separator() {
    for input in [
        r"C:\x\ghost/master",
        r"C:\x\ghost\master\",
        r"C:\x\ghost/master/",
    ] {
        let got = helper_load_dir(Path::new(input)).expect("絶対パスにできる");
        assert_eq!(
            got.as_os_str(),
            r"C:\x\ghost\master\",
            "入力 {input:?} は決定 6 の形になる"
        );
    }
}

/// 整えたパスは helper にだけ渡し、記録のゴーストの名前は元のパスから取る（要件 6.4）。
#[test]
fn real_connect_spawns_with_the_shaped_dir_and_labels_from_the_original() {
    const SOURCE: &str = include_str!("shiori_wiring.rs");
    let body = &SOURCE[SOURCE
        .find("pub fn real_connect(")
        .expect("real_connect がある")..];
    let shaped = body
        .find("helper_load_dir(&load_dir)")
        .expect("元のパスを整える");
    let spawned = body
        .find("spawn(&helper_exe, &helper_dir,")
        .expect("整えたパスで helper を起こす");
    assert!(shaped < spawned, "整えてから起こす");
    assert!(
        body.contains("ghost_label(&load_dir)"),
        "ゴーストの名前は元のパスのまま"
    );
}

/// 確かめの往復の結線（要件 2.1・2.3・7.4）。
///
/// 接続の値は中継が無いと組めないので（先例
/// `the_connection_receives_the_already_decided_negotiator` と同じ理由）、本番ファイルの
/// 本文で固定する: 確かめの呼び出しは 1 回だけ、`real_connect` の接続ごとの閉包の中の
/// LOAD の後かつ接続を組む前にある。閉包の外（`real_connect` を呼んだ時点）で確かめたり
/// 記録したりすると、読み直しで確かめ直さなくなるので赤になる。
#[test]
fn the_probe_runs_once_inside_the_closure_after_load_before_the_connection() {
    const SOURCE: &str = include_str!("shiori_wiring.rs");
    const PROBE: &str = "probe_version(";

    assert_eq!(
        SOURCE.matches(PROBE).count(),
        1,
        "確かめの呼び出しは 1 回だけ"
    );

    let fn_start = SOURCE
        .find("pub fn real_connect(")
        .expect("real_connect がある");
    let closure = fn_start
        + SOURCE[fn_start..]
            .find("move ||")
            .expect("real_connect は閉包を返す");
    let fn_end = closure
        + SOURCE[closure..]
            // 行頭の `}`＝関数の終わり（改行コードに依らない）。
            .find("\n}")
            .expect("real_connect の終わり");
    let before_closure = &SOURCE[fn_start..closure];
    let body = &SOURCE[closure..fn_end];

    assert!(
        !before_closure.contains(PROBE) && !before_closure.contains("record_version("),
        "版を閉包の外で決めない（接続ごとに確かめ直す・要件 2.3）"
    );
    let load = body.find("MsgTag::Load").expect("閉包の中に LOAD がある");
    let probe = body.find(PROBE).expect("閉包の中に確かめがある");
    let record = body.find("record_version(").expect("閉包の中で記録する");
    let connection = body
        .find("ShioriConnection {")
        .expect("閉包の中で接続を組む");
    assert!(load < probe, "確かめは LOAD の後");
    assert!(
        probe < record && record < connection,
        "記録して版を接続へ渡す"
    );
}
