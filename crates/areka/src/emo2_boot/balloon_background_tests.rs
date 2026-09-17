// =============================================================================
// `face_origin_color` の兄弟テスト（要件 4.6・design「emo2_boot/balloon_background.rs＋配線」）
//
// `MemoryDecoder`＋`bake` で **本番と同じ焼き込み経路**を通した 3 つの画像（不透明・半透明・
// トリム付き）を作り、それぞれ「原点画素の色」「白」「白」になることを固定する。加えて面が
// 引けないときも白になること、白へ落ちた各経路が必ず 1 件の記録を残すこと（記録なしの失敗経路
// を作らない）を固定する。
//
// 較正の要点（この 2 つを取り違えた実装が緑で通らないこと）:
//  ⑴ 半透明の画像は原点画素に **白ではない色**を置いてある。α を見ずに画素の色をそのまま採る
//     実装は (36,24,12) を返して赤になる。
//  ⑵ トリム付きの画像は bbox の左上に **白ではない色**を置いてある。`trim_offset` を見ずに
//     アトラス頁の uv 原点を読む実装は (30,45,60) を返して赤になる。
//
// ログ捕捉は硬化機構の唯一の定義元 `log-capture-kit` の捕捉窓へ委譲する。捕捉が空振りしていない
// ことは、同じ捕捉窓の中でわざと 1 件出す番兵で確かめる（「0 件」の主張が恒真にならないように）。
// =============================================================================

use super::*;

use areka_emo_atlas::{
    AlphaParams, AtlasTable, MemoryDecoder, PackConfig, SurfaceSet, UseSelfAlpha, bake,
};
use areka_parsers::shell::{AppendTarget, Element, ElementPath, Surface};
use log_capture_kit::{LineFormat, capture_lines};
use std::path::Path;

/// 合成バルーンディレクトリ（`MemoryDecoder` は実 I/O をしないので実在しなくてよい）。
const BASE: &str = "balloon/decoration-test";

/// 不透明な面。原点画素だけ他と違う色を置く（「別の画素を読む」実装を赤にするため）。
const OPAQUE: &str = "opaque0.png";
/// 原点画素だけ半透明な面（残りは不透明ゆえトリムは走らない＝`trim_offset` は (0,0)）。
const SEMI: &str = "semi0.png";
/// 原点を含む 3 画素が全透明な面（bbox は右下 1 画素・`trim_offset` は (1,1)）。
const TRIMMED: &str = "trimmed0.png";

/// 不透明画像の原点画素の色（BGRA 順で置き、期待値は RGB 順）。
const OPAQUE_ORIGIN_RGB: (u8, u8, u8) = (10, 20, 30);
/// 半透明画像の原点画素を「α を見ずに」読んだときに出る色（較正用の誤り値）。
const SEMI_ORIGIN_RGB_IF_ALPHA_IGNORED: (u8, u8, u8) = (36, 24, 12);
/// トリム画像の bbox 左上を「`trim_offset` を見ずに」読んだときに出る色（較正用の誤り値）。
const TRIMMED_BBOX_RGB_IF_TRIM_IGNORED: (u8, u8, u8) = (30, 45, 60);

/// 白（`DEFAULT_BALLOON_BACKGROUND` の実値と一致することは下のテストが別に固定する）。
const WHITE: (u8, u8, u8) = (255, 255, 255);

/// 1 element だけの surface（バルーン面の合成 surfaces.txt と同型）。
fn surface(id: u32, rel: &str) -> Surface {
    Surface {
        id,
        targets: vec![AppendTarget::Single(id)],
        elements: vec![Element {
            layer: 0,
            path: ElementPath::new(rel.to_string()),
            x: 0,
            y: 0,
        }],
        collisions: Vec::new(),
        animations: Vec::new(),
    }
}

/// 3 画像を **1 度の `bake`** で焼いたアトラスを組む（本番 `build_balloon_target_from_faces` と
/// 同じ `UseSelfAlpha::On`／`PackConfig::default()`）。
fn baked_atlas() -> AtlasTable {
    let base = Path::new(BASE);

    // 2×2 premultiplied BGRA。B,G,R <= A（premultiplied 適合）。
    // 不透明: 原点 (0,0) は BGRA=(30,20,10,255)＝RGB(10,20,30)。他 3 画素は別色。
    let opaque = vec![
        30, 20, 10, 255, //
        200, 200, 200, 255, //
        200, 200, 200, 255, //
        200, 200, 200, 255,
    ];
    // 半透明: 原点 (0,0) は BGRA=(12,24,36,128)＝α を無視すると RGB(36,24,12)。
    // 他 3 画素は α=255 ゆえ bbox は全域＝`trim_offset` は (0,0)（トリム経路と混ざらない）。
    let semi = vec![
        12, 24, 36, 128, //
        200, 200, 200, 255, //
        200, 200, 200, 255, //
        200, 200, 200, 255,
    ];
    // トリム付き: (0,0)/(1,0)/(0,1) は α=0、(1,1) だけ BGRA=(60,45,30,255)＝RGB(30,45,60)。
    // bbox は (1,1)-(1,1) ゆえ `trim_offset` は (1,1) で、原点は bbox の外。
    let trimmed = vec![
        0, 0, 0, 0, //
        0, 0, 0, 0, //
        0, 0, 0, 0, //
        60, 45, 30, 255,
    ];

    let mut dec = MemoryDecoder::new();
    for (rel, bgra) in [(OPAQUE, opaque), (SEMI, semi), (TRIMMED, trimmed)] {
        dec.insert(base.join(rel), 2, 2, 8, bgra, true);
    }

    let surfaces = vec![surface(0, OPAQUE), surface(1, SEMI), surface(2, TRIMMED)];
    let set = SurfaceSet {
        surfaces: &surfaces,
        base_dir: base,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let baked = bake(&[set], &dec, PackConfig::default());
    assert!(
        baked.errors.is_empty(),
        "テスト前提: 3 画像とも bake に成功する: {:?}",
        baked.errors
    );
    baked.table
}

/// 捕捉行から本番モジュールが出した `reason=` の値だけを取り出す（番兵は `reason` を持たない）。
fn fallback_reasons(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .filter_map(|line| line.split(" reason=").nth(1))
        .map(|rest| rest.split(' ').next().unwrap_or_default().to_string())
        .collect()
}

/// 捕捉窓の中でわざと 1 件出す番兵（捕捉が空振りしていないことの対照）。
fn sentinel() {
    tracing::debug!(sentinel = true, "balloon_background_tests: 番兵");
}

/// 番兵が捕捉窓に載っている件数。
fn sentinel_count(lines: &[String]) -> usize {
    lines.iter().filter(|l| l.contains("sentinel=true")).count()
}

/// ① 不透明: 原点画素の色をそのまま採る。記録は 0 件（番兵で捕捉の生存を確かめたうえでの 0）。
#[test]
fn opaque_face_adopts_the_origin_pixel_without_any_record() {
    let atlas = baked_atlas();
    let (color, lines) = capture_lines(LineFormat::LevelTargetFields, || {
        let color = face_origin_color(&atlas, OPAQUE);
        sentinel();
        color
    });

    assert_eq!(
        color, OPAQUE_ORIGIN_RGB,
        "不透明な面 0 の原点画素 BGRA=(30,20,10,255) は RGB(10,20,30) として採る: {lines:?}"
    );
    assert_eq!(
        sentinel_count(&lines),
        1,
        "番兵が捕捉されていない＝捕捉窓が空振りしており下の 0 件は恒真: {lines:?}"
    );
    assert_eq!(
        fallback_reasons(&lines),
        Vec::<String>::new(),
        "正常に採れた経路は白へ落ちないので記録も出さない: {lines:?}"
    );
}

/// ② 半透明: α が 255 でないので白へ落ち、理由 1 件を残す。
///
/// 較正: α を見ない実装は原点画素の色（RGB(36,24,12)）を返して赤になる。
#[test]
fn semi_transparent_origin_falls_back_to_white_with_one_record() {
    let atlas = baked_atlas();
    let (color, lines) = capture_lines(LineFormat::LevelTargetFields, || {
        let color = face_origin_color(&atlas, SEMI);
        sentinel();
        color
    });

    assert_ne!(
        color, SEMI_ORIGIN_RGB_IF_ALPHA_IGNORED,
        "α を見ずに画素の色をそのまま採ってはならない（半透明は背景色として使えない）: {lines:?}"
    );
    assert_eq!(color, WHITE, "半透明の原点画素は白へ落とす: {lines:?}");
    assert_eq!(
        sentinel_count(&lines),
        1,
        "捕捉窓が空振りしていないこと: {lines:?}"
    );
    assert_eq!(
        fallback_reasons(&lines),
        vec![String::from("\"not_opaque\"")],
        "白へ落ちた理由を 1 件だけ残す（記録なしの失敗経路を作らない）: {lines:?}"
    );
}

/// ③ トリム付き: 原点が bbox の外なので白へ落ち、理由 1 件を残す。
///
/// 較正: `trim_offset` を見ずにアトラス頁の uv 原点を読む実装は RGB(30,45,60) を返して赤になる。
#[test]
fn trimmed_away_origin_falls_back_to_white_with_one_record() {
    let atlas = baked_atlas();
    let (color, lines) = capture_lines(LineFormat::LevelTargetFields, || {
        let color = face_origin_color(&atlas, TRIMMED);
        sentinel();
        color
    });

    assert_ne!(
        color, TRIMMED_BBOX_RGB_IF_TRIM_IGNORED,
        "トリムで原点が bbox の外に出たとき、bbox 左上の別画素を読んではならない: {lines:?}"
    );
    assert_eq!(
        color, WHITE,
        "原点が bbox の外なら白へ落とす（原点は透明だった）: {lines:?}"
    );
    assert_eq!(
        sentinel_count(&lines),
        1,
        "捕捉窓が空振りしていないこと: {lines:?}"
    );
    assert_eq!(
        fallback_reasons(&lines),
        vec![String::from("\"origin_trimmed_away\"")],
        "白へ落ちた理由を 1 件だけ残す: {lines:?}"
    );
}

/// ④ 面が引けない: アトラスに無いファイル名は白へ落ち、理由 1 件を残す。宛先も固定する。
#[test]
fn missing_face_falls_back_to_white_with_one_record() {
    let atlas = baked_atlas();
    let (color, lines) = capture_lines(LineFormat::LevelTargetFields, || {
        let color = face_origin_color(&atlas, "no-such-face.png");
        sentinel();
        color
    });

    assert_eq!(color, WHITE, "引けない面は白へ落とす: {lines:?}");
    assert_eq!(
        sentinel_count(&lines),
        1,
        "捕捉窓が空振りしていないこと: {lines:?}"
    );
    assert_eq!(
        fallback_reasons(&lines),
        vec![String::from("\"face_missing\"")],
        "白へ落ちた理由を 1 件だけ残す: {lines:?}"
    );
    assert_eq!(
        lines
            .iter()
            .filter(|l| l.contains("target=areka::emo2_boot::balloon_background "))
            .count(),
        1,
        "記録の宛先は本モジュール（番兵の `::tests` と混ざっていない）: {lines:?}"
    );
}

/// ⑤ 全透明の面は焼かれない（`placement` が `None`）ので白へ落ち、理由 1 件を残す。
#[test]
fn fully_transparent_face_falls_back_to_white_with_one_record() {
    let base = Path::new(BASE);
    let mut dec = MemoryDecoder::new();
    dec.insert(base.join(OPAQUE), 2, 2, 8, vec![0u8; 16], true);
    let surfaces = vec![surface(0, OPAQUE)];
    let set = SurfaceSet {
        surfaces: &surfaces,
        base_dir: base,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let atlas = bake(&[set], &dec, PackConfig::default()).table;

    let (color, lines) = capture_lines(LineFormat::LevelTargetFields, || {
        let color = face_origin_color(&atlas, OPAQUE);
        sentinel();
        color
    });

    assert_eq!(color, WHITE, "全透明の面は白へ落とす: {lines:?}");
    assert_eq!(
        sentinel_count(&lines),
        1,
        "捕捉窓が空振りしていないこと: {lines:?}"
    );
    assert_eq!(
        fallback_reasons(&lines),
        vec![String::from("\"empty_entry\"")],
        "白へ落ちた理由を 1 件だけ残す: {lines:?}"
    );
}

/// 落とし先の白は文字レンダリング層の既定と同じ 1 定数（二重定義の陳腐化を塞ぐ・印字でなく判定）。
#[test]
fn the_fallback_white_is_the_text_layers_default_background() {
    assert_eq!(
        DEFAULT_BALLOON_BACKGROUND, WHITE,
        "落とし先は `areka_emo_text::draw::DEFAULT_BALLOON_BACKGROUND`（白）と同じ値"
    );
}
