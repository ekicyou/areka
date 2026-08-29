// =============================================================================
// zorder drain 相の決定論テスト用の道具立て（実機・実ディスプレイ不要・要件 10.1）
//
// 2 本の兄弟テスト（指令の適用／台帳の射影）が同じ World の組み方を使うので、道具は
// ここへ 1 つだけ置く。二重に持つと、片方だけを直したときに檻の前提が静かにずれる。
// =============================================================================

use super::*;

use bevy_ecs::world::World;
use log_capture_kit::{LineFormat, capture_lines};
use std::sync::mpsc::{Sender, channel};

use crate::placement::resolver::{Anchor, PointPx, ScopePlacement, SizePx};
use crate::placement::source::GhostTitles;
use crate::placement::spawn::spawn_ghost_windows;

/// 1 スコープぶんの合成配置（値は互いに重ならない程度に散らしただけで、意味を持たない）。
///
/// この相は窓を 1 mm も動かさないので、位置も寸法も判定に一切関与しない
/// （要件 11.1——本機能は窓の位置と寸法を変更しない）。
pub(super) fn placement(scope: usize) -> ScopePlacement {
    let base = 100 * (scope as i32 + 1);
    ScopePlacement {
        scope,
        char_pos: PointPx { x: base, y: base },
        char_size: SizePx { w: 200, h: 300 },
        balloon_pos: PointPx {
            x: base + 220,
            y: base,
        },
        balloon_size: SizePx { w: 180, h: 120 },
        balloon_offset: PointPx { x: 220, y: 0 },
        balloon_limit: false,
        anchor: Anchor::Bottom,
        balloon_keyword_base: None,
    }
}

/// 指定したスコープの窓だけを持つ World を組む（`GhostWindows` は Resource として載る）。
///
/// 「まだ現れていないスコープ」はこの一覧から外すことで作る——台帳は書かれたスコープを
/// そのまま持ち続けるので、窓の有無だけが射影の入力になる（要件 1.4）。
pub(super) fn world_with_scopes(scopes: &[usize]) -> World {
    let mut world = World::new();
    spawn_scopes(&mut world, scopes);
    world
}

/// 既存の World へスコープ一式を（作り直して）載せる。
///
/// `spawn_ghost_windows` は正本ごと差し替えるので、`scopes` は**その時点で在る窓の全体**
/// である。後から 1 スコープ増やす検証は「前の一覧＋新しいスコープ」を渡して呼ぶ。
pub(super) fn spawn_scopes(world: &mut World, scopes: &[usize]) {
    let placements: Vec<ScopePlacement> = scopes.iter().map(|s| placement(*s)).collect();
    let titles = GhostTitles::from_scope_titles(
        scopes
            .iter()
            .map(|s| (*s, format!("scope-{s}")))
            .collect::<Vec<_>>(),
    );
    spawn_ghost_windows(world, &placements, &titles);
}

/// World に載っている `GhostWindows` の写し（テストが entity を引くための読み口）。
pub(super) fn ghost_windows(world: &World) -> GhostWindows {
    world
        .get_resource::<GhostWindows>()
        .expect("GhostWindows が載っていない World で窓を引こうとした")
        .clone()
}

/// スコープのバルーン窓 entity。
pub(super) fn balloon_of(world: &World, scope: usize) -> Entity {
    ghost_windows(world)
        .balloon_window(scope)
        .unwrap_or_else(|| panic!("scope {scope} のバルーン窓が無い"))
}

/// スコープのキャラ窓 entity。
pub(super) fn char_of(world: &World, scope: usize) -> Entity {
    ghost_windows(world)
        .char_window(scope)
        .unwrap_or_else(|| panic!("scope {scope} のキャラ窓が無い"))
}

/// `\![set,zorder,tokens...]` 相当の指令。
pub(super) fn set_directive(tokens: &[&str]) -> ZOrderDirective {
    ZOrderDirective::Set {
        tokens: tokens.iter().map(|t| (*t).to_string()).collect(),
    }
}

/// 送信端と受信端の対（送信端は複製できるので、複数の指令を順に流せる）。
pub(super) fn directive_channel() -> (Sender<ZOrderDirective>, Receiver<ZOrderDirective>) {
    channel()
}

/// 受け口（`ZOrderGroups`）の現在の内容を `(グループ id, 窓の列)` の並びで読む。
///
/// Resource そのものが無いときは `None`——「空の受け口が在る」と「受け口が無い」は
/// 要件 6.1 の判定で意味が違うので、潰さずに区別する。
pub(super) fn projected(world: &World) -> Option<Vec<(u32, Vec<Entity>)>> {
    world.get_resource::<ZOrderGroups>().map(|groups| {
        groups
            .groups
            .iter()
            .map(|spec| (spec.id, spec.members.clone()))
            .collect()
    })
}

/// 受け口の印（`pending`）。Resource が無いときは `None`。
pub(super) fn pending(world: &World) -> Option<bool> {
    world.get_resource::<ZOrderGroups>().map(|g| g.pending)
}

/// 受け口の印を倒す（次の巡で「立ったかどうか」を測るための下ごしらえ）。
pub(super) fn clear_pending(world: &mut World) {
    world
        .get_resource_mut::<ZOrderGroups>()
        .expect("受け口が無い World で印を倒そうとした")
        .pending = false;
}

/// クロージャ実行中に**現在のスレッド**で発火した記録を 1 行 1 件で返す。
///
/// 硬化機構の唯一の定義元 `log-capture-kit` の捕捉窓へ委譲する
/// （`zorder_cue_tests.rs` と同じ流儀）。
pub(super) fn capture_logs<F: FnOnce()>(f: F) -> Vec<String> {
    let ((), lines) = capture_lines(LineFormat::LevelTargetFields, f);
    lines
}

/// 捕捉行のうち、指定した字面を含むものだけを返す。
pub(super) fn lines_with<'a>(logs: &'a [String], needle: &str) -> Vec<&'a str> {
    logs.iter()
        .filter(|line| line.contains(needle))
        .map(String::as_str)
        .collect()
}

/// 受理・拒否の記録の出力先（実機サインオフの grep 対象と同じ 1 本）。
///
/// 要件 9.5 の保全対象である `[zorder-group] applied`／`rejected` は、退役する
/// `zorder_group` 系から `zorder_chain_diag` へ移設された。**タグの字面は 1 字も
/// 変わっておらず**、変わったのは `tracing` の出力先（module path 既定）だけである。
/// サインオフの `RUST_LOG` は `wintf::ecs::window::zorder_chain=debug` を含み、
/// 指定は前方一致なのでこの出力先を点灯させる（判定に影響しない）。
pub(super) const GROUP_TARGET: &str = "target=wintf::ecs::window::zorder_chain_diag";

/// 不在メンバーの見送り（`[zorder-group] skip reason=MemberMissing`）の出力先。
///
/// この記録だけは移設の対象ではない——要件 9.5 の保全対象は受理・拒否の 2 語であり、
/// こちらは退役予定（後継は `[zorder-chain] absent`・要件 8.4）だからである。よって
/// 出力先も退役予定のモジュールのままである。
pub(super) const GROUP_SKIP_TARGET: &str = "target=wintf::ecs::window::zorder_group";
