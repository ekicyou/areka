//! element定義の描画メソッド `base` を、権威の核 [`build_shell_target`] から合成まで通して
//! 外形と画素で確かめる檻（要件 4.1・1.1・1.2・1.3・1.7・2.4・2.5・3.2・3.4）。
//!
//! 読み手（`areka-parsers` の `is_image_element_method`）が `base` を値にすることは
//! `decode_tests_method_matrix_tests.rs` が固定する。ここで見るのは、その値が焼く絵の一覧・
//! 土台の絵の決定・合成を通って**見える形**（大きさと画素）になることである。
//!
//! - ⑴ `base` の土台に `overlay` を重ねたサーフェス: 外形が土台の絵の実寸・土台と部品の両方の
//!   画素・同じ番号の `surface*.png` は使われない。読み手の判定を `overlay` だけに戻すと赤になる
//!   （`element0` が値にならず、`surface26.png` が土台に敷かれて外形も色も動く）。
//! - ⑵〜⑷ 今の見え方の不変: 比べる合成結果そのものは判定を戻しても等しい（戻した側でも同じ絵に
//!   なる文面を比べているため）。この 3 本は「`base` を値にしたことで見え方が変わっていない」を
//!   固定する。判定を戻すと ⑵ は前提（`shadowed` の内訳）で、⑷ は脱落の記録の件数で赤になり、
//!   ⑶ は `base` を含まないので緑のままである。
//!
//! ⑴〜⑷ は fs に触れない。画像はメモリ上の復号器（[`MemoryDecoder`]）に実寸と色で登録し、
//! シェルのフォルダは実在しないパスでよい（`shell_target_base_image_tests.rs` と同じ形）。
//!
//! 続く 3 本は入口 [`load_shell_target`] の警告（描けない行 1 行につき `warn!` 1 行・読み込み
//! 1 回につき 1 度）を見る。こちらは surfaces.txt を `TempPath` に置いて読ませる。
//!
//! 最後の 1 本は不具合の実物——無改変のクローディア（`claudia` 検体）を実際の画像の復号で読み、
//! `base` を使う surface6・11・26 の外形を固定する（要件 4.3）。

use super::*;

use std::path::{Path, PathBuf};

use areka_emo_atlas::{MemoryDecoder, WicDecoderArm};
use areka_emo_compose::{BindSet, ComposeError, Composer, PatternState};
use areka_parsers::shell::parse;
use temp_path_kit::TempPath;

use super::test_support::{
    CapturedEvent, capture_events, claudia_shell_dir, with_com_initialized, write_descript,
};

/// 画像 1 枚の登録内容（ファイル名・実寸・全画素の色＝premultiplied BGRA 不透明）。
type Image = (&'static str, (u32, u32), [u8; 4]);

/// 合成した結果の比べられる形（外形と画素列）。
type Snapshot = Result<(u32, u32, Vec<u8>), ComposeError>;

/// ⑴ の土台の絵。`surface26.png` とは縦横どちらも違う。
const BODY: (u32, u32) = (10, 14);
/// ⑴ の部品の絵と位置（土台の内側に収まる）。
const FACE: (u32, u32) = (4, 4);
const FACE_AT: (u32, u32) = (3, 5);
/// ⑴ の同じ番号の画像（使われてはならない・大きさも色も違う）。
const SURFACE26: (u32, u32) = (20, 6);

const COLOR_BODY: [u8; 4] = [200, 0, 0, 255];
const COLOR_FACE: [u8; 4] = [0, 0, 200, 255];
const COLOR_SURFACE: [u8; 4] = [0, 200, 0, 255];
const COLOR_X: [u8; 4] = [180, 180, 0, 255];

/// 実在しないシェルのフォルダ（核は fs を触らないので、復号器の鍵を組むためだけに使う）。
fn shell_dir() -> PathBuf {
    PathBuf::from(r"C:\areka-test\shell-target-element-base")
}

/// 全画素が同じ色の不透明な絵を復号器へ登録する。
fn insert_solid(dec: &mut MemoryDecoder, dir: &Path, (name, (w, h), color): Image) {
    let bgra: Vec<u8> = color
        .iter()
        .copied()
        .cycle()
        .take((w * h * 4) as usize)
        .collect();
    dec.insert(dir.join(name), w, h, w * 4, bgra, true);
}

/// 文面を読み、`files` をフォルダ直下に在るファイル名・`images` を復号器の中身として焼く。
fn bake(text: &str, files: &[&str], images: &[Image]) -> ShellTarget {
    let dir = shell_dir();
    let mut dec = MemoryDecoder::new();
    for &image in images {
        insert_solid(&mut dec, &dir, image);
    }
    build_shell_target(
        parse(text),
        select_surface_images(files),
        &dir,
        &dec,
        UseSelfAlpha::On,
    )
}

/// 面 1 枚を素の状態（有効 bind もコマも無い）で合成し、外形と画素列にする。
fn snapshot(target: &ShellTarget, id: u32) -> Snapshot {
    let world = target.build_world();
    Composer::new()
        .compose(
            &world,
            target.atlas(),
            id,
            &BindSet::default(),
            &PatternState::default(),
        )
        .map(|s| (s.width(), s.height(), s.bytes().to_vec()))
}

/// 合成結果の 1 画素を引く（`stride` は幅×4 の詰めた並び）。
fn pixel((w, _, bytes): &(u32, u32, Vec<u8>), x: u32, y: u32) -> [u8; 4] {
    let at = ((y * w + x) * 4) as usize;
    bytes[at..at + 4].try_into().expect("4 バイトの画素")
}

/// 脱落の記録を比べられる形（表示の文字列）にする。
fn bake_errors(target: &ShellTarget) -> Vec<String> {
    target.bake_errors().iter().map(|e| e.to_string()).collect()
}

/// ⑴（要件 4.1・1.1・1.2・1.3）: `base` の土台に `overlay` の部品を重ねると、外形は土台の絵の
/// 実寸になり、土台と部品の両方の画素が出て、同じ番号の `surface26.png` は使われない。
///
/// 較正: `surface26.png` を大きさも色も違う絵にしてある。読み手の判定を `overlay` だけに戻すと
/// `element0` が値にならず、層 0 の空いた面へ `surface26.png` が敷かれるので、外形が
/// 20×6 へ動き、`shadowed` から 26 が消えて本テストが赤になる。
#[test]
fn base_element_zero_underlays_the_overlay_part() {
    let text = format!(
        "surface26\n{{\nelement0,base,body.png,0,0\nelement1,overlay,face.png,{x},{y}\n}}\n",
        x = FACE_AT.0,
        y = FACE_AT.1,
    );
    let target = bake(
        &text,
        &["surface26.png", "body.png", "face.png", "surfaces.txt"],
        &[
            ("surface26.png", SURFACE26, COLOR_SURFACE),
            ("body.png", BODY, COLOR_BODY),
            ("face.png", FACE, COLOR_FACE),
        ],
    );
    assert!(
        target.bake_errors().is_empty(),
        "前提: 登録済みの絵だけを焼くので脱落は 0 件: {:?}",
        target.bake_errors()
    );
    assert_ne!(BODY, SURFACE26, "前提: 土台の絵と面の画像は大きさが違う");

    let composed = snapshot(&target, 26).expect("面 26 は合成できる");
    assert_eq!(
        (composed.0, composed.1),
        BODY,
        "外形は `element0,base` の絵の実寸でなければならない（20×6 なら `surface26.png` を敷いている）"
    );
    assert_eq!(
        pixel(&composed, 0, 0),
        COLOR_BODY,
        "部品に覆われない位置に土台の絵が出ていない"
    );
    assert_eq!(
        pixel(&composed, FACE_AT.0, FACE_AT.1),
        COLOR_FACE,
        "部品が土台の上に描かれていない"
    );
    assert_eq!(
        target
            .build_world()
            .base_images()
            .shadowed
            .get(&26)
            .map(String::as_str),
        Some("surface26.png"),
        "`element0` が在るので `surface26.png` は使わなかった側に数えられる"
    );
}

/// ⑵（要件 3.2）: `element0,base,surfaceN.png,0,0` を持つ文面と、そのブレスごと除いた文面
/// （`surface*.png` が土台に敷かれる今の経路）とで、合成した外形と画素が等しい。
///
/// 合成結果の等しさは、読み手の判定を `overlay` だけに戻しても成り立つ。2 つの文面が本当に
/// 別の経路を通っていること（`shadowed` と `used` の内訳の違い）を前提として確かめる
/// （判定を戻すとこの前提で赤になる）。
#[test]
fn base_pointing_at_own_surface_image_looks_the_same_as_the_image_alone() {
    let files = ["surface5.png", "surfaces.txt"];
    let images = [("surface5.png", (6, 9), COLOR_SURFACE)];
    let with_base = bake(
        "surface5\n{\nelement0,base,surface5.png,0,0\n}\n",
        &files,
        &images,
    );
    let image_only = bake("", &files, &images);

    assert!(
        with_base
            .build_world()
            .base_images()
            .shadowed
            .contains_key(&5),
        "前提: `element0` の在る文面では面の画像は使わなかった側"
    );
    assert!(
        image_only.build_world().base_images().used.contains_key(&5),
        "前提: ブレスの無い文面では面の画像を土台に敷く"
    );
    assert_eq!(
        snapshot(&with_base, 5),
        snapshot(&image_only, 5),
        "`element0,base,surface5.png` の面は画像だけの面と同じ絵・同じ大きさでなければならない"
    );
}

/// ⑶（要件 2.4・2.5・3.4）: 描けない `element0,replace,x.png` を持つサーフェスの合成結果は、
/// その行を除いた文面と等しく、`x.png` は焼かれていない（面の画像が土台に敷かれたまま）。
///
/// 不変の檻なので、読み手の判定を `overlay` だけに戻しても緑のままである。`x.png` は復号器に
/// 入れてあるので、焼かれていれば索引表に載る。
#[test]
fn undrawable_element_zero_leaves_the_surface_as_without_the_line() {
    let files = ["surface8.png", "x.png", "face.png", "surfaces.txt"];
    let images = [
        ("surface8.png", (8, 8), COLOR_SURFACE),
        ("x.png", (12, 3), COLOR_X),
        ("face.png", FACE, COLOR_FACE),
    ];
    let with_replace = bake(
        "surface8\n{\nelement0,replace,x.png,0,0\nelement1,overlay,face.png,2,2\n}\n",
        &files,
        &images,
    );
    let without = bake(
        "surface8\n{\nelement1,overlay,face.png,2,2\n}\n",
        &files,
        &images,
    );

    assert_eq!(
        snapshot(&with_replace, 8),
        snapshot(&without, 8),
        "描けない行を持つ面の見え方が、その行を除いた文面と違う"
    );
    assert!(
        with_replace.atlas().resolve(SetId(0), "x.png").is_none(),
        "描けない行の絵が焼かれている"
    );
    assert!(
        with_replace
            .build_world()
            .base_images()
            .used
            .contains_key(&8),
        "描けない `element0` は層 0 を埋めないので、面の画像は土台に敷かれる"
    );
}

/// ⑷（要件 1.7）: 読み込めない `element0,base,missing.png` は、`,overlay,` に書き替えた文面と
/// 同じ脱落の記録（`missing.png` を名指す 1 件）を残し、同じ面の部品は描かれ、合成結果も等しい。
///
/// 記録の `warn!` はこの `bake_errors` を `load_shell_target` が出すので、中身の一致で足りる。
/// 較正として、`missing.png` を復号器に入れると脱落が 0 件になる（＝1 件は `missing.png` の分）。
#[test]
fn unreadable_base_image_is_recorded_like_overlay_and_the_part_is_drawn() {
    let files = ["face.png", "surfaces.txt"];
    let images = [("face.png", FACE, COLOR_FACE)];
    let text_of = |method: &str| {
        format!(
            "surface9\n{{\nelement0,{method},missing.png,0,0\nelement1,overlay,face.png,0,0\n}}\n"
        )
    };
    let base = bake(&text_of("base"), &files, &images);
    let overlay = bake(&text_of("overlay"), &files, &images);

    let errors = bake_errors(&base);
    assert_eq!(errors.len(), 1, "脱落は `missing.png` の 1 件: {errors:?}");
    assert!(
        errors[0].contains("missing.png"),
        "脱落の記録が `missing.png` を名指していない: {errors:?}"
    );
    assert_eq!(
        errors,
        bake_errors(&overlay),
        "`base` の脱落の記録が `overlay` と違う"
    );

    let composed = snapshot(&base, 9);
    assert_eq!(
        composed,
        snapshot(&overlay, 9),
        "読み込めない `base` の面の見え方が `overlay` と違う"
    );
    let composed = composed.expect("部品が残るので面 9 は合成できる");
    assert_eq!(
        pixel(&composed, 0, 0),
        COLOR_FACE,
        "同じ面の部品が描かれていない"
    );

    let mut with_missing = images.to_vec();
    with_missing.push(("missing.png", BODY, COLOR_BODY));
    assert!(
        bake(
            &text_of("base"),
            &["missing.png", "face.png"],
            &with_missing
        )
        .bake_errors()
        .is_empty(),
        "較正: `missing.png` を入れれば脱落は 0 件"
    );
}

// ── 入口の警告（要件 2.1・2.2・2.3・2.6・4.2）──────────────────────────────────────
//
// ここから先は `load_shell_target` を通すので fs に触れる。surfaces.txt は `TempPath` に置き、
// 描ける行が指す絵（`body.png`・`face.png`）はすべて復号器に入れる——焼く段の脱落の `warn!` が
// 混ざると、本 spec の警告の数が数えられなくなるためである。

/// 本 spec の警告の本文。
const UNDRAWN_WARN: &str = "shell: areka が描けない描画メソッドの element定義を描かない";

/// 描ける語の行だけの文面（`balloon` の行には正しい `balloon.*`ブレスを添え、箱の警告を出さない）。
const DRAWABLE_ONLY: &str = concat!(
    "charset,UTF-8\n",
    "balloon.ok\n{\nsize,10,10\n}\n",
    "surface0,1\n{\n",
    "element0,base,body.png,0,0\n",
    "element1,overlay,face.png,1,1\n",
    "element3,balloon,ok,0,0\n",
    "}\n",
);

/// [`DRAWABLE_ONLY`] に描けない行 `element2,replace,x.png` を 1 行足した文面。
const WITH_UNDRAWN: &str = concat!(
    "charset,UTF-8\n",
    "balloon.ok\n{\nsize,10,10\n}\n",
    "surface0,1\n{\n",
    "element0,base,body.png,0,0\n",
    "element1,overlay,face.png,1,1\n",
    "element2,replace,x.png,0,0\n",
    "element3,balloon,ok,0,0\n",
    "}\n",
);

/// `text` を surfaces.txt として置き、`load_shell_target` を `times` 回呼んだ間の記録を返す。
fn load_events(text: &str, times: usize) -> Vec<CapturedEvent> {
    let dir = TempPath::new("shell-target-element-base-warn");
    std::fs::write(dir.child("surfaces.txt"), text).expect("記述ファイル作成");
    write_descript(dir.path());
    let mut dec = MemoryDecoder::new();
    insert_solid(&mut dec, dir.path(), ("body.png", BODY, COLOR_BODY));
    insert_solid(&mut dec, dir.path(), ("face.png", FACE, COLOR_FACE));
    let ((), events) = capture_events(|| {
        for _ in 0..times {
            load_shell_target(dir.path(), &dec).expect("シェルは読める");
        }
    });
    events
}

/// `warn` 以上の記録を「本文|欄=値…」の文字列にする（`message` 以外の欄は名前の昇順）。
fn warn_or_worse(events: &[CapturedEvent]) -> Vec<String> {
    events
        .iter()
        .filter(|e| e.level <= tracing::Level::WARN)
        .map(|e| {
            let mut line = e.message().to_string();
            for (name, value) in e.fields_map() {
                if name != "message" {
                    line.push_str(&format!("|{name}={value}"));
                }
            }
            line
        })
        .collect()
}

/// 本 spec の警告の件数。
fn undrawn_count(events: &[CapturedEvent]) -> usize {
    events
        .iter()
        .filter(|e| e.message() == UNDRAWN_WARN)
        .count()
}

/// 要件 2.1・2.2・2.3: 複数の番号の見出しの下の描けない行 1 行は、見出しの原文・element番号・
/// 語を持つ `warn!` 1 行だけになる（展開した番号の数だけ出ない・ほかの `warn` 以上は 0 行）。
#[test]
fn undrawable_line_warns_once_with_heading_element_and_method() {
    let events = load_events(WITH_UNDRAWN, 1);
    assert_eq!(
        warn_or_worse(&events),
        vec![format!(
            "{UNDRAWN_WARN}|element=\"2\"|heading=\"surface0,1\"|method=\"replace\""
        )]
    );
}

/// 要件 2.6: 描ける語（`overlay`・`base`・`balloon`）だけの文面では `warn` 以上が 0 行。
#[test]
fn drawable_only_shell_warns_nothing() {
    let events = load_events(DRAWABLE_ONLY, 1);
    assert_eq!(warn_or_worse(&events), Vec::<String>::new());
}

/// 要件 4.2: 警告は読み込み 1 回につき 1 行 1 件——2 回読むとちょうど 2 倍になる。
#[test]
fn undrawable_warning_repeats_once_per_load() {
    let once = undrawn_count(&load_events(WITH_UNDRAWN, 1));
    let twice = undrawn_count(&load_events(WITH_UNDRAWN, 2));
    assert_eq!(once, 1, "読み込み 1 回で 1 件");
    assert_eq!(twice, 2 * once, "読み込み 2 回でちょうど 2 倍");
}

// ── 無改変のクローディア（要件 4.3・2.6・3.2）────────────────────────────────────

/// クローディアの surface6・11・26 の外形（土台の絵 `surface0.png`／`surface10.png` の実寸）。
const CLAUDIA_EXTENT: (u32, u32) = (333, 500);

/// 要件 4.3・2.6・3.2: 無改変の `claudia` を DLL なしで実際の画像の復号で読むと、
/// `element0,base` の土台に顔の部品を重ねる surface6・11・26 の外形が 333×500 になり、
/// 本 spec の警告は 0 件（`overlay`・`base` だけで書かれている）。
///
/// 較正: 読み手の判定を `overlay` だけに戻すと土台が値にならず、部品の大きさだけで描かれて
/// surface26 は 100×56（surface11 は 71×42）になって赤になる。
#[test]
fn claudia_base_surfaces_are_full_size_without_warnings() {
    with_com_initialized(|| {
        let dec = WicDecoderArm::new().expect("COM 初期化下で WIC ファクトリが作れる");
        let (target, events) = capture_events(|| {
            load_shell_target(&claudia_shell_dir(), &dec).expect("claudia のシェルは読める")
        });
        // 3 面の外形を並べて比べる（赤のとき、どの面がどの大きさに縮んだかが一度に見える）。
        let extents: Vec<(u32, (u32, u32))> = [6, 11, 26]
            .into_iter()
            .map(|id| {
                let (w, h, _) =
                    snapshot(&target, id).unwrap_or_else(|e| panic!("面 {id} は合成できる: {e}"));
                (id, (w, h))
            })
            .collect();
        assert_eq!(
            extents,
            vec![
                (6, CLAUDIA_EXTENT),
                (11, CLAUDIA_EXTENT),
                (26, CLAUDIA_EXTENT)
            ],
            "`element0,base` の面の外形が土台の絵の実寸でない"
        );
        assert_eq!(undrawn_count(&events), 0, "claudia に描けない行は無い");
    });
}
