//! 透過の宣言の読み（spec: areka-P0-self-alpha-declaration 要件 1・2・7）。
//!
//! シェル（`seriko.use_self_alpha`）とバルーン（`use_self_alpha`）の 2 つの入口が、キー名と記録の
//! `kind` の欄だけを変えて同じ関数 [`read_use_self_alpha`] を通る。fs には触らない（descript.txt の
//! 本文は呼び手が読んで渡す）。値の字面から [`UseSelfAlpha`] への読み替えは [`parse`] が担う。

use std::path::Path;

use areka_emo_atlas::UseSelfAlpha;

/// descript.txt の本文から透過の宣言を読む。
///
/// `descript` は descript.txt の本文（読めなかったら `None`）。`kind` は "shell" か "balloon"。
/// 呼ぶたびに `info!` を 1 行（採った扱い・宣言によるかどうか）、値が読めないときは値つきの
/// `warn!` をもう 1 行出す（要件 7.1・7.2）。同じキーが 2 行在るときは後の行が勝つ（`parse_kv` の決まり）。
pub(crate) fn read_use_self_alpha(
    kind: &'static str,
    key: &'static str,
    dir: &Path,
    descript: Option<&str>,
) -> UseSelfAlpha {
    let value = descript.and_then(|text| areka_parsers::kv::parse_kv(text).remove(key));
    let declared = value.as_deref().and_then(parse);
    if let (Some(value), None) = (&value, declared) {
        tracing::warn!(
            kind,
            key,
            dir = %dir.display(),
            value = value.as_str(),
            "self_alpha: 透過の宣言の値が読めないので宣言なしとして扱う"
        );
    }
    let treatment = match declared {
        Some(UseSelfAlpha::On) => "1",
        Some(UseSelfAlpha::Full) => "full",
        Some(UseSelfAlpha::Off) => "0",
        Some(UseSelfAlpha::Undeclared) | None => "none",
    };
    tracing::info!(
        kind,
        key,
        dir = %dir.display(),
        treatment,
        declared = declared.is_some(),
        "self_alpha: 透過の扱いを決めた"
    );
    declared.unwrap_or(UseSelfAlpha::Undeclared)
}

/// 値の読み（純粋）。前後の空白を除き、英字の大小を区別せずに比べる。読めない値は `None`。
/// `Undeclared` は返さない。
fn parse(value: &str) -> Option<UseSelfAlpha> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" => Some(UseSelfAlpha::On),
        "full" => Some(UseSelfAlpha::Full),
        "0" => Some(UseSelfAlpha::Off),
        _ => None,
    }
}

#[cfg(test)]
#[path = "self_alpha_tests.rs"]
mod tests;
