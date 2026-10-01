//! シェルの差し替えで、新しいシェルの `descript.txt` の見た目の値を走っている窓の部品へ
//! 入れ直す（areka-P0-shell-balloon-switch 要件 2.7・design「Reseed」）。
//!
//! 起動と同じ純関数を同じ順で通す: 配置の設定（[`build_placement_config`]）→ 作者の空間の
//! バルーンのずらしを物理 px へ（[`apply_author_balloon_offset_scale`]）→ バルーンの
//! `windowposition` を合流（[`merge_scope_windowpositions`]）→ 配置の解決
//! （[`resolve_placement`]・起動と同じく全スコープを昇順で 1 度に通す＝連鎖 P2 を保つ）→
//! 利用者のドラッグの記憶を重ねる（[`apply_restored_placements`]）。
//! 結果からはスコープごとの揃え方（`Anchored`）・バルーンのずらしの基準（`BalloonFollow`）・
//! キーワードの素材（`BalloonKeywordBase`）だけを取り、キャラ窓の位置は捨てる。
//!
//! [`apply_shell_descript`] が触らないもの: 窓の位置（`WindowPos`）・スコープの集合（起動時の
//! [`GhostWindows`]）。揃え方が変わったときのキャラ窓の置き直しは、続けて呼ぶ
//! [`reanchor_char_windows`] が今の窓寸のまま行う（窓の大きさとスコープの集合は変えない）。
//! ファイルは読まない（新しいシェルの値・走っているバルーンの `windowposition` と作者の DPI・
//! 位置の記憶は、背景の資産づくりが読んでおいたもの）。

use areka_emo_compose::ScaleRatio;
use areka_sylphya::PersistKey;
use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;
use tracing::{debug, warn};
use wintf::ecs::drag::DragConfig;
use wintf::ecs::{DPI, Point, WindowPos};

use super::config::build_placement_config;
use super::diag::PlacementRoute;
use super::follow::{
    Anchored, BalloonFollow, BalloonFollowTrigger, MonitorSnapshot, follow_balloon,
    resize_window_to, work_area_for_window,
};
use super::persist::apply_restored_placements;
use super::resolver::{RectPx, ScopeInput, SizePx, resolve_placement};
use super::source::DescriptSource;
use super::spawn::{BalloonKeywordBase, GhostWindows};
use super::{ScopeWindowPositions, apply_author_balloon_offset_scale, merge_scope_windowpositions};

/// 走っているバルーンの配置の値（背景の資産づくりがバルーンのフォルダから読んでおいたもの）。
///
/// シェルを替えてもバルーンは替わらないが、配置の設定は新しいシェルから組み直すので、
/// 起動の準備と同じくバルーン側の値をもう一度合流させる必要がある（要件 2.7）。
#[derive(Debug, Clone)]
pub(crate) struct BalloonPlacementInputs {
    /// バルーンの作者の DPI（`dpi`）。`windowposition` の換算の分母。
    pub author_dpi: u16,
    /// scope ごとの `windowposition`（[`super::load_scope_windowpositions`] の結果）。
    pub windowpositions: ScopeWindowPositions,
}

/// 新しいシェルの見た目の値を走っている窓の部品へ入れ直し、バルーン窓を 1 度置き直す。
///
/// - 新しい設定に在って走っている窓に無いスコープは `debug!` で読み飛ばす（窓を作らない）。
/// - 窓の部品が揃わないスコープは `warn!`（`event = "reseed_skipped"`）を残して読み飛ばし、
///   残りのスコープは続ける（差し替えは済んでいるので、配置の値だけが古いまま残る）。
pub(crate) fn apply_shell_descript(
    world: &mut World,
    windows: &GhostWindows,
    src: &DescriptSource,
    balloon: &BalloonPlacementInputs,
    restored: &[(PersistKey, String)],
) {
    let mut cfg = build_placement_config(&src.ghost_kv, &src.shell_kv);
    for &scope in cfg.scopes.keys() {
        if windows.char_window(scope).is_none() {
            debug!(
                event = "reseed_scope_not_running",
                scope, "reseed: 新しいシェルの設定に在るが走っている窓に無いスコープを読み飛ばす"
            );
        }
    }
    let snapshot = world
        .get_resource::<MonitorSnapshot>()
        .cloned()
        .unwrap_or(MonitorSnapshot {
            work_areas: Vec::new(),
        });
    let shell_author_dpi = src.shell_author_dpi();

    // 1 周目: 走っているスコープの窓の部品を集め、スコープごとの構成へその窓の DPI で換算と
    // 合流を 1 度ずつ当てる（拡大率はスコープの窓ごと・軸ごと＝シェル／バルーン。他のスコープへ
    // 漏らさない）。
    let mut running_scopes = Vec::new();
    for scope in windows.scopes() {
        let running = match running_scope(world, windows, scope) {
            Ok(running) => running,
            Err(reason) => {
                warn!(
                    event = "reseed_skipped",
                    scope,
                    reason,
                    "reseed: 窓の部品が揃わないので見た目の値を入れ直さない（古い値のまま）"
                );
                continue;
            }
        };
        // 新しい設定に無い走っているスコープは、解決が使うのと同じ既定の構成を置いて合流先にする
        // （起動では設定の全スコープに窓があるので、合流先が無いことは起きない）。
        cfg.scopes.entry(scope).or_default();
        // 窓の DPI は 0 を弾いてあり、作者の DPI は縮退梯子が常に非 0 を返すので `ONE` へは
        // 落ちない。順序は起動の準備と同じ（シェル軸の換算 → 合流）。
        let k = |author: u16| {
            ScaleRatio::new(u32::from(running.dpi.dpi_x), u32::from(author))
                .unwrap_or(ScaleRatio::ONE)
        };
        apply_author_balloon_offset_scale(&mut cfg, &[scope], k(shell_author_dpi));
        merge_scope_windowpositions(
            &mut cfg,
            &balloon.windowpositions,
            &[scope],
            k(balloon.author_dpi),
        );
        running_scopes.push(running);
    }
    // 解決は起動と同じく全スコープを昇順に並べて 1 度に通す——スコープ 1 以降の仮の X は前の
    // スコープの X に連なり（P2）、`balloon.alignment,none` の側はその仮の X の中心で決まる（P5）。
    let inputs: Vec<ScopeInput> = running_scopes.iter().map(|r| r.input).collect();

    // 2 周目: スコープごとにその窓の作業領域と DPI で解き、自分の要素だけを取る。
    for running in &running_scopes {
        let scope = running.input.scope;
        // 引けなければ窓の矩形で代える（架空の矩形は作らない）。
        let work_area =
            work_area_for_window(&snapshot, running.char_rect).unwrap_or(running.char_rect);
        let own: Vec<_> = resolve_placement(&cfg, work_area, &inputs, running.dpi)
            .into_iter()
            .filter(|p| p.scope == scope)
            .collect();
        // 自分の要素はちょうど 1 件（resolver・merge の事後条件）。破れたら黙って進めない。
        let Some(p) = apply_restored_placements(own, restored, &snapshot).pop() else {
            warn!(
                event = "reseed_skipped",
                scope,
                reason = "resolve_empty",
                "reseed: 配置の解決が空を返した（見た目の値を入れ直さない）"
            );
            continue;
        };

        let c = running.char_window;
        world.entity_mut(c).insert((
            Anchored(p.anchor),
            BalloonFollow::new(running.balloon_window, p.balloon_offset_base),
        ));
        // ドラッグの単一ライターは揃え方から導く（spawn と同じ規則）。
        if let Some(mut drag) = world.get_mut::<DragConfig>(c) {
            drag.move_window = p.anchor.is_free();
        }
        match p.balloon_keyword_base {
            Some((mode, adjust)) => {
                world
                    .entity_mut(c)
                    .insert(BalloonKeywordBase { mode, adjust });
            }
            None => {
                world.entity_mut(c).remove::<BalloonKeywordBase>();
            }
        }
        // 置き直しは記憶の復元と同じ明示の配置として扱う（可視性の遷移ガードは掛けない）。
        follow_balloon(
            world,
            c,
            running.char_pos,
            BalloonFollowTrigger::Placement(PlacementRoute::Restore),
        );
    }
}

/// 入れ直した揃え方（`Anchored`）でキャラ窓を今の窓寸のまま置き直す（要件 2.7「揃え方は新しい
/// シェルから読み直す」）。[`apply_shell_descript`] の後に呼ぶ。
///
/// 後段の再スナップは窓寸が変わらないと置き直さないので、揃え方だけが変わった差し替え
/// （bottom → top など）はここで直す。揃え方も窓寸も同じなら [`resize_window_to`] がべき等に
/// 読み飛ばす。窓寸の読めない窓は `warn!`（`event = "reseed_skipped"`）を残して読み飛ばす。
pub(crate) fn reanchor_char_windows(world: &mut World, windows: &GhostWindows) {
    for scope in windows.scopes() {
        let size = windows
            .char_window(scope)
            .and_then(|c| Some((c, world.get::<WindowPos>(c)?.size?)));
        let Some((char_window, size)) = size else {
            warn!(
                event = "reseed_skipped",
                scope,
                reason = "char_size_unknown",
                "reseed: キャラ窓の窓寸が読めないので新しい揃え方で置き直さない"
            );
            continue;
        };
        resize_window_to(
            world,
            char_window,
            SizePx {
                w: size.width,
                h: size.height,
            },
            PlacementRoute::AnchorChange,
        );
    }
}

/// 走っている 1 スコープの窓から読んだ解決の入力。
struct RunningScope {
    char_window: Entity,
    balloon_window: Entity,
    char_pos: Point,
    char_rect: RectPx,
    input: ScopeInput,
    dpi: DPI,
}

/// スコープの窓の部品を読む。揃わなければその理由を返す。
fn running_scope(
    world: &World,
    windows: &GhostWindows,
    scope: usize,
) -> Result<RunningScope, &'static str> {
    let char_window = windows.char_window(scope).ok_or("no_char_window")?;
    let balloon_window = windows.balloon_window(scope).ok_or("no_balloon_window")?;
    let char_wp = world
        .get::<WindowPos>(char_window)
        .ok_or("char_window_gone")?;
    let char_pos = char_wp.position.ok_or("char_position_unknown")?;
    let char_size = char_wp.size.ok_or("char_size_unknown")?;
    let balloon_size = world
        .get::<WindowPos>(balloon_window)
        .and_then(|wp| wp.size)
        .ok_or("balloon_window_gone")?;
    let dpi = *world.get::<DPI>(char_window).ok_or("char_dpi_unknown")?;
    if dpi.dpi_x == 0 {
        return Err("char_dpi_zero");
    }
    world
        .get::<BalloonFollow>(char_window)
        .ok_or("no_balloon_follow")?;
    let char_size = SizePx {
        w: char_size.width,
        h: char_size.height,
    };
    Ok(RunningScope {
        char_window,
        balloon_window,
        char_pos,
        char_rect: RectPx {
            left: char_pos.x,
            top: char_pos.y,
            right: char_pos.x.saturating_add(char_size.w),
            bottom: char_pos.y.saturating_add(char_size.h),
        },
        input: ScopeInput {
            scope,
            char_size,
            balloon_size: SizePx {
                w: balloon_size.width,
                h: balloon_size.height,
            },
        },
        dpi,
    })
}

#[cfg(test)]
#[path = "reseed_tests.rs"]
pub(crate) mod tests;
