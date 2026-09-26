//! ゴーストの切替の入口（areka-P0-ghost-shell-balloon-switch・design「GhostSwitch」）。
//!
//! 切替要求は台本の `\![change,ghost,…]` とメニューの「ゴースト」枠の 2 つの出どころから、
//! ここの [`request_ghost_switch`] 1 本だけを通る（要件 1.1）。入口は降ろす前・`OnGhostChanging`
//! を送る前に目録と突き合わせ（要件 1.5〜1.7）、受理したら切替の予約 [`SwitchInFlight`] を
//! 立てて kanade へ切替の要求を送る。

// 入口の本番の呼び手は 7.2（台本の取り出しの系）と 7.3（メニューの「ゴースト」枠）。それまでは
// test からだけ呼ぶ。
#![cfg_attr(not(test), allow(dead_code))]

use std::path::PathBuf;

use areka_ghost::GhostEntry;
use areka_kanade::{ChangeOrigin, ChangeRequest, ChangeTarget, KanadeMsg};
use bevy_ecs::world::World;

use crate::boot_config::BootContext;
use crate::ghost_session::GhostSlot;

/// 切替要求（要件 1.1 の入口の型）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SwitchRequest {
    pub ghost: GhostSpec,
    /// 真なら送り出す側へ `OnGhostChanging` を送る。
    pub raise_event: bool,
    pub origin: ChangeOrigin,
}

/// 切替先の指し方。台本は名前（`descript.txt` の `name` → フォルダ名の順）、メニューはフォルダ名。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GhostSpec {
    Name(String),
    Folder(String),
}

/// 入口の判定（記録の語彙・呼び手は分岐しない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SwitchVerdict {
    Accepted,
    Busy,
    NotFound,
    NoContext,
}

/// 切替先（`OnGhostChanging` の Ref0〜3 の材料）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SwitchTarget {
    pub dir: PathBuf,
    pub folder: String,
    /// `descript.txt` の `name`（無ければフォルダ名）。
    pub name: String,
    /// `descript.txt` の `sakura.name`（切替先だけ読む・要件 8.7）。
    pub sakura_name: Option<String>,
}

/// 直前のゴースト（`OnGhostChanged` の Ref0・2・3 の材料）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PrevGhost {
    pub dir: PathBuf,
    pub name: Option<String>,
    pub sakura_name: Option<String>,
}

/// 切替の段。迎え入れの段（`Welcoming`）は切替先を起こす 8.2 が足す。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SwitchStage {
    /// kanade へ切替の要求を送り、送り出しの握手の終わり（停止通知）を待っている。
    SendOff,
}

/// 切替の予約（World の NonSend・在れば切替中）。二重要求の判定はこの有無で行う（要件 1.9）。
#[derive(Debug)]
pub(crate) struct SwitchInFlight {
    pub target: SwitchTarget,
    pub prev: PrevGhost,
    pub stage: SwitchStage,
}

/// 純粋: 目録の項目と指し方から切替先を決める（要件 1.5・1.8・11.8）。
///
/// 名前は `descript.txt` の `name` と先に突き合わせ、一致が無ければフォルダ名と突き合わせる。
/// フォルダ名の指し方はフォルダ名とだけ突き合わせる。比較は大文字小文字を区別し、今のゴーストも
/// 除外しない。`sakura_name` は fs を読むので入口が埋める（ここでは `None`）。
pub(crate) fn resolve_switch_target(
    entries: &[GhostEntry],
    spec: &GhostSpec,
) -> Option<SwitchTarget> {
    let by_folder = |want: &str| entries.iter().find(|e| e.identity.folder == want);
    let hit = match spec {
        GhostSpec::Name(want) => entries
            .iter()
            .find(|e| e.identity.name.as_deref() == Some(want.as_str()))
            .or_else(|| by_folder(want)),
        GhostSpec::Folder(want) => by_folder(want),
    }?;
    Some(SwitchTarget {
        dir: hit.dir.clone(),
        folder: hit.identity.folder.clone(),
        name: hit
            .identity
            .name
            .clone()
            .unwrap_or_else(|| hit.identity.folder.clone()),
        sakura_name: None,
    })
}

/// 切替要求の唯一の入口（要件 1.1・1.6・1.7・1.9）。
///
/// ⑴ 予約が在れば `warn!(ghost_switch_busy)`。⑵ 起動の文脈の根で目録を読み突き合わせ、該当なしは
/// `warn!(ghost_switch_unknown)`（降ろさず kanade へ何も送らない）。⑶ 置き場のゴーストの送出端が
/// 無ければ `warn!(ghost_switch_no_context)`。⑷ 切替先の `sakura.name` だけを読み（要件 8.7）、
/// kanade へ切替の要求を送って予約を立てる。送出に失敗したら `error!(ghost_switch_send_failed)` で
/// 予約は立てない（送ってから立てるので、下ろすべき予約は残らない）。
pub(crate) fn request_ghost_switch(world: &mut World, req: SwitchRequest) -> SwitchVerdict {
    if world.get_non_send::<SwitchInFlight>().is_some() {
        tracing::warn!(
            event = "ghost_switch_busy",
            spec = ?req.ghost,
            "切替の途中に新しい切替要求が届いた——重ねずに無視する"
        );
        return SwitchVerdict::Busy;
    }
    let Some(ctx) = world.get_resource::<BootContext>() else {
        tracing::warn!(
            event = "ghost_switch_no_context",
            reason = "boot_context",
            "起動の文脈が無いので切替要求を無視する"
        );
        return SwitchVerdict::NoContext;
    };
    let entries = areka_ghost::catalog::list_ghosts(&ctx.root);
    let Some(mut target) = resolve_switch_target(&entries, &req.ghost) else {
        tracing::warn!(
            event = "ghost_switch_unknown",
            spec = ?req.ghost,
            "切替先が目録のどのゴーストにも一致しない——切替を無視する（降ろさず知らせも送らない）"
        );
        return SwitchVerdict::NotFound;
    };
    let current_folder = ctx.current.ghost.folder.clone();

    let session = world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref());
    let Some((kanade, prev)) = session.and_then(|s| {
        let names = s.names();
        let prev = PrevGhost {
            dir: s.ghost_dir().to_path_buf(),
            name: names.and_then(|n| n.name.clone()).or(current_folder),
            sakura_name: names.and_then(|n| n.sakura_name.clone()),
        };
        s.kanade().map(|k| (k.clone(), prev))
    }) else {
        tracing::warn!(
            event = "ghost_switch_no_context",
            reason = "ghost_slot",
            "切替の要求を送る先のゴーストが居ないので切替要求を無視する"
        );
        return SwitchVerdict::NoContext;
    };

    target.sakura_name = areka_ghost::catalog::sakura_name(&target.dir);
    let change = ChangeRequest {
        target: ChangeTarget {
            sakura_name: target.sakura_name.clone().unwrap_or_default(),
            name: target.name.clone(),
            dir: std::path::absolute(&target.dir)
                .unwrap_or_else(|_| target.dir.clone())
                .display()
                .to_string(),
        },
        origin: req.origin,
        raise_event: req.raise_event,
    };
    if kanade.send(KanadeMsg::ChangeGhost(change)).is_err() {
        tracing::error!(
            event = "ghost_switch_send_failed",
            to = %target.name,
            "kanade へ切替の要求を送れなかった（kanade は止まっている）——切替を行わない"
        );
        return SwitchVerdict::NoContext;
    }
    tracing::info!(
        event = "ghost_switch_requested",
        from = ?prev.name,
        to = %target.name,
        raise_event = req.raise_event,
        origin = req.origin.as_ref_str(),
        "切替の要求を kanade へ送った"
    );
    world.insert_non_send(SwitchInFlight {
        target,
        prev,
        stage: SwitchStage::SendOff,
    });
    SwitchVerdict::Accepted
}

#[cfg(test)]
#[path = "ghost_switch_tests.rs"]
mod tests;
