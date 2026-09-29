//! 置換語 `%lastghostname`・`%lastobjectname` の値（design「areka / install / names」・設計で決めたこと 10）。
//!
//! 値はプロセスで 1 つ（[`LastInstallNames`]・入れるまでは World に無い）。書庫を 1 本入れ終えたら
//! [`update`] が値を更新して今のゴーストの記憶の書き手へ載せ、ゴーストを起こすたびに [`reseed`] が
//! 起こしたゴーストの記憶の書き手へ載せ直す（記憶の置き場は起こすたびに新しくなる）。台詞の開始で
//! 凍結する表は語彙表を見ないので、載せた値はそのまま置き換えに使われる。値の無い語は載せない
//! （まだ入れていなければ今日どおり置き換えない＝要件 6.9）。

use areka_ghost::GhostRuntime;
use areka_ghost::sylphya_wiring::ghost_asker_id;
use areka_sylphya::{AskerId, SylphyaPublisher};
use bevy_ecs::resource::Resource;
use bevy_ecs::world::World;

use crate::ghost_session::GhostSlot;

/// 置換語 2 つの値（プロセスで 1 つ・入れるまでは World に無い）。
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct LastInstallNames {
    /// `%lastghostname`（関わったゴーストの名前）。
    pub ghost: Option<String>,
    /// `%lastobjectname`（最後に入れた書庫の `install.txt` の `name`）。
    pub object: Option<String>,
}

/// 書庫を 1 本入れ終えた（要件 6.8・12.11）: `%lastobjectname` を `object` に、`%lastghostname` を
/// `ghost`（`ghost` なら入れたゴースト・`shell`／`supplement` なら宛先のゴーストの名前）にして、今の
/// ゴーストの記憶の書き手へ載せる。`ghost` が None（バルーンだけ）なら `%lastghostname` は前の値のまま。
/// 反映は待たない（呼び手が書き手の柵を掛ける）。
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_25lastghostname:1
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_25lastobjectname:1
pub(crate) fn update(world: &mut World, object: String, ghost: Option<String>) {
    let mut names = world
        .get_resource::<LastInstallNames>()
        .cloned()
        .unwrap_or_default();
    names.object = Some(object);
    if ghost.is_some() {
        names.ghost = ghost;
    }
    let published = current_runtime(world).map(|runtime| publish_to(runtime, &names));
    tracing::info!(
        event = "install_names_updated",
        ghost = names.ghost.as_deref(),
        object = names.object.as_deref(),
        published = published.is_some(),
        "[install] 置換語 %lastghostname・%lastobjectname の値を替えました（published=false は今のゴーストが居ない）"
    );
    world.insert_resource(names);
}

/// ゴーストを起こした（`ghost_session::boot_wired` から）: 値が在れば、起こしたゴーストの記憶の
/// 書き手へ載せ直す。まだ入れていなければ何もしない。
pub(crate) fn reseed(world: &World, runtime: &GhostRuntime) {
    let Some(names) = world.get_resource::<LastInstallNames>() else {
        return;
    };
    let asker = publish_to(runtime, names);
    tracing::debug!(
        event = "install_names_reseeded",
        ghost = names.ghost.as_deref(),
        object = names.object.as_deref(),
        asker = asker.as_str(),
        "[install] 起こしたゴーストへ置換語の値を載せ直しました"
    );
}

/// 置き場のゴーストの実行系（今のゴーストの記憶の書き手を持つ）。居なければ None。
pub(super) fn current_runtime(world: &World) -> Option<&GhostRuntime> {
    world.get_non_send::<GhostSlot>()?.0.as_ref()?.runtime()
}

/// `runtime` のゴーストの問い合わせ元へ載せ、その問い合わせ元を返す。
fn publish_to(runtime: &GhostRuntime, names: &LastInstallNames) -> AskerId {
    let asker = ghost_asker_id(&runtime.mount().shiori.dir);
    publish(runtime.sylphya_publisher(), asker.clone(), names);
    asker
}

/// 値の在る語だけを載せる（どちらも無ければ投函しない）。
fn publish(publisher: &SylphyaPublisher, asker: AskerId, names: &LastInstallNames) {
    let flat: Vec<(String, String)> = [
        ("lastghostname", &names.ghost),
        ("lastobjectname", &names.object),
    ]
    .into_iter()
    .filter_map(|(key, value)| value.clone().map(|value| (key.to_owned(), value)))
    .collect();
    if !flat.is_empty() {
        publisher.publish_static(asker, flat, Vec::new());
    }
}

#[cfg(test)]
#[path = "names_tests.rs"]
mod tests;
