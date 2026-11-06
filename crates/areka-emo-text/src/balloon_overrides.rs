//! バルーン定義の書体設定を受け口（[`LookLayers::from_balloon`](crate::look::LookLayers::from_balloon)）の 2 引数へ写す純粋層。
//!
//! 宣言されたキーだけを `\f` と**同じ形のトークン列**（`[0]` がキー・`[1..]` が値の列）へ
//! 写す。受け口が飛ばす値の記録は受け口（`look.rs` の `apply_overrides`）が適用と同じ 1 回で
//! 出す——本モジュールは語彙表を持たず、列を通し直さない。ここで記録するのは列に入れない
//! 色の成分不足だけ。
//!
//! - 基底の飾り 5 本（`font.bold`／`italic`／`outline`／`strike`／`underline`）→ `font`。
//! - `disable.font.*` の飾り 5 本・`name`・`height`・`color` → `disable`。
//! - 影の 4 キー（`font.shadowcolor.*`／`font.shadowstyle`・無効表示の同名）は**渡さない**
//!   ——語彙の形と `none` の扱いは影まわりの仕様（`text-align-shadow-canon`）が決める。

use areka_parsers::balloon::{BalloonModel, FontDecorationRaw};
use tracing::warn;

/// 受け口が飛ばす値の記録文（基底・無効表示・色の成分不足で共通・`look.rs` も使う）。
pub(crate) const SKIPPED: &str = "バルーン定義の書体設定を適用できない——当該項目は既定のまま";

/// [`LookLayers::from_balloon`](crate::look::LookLayers::from_balloon) の `font_overrides`／`disable_overrides` に渡す列。
#[non_exhaustive]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BalloonOverrides {
    /// 基底の書体設定（`font.*`）のトークン列。
    pub font: Vec<Vec<String>>,
    /// 無効表示の書体設定（`disable.font.*`）のトークン列。
    pub disable: Vec<Vec<String>>,
}

/// バルーン定義から受け口へ渡す 2 つの列を組む（色の成分不足だけは列に入れず `warn!`）。
///
/// 飛ばされる値も列からは外さない——受け口が飛ばし、そこで 1 度だけ記録する。
pub fn overrides(model: &BalloonModel) -> BalloonOverrides {
    let mut font = Vec::new();
    push_decoration(&mut font, model.font_decoration_raw());

    let disable_font = model.disable_font();
    let mut disable = Vec::new();
    let db = disable_font.font();
    if let Some(raw) = db.name() {
        // カンマで切って trim する。基底の書体名（`draw.rs` の `resolve_with_background`）は
        // 空トークンを捨てるが、ここでは残す——受け口の `apply_name` が「途中の空トークンは
        // 記述順のまま候補列に残す」「全部空なら飛ばす」と判定するので、その判定に委ねる。
        let mut tokens = vec!["name".to_owned()];
        tokens.extend(raw.split(',').map(|s| s.trim().to_owned()));
        disable.push(tokens);
    }
    if let Some(h) = db.height() {
        disable.push(vec!["height".to_owned(), h.to_string()]);
    }
    let color = db.color();
    match (color.r(), color.g(), color.b()) {
        (Some(r), Some(g), Some(b)) => disable.push(vec![
            "color".to_owned(),
            r.to_string(),
            g.to_string(),
            b.to_string(),
        ]),
        (None, None, None) => {}
        (r, g, b) => {
            let part = |c: Option<u8>| c.map_or(String::new(), |v| v.to_string());
            warn!(
                key = "disable.font.color",
                value = format!("{},{},{}", part(r), part(g), part(b)),
                reason = "3 成分が揃っていない",
                "{SKIPPED}"
            );
        }
    }
    push_decoration(&mut disable, disable_font.decoration_raw());

    BalloonOverrides { font, disable }
}

/// 飾り 5 本のうち宣言されたものを `[キー, 値]`（値はそのまま）で積む。
fn push_decoration(out: &mut Vec<Vec<String>>, raw: &FontDecorationRaw) {
    for (key, value) in [
        ("bold", raw.bold()),
        ("italic", raw.italic()),
        ("outline", raw.outline()),
        ("strike", raw.strike()),
        ("underline", raw.underline()),
    ] {
        if let Some(value) = value {
            out.push(vec![key.to_owned(), value.to_owned()]);
        }
    }
}

#[cfg(test)]
#[path = "balloon_overrides_tests.rs"]
mod balloon_overrides_tests;
