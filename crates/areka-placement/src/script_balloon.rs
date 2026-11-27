//! 台本のバルーン位置の値（ずらし・向き）の型と引数の読み。

use tracing::debug;

use crate::config::{BalloonSide, BalloonXMode};

/// 台本のずらしの 1 軸。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetAxis {
    /// 符号つきの整数（descript のずらしの代わり）。
    Absolute(i32),
    /// 頭に `@` の符号つきの整数（descript のずらしからの相対）。
    Relative(i32),
}

/// 台本のずらし（x・y の軸ごとに絶対か相対）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScriptOffset {
    pub x: OffsetAxis,
    pub y: OffsetAxis,
}

/// 台本の向き（ukadoc の 6 語と上記以外）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptAlign {
    Left,
    Right,
    Center,
    Top,
    Bottom,
    None,
    /// 6 語以外（受け取った字のまま）。
    Unknown(String),
}

/// ずらしの引数が読めなかった理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetArgReason {
    MissingAxis,
    NotANumber,
    AtWithoutNumber,
}

/// ずらしの引数が読めなかった結果（理由と受け取った字）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OffsetArgError {
    pub reason: OffsetArgReason,
    pub raw: Vec<String>,
}

fn parse_axis(raw: &str) -> Result<OffsetAxis, OffsetArgReason> {
    let s = raw.trim();
    let (rest, relative) = match s.strip_prefix('@') {
        Some(rest) => (rest.trim(), true),
        None => (s, false),
    };
    if relative && rest.is_empty() {
        return Err(OffsetArgReason::AtWithoutNumber);
    }
    let n: i32 = rest.parse().map_err(|_| OffsetArgReason::NotANumber)?;
    Ok(if relative {
        OffsetAxis::Relative(n)
    } else {
        OffsetAxis::Absolute(n)
    })
}

/// ずらしの命令の引数（x・y）を読む。どちらかが読めなければ両軸とも採らない。
///
/// 3 つ目以降の引数は `debug!` だけで読まない。
pub fn parse_balloon_offset(args: &[&str]) -> Result<ScriptOffset, OffsetArgError> {
    let fail = |reason| OffsetArgError {
        reason,
        raw: args.iter().map(|s| (*s).to_string()).collect(),
    };
    let [x, y, extra @ ..] = args else {
        return Err(fail(OffsetArgReason::MissingAxis));
    };
    if !extra.is_empty() {
        debug!(
            event = "balloon_offset_extra_args",
            ?extra,
            "ずらしの命令の 3 つ目以降の引数は読まない"
        );
    }
    match (parse_axis(x), parse_axis(y)) {
        (Ok(x), Ok(y)) => Ok(ScriptOffset { x, y }),
        (Err(reason), _) | (_, Err(reason)) => Err(fail(reason)),
    }
}

/// 向きの命令の ID を読む。`None` は ID が無い（引数が無い・空・空白だけ）。
///
/// 2 つ目以降の引数は `debug!` だけで読まない。
pub fn parse_balloon_align(args: &[&str]) -> Option<ScriptAlign> {
    if args.len() > 1 {
        debug!(
            event = "balloon_align_extra_args",
            extra = ?&args[1..],
            "向きの命令の 3 つ目以降の引数は読まない"
        );
    }
    let raw = args.first()?.trim();
    if raw.is_empty() {
        return None;
    }
    Some(match raw.to_ascii_lowercase().as_str() {
        "left" => ScriptAlign::Left,
        "right" => ScriptAlign::Right,
        "center" => ScriptAlign::Center,
        "top" => ScriptAlign::Top,
        "bottom" => ScriptAlign::Bottom,
        "none" => ScriptAlign::None,
        _ => ScriptAlign::Unknown(raw.to_string()),
    })
}

impl ScriptAlign {
    /// 左右（`None` は今の左右を変えない）と水平配置モード。
    pub fn side_and_mode(&self) -> (Option<BalloonSide>, BalloonXMode) {
        match self {
            Self::Left => (Some(BalloonSide::Left), BalloonXMode::Side),
            Self::Right => (Some(BalloonSide::Right), BalloonXMode::Side),
            Self::Center | Self::Top => (None, BalloonXMode::CenterTop),
            Self::Bottom => (None, BalloonXMode::CenterBottom),
            Self::None | Self::Unknown(_) => (Some(BalloonSide::Auto), BalloonXMode::Side),
        }
    }
}

#[cfg(test)]
#[path = "script_balloon_args_tests.rs"]
mod args_tests;
