//! 横断バリデーション単体テスト（タスク 4.2）。
//!
//! 本ファイルは spec の**統合／バリデーション層**として、公開エントリ
//! `super::resolve`（`pub fn resolve(&Path, DefaultEncoding) -> Result<MountModel, MountError>`）
//! を、**実フィクスチャ emo2** を入力に end-to-end で解決させ、観測可能挙動を固定する。
//!
//! resolve_tests.rs は合成 tempdir によるユニット網羅（存在確認・フォールバック・
//! 失敗型）を担うのに対し、本ファイルは別クレート（`pilot`）配下の実配布形フィクスチャ
//! `crates/pilot/examples/shiori-host-32/fixtures/emo2/` を「そのまま」入力にした
//! クレート跨ぎの回帰固定を担う。foundation 経由の charset デコード（UTF-8）・既定
//! フォールバック（shell=master）・未使用フィールドの無影響を、実バイト列で確認する。
//!
//! フィクスチャ所在: `env!("CARGO_MANIFEST_DIR")`（= `.../crates/areka-parsers`）から
//! `..` で `crates/` へ上がり `pilot/...` へ降りて合成する（design「Implementation
//! Notes → Validation」準拠・絶対パス埋め込みやフィクスチャ複製をしない）。

use crate::charset::DefaultEncoding;

use super::{MountModel, resolve};
use std::path::{Path, PathBuf};

/// emo2 実フィクスチャのルート（`crates/pilot/examples/shiori-host-32/fixtures/emo2/`）。
///
/// `CARGO_MANIFEST_DIR` 相対で組み立てる（クロスプラットフォーム・`Path::join` のみ）。
fn emo2_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("pilot")
        .join("examples")
        .join("shiori-host-32")
        .join("fixtures")
        .join("emo2")
}

/// emo2 を解決して `MountModel` を得るヘルパ。
///
/// 既定エンコーディングは `Utf8` を渡すが、emo2 の descript.txt は先頭で
/// `charset,UTF-8` を宣言するため、foundation の prescan が既定に関わらず UTF-8 を
/// 選ぶ（＝結果は default_encoding に不変）。ここで `Utf8` を渡すのは宣言と一致させ
/// 意図を明示するためで、`Ansi` を渡しても同一結果になる（宣言優先・メモ参照）。
fn resolve_emo2() -> MountModel {
    let root = emo2_root();
    match resolve(&root, DefaultEncoding::Utf8) {
        Ok(model) => model,
        Err(e) => panic!("emo2 の解決に失敗: {e:?}（root={root:?}）"),
    }
}

// ───────────────────────────────────────────────────────────────────
// レイアウト解決（要件 4.1/4.3/5.2）
//
// 採取元: crates/pilot/examples/shiori-host-32/fixtures/emo2/
//   - ghost/master/descript.txt L7: `shiori,pasta.dll` → shiori.file = "pasta.dll"
//   - ghost/master/                → shiori.dir = <root>/ghost/master（起点の親）
//   - shell/master/（実在）        → shell.dir = <root>/shell/master
//     （descript.txt に `seriko.defaultsurfacedirectoryname` 不在 → 既定 master
//      へフォールバック・要件 3.1/3.2）
// ───────────────────────────────────────────────────────────────────

/// emo2 レイアウトが 2 点マウント（SHIORI / shell）へ正しく解決される。
/// SHIORI ファイル名＝`pasta.dll`・SHIORI dir＝`ghost/master`・shell dir＝既定
/// `shell/master`（`seriko.defaultsurfacedirectoryname` 不在のためフォールバック）。
#[test]
fn emo2_resolves_shiori_and_shell_layout() {
    let root = emo2_root();
    let model = resolve_emo2();

    // SHIORI ファイル名は descript の `shiori,pasta.dll` から。
    assert_eq!(model.shiori.file, Some("pasta.dll".to_string()));

    // SHIORI dir は起点 descript.txt の親 = <root>/ghost/master。
    let expected_shiori_dir = root.join("ghost").join("master");
    assert_eq!(model.shiori.dir, expected_shiori_dir);

    // shell dir は既定 master へフォールバック = <root>/shell/master。
    let expected_shell_dir = root.join("shell").join("master");
    assert_eq!(model.shell.dir, expected_shell_dir);
}

// ───────────────────────────────────────────────────────────────────
// 名前情報の UTF-8 取得（要件 1.4/1.5）
//
// 採取元: crates/pilot/examples/shiori-host-32/fixtures/emo2/ghost/master/descript.txt
//   L1: `charset,UTF-8`
//   L4: `name,えも？？`      （バイト: e38188 e38282 efbc9f efbc9f）
//   L5: `sakura.name,むらさき`（バイト: e38280 e38289 e38195 e3818d）
//   L6: `kero.name,エモ`      （バイト: e382a8 e383a2）
//
// foundation の charset デコード（UTF-8 宣言 → UTF-8 デコード）を通した実バイト列
// からの取得を固定する（全角「？」= U+FF1F を含む点に注意・ASCII `?` ではない）。
// ───────────────────────────────────────────────────────────────────

/// name / sakura.name / kero.name が descript.txt の実バイト列どおり UTF-8 で取得される。
#[test]
fn emo2_names_decoded_as_utf8() {
    let model = resolve_emo2();

    assert_eq!(model.names.name, Some("えも？？".to_string()));
    assert_eq!(model.names.sakura_name, Some("むらさき".to_string()));
    assert_eq!(model.names.kero_name, Some("エモ".to_string()));
}

// ───────────────────────────────────────────────────────────────────
// 未使用フィールド・未使用ファイルの無影響（要件 5.2/5.3）
//
// emo2 は結果に写らない要素を多数含む。これらが解決結果へ一切漏れないことを、
// 「クリーンなレイアウト＋名前アサーションが緑であること」を証拠として固定する。
//
// 採取元（無影響であるべき要素）:
//   descript.txt L3: `id.emo2`（カンマ無し行）
//   descript.txt L8-10: `craftman` / `craftmanw` / `craftmanurl`
//   descript.txt L11: `homeurl`
//   fixtures/emo2/install.txt（NAR 配置マニフェスト・起動時不使用）
//   fixtures/emo2/emo2-kakukaku/（別サブディレクトリ）
//   fixtures/emo2/delete.txt
//
// 構造保証: `MountModel` には id/craftman/homeurl 等に相当するフィールドが存在せず、
// これらが漏れ込む先の型が無い（型レベルで排他）。ゆえに本テストは、これらの要素が
// 存在してもなお shiori/shell/names が上記実測値どおりに解決されることを再確認する。
// ───────────────────────────────────────────────────────────────────

/// emo2 の未使用フィールド（カンマ無し `id.emo2`・`craftman*`・`homeurl`）と未使用
/// ファイル（ルート `install.txt`・`emo2-kakukaku/`・`delete.txt`）が結果に一切影響
/// しないことを確認する。`MountModel` はこれらを写す先を持たず（型で排他）、有効な
/// 解決値（shiori/shell/names）のみが残る。
#[test]
fn emo2_unused_fields_and_files_do_not_leak() {
    let root = emo2_root();
    let model = resolve_emo2();

    // 有効フィールドは上記 2 テストと同一の実測値で解決される（未使用要素の存在下でも不変）。
    assert_eq!(model.shiori.file, Some("pasta.dll".to_string()));
    assert_eq!(model.shiori.dir, root.join("ghost").join("master"));
    assert_eq!(model.shell.dir, root.join("shell").join("master"));
    assert_eq!(model.names.name, Some("えも？？".to_string()));
    assert_eq!(model.names.sakura_name, Some("むらさき".to_string()));
    assert_eq!(model.names.kero_name, Some("エモ".to_string()));

    // MountModel の観測可能な面は names / shiori / shell の 3 つのみ。
    // id / craftman / homeurl 等に相当するフィールドが型に存在しないこと自体が、
    // 未使用フィールドの漏れ込みを構造的に排除している（この行はコンパイル時に
    // フィールド網羅を強制し、将来のフィールド追加時に本テストの見直しを促す）。
    let MountModel {
        names: _,
        shiori: _,
        shell: _,
        bindgroups: _,
    } = &model;
}

// ───────────────────────────────────────────────────────────────────
// 既定エンコーディング不変（要件 1.5・宣言優先）
// ───────────────────────────────────────────────────────────────────

/// emo2 は `charset,UTF-8` を宣言するため、既定エンコーディングを `Ansi` に変えても
/// prescan が UTF-8 を選び、解決結果（特に名前）は不変であることを固定する。
#[test]
fn emo2_result_invariant_to_default_encoding() {
    let root = emo2_root();

    let via_utf8 = resolve(&root, DefaultEncoding::Utf8)
        .unwrap_or_else(|e| panic!("Utf8 既定で解決失敗: {e:?}"));
    let via_ansi = resolve(&root, DefaultEncoding::Ansi)
        .unwrap_or_else(|e| panic!("Ansi 既定で解決失敗: {e:?}"));

    // charset 宣言が既定を上書きするため、両者は完全一致する。
    assert_eq!(via_utf8, via_ansi);
}

// ───────────────────────────────────────────────────────────────────
// bindgroup default 保持（要件 4.5）
//
// bindgroup default（`sakura.bindgroup*.default,数値`／`kero.*`）は ukadoc の
// カテゴリ `descript_shell` に属し、**shell の descript.txt**（= <root>/shell/master/
// descript.txt）に定義される（ghost/master/descript.txt には無い）。resolve は
// shell dir を解決した後、その descript.txt からも bindgroup default KV を拾って
// `MountModel.bindgroups` へ転記する（転記のみ・展開しない・parsers 転写層原則）。
//
// 採取元: crates/pilot/examples/shiori-host-32/fixtures/emo2/shell/master/descript.txt
//   L22: sakura.bindgroup1100.default,1
//   L34: sakura.bindgroup1207.default,1
//   L44: sakura.bindgroup1302.default,1
//   L57: sakura.bindgroup1500.default,1
//   L71: sakura.bindgroup1800.default,1
// → sakura_default_on == {1100,1207,1302,1500,1800}（順不同集合として）。
// kero.bindgroup*.default,1 は emo2 に存在しない（本体のみ）ため kero 側は空集合。
// ───────────────────────────────────────────────────────────────────

/// emo2 shell descript.txt から `default,1` の bindgroup 番号集合が読み取れる。
/// 本体（sakura）スコープは {1100,1207,1302,1500,1800}、相方（kero）スコープは空。
#[test]
fn emo2_bindgroup_defaults_are_transcribed() {
    let model = resolve_emo2();

    // 本体スコープ: default,1 の bindgroup 番号集合が emo2 実測と一致する。
    // 保持は転記順ゆえ、順不同の集合として照合する（design「昇順不問・保持は転記順」）。
    let sakura: std::collections::BTreeSet<u32> =
        model.bindgroups.sakura_default_on.iter().copied().collect();
    let expected: std::collections::BTreeSet<u32> =
        [1100, 1207, 1302, 1500, 1800].into_iter().collect();
    assert_eq!(
        sakura, expected,
        "sakura_default_on は emo2 実測の default,1 集合と一致する: {:?}",
        model.bindgroups.sakura_default_on
    );

    // 相方スコープ: emo2 は kero.bindgroup*.default,1 を持たない → 空集合。
    assert!(
        model.bindgroups.kero_default_on.is_empty(),
        "kero_default_on は空であるべき（emo2 に kero.bindgroup*.default,1 は無い）: {:?}",
        model.bindgroups.kero_default_on
    );

    // 転記のみ・展開しない: 番号はそのまま保持され、範囲展開や surface 解決を伴わない
    // （要素数は default,1 の行数ちょうど 5）。
    assert_eq!(
        model.bindgroups.sakura_default_on.len(),
        5,
        "default,1 の行数ちょうど（範囲展開しない）"
    );
}

/// 回帰: bindgroup default の増設後も既存の name 系フィールド保持が壊れない（要件 4.5）。
/// name / sakura.name / kero.name が ghost/master descript.txt どおりに保持される。
#[test]
fn emo2_name_fields_retained_after_bindgroup_extension() {
    let model = resolve_emo2();

    // name 系（GhostNames）は従来どおり ghost/master descript.txt から保持される。
    assert_eq!(model.names.name, Some("えも？？".to_string()));
    assert_eq!(model.names.sakura_name, Some("むらさき".to_string()));
    assert_eq!(model.names.kero_name, Some("エモ".to_string()));

    // shiori / shell マウントも従来どおり（bindgroup 増設が既存経路を壊さない）。
    assert_eq!(model.shiori.file, Some("pasta.dll".to_string()));
}
