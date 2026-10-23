//! 誤りの無い検体の書き出し: 終了コード 0・CSV を作らない・在った CSV を残す・PNG の名前と
//! 大きさ・同じ指定で 2 度回して同じバイト列（要件 2.1・3.1・4.3・4.9・6.1・6.3・7.6・8.1）。

use temp_path_kit::TempPath;

use crate::common::{BASE, RED, build_ghost, file_names, png_size, run_areka};

/// 面 0 は土台だけ、面 1 は土台の右（x=16）に赤。シェルの下地は 24×8 で、どの面も同じ大きさ。
const SURFACES: &[&str] = &[
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

fn clean_ghost() -> TempPath {
    build_ghost(
        "offline-exe-clean",
        &["name,g_clean"],
        SURFACES,
        &[BASE, RED],
    )
}

/// 誤りが無いと 0 で終わり、CSV を作らず、出力先には `surface<番号>.png` だけが下地の大きさで並ぶ。
/// 既に在る CSV は上書きも削除もしない。
#[test]
fn clean_dump_exits_zero_writes_pngs_and_leaves_csv_alone() {
    let root = clean_ghost();
    let ghost = root.path().display().to_string();

    let fresh_out = root.child("out-fresh");
    let fresh_csv = root.child("fresh.csv");
    let (code, stderr) = run_areka(
        root.path(),
        &[
            "--offline-dump",
            &ghost,
            "--dump-surface-list",
            "0,surface1",
            "--dump-output-dir",
            &fresh_out.display().to_string(),
            "--dump-error-log",
            &fresh_csv.display().to_string(),
        ],
    );
    assert_eq!(code, 0, "誤りの無い検体は 0: {stderr}");
    assert!(!fresh_csv.exists(), "記録が無ければ CSV を作らない");
    assert_eq!(file_names(&fresh_out), ["surface0.png", "surface1.png"]);
    for name in ["surface0.png", "surface1.png"] {
        assert_eq!(
            png_size(&fresh_out.join(name)),
            (24, 8),
            "{name} は下地の大きさ"
        );
    }

    let kept_csv = root.child("kept.csv");
    std::fs::write(&kept_csv, b"keep me\r\n").expect("在った CSV を置く");
    let (code, stderr) = run_areka(
        root.path(),
        &[
            "--offline-dump",
            &ghost,
            "--dump-surface-list",
            "0",
            "--dump-output-dir",
            &root.child("out-kept").display().to_string(),
            "--dump-error-log",
            &kept_csv.display().to_string(),
        ],
    );
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(
        std::fs::read(&kept_csv).expect("在った CSV は残る"),
        b"keep me\r\n",
        "記録が無ければ在った CSV に触れない"
    );
}

/// 同じ指定で 2 度回すと PNG のバイト列が同じ（4.9）。
#[test]
fn the_same_dump_twice_writes_the_same_png_bytes() {
    let root = clean_ghost();
    let ghost = root.path().display().to_string();
    let outs = [root.child("out-a"), root.child("out-b")];
    for out in &outs {
        let (code, stderr) = run_areka(
            root.path(),
            &[
                "--offline-dump",
                &ghost,
                "--dump-surface-list",
                "0-1",
                "--dump-output-dir",
                &out.display().to_string(),
            ],
        );
        assert_eq!(code, 0, "{stderr}");
    }
    for name in ["surface0.png", "surface1.png"] {
        let a = std::fs::read(outs[0].join(name)).expect("1 度目の PNG");
        let b = std::fs::read(outs[1].join(name)).expect("2 度目の PNG");
        assert_eq!(a, b, "{name} は 2 度で同じバイト列");
    }
}
