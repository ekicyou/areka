//! バルーン面 0 の原点画素から「バルーンの背景色」を導く（要件 4.6）。
//!
//! 無効表示（`\f[disable]`）の文字色は「既定の文字色をバルーンの背景色の側へ寄せた色」として
//! 導く（混色の式は `areka_emo_text::color::mix_disabled` が唯一の実装点）。その「背景色」の
//! 源が、面 0 の焼き込み済み画像の原点画素である——ukadoc の shell 側 `menu.disable.font.color`
//! の式が「background 画像の (0,0) の色」を読むのに倣う。
//!
//! 原点画素が背景色として使えないとき（面が引けない・全透明ゆえ焼かれていない・トリムで原点が
//! bbox の外・α が 255 でない）は白（[`DEFAULT_BALLOON_BACKGROUND`]）へ落とし、理由を `debug!`
//! で残す（log-first・記録なしの失敗経路を作らない）。

use areka_emo_atlas::{AtlasTable, SetId};
use areka_emo_text::draw::DEFAULT_BALLOON_BACKGROUND;
use tracing::debug;

/// アトラスに当該ファイル名の面が無い（`resolve` が引けない）。
const REASON_FACE_MISSING: &str = "face_missing";
/// 面は在るが全透明で焼かれていない（`placement` が `None`＝転写スキップ）。
const REASON_EMPTY_ENTRY: &str = "empty_entry";
/// α トリムで bbox が原点を含まなくなった（原点画素は透明だった）。
const REASON_ORIGIN_TRIMMED_AWAY: &str = "origin_trimmed_away";
/// 配置が指す頁がアトラスに無い（契約違反・防御）。
const REASON_PAGE_MISSING: &str = "page_missing";
/// 配置が指す画素が頁バッファの範囲外（契約違反・防御）。
const REASON_PIXEL_OUT_OF_RANGE: &str = "pixel_out_of_range";
/// 原点画素の α が 255 でない（半透明は背景色として使えない）。
const REASON_NOT_OPAQUE: &str = "not_opaque";

/// 面 0 の原画像の (0,0) の色（sRGB・非 premultiplied）。
///
/// `atlas.resolve(SetId(0), file_name)` → `entry.placement` と辿り、`trim_offset == (0,0)`
/// （＝原画像の原点が bbox の左上と一致する）かつ頁の当該画素の α が 255 のときだけ `(r,g,b)`
/// を返す。頁バッファは premultiplied BGRA だが、α が 255 のときに限り premultiplied 値と
/// 非 premultiplied 値は一致するので、そのまま採ってよい（これが α を 255 に限る理由でもある）。
///
/// それ以外（面が無い・`placement` が `None`・原点が bbox の外・α ≠ 255・頁や画素が引けない）は
/// [`DEFAULT_BALLOON_BACKGROUND`]（白）へ落とし、理由を `debug!` で残す。
pub(super) fn face_origin_color(atlas: &AtlasTable, file_name: &str) -> (u8, u8, u8) {
    let Some(id) = atlas.resolve(SetId(0), file_name) else {
        return fall_back(file_name, REASON_FACE_MISSING);
    };
    let Some(placement) = atlas.entry(id).placement.as_ref() else {
        return fall_back(file_name, REASON_EMPTY_ENTRY);
    };
    // 原画像の (0,0) が焼かれているのは bbox の左上が原点に一致するときだけ。ずれているなら
    // 原点画素は透明で捨てられており、頁の bbox 左上は**別の画素**である（読んではならない）。
    if placement.trim_offset.x != 0 || placement.trim_offset.y != 0 {
        return fall_back(file_name, REASON_ORIGIN_TRIMMED_AWAY);
    }
    let Some(page) = atlas.page(placement.page) else {
        return fall_back(file_name, REASON_PAGE_MISSING);
    };
    // uv_rect は padding 非包含ゆえ、その左上が焼かれた bbox の左上＝原画像の原点。
    let offset = (placement.uv_rect.y as usize) * (page.stride as usize)
        + (placement.uv_rect.x as usize) * 4;
    let Some(pixel) = page.bytes.get(offset..offset + 4) else {
        return fall_back(file_name, REASON_PIXEL_OUT_OF_RANGE);
    };
    // premultiplied BGRA（B,G,R,A の順）。
    if pixel[3] != u8::MAX {
        return fall_back(file_name, REASON_NOT_OPAQUE);
    }
    (pixel[2], pixel[1], pixel[0])
}

/// 白へ落として理由を残す（記録なしの失敗経路を作らない・log-first）。
fn fall_back(file_name: &str, reason: &'static str) -> (u8, u8, u8) {
    debug!(
        file = file_name,
        reason, "balloon_background: 面 0 の原点画素を背景色に採れないので白へ落とす"
    );
    DEFAULT_BALLOON_BACKGROUND
}

#[cfg(test)]
#[path = "balloon_background_tests.rs"]
mod tests;
