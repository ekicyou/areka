//! 保管フォルダの名前集合と枠画像の原寸を固定する。
//!
//! 出典 spec: `areka-P0-default-balloon-bundle`（要件 **2.2**／**5.4**・設計 **C2** の A 行）。

use std::path::Path;

use super::test_support::{EXPECTED_FILE_NAMES, EXPECTED_FRAME_SIZES, staysee_root};

/// PNG の IHDR から原寸 `(width, height)` を読む（署名 8B ＋ 長さ 4B ＋ `IHDR` 4B の直後）。
///
/// 画像デコーダ（WIC）を持ち込まないのは本テーマを純粋層・非 COM に保つため。
/// IHDR がストリーム先頭に在ることは PNG 仕様が定めているので固定オフセットで足りる。
fn png_native_size(path: &Path) -> (u32, u32) {
    let bytes = std::fs::read(path)
        .unwrap_or_else(|e| panic!("バルーン枠画像 {} の読取に失敗した: {e}", path.display()));
    assert!(
        bytes.len() >= 24 && &bytes[..8] == b"\x89PNG\r\n\x1a\n" && &bytes[12..16] == b"IHDR",
        "{} が PNG（先頭 IHDR チャンク付き）として読めない",
        path.display()
    );
    let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    (w, h)
}

/// 保管フォルダ直下のエントリ名が 29 本の固定集合と**過不足なく**一致し、サブフォルダが 0 である。
///
/// 母数（数えた本数）を先に固定してから集合を突合する。上流を取り直してファイルが増減したら
/// ここが赤になる＝意図した検出であり、期待値を緩めるのではなく `provenance.md` を採り直す。
#[test]
fn stored_folder_holds_exactly_the_upstream_29_files() {
    let root = staysee_root();
    let entries = std::fs::read_dir(&root).unwrap_or_else(|e| {
        panic!(
            "既定バルーンの保管フォルダ {} を読めない（本テストはこのフォルダの実在が前提）: {e}",
            root.display()
        )
    });

    let mut names: Vec<String> = Vec::new();
    let mut dir_names: Vec<String> = Vec::new();
    for entry in entries {
        let entry = entry.unwrap_or_else(|e| {
            panic!("{} の列挙中に失敗した: {e}", root.display());
        });
        let name = entry.file_name().to_string_lossy().into_owned();
        let file_type = entry
            .file_type()
            .unwrap_or_else(|e| panic!("{} の種別を取れない: {e}", root.display()));
        if file_type.is_dir() {
            dir_names.push(name.clone());
        }
        names.push(name);
    }
    names.sort();

    assert_eq!(
        dir_names.len(),
        0,
        "{}: サブフォルダは 0 であること（上流の配布物は平坦・`.git` も置かない）。実在: {:?}",
        root.display(),
        dir_names
    );
    assert_eq!(
        names.len(),
        EXPECTED_FILE_NAMES.len(),
        "{}: 直下のエントリ数が期待 {} 本に対し実測 {} 本（出所: 上流 fe1b02f3 の配布物・要件 2.2）",
        root.display(),
        EXPECTED_FILE_NAMES.len(),
        names.len()
    );

    let expected: Vec<String> = EXPECTED_FILE_NAMES.iter().map(|s| s.to_string()).collect();
    let extra: Vec<&String> = names.iter().filter(|n| !expected.contains(n)).collect();
    let missing: Vec<&String> = expected.iter().filter(|n| !names.contains(n)).collect();
    assert!(
        extra.is_empty() && missing.is_empty(),
        "{}: 保管ファイルの集合が上流 29 本と食い違う。余分 {} 件 {:?}／欠落 {} 件 {:?}",
        root.display(),
        extra.len(),
        extra,
        missing.len(),
        missing
    );
}

/// 枠として焼き込む 6 枚の原寸が実 PNG の IHDR と一致する。
///
/// 原寸をハードコードしつつ実ファイルと突合するのは、PNG が差し替わったときに期待値が
/// 黙って追従して「テストが自分で期待値を作る」形になるのを防ぐためである。
#[test]
fn frame_png_native_sizes_are_pinned() {
    let root = staysee_root();
    assert_eq!(
        EXPECTED_FRAME_SIZES.len(),
        6,
        "枠画像の母数は 6 枚（`balloons0〜3` と `balloonk0〜1`）であること"
    );
    for (name, width, height) in EXPECTED_FRAME_SIZES {
        assert_eq!(
            png_native_size(&root.join(name)),
            (width, height),
            "{}: 原寸が期待 {width}×{height} と違う（出所: 上流 fe1b02f3 の IHDR 実測）",
            root.join(name).display()
        );
    }
}
