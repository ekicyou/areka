//! 台本からの吸着の向きの変更（`\![set,alignmenttodesktop,...]`・areka-P0-window-link-directives
//! 要件 1・design「script_alignment」）。

use bevy_ecs::world::World;
use tracing::{debug, info, warn};
use wintf::ecs::WindowPos;

use super::diag::PlacementRoute;
use super::follow::{AnchorSource, Anchored, resize_window_to, sync_drag_writer};
use super::resolver::{Anchor, SizePx};
use super::spawn::GhostWindows;

/// 台本の吸着の語。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlignmentWord {
    /// top・bottom・left・right・free。
    Anchor(Anchor),
    /// default（descript の向きへ戻す）。
    Default,
    /// 6 つの語のどれでもない値（受け取った値をそのまま持つ）。
    Unknown(String),
    /// 方向の引数が無い。
    Missing,
}

/// `alignmenttodesktop`（`alignmentondesktop`）の後ろの引数を読む。最初の 1 つが方向で、
/// 前後の空白を除き大文字小文字は区別しない（descript の読み `Anchor::from_alignment` と同じ）。
/// 空の値は欠けた値として扱う。2 つ目以降（指令の 3 つ目以降の引数）は `debug!` だけ。
pub fn parse_alignment_word(args: &[&str]) -> AlignmentWord {
    if args.len() > 1 {
        debug!(
            event = "alignment_extra_args",
            extra = ?&args[1..],
            "吸着の指令の 3 つ目以降の引数は読まない"
        );
    }
    let Some(raw) = args.first() else {
        return AlignmentWord::Missing;
    };
    match raw.trim().to_ascii_lowercase().as_str() {
        "" => AlignmentWord::Missing,
        "top" => AlignmentWord::Anchor(Anchor::Top),
        "bottom" => AlignmentWord::Anchor(Anchor::Bottom),
        "left" => AlignmentWord::Anchor(Anchor::Left),
        "right" => AlignmentWord::Anchor(Anchor::Right),
        "free" => AlignmentWord::Anchor(Anchor::Free),
        "default" => AlignmentWord::Default,
        _ => AlignmentWord::Unknown(raw.to_string()),
    }
}

/// 現スコープのキャラクターの窓へ当てる。当てたら真。
///
/// 窓は今の大きさのまま経路 `MoveCue` で新しい向きへ当て直す（`\![move]` と同じ明示の操作＝
/// 記憶に書かず、既定の位置も連れて行かない・design 議題 2）。バルーンは `resize_window_to`
/// の中で付いて動く（要件 1.6）。
pub fn apply_script_alignment(world: &mut World, scope: u32, word: &AlignmentWord) -> bool {
    if *word == AlignmentWord::Missing {
        warn!(
            event = "alignment_skipped",
            scope,
            reason = "missing_value",
            "吸着の指令に方向が無いので向きを変えない"
        );
        return false;
    }
    let Some(windows) = world.get_resource::<GhostWindows>() else {
        warn!(
            event = "alignment_skipped",
            scope,
            reason = "no_ghost_windows",
            "窓の一式が無いので吸着の向きを変えない"
        );
        return false;
    };
    let Some(char_window) = windows.char_window(scope as usize) else {
        warn!(
            event = "alignment_skipped",
            scope,
            reason = "no_char_window",
            "現スコープのキャラクターの窓が無いので吸着の向きを変えない"
        );
        return false;
    };
    let (Some(source), Some(Anchored(from))) = (
        world.get::<AnchorSource>(char_window).copied(),
        world.get::<Anchored>(char_window).copied(),
    ) else {
        warn!(
            event = "alignment_skipped",
            scope,
            reason = "no_anchor_source",
            "キャラクターの窓に吸着の向きの部品が無いので向きを変えない"
        );
        return false;
    };

    let (to, by_script) = match word {
        AlignmentWord::Anchor(a) => (*a, true),
        AlignmentWord::Unknown(w) => {
            warn!(
                event = "alignment_unknown_word",
                scope,
                word = %w,
                "吸着の指令の方向が読めないので free と同じに外す"
            );
            (Anchor::Free, true)
        }
        AlignmentWord::Default => (source.descript, false),
        // 関数の入口で記録して返した。
        AlignmentWord::Missing => return false,
    };

    world.entity_mut(char_window).insert((
        Anchored(to),
        AnchorSource {
            descript: source.descript,
            by_script,
        },
    ));
    sync_drag_writer(world);

    let size = world
        .get::<WindowPos>(char_window)
        .and_then(|wp| wp.size)
        .filter(|s| s.width > 0 && s.height > 0);
    match size {
        Some(s) => {
            resize_window_to(
                world,
                char_window,
                SizePx {
                    w: s.width,
                    h: s.height,
                },
                PlacementRoute::MoveCue,
            );
        }
        None => warn!(
            event = "alignment_reanchor_skipped",
            scope,
            reason = "char_size_unknown",
            ?to,
            "キャラクターの窓の大きさが読めないので向きだけ変えて窓は動かさない"
        ),
    }

    info!(
        event = "alignment_changed",
        scope,
        from = ?from,
        to = ?to,
        source = if by_script { "script" } else { "descript" },
        "吸着の向きを変えた"
    );
    true
}

#[cfg(test)]
#[path = "script_alignment_tests.rs"]
mod tests;
