//! バルーン定義の書体設定を受け口（[`LookLayers::from_balloon`]）の 2 引数へ写す純粋層。
//!
//! 宣言されたキーだけを `\f` と**同じ形のトークン列**（`[0]` がキー・`[1..]` が値の列）へ
//! 写し、受け口と同じ順序・同じ土台で [`apply_font_tag`] に通して、受け口が黙って飛ばす値を
//! `warn!` で記録する（受け口の doc が「記録はバルーン定義を読む側が出す」と申し送っている）。
//! 判定の実体は受け口 1 か所——本モジュールは語彙表を持たない。
//!
//! - 基底の飾り 5 本（`font.bold`／`italic`／`outline`／`strike`／`underline`）→ `font`。
//! - `disable.font.*` の飾り 5 本・`name`・`height`・`color` → `disable`。
//! - 影の 4 キー（`font.shadowcolor.*`／`font.shadowstyle`・無効表示の同名）は**渡さない**
//!   ——語彙の形と `none` の扱いは影まわりの仕様（`areka-P0-text-align-shadow-canon`）が決める。

use areka_parsers::balloon::{BalloonModel, FontDecorationRaw};
use tracing::warn;

use crate::look::{FontTagIssue, LookLayers, TextLook, apply_font_tag};

/// 受け口が飛ばす値の記録文（基底・無効表示・色の成分不足で共通）。
const SKIPPED: &str = "バルーン定義の書体設定を適用できない——当該項目は既定のまま";

/// [`LookLayers::from_balloon`] の `font_overrides`／`disable_overrides` に渡す列。
#[non_exhaustive]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BalloonOverrides {
    /// 基底の書体設定（`font.*`）のトークン列。
    pub font: Vec<Vec<String>>,
    /// 無効表示の書体設定（`disable.font.*`）のトークン列。
    pub disable: Vec<Vec<String>>,
}

/// バルーン定義から受け口へ渡す 2 つの列を組み、受け口が飛ばす値を `warn!` で記録する。
///
/// `base` はバルーン定義から**空の列で**組んだ 2 層（`LookLayers::from_balloon(…, &[], &[])`）で、
/// 受け口と同じ順序で事前検証するための土台。既定の層（[`LookLayers::default`]）で代用しては
/// ならない——相対・百分率の `height` は「今効いている大きさ」に足して判定されるので、土台が
/// 違うと受け口と判定が食い違う。
///
/// 飛ばされる値も列からは外さない（受け口が同じ判定で飛ばすので結果は同じ）。
pub fn overrides(model: &BalloonModel, base: &LookLayers) -> BalloonOverrides {
    let mut font = Vec::new();
    push_decoration(&mut font, model.font_decoration_raw());

    let disable_font = model.disable_font();
    let mut disable = Vec::new();
    let db = disable_font.font();
    if let Some(raw) = db.name() {
        // 基底の `resolve` と同じ切り方。空トークンは潰さない（受け口が判定する）。
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

    // 事前検証——受け口の `from_balloon` と同じ順序・同じ土台（基底 → その複製へ無効表示）。
    let mut probe = base.default.clone();
    probe_all(&mut probe, base, &font);
    let mut probe_disable = probe;
    probe_all(&mut probe_disable, base, &disable);

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

/// 列を順に [`apply_font_tag`] へ通し、`Err` だけを記録する（語彙のみ・`Ok(None)` は記録しない）。
fn probe_all(probe: &mut TextLook, base: &LookLayers, lists: &[Vec<String>]) {
    for tokens in lists {
        let args: Vec<&str> = tokens.iter().map(String::as_str).collect();
        if let Err(FontTagIssue { key, value, reason }) = apply_font_tag(probe, base, &args) {
            warn!(key, value, reason, "{SKIPPED}");
        }
    }
}

#[cfg(test)]
#[path = "balloon_overrides_tests.rs"]
mod balloon_overrides_tests;
