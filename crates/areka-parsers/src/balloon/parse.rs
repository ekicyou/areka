//! balloon 公開 facade（2 層マージ＋KV マップ → バルーンモデル写像）。
//!
//! フラット KV マップの 2 層（descript 既定層＋画像別上書き層・任意）を後勝ち `insert` で
//! 1 マップへマージし、そのマージ済み 1 マップから各モデル化キーを完全一致で引いて
//! `i32`/`u32`/`u8` へ整数パースし、幾何＋フォント subset の `BalloonModel` を構築する。
//!
//! 契約:
//! - `Result` を返さず panic しない寛容写像（キー不在・非数値・範囲外は当該スカラを `None`）。
//!   未知キー・非モデル化キーは完全一致引きゆえ自然に無視する。
//! - 2 層マージで 3 段優先度（画像別 ＞ descript ＞ 未指定＝`None`）を単一機構で満たす。
//! - 負値は符号付き `i32` のまま保持しピクセル解決は行わない。
//! - `font.color.{r,g,b}` の 3 キーを個別に引き `FontColor` へ束ねる（部分欠落は個別 `None`）。
//! - KV 化は上流 `kv::parse_kv` に任せ、再実装しない。
//! - `windowposition.x`/`.limit` は数値写像とは別枠で生文字列を忠実転記する（解釈も警告もしない）。
//! - `font.{bold,italic,outline,strike,underline}`／`font.shadowcolor.{r,g,b}`／`font.shadowstyle`
//!   の 9 本は生文字列のまま転記し、`disable.font.*` 14 本は基底と同じ形（書体名・大きさ・色は
//!   既存 5 本と同じ縮退規則）で別の層へ束ねる（解釈も警告もしない）。
//!
//! 依存方向は `model ← parse`。KV 化は `crate::kv::parse_kv` に委譲する。

use std::collections::BTreeMap;
use std::str::FromStr;

use crate::kv::parse_kv;

use super::marker::{KeyIssue, read_arrow_keys, read_marker_filename};
use super::model::{
    BalloonCursor, BalloonModel, CursorColor, DisableFont, Font, FontColor, FontDecorationRaw,
    FontShadowRaw, Origin, ValidRect, WindowPosition, WindowPositionRaw, WordWrapPoint,
};

/// 主入口: 既に KV マップ化済みの 2 層（descript 既定層＋画像別上書き層・任意）を写像する。
///
/// descript 基層を複製し、画像別層があればその各エントリを後勝ち `insert` で重ねた
/// 「マージ済み 1 マップ」から 1 回だけ写像する。画像別層 `None` なら descript 基層のみを写像する。
///
/// - Preconditions: 入力はデコード済み（charset は上流責務）。
/// - Postconditions: 常に `BalloonModel` を返す（`Result` 無し）。存在キーは型付き・符号保持で
///   反映、不在/非数値/範囲外キーは当該スカラ `None`。
/// - 未知キー・非モデル化キーは完全一致引きで自然に無視する。panic しない。
pub fn parse(
    descript: &BTreeMap<String, String>,
    image: Option<&BTreeMap<String, String>>,
) -> BalloonModel {
    match image {
        Some(image_map) => {
            let mut merged = descript.clone();
            merged.extend(image_map.iter().map(|(k, v)| (k.clone(), v.clone())));
            map_merged(&merged)
        }
        None => map_merged(descript),
    }
}

/// 便宜入口: デコード済み文字列 2 層を内部で `kv::parse_kv` して `parse` へ委譲する。
///
/// 画像別層 `None` は descript 文字列のみを写像する。
pub fn parse_str(descript: &str, image: Option<&str>) -> BalloonModel {
    let mut merged = parse_kv(descript);
    if let Some(image) = image {
        merged.extend(parse_kv(image));
    }
    map_merged(&merged)
}

/// マージ済み 1 マップから各モデル化キーを完全一致で引き `BalloonModel` を構築する。
fn map_merged(merged: &BTreeMap<String, String>) -> BalloonModel {
    let mut issues = Vec::new();
    let windowposition = WindowPosition::new(
        // `center` などの語を正しく持つ鍵なので、数として読めなくても積まない（語の判別は配置層）。
        merged.get("windowposition.x").and_then(|v| v.parse().ok()),
        get_scalar::<i32>(merged, &mut issues, "windowposition.y"),
    );
    // `windowposition.x`/`.limit` は数値化せず生文字列のまま持ち上げる（数値化できない
    // center/top/bottom や limit の 0/1 を含む）。検証・判別・警告付き縮退は配置層の責務。
    let windowposition_raw = WindowPositionRaw::new(
        get_raw(merged, "windowposition.x"),
        get_raw(merged, "windowposition.limit"),
    );
    let origin = Origin::new(
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#origin.x_2c_5ea7_6a19_20_2a1:1
        get_scalar::<i32>(merged, &mut issues, "origin.x"),
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#origin.y_2c_5ea7_6a19_20_2a1:1
        get_scalar::<i32>(merged, &mut issues, "origin.y"),
    );
    let wordwrappoint = WordWrapPoint::new(
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#wordwrappoint.x_2c_5ea7_6a19_20_2a1:1
        get_scalar::<i32>(merged, &mut issues, "wordwrappoint.x"),
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#wordwrappoint.y_2c_5ea7_6a19_20_2a1:1
        get_scalar::<i32>(merged, &mut issues, "wordwrappoint.y"),
    );
    let validrect = ValidRect::new(
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#validrect.top_2c_5ea7_6a19_20_2a1:1
        get_scalar::<i32>(merged, &mut issues, "validrect.top"),
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#validrect.bottom_2c_5ea7_6a19_20_2a1:1
        get_scalar::<i32>(merged, &mut issues, "validrect.bottom"),
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#validrect.left_2c_5ea7_6a19_20_2a1:1
        get_scalar::<i32>(merged, &mut issues, "validrect.left"),
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#validrect.right_2c_5ea7_6a19_20_2a1:1
        get_scalar::<i32>(merged, &mut issues, "validrect.right"),
    );
    let color = FontColor::new(
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#font.color.r_2c_6570_5024:1
        get_scalar::<u8>(merged, &mut issues, "font.color.r"),
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#font.color.g_2c_6570_5024:1
        get_scalar::<u8>(merged, &mut issues, "font.color.g"),
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#font.color.b_2c_6570_5024:1
        get_scalar::<u8>(merged, &mut issues, "font.color.b"),
    );
    let font = Font::new(
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#font.name_2c_30d5_30a9_30f3_30c8_540d:1
        get_raw(merged, "font.name"),
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#font.height_2c_6570_5024:1
        get_scalar::<u32>(merged, &mut issues, "font.height"),
        color,
    );
    // writing_mode は生文字列の転記のみ（値の検証・語彙判定・fallback は下流のテキスト層の責務）。
    let writing_mode = get_raw(merged, "writing_mode");
    // budoux_newline も writing_mode と同じく生文字列の転記のみ。
    let budoux_newline = get_raw(merged, "budoux_newline");
    // vertical も生文字列の転記のみ。上 2 者と異なり areka 拡張キーではなく SSP 正典キー
    // （`0`／`1`・既定は横書き）。未宣言（None）と宣言（Some・空文字列を含む）を潰さず持ち上げ、
    // `0`/`1` の検証・語彙外値の縮退・`writing_mode` との共存裁定は書字方向の解決層の責務。
    let vertical = get_raw(merged, "vertical");

    // style/blendmethod は生文字列転記（語彙判定は下流）。brush/pen/font の各色成分は
    // `font.color.*` と同型に個別 u8 パースする（部分欠落は個別 None）。未モデル化サブキー
    // （cursor.shadowcolor.* / cursor.shadowstyle 等）は完全一致引きゆえ採らない。
    let cursor = BalloonCursor::new(
        get_raw(merged, "cursor.style"),
        CursorColor::new(
            // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#cursor.brush.color.r_2c_6570_5024:1
            get_scalar::<u8>(merged, &mut issues, "cursor.brush.color.r"),
            // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#cursor.brush.color.g_2c_6570_5024:1
            get_scalar::<u8>(merged, &mut issues, "cursor.brush.color.g"),
            // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#cursor.brush.color.b_2c_6570_5024:1
            get_scalar::<u8>(merged, &mut issues, "cursor.brush.color.b"),
        ),
        CursorColor::new(
            get_scalar::<u8>(merged, &mut issues, "cursor.pen.color.r"),
            get_scalar::<u8>(merged, &mut issues, "cursor.pen.color.g"),
            get_scalar::<u8>(merged, &mut issues, "cursor.pen.color.b"),
        ),
        CursorColor::new(
            // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#cursor.font.color.r_2c_6570_5024:1
            get_scalar::<u8>(merged, &mut issues, "cursor.font.color.r"),
            // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#cursor.font.color.g_2c_6570_5024:1
            get_scalar::<u8>(merged, &mut issues, "cursor.font.color.g"),
            // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#cursor.font.color.b_2c_6570_5024:1
            get_scalar::<u8>(merged, &mut issues, "cursor.font.color.b"),
        ),
        get_raw(merged, "cursor.blendmethod"),
    );

    // 書体・影は生文字列のまま保つ（`get_scalar` で数値化すると語彙外の値・無効化の語 `none`・
    // 宣言された `0` が消える）。`disable.font.*` の書体名・大きさ・色は基底と同じ式で引き、
    // 既存の縮退規則を継ぐ。接頭辞付きキーは完全一致引きゆえ基底と無効表示は混ざらない。
    let font_decoration_raw = FontDecorationRaw::new(
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#font.bold_2c0_2f1:1
        get_raw(merged, "font.bold"),
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#font.italic_2c0_2f1:1
        get_raw(merged, "font.italic"),
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#font.outline_2c0_2f1:1
        get_raw(merged, "font.outline"),
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#font.strike_2c0_2f1:1
        get_raw(merged, "font.strike"),
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#font.underline_2c0_2f1:1
        get_raw(merged, "font.underline"),
    );
    let font_shadow_raw = FontShadowRaw::new(
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#font.shadowcolor.r_2c_6570_5024:1
        get_raw(merged, "font.shadowcolor.r"),
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#font.shadowcolor.g_2c_6570_5024:1
        get_raw(merged, "font.shadowcolor.g"),
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#font.shadowcolor.b_2c_6570_5024:1
        get_raw(merged, "font.shadowcolor.b"),
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#font.shadowstyle_2c_5f62_614b_6307_5b9a:1
        get_raw(merged, "font.shadowstyle"),
    );
    // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#disable.font._28_30d5_30a9_30f3_30c8_5b9a_7fa9_29_2c_28_6307_5b9a_29:1
    let disable_font = DisableFont::new(
        Font::new(
            get_raw(merged, "disable.font.name"),
            get_scalar::<u32>(merged, &mut issues, "disable.font.height"),
            FontColor::new(
                get_scalar::<u8>(merged, &mut issues, "disable.font.color.r"),
                get_scalar::<u8>(merged, &mut issues, "disable.font.color.g"),
                get_scalar::<u8>(merged, &mut issues, "disable.font.color.b"),
            ),
        ),
        FontDecorationRaw::new(
            get_raw(merged, "disable.font.bold"),
            get_raw(merged, "disable.font.italic"),
            get_raw(merged, "disable.font.outline"),
            get_raw(merged, "disable.font.strike"),
            get_raw(merged, "disable.font.underline"),
        ),
        FontShadowRaw::new(
            get_raw(merged, "disable.font.shadowcolor.r"),
            get_raw(merged, "disable.font.shadowcolor.g"),
            get_raw(merged, "disable.font.shadowcolor.b"),
            get_raw(merged, "disable.font.shadowstyle"),
        ),
    );

    BalloonModel::new(
        windowposition,
        origin,
        wordwrappoint,
        validrect,
        font,
        writing_mode,
        budoux_newline,
    )
    .with_cursor(cursor)
    .with_windowposition_raw(windowposition_raw)
    .with_vertical_raw(vertical)
    .with_font_decoration_raw(font_decoration_raw)
    .with_font_shadow_raw(font_shadow_raw)
    .with_disable_font(disable_font)
    // 矢印の鍵と印の絵の名前は同じマージ済みの表から読むので、面の後勝ちに乗る。
    .with_arrow_keys(read_arrow_keys(merged))
    .with_marker_filename(read_marker_filename(merged))
    .with_key_issues(issues)
}

/// マージ済みマップから `key` を完全一致で引き、値を生文字列のまま複製する（数値化しない）。
fn get_raw(merged: &BTreeMap<String, String>, key: &str) -> Option<String> {
    merged.get(key).map(|v| v.to_owned())
}

/// マージ済みマップから `key` を完全一致で引き、値を `T` へ整数パースする寛容ヘルパ。
///
/// 完全一致引き（`get`）ゆえ distractor キー（`anchor.font.color.r` 等）を巻き込まない。
/// キー不在・非数値・範囲外（`parse` が `Err`）はいずれも `None` へ降格し panic しない。
/// 鍵が在って読めないとき（空の値を含む）は `issues` に 1 件積む（記録は読み込む側）。
fn get_scalar<T: FromStr>(
    merged: &BTreeMap<String, String>,
    issues: &mut Vec<KeyIssue>,
    key: &'static str,
) -> Option<T> {
    let value = merged.get(key)?;
    let parsed = value.parse::<T>().ok();
    if parsed.is_none() {
        issues.push(KeyIssue {
            key,
            value: value.clone(),
        });
    }
    parsed
}
