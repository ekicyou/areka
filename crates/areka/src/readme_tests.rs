//! 説明書のファイルの決め方と在否の決定論テスト（要件 9.4）。
//!
//! 確かめること: `readme` キーがあるときと無いときのパス・ファイルを置く／置かないで
//! 在否が変わること・「無い」の記録が初回だけであること・要求の取り出しが溜まった件数を
//! 残さず返すこと・持ち物が無いときに黙って済ませないこと。
//!
//! 既定のアプリで開く関数（`open`）は OS を触るので決定論テストに入れず、実機確認
//! （要件 9.9 ⑵）へ回す。

use std::sync::mpsc::{self, Sender};

use log_capture_kit::{LineFormat, capture_lines};
use temp_path_kit::TempPath;

use super::*;

/// 説明書の持ち物を World へ直に入れる（結線 `wire_readme` はスケジュール登録を伴うので、
/// 判断分岐だけを見るここでは通さない）。送出端は要求を送るために返す。
fn wired(world: &mut World, path: PathBuf) -> Sender<ReadmeRequest> {
    let (tx, rx) = mpsc::channel::<ReadmeRequest>();
    world.insert_non_send(ReadmeWiring {
        path,
        rx,
        missing_logged: Cell::new(false),
    });
    tx
}

/// クロージャ実行中にこのスレッドで発火した記録を 1 行 1 件で返す。
fn capture<F: FnOnce()>(f: F) -> Vec<String> {
    let ((), lines) = capture_lines(LineFormat::LevelFields, f);
    lines
}

/// 指定した出来事の行だけを拾う。
fn lines_of<'a>(lines: &'a [String], event: &str) -> Vec<&'a String> {
    lines.iter().filter(|l| l.contains(event)).collect()
}

// -------------------------------------------------------------------------
// resolve_path（要件 4.1）
// -------------------------------------------------------------------------

/// `readme` キーがあれば、その値をゴーストのフォルダの根の直下で解決する（要件 4.1 ⑴）。
#[test]
fn resolve_path_uses_the_readme_key_when_present() {
    let root = Path::new("C:\\ghosts\\sample");

    assert_eq!(
        resolve_path(root, Some("manual.txt")),
        root.join("manual.txt"),
        "キーの値をそのまま根の直下で解決する（要件 4.1 ⑴）"
    );
}

/// キーが無ければ正典の既定名 `readme.txt` を使う（要件 4.1 ⑵）。
#[test]
fn resolve_path_falls_back_to_the_canonical_default_name() {
    let root = Path::new("C:\\ghosts\\sample");

    assert_eq!(DEFAULT_README, "readme.txt", "正典の既定名（要件 4.1 ⑵）");
    assert_eq!(
        resolve_path(root, None),
        root.join("readme.txt"),
        "キーが無いときは既定名で解決する（要件 4.1 ⑵）"
    );
}

// -------------------------------------------------------------------------
// is_available（要件 4.3・11.4）
// -------------------------------------------------------------------------

/// 持ち物が無ければ「無い」扱い（結線前でも判断を止めない）。
#[test]
fn is_available_is_false_without_wiring() {
    let world = World::new();

    assert!(
        !is_available(&world),
        "ReadmeWiring 不在は無効（灰色）で扱う"
    );
}

/// ファイルが無ければ無効で、記録は初回だけ。置けば有効に変わる（要件 4.3・11.4）。
#[test]
fn is_available_follows_the_file_and_records_the_absence_only_once() {
    let dir = TempPath::new("readme-availability");
    let path = dir.child("readme.txt");
    let mut world = World::new();
    let _tx = wired(&mut world, path.clone());

    let missing = capture(|| {
        for _ in 0..3 {
            assert!(!is_available(&world), "ファイルが無い間は無効（要件 4.3）");
        }
    });
    assert_eq!(
        lines_of(&missing, "readme_missing").len(),
        1,
        "「無い」の記録は初回だけ（3 回見ても 1 行・要件 4.3）: {missing:?}"
    );
    assert!(
        missing
            .iter()
            .any(|l| l.contains("readme_missing") && l.contains("level=DEBUG")),
        "「無い」は debug! で記録する（要件 4.3）: {missing:?}"
    );

    std::fs::write(&path, "説明書").expect("一時ディレクトリへ書けるはず");

    let after = capture(|| {
        assert!(
            is_available(&world),
            "ファイルを置けば有効に変わる（要件 4.3）"
        );
    });
    assert!(
        lines_of(&after, "readme_missing").is_empty(),
        "有効になった後に「無い」の記録は出ない: {after:?}"
    );
}

// -------------------------------------------------------------------------
// 要求の取り出し（要件 4.5 の受け口）
// -------------------------------------------------------------------------

/// 持ち物が無ければ取り出しは 0 件で、黙って済ませない。
#[test]
fn take_pending_is_zero_without_wiring_and_records_it() {
    let world = World::new();

    let lines = capture(|| {
        assert_eq!(take_pending(&world), 0, "ReadmeWiring 不在は 0 件");
    });
    assert_eq!(
        lines_of(&lines, "readme_drain_no_wiring").len(),
        1,
        "不在を記録する（無記録の失敗経路を作らない）: {lines:?}"
    );
}

/// 要求が無ければ 0 件（毎 tick 走る取り出しの no-op 経路）。
#[test]
fn take_pending_is_zero_when_nothing_is_queued() {
    let mut world = World::new();
    let _tx = wired(&mut world, PathBuf::from("readme.txt"));

    assert_eq!(take_pending(&world), 0, "要求が無ければ 0 件");
}

/// 溜まった要求は全件数えられ、受け口に 1 件も残らない（要件 4.5）。
#[test]
fn take_pending_counts_every_queued_request_and_leaves_none() {
    let mut world = World::new();
    let tx = wired(&mut world, PathBuf::from("readme.txt"));
    for _ in 0..3 {
        tx.send(ReadmeRequest).expect("受信口は生存している");
    }

    assert_eq!(
        take_pending(&world),
        3,
        "溜まった 3 件を全件返す（要件 4.5）"
    );
    assert_eq!(
        take_pending(&world),
        0,
        "取り出した要求は残らない（同じ要求で二度開かない）"
    );
}

// -------------------------------------------------------------------------
// open_from_world（要件 4.2 の入口・持ち物不在）
// -------------------------------------------------------------------------

/// 持ち物が無ければ開かずに `warn!` で記録する（無記録の失敗経路を作らない）。
#[test]
fn open_from_world_warns_without_wiring() {
    let world = World::new();

    let lines = capture(|| open_from_world(&world));

    let warned = lines_of(&lines, "readme_open_no_wiring");
    assert_eq!(
        warned.len(),
        1,
        "ReadmeWiring 不在を 1 行記録する: {lines:?}"
    );
    assert!(
        warned[0].contains("level=WARN"),
        "不在は warn! レベル（エラー表）: {warned:?}"
    );
}

/// 開けなかった失敗はパスと符号を添えて `error!` で 1 行記録し、`Err` を返す（要件 4.4）。
///
/// 実在しないファイルを渡すので、開発者の机では何も開かない（`ShellExecuteW` は「見つからない」の
/// 符号 2 を返すだけで、エラーの窓も出さない）。成功側はアプリが開くのでここでは踏まない。
#[test]
fn open_records_a_missing_file_as_an_error_and_returns_err() {
    let dir = TempPath::new("readme-open-failure");
    let path = dir.child("no-such-readme.txt");

    let mut result = Ok(());
    let lines = capture(|| result = open(&path));

    assert!(result.is_err(), "開けなければ Err（要件 4.4）");
    let failed = lines_of(&lines, "readme_open_failed");
    assert_eq!(failed.len(), 1, "失敗の記録は 1 行: {lines:?}");
    assert!(
        failed[0].contains("level=ERROR") && failed[0].contains("no-such-readme.txt"),
        "error! でパスを添える: {lines:?}"
    );
}

/// 説明書のファイルが無ければ OS を呼ばず、パスを添えて `warn!` で 1 行記録する（要件 5.1・5.5）。
///
/// `readme_open_failed`（`error!`）も `readme_opened`（`info!`）も出ない＝`open` を通っていない。
#[test]
fn open_from_world_skips_a_missing_file_with_a_warning_and_no_os_call() {
    let dir = TempPath::new("readme-open-missing");
    let mut world = World::new();
    let _tx = wired(&mut world, dir.child("readme.txt"));

    let lines = capture(|| open_from_world(&world));

    assert!(
        lines_of(&lines, "readme_open_failed").is_empty(),
        "無いファイルで OS を呼ばない＝error! は 0 行（要件 5.1）: {lines:?}"
    );
    assert!(
        lines_of(&lines, "readme_opened").is_empty(),
        "開いた記録も 0 行: {lines:?}"
    );
    let skipped = lines_of(&lines, "readme_open_skipped_missing");
    assert_eq!(
        skipped.len(),
        1,
        "要求 1 件につき 1 行（要件 5.1）: {lines:?}"
    );
    assert!(
        skipped[0].contains("level=WARN") && skipped[0].contains("readme.txt"),
        "warn! でパスを添える（要件 5.1）: {skipped:?}"
    );
}
