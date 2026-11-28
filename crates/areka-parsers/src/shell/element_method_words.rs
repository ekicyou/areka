//! element定義の描画メソッドの語の表 — 画像の element定義にする綴りを完全一致で 1 か所で答える。

/// `blend-<種類>` と `blend-<種類>-fast` の 27 種から綴りの並びを作る（種類の並びを 1 回だけ書く）。
macro_rules! blend_element_words {
    ($($kind:literal),* $(,)?) => {
        &[
            $(concat!("blend-", $kind), concat!("blend-", $kind, "-fast"),)*
            "blend-dither",
            "overlaymultiply",
            "overlayscreen",
        ]
    };
}

/// 画像の element定義にする `blend-*` と旧い別名の綴り（完全一致）。
///
/// 種類は ukadoc の 27 種。`blend-dither` には `-fast` が無い。
/// ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html
pub const BLEND_ELEMENT_WORDS: &[&str] = blend_element_words!(
    "add",
    "add-glow",
    "color",
    "color-burn",
    "color-dodge",
    "color-dodge-glow",
    "darken",
    "darker-color",
    "difference",
    "divide",
    "exclusion",
    "hard-light",
    "hard-mix",
    "hue",
    "lighten",
    "lighter-color",
    "linear-burn",
    "linear-light",
    "luminosity",
    "multiply",
    "overlay",
    "pin-light",
    "saturation",
    "screen",
    "soft-light",
    "subtract",
    "vivid-light",
);

/// 画像の element定義として値にする描画メソッドの語か（綴りの完全一致）。
///
/// 重ね方の語 7 つ・`base`・[`BLEND_ELEMENT_WORDS`] が真。大文字の綴り・空の欄・未知の語・
/// `add`・`bind`・`auto`・`balloon` は偽。描けない行の転記もこの答えで行を外す。
pub(super) fn is_image_element_method(word: &str) -> bool {
    matches!(
        word,
        "overlay"
            | "overlay-fast"
            | "overlayfast"
            | "interpolate"
            | "replace"
            | "asis"
            | "reduce"
            | "base"
    ) || BLEND_ELEMENT_WORDS.contains(&word)
}
