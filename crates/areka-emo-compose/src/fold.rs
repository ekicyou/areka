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
    Animation, AppendTarget, Collision, DefRef, Element, Shell, Surface, SurfaceAlias,
    SurfaceAppend,
};
use std::collections::{HashMap, HashSet};

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
    let mut added = HashMap::new();
    let mut copied = 0;
    for def in &shell.definitions {
        match *def {
            DefRef::Surface(index) => match shell.surfaces.get(index) {
                Some(surface) => {
                    fold_plain_surface(world, surface, records, &mut added, &mut copied)
                }
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
                Some(append) => fold_append(world, append, records, &mut added, &mut copied),
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
    // 行を足した面の element を layer 昇順へ 1 度だけ安定ソートする（同じ番号の中は前の行が先・
    // 足すたびに並べ直さない・指摘 27）。波括弧が置き換えた collision の前の行もここで 1 度だけ除く（指摘 131）。
    for (id, numbers) in added {
        let entity = world.resource::<SurfaceIndex>().0.get(&id).copied();
        if let Some(mut master) = entity.and_then(|e| world.get_mut::<SurfaceMaster>(e)) {
            master.elements.sort_by_key(|e| e.layer);
            if !numbers.cuts.is_empty() {
                let mut at = 0;
                master.collisions.retain(|c| {
                    at += 1;
                    numbers.cuts.get(&c.index).is_none_or(|&cut| at > cut)
                });
            }
        }
    }
}

/// 畳み込み 1 回で、見出しの番号ごとに本体の行を写す数の合計の上限（指摘 78・面と箱で 1 つずつ）。
/// 読む側の上限は番号の数だけを抑えるので、本体の行数との積をここで抑える。仮の値で、測り直しの候補。
pub(crate) const MAX_COPIED_LINES: usize = 1 << 20;

/// 記述子 1 つの本体（`lines` 行）を `ids` の番号ごとに写してよいか（指摘 78）。写す数を `copied` に足し、
/// 合計が [`MAX_COPIED_LINES`] を越えるなら足さずに `error!`（番号の範囲・行の数・上限）を出して false
/// （呼び手はその記述子を捨てる）。
pub(crate) fn may_copy(copied: &mut usize, ids: &[u32], lines: usize, records: Records) -> bool {
    let n = ids.len().saturating_mul(lines);
    if n <= MAX_COPIED_LINES - *copied {
        *copied += n;
        return true;
    }
    if records.emits() {
        tracing::error!(
            target: "areka_emo_compose",
            first = ids.first().copied().unwrap_or_default(),
            last = ids.last().copied().unwrap_or_default(),
            ids = ids.len(),
            lines,
            limit = MAX_COPIED_LINES,
            "範囲の見出しで写す行の数が畳み込みの上限を越えた: この記述子を捨てる"
        );
    }
    false
}

/// 記述子の本体の行の数（element・collision と、animation ごとに interval の 1 行＋pattern の行）。番号ごとに
/// 写すのは pattern の列も全部なので、pattern の行も数える。
fn body_lines(elements: &[Element], collisions: &[Collision], animations: &[Animation]) -> usize {
    animations
        .iter()
        .fold(elements.len().saturating_add(collisions.len()), |n, a| {
            n.saturating_add(1).saturating_add(a.patterns.len())
        })
}

/// 行を足した面ごとの、すでにある番号の集合（[`add_lines`] が重なりと先勝ちを表で引く・指摘 27）と、
/// 波括弧が置き換えた collision番号 → その波括弧の行の始まりの位置（この位置より前の同じ番号の行は
/// 畳み込みの最後に 1 度だけ除く・指摘 131）。
#[derive(Default)]
struct Numbers {
    elements: HashSet<u32>,
    collisions: HashSet<u32>,
    animations: HashSet<u32>,
    cuts: HashMap<u32, usize>,
}

impl Numbers {
    fn of(master: &SurfaceMaster) -> Numbers {
        Numbers {
            elements: master.elements.iter().map(|e| e.layer).collect(),
            collisions: master.collisions.iter().map(|c| c.index).collect(),
            animations: master.animations.iter().map(|a| a.id).collect(),
            cuts: HashMap::new(),
        }
    }
}

/// plain `surface` 定義 1 件を展開し、各 id を surface として常駐させる（要件 1.1/2.1）。
///
/// ターゲット記述子（単一・列挙・範囲）を記述順に展開し、共有ボディ（element/collision/animation）
/// から正規化 [`SurfaceMaster`] を id ごとに生成する。無い id は新設し、既にある id（前の波括弧・
/// 追記で定義された番号）は前の定義へ後のブレスの行を足す（[`add_lines`]・
/// spec: areka-P0-emily-ghost-verification 開発者の決め 2026-10-17「足し合わせる」）。
fn fold_plain_surface(
    world: &mut World,
    surface: &Surface,
    records: Records,
    added: &mut HashMap<u32, Numbers>,
    copied: &mut usize,
) {
    let ids = expand_targets_with(&surface.targets, records);
    let lines = body_lines(&surface.elements, &surface.collisions, &surface.animations);
    if !may_copy(copied, &ids, lines, records) {
        return;
    }
    for id in ids {
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
                let numbers = added.entry(id).or_insert_with(|| Numbers::of(&master));
                add_lines(&mut master, later, Source::Brace, records, numbers);
            }
            None => {
                added.remove(&id);
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
/// - element: 同じ element番号でも前の行を残して両方を足す。layer 昇順への安定ソート（同じ番号の
///   中は前の行が先）は畳み込みの最後に 1 度（[`fold_shell_with`]）。同じ番号が重なったときだけ
///   `warn!`（SSP も「二重定義」の Warning を出す）。
/// - animation: すでにある animation番号の後の行は interval も pattern も捨てる（先勝ち・`warn!`）。
///   無い番号は末尾へ足す。
/// - collision: SSP で測れなかった（画素に出ず MCP で読めない）ので今のまま。波括弧は同じ
///   collision番号の前の行を除いて末尾へ（後に書かれた行＝画家のアルゴリズムで手前）、追記は
///   重ねて末尾へ足す。除くのは畳み込みの最後に 1 度（ここは番号ごとの切れ目を `numbers.cuts` に置くだけ）。
///
/// 重なりと先勝ちは `numbers`（その面のすでにある番号）で引き、足した番号を書き足す。`warn!` は種類ごとに
/// 足す記述子 1 つにつき 1 行（最初の番号と重なった数・指摘 136）。
fn add_lines(
    master: &mut SurfaceMaster,
    later: SurfaceMaster,
    source: Source,
    records: Records,
    numbers: &mut Numbers,
) {
    let surface_id = master.id;
    let mut twice = later
        .elements
        .iter()
        .filter(|e| numbers.elements.contains(&e.layer));
    if records.emits()
        && let Some(first) = twice.next()
    {
        tracing::warn!(
            target: "areka_emo_compose",
            surface_id,
            element = first.layer,
            count = 1 + twice.count(),
            "element番号の二重定義: 前の行も残して両方を足す（SSP と同じ）"
        );
    }
    numbers
        .elements
        .extend(later.elements.iter().map(|e| e.layer));
    master.elements.extend(later.elements);

    // 波括弧が前の行と同じ collision番号を持つときだけ、その番号の前の行を除く印を置く（除くのは最後に 1 度）。
    if source == Source::Brace {
        let at = master.collisions.len();
        for l in &later.collisions {
            if numbers.collisions.contains(&l.index) {
                numbers.cuts.insert(l.index, at);
            }
        }
    }
    numbers
        .collisions
        .extend(later.collisions.iter().map(|c| c.index));
    master.collisions.extend(later.collisions);

    let (mut first, mut dropped) = (None, 0usize);
    for add in later.animations {
        if numbers.animations.insert(add.id) {
            master.animations.push(add);
        } else {
            first.get_or_insert(add.id);
            dropped += 1;
        }
    }
    if records.emits()
        && let Some(first) = first
    {
        tracing::warn!(
            target: "areka_emo_compose",
            surface_id,
            animation_id = first,
            count = dropped,
            "animation番号の二重定義: 後の行を捨てる（先勝ち・SSP と同じ）"
        );
    }
}

/// `surface.append` 定義 1 件を展開し、**その時点で既にある id のみ**へ追記する（要件 2.2/2.4/3.7）。
///
/// ターゲット記述子を plain と同一規則で展開し（単一・列挙・両端含む範囲・[`expand_targets_with`]）、
/// 各 id が [`SurfaceIndex`] に存在する場合は対象 [`SurfaceMaster`] へ波括弧と同じ [`add_lines`] で
/// in-place に足す（despawn/respawn しない）。波括弧が 1 行も無くても [`SurfaceImages`] に画像が在る番号は
/// 「既にある面」に数え、空の [`SurfaceMaster`] をその場で作ってから同じ追記を行う（要件 3.7・C6）。
/// どちらにも無い id は新設せず `warn` でスキップする（要件 1.4/2.2/3.8）。
/// 登場順の走査（[`fold_shell_with`]）ゆえ、後続で定義される surface へは遡及しない（要件 2.3）。
fn fold_append(
    world: &mut World,
    append: &SurfaceAppend,
    records: Records,
    added: &mut HashMap<u32, Numbers>,
    copied: &mut usize,
) {
    let ids = expand_targets_with(&append.targets, records);
    let lines = body_lines(&append.elements, &append.collisions, &append.animations);
    if !may_copy(copied, &ids, lines, records) {
        return;
    }
    // 追記 element は plain と同一の正規化（x,y→Transform・method=Overlay）で用意する。
    let append_elements: Vec<NormalizedElement> =
        append.elements.iter().map(normalize_element).collect();

    for id in ids {
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
        let numbers = added.entry(id).or_insert_with(|| Numbers::of(&master));
        add_lines(&mut master, later, Source::Append, records, numbers);
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
/// 両端含む）を番号へ広げず範囲の組として集め、包含列からどれかの範囲に入る id を取り除く。除外は記述子の登場位置に
/// 依らず展開結果全体へ効く（ukadoc: `!` は集合から除去）。生存 id の記述順・重複は保つ（design
/// 「展開結果の適用順は記述順」）。
///
/// emo2 は除外を使用しないが、記述子の口は保持し減算まで実装する（要件 2.5/12.3）。
///
/// 範囲の幅に上限はかけない: 読む側（parsers）がシェル全体の範囲の幅の合計に上限をかけ、越えた記述子を捨てて記録するので、ここへ届く展開は有界。
///
/// 知らない記述子の記録を出すかどうかは `records` で切り替える（面と箱の畳み込みが面の表の組み方を渡す）。
pub(crate) fn expand_targets_with(targets: &[AppendTarget], records: Records) -> Vec<u32> {
    // (1) 包含 id を記述順に列挙（重複はそのまま保つ）。
    let mut included: Vec<u32> = Vec::new();
    // (2) 除外は両端含みの範囲 (lo, hi) のまま集める（記述位置に依らず全体へ効かせるため先に全走査）。
    // 番号へ広げない: 読む側の範囲の上限は包含だけにかかり、`!0-4294967295` は約 43 億件になる。
    let mut excluded: Vec<(u32, u32)> = Vec::new();

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
                excluded.push((id, id));
            }
            AppendTarget::ExcludeRange { start, end } => {
                // 除外範囲も両端含み（記述子の向き不問）。
                let (lo, hi) = if start <= end {
                    (start, end)
                } else {
                    (end, start)
                };
                excluded.push((lo, hi));
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

    // 除外の組を並べ、重なり・隣接を 1 つにまとめる（単独の除外を大量に並べても 2 乗にしない）。
    excluded.sort_unstable();
    let mut merged: Vec<(u32, u32)> = Vec::with_capacity(excluded.len());
    for (lo, hi) in excluded {
        match merged.last_mut() {
            Some(last) if lo <= last.1.saturating_add(1) => last.1 = last.1.max(hi),
            _ => merged.push((lo, hi)),
        }
    }
    // 減算: どの除外範囲にも入らない id のみを記述順・重複保持で残す（`lo <= id` の最後の組を二分探索）。
    included.retain(|&id| {
        let i = merged.partition_point(|&(lo, _)| lo <= id);
        i == 0 || id > merged[i - 1].1
    });
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
