//! balloon 公開 facade（2 層マージ＋KV マップ → バルーンモデル写像）。
//!
//! フラット KV マップの 2 層（descript 既定層＋画像別上書き層・任意）を後勝ち `insert` で
//! 1 マップへマージし（D4）、そのマージ済み 1 マップから各モデル化キーを完全一致で引いて
//! `i32`/`u32`/`u8` へ整数パースし、幾何＋フォント subset の `BalloonModel` を構築する。
//!
//! 契約（design.md「Parser Layer / balloon::parse」）:
//! - `Result` を返さず panic しない寛容写像（キー不在・非数値・範囲外は当該スカラを `None`・
//!   R1.2/R1.4/R2.6/R3.4）。未知キー・非モデル化キーは完全一致引きゆえ自然に無視（R1.3/R2.7）。
//! - 2 層マージ（D4）で 3 段優先度（画像別 ＞ descript ＞ 未指定＝`None`）を単一機構で満たす
//!   （同一キー画像別優先・R3.2／画像別欠落時 descript 継承・R3.3／descript のみ・R3.5）。
//! - 負値は符号付き `i32` のまま保持しピクセル解決は行わない（R4.1/R4.4/R4.5）。
//! - `font.color.{r,g,b}` の 3 キーを個別に引き `FontColor` へ束ねる（部分欠落は個別 `None`・R2.6）。
//! - 上流 `kv::parse_kv` のフラット KV 出力を消費し、KV 化を再実装しない（R1.5・foundation 所有）。
//! - `windowposition.x`/`.limit` は数値写像とは別枠で生文字列を忠実転記する（解釈も警告もしない
//!   転記層契約・windowposition-limit 要件 1.1/4.1。既存の数値写像は無改変＝同要件 5.1）。
//!
//! 依存方向は `model ← parse`。KV 化は `crate::kv::parse_kv` に委譲する。

use std::collections::BTreeMap;
use std::str::FromStr;

use crate::kv::parse_kv;

use super::model::{
    BalloonCursor, BalloonModel, CursorColor, Font, FontColor, Origin, ValidRect, WindowPosition,
    WindowPositionRaw, WordWrapPoint,
};

/// 主入口: 既に KV マップ化済みの 2 層（descript 既定層＋画像別上書き層・任意）を写像する。
///
/// 2 層マージ（D4）で 3 段優先度を単一機構で満たす: descript 基層を複製し、画像別層があれば
/// その各エントリを後勝ち `insert` で重ね合わせた「マージ済み 1 マップ」を作り、その 1 マップから
/// 1 回だけ写像する。画像別層 `None` なら descript 基層のみを写像する（R3.5）。
///
/// - Preconditions: 入力はデコード済み（charset は上流責務）。
/// - Postconditions: 常に `BalloonModel` を返す（`Result` 無し・R1.2）。存在キーは型付き・符号保持で
///   反映、不在/非数値/範囲外キーは当該スカラ `None`（R2.6/R3.4/R1.4）。
/// - 未知キー・非モデル化キーは完全一致引きで自然に無視（R1.3/R2.7）。panic しない。
pub fn parse(
    descript: &BTreeMap<String, String>,
    image: Option<&BTreeMap<String, String>>,
) -> BalloonModel {
    // 2 層マージ（D4）: descript 基層を複製し、画像別層を後勝ちで重ね合わせる（R3.2/R3.3/R3.5）。
    let merged = match image {
        Some(image_map) => {
            let mut merged = descript.clone();
            for (key, value) in image_map {
                merged.insert(key.clone(), value.clone());
            }
            merged
        }
        None => descript.clone(),
    };

    map_merged(&merged)
}

/// 便宜入口: デコード済み文字列 2 層を内部で `kv::parse_kv` して `parse` へ委譲する（R1.1/R1.5）。
///
/// KV 化（行分割・後勝ち・trim）は foundation の `kv::parse_kv` が所有するため再実装しない（R1.5）。
/// 画像別層 `None` は descript 文字列のみを写像する（R3.5）。
pub fn parse_str(descript: &str, image: Option<&str>) -> BalloonModel {
    let descript_map = parse_kv(descript);
    let image_map = image.map(parse_kv);
    parse(&descript_map, image_map.as_ref())
}

/// マージ済み 1 マップから各モデル化キーを完全一致で引き `BalloonModel` を構築する（R2/R4）。
fn map_merged(merged: &BTreeMap<String, String>) -> BalloonModel {
    let windowposition = WindowPosition::new(
        get_scalar::<i32>(merged, "windowposition.x"),
        get_scalar::<i32>(merged, "windowposition.y"),
    );
    // windowposition.x/.limit の生文字列転記（windowposition-limit 要件 1.1/4.1）。
    // 上の数値写像とは別枠であり、数値化できないキーワード（center/top/bottom）や limit の 0/1 を
    // 解釈せず・警告せずそのまま持ち上げる。trim は kv 層（`kv::parse_kv`）で済んでいる。
    // 0/1 検証・キーワード判別・語彙外値の警告付き縮退は下流（配置層）の責務。
    let windowposition_raw = WindowPositionRaw::new(
        merged.get("windowposition.x").map(|v| v.to_owned()),
        merged.get("windowposition.limit").map(|v| v.to_owned()),
    );
    let origin = Origin::new(
        get_scalar::<i32>(merged, "origin.x"),
        get_scalar::<i32>(merged, "origin.y"),
    );
    let wordwrappoint = WordWrapPoint::new(
        get_scalar::<i32>(merged, "wordwrappoint.x"),
        get_scalar::<i32>(merged, "wordwrappoint.y"),
    );
    let validrect = ValidRect::new(
        get_scalar::<i32>(merged, "validrect.top"),
        get_scalar::<i32>(merged, "validrect.bottom"),
        get_scalar::<i32>(merged, "validrect.left"),
        get_scalar::<i32>(merged, "validrect.right"),
    );
    let color = FontColor::new(
        get_scalar::<u8>(merged, "font.color.r"),
        get_scalar::<u8>(merged, "font.color.g"),
        get_scalar::<u8>(merged, "font.color.b"),
    );
    let font = Font::new(
        // font.name は文字列値（数値化しない・R2.5）。
        merged.get("font.name").map(|v| v.to_owned()),
        get_scalar::<u32>(merged, "font.height"),
        color,
    );
    // writing_mode は生文字列の転記のみ（trim は kv 層済み・値の検証・語彙判定・fallback は
    // 下流 emo テキスト層の責務・emo-text-layer 要件 5.6）。
    let writing_mode = merged.get("writing_mode").map(|v| v.to_owned());
    // budoux_newline も生文字列の転記のみ（writing_mode と同一規律・値の検証・語彙判定・
    // fallback は下流 emo テキスト層の責務・budoux-newline 要件 1.1）。
    let budoux_newline = merged.get("budoux_newline").map(|v| v.to_owned());
    // vertical も生文字列の転記のみ。上 2 者と異なりこれは areka 拡張キーではなく SSP 正典キー
    // （SSP 2.8.80 が確立・`0`／`1`）であり、正典側の既定は横書きである。ただし転記層の扱いは
    // 同一で、未宣言（None）と宣言（Some・空文字列を含む）を潰さずそのまま持ち上げる。
    // `0`/`1` の検証・語彙外値の警告付き縮退・`writing_mode` との共存裁定は、いずれも下流の
    // 書字方向の解決層の責務である（balloon-vertical-canon 要件 1.4/1.6/1.7）。
    let vertical = merged.get("vertical").map(|v| v.to_owned());

    // cursor.* スタイルモデル（additive・要件 4.2/6.2）。既存の完全一致引き＋2 層後勝ちマージに
    // 相乗りする。style/blendmethod は生文字列転記（語彙判定は下流）。brush/pen/font の各色成分は
    // `font.color.*` と同型に個別 u8 パース（部分欠落は個別 None）。未モデル化サブキー
    // （cursor.shadowcolor.* / cursor.shadowstyle 等）は完全一致引きゆえ自然に非採用＝KV 層が
    // 語彙を落とさない寛容パス素通しのまま（6.2 の語彙シーム）。
    let cursor = BalloonCursor::new(
        // style は文字列値（数値化しない・要件 4.2/6.5）。
        merged.get("cursor.style").map(|v| v.to_owned()),
        CursorColor::new(
            get_scalar::<u8>(merged, "cursor.brush.color.r"),
            get_scalar::<u8>(merged, "cursor.brush.color.g"),
            get_scalar::<u8>(merged, "cursor.brush.color.b"),
        ),
        CursorColor::new(
            get_scalar::<u8>(merged, "cursor.pen.color.r"),
            get_scalar::<u8>(merged, "cursor.pen.color.g"),
            get_scalar::<u8>(merged, "cursor.pen.color.b"),
        ),
        CursorColor::new(
            get_scalar::<u8>(merged, "cursor.font.color.r"),
            get_scalar::<u8>(merged, "cursor.font.color.g"),
            get_scalar::<u8>(merged, "cursor.font.color.b"),
        ),
        // blendmethod は不透明転写（数値化しない・要件 6.5）。
        merged.get("cursor.blendmethod").map(|v| v.to_owned()),
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
}

/// マージ済みマップから `key` を完全一致で引き、値を `T` へ整数パースする寛容ヘルパ（R1.4/R2.6）。
///
/// 完全一致引き（`get`）ゆえ distractor キー（`anchor.font.color.r` 等）を巻き込まない（R2.7）。
/// キー不在・非数値・範囲外（`parse` が `Err`）はいずれも `None` へ降格し panic しない（R1.4/R3.4）。
fn get_scalar<T: FromStr>(merged: &BTreeMap<String, String>, key: &str) -> Option<T> {
    merged.get(key).and_then(|value| value.parse::<T>().ok())
}
