//! single-pass fold: parser の登場順定義ストリームを `EmoWorld` へ畳み込む。
//!
//! plain `surfaceN,M`／`N-M` は全 id を新設・`surface.append` は既存 id のみへ追記
//! （存在条件付き・ukadoc 意味論）。ターゲット記述子の単一・列挙・範囲を展開し、除外指定
//! （`!N`／`!a-b`）を展開時に減算適用する。複数定義が同一 surface に効く場合は登場順を保った
//! 順序で決定的に適用し、append ブロックが持つ element・collision・animation を対象 surface へ
//! 反映しつつ alias を収集する。参照 id が存在しない場合はパニックせず `warn` 以上で観測可能に扱う。

use areka_parsers::shell::{
    AppendTarget, DefRef, Element, Shell, Surface, SurfaceAlias, SurfaceAppend,
};
use bevy_ecs::world::World;

use crate::method::ComposeMethod;
use crate::normalized::{NormalizedElement, SurfaceMaster, Transform};
use crate::world::{AliasMap, SurfaceId, SurfaceIndex};

/// `Shell.definitions` を登場順に single-pass で走査し `World` へ畳み込む（要件 1.7）。
///
/// plain `surface` ヘッダ（[`DefRef::Surface`]）は全 id 新設、`surface.append`
/// （[`DefRef::Append`]）は展開後その時点でツリーに存在する id のみへ追記する（存在条件付き・
/// 要件 2.2）。登場順で走査するため、append は「その時点までに積まれた状態」に対して効き、後続で
/// 定義される surface へは遡及しない（要件 2.3・前方参照なし）。alias（[`DefRef::Alias`]）は
/// `AliasMap` へ収集し、同一キーは登場順で後勝ちとする（要件 3.1/3.2）。
///
/// 欠落・不整合はパニックせず `warn` で観測可能化する（要件 1.4）。
pub(crate) fn fold_shell(world: &mut World, shell: &Shell) {
    for def in &shell.definitions {
        match *def {
            DefRef::Surface(index) => match shell.surfaces.get(index) {
                Some(surface) => fold_plain_surface(world, surface),
                None => {
                    // 転記層と定義ストリームの不整合（本来生じない）。パニックせず観測可能化する。
                    tracing::warn!(
                        target: "areka_emo_compose",
                        index,
                        "DefRef::Surface が surfaces 範囲外を指す: スキップ"
                    );
                }
            },
            DefRef::Append(index) => match shell.appends.get(index) {
                Some(append) => fold_append(world, append),
                None => {
                    // 転記層と定義ストリームの不整合（本来生じない）。パニックせず観測可能化する。
                    tracing::warn!(
                        target: "areka_emo_compose",
                        index,
                        "DefRef::Append が appends 範囲外を指す: スキップ"
                    );
                }
            },
            DefRef::Alias(index) => match shell.aliases.get(index) {
                Some(alias) => fold_alias(world, alias),
                None => {
                    // 転記層と定義ストリームの不整合（本来生じない）。パニックせず観測可能化する。
                    tracing::warn!(
                        target: "areka_emo_compose",
                        index,
                        "DefRef::Alias が aliases 範囲外を指す: スキップ"
                    );
                }
            },
            // `DefRef` は `#[non_exhaustive]`。未知の定義種別はパニックせず観測可能化する（要件 1.4）。
            other => {
                tracing::warn!(
                    target: "areka_emo_compose",
                    def = ?other,
                    "未知の DefRef 種別: スキップ"
                );
            }
        }
    }
}

/// plain `surface` 定義 1 件を展開し、各 id を新規 surface として常駐させる（要件 1.1/2.1）。
///
/// ターゲット記述子（単一・列挙・範囲）を記述順に展開し、共有ボディ（element/collision/animation）
/// から正規化 [`SurfaceMaster`] を id ごとに生成して登録する。既存 id は全置換（後勝ち・`warn`）。
fn fold_plain_surface(world: &mut World, surface: &Surface) {
    for id in expand_targets(&surface.targets) {
        let master = normalize_surface(id, surface);
        upsert_surface(world, id, master);
    }
}

/// `surface.append` 定義 1 件を展開し、**その時点で既存の id のみ**へ追記する（要件 2.2/2.4）。
///
/// ターゲット記述子を plain と同一規則で展開し（単一・列挙・両端含む範囲・[`expand_targets`]）、
/// 各 id が [`SurfaceIndex`] に存在する場合のみ対象 [`SurfaceMaster`] を in-place マージする
/// （despawn/respawn しない）。非存在 id は新設せず `warn` でスキップする（要件 1.4/2.2）。
/// 登場順の走査（[`fold_shell`]）ゆえ、後続で定義される surface へは遡及しない（要件 2.3）。
fn fold_append(world: &mut World, append: &SurfaceAppend) {
    // 追記 element は plain と同一の正規化（x,y→Transform・method=Overlay）で用意する。
    let append_elements: Vec<NormalizedElement> =
        append.elements.iter().map(normalize_element).collect();

    for id in expand_targets(&append.targets) {
        // 存在条件: その時点でツリーに存在する id のみ対象（非存在は新設しない・要件 2.2）。
        let Some(entity) = world.resource::<SurfaceIndex>().0.get(&id).copied() else {
            tracing::warn!(
                target: "areka_emo_compose",
                id,
                "surface.append 対象 id が未存在: 新設せずスキップ（存在条件付き）"
            );
            continue;
        };
        let Some(mut master) = world.get_mut::<SurfaceMaster>(entity) else {
            // SurfaceIndex に載るが component 欠落（本来生じない不整合）。観測可能化してスキップ。
            tracing::warn!(
                target: "areka_emo_compose",
                id,
                "SurfaceIndex は指すが SurfaceMaster component が欠落: スキップ"
            );
            continue;
        };

        // element: 末尾連結してから layer 昇順で安定ソート（不変条件を保つ・要件 2.4）。
        master.elements.extend(append_elements.iter().cloned());
        master.elements.sort_by_key(|e| e.layer);

        // collision: 末尾連結（転記のまま・要件 2.4）。
        master.collisions.extend(append.collisions.iter().cloned());

        // animation: 同一 id は後勝ち置換（+warn）・新 id は追加（要件 2.4）。
        merge_animations(id, &mut master.animations, &append.animations);
    }
}

/// `kero.surface.alias` の 1 エントリを `AliasMap` へ収集する（要件 3.1/3.2）。
///
/// alias キー → 順序付き数値 id リストを `BTreeMap` へ挿入する。`BTreeMap::insert` は同一キーで
/// 上書きするため、登場順 single-pass 走査（[`fold_shell`]）と併せて重複キーは後勝ちで決定的に
/// なる（要件 3.2）。id リストはソート・重複除去せず記述順のまま複製する（alias の順序付き
/// ターゲット列＝要件 3.1・BindSet の正規化とは別物）。
fn fold_alias(world: &mut World, alias: &SurfaceAlias) {
    world
        .resource_mut::<AliasMap>()
        .0
        .insert(alias.key.as_str().to_string(), alias.ids.clone());
}

/// append の animation を対象 surface の animation 群へマージする（要件 2.4）。
///
/// 同一 animation id が既存にあれば後勝ち置換（既存を append 版で差し替え・単一に保つ）、
/// 新 id なら末尾へ追加する。置換は ukadoc 明文規則が無い de-facto 挙動ゆえ `warn` で記録する。
fn merge_animations(
    surface_id: u32,
    existing: &mut Vec<areka_parsers::shell::Animation>,
    additions: &[areka_parsers::shell::Animation],
) {
    for add in additions {
        if let Some(slot) = existing.iter_mut().find(|a| a.id == add.id) {
            tracing::warn!(
                target: "areka_emo_compose",
                surface_id,
                animation_id = add.id,
                "append が既存 animation id を再定義: 後勝ちで置換する（de-facto）"
            );
            *slot = add.clone();
        } else {
            existing.push(add.clone());
        }
    }
}

/// ターゲット記述子を記述順に展開し、除外指定（`!N`／`!a-b`）を減算した id 列を返す（要件 2.5）。
///
/// 二相構成: (1) 包含記述子（[`AppendTarget::Single`]／[`AppendTarget::Range`]・両端含む）を
/// **記述順**に列挙し、(2) 除外記述子（[`AppendTarget::Exclude`]／[`AppendTarget::ExcludeRange`]・
/// 両端含む）を集合として集め、包含列から除外集合に属する id を取り除く。除外は記述子の登場位置に
/// 依らず展開結果全体へ効く（ukadoc: `!` は集合から除去）。生存 id の記述順・重複は保つ（design
/// 「展開結果の適用順は記述順」）。
///
/// emo2 は除外を使用しないため実処理は型シームだが、記述子の口は保持し減算まで実装する（要件 2.5/12.3）。
fn expand_targets(targets: &[AppendTarget]) -> Vec<u32> {
    use std::collections::BTreeSet;

    // (1) 包含 id を記述順に列挙（重複はそのまま保つ）。
    let mut included: Vec<u32> = Vec::new();
    // (2) 除外 id を集合へ集める（記述位置に依らず全体へ効かせるため先に全走査）。
    let mut excluded: BTreeSet<u32> = BTreeSet::new();

    for target in targets {
        match *target {
            AppendTarget::Single(id) => included.push(id),
            AppendTarget::Range { start, end } => {
                // 記述子の向きに関わらず両端含みで昇順展開する（`a-b` は a..=b）。
                let (lo, hi) = if start <= end {
                    (start, end)
                } else {
                    (end, start)
                };
                included.extend(lo..=hi);
            }
            AppendTarget::Exclude(id) => {
                excluded.insert(id);
            }
            AppendTarget::ExcludeRange { start, end } => {
                // 除外範囲も両端含み（記述子の向き不問）。
                let (lo, hi) = if start <= end {
                    (start, end)
                } else {
                    (end, start)
                };
                excluded.extend(lo..=hi);
            }
            // `AppendTarget` は `#[non_exhaustive]`。未知の記述子はパニックせず観測可能化する（要件 1.4）。
            ref other => {
                tracing::warn!(
                    target: "areka_emo_compose",
                    target_desc = ?other,
                    "未知の AppendTarget 記述子: 包含集合から除外"
                );
            }
        }
    }

    // 減算: 除外集合に属さない id のみを記述順・重複保持で残す。
    included.retain(|id| !excluded.contains(id));
    included
}

/// 共有ボディから id 固有の正規化 [`SurfaceMaster`] を生成する（要件 4.2/4.4）。
///
/// element は layer 昇順（同 layer は登場順）に安定ソートし、x,y を [`Transform::translate`] へ、
/// method は M1 契約により常に [`ComposeMethod::Overlay`] とする。collision/animation は転記のまま
/// 複製する。
fn normalize_surface(id: u32, surface: &Surface) -> SurfaceMaster {
    let mut elements: Vec<NormalizedElement> =
        surface.elements.iter().map(normalize_element).collect();
    // layer 昇順・同 layer は登場順（安定ソート）。
    elements.sort_by_key(|e| e.layer);

    SurfaceMaster {
        id,
        elements,
        collisions: surface.collisions.clone(),
        animations: surface.animations.clone(),
    }
}

/// 転記 element を正規化 element へ写す（x,y→[`Transform`]・method は M1 固定 [`Overlay`]）。
///
/// [`Overlay`]: ComposeMethod::Overlay
fn normalize_element(element: &Element) -> NormalizedElement {
    NormalizedElement {
        layer: element.layer,
        path: element.path.clone(),
        transform: Transform::translate(element.x, element.y),
        method: ComposeMethod::Overlay,
    }
}

/// id→entity を登録する。既存 id は全置換（後勝ち）＋ `warn`（要件 2.1・ukadoc 明文規則なし＝de-facto）。
fn upsert_surface(world: &mut World, id: u32, master: SurfaceMaster) {
    let existing = world.resource::<SurfaceIndex>().0.get(&id).copied();
    if let Some(old_entity) = existing {
        tracing::warn!(
            target: "areka_emo_compose",
            id,
            "surface id 重複: 既存定義を全置換する（後勝ち）"
        );
        // 全置換のため旧 entity を除去してから新設する（画素バッファは持たない・要件 10.6）。
        world.despawn(old_entity);
    }
    let entity = world.spawn((SurfaceId(id), master)).id();
    world.resource_mut::<SurfaceIndex>().0.insert(id, entity);
}

#[cfg(test)]
#[path = "fold_tests.rs"]
mod tests;
