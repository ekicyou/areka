use super::*;

/// 名前の列から「番号 → 採ったファイル名」だけを取り出す（表明を短く保つための道具）。
fn images(names: &[&str]) -> Vec<(u32, String)> {
    select_surface_images(names)
        .images
        .into_iter()
        .collect::<Vec<_>>()
}

/// R1.2/R7.1 先頭の 0 を無視して 10 進数として読む: 4 表記がすべて面 0 になる。
///
/// 数字列を**数として**読まず字面のまま扱うと（`surface00.png` を面 0 と認めない・
/// `surface0010.png` を面 10 と読まない）この表明が赤になる（R7.9 の摂動点）。
#[test]
fn leading_zeros_are_ignored_and_digits_read_as_decimal() {
    for name in [
        "surface0.png",
        "surface00.png",
        "surface000.png",
        "surface0000.png",
    ] {
        assert_eq!(
            images(&[name]),
            vec![(0, name.to_string())],
            "{name} は面 0 の画像である"
        );
    }
    assert_eq!(
        images(&["surface0010.png"]),
        vec![(10, "surface0010.png".to_string())],
        "surface0010.png は面 10（先頭の 0 を無視した 10 進数）"
    );
}

/// R1.3/R7.1 面の画像と認めない名前は 0 件になる。
///
/// `surface` で始まらないもの・数字以外を含むもの・拡張子が `.png` でないものの 3 系統。
/// `.pna` が 0 件であることが R2.6（同名の `.pna` を読まない）の構造的な担保である。
#[test]
fn non_surface_image_names_yield_nothing() {
    for name in [
        "menu_background.png", // surface で始まらない
        "surfaces.txt",        // 数字以外を含む・非 png
        "surfacetable.txt",    // 数字以外を含む・非 png
        "surface+0.png",       // 符号付きは全数字でない
        "surface-1.png",       // 符号付きは全数字でない
        "surface.png",         // 残余が空
        "surface0.pna",        // 非 png（R2.6）
        "surface0.jpg",        // 非 png
    ] {
        let selection = select_surface_images(&[name]);
        assert!(
            selection.images.is_empty(),
            "{name} を面の画像と認めてはならない: {:?}",
            selection.images
        );
        assert!(
            selection.duplicates.is_empty() && selection.overflow.is_empty(),
            "{name} は重複にも桁溢れにも数えない"
        );
    }
}

/// R1.4 接頭辞 `surface` と拡張子 `.png` の大小を区別しない（`face_digits_of` と同じ扱い）。
#[test]
fn prefix_and_extension_are_case_insensitive() {
    assert_eq!(
        images(&["SURFACE0.PNG"]),
        vec![(0, "SURFACE0.PNG".to_string())],
        "大小無視で面 0・採ったファイル名は元の綴りのまま"
    );
}

/// R1.5 同じ番号に複数あれば**ファイル名の辞書順で最小**を採り、捨てた名前を `duplicates` に積む。
///
/// 記録（`warn!`）を出すのは呼び手（読み込みの権威）であり、本純関数は事実を返すだけである。
#[test]
fn duplicate_ids_adopt_lexicographic_minimum_and_report_the_dropped() {
    let selection = select_surface_images(&["surface0000.png", "surface0.png"]);
    assert_eq!(
        selection.images.get(&0).map(String::as_str),
        Some("surface0.png"),
        "辞書順で最小の名前を採る"
    );
    assert_eq!(
        selection.duplicates,
        vec![(
            0,
            "surface0.png".to_string(),
            vec!["surface0000.png".to_string()]
        )],
        "捨てた名前を 1 件だけ積む"
    );
}

/// R1.6 数字列が面の番号として表せないほど大きければ、画像と認めず `overflow` に積む。
#[test]
fn digits_too_large_for_a_surface_id_go_to_overflow() {
    let selection = select_surface_images(&["surface99999999999.png", "surface1.png"]);
    assert_eq!(
        selection.overflow,
        vec!["surface99999999999.png".to_string()],
        "u32 に収まらない数字列は桁溢れ"
    );
    assert_eq!(
        images(&["surface99999999999.png", "surface1.png"]),
        vec![(1, "surface1.png".to_string())],
        "桁溢れの名前は画像に数えない"
    );
}

/// R1.8 同じフォルダの内容なら、走査順が違っても同じ対応を得る。
#[test]
fn selection_is_independent_of_input_order() {
    let ascending = [
        "surface0.png",
        "surface0000.png",
        "surface2.png",
        "surface0010.png",
        "surface99999999999.png",
        "surfaces.txt",
    ];
    let mut shuffled = ascending;
    shuffled.reverse();

    let a = select_surface_images(&ascending);
    let b = select_surface_images(&shuffled);
    assert_eq!(
        a.images, b.images,
        "番号 → ファイル名の対応が走査順に依存しない"
    );
    assert_eq!(a.duplicates, b.duplicates, "重複の報告が走査順に依存しない");
    assert_eq!(a.overflow, b.overflow, "桁溢れの報告が走査順に依存しない");
    assert_eq!(
        a.images.keys().copied().collect::<Vec<_>>(),
        vec![0, 2, 10],
        "番号は数値の昇順（辞書順ではない）"
    );
}
