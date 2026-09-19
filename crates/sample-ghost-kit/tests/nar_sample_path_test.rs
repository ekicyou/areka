//! コマンド `nar-sample-path` の検査（spec: `areka-P0-nar-install` 要件 1.9）。
//!
//! # 組み上がった実行体を起こす
//!
//! 判定するのは出力の形・終了コード・どちらの流れに出るかの 3 つで、どれも `main` の配線
//! そのものである。中の関数を直に呼ぶと「関数は正しいが `main` が繋いでいない」形を見逃す
//! ので、**実行体を起こして**実測する。実行体の場所は cargo が `CARGO_BIN_EXE_<名>` で
//! 教える（この検査が結合テストとして置かれているのは、この 1 点のため——`src/` の兄弟
//! テストにすると実行体は組まれず、場所も教わらない）。
//!
//! `cargo run` は使わない。検査の途中で組み直しが走ると、測っているものが変わる。
//!
//! # 手動用の根は検体ごとに 1 つ
//!
//! `manual/<名>/` は呼ぶたびに消して作り直す。同じ検体を触る検査が同時に走ると、片方の
//! 複写の途中でもう片方が消す。同じテストバイナリの中で直列化する。

use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use sample_ghost_kit::{SAMPLES, SampleKind};

/// 組み上がった実行体。cargo が結合テストにだけ教える。
const COMMAND: &str = env!("CARGO_BIN_EXE_nar-sample-path");

/// 手動用の根を触る検査を直列化する錠。
static MANUAL: Mutex<()> = Mutex::new(());

/// 錠を取る。前の検査が落ちて毒されていても、木の状態は次の呼び出しが作り直すので続ける。
fn manual_lock() -> MutexGuard<'static, ()> {
    MANUAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 実行体を起こして (終了コード, 標準出力, 標準エラー) を取る。
fn run(name: &str) -> (Option<i32>, String, String) {
    let out = std::process::Command::new(COMMAND)
        .arg(name)
        .output()
        .expect("実行体を起こせること");
    (
        out.status.code(),
        String::from_utf8(out.stdout).expect("標準出力は UTF-8"),
        String::from_utf8(out.stderr).expect("標準エラーは UTF-8"),
    )
}

/// 標準出力を `key=value` の組に割る。1 行 1 組で、最後の行も改行で終わる。
fn pairs(out: &str) -> Vec<(String, String)> {
    assert!(
        out.ends_with('\n'),
        "最後の組も 1 行として終わること: {out:?}"
    );
    out.lines()
        .map(|line| {
            let (key, value) = line
                .split_once('=')
                .unwrap_or_else(|| panic!("`key=value` の形をしていない行: {line:?}"));
            (key.to_owned(), value.to_owned())
        })
        .collect()
}

/// 登記した検体すべてで、1 行 1 組・`key=value`・絶対パス・末尾の区切り記号なしの形で
/// 根・検体フォルダ・同梱バルーンが出る（要件 1.9）。
///
/// この出力は `tools/perf/` の 2 本が `folder=` の 1 行として読む。鍵の綴りと並び、値の形
/// （`..` を畳んだ絶対パス）をここで固定する——受け手は `=` の左右に割った値をそのまま別の
/// 道具へ渡すので、綴りが揺れたら読めない。
#[test]
fn the_command_prints_one_absolute_path_per_line_for_every_registered_sample() {
    let _serialized = manual_lock();
    for sample in SAMPLES {
        let (code, out, err) = run(sample.name);
        assert_eq!(
            code,
            Some(0),
            "登記済みの検体 {} は成功すること: {err}",
            sample.name
        );
        assert!(
            err.is_empty(),
            "成功時は標準エラーに何も出さないこと: {err:?}"
        );

        let printed = pairs(&out);
        let keys: Vec<&str> = printed.iter().map(|(key, _)| key.as_str()).collect();
        let mut expected = vec!["root".to_owned(), "folder".to_owned()];
        expected.extend(
            sample
                .balloons
                .iter()
                .map(|balloon| format!("balloon.{balloon}")),
        );
        assert_eq!(keys, expected, "検体 {} の鍵の綴りと並び", sample.name);

        for (key, value) in &printed {
            let path = Path::new(value);
            assert!(path.is_absolute(), "{key} が絶対パスでない: {value}");
            assert!(path.is_dir(), "{key} が実在しない: {value}");
            assert!(
                !value.ends_with('\\') && !value.ends_with('/'),
                "{key} の末尾に区切り記号が付いている: {value}"
            );
            assert!(
                !path.components().any(|part| part == Component::ParentDir),
                "{key} に `..` が残っている（受け手がそのまま別の道具へ渡せない）: {value}"
            );
        }

        let root = Path::new(&printed[0].1);
        let store = match sample.kind {
            SampleKind::Ghost => "ghost",
            SampleKind::Balloon => "balloon",
        };
        assert_eq!(
            Path::new(&printed[1].1),
            root.join(store).join(sample.name),
            "検体 {} の `folder=` は登記の行から導かれること",
            sample.name
        );
        for (index, balloon) in sample.balloons.iter().enumerate() {
            assert_eq!(
                Path::new(&printed[2 + index].1),
                root.join("balloon").join(balloon),
                "同梱バルーン {balloon} は `<根>/balloon/<名>`"
            );
        }
    }
}

/// コマンドを 2 回続けて呼ぶと、2 回とも**起動記録の無い根**が印字される（完了状態・
/// 要件 1.9・9.7）。
///
/// 「2 回呼べる」だけでは作り直さない実装でも緑になる。1 回目に印字された根へ起動記録の形
/// をしたファイルを置き、実在を較正してから 2 回目を呼ぶ。プロセスが終わっても木が残ること
/// （実機はこの後で `areka.exe` にこのパスを渡す）も同時に判定する。
#[test]
fn two_runs_in_a_row_each_hand_out_a_root_with_no_boot_record() {
    let _serialized = manual_lock();
    let (first_code, first_out, first_err) = run("emo2");
    assert_eq!(first_code, Some(0), "1 回目: {first_err}");
    let first = pairs(&first_out);
    let folder = PathBuf::from(&first[1].1);
    assert!(
        folder.join("ghost").join("master").is_dir(),
        "プロセスが終わっても木が残ること: {}",
        folder.display()
    );

    let profile = folder.join("profile").join("areka");
    std::fs::create_dir_all(&profile).expect("起動記録の置き場を作れるはず");
    let record = profile.join("sylphya.toml");
    std::fs::write(&record, b"boot_count = 1\n").expect("起動記録を置けるはず");
    assert!(
        record.is_file(),
        "較正: 置いた起動記録が実在すること: {}",
        record.display()
    );

    let (second_code, second_out, second_err) = run("emo2");
    assert_eq!(second_code, Some(0), "2 回目: {second_err}");
    assert_eq!(pairs(&second_out), first, "2 回とも同じ場所を印字すること");
    assert!(
        !record.exists(),
        "2 回目の根に 1 回目の起動記録が残っている: {}",
        record.display()
    );
    assert!(
        folder.join("ghost").join("master").is_dir(),
        "作り直した木が完全であること: {}",
        folder.display()
    );
}

/// 未登録の名前は、理由を**標準エラー**に出して終了コード 2 で終わる（要件 1.4・1.9）。
///
/// 標準出力は空でなければならない——`folder=` を読む側は行が来ないことで失敗に気付く。
#[test]
fn an_unregistered_name_prints_the_reason_to_standard_error_and_exits_with_two() {
    let (code, out, err) = run("no-such-ghost");
    assert_eq!(code, Some(2), "未登録の名前の終了コード");
    assert!(out.is_empty(), "標準出力には何も出さないこと: {out:?}");
    assert!(
        err.contains("no-such-ghost"),
        "理由に渡された名前が載ること: {err:?}"
    );
    for sample in SAMPLES {
        assert!(
            err.contains(sample.name),
            "理由に既知の名前 {} が載ること: {err:?}",
            sample.name
        );
    }
}
