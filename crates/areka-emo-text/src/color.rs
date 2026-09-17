//! 「色指定」の解析と無効表示の混色（純粋層・要件 8.1〜8.4／8.9／8.10・4.6）。
//!
//! ukadoc は `\f[color,色指定]` の値の書式を `\f` 直下の注（※）で 1 度だけ定め、影の色・
//! 選択肢マーカーの色・アンカーの色はいずれも「指定方法については※下記参照」と同じ注を指す。
//! したがって書式の解析も 1 か所であるべきで（要件 8.10）、本モジュールがその唯一の解析点である。
//! 後続仕様（`areka-P0-text-align-shadow-canon`／`areka-P0-choice-marker-styling`／
//! `areka-P0-anchor-tag-canon`）は [`parse_color`] をそのまま呼ぶ。
//!
//! ## 正典（2026-09-11 取得・requirements.md 付録 A「色指定（※）」）
//!
//! - 「基本は HTML・CSS の色の表現と似ています。」
//! - 「赤・緑・青の三原色の明度をそれぞれ 0〜255 の 10 進数値で表現し、カンマで区切る。」
//! - 「…0〜100% の百分率で表現し、カンマで区切る。」
//! - 「『#』に続けて 16 進数で表現し、つなげる」（3 桁形と 6 桁形）
//! - 「red、white、black というような色名を表すキーワード(全て小文字)で指定可能」
//! - `default`「指定した文字をその文字のデフォルト色に戻す」／`disable`「無効表示用の色に
//!   設定できる」／`default.plain`・`default.cursor`・`default.cursornotselect`・
//!   `default.anchor`・`default.anchornotselect`・`default.anchorvisited`
//!
//! <https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5cf_5bcolor_2c_8272_6307_5b9a_5d:1>
//!
//! 正典が定める 3 成分の形は「3 成分とも 10 進」か「3 成分とも百分率」の 2 つだけなので、
//! `100,50%,200` のように混ざった形はどちらでもなく、[`parse_color`] は受けない（要件 8.9）。
//! 先頭の符号も同様に受けない——`+N` は本仕様では `\f[height,+3]` の「相対指定」という別の
//! 意味を持つ語（要件 7.2）であり、色指定が黙って吸収してよい根拠がどこにも無い。
//!
//! 無効表示の色の式は shell 側 `menu.disable.font.color.r` の逐語「(background 画像の 0,0 の
//! 色 + menu.background.font.color * 2 ) / 3」を輸入したもの（[`mix_disabled`]・要件 4.6・
//! areka の裁量として `doc/COMPAT_ARCHITECTURE.md` §8 に登記する）。
//! <https://ssp.shillest.net/ukadoc/manual/descript_shell.html#menu.disable.font.color.r:1>
//!
//! ## 本モジュールが知らないこと
//!
//! - **語彙の解決**——`default`／`disable`／`default.cursor` 等が実際にどの色になるかは
//!   2 層（既定・無効表示・選択肢文字色）を持つ側の領分であり、ここでは [`ColorSpec`] の
//!   語彙として返すに留める（`parse_cursor_coord` と同じ「語彙と解決の分離」）。
//! - **記録**——解析の失敗は [`ColorParseError`] の理由として返すだけで、`warn!` を出すのは
//!   キーと値を知っている呼び手（`\f` の状態機械）の担当（design.md「Error Handling」）。
//!   本モジュールは全入力で値を返す全域関数で、panic も `error!` も持たない。

/// 「色指定」の解析結果。実際の色（[`ColorSpec::Rgb`]）か、戻し先を表す語彙かのいずれか。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorSpec {
    /// 具体的な色（10 進 3 成分・百分率 3 成分・16 進・色名のいずれかから解決済み）。
    Rgb(u8, u8, u8),
    /// `default`——その文字の既定の色へ戻す（R8.5）。
    Default,
    /// `default.plain`——素の既定の色へ戻す（R8.5）。
    DefaultPlain,
    /// `disable`——無効表示の色にする（R8.6）。
    Disable,
    /// `default.cursor`——選択中の選択肢の既定文字色（R8.7）。
    DefaultCursor,
    /// `default.cursornotselect`——非選択の選択肢の既定文字色（R8.7）。
    DefaultCursorNotSelect,
    /// `default.anchor`——アンカーの既定文字色（R8.8・現状は `Default` として扱う）。
    DefaultAnchor,
    /// `default.anchornotselect`——非選択アンカーの既定文字色（R8.8・同上）。
    DefaultAnchorNotSelect,
    /// `default.anchorvisited`——訪問済みアンカーの既定文字色（R8.8・同上）。
    DefaultAnchorVisited,
}

/// 解析の失敗。呼び手が `warn!(key, value, reason)` に載せるための理由だけを持つ（R8.9）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColorParseError {
    /// 何が受け付けられなかったか（記録用の短い日本語）。
    pub reason: &'static str,
}

/// 成分数が 1（単一トークン）でも 3（三原色）でもない。
pub(crate) const REASON_COMPONENT_COUNT: &str = "色指定の成分数が 1 でも 3 でもない";
/// 三原色の成分が 10 進数（または百分率）として読めない。
pub(crate) const REASON_NOT_A_NUMBER: &str = "色指定の成分が 10 進数でない";
/// 三原色の成分が 0〜255（百分率は 0〜100）の外。
pub(crate) const REASON_OUT_OF_RANGE: &str = "色指定の成分が範囲外";
/// 三原色の 3 成分で 10 進数と百分率が混ざっている（正典はどちらかに揃った形だけを定める）。
pub(crate) const REASON_MIXED_NOTATION: &str = "色指定の 3 成分で 10 進と百分率が混在";
/// `#` 表記の桁数が 3 でも 6 でもない、または 16 進数字でない字を含む。
pub(crate) const REASON_BAD_HEX: &str = "16 進の色指定が 3 桁でも 6 桁でもない";
/// 色名表にも戻し先の語彙にも無い単一トークン（色名は小文字の完全一致のみ）。
pub(crate) const REASON_UNKNOWN_TOKEN: &str = "未知の色指定（色名は小文字のみ）";

const fn err(reason: &'static str) -> ColorParseError {
    ColorParseError { reason }
}

/// `\f[color,…]` の値の列（キーを除く）を色指定へ解析する。
///
/// 受ける形は 1 成分（`#RGB`／`#RRGGBB`／小文字の色名／戻し先の語彙）と
/// 3 成分（`R,G,B` の 10 進 0〜255／`R%,G%,B%` の百分率 0〜100）。
/// **3 成分は 10 進か百分率のどちらかに揃っていること**——正典が定めるのは R8.1 と R8.2 の
/// 2 形だけで、`100,50%,200` のような混ざった形はどちらでもないので [`Err`]（R8.9）。
/// 先頭の符号も受けない（`+3` は本仕様では `\f[height,+3]` の「相対指定」という別の意味を
/// 持つ・R7.2）。先頭ゼロ（`0255`）は 10 進数値として読む。
/// 百分率は `round(v * 255 / 100)` で 0〜255 へ写し、16 進 3 桁は各桁を 2 倍にする
/// （`#f0a` → `#ff00aa`）。成分数不足・範囲外・非数・大文字の色名は [`Err`]（R8.9）。
pub fn parse_color(args: &[&str]) -> Result<ColorSpec, ColorParseError> {
    match args {
        [single] => parse_single(single),
        [r, g, b] => {
            // 書式は先頭の成分が決め、残る 2 成分は同じ書式であることを要求する。
            let percent = r.ends_with('%');
            if g.ends_with('%') != percent || b.ends_with('%') != percent {
                return Err(err(REASON_MIXED_NOTATION));
            }
            Ok(ColorSpec::Rgb(
                parse_component(r, percent)?,
                parse_component(g, percent)?,
                parse_component(b, percent)?,
            ))
        }
        _ => Err(err(REASON_COMPONENT_COUNT)),
    }
}

/// 単一トークン——`#` 表記・戻し先の語彙・小文字の色名の順に見る。
fn parse_single(token: &str) -> Result<ColorSpec, ColorParseError> {
    if let Some(hex) = token.strip_prefix('#') {
        return parse_hex(hex);
    }
    // 戻し先の語彙は小文字の完全一致のみ（6 値の語と同じ規律）。
    let vocabulary = match token {
        "default" => Some(ColorSpec::Default),
        "default.plain" => Some(ColorSpec::DefaultPlain),
        "disable" => Some(ColorSpec::Disable),
        "default.cursor" => Some(ColorSpec::DefaultCursor),
        "default.cursornotselect" => Some(ColorSpec::DefaultCursorNotSelect),
        "default.anchor" => Some(ColorSpec::DefaultAnchor),
        "default.anchornotselect" => Some(ColorSpec::DefaultAnchorNotSelect),
        "default.anchorvisited" => Some(ColorSpec::DefaultAnchorVisited),
        _ => None,
    };
    if let Some(spec) = vocabulary {
        return Ok(spec);
    }
    lookup_color_name(token)
        .map(|(r, g, b)| ColorSpec::Rgb(r, g, b))
        .ok_or(err(REASON_UNKNOWN_TOKEN))
}

/// `#` の後ろ——3 桁は各桁を 2 倍、6 桁は 2 桁ずつ 1 成分。
fn parse_hex(body: &str) -> Result<ColorSpec, ColorParseError> {
    let digits: Vec<u8> = body
        .chars()
        .map(|c| c.to_digit(16).map(|d| d as u8))
        .collect::<Option<Vec<u8>>>()
        .ok_or(err(REASON_BAD_HEX))?;
    match digits.as_slice() {
        // 3 桁形: 各桁を 2 倍（`#f0a` → `ff 00 aa`＝`d * 17`）。
        [r, g, b] => Ok(ColorSpec::Rgb(r * 17, g * 17, b * 17)),
        [r0, r1, g0, g1, b0, b1] => Ok(ColorSpec::Rgb(r0 * 16 + r1, g0 * 16 + g1, b0 * 16 + b1)),
        _ => Err(err(REASON_BAD_HEX)),
    }
}

/// 三原色の 1 成分——書式は呼び手（[`parse_color`]）が 3 成分まとめて決める。
/// `percent` が真なら `N%` を 0〜100 から 0〜255 へ写し、偽なら `N` を 0〜255 のまま読む。
///
/// 先頭の符号は受けない。`u32` の解析は `+5` を 5 として受けてしまうが、`-1` が落ちるのに
/// `+1` が通る非対称は正典にも本仕様にも根拠が無い（`+N` は R7.2 で相対指定の意味を持つ）。
fn parse_component(token: &str, percent: bool) -> Result<u8, ColorParseError> {
    if token.starts_with('+') {
        return Err(err(REASON_NOT_A_NUMBER));
    }
    if percent {
        let body = token
            .strip_suffix('%')
            .ok_or_else(|| err(REASON_MIXED_NOTATION))?;
        let value: u32 = body.parse().map_err(|_| err(REASON_NOT_A_NUMBER))?;
        if value > 100 {
            return Err(err(REASON_OUT_OF_RANGE));
        }
        // round(value * 255 / 100)（四捨五入・50% → 128）。
        Ok(((value * 255 + 50) / 100) as u8)
    } else {
        let value: u32 = token.parse().map_err(|_| err(REASON_NOT_A_NUMBER))?;
        if value > 255 {
            return Err(err(REASON_OUT_OF_RANGE));
        }
        Ok(value as u8)
    }
}

/// 色名表の引き当て（小文字の完全一致・[`CSS_COLOR_NAMES`] は名前の昇順なので二分探索）。
fn lookup_color_name(name: &str) -> Option<(u8, u8, u8)> {
    CSS_COLOR_NAMES
        .binary_search_by(|(candidate, _)| (*candidate).cmp(name))
        .ok()
        .map(|index| CSS_COLOR_NAMES[index].1)
}

/// 無効表示の色——成分ごとに `(背景 + 文字色 × 2) / 3`（整数除算・要件 4.6）。
///
/// 式はここ 1 か所にしか置かない（2 層を組む側も、この関数を呼ぶ）。
pub fn mix_disabled(text: (u8, u8, u8), background: (u8, u8, u8)) -> (u8, u8, u8) {
    fn component(text: u8, background: u8) -> u8 {
        ((u16::from(background) + u16::from(text) * 2) / 3) as u8
    }
    (
        component(text.0, background.0),
        component(text.1, background.1),
        component(text.2, background.2),
    )
}

/// HTML/CSS の拡張色名キーワード 147 語（SVG 1.1／CSS Color 3 の色名全体・すべて小文字）。
///
/// ukadoc は色名を列挙せず「基本は HTML・CSS の色の表現と似ています」とだけ定めるので、
/// 表そのものは CSS の正典から採る。CSS Color 4 で追加された `rebeccapurple` は
/// この 147 語に含めない（HTML/CSS 側で後から足された 148 番目であり、
/// SSP 互換の側に足す根拠が正典に無い）。
///
/// **名前の昇順で並べること**——[`lookup_color_name`] が二分探索する
/// （並びは `color_tests.rs::color_name_table_is_strictly_sorted` が守る）。
pub const CSS_COLOR_NAMES: &[(&str, (u8, u8, u8))] = &[
    ("aliceblue", (0xf0, 0xf8, 0xff)),
    ("antiquewhite", (0xfa, 0xeb, 0xd7)),
    ("aqua", (0x00, 0xff, 0xff)),
    ("aquamarine", (0x7f, 0xff, 0xd4)),
    ("azure", (0xf0, 0xff, 0xff)),
    ("beige", (0xf5, 0xf5, 0xdc)),
    ("bisque", (0xff, 0xe4, 0xc4)),
    ("black", (0x00, 0x00, 0x00)),
    ("blanchedalmond", (0xff, 0xeb, 0xcd)),
    ("blue", (0x00, 0x00, 0xff)),
    ("blueviolet", (0x8a, 0x2b, 0xe2)),
    ("brown", (0xa5, 0x2a, 0x2a)),
    ("burlywood", (0xde, 0xb8, 0x87)),
    ("cadetblue", (0x5f, 0x9e, 0xa0)),
    ("chartreuse", (0x7f, 0xff, 0x00)),
    ("chocolate", (0xd2, 0x69, 0x1e)),
    ("coral", (0xff, 0x7f, 0x50)),
    ("cornflowerblue", (0x64, 0x95, 0xed)),
    ("cornsilk", (0xff, 0xf8, 0xdc)),
    ("crimson", (0xdc, 0x14, 0x3c)),
    ("cyan", (0x00, 0xff, 0xff)),
    ("darkblue", (0x00, 0x00, 0x8b)),
    ("darkcyan", (0x00, 0x8b, 0x8b)),
    ("darkgoldenrod", (0xb8, 0x86, 0x0b)),
    ("darkgray", (0xa9, 0xa9, 0xa9)),
    ("darkgreen", (0x00, 0x64, 0x00)),
    ("darkgrey", (0xa9, 0xa9, 0xa9)),
    ("darkkhaki", (0xbd, 0xb7, 0x6b)),
    ("darkmagenta", (0x8b, 0x00, 0x8b)),
    ("darkolivegreen", (0x55, 0x6b, 0x2f)),
    ("darkorange", (0xff, 0x8c, 0x00)),
    ("darkorchid", (0x99, 0x32, 0xcc)),
    ("darkred", (0x8b, 0x00, 0x00)),
    ("darksalmon", (0xe9, 0x96, 0x7a)),
    ("darkseagreen", (0x8f, 0xbc, 0x8f)),
    ("darkslateblue", (0x48, 0x3d, 0x8b)),
    ("darkslategray", (0x2f, 0x4f, 0x4f)),
    ("darkslategrey", (0x2f, 0x4f, 0x4f)),
    ("darkturquoise", (0x00, 0xce, 0xd1)),
    ("darkviolet", (0x94, 0x00, 0xd3)),
    ("deeppink", (0xff, 0x14, 0x93)),
    ("deepskyblue", (0x00, 0xbf, 0xff)),
    ("dimgray", (0x69, 0x69, 0x69)),
    ("dimgrey", (0x69, 0x69, 0x69)),
    ("dodgerblue", (0x1e, 0x90, 0xff)),
    ("firebrick", (0xb2, 0x22, 0x22)),
    ("floralwhite", (0xff, 0xfa, 0xf0)),
    ("forestgreen", (0x22, 0x8b, 0x22)),
    ("fuchsia", (0xff, 0x00, 0xff)),
    ("gainsboro", (0xdc, 0xdc, 0xdc)),
    ("ghostwhite", (0xf8, 0xf8, 0xff)),
    ("gold", (0xff, 0xd7, 0x00)),
    ("goldenrod", (0xda, 0xa5, 0x20)),
    ("gray", (0x80, 0x80, 0x80)),
    ("green", (0x00, 0x80, 0x00)),
    ("greenyellow", (0xad, 0xff, 0x2f)),
    ("grey", (0x80, 0x80, 0x80)),
    ("honeydew", (0xf0, 0xff, 0xf0)),
    ("hotpink", (0xff, 0x69, 0xb4)),
    ("indianred", (0xcd, 0x5c, 0x5c)),
    ("indigo", (0x4b, 0x00, 0x82)),
    ("ivory", (0xff, 0xff, 0xf0)),
    ("khaki", (0xf0, 0xe6, 0x8c)),
    ("lavender", (0xe6, 0xe6, 0xfa)),
    ("lavenderblush", (0xff, 0xf0, 0xf5)),
    ("lawngreen", (0x7c, 0xfc, 0x00)),
    ("lemonchiffon", (0xff, 0xfa, 0xcd)),
    ("lightblue", (0xad, 0xd8, 0xe6)),
    ("lightcoral", (0xf0, 0x80, 0x80)),
    ("lightcyan", (0xe0, 0xff, 0xff)),
    ("lightgoldenrodyellow", (0xfa, 0xfa, 0xd2)),
    ("lightgray", (0xd3, 0xd3, 0xd3)),
    ("lightgreen", (0x90, 0xee, 0x90)),
    ("lightgrey", (0xd3, 0xd3, 0xd3)),
    ("lightpink", (0xff, 0xb6, 0xc1)),
    ("lightsalmon", (0xff, 0xa0, 0x7a)),
    ("lightseagreen", (0x20, 0xb2, 0xaa)),
    ("lightskyblue", (0x87, 0xce, 0xfa)),
    ("lightslategray", (0x77, 0x88, 0x99)),
    ("lightslategrey", (0x77, 0x88, 0x99)),
    ("lightsteelblue", (0xb0, 0xc4, 0xde)),
    ("lightyellow", (0xff, 0xff, 0xe0)),
    ("lime", (0x00, 0xff, 0x00)),
    ("limegreen", (0x32, 0xcd, 0x32)),
    ("linen", (0xfa, 0xf0, 0xe6)),
    ("magenta", (0xff, 0x00, 0xff)),
    ("maroon", (0x80, 0x00, 0x00)),
    ("mediumaquamarine", (0x66, 0xcd, 0xaa)),
    ("mediumblue", (0x00, 0x00, 0xcd)),
    ("mediumorchid", (0xba, 0x55, 0xd3)),
    ("mediumpurple", (0x93, 0x70, 0xdb)),
    ("mediumseagreen", (0x3c, 0xb3, 0x71)),
    ("mediumslateblue", (0x7b, 0x68, 0xee)),
    ("mediumspringgreen", (0x00, 0xfa, 0x9a)),
    ("mediumturquoise", (0x48, 0xd1, 0xcc)),
    ("mediumvioletred", (0xc7, 0x15, 0x85)),
    ("midnightblue", (0x19, 0x19, 0x70)),
    ("mintcream", (0xf5, 0xff, 0xfa)),
    ("mistyrose", (0xff, 0xe4, 0xe1)),
    ("moccasin", (0xff, 0xe4, 0xb5)),
    ("navajowhite", (0xff, 0xde, 0xad)),
    ("navy", (0x00, 0x00, 0x80)),
    ("oldlace", (0xfd, 0xf5, 0xe6)),
    ("olive", (0x80, 0x80, 0x00)),
    ("olivedrab", (0x6b, 0x8e, 0x23)),
    ("orange", (0xff, 0xa5, 0x00)),
    ("orangered", (0xff, 0x45, 0x00)),
    ("orchid", (0xda, 0x70, 0xd6)),
    ("palegoldenrod", (0xee, 0xe8, 0xaa)),
    ("palegreen", (0x98, 0xfb, 0x98)),
    ("paleturquoise", (0xaf, 0xee, 0xee)),
    ("palevioletred", (0xdb, 0x70, 0x93)),
    ("papayawhip", (0xff, 0xef, 0xd5)),
    ("peachpuff", (0xff, 0xda, 0xb9)),
    ("peru", (0xcd, 0x85, 0x3f)),
    ("pink", (0xff, 0xc0, 0xcb)),
    ("plum", (0xdd, 0xa0, 0xdd)),
    ("powderblue", (0xb0, 0xe0, 0xe6)),
    ("purple", (0x80, 0x00, 0x80)),
    ("red", (0xff, 0x00, 0x00)),
    ("rosybrown", (0xbc, 0x8f, 0x8f)),
    ("royalblue", (0x41, 0x69, 0xe1)),
    ("saddlebrown", (0x8b, 0x45, 0x13)),
    ("salmon", (0xfa, 0x80, 0x72)),
    ("sandybrown", (0xf4, 0xa4, 0x60)),
    ("seagreen", (0x2e, 0x8b, 0x57)),
    ("seashell", (0xff, 0xf5, 0xee)),
    ("sienna", (0xa0, 0x52, 0x2d)),
    ("silver", (0xc0, 0xc0, 0xc0)),
    ("skyblue", (0x87, 0xce, 0xeb)),
    ("slateblue", (0x6a, 0x5a, 0xcd)),
    ("slategray", (0x70, 0x80, 0x90)),
    ("slategrey", (0x70, 0x80, 0x90)),
    ("snow", (0xff, 0xfa, 0xfa)),
    ("springgreen", (0x00, 0xff, 0x7f)),
    ("steelblue", (0x46, 0x82, 0xb4)),
    ("tan", (0xd2, 0xb4, 0x8c)),
    ("teal", (0x00, 0x80, 0x80)),
    ("thistle", (0xd8, 0xbf, 0xd8)),
    ("tomato", (0xff, 0x63, 0x47)),
    ("turquoise", (0x40, 0xe0, 0xd0)),
    ("violet", (0xee, 0x82, 0xee)),
    ("wheat", (0xf5, 0xde, 0xb3)),
    ("white", (0xff, 0xff, 0xff)),
    ("whitesmoke", (0xf5, 0xf5, 0xf5)),
    ("yellow", (0xff, 0xff, 0x00)),
    ("yellowgreen", (0x9a, 0xcd, 0x32)),
];

#[cfg(test)]
#[path = "color_tests.rs"]
mod tests;
