//! シェルの透過の宣言が焼きに届く檻（spec: areka-P0-self-alpha-declaration 要件 1.1・1.3・1.4・
//! 1.5・1.7・2.8・5.7・6.4・7.4）。
//!
//! 入口 [`load_shell_target`] が呼ばれるたびにシェルの descript.txt の `seriko.use_self_alpha` を
//! 読み、その値で絵を焼くことを、一時フォルダのシェルと焼いた画素で固定する。復号器はメモリ上の
//! もの（パスで引くだけで、ファイルの実在を見ない）。

use super::*;

use areka_emo_atlas::MemoryDecoder;
use temp_path_kit::TempPath;

use super::test_support::{CapturedEvent, capture_events};

/// 本モジュールが出す記録の宛先（既定の target＝モジュールパス）。
const SHELL_TARGET: &str = "areka_emo_present::shell_target";

/// 面 0 の土台に `base.png` を置くだけの `surfaces.txt`。
const SURFACES: &str = "charset,UTF-8\nsurface0\n{\nelement0,overlay,base.png,0,0\n}\n";

/// 同じ色の不透明な 2 画素（左上の色を抜けば全部透明になる）。
const PX: [u8; 4] = [10, 20, 30, 255];

/// `descript` を descript.txt に書いた一時フォルダのシェル（`base.png` の絵は復号器が持つ）。
fn shell_with(label: &str, descript: &[u8]) -> TempPath {
    let dir = TempPath::new(label);
    std::fs::write(dir.child("surfaces.txt"), SURFACES).expect("記述ファイル作成");
    std::fs::write(dir.child("descript.txt"), descript).expect("descript.txt 作成");
    dir
}

/// `base.png` に `pixels`（横 1 列）を入れた復号器。
fn decoder(dir: &TempPath, pixels: &[[u8; 4]], has_alpha: bool) -> MemoryDecoder {
    let mut dec = MemoryDecoder::new();
    let w = pixels.len() as u32;
    dec.insert(
        dir.child("base.png"),
        w,
        1,
        w * 4,
        pixels.concat(),
        has_alpha,
    );
    dec
}

/// 焼いた `base.png` の画素（全部透明で載らなければ `None`）。
fn baked_pixels(dir: &TempPath, dec: &MemoryDecoder) -> Option<Vec<[u8; 4]>> {
    let target = load_shell_target(dir.path(), dec).expect("シェルは読める");
    let table = target.atlas();
    let id = table
        .resolve(SetId(0), "base.png")
        .expect("絵がアトラスに解決される");
    let p = table.entry(id).placement.clone()?;
    let page = table.page(p.page).expect("page exists");
    let mut out = Vec::new();
    for y in p.uv_rect.y..p.uv_rect.y + p.uv_rect.h {
        for x in p.uv_rect.x..p.uv_rect.x + p.uv_rect.w {
            let at = (y * page.stride + x * 4) as usize;
            out.push(page.bytes[at..at + 4].try_into().expect("4 バイト"));
        }
    }
    Some(out)
}

/// `full`: α なしの面の画像の左上の色を抜かない（要件 1.4・1.1・6.4）。対照の `1` は抜く。
#[test]
fn full_keeps_the_top_left_color_of_an_alphaless_face() {
    let dir = shell_with(
        "shell-alpha-full",
        b"charset,UTF-8\nseriko.use_self_alpha,full\n",
    );
    let dec = decoder(&dir, &[PX, PX], false);
    assert_eq!(baked_pixels(&dir, &dec), Some(vec![PX, PX]));

    let control = shell_with("shell-alpha-full-control", b"seriko.use_self_alpha,1\n");
    let dec = decoder(&control, &[PX, PX], false);
    assert_eq!(
        baked_pixels(&control, &dec),
        None,
        "対照: 1 は左上の色で抜く"
    );
}

/// `0`: α ありの絵の半透明が無くなる（要件 1.5）。対照の `1` は半透明のまま。
#[test]
fn zero_makes_an_alpha_picture_opaque() {
    // 左上（半透明）・半透明・不透明の 3 画素（premultiplied）。
    let pixels = [[20, 20, 20, 128], [5, 10, 15, 128], PX];

    let dir = shell_with("shell-alpha-zero", b"seriko.use_self_alpha,0\n");
    let got = baked_pixels(&dir, &decoder(&dir, &pixels, true)).expect("抜かれない画素が残る");
    assert!(
        got.iter().all(|px| px[3] == 0 || px[3] == 255),
        "0 では α は 0 か 255 だけ: {got:?}"
    );
    assert!(got.iter().any(|px| px[3] == 255), "{got:?}");

    let control = shell_with("shell-alpha-zero-control", b"seriko.use_self_alpha,1\n");
    let got = baked_pixels(&control, &decoder(&control, &pixels, true)).expect("載る");
    assert!(
        got.iter().any(|px| px[3] == 128),
        "対照: 1 は半透明のまま: {got:?}"
    );
}

/// 行が無ければ宣言なしの決まり（要件 1.3）: 透明な画素を持たない絵は、α の印があっても左上の
/// 色を抜く（`1` なら α の印のある絵は抜かない）。
#[test]
fn no_line_is_undeclared() {
    let dir = shell_with("shell-alpha-none", "charset,UTF-8\nname,えも\n".as_bytes());
    assert_eq!(baked_pixels(&dir, &decoder(&dir, &[PX, PX], true)), None);

    let control = shell_with("shell-alpha-none-control", b"seriko.use_self_alpha,1\n");
    assert_eq!(
        baked_pixels(&control, &decoder(&control, &[PX, PX], true)),
        Some(vec![PX, PX]),
        "対照: 1 は α の印のある絵を抜かない"
    );
}

/// バルーンのキー `use_self_alpha` だけを書いたシェルは宣言なし（要件 2.8）。
#[test]
fn balloon_key_alone_is_undeclared() {
    let dir = shell_with("shell-alpha-balloon-key", b"use_self_alpha,full\n");
    assert_eq!(baked_pixels(&dir, &decoder(&dir, &[PX, PX], true)), None);
}

/// 同じフォルダの descript.txt を書き換えて入口を呼び直すと、後の宣言で描く（要件 1.7）。
#[test]
fn reload_after_rewriting_descript_uses_the_new_declaration() {
    let dir = shell_with("shell-alpha-rewrite", b"seriko.use_self_alpha,full\n");
    let dec = decoder(&dir, &[PX, PX], false);
    assert_eq!(baked_pixels(&dir, &dec), Some(vec![PX, PX]));

    std::fs::write(dir.child("descript.txt"), b"seriko.use_self_alpha,1\n").expect("書き換え");
    assert_eq!(baked_pixels(&dir, &dec), None, "前の full を持ち越さない");
}

/// 日本語の行を含む Shift_JIS の descript.txt（charset の宣言なし）の `full` が読める
/// （文字コードの既定は ANSI＝Shift_JIS）。
#[test]
fn shift_jis_descript_is_read() {
    // 「name,えも？？」の Shift_JIS のバイト列（え＝82 A6・も＝82 E0・？＝81 48）。
    let bytes = b"name,\x82\xA6\x82\xE0\x81\x48\x81\x48\r\nseriko.use_self_alpha,full\r\n".to_vec();
    assert!(
        String::from_utf8(bytes.clone()).is_err(),
        "前提: UTF-8 では読めない"
    );

    let dir = shell_with("shell-alpha-sjis", &bytes);
    let dec = decoder(&dir, &[PX, PX], false);
    assert_eq!(baked_pixels(&dir, &dec), Some(vec![PX, PX]));
}

/// descript.txt が読めないときは `warn!` を 1 行出し、宣言なしとして続ける（要件 7.4）。
#[test]
fn unreadable_descript_warns_and_is_undeclared() {
    let dir = TempPath::new("shell-alpha-no-descript");
    std::fs::write(dir.child("surfaces.txt"), SURFACES).expect("記述ファイル作成");
    let dec = decoder(&dir, &[PX, PX], true);

    let (pixels, events) = capture_events(|| baked_pixels(&dir, &dec));
    assert_eq!(pixels, None, "宣言なしの決まりで抜かれる");
    let warns: Vec<&CapturedEvent> = events
        .iter()
        .filter(|e| e.target == SHELL_TARGET && e.level == tracing::Level::WARN)
        .collect();
    assert_eq!(warns.len(), 1, "{events:?}");
    assert!(warns[0].message().contains("descript.txt"), "{warns:?}");
}

/// 焼いた絵に `.pna` が添えてあれば、数つきの `warn!` を 1 行出す（要件 5.7）。
#[test]
fn ignored_pna_is_recorded_once_with_the_count() {
    let dir = shell_with("shell-alpha-pna", b"seriko.use_self_alpha,1\n");
    let mut dec = decoder(&dir, &[PX, PX], true);
    dec.insert_pna(dir.child("base.png"));

    let (target, events) =
        capture_events(|| load_shell_target(dir.path(), &dec).expect("シェルは読める"));
    assert_eq!(target.ignored_pna, 1);
    let warns: Vec<&CapturedEvent> = events
        .iter()
        .filter(|e| e.target == SHELL_TARGET && e.level == tracing::Level::WARN)
        .collect();
    assert_eq!(warns.len(), 1, "{events:?}");
    assert_eq!(warns[0].field("ignored_pna"), Some("1"), "{warns:?}");
}

// ── 画像だけのシェル（要件 6.1・6.2・6.3・6.5・6.6・7.3）──────────────────────────

/// `surfaces` を（`Some` なら）surfaces.txt に書き、`images` の名前の面の画像を置いた一時フォルダの
/// シェル（descript.txt は `seriko.use_self_alpha,1`）。画像の画素は復号器が持つ。
fn image_only_shell(
    label: &str,
    surfaces: Option<&str>,
    images: &[&str],
) -> (TempPath, MemoryDecoder) {
    let dir = TempPath::new(label);
    super::test_support::write_descript(dir.path());
    if let Some(text) = surfaces {
        std::fs::write(dir.child("surfaces.txt"), text).expect("記述ファイル作成");
    }
    let mut dec = MemoryDecoder::new();
    for name in images {
        std::fs::File::create(dir.child(name)).expect("プレースホルダ作成");
        dec.insert(dir.child(name), 2, 1, 8, [PX, PX].concat(), true);
    }
    (dir, dec)
}

/// 画像だけで組んだ記録（`surfaces_txt` の欄を持つ `info!`）を集める。
fn image_only_infos(events: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    events
        .iter()
        .filter(|e| {
            e.target == SHELL_TARGET
                && e.level == tracing::Level::INFO
                && e.field("surfaces_txt").is_some()
        })
        .collect()
}

/// 読み込みが成功し面 0 が在ること、画像だけで組んだ記録が 1 行で `surfaces_txt` と面の数を持つ
/// ことを確かめる。
fn assert_image_only(dir: &TempPath, dec: &MemoryDecoder, surfaces_txt: &str, count: &str) {
    let (target, events) =
        capture_events(|| load_shell_target(dir.path(), dec).expect("画像だけで組める"));
    let ids: Vec<u32> = target.build_world().surface_ids().collect();
    assert!(ids.contains(&0), "面 0 が在る: {ids:?}");
    let infos = image_only_infos(&events);
    assert_eq!(infos.len(), 1, "{events:?}");
    assert_eq!(
        infos[0].field_str("surfaces_txt"),
        Some(surfaces_txt),
        "{infos:?}"
    );
    assert_eq!(infos[0].field("surfaces"), Some(count), "{infos:?}");
}

/// `surfaces.txt` が無く `surface0.png` だけのシェルが起動する（要件 6.1・6.3・7.3）。
#[test]
fn missing_surfaces_txt_with_an_image_builds_from_images() {
    let (dir, dec) = image_only_shell("shell-image-only-missing", None, &["surface0.png"]);
    assert_image_only(&dir, &dec, "missing", "1");
}

/// 空の `surfaces.txt`＋画像で起動し、記録の面の数は認めた画像の数（要件 6.2・6.3・7.3）。
#[test]
fn empty_surfaces_txt_with_images_builds_from_images() {
    let (dir, dec) = image_only_shell(
        "shell-image-only-empty",
        Some(""),
        &["surface0.png", "surface10.png"],
    );
    assert_image_only(&dir, &dec, "empty", "2");
}

/// 波括弧 0 個の `surfaces.txt`＋画像で起動する（要件 6.2）。
#[test]
fn surfaces_txt_without_braces_with_an_image_builds_from_images() {
    let (dir, dec) = image_only_shell(
        "shell-image-only-no-brace",
        Some("charset,UTF-8\ndescript\n"),
        &["surface0.png"],
    );
    assert_image_only(&dir, &dec, "empty", "1");
}

/// 面も画像も無ければ `Empty`（場所はシェルのフォルダ）で、`error!` を 1 行伴う（要件 6.5・7.4）。
#[test]
fn no_surface_and_no_image_yields_empty_at_the_shell_dir() {
    for (label, surfaces) in [
        ("shell-image-only-none-missing", None),
        ("shell-image-only-none-empty", Some("charset,UTF-8\n")),
    ] {
        let (dir, dec) = image_only_shell(label, surfaces, &[]);
        let (result, events) = capture_events(|| load_shell_target(dir.path(), &dec));
        match result {
            Err(ShellLoadError::Empty { path }) => assert_eq!(path, dir.path()),
            other => panic!("{label}: 面が無い失敗は Empty でなければならない: {other:?}"),
        }
        let errors = events
            .iter()
            .filter(|e| e.target == SHELL_TARGET && e.level == tracing::Level::ERROR)
            .count();
        assert_eq!(errors, 1, "{label}: {events:?}");
    }
}

/// `surfaces.txt` が在るのに読めない（フォルダ）なら、画像が在っても今までどおり `Read`（要件 6.6）。
#[test]
fn unreadable_surfaces_txt_yields_read_even_with_images() {
    let (dir, dec) = image_only_shell("shell-image-only-unreadable", None, &["surface0.png"]);
    std::fs::create_dir(dir.child("surfaces.txt")).expect("フォルダ作成");
    match load_shell_target(dir.path(), &dec) {
        Err(ShellLoadError::Read { path, .. }) => assert_eq!(path, dir.child("surfaces.txt")),
        other => panic!("読めない surfaces.txt は Read でなければならない: {other:?}"),
    }
}

/// 面を定義する `surfaces.txt` では画像だけで組んだ記録を出さない（要件 6.7 の対照）。
#[test]
fn defined_surfaces_do_not_record_image_only() {
    let dir = shell_with("shell-image-only-control", b"seriko.use_self_alpha,1\n");
    let dec = decoder(&dir, &[PX, PX], true);
    let (_, events) =
        capture_events(|| load_shell_target(dir.path(), &dec).expect("シェルは読める"));
    assert!(image_only_infos(&events).is_empty(), "{events:?}");
}
