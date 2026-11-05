//! single-pass fold: parser の登場順定義ストリームを `EmoWorld` へ畳み込む。
//!
//! plain `surfaceN,M`／`N-M` は無い id を新設し、既にある id へは行を足す（[`add_lines`]・
//! 追記と同じ足し方）・`surface.append` は既にある id のみへ追記（存在条件付き・ukadoc 意味論）。「既にある」には、波括弧が 1 行も無くてもファイル名だけで
//! 置かれた絵を持つ番号（`SurfaceImages`）を含む（要件 3.7）。ターゲット記述子の単一・列挙・
//! 範囲を展開し、除外指定
//! （`!N`／`!a-b`）を展開時に減算適用する。複数定義が同一 surface に効く場合は登場順を保った
//! 順序で決定的に適用し、append ブロックが持つ element・collision・animation を対象 surface へ
//! 反映しつつ alias を収集する。参照 id が存在しない場合はパニックせず `warn` 以上で観測可能に扱う。

use areka_parsers::shell::{
    AppendTarget, DefRef, Element, Shell, Surface, SurfaceAlias, SurfaceAppend,
};
use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;

use crate::base_image::SurfaceImages;
use crate::method::ComposeMethod;
use crate::nesting::element_kind;
use crate::normalized::{NormalizedElement, SurfaceMaster, Transform};
use crate::world::{AliasMap, Records, SurfaceId, SurfaceIndex};

/// `Shell.definitions` を登場順に single-pass で走査し `World` へ畳み込む（要件 1.7）。
///
/// plain `surface` ヘッダ（[`DefRef::Surface`]）は無い id を新設し、既にある id へは行を足す
/// （[`add_lines`]・[`fold_plain_surface`]）、`surface.append`
/// （[`DefRef::Append`]）は展開後その時点で既にある id のみへ追記する（存在条件付き・要件 2.2。
/// 「既にある」は画像だけで存在する面を含む＝[`fold_append`]）。登場順で走査するため、append は「その時点までに積まれた状態」に対して効き、後続で
/// 定義される surface へは遡及しない（要件 2.3・前方参照なし）。alias（[`DefRef::Alias`]）は
/// `AliasMap` へ収集し、同一キーは登場順で後勝ちとする（要件 3.1/3.2）。
///
/// 欠落・不整合はパニックせず `warn` で観測可能化する（要件 1.4）。
///
/// 記録の行を出すかどうかは `records` で切り替える
/// （枝の選び方と面の表への書き込みは変えない・spec: emo-bake-log-overlap 要件 2.3）。
pub(crate) fn fold_shell_with(world: &mut World, shell: &Shell, records: Records) {
    for def in &shell.definitions {
        match *def {
            DefRef::Surface(index) => match shell.surfaces.get(index) {
                Some(surface) => fold_plain_surface(world, surface, records),
                None if records.emits() => {
                    // 転記層と定義ストリームの不整合（本来生じない）。パニックせず観測可能化する。
                    tracing::warn!(
                        target: "areka_emo_compose",
                        index,
                        "DefRef::Surface が surfaces 範囲外を指す: スキップ"
                    );
                }
                None => {}
            },
            DefRef::Append(index) => match shell.appends.get(index) {
                Some(append) => fold_append(world, append, records),
                None if records.emits() => {
                    // 転記層と定義ストリームの不整合（本来生じない）。パニックせず観測可能化する。
                    tracing::warn!(
                        target: "areka_emo_compose",
                        index,
                        "DefRef::Append が appends 範囲外を指す: スキップ"
                    );
                }
                None => {}
            },
            DefRef::Alias(index) => match shell.aliases.get(index) {
                Some(alias) => fold_alias(world, alias),
                None if records.emits() => {
                    // 転記層と定義ストリームの不整合（本来生じない）。パニックせず観測可能化する。
                    tracing::warn!(
                        target: "areka_emo_compose",
                        index,
                        "DefRef::Alias が aliases 範囲外を指す: スキップ"
                    );
                }
                None => {}
            },
            // `DefRef` は `#[non_exhaustive]`。未知の定義種別はパニックせず観測可能化する（要件 1.4）。
            other if records.emits() => {
                tracing::warn!(
                    target: "areka_emo_compose",
                    def = ?other,
                    "未知の DefRef 種別: スキップ"
                );
            }
            _ => {}
        }
    }
}

/// plain `surface` 定義 1 件を展開し、各 id を surface として常駐させる（要件 1.1/2.1）。
///
/// ターゲット記述子（単一・列挙・範囲）を記述順に展開し、共有ボディ（element/collision/animation）
/// から正規化 [`SurfaceMaster`] を id ごとに生成する。無い id は新設し、既にある id（前の波括弧・
/// 追記で定義された番号）は前の定義へ後のブレスの行を足す（[`add_lines`]・
/// spec: areka-P0-emily-ghost-verification 開発者の決め 2026-10-17「足し合わせる」）。
fn fold_plain_surface(world: &mut World, surface: &Surface, records: Records) {
    for id in expand_targets_with(&surface.targets, records) {
        let later = normalize_surface(id, surface);
        let existing = world.resource::<SurfaceIndex>().0.get(&id).copied();
        let merged = existing.and_then(|entity| world.get_mut::<SurfaceMaster>(entity));
        match merged {
            Some(mut master) => {
                // 失敗ではなく定めた振る舞い（えみりは 1 本のシェルで百数十回当たる）なので debug。
                if records.emits() {
                    tracing::debug!(
                        target: "areka_emo_compose",
                        id,
                        "surface id 重複: 前の定義へ後のブレスの行を足す"
                    );
                }
                add_lines(&mut master, later, Source::Brace, records);
            }
            None => {
                upsert_surface(world, id, later, records);
            }
        }
    }
}

/// 足す行がどこから来たか（collision の足し方だけが違う）。
#[derive(Clone, Copy, PartialEq)]
enum Source {
    /// 同じ番号の 2 つ目の `surface*`ブレス。
    Brace,
    /// `surface.append*`ブレス。
    Append,
}

/// 追記と 2 つ目の波括弧の 1 つの足し方（SSP 2.9.29 の測り・spec: audit-emo-compose 設計の議題 7 案 A）。
///
/// - element: 同じ element番号でも前の行を残して両方を足し、layer 昇順へ安定ソート（同じ番号の
///   中は前の行が先）。同じ番号が重なったときだけ `warn!`（SSP も「二重定義」の Warning を出す）。
/// - animation: すでにある animation番号の後の行は interval も pattern も捨てる（先勝ち・`warn!`）。
///   無い番号は末尾へ足す。
/// - collision: SSP で測れなかった（画素に出ず MCP で読めない）ので今のまま。波括弧は同じ
///   collision番号の前の行を除いて末尾へ（後に書かれた行＝画家のアルゴリズムで手前）、追記は
///   重ねて末尾へ足す。
fn add_lines(master: &mut SurfaceMaster, later: SurfaceMaster, source: Source, records: Records) {
    let surface_id = master.id;
    for add in &later.elements {
        if records.emits() && master.elements.iter().any(|e| e.layer == add.layer) {
            tracing::warn!(
                target: "areka_emo_compose",
                surface_id,
                element = add.layer,
                "element番号の二重定義: 前の行も残して両方を足す（SSP と同じ）"
            );
        }
    }
    master.elements.extend(later.elements);
    master.elements.sort_by_key(|e| e.layer);

    if source == Source::Brace {
        master
            .collisions
            .retain(|c| !later.collisions.iter().any(|l| l.index == c.index));
    }
    master.collisions.extend(later.collisions);

    for add in later.animations {
        if master.animations.iter().any(|a| a.id == add.id) {
            if records.emits() {
                tracing::warn!(
                    target: "areka_emo_compose",
                    surface_id,
                    animation_id = add.id,
                    "animation番号の二重定義: 後の行を捨てる（先勝ち・SSP と同じ）"
                );
            }
        } else {
            master.animations.push(add);
        }
    }
}

/// `surface.append` 定義 1 件を展開し、**その時点で既にある id のみ**へ追記する（要件 2.2/2.4/3.7）。
///
/// ターゲット記述子を plain と同一規則で展開し（単一・列挙・両端含む範囲・[`expand_targets`]）、
/// 各 id が [`SurfaceIndex`] に存在する場合は対象 [`SurfaceMaster`] へ波括弧と同じ [`add_lines`] で
/// in-place に足す（despawn/respawn しない）。波括弧が 1 行も無くても [`SurfaceImages`] に画像が在る番号は
/// 「既にある面」に数え、空の [`SurfaceMaster`] をその場で作ってから同じ追記を行う（要件 3.7・C6）。
/// どちらにも無い id は新設せず `warn` でスキップする（要件 1.4/2.2/3.8）。
/// 登場順の走査（[`fold_shell_with`]）ゆえ、後続で定義される surface へは遡及しない（要件 2.3）。
fn fold_append(world: &mut World, append: &SurfaceAppend, records: Records) {
    // 追記 element は plain と同一の正規化（x,y→Transform・method=Overlay）で用意する。
    let append_elements: Vec<NormalizedElement> =
        append.elements.iter().map(normalize_element).collect();

    for id in expand_targets_with(&append.targets, records) {
        // 存在条件: その時点でツリーに在る id と、ファイル名だけで置かれた絵を持つ id が対象
        // （どちらにも無ければ新設しない・要件 2.2/3.7/3.8）。
        let entity = match world.resource::<SurfaceIndex>().0.get(&id).copied() {
            Some(entity) => entity,
            None if has_base_image(world, id) => {
                // 画像だけで存在する面。空の master を置いてから、既存の面と同じ追記を行う。
                // 画像そのものは畳み込みの後の `apply_base_images` が層 0 へ敷く（追記が層 0 を
                // 足していればそちらが優先され、画像は `shadowed` に数えられる）。
                upsert_surface(
                    world,
                    id,
                    SurfaceMaster {
                        id,
                        elements: Vec::new(),
                        collisions: Vec::new(),
                        animations: Vec::new(),
                    },
                    records,
                )
            }
            None => {
                if records.emits() {
                    tracing::warn!(
                        target: "areka_emo_compose",
                        id,
                        "surface.append 対象 id が未存在: 新設せずスキップ（存在条件付き）"
                    );
                }
                continue;
            }
        };
        let Some(mut master) = world.get_mut::<SurfaceMaster>(entity) else {
            // SurfaceIndex に載るが component 欠落（本来生じない不整合）。観測可能化してスキップ。
            if records.emits() {
                tracing::warn!(
                    target: "areka_emo_compose",
                    id,
                    "SurfaceIndex は指すが SurfaceMaster component が欠落: スキップ"
                );
            }
            continue;
        };

        let later = SurfaceMaster {
            id,
            elements: append_elements.clone(),
            collisions: append.collisions.clone(),
            animations: append.animations.clone(),
        };
        add_lines(&mut master, later, Source::Append, records);
    }
}

/// 番号 `id` が、ファイル名だけで置かれた絵を持つか（要件 3.7）。
///
/// [`SurfaceImages`] は面の表を組む入口（[`crate::EmoWorld::build_with_images`]）が畳み込みの前に
/// 置く。[`fold_shell_with`] を直に呼ぶテストは置かないことがあるので、欠けていれば「画像 0 件」と
/// 読む（畳み込みだけを通した素の結果＝本仕様の適用前と同じ振る舞い）。
fn has_base_image(world: &World, id: u32) -> bool {
    world
        .get_resource::<SurfaceImages>()
        .is_some_and(|images| images.0.contains_key(&id))
}

/// `kero.surface.alias` の 1 エントリを `AliasMap` へ収集する（要件 3.1/3.2）。
///
/// alias キー → 順序付き数値 id リストを `BTreeMap` へ挿入する。`BTreeMap::insert` は同一キーで
/// 上書きするため、登場順 single-pass 走査（[`fold_shell_with`]）と併せて重複キーは後勝ちで決定的に
/// なる（要件 3.2）。id リストはソート・重複除去せず記述順のまま複製する（alias の順序付き
/// ターゲット列＝要件 3.1・BindSet の正規化とは別物）。
fn fold_alias(world: &mut World, alias: &SurfaceAlias) {
    world
        .resource_mut::<AliasMap>()
        .0
        .insert(alias.key.as_str().to_string(), alias.ids.clone());
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
///
/// 範囲の幅に上限はかけない: 読む側（parsers）がシェル全体の範囲の幅の合計に上限をかけ、越えた記述子を捨てて記録するので、ここへ届く展開は有界。
pub(crate) fn expand_targets(targets: &[AppendTarget]) -> Vec<u32> {
    expand_targets_with(targets, Records::Emit)
}

/// [`expand_targets`] と同じ展開で、知らない記述子の記録を出すかどうかを `records` で切り替える。
fn expand_targets_with(targets: &[AppendTarget], records: Records) -> Vec<u32> {
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
                if records.emits() {
                    tracing::warn!(
                        target: "areka_emo_compose",
                        target_desc = ?other,
                        "未知の AppendTarget 記述子: 包含集合から除外"
                    );
                }
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

/// 転記 element を正規化 element へ写す（x,y→[`Transform`]・置くものは [`element_kind`] の読み分け
/// ＝`surface*`・`surface.append*` の両方がここを通る・要件 1.1/1.2）。
///
/// 読み手が届ける element定義は描画メソッドが `overlay` と `base` の行だけで、どちらも
/// [`Overlay`] で置き、X,Y は必ず位置として使う（`base` の「XY 無視」は pattern定義の話で、
/// element定義には当てない・areka-P0-element-base-method 要件 1.1・4.4）。
///
/// [`Overlay`]: ComposeMethod::Overlay
fn normalize_element(element: &Element) -> NormalizedElement {
    NormalizedElement {
        layer: element.layer,
        path: element.path.clone(),
        transform: Transform::translate(element.x, element.y),
        method: ComposeMethod::Overlay,
        kind: element_kind(&element.path),
    }
}

/// id を新設して id→entity を登録し、その entity を返す。既にある id の定義へは波括弧も追記も
/// [`add_lines`] で足すので、ここへ来るのは無い id だけ（索引は在るが
/// component が欠けた不整合のときだけ、古い entity を除いて作り直す）。画像だけで在る面の新設
/// （[`crate::base_image::apply_base_images`]）も同じ口を通る。
pub(crate) fn upsert_surface(
    world: &mut World,
    id: u32,
    master: SurfaceMaster,
    records: Records,
) -> Entity {
    if let Some(stale) = world.resource::<SurfaceIndex>().0.get(&id).copied() {
        // 本来生じない不整合（索引は指すが SurfaceMaster が無い）。黙らずに残して作り直す。
        if records.emits() {
            tracing::warn!(
                target: "areka_emo_compose",
                id,
                "SurfaceIndex は指すが SurfaceMaster component が欠落: 作り直す"
            );
        }
        world.despawn(stale);
    }
    let entity = world.spawn((SurfaceId(id), master)).id();
    world.resource_mut::<SurfaceIndex>().0.insert(id, entity);
    entity
}

#[cfg(test)]
#[path = "fold_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "fold_merge_tests.rs"]
mod merge_tests;
