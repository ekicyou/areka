//! # vertical_fixture_test — 縦書き観測用 fixture 変種の読み込み検証（task 9.1・R11.4）
//!
//! example ローカル fixture 変種 `examples/fixtures/emo2-vertical/` を実ファイルから
//! 読み込み、以下を檻化する（design.md「File Structure Plan」・「観測」縦横切替が正典）:
//!
//! - **writing_mode の縦書き解決**: descript 基層の `writing_mode,vertical_rl` 宣言が
//!   [`WritingMode::VerticalRl`] へ解決される（R11.4）。
//! - **折返し閾値の非退化**: 縦書きの折返し軸は `wordwrappoint.y`（軸読み替え正準表）。
//!   共有 fixture は `wordwrappoint.y,0`＝縦書き折返しが退化するため（design.md
//!   「軸読み替え正準表」補足）、変種は非ゼロ・有効領域内で意味のある折返しが起きる
//!   値を与える。
//! - **画像別上書き層の実観測**: descript 基層＜ `balloons0s.txt`（後勝ち）の 2 層
//!   マージが変種でも有効である（R11.4・2層マージの実観測）。
//!
//! バルーン枠画像は共有 fixture（`crates/pilot/examples/shiori-host-32/fixtures/emo2/
//! emo2-kakukaku/balloons0.png`・400×224 image px）を再利用し、変種が差し替えるのは
//! balloon descript の parse 入力（descript.txt＋balloons0s.txt）だけ——本テストの
//! 画像原寸定数はその共有枠画像の実測原寸である。
//!
//! ## 正典キー版フィクスチャ `emo2-vertical-canon`（areka-P0-balloon-vertical-canon task 5.1）
//!
//! 上記の拡張キー版に加え、SSP 正典キー `vertical,1` を宣言するだけが異なる第 2 の変種
//! `examples/fixtures/emo2-vertical-canon/` を読み、**両版が同一の [`WritingMode`] と
//! 同一の [`TextRegion`]（全成分）を与える**ことを檻にする（areka-P0-balloon-vertical-canon
//! 要件 10.1／10.2・design.md「C6 縦書きフィクスチャ 2 種」／DD9）。
//! あわせて、両版の `descript.txt` の差分が **`writing_mode,vertical_rl` → `vertical,1` の
//! 1 行だけ**であること・`balloons0s.txt` が同内容であること・**両版とも `origin` を
//! 宣言しない**こと（正典推奨形・要件 10.9）を機械で固定する（人の目視に頼らない）。

use std::path::PathBuf;

use areka_emo_text::region::TextRegion;
use areka_emo_text::writing::WritingMode;
use areka_parsers::balloon::{BalloonModel, parse_str};
use areka_parsers::charset::{DefaultEncoding, decode};

/// 共有 fixture のバルーン枠画像 balloons0.png の実測原寸（image px・再利用・非改変）。
const FIXTURE_IMAGE_SIZE: (u32, u32) = (400, 224);

/// fixture 変種の配置（design.md File Structure Plan 正典:
/// `examples/fixtures/emo2-vertical/`——task 9.2 の example が起動引数で切り替える参照先）。
fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("fixtures")
        .join("emo2-vertical")
}

/// fixture 変種ファイルを読み、charset 宣言（`charset,UTF-8`）に従いデコードする
/// （parser-foundation の decode 経路＝example 9.2 と同じ読み込み規約）。
fn read_decoded(name: &str) -> String {
    let path = fixture_dir().join(name);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "fixture 変種 {} の読取に失敗した（task 9.1 の観測 fixture が必要）: {e}",
            path.display()
        )
    });
    decode(&bytes, DefaultEncoding::Utf8)
}

/// descript 基層のみ（画像別上書き層なし）の BalloonModel。
fn base_model() -> BalloonModel {
    parse_str(&read_decoded("descript.txt"), None)
}

/// 2 層マージ済み（descript 基層＋balloons0s.txt 画像別上書き層・後勝ち）の BalloonModel。
fn merged_model() -> BalloonModel {
    parse_str(
        &read_decoded("descript.txt"),
        Some(&read_decoded("balloons0s.txt")),
    )
}

// ── R11.4: writing_mode が縦書きへ解決される ──

/// 縦書き宣言は descript 基層が持つ（design.md L160 正典: `writing_mode,vertical_rl`）——
/// 基層のみでも 2 層マージ後でも `VerticalRl` へ解決される。
#[test]
fn variant_writing_mode_resolves_to_vertical_rl() {
    assert_eq!(
        WritingMode::resolve(&base_model()),
        WritingMode::VerticalRl,
        "descript 基層の writing_mode 宣言が縦書きへ解決される"
    );
    assert_eq!(
        WritingMode::resolve(&merged_model()),
        WritingMode::VerticalRl,
        "画像別上書き層を重ねても縦書き解決が保たれる"
    );
}

// ── R11.4: 縦書きの折返し閾値が退化しない ──

/// 2 層マージ後の変種を共有枠画像原寸で解決すると、縦書きの折返し閾値
/// （`wordwrappoint.y` 軸・負値=下辺基準）が非ゼロかつ有効領域内の値になり、
/// validrect も非退化である（共有 fixture の `wordwrappoint.y,0` 退化の解消）。
#[test]
fn variant_vertical_wrap_threshold_is_nondegenerate() {
    let merged = merged_model();
    let mode = WritingMode::resolve(&merged);
    let region = TextRegion::resolve(&merged, FIXTURE_IMAGE_SIZE, mode);

    // validrect は非退化（共有 fixture 由来の上書き値 top46/bottom-56/left36/right-44）。
    assert_eq!(
        (region.left(), region.top(), region.right(), region.bottom()),
        (36.0, 46.0, 356.0, 168.0),
        "validrect が画像座標空間の非退化矩形へ解決される"
    );

    // 折返し閾値: balloons0s.txt の wordwrappoint.y,-60（負値=下辺基準）→ 224-60=164。
    assert_eq!(region.wrap_threshold(), 164.0);

    // 非退化の述語: 非ゼロ・書字開始角（top）より先・有効領域内（bottom 以下）
    // ＝行内軸（+y）で意味のある折返しが起きる値。
    assert!(region.wrap_threshold() != 0.0, "閾値が非ゼロである");
    assert!(
        region.wrap_threshold() > region.top(),
        "閾値が書字開始側（top）より先にあり、行内に文字を置く余地がある"
    );
    assert!(
        region.wrap_threshold() <= region.bottom(),
        "閾値が有効領域内（bottom 以下）にある"
    );

    // vertical_rl の書字開始角は validrect 右上（軸読み替え正準表）——変種でも成立。
    assert_eq!(region.start(), (356.0, 46.0));
}

/// 基層だけでも折返し閾値は退化しない（`wordwrappoint.y,150`＝非ゼロ）——上書き層の
/// 欠落・不読でも縦書き観測が全損しない fixture 設計であることの檻。
#[test]
fn base_layer_alone_has_nondegenerate_vertical_threshold() {
    let base = base_model();
    assert_eq!(
        base.wordwrappoint().y(),
        Some(150),
        "descript 基層が有意な（非ゼロ）wordwrappoint.y を持つ"
    );
}

// ── R11.4: 画像別上書き層（後勝ち）の実観測 ──

/// 画像別上書き層 `balloons0s.txt` が descript 基層の値を後勝ちで上書きする——
/// 2 層マージの実観測（基層 150 → 上書き -60・値が異なることでマージ経路を証明）。
#[test]
fn image_override_layer_wins_for_wrap_threshold() {
    let base = base_model();
    let merged = merged_model();
    assert_eq!(base.wordwrappoint().y(), Some(150), "基層の値");
    assert_eq!(
        merged.wordwrappoint().y(),
        Some(-60),
        "画像別上書き層の値が後勝ちで有効になる"
    );
    assert_ne!(
        base.wordwrappoint().y(),
        merged.wordwrappoint().y(),
        "基層と上書き層の値が異なることで 2 層マージが実観測できる"
    );
}

// ── areka-P0-balloon-vertical-canon task 5.1: 正典キー版フィクスチャと拡張キー版の同値 ──

/// 拡張キー版（`writing_mode,vertical_rl`）フィクスチャのディレクトリ名。
const EXT_KEY_FIXTURE: &str = "emo2-vertical";
/// 正典キー版（`vertical,1`）フィクスチャのディレクトリ名。
const CANON_KEY_FIXTURE: &str = "emo2-vertical-canon";

/// 名前でフィクスチャ変種のディレクトリを引く（既存 `fixture_dir` の 2 変種一般化）。
fn variant_dir(variant: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("fixtures")
        .join(variant)
}

/// 指定変種のファイルを生バイトで読む——**存在しなければ panic する**
/// （フィクスチャ不在が「対象 0 件だから緑」へ化けない規律・既存 `read_decoded` と同じ）。
fn read_variant_bytes(variant: &str, name: &str) -> Vec<u8> {
    let path = variant_dir(variant).join(name);
    std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "フィクスチャ {} の読取に失敗した（areka-P0-balloon-vertical-canon task 5.1 の同値檻には両変種の実ファイルが要る）: {e}",
            path.display()
        )
    })
}

/// 指定変種のファイルを charset 宣言（`charset,UTF-8`）に従いデコードする。
fn read_variant_decoded(variant: &str, name: &str) -> String {
    decode(&read_variant_bytes(variant, name), DefaultEncoding::Utf8)
}

/// 指定変種の descript 基層のみ（画像別上書き層なし）の BalloonModel。
fn variant_base_model(variant: &str) -> BalloonModel {
    parse_str(&read_variant_decoded(variant, "descript.txt"), None)
}

/// 指定変種の 2 層マージ済み（descript 基層＋`balloons0s.txt` 後勝ち）の BalloonModel。
fn variant_merged_model(variant: &str) -> BalloonModel {
    parse_str(
        &read_variant_decoded(variant, "descript.txt"),
        Some(&read_variant_decoded(variant, "balloons0s.txt")),
    )
}

/// 両変種のフィクスチャ 2 ファイルが実在し、空でないこと——同値檻・構造檻が
/// 「読めなかったので比較 0 件」で緑になる経路を塞ぐ（読取失敗はヘルパが panic する）。
#[test]
fn canon_and_extension_fixture_files_exist_and_are_non_empty() {
    for variant in [EXT_KEY_FIXTURE, CANON_KEY_FIXTURE] {
        for name in ["descript.txt", "balloons0s.txt"] {
            let bytes = read_variant_bytes(variant, name);
            assert!(
                !bytes.is_empty(),
                "{variant}/{name} が空である（フィクスチャの実在が同値檻の前提）"
            );
        }
    }
}

/// 要件 10.1: 正典キー `vertical,1` を宣言するフィクスチャが縦書きへ解決される——
/// descript 基層のみでも、`balloons0s.txt` を重ねた 2 層マージ後でも `VerticalRl`。
#[test]
fn canon_key_fixture_resolves_to_vertical_rl() {
    assert_eq!(
        variant_base_model(CANON_KEY_FIXTURE).vertical_raw(),
        Some("1"),
        "正典キー版の descript 基層が vertical,1 を宣言している"
    );
    assert_eq!(
        WritingMode::resolve(&variant_base_model(CANON_KEY_FIXTURE)),
        WritingMode::VerticalRl,
        "正典キーの宣言だけで（基層のみでも）縦書きへ解決される"
    );
    assert_eq!(
        WritingMode::resolve(&variant_merged_model(CANON_KEY_FIXTURE)),
        WritingMode::VerticalRl,
        "画像別上書き層を重ねても正典キーの縦書き解決が保たれる"
    );
}

/// 要件 10.2: 正典キー版と拡張キー版の `WritingMode` が一致する（基層のみ／2 層マージ後）。
#[test]
fn canon_and_extension_fixtures_agree_on_writing_mode() {
    assert_eq!(
        WritingMode::resolve(&variant_base_model(CANON_KEY_FIXTURE)),
        WritingMode::resolve(&variant_base_model(EXT_KEY_FIXTURE)),
        "基層のみで両版の書字方向が一致する"
    );
    assert_eq!(
        WritingMode::resolve(&variant_merged_model(CANON_KEY_FIXTURE)),
        WritingMode::resolve(&variant_merged_model(EXT_KEY_FIXTURE)),
        "2 層マージ後も両版の書字方向が一致する"
    );
}

/// 要件 10.2: 両版の `TextRegion` が**全成分で逐語一致**する（同一の表示結果の実観測）。
#[test]
fn canon_and_extension_fixtures_agree_on_text_region() {
    let canon_merged = variant_merged_model(CANON_KEY_FIXTURE);
    let ext_merged = variant_merged_model(EXT_KEY_FIXTURE);
    let canon = TextRegion::resolve(
        &canon_merged,
        FIXTURE_IMAGE_SIZE,
        WritingMode::resolve(&canon_merged),
    );
    let ext = TextRegion::resolve(
        &ext_merged,
        FIXTURE_IMAGE_SIZE,
        WritingMode::resolve(&ext_merged),
    );

    assert_eq!(canon.left(), ext.left(), "left が一致する");
    assert_eq!(canon.top(), ext.top(), "top が一致する");
    assert_eq!(canon.right(), ext.right(), "right が一致する");
    assert_eq!(canon.bottom(), ext.bottom(), "bottom が一致する");
    assert_eq!(canon.start(), ext.start(), "書字開始角が一致する");
    assert_eq!(
        canon.wrap_threshold(),
        ext.wrap_threshold(),
        "折返し閾値が一致する"
    );
    assert_eq!(canon, ext, "TextRegion 全体が一致する");
}

/// 要件 10.1／10.2: 正典キー版の解決後 `TextRegion` を design.md「Data Models →
/// `emo2-vertical-canon` フィクスチャのデータ形」の表どおりに逐語固定する
/// （left 36／top 46／right 356／bottom 168／start (356,46)／wrap 224-60=164）。
#[test]
fn canon_fixture_text_region_matches_design_data_model() {
    let merged = variant_merged_model(CANON_KEY_FIXTURE);
    let region = TextRegion::resolve(&merged, FIXTURE_IMAGE_SIZE, WritingMode::resolve(&merged));

    assert_eq!(
        (region.left(), region.top(), region.right(), region.bottom()),
        (36.0, 46.0, 356.0, 168.0),
        "validrect が画像座標空間の非退化矩形へ解決される"
    );
    assert_eq!(
        region.start(),
        (356.0, 46.0),
        "vertical_rl の書字開始角は validrect 右上"
    );
    assert_eq!(
        region.wrap_threshold(),
        164.0,
        "wordwrappoint.y,-60（負値=下辺基準）→ 画像高 224 - 60 = 164"
    );
}

/// 要件 10.9: 両版とも `origin` を宣言しない（正典推奨形＝「通常は指定せず validrect の
/// 定義に任せる」）——基層・2 層マージ後のいずれでも `origin.x`／`origin.y` が未宣言。
#[test]
fn neither_fixture_declares_origin() {
    for variant in [EXT_KEY_FIXTURE, CANON_KEY_FIXTURE] {
        for (label, model) in [
            ("基層のみ", variant_base_model(variant)),
            ("2 層マージ後", variant_merged_model(variant)),
        ] {
            assert_eq!(
                model.origin().x(),
                None,
                "{variant}（{label}）が origin.x を宣言していない"
            );
            assert_eq!(
                model.origin().y(),
                None,
                "{variant}（{label}）が origin.y を宣言していない"
            );
        }
    }
}

/// DD9 の「差分は 1 行だけ」を人の目視でなく機械で守る構造檻——2 つの `descript.txt` を
/// 行単位で比較し、**異なる行がちょうど 1 組**で、それが
/// `writing_mode,vertical_rl` → `vertical,1` であることを固定する。
/// `balloons0s.txt` は全行一致（同内容）であることも固定する。
#[test]
fn canon_descript_differs_from_extension_by_exactly_one_line() {
    let ext = read_variant_decoded(EXT_KEY_FIXTURE, "descript.txt");
    let canon = read_variant_decoded(CANON_KEY_FIXTURE, "descript.txt");
    let ext_lines: Vec<&str> = ext.lines().collect();
    let canon_lines: Vec<&str> = canon.lines().collect();

    assert_eq!(
        ext_lines.len(),
        canon_lines.len(),
        "descript.txt の行数が一致する（差分は 1 行の置換だけ・行の増減は許さない）"
    );

    let diffs: Vec<(usize, &str, &str)> = ext_lines
        .iter()
        .zip(canon_lines.iter())
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(i, (a, b))| (i, *a, *b))
        .collect();

    assert_eq!(
        diffs.len(),
        1,
        "2 つの descript.txt の相違はちょうど 1 行である（実際の相違: {diffs:?}）"
    );
    assert_eq!(
        (diffs[0].1, diffs[0].2),
        ("writing_mode,vertical_rl", "vertical,1"),
        "唯一の相違は拡張キー宣言から正典キー宣言への置換である"
    );

    let ext_overlay = read_variant_bytes(EXT_KEY_FIXTURE, "balloons0s.txt");
    let canon_overlay = read_variant_bytes(CANON_KEY_FIXTURE, "balloons0s.txt");
    assert_eq!(
        ext_overlay, canon_overlay,
        "画像別上書き層 balloons0s.txt は両版で同内容である"
    );
}
