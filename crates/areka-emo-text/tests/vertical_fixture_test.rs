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
