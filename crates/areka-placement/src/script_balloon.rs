//! 台本のバルーン位置の値（ずらし・向き）の型と引数の読み。

use bevy_ecs::lifecycle::HookContext;
use bevy_ecs::prelude::*;
use bevy_ecs::world::DeferredWorld;
use tracing::{debug, info};

use crate::config::{BalloonSide, BalloonXMode, ScopeConfig};
use crate::reseed::{StandardScope, resolve_descript_scopes, resolve_standard_scopes};
use crate::reset_pos::DefaultPlacementBasis;
use crate::resolver::PointPx;
use crate::spawn::GhostWindows;

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

/// キャラ窓に付く、そのスコープの台本のバルーン位置の値（欄ごとに独立）。
#[derive(Component, Debug, Clone, PartialEq, Eq)]
#[component(on_remove = log_script_balloon_dropped)]
pub struct ScriptBalloonPlacement {
    pub scope: usize,
    pub offset: Option<ScriptOffset>,
    pub align: Option<ScriptAlign>,
}

/// 着地の前に台本の値を当てたキャラ窓の印（着地した巡に解き直す）。
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScriptBalloonUnsettled;

fn log_script_balloon_dropped(world: DeferredWorld, hook: HookContext) {
    let Some(value) = world.get::<ScriptBalloonPlacement>(hook.entity) else {
        return;
    };
    info!(
        event = "script_balloon_dropped",
        scope = value.scope,
        "台本のバルーン位置の値を捨てた"
    );
}

/// 台本の値が効いているか（Component があるか）。
pub(crate) fn is_script_placed(world: &World, char_window: Entity) -> bool {
    world.get::<ScriptBalloonPlacement>(char_window).is_some()
}

/// ずらしを descript の生値へ重ねる（`scale_then_merge` の前に当てる）。
pub(crate) fn overlay_offset(sc: &mut ScopeConfig, offset: &ScriptOffset) {
    let (dx, dy) = sc.balloon_offset.unwrap_or((0, 0));
    let axis = |axis: OffsetAxis, d: i32| match axis {
        OffsetAxis::Absolute(v) => v,
        OffsetAxis::Relative(v) => d.saturating_add(v),
    };
    let (x, y) = (axis(offset.x, dx), axis(offset.y, dy));
    sc.balloon_offset = Some((x, y));
    sc.balloon_offset_descript_x = x;
}

/// 向きを合流の後の値へ重ねる（`windowposition.x` の語を上書きし返すため後に当てる）。
pub(crate) fn overlay_align(sc: &mut ScopeConfig, align: &ScriptAlign) {
    let (side, mode) = align.side_and_mode();
    if let Some(side) = side {
        sc.balloon_alignment = side;
    }
    sc.balloon_x_mode = mode;
}

/// 台本の値の分（台本ありの標準 − descript だけの標準）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScriptShare {
    /// 台本の値が無い（分は 0）。
    NotPlaced,
    /// キャラ窓の左上からの相対の差（物理 px・今の表示 DPI）。
    Share(PointPx),
    /// 解けなかった理由。
    Unresolved(&'static str),
}

/// そのスコープの台本の値の分を、今の窓の大きさと DPI で解く。
pub(crate) fn script_share(world: &World, scope: usize) -> ScriptShare {
    let Some(windows) = world.get_resource::<GhostWindows>() else {
        return ScriptShare::Unresolved("no_ghost_windows");
    };
    let Some(char_window) = windows.char_window(scope) else {
        return ScriptShare::Unresolved("no_char_window");
    };
    if !is_script_placed(world, char_window) {
        return ScriptShare::NotPlaced;
    }
    let Some(basis) = world.get_resource::<DefaultPlacementBasis>() else {
        return ScriptShare::Unresolved("no_default_basis");
    };
    let offset_of = |solved: Vec<StandardScope>| {
        solved
            .into_iter()
            .find(|s| s.placement.scope == scope)
            .map(|s| s.placement.balloon_offset)
    };
    let script = offset_of(resolve_standard_scopes(
        world,
        windows,
        &basis.source,
        &basis.balloon,
    ));
    let descript = offset_of(resolve_descript_scopes(
        world,
        windows,
        &basis.source,
        &basis.balloon,
    ));
    match (script, descript) {
        (Some(a), Some(b)) => ScriptShare::Share(PointPx {
            x: a.x.saturating_sub(b.x),
            y: a.y.saturating_sub(b.y),
        }),
        _ => ScriptShare::Unresolved("not_resolved"),
    }
}

#[cfg(test)]
#[path = "script_balloon_apply_tests.rs"]
mod apply_tests;

#[cfg(test)]
#[path = "script_balloon_args_tests.rs"]
mod args_tests;

#[cfg(test)]
#[path = "script_balloon_overlay_tests.rs"]
mod overlay_tests;

#[cfg(test)]
#[path = "script_balloon_test_support.rs"]
mod test_support;
