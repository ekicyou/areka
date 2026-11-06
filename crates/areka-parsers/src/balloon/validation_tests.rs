//! emo2-kakukaku fixture 適合テスト（タスク 3.1）。
//!
//! 本ファイルは spec の**統合／フィクスチャ適合層**として、公開エントリ
//! `super::parse::parse_str`（`pub fn parse_str(&str, Option<&str>) -> BalloonModel`）を
//! 通した end-to-end アサーションで、emo2-kakukaku 実物 fixture に対する parser 適合を固定する:
//! - R5.1: balloon `descript.txt` 単体 → 幾何＋フォント subset の実物値（`wordwrappoint`/
//!   `validrect`/`font.*`）＋ descript 非搭載の `windowposition` は全 `None`（R3.4）。
//! - R5.2: `descript.txt`＋`balloons0s.txt` → 画像別上書き（`windowposition`/`wordwrappoint.x`/
//!   `validrect`）＋ descript 継承（`font`）（R3.2/R3.3）。
//! - R5.3: `descript.txt`＋`balloonk0s.txt` → 画像別上書き（`windowposition`/`validrect`）＋
//!   descript 継承（`wordwrappoint.x`/`font`・画像別に無いキー）（R3.3）。
//! - 本番と同じ既定 `Ansi` で復号し、`charset,UTF-8` の宣言が効いていること（非 ASCII の値）。
//!
//! fixture は実行時に `std::fs` で**実物ファイル**を読む（クレート跨ぎ `include_str!` 不使用・
//! `charset`/`sakura` validation_tests の流儀）。charset→balloon の実パイプラインを忠実に辿るため
//! `crate::charset::decode` でデコードしてから `parse_str` へ渡す（charset の上流消費であり
//! balloon 境界侵犯ではない）。テストは host 不要・純粋（COM/network 無し・checked-in fixture の file IO のみ）。
//! 期待モデル値はすべて R5.1/R5.2/R5.3 のリテラル直書き。
//!
//! **`origin` について**: fixture の `descript.txt` はかつて `origin.x,0`／`origin.y,0` を宣言して
//! おり、本ファイルはその生値をマージ継承の証拠に使っていた。仕様
//! `areka-P0-balloon-vertical-canon`（要件 3.10・10.9）が正典 ukadoc の「通常は指定せず validrect の
//! 定義に任せる」に合わせて宣言を削除したため、以降 `origin` は**両成分とも未宣言＝`None`**である。
//! 3 テストはこの未宣言形を逐語で固定する（`Some(0)` へ戻る回帰＝宣言の復活を検出する）。マージ継承
//! （R3.3）の証拠は `font.*`（全テスト）と `wordwrappoint.x`（R5.3）が引き続き担う。

use super::parse::parse_str;
use crate::charset::{DefaultEncoding, decode};

/// emo2 が同時にインストールするバルーンの fixture ファイル（検体の窓口から得る）を実物バイトで読み、
/// charset デコードして文字列化する。
///
/// 本番（`areka-emo-present` の balloon の読み）と同じ既定 `Ansi` でデコードする。descript.txt L1 の
/// `charset,UTF-8` 宣言が効いて UTF-8 で読まれる（画像別 .txt は ASCII ゆえ同じく素通り）。
fn load_decoded(file: &str) -> String {
    let path = crate::sample_test_support::emo2_balloon_root().join(file);
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|e| panic!("fixture readable: {}: {e}", path.display()));
    decode(&bytes, DefaultEncoding::Ansi)
}

// ───────────────────────────────────────────────────────────────────
// R5.1 — balloon descript.txt 単体（基層のみ・画像別 None）
// ───────────────────────────────────────────────────────────────────

/// emo2-kakukaku `descript.txt` 単体を `parse_str(_, None)` へ通し、幾何＋フォント subset の
/// 実物値を固定する（R5.1）。加えて descript に `windowposition` 行が無いため両成分 `None`
/// （不在 → `None`・`Some(0)` と判別・R3.4/R5.1）を固定する。`origin` も同様に行が無く両成分
/// `None`（正典推奨形＝宣言しない・`areka-P0-balloon-vertical-canon` 要件 3.10/10.9）。
///
/// 採取元: `descript.txt` L14–L15（wordwrappoint）/L16–L19（validrect）/
/// L25–L26（font.name/height）/L28–L30（font.color）実測 verbatim。
#[test]
fn r5_1_descript_only_pins_geometry_and_font_subset() {
    let descript = load_decoded("descript.txt");
    // `Ansi` 既定でも宣言どおり UTF-8 で読めている（宣言が消えれば化けて落ちる）。
    assert_eq!(
        crate::kv::parse_kv(&descript)
            .get("craftmanw")
            .map(String::as_str),
        Some("えちょ")
    );
    let model = parse_str(&descript, None);

    // origin は descript に行が無い → 両成分 None（正典推奨形＝宣言せず validrect の定義に
    // 任せる・`areka-P0-balloon-vertical-canon` 要件 3.10/10.9・不在は Some(0) と判別）。
    assert_eq!(model.origin().x(), None);
    assert_eq!(model.origin().y(), None);

    // wordwrappoint.x(-34)（descript L14・負値＝反対辺基準を符号保持）／y=Some(0)（L15）。
    assert_eq!(model.wordwrappoint().x(), Some(-34));
    assert_eq!(model.wordwrappoint().y(), Some(0));

    // validrect(0,0,0,0)（descript L16–L19・4 辺すべて Some(0)・None とは判別）。
    assert_eq!(model.validrect().top(), Some(0));
    assert_eq!(model.validrect().bottom(), Some(0));
    assert_eq!(model.validrect().left(), Some(0));
    assert_eq!(model.validrect().right(), Some(0));

    // font.name(Yu Gothic UI)（L25）／height(28)（L26）。
    assert_eq!(model.font().name(), Some("Yu Gothic UI"));
    assert_eq!(model.font().height(), Some(28));

    // font.color(0,0,0)（L28–L30・r/g/b すべて Some(0)）。
    assert_eq!(model.font().color().r(), Some(0));
    assert_eq!(model.font().color().g(), Some(0));
    assert_eq!(model.font().color().b(), Some(0));

    // windowposition は descript に行が無い → 両成分 None（R3.4・不在は Some(0) と判別・R5.1）。
    assert_eq!(model.windowposition().x(), None);
    assert_eq!(model.windowposition().y(), None);
}

// ───────────────────────────────────────────────────────────────────
// R5.2 — descript.txt ＋ balloons0s.txt（さくら側画像別上書き）
// ───────────────────────────────────────────────────────────────────

/// emo2-kakukaku `descript.txt`＋`balloons0s.txt` をマージ解析し、画像別上書き＋descript 継承を
/// 固定する（R5.2）。`windowposition`(266,-129)・`wordwrappoint.x`(-49・画像別が descript の -34 を
/// 上書き・R3.2)・`validrect`(46,-56,36,-44) は画像別由来、`font`（name/height/color の 5 値）は
/// 画像別に無く descript 継承（R3.3）。`origin` はどちらの層にも行が無く両成分 `None`。
///
/// 採取元: `balloons0s.txt` L1–L2（windowposition）/L4（wordwrappoint.x）/L6–L9（validrect）実測 verbatim。
#[test]
fn r5_2_descript_plus_sakura_image_overrides_and_inherits() {
    let descript = load_decoded("descript.txt");
    let image = load_decoded("balloons0s.txt");
    let model = parse_str(&descript, Some(&image));

    // windowposition(266,-129)（balloons0s L1–L2・画像別のみが持つ）。
    assert_eq!(model.windowposition().x(), Some(266));
    assert_eq!(model.windowposition().y(), Some(-129));

    // wordwrappoint.x(-49)（balloons0s L4・画像別が descript の Some(-34) を上書き・後勝ち・R3.2）。
    assert_eq!(model.wordwrappoint().x(), Some(-49));

    // validrect(46,-56,36,-44)（balloons0s L6–L9・画像別由来）。
    assert_eq!(model.validrect().top(), Some(46));
    assert_eq!(model.validrect().bottom(), Some(-56));
    assert_eq!(model.validrect().left(), Some(36));
    assert_eq!(model.validrect().right(), Some(-44));

    // origin はどちらの層にも行が無い → 両成分 None（正典推奨形・要件 3.10/10.9）。
    assert_eq!(model.origin().x(), None);
    assert_eq!(model.origin().y(), None);

    // font（name/height/color）は画像別に無く descript 継承（R3.3・継承の生き証人）。
    assert_eq!(model.font().name(), Some("Yu Gothic UI"));
    assert_eq!(model.font().height(), Some(28));
    assert_eq!(model.font().color().r(), Some(0));
    assert_eq!(model.font().color().g(), Some(0));
    assert_eq!(model.font().color().b(), Some(0));
}

// ───────────────────────────────────────────────────────────────────
// R5.3 — descript.txt ＋ balloonk0s.txt（けろ側画像別上書き）
// ───────────────────────────────────────────────────────────────────

/// emo2-kakukaku `descript.txt`＋`balloonk0s.txt` をマージ解析し、画像別上書き＋descript 継承を
/// 固定する（R5.3）。`windowposition`(-190,-75)・`validrect`(40,-70,24,-48) は画像別由来。
/// `wordwrappoint.x` は画像別 `balloonk0s.txt` に行が無く descript の `Some(-34)` 継承（R3.3）。
/// `font` も descript 継承。`origin` はどちらの層にも行が無く両成分 `None`。
///
/// 採取元: `balloonk0s.txt` L1–L2（windowposition）/L4–L7（validrect）実測 verbatim。
/// `balloonk0s.txt` に `wordwrappoint.*` 行が無いことが descript 継承の生き証人（R3.3・R5.3）。
#[test]
fn r5_3_descript_plus_kero_image_inherits_wordwrappoint() {
    let descript = load_decoded("descript.txt");
    let image = load_decoded("balloonk0s.txt");
    let model = parse_str(&descript, Some(&image));

    // windowposition(-190,-75)（balloonk0s L1–L2・画像別のみが持つ）。
    assert_eq!(model.windowposition().x(), Some(-190));
    assert_eq!(model.windowposition().y(), Some(-75));

    // validrect(40,-70,24,-48)（balloonk0s L4–L7・画像別由来）。
    assert_eq!(model.validrect().top(), Some(40));
    assert_eq!(model.validrect().bottom(), Some(-70));
    assert_eq!(model.validrect().left(), Some(24));
    assert_eq!(model.validrect().right(), Some(-48));

    // wordwrappoint.x は画像別に無く descript の Some(-34) 継承（R3.3・R5.3）。
    assert_eq!(model.wordwrappoint().x(), Some(-34));

    // origin はどちらの層にも行が無い → 両成分 None（正典推奨形・要件 3.10/10.9）。
    assert_eq!(model.origin().x(), None);
    assert_eq!(model.origin().y(), None);

    // font は画像別に無く descript 継承（R3.3・継承の生き証人）。
    assert_eq!(model.font().name(), Some("Yu Gothic UI"));
    assert_eq!(model.font().height(), Some(28));
    assert_eq!(model.font().color().r(), Some(0));
    assert_eq!(model.font().color().g(), Some(0));
    assert_eq!(model.font().color().b(), Some(0));
}
