//! ghost の読み直しでシェルの状態（スコープごとの表示中の面と着せ替え）を作り直した窓へ持ち越す
//! （areka-P0-mcp-reload 要件 3.8・2026-10-19 の開発者の裁定）。
//!
//! ghost の読み直しは自分自身への切替と同じく窓を閉じて作り直す（要件 3.1）。新しい窓のシェルは
//! 最初の `\s` まで出さない決まり（defect #5）で、読み直しは起動の挨拶を送らない（要件 3.3）ので、
//! 控えが無いと姿が出ないまま残り、着せ替えも既定へ戻る。そこで、窓口自身の ghost の読み直しのときだけ:
//!
//! 1. 降ろす前に、表示層が最後に確立した各スコープのシェルの面と着せ替えの集合を控える
//!    （[`capture_for_reload`]）。
//! 2. 起こし直しが成功したら、控えを新しい結線（`Emo2Wiring`）へ渡す（[`hand_over`]）。
//! 3. 装着の相が、読み直したシェルにその面が在るかを確かめ（[`decide`]）、無ければ既定の面へ戻す。
//!    着せ替えは、読み直しの前のシェルの既定との差（台本で着た・脱いだ分）だけを、読み直したシェルの
//!    既定の上に、読み直したシェルの名前で `\![bind,カテゴリ,パーツ,1|0]` にし（[`decide_binds`]）、
//!    名前の無くなった着せ替えは落とす（シェルの descript の既定の書き換えは読み直しで効く）。
//! 4. 新しい seriko へ、着せ替えを `\![bind,…]` と、面を `\s[面]` と同じ合図で、着せ替えから先に送る
//!    （[`replay`]・面の出ていないスコープの着せ替えは状態だけが変わるので、最初の `Show` から着せ替えた
//!    姿になる）。seriko の状態・ループのアニメも台本と同じ道で立つ。
//!
//! シェル側のパターン（アニメ）の進行は持ち越さない（新しい seriko で最初から動く）。バルーンの面も
//! 持ち越さない（バルーンは台詞の外では見えず、次の台詞の `\b` が決める）。

use std::collections::BTreeMap;

use areka_emo_compose::BindSet;
use areka_seriko::{BIND_COMMAND, BindChoicePolicy, BindResolver, scope_namespace};
use bevy_ecs::world::World;
use dola::DynamicValue;
use dola::cue::{ActorKey, CueCommand, CueSink, TalkCue};
use tracing::{info, warn};

use super::frame::Emo2Wiring;
use super::target_map::shell_target;
use crate::ghost_session::GhostSlot;
use crate::placement::spawn::GhostWindows;

/// 控えたシェルの状態（スコープ → 表示中の面・最後に表示したときの着せ替えの集合）。
/// 非表示・未表示のスコープは面が載らない。一度も表示していないスコープは着せ替えも載らない。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ShellCarry {
    surfaces: BTreeMap<u32, u32>,
    binds: BTreeMap<u32, BindSet>,
    /// 読み直しの前のシェルの静的な着せ替えの既定（控えた着せ替えとの差が台本で着た・脱いだ分）。
    old_static: BindSet,
}

impl ShellCarry {
    /// スコープの控えた面（無ければ `None`＝出し直さない）。
    pub(crate) fn surface(&self, scope: u32) -> Option<u32> {
        self.surfaces.get(&scope).copied()
    }

    /// スコープの控えた着せ替えの集合（無ければ `None`＝出し直さない）。
    pub(crate) fn binds(&self, scope: u32) -> Option<&BindSet> {
        self.binds.get(&scope)
    }

    /// 読み直しの前のシェルの静的な着せ替えの既定。
    pub(crate) fn old_static(&self) -> &BindSet {
        &self.old_static
    }
}

/// 控えた面を読み直したシェルで出し直すときの決め方。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CarriedShow {
    /// 控えた面が読み直したシェルにも在る。
    Carried(u32),
    /// 控えた面が無くなっていたので既定の面へ戻した。
    FellBack { missing: u32, to: u32 },
    /// 控えた面も既定の面も無い（出さない）。
    Skipped { missing: u32 },
}

impl CarriedShow {
    /// 出す面（出さないなら `None`）。
    pub(crate) fn surface(self) -> Option<u32> {
        match self {
            Self::Carried(id) | Self::FellBack { to: id, .. } => Some(id),
            Self::Skipped { .. } => None,
        }
    }
}

/// 出し直す着せ替え 1 件（`\![bind,category,part,1|0]` と同じ合図になる）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BindReplay {
    pub(crate) category: String,
    pub(crate) part: String,
    pub(crate) on: bool,
}

/// 控えた着せ替えを読み直したシェルで出し直すときの決め方。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CarriedBinds {
    /// 出し直す着せ替え（着る方が先・脱ぐ方が後）。
    pub(crate) replays: Vec<BindReplay>,
    /// 読み直したシェルに名前が無く、出し直せない着せ替えの ID（`warn!` で残す）。
    pub(crate) dropped: Vec<u32>,
}

/// 控えた面 `carried` を読み直したシェルで出せるか決める（純粋）。在ればそのまま、無ければ既定の面
/// `initial`（scope 0 は 0・ほかは 10）、それも無ければ出さない。
pub(crate) fn decide(carried: u32, initial: u32, exists: impl Fn(u32) -> bool) -> CarriedShow {
    if exists(carried) {
        CarriedShow::Carried(carried)
    } else if exists(initial) {
        CarriedShow::FellBack {
            missing: carried,
            to: initial,
        }
    } else {
        CarriedShow::Skipped { missing: carried }
    }
}

/// 控えた着せ替えの集合 `carried` のうち、読み直しの前の既定 `old_static` との差（台本で着た・脱いだ分）
/// だけを、読み直したシェルの既定 `new_static` の上に名前表 `resolver` で出し直す合図にする（純粋）。
/// 着せ替えは ID（シェルの bindgroup の番号）で同じものとみなす。既定そのものの違い（descript の
/// `*.bindgroupN.default` の書き換え）は持ち越さない＝読み直したシェルの既定がそのまま効く。
///
/// - 台本で着た ID（`carried − old_static`）のうち新しい既定に無いもの → 名前があれば着る（`on`）・
///   無ければ落とす。
/// - 台本で脱いだ ID（`old_static − carried`）のうち新しい既定にあるもの → 名前があれば脱ぐ（`off`）・
///   無ければ落とす。ただし同じカテゴリを着る合図があり、そのカテゴリが複数可（`multiple`）でなければ
///   送らない（着る方の排他の置き換えが外す・mustselect の脱衣の警告を出さない）。
/// - スコープに名前空間が無い（scope 2 以降）→ 違いは全部落とす（seriko が着せ替えを扱えない）。
pub(crate) fn decide_binds(
    scope: u32,
    carried: &BindSet,
    old_static: &BindSet,
    new_static: &BindSet,
    resolver: &BindResolver,
) -> CarriedBinds {
    let wear: Vec<u32> = carried
        .ids()
        .iter()
        .copied()
        .filter(|id| !old_static.contains(*id) && !new_static.contains(*id))
        .collect();
    let shed: Vec<u32> = old_static
        .ids()
        .iter()
        .copied()
        .filter(|id| !carried.contains(*id) && new_static.contains(*id))
        .collect();
    let Some(ns) = scope_namespace(&ActorKey::from(scope.to_string())) else {
        return CarriedBinds {
            replays: Vec::new(),
            dropped: wear.into_iter().chain(shed).collect(),
        };
    };
    let mut out = CarriedBinds::default();
    for id in wear {
        match resolver.name_of(ns, id) {
            Some((category, part)) => out.replays.push(BindReplay {
                category: category.to_owned(),
                part: part.to_owned(),
                on: true,
            }),
            None => out.dropped.push(id),
        }
    }
    let worn: Vec<String> = out.replays.iter().map(|r| r.category.clone()).collect();
    for id in shed {
        match resolver.name_of(ns, id) {
            Some((category, _))
                if worn.iter().any(|c| c == category)
                    && resolver.policy(ns, category) != BindChoicePolicy::Multiple => {}
            Some((category, part)) => out.replays.push(BindReplay {
                category: category.to_owned(),
                part: part.to_owned(),
                on: false,
            }),
            None => out.dropped.push(id),
        }
    }
    out
}

/// スコープの並びと「スコープ → 表示中の面」「スコープ → 最後に表示した着せ替え」の読み手と、
/// 読み直しの前のシェルの既定 `old_static` から控えを組む（純粋）。
pub(crate) fn snapshot(
    scopes: impl IntoIterator<Item = u32>,
    current: impl Fn(u32) -> Option<u32>,
    binds: impl Fn(u32) -> Option<BindSet>,
    old_static: BindSet,
) -> ShellCarry {
    let mut carry = ShellCarry {
        old_static,
        ..ShellCarry::default()
    };
    for scope in scopes {
        if let Some(id) = current(scope) {
            carry.surfaces.insert(scope, id);
        }
        if let Some(set) = binds(scope) {
            carry.binds.insert(scope, set);
        }
    }
    carry
}

/// 窓口自身の ghost の読み直しのときだけ、降ろす前に表示中の面と着せ替えを控える（それ以外は
/// `None`＝今までどおり）。結線か窓の一覧が無ければ空の控え（`warn!` を 1 件）。
pub(crate) fn capture_for_reload(world: &World) -> Option<ShellCarry> {
    if !super::reload::carries_shell_state(world) {
        return None;
    }
    let (Some(wiring), Some(windows)) = (
        world.get_non_send::<Emo2Wiring>(),
        world.get_resource::<GhostWindows>(),
    ) else {
        warn!(
            event = "reload_shell_carry_unavailable",
            "表示の結線か窓の一覧が無いので、読み直しの前の面を控えられない"
        );
        return Some(ShellCarry::default());
    };
    let presenter = wiring.presenter();
    let carry = snapshot(
        windows.scopes().filter_map(|s| u32::try_from(s).ok()),
        |scope| presenter.current_surface_id(shell_target(scope)),
        |scope| presenter.last_binds(shell_target(scope)).cloned(),
        wiring.static_binds.clone(),
    );
    info!(
        event = "reload_shell_captured",
        surfaces = ?carry.surfaces,
        binds = ?carry.binds,
        old_static = ?carry.old_static,
        "読み直しの前に表示中の面と着せ替えを控えた"
    );
    Some(carry)
}

/// 起こし直しが成功した後に、控えを新しい結線へ渡す（装着の相が 1 度だけ使う）。`None` なら何もしない。
pub(crate) fn hand_over(world: &mut World, carry: Option<ShellCarry>) {
    let Some(carry) = carry else {
        return;
    };
    match world.get_non_send_mut::<Emo2Wiring>() {
        Some(mut wiring) => wiring.shell_carry = Some(carry),
        None => warn!(
            event = "reload_shell_carry_unavailable",
            "新しい表示の結線が無いので、読み直しの前の面を渡せない"
        ),
    }
}

/// 装着の相で決めた面と着せ替えを置き場のゴーストの seriko へ、着せ替えは `\![bind,…]`・面は
/// `\s[面]` と同じ合図で送る（着せ替えが先: 面の出ていないスコープの着せ替えは状態だけが変わり、
/// 続く面の最初の `Show` が着せ替えた集合を載せる＝既定の姿が 1 度も出ない）。決め方ごとに記録を 1 件残す（戻した・出さない・落とした
/// は `warn!`）。
pub(crate) fn replay(world: &World, shows: &[(u32, CarriedShow)], binds: &[(u32, CarriedBinds)]) {
    if shows.is_empty() && binds.is_empty() {
        return;
    }
    for (scope, decided) in binds {
        for r in &decided.replays {
            info!(
                event = "reload_shell_bind_restored",
                scope,
                category = %r.category,
                part = %r.part,
                on = r.on,
                "読み直しの前の着せ替えを出し直す"
            );
        }
        for id in &decided.dropped {
            warn!(
                event = "reload_shell_bind_dropped",
                scope, id, "読み直しの前の着せ替えが読み直したシェルで名前を持たない——持ち越さない"
            );
        }
    }
    for &(scope, show) in shows {
        match show {
            CarriedShow::Carried(id) => info!(
                event = "reload_shell_restored",
                scope,
                surface = id,
                "読み直しの前の面を出し直す"
            ),
            CarriedShow::FellBack { missing, to } => warn!(
                event = "reload_shell_fell_back",
                scope, missing, to, "読み直しの前の面が読み直したシェルに無い——既定の面を出す"
            ),
            CarriedShow::Skipped { missing } => warn!(
                event = "reload_shell_skipped",
                scope, missing, "読み直しの前の面も既定の面も読み直したシェルに無い——面を出さない"
            ),
        }
    }
    let Some(sink) = world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref())
        .and_then(|session| session.seriko_sink())
    else {
        warn!(
            event = "reload_shell_carry_unavailable",
            "置き場のゴーストの seriko が無いので、読み直しの前の面を出し直せない"
        );
        return;
    };
    let mut sink = sink.clone();
    emit_binds(&mut sink, binds);
    emit_shows(&mut sink, shows);
}

/// 出す面ごとに `\s[面]` と同じ合図（時刻 0・長さ 0）を送る（出さないものは送らない）。
pub(crate) fn emit_shows(sink: &mut impl CueSink, shows: &[(u32, CarriedShow)]) {
    for &(scope, show) in shows {
        if let Some(id) = show.surface() {
            sink.emit(cue(
                scope,
                CueCommand::Emote {
                    key: id.to_string(),
                },
            ));
        }
    }
}

/// 出し直す着せ替えごとに `\![bind,カテゴリ,パーツ,1|0]` と同じ合図（時刻 0・長さ 0）を送る。
pub(crate) fn emit_binds(sink: &mut impl CueSink, binds: &[(u32, CarriedBinds)]) {
    for (scope, decided) in binds {
        for r in &decided.replays {
            let params = [
                r.category.as_str(),
                r.part.as_str(),
                if r.on { "1" } else { "0" },
            ]
            .into_iter()
            .map(|t| DynamicValue::String(t.to_owned()))
            .collect();
            sink.emit(cue(
                *scope,
                CueCommand::Custom {
                    command: BIND_COMMAND.to_owned(),
                    params: DynamicValue::Array(params),
                },
            ));
        }
    }
}

fn cue(scope: u32, command: CueCommand) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from(scope.to_string()),
        command,
        duration: 0.0,
    }
}

#[cfg(test)]
#[path = "shell_carry_tests.rs"]
mod tests;
