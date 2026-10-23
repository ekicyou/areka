//! 失敗の分岐と終了コード: ゴーストのフォルダの末尾の `\`・今のフォルダからの相対パス・
//! `--dump-error-log` の有無・無い `--dump-shell`・無いゴースト・面の指定なし・`--offline-dump` の
//! 値なし・中身の無い面・無いシェルのフォルダ（要件 1.8・2.1・2.2・2.5〜2.7・3.6・4.8・8.2・8.4・
//! 12.4・12.6）。根・今のフォルダ・出力はどれも `temp-path-kit` で `target\` の下。

use temp_path_kit::TempPath;

use crate::common::{BASE, RED, build_ghost, csv_rows, file_names, run_areka, write_ghost};

/// 面 0 は土台だけ、面 1 は土台の右に赤。誤りの無いシェル。
const PLAIN: &[&str] = &[
    "surface0",
    "{",
    "element0,overlay,base.png,0,0",
    "}",
    "surface1",
    "{",
    "element0,overlay,base.png,0,0",
    "element1,overlay,red.png,16,0",
    "}",
];

fn plain_ghost(label: &str) -> TempPath {
    build_ghost(label, &["name,g_fail"], PLAIN, &[BASE, RED])
}

/// `args` の頭に `--offline-dump <ghost>` を付けて `ghost` の根で回す。
fn dump(ghost: &TempPath, args: &[&str]) -> (i32, String) {
    let dir = ghost.path().display().to_string();
    let mut all = vec!["--offline-dump", dir.as_str()];
    all.extend_from_slice(args);
    run_areka(ghost.path(), &all)
}

/// 末尾の `\` の有無で同じ PNG のバイト列（2.1）。
#[test]
fn a_trailing_backslash_on_the_ghost_folder_writes_the_same_pngs() {
    let root = plain_ghost("offline-exe-trailing");
    let bare = root.path().display().to_string();
    let slashed = format!("{bare}\\");
    let outs = [root.child("out-bare"), root.child("out-slashed")];
    for (ghost, out) in [&bare, &slashed].into_iter().zip(&outs) {
        let (code, stderr) = run_areka(
            root.path(),
            &[
                "--offline-dump",
                ghost,
                "--dump-surface-list",
                "0-1",
                "--dump-output-dir",
                &out.display().to_string(),
            ],
        );
        assert_eq!(code, 0, "{ghost}: {stderr}");
    }
    assert_eq!(file_names(&outs[0]), ["surface0.png", "surface1.png"]);
    assert_eq!(file_names(&outs[1]), file_names(&outs[0]));
    for name in ["surface0.png", "surface1.png"] {
        assert_eq!(
            std::fs::read(outs[0].join(name)).expect("末尾の \\ なしの PNG"),
            std::fs::read(outs[1].join(name)).expect("末尾の \\ ありの PNG"),
            "{name}"
        );
    }
}

/// 相対の `--offline-dump`・`--dump-output-dir`・`--dump-error-log` は起動したときの今のフォルダから
/// 読む（2.2）。CSV は単独で指した無い面 999 の Warning で出る。
#[test]
fn relative_paths_are_read_from_the_current_folder() {
    let cwd = TempPath::under_target("offline-exe-relative");
    write_ghost(&cwd.child("g"), &["name,g_rel"], PLAIN, &[BASE, RED]);
    let (code, stderr) = run_areka(
        cwd.path(),
        &[
            "--offline-dump",
            "g",
            "--dump-surface-list",
            "0,999",
            "--dump-output-dir",
            "out",
            "--dump-error-log",
            "errors.csv",
        ],
    );
    assert_eq!(code, 1, "Warning までなら 1: {stderr}");
    assert_eq!(file_names(&cwd.child("out")), ["surface0.png"]);
    let rows = csv_rows(&cwd.child("errors.csv"));
    assert_eq!(rows.len(), 1, "無い面 999 の 1 行");
    assert_eq!(rows[0].level, "Warning");
    assert_eq!(rows[0].ghost, "g_rel");
}

/// 終了コードは `--dump-error-log` の有無で変わらない（8.2）。誤りの無いシェルで無い面 999 の
/// Warning だけなら 1、絵の欠けたシェル（読み込みの段で Error）なら 2。
#[test]
fn the_exit_code_is_the_same_without_an_error_log() {
    let warn_only = plain_ghost("offline-exe-no-log-warn");
    let broken = build_ghost(
        "offline-exe-no-log-error",
        &["name,g"],
        &[
            "surface0",
            "{",
            "element0,overlay,base.png,0,0",
            "}",
            "surface5",
            "{",
            "element0,overlay,base.png,0,0",
            "element1,overlay,nofile.png,8,0",
            "}",
        ],
        &[BASE],
    );
    for (root, list, want) in [(&warn_only, "0,999", 1), (&broken, "5", 2)] {
        let out = root.child("out");
        let out = out.display().to_string();
        let csv = root.child("errors.csv");
        let csv = csv.display().to_string();
        let base = ["--dump-surface-list", list, "--dump-output-dir", &out];
        let (with_log, stderr) = dump(root, &[&base[..], &["--dump-error-log", &csv]].concat());
        assert_eq!(with_log, want, "{list}: {stderr}");
        let (without_log, stderr) = dump(root, &base);
        assert_eq!(without_log, want, "{list} を CSV なしで: {stderr}");
    }
}

/// 無い `--dump-shell` は既定のシェルに代えず PNG を書かず、Error 1 行で 2（2.5）。
#[test]
fn a_missing_dump_shell_writes_no_png_and_exits_two() {
    let root = plain_ghost("offline-exe-no-dump-shell");
    let (out, csv) = (root.child("out"), root.child("errors.csv"));
    let (code, stderr) = dump(
        &root,
        &[
            "--dump-shell",
            "no_such_shell",
            "--dump-surface-list",
            "0-1",
            "--dump-output-dir",
            &out.display().to_string(),
            "--dump-error-log",
            &csv.display().to_string(),
        ],
    );
    assert_eq!(code, 2, "{stderr}");
    assert!(!out.exists(), "PNG を 1 枚も書かない");
    let rows = csv_rows(&csv);
    assert_eq!(rows.len(), 1, "見つからない 1 行");
    assert_eq!(rows[0].level, "Error");
    assert!(rows[0].body.contains("no_such_shell"), "{}", rows[0].body);
}

/// ゴーストのフォルダが無いと 0 で、CSV も PNG も作らない（2.6・8.4）。
#[test]
fn a_missing_ghost_folder_exits_zero_without_csv() {
    let root = TempPath::under_target("offline-exe-no-ghost");
    let (out, csv) = (root.child("out"), root.child("errors.csv"));
    let (code, stderr) = run_areka(
        root.path(),
        &[
            "--offline-dump",
            &root.child("no-such-ghost").display().to_string(),
            "--dump-surface-list",
            "0",
            "--dump-output-dir",
            &out.display().to_string(),
            "--dump-error-log",
            &csv.display().to_string(),
        ],
    );
    assert_eq!(code, 0, "{stderr}");
    assert!(!csv.exists(), "CSV を作らない");
    assert!(!out.exists(), "PNG を書かない");
}

/// `--offline-dump` の値なしはゴーストの読み込みの失敗と同じに 0・CSV なし（1.8）。
#[test]
fn offline_dump_without_a_value_exits_zero_without_csv() {
    let root = plain_ghost("offline-exe-no-value");
    let (out, csv) = (root.child("out"), root.child("errors.csv"));
    let (code, stderr) = run_areka(
        root.path(),
        &[
            "--dump-surface-list",
            "0",
            "--dump-output-dir",
            &out.display().to_string(),
            "--dump-error-log",
            &csv.display().to_string(),
            // 最後に置いて値を取らせない。
            "--offline-dump",
        ],
    );
    assert_eq!(code, 0, "{stderr}");
    assert!(!csv.exists(), "CSV を作らない");
    assert!(!out.exists(), "PNG を書かない");
}

/// 面の指定なしは PNG を書かず、読み込みの記録だけを CSV に残す（3.6）。CSV を作るのは知らない
/// `animation-sort` の Warning（シェルの読み込みの段で出る）の 1 行。
#[test]
fn without_a_surface_list_no_png_is_written_but_the_csv_is() {
    let surfaces: Vec<&str> = ["animation-sort,sideways"]
        .into_iter()
        .chain(PLAIN.iter().copied())
        .collect();
    let root = build_ghost("offline-exe-no-list", &["name,g"], &surfaces, &[BASE, RED]);
    let (out, csv) = (root.child("out"), root.child("errors.csv"));
    let (code, stderr) = dump(
        &root,
        &[
            "--dump-output-dir",
            &out.display().to_string(),
            "--dump-error-log",
            &csv.display().to_string(),
        ],
    );
    assert_eq!(code, 1, "Warning までなら 1: {stderr}");
    assert!(!out.exists(), "PNG を書かない");
    let rows = csv_rows(&csv);
    assert_eq!(rows.len(), 1, "animation-sort の 1 行");
    assert_eq!(rows[0].level, "Warning");
    assert!(rows[0].body.contains("animation-sort"), "{}", rows[0].body);
}

/// 中身の無い面は合成の失敗の Error ちょうど 1 行で飛ばし、残りの面は書く（4.8）。
#[test]
fn an_empty_surface_is_one_error_and_the_rest_are_written() {
    let surfaces: Vec<&str> = PLAIN
        .iter()
        .copied()
        .chain(["surface6", "{", "}"])
        .collect();
    let root = build_ghost("offline-exe-empty", &["name,g"], &surfaces, &[BASE, RED]);
    let (out, csv) = (root.child("out"), root.child("errors.csv"));
    let (code, stderr) = dump(
        &root,
        &[
            "--dump-surface-list",
            "0-10",
            "--dump-output-dir",
            &out.display().to_string(),
            "--dump-error-log",
            &csv.display().to_string(),
        ],
    );
    assert_eq!(code, 2, "{stderr}");
    assert_eq!(file_names(&out), ["surface0.png", "surface1.png"]);
    let rows = csv_rows(&csv);
    let errors: Vec<_> = rows.iter().filter(|r| r.level == "Error").collect();
    assert_eq!(errors.len(), 1, "合成の失敗の 1 行");
    assert!(
        errors[0].body.contains("EmptyComposition"),
        "{}",
        errors[0].body
    );
}

/// 既定のシェルのフォルダが無いと PNG を書かず 2（2.7）。
#[test]
fn a_missing_shell_folder_writes_no_png_and_exits_two() {
    let root = plain_ghost("offline-exe-no-shell");
    std::fs::remove_dir_all(root.child("shell")).expect("シェルのフォルダを消す");
    let out = root.child("out");
    let (code, stderr) = dump(
        &root,
        &[
            "--dump-surface-list",
            "0-1",
            "--dump-output-dir",
            &out.display().to_string(),
        ],
    );
    assert_eq!(code, 2, "{stderr}");
    assert!(!out.exists(), "PNG を書かない");
}
