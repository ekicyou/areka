//! `BlitOp` / `PlanBuilder`: 正規化定義からバックエンド非依存の合成命令列を導出する。
//!
//! element レイヤ順・有効 bind 集合の `animation-sort`→animation ID 順の2段合成規則を確定し、
//! 各命令へ `AtlasTable` 解決結果（`ElementId`・`Placement`）と変換行列を含める。入れ子 surface
//! 参照はオフセット累積で再帰的に inline 展開（flatten）し、visited 集合で循環を検出する。
//! キャンバス外形を算出し、同一入力に対して決定的な命令列を生成する。
//!
//! # 本 module の到達点（task 5.1／5.2／5.3）とシーム
//!
//! - **task 5.1**: 静的 element 層の列挙（[`push_static_element_ops`]・layer 昇順）。
//! - **task 5.2**: 有効 bind（`BindSet`）pattern0 層の合成対象化＋`animation-sort`→
//!   animation ID 順の 2 段規則（[`derive_ops`]）。静的層の後（上）へ、descend（既定）なら ID 昇順・
//!   ascend なら ID 降順で bind pattern0 を積む（design 決定5・画家のアルゴリズム）。全パーツが
//!   bind な surface でも非空 bind 集合から可視層を生成する。
//! - **task 5.3**: 入れ子 surface 参照の**多段再帰 flatten＋循環検出**
//!   （[`flatten_surface`]）。pattern0 の入れ子参照を pattern の (x,y) をオフセット累積して
//!   再帰的に inline 展開し、visited 祖先スタックで自己参照・相互参照の循環を検出して非パニックで
//!   枝を打ち切る（部分結果＋warn・要件 7.1/7.2/7.3）。
//! - **task 5.4（本 task）**: **placement None スキップ**（[`derive_ops`] へ `AtlasTable` を注入し
//!   `AtlasEntry.placement` が None の element は命令化せずスキップ・要件 6.3）＋**静的キャンバス外形
//!   算出**（[`compute_extent`]・[`Extent`]）。外形は**有効 bind 集合に依存せず**、全定義層（全 element
//!   ＋全 bind animation の pattern0＝有効 bind 非依存）の「配置オフセット＋原寸」の和集合を、原点 (0,0)
//!   固定・負オフセット分は原点でクリップして算出する（要件 6.5・議題2裁定 (A)）。
//!
//! - **task 5.5（本 task）**: **命令ゼロ時の 3 分類**（[`build_plan`]）。[`derive_ops`]（有効 bind
//!   依存の命令列）＋[`compute_extent`]（有効 bind 非依存の静的外形）を wrap し、対象 surface 不在→
//!   [`ComposeError::SurfaceNotFound`]・定義層皆無で外形 0×0→[`ComposeError::EmptyComposition`]・
//!   surface 存在＋外形非ゼロ（描画可能命令ゼロでも可）→`Ok(Extent)` の 3 状態を厳密に区別する
//!   （要件 6.6/10.5・議題2裁定）。非パニック・`error` ログ＋`Err`。
//!
//! 本 module は stub で偽装せず、生成する命令列はすべて実挙動・決定的・テスト済みである。
//!
//! - **task 7（本 task）**: design 署名どおり `visited` 祖先スタックのスクラッチを引数化する
//!   （[`build_plan`]/[`derive_ops`]/[`compute_extent`] が呼び手＝Composer の `Vec<u32>` を借用・
//!   各走査入口で `clear`）。これにより毎フレーム経路（`compose_into`）で visited の再割り当てが
//!   起きず、O(elements) の定常状態アロケーションなしを満たす（要件 10.3）。

use areka_emo_atlas::{AtlasTable, ElementId};
use areka_parsers::shell::{Interval, SortOrder};
use bevy_ecs::entity::Entity;

use crate::bind::BindSet;
use crate::error::ComposeError;
use crate::method::ComposeMethod;
use crate::normalized::{SurfaceMaster, Transform};
use crate::world::{AtlasBinding, EmoWorld, SurfaceIndex};

/// バックエンド非依存の転写命令（これがバックエンド差替えシーム＝design 決定1）。
///
/// `element` はアトラス参照（束縛時に `Some(ElementId)` へ解決済みのもののみ命令化される）、
/// `transform` は flatten 済み最終配置（M1 は平行移動のみ）、`method` は M1 では常に
/// [`ComposeMethod::Overlay`]。`Placement` の実引きは blit 実行時（task 6）に `AtlasTable::entry`
/// で O(1) で行うため、本命令は `ElementId` のみを保持する（design Contracts）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlitOp {
    /// アトラス参照（束縛済み `ElementId`・placement の実引きは blit 時）。
    pub element: ElementId,
    /// flatten 済み最終配置（M1 は平行移動のみ）。
    pub transform: Transform,
    /// 合成メソッド（M1 は常に [`ComposeMethod::Overlay`]）。
    pub method: ComposeMethod,
}

/// キャンバス外形（合成先1枚物ビットマップの寸法・原点 (0,0) 固定）。
///
/// 全定義層（全 element ＋全 bind animation の pattern0）の「配置オフセット＋原寸」の和集合
/// として [`compute_extent`] が算出する。**有効 bind 集合に依存しない静的量**（要件 6.5・議題2
/// 裁定 (A)）ゆえ、bind のオン/オフでサイズが変わらず、下流 emo-present のバッファ再利用・
/// キャッシュ・窓サイズ安定に用いる。負オフセット層は原点でクリップされ、外形は常に (0,0) を
/// 左上とする。`build_plan`（task 5.5）が `Result<Extent, ComposeError>` として wrap して返す。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Extent {
    /// 幅（px・原点 (0,0) からの右端）。
    pub w: u32,
    /// 高さ（px・原点 (0,0) からの下端）。
    pub h: u32,
}

/// 静的 element 層の転写命令を `out_ops` へ layer 昇順（同 layer は登場順）で積む（要件 4.1/4.5）。
///
/// 指定 surface の [`SurfaceMaster`] と平行な [`AtlasBinding`] を World から読み、element を
/// **layer 昇順・同 layer は登場順**（安定ソート）で列挙する。束縛が `Some(ElementId)` の
/// element のみ [`BlitOp`] を積み、`None`（未解決・bind 時に warn 済み）の element はスキップ
/// する（要件 4.3）。さらに束縛済み element でも [`AtlasTable::entry`] の `placement` が None
/// （全透明＝空エントリ）なら命令化せずスキップする（要件 6.3・通常系ゆえ warn しない）。
/// element の [`Transform`]（転記 x,y の行列表現・要件 4.2）と [`ComposeMethod`] をそのまま
/// 命令へ伝播する。
///
/// `out_ops` は呼び手のスクラッチ Vec を再利用する意図（要件 10.3・完全な再利用形は
/// task 7 で実現）ゆえ、本関数は **追記のみ**（clear しない）を行う。element 数ぶんの命令を
/// 末尾へ push する。surface が存在しない場合・`AtlasBinding` 未挿入の場合は何も積まない
/// （後続 task が `SurfaceNotFound`/未束縛の分類を担う）。
///
/// # 決定性（要件 4.5/10.1）
///
/// 同一の World／surface に対して、本関数は毎回同一の命令列を append する。layer 昇順の
/// **安定ソート**により同 layer element の登場順が保たれ、束縛（index 平行）も決定的ゆえ、
/// 生成される `Vec<BlitOp>` はバイト等価になる。
///
/// # シーム（後続 task）
///
/// bind 層（5.2）・入れ子 flatten（5.3）・placement None スキップと外形（5.4）・zero-op 分類
/// （5.5）は本関数を **拡張**して埋める。本関数は静的 element 経路のみを担い、偽の結果を
/// 返さない。
///
/// 消費は task 5.2 の [`derive_ops`] が本関数を wrap して行う（合成対象 surface 自身の静的層＝
/// (offset_x, offset_y)=(0,0)・bind pattern0 の入れ子参照＝pattern の (x,y) をオフセット）。
pub(crate) fn push_static_element_ops(
    out_ops: &mut Vec<BlitOp>,
    world: &EmoWorld,
    atlas: &AtlasTable,
    surface_id: u32,
    offset_x: i64,
    offset_y: i64,
) {
    let Some((master, binding)) = surface_and_binding(world, surface_id) else {
        // surface 不在・または binding 未挿入。本 task では静的 element を積まない（後続 task が
        // SurfaceNotFound/未束縛の分類を担当する）。
        return;
    };

    // layer 昇順・同 layer は登場順（安定ソート）。fold 段で既に整列済み（不変条件）だが、
    // plan は自 module で決定性を保証するため添字と layer を安定ソートし直す。
    let mut order: Vec<usize> = (0..master.elements.len()).collect();
    order.sort_by_key(|&i| master.elements[i].layer);

    for i in order {
        let element = &master.elements[i];
        // 束縛が Some の element のみ命令化する。None（未解決・bind 時に warn 済み）はスキップ。
        let Some(element_id) = binding.0.get(i).copied().flatten() else {
            tracing::trace!(
                target: "areka_emo_compose",
                surface_id,
                layer = element.layer,
                path = element.path.as_str(),
                "未束縛 element を静的命令からスキップ（bind 時 warn 済み）"
            );
            continue;
        };
        // placement None（全透明＝空エントリ）の element は命令化せずスキップ（要件 6.3）。原寸は
        // 既知ゆえ外形（compute_extent）へは寄与するが、転写命令には現れない（通常系・warn しない）。
        if atlas.entry(element_id).placement.is_none() {
            tracing::trace!(
                target: "areka_emo_compose",
                surface_id,
                layer = element.layer,
                path = element.path.as_str(),
                "placement None（全透明）element を命令からスキップ（外形へは寄与）"
            );
            continue;
        }
        // 入れ子参照の (x,y) オフセットを element 配置へ足し込む（1 段 inline 展開・要件 5.2 の
        // pattern0 入れ子参照。M1 は平行移動のみゆえ加算で足りる）。offset=(0,0) の合成対象自身の
        // 静的層では element の transform そのままになる。
        let (ex, ey) = element.transform.offset();
        out_ops.push(BlitOp {
            element: element_id,
            transform: Transform::translate(ex + offset_x, ey + offset_y),
            method: element.method.clone(),
        });
    }
}

/// 合成対象 surface の静的 element 層＋有効 bind pattern0 層の転写命令を `out_ops` へ導出する（要件 5.2/5.3/5.4/5.6）。
///
/// design「PlanBuilder（plan.rs）」の層列挙規則に厳密に従う:
///
/// 1. **静的 element 層**（i）: 合成対象 surface 自身の element を layer 昇順・同 layer は登場順で
///    [`push_static_element_ops`]（offset=(0,0)）により先頭へ積む（**下＝基底層**）。
/// 2. **有効 bind pattern0 層**（ii）: 合成対象 surface の `SurfaceMaster.animations` のうち、
///    interval が bind 種（[`Interval::Bind`]/[`Interval::BindRandom`]）**かつ** animation id が
///    `binds` に含まれる animation を「有効 bind」とし、その pattern0（index 0 の [`Pattern`]）を
///    **2 段ソート順**で静的層の**後（＝上）**へ積む。
///    - **段1（animation-sort・要件 5.3/5.6・design 決定5）**: [`EmoWorld::animation_sort`] が
///      [`SortOrder::Descend`]（既定）→ animation id **昇順**に描画（大 id が上）。[`SortOrder::Ascend`]
///      → id **降順**に描画（小 id が上）。これが画家のアルゴリズムの画素積層写像。
///    - **段2**: 上記方向の中で animation id をキーに整列する。
///
/// pattern0 の `surface_id >= 0` は**入れ子 surface 参照**として、参照先 [`SurfaceMaster`] の
/// **elements のみ**を pattern の (x,y) をオフセット**累積**して再帰的に inline 展開する
/// （多段 flatten・design 決定2）。参照先 surface 自身の bind animation は展開しない（design
/// 「`active_binds` の適用範囲は compose 対象 surface のみ」）。`surface_id < 0` はレイヤクリア/停止
/// センチネル＝**非描画 skip**（trace ログ・非パニック・要件 5.5 前段）。
///
/// # 循環検出（要件 7.1/7.2/7.3・design 決定2）
///
/// 入れ子参照の再帰は visited 祖先スタック（`Vec<u32>`）で保護する。既に**現在の祖先経路**に
/// 存在する surface を再訪すると（自己参照・相互参照の循環）、`warn!` の上その枝のみを打ち切って
/// 部分結果を残す（パニックしない・スタック深度に依存しない構造的停止保証）。visited は枝離脱時
/// に pop する祖先スタック規律ゆえ、非循環の重複参照（DAG の共有子）は各経路で正しく展開される。
///
/// # 決定性（要件 4.5/10.1）
///
/// 静的層は [`push_static_element_ops`] の安定順、bind 層は (animation id → 昇/降順) の全順序で
/// 整列し、入れ子 flatten も同一走査順で決定的ゆえ、同一入力（World／surface_id／binds）に対し
/// 毎回同一命令列を append する（循環入力でも有界かつバイト等価）。
///
/// # placement None スキップ（task 5.4・要件 6.3）
///
/// `atlas` を [`push_static_element_ops`] へ渡し、束縛済みでも `AtlasEntry.placement` が None
/// （全透明）の element は命令化せずスキップする。**有効 bind pattern0 の入れ子参照先も本経路の
/// element として展開される**ため、そこにある placement None element も同様にスキップされる。
///
/// # シーム（後続 task が本関数を拡張して埋める）
///
/// - **描画可能命令ゼロの Err 分類（`SurfaceNotFound`/`EmptyComposition`）** → task 5.5。本関数は
///   分類を返さず、不在 surface に対しては何も積まない（非パニック）。公開ファサード `build_plan`
///   （`Result<Extent, ComposeError>`・visited をスクラッチ引数へ）は 5.5 が本関数＋[`compute_extent`]
///   を wrap して導入する。
///
/// 消費は task 5.5 の [`build_plan`] が本関数を wrap して行う（本 module 内の呼び出し鎖
/// `push_static_element_ops`/`flatten_surface`/`surface_and_binding`/`is_bind_interval` は
/// [`build_plan`] → [`derive_ops`]/[`compute_extent`] の一点から辿られる）。
pub(crate) fn derive_ops(
    out_ops: &mut Vec<BlitOp>,
    visited: &mut Vec<u32>,
    world: &EmoWorld,
    atlas: &AtlasTable,
    surface_id: u32,
    binds: &BindSet,
) {
    // visited 祖先スタック（design: Composer スクラッチの Vec<u32>）は呼び手（[`build_plan`]／
    // Composer ファサード）から借用して再利用する（要件 10.3・定常状態アロケーションなし）。
    // 走査開始前に空へ戻し、祖先スタック規律（push-on-enter／pop-on-exit）で走査後も空に戻す。
    visited.clear();
    flatten_surface(out_ops, visited, world, atlas, surface_id, binds, 0, 0);
}

/// 合成対象 surface（top-level）／入れ子参照先 surface（再帰）を平坦化して `out_ops` へ積む。
///
/// - 層（i）: 当 surface の静的 element を (offset_x, offset_y) 累積で積む（[`push_static_element_ops`]）。
/// - 層（ii）: 有効 bind（interval が bind 種 ∧ id ∈ `binds`）の pattern0 を animation-sort→ID 順の
///   2 段規則で積む。pattern0 が入れ子 surface を参照（`surface_id >= 0`）するなら、pattern の (x,y)
///   を累積オフセットへ足して**再帰**する。
///
/// **`binds` の適用範囲は top-level のみ**ではなく全再帰段で同一 `binds` を参照するが、参照先の
/// bind animation は「その surface の `animations` に定義され `binds` に含まれる」もののみが有効で、
/// emo2 の bind パーツ surface（1100 系）は element のみで bind を持たないため差は生じない（design
/// 記録）。将来 BindSet のスコープ設計を入れ子ごとに分ける必要が生じた場合はシーム再検討。
///
/// visited は祖先スタック: 入口で `surface_id` を push、出口で pop。既訪問（＝現在の祖先経路に
/// 存在）なら循環ゆえ `warn!` して即 return（枝打ち切り・非パニック・要件 7.2/7.3）。
///
/// 引数は out_ops/visited のスクラッチ2本＋world/atlas の入力2本＋surface_id＋binds＋累積
/// offset(x,y) の計8本。全て再帰の各段で必要ゆえ削れない（スクラッチ構造体化は将来の整理余地）。
#[allow(clippy::too_many_arguments)]
fn flatten_surface(
    out_ops: &mut Vec<BlitOp>,
    visited: &mut Vec<u32>,
    world: &EmoWorld,
    atlas: &AtlasTable,
    surface_id: u32,
    binds: &BindSet,
    offset_x: i64,
    offset_y: i64,
) {
    // 循環検出（要件 7.2/7.3）: 現在の祖先経路に既出なら打ち切り（warn・非パニック）。
    if visited.contains(&surface_id) {
        tracing::warn!(
            target: "areka_emo_compose",
            surface_id,
            "入れ子参照の循環を検出・枝を打ち切り"
        );
        return;
    }
    visited.push(surface_id);

    // 層（i）: 当 surface 自身の静的 element を累積オフセットで積む（placement None はスキップ）。
    push_static_element_ops(out_ops, world, atlas, surface_id, offset_x, offset_y);

    // 当 surface 不在なら bind 層もない（後続 task が SurfaceNotFound 分類を担う）。存在する場合のみ
    // bind 層を積む。いずれにせよ枝離脱で visited を pop する。
    if let Some((master, _binding)) = surface_and_binding(world, surface_id) {
        // 層（ii）: 有効 bind（interval が bind 種 ∧ id ∈ binds）の animation id を収集する。
        let mut active_ids: Vec<u32> = master
            .animations
            .iter()
            .filter(|a| is_bind_interval(&a.interval) && binds.contains(a.id))
            .map(|a| a.id)
            .collect();

        // 段1: animation-sort に応じて描画順を決める（design 決定5・画家のアルゴリズム写像）。
        //   Descend（既定）→ id 昇順に描画（大 id が上）。Ascend → id 降順に描画（小 id が上）。
        // 段2: その方向の中で id をキーに整列する。
        match world.animation_sort() {
            SortOrder::Descend => active_ids.sort_unstable(),
            SortOrder::Ascend => active_ids.sort_unstable_by(|a, b| b.cmp(a)),
            // `SortOrder` は `#[non_exhaustive]`。未知値は既定 Descend（id 昇順描画）へ倒す（非パニック）。
            other => {
                tracing::warn!(
                    target: "areka_emo_compose",
                    surface_id,
                    sort = ?other,
                    "未知の animation-sort: 既定 Descend（id 昇順描画）として扱う"
                );
                active_ids.sort_unstable();
            }
        }

        // 描画順に各有効 bind の pattern0 を静的層の後（＝上）へ積む。
        for id in active_ids {
            // 同 id の animation は fold 段で単一化済み（後勝ち）ゆえ find で足りる。
            let Some(anim) = master.animations.iter().find(|a| a.id == id) else {
                continue;
            };
            // pattern0＝index 昇順の先頭 pattern（疎 index 許容ゆえ最小 index を pattern0 とする）。
            let Some(pattern0) = anim.patterns.iter().min_by_key(|p| p.index) else {
                // pattern を持たない bind animation は積むべき層がない（非パニック・skip）。
                tracing::trace!(
                    target: "areka_emo_compose",
                    surface_id,
                    animation_id = id,
                    "有効 bind に pattern が無い: skip"
                );
                continue;
            };

            if pattern0.surface_id < 0 {
                // 負値＝レイヤクリア/停止センチネル。非描画ゆえ命令を積まない（要件 5.5 前段）。
                tracing::trace!(
                    target: "areka_emo_compose",
                    surface_id,
                    animation_id = id,
                    sentinel = pattern0.surface_id,
                    "bind pattern0 がセンチネル（surface_id<0）: 非描画 skip"
                );
                continue;
            }

            // 入れ子 surface 参照: pattern0 の (x,y) を累積オフセットへ足して再帰的に flatten する
            // （多段・オフセット累積・要件 7.1）。循環は再帰入口の visited 判定で打ち切る（7.2/7.3）。
            let nested_id = pattern0.surface_id as u32;
            flatten_surface(
                out_ops,
                visited,
                world,
                atlas,
                nested_id,
                binds,
                offset_x + pattern0.x,
                offset_y + pattern0.y,
            );
        }
    }

    // 枝離脱: 祖先スタックから pop（非循環の重複参照は別経路で再展開可能に保つ・要件 7.1）。
    let popped = visited.pop();
    debug_assert_eq!(popped, Some(surface_id), "visited は祖先スタック（LIFO）");
}

/// キャンバス外形（[`Extent`]）を**有効 bind 集合に依存せず静的に**算出する（要件 6.5・議題2裁定 (A)）。
///
/// 母集合は **surface の全定義層**＝全 element ＋**全 bind animation の pattern0**（`binds` に
/// 含まれるか否かに関係なく全部）。各層 element の「累積配置オフセット＋原寸（[`AtlasEntry::original`]）」
/// の和集合を、原点 (0,0) 固定・負オフセット分は原点でクリップして取る。`extent = (max_x, max_y)`、
/// 各層で `max_x = max(max_x, max(0, offset_x + original.w))`・同様に `max_y`。**`placement: None`
/// （全透明）の element も原寸は既知ゆえ外形へ寄与する**（design 明記）。
///
/// これにより **bind のオン/オフでサイズが変わらない**（同一 surface へ異なる `BindSet` を渡しても
/// [`Extent`] が不変・emo-present のバッファ再利用/窓サイズ安定に必須）。入れ子 surface 参照は
/// [`flatten_surface`] と同一の pattern の (x,y) オフセット累積＋visited 祖先スタックによる循環検出で
/// 走査する（ops 経路との違いは「全 bind pattern0 を母集合とする」点のみ）。
///
/// element ゼロ・bind ゼロの surface（外形へ寄与する層が皆無）や不在 surface では `Extent { w:0, h:0 }`
/// を返す（0×0 退化の Err 分類は [`build_plan`] の責務・本関数は分類しない）。
pub(crate) fn compute_extent(
    visited: &mut Vec<u32>,
    world: &EmoWorld,
    atlas: &AtlasTable,
    surface_id: u32,
) -> Extent {
    let mut max_x: i64 = 0;
    let mut max_y: i64 = 0;
    // ops 経路と同じ祖先スタック規律で循環検出する（別走査・別集約器だが構造は共有）。visited は
    // 呼び手のスクラッチを借用して再利用する（要件 10.3）。走査開始前に空へ戻す。
    visited.clear();
    flatten_extent(&mut max_x, &mut max_y, visited, world, atlas, surface_id, 0, 0);
    Extent {
        // max_x/max_y は原点クリップ済みゆえ常に >= 0。u32 化は飽和で安全側（負にはならない）。
        w: max_x.max(0) as u32,
        h: max_y.max(0) as u32,
    }
}

/// plan 段の公開ファサード: 命令列を導出し、命令ゼロ時の 3 分類を確定して外形を返す（要件 1.4/6.6/10.5）。
///
/// [`derive_ops`]（有効 bind 依存の命令列）と [`compute_extent`]（有効 bind 非依存の静的外形）を
/// wrap し、design「Error Handling」表・議題2裁定の**3 状態**を厳密に区別する:
///
/// 1. **対象 surface 不在**（[`EmoWorld::surface`] が `None`）→ `error` ログ＋
///    [`ComposeError::SurfaceNotFound`]（要件 10.5）。命令/外形の算出**前**に返す。
/// 2. **surface 存在 & 外形非ゼロ**（描画可能命令ゼロでも可）→ `Ok(extent)`。全 element 全透明・
///    空の有効 bind 集合などで `out_ops` が空でも、これは**正常系**であり `Err` にしない
///    （要件 6.6・議題2裁定）。非ゼロの静的外形と（空かもしれない）命令列を返す。
/// 3. **定義層が皆無で外形 0×0**（surface は存在するが `compute_extent` が `Extent{0,0}`）→
///    `error` ログ＋[`ComposeError::EmptyComposition`]（要件 10.5・議題2裁定: これが唯一の真の
///    失敗となる退化ケース）。
///
/// `out_ops` は呼び手のスクラッチ Vec を再利用する意図（要件 10.3）ゆえ、**エントリで `clear`**
/// してから命令を積む（前回内容のゴミが混ざらない）。不在/退化の `Err` 時も `out_ops` は空である
/// （不在は算出前 return・退化は derive 済みでも 0×0 定義層皆無ゆえ命令は元々ゼロ）。
///
/// # 決定性（要件 10.1）
///
/// [`derive_ops`]/[`compute_extent`] がともに決定的ゆえ、同一入力（World／atlas／surface_id／
/// binds）に対し毎回同一の `(out_ops, Extent)` または同一 `Err` を返す。
///
/// # 非パニック（要件 1.4）
///
/// 全経路で `panic!` せず、失敗は `error` ログ＋`Err` で表現する（[`crate::error`] 規律）。
///
/// # visited スクラッチ（要件 10.3・buffer 再利用形）
///
/// `visited` は呼び手（Composer ファサード）が保持する祖先スタックのスクラッチ Vec を借用する。
/// 外形走査（[`compute_extent`]）と命令走査（[`derive_ops`]）は逐次に走るため、両者が同一
/// スクラッチを共有し**各走査の入口で `clear`**（走査後も祖先スタック規律で空へ戻る）する。これに
/// より定常状態（毎フレーム経路）で visited の再割り当てが起きない（要件 10.3・O(elements)）。
///
/// 本関数は plan module の呼び出し鎖の頂点（`build_plan` → `derive_ops`/`compute_extent` → …）で、
/// task 7 の Composer ファサード（`compose_into`/`compose`）から結線して消費される。
pub(crate) fn build_plan(
    out_ops: &mut Vec<BlitOp>,
    visited: &mut Vec<u32>,
    world: &EmoWorld,
    atlas: &AtlasTable,
    surface_id: u32,
    binds: &BindSet,
) -> Result<Extent, ComposeError> {
    // スクラッチ再利用: 前回内容を捨ててから積む（要件 10.3）。以降の Err 経路でも空を保つ。
    out_ops.clear();

    // 分類1: 対象 surface 不在 → SurfaceNotFound（算出前に返す・要件 10.5）。
    if world.surface(surface_id).is_none() {
        tracing::error!(
            target: "areka_emo_compose",
            surface_id,
            "合成対象 surface が存在しない: SurfaceNotFound"
        );
        return Err(ComposeError::SurfaceNotFound(surface_id));
    }

    // 静的外形（有効 bind 非依存）を先に算出する。0×0 なら定義層皆無の退化データ。visited は
    // compute_extent 入口で clear される（要件 10.3・共有スクラッチ）。
    let extent = compute_extent(visited, world, atlas, surface_id);

    // 分類3: 定義層皆無で外形 0×0 の退化データのみ真の失敗（議題2裁定・要件 10.5）。
    // 命令列は元々ゼロ（外形へ寄与する層＝命令の母集合が皆無）ゆえ out_ops は空のまま返す。
    if extent.w == 0 && extent.h == 0 {
        tracing::error!(
            target: "areka_emo_compose",
            surface_id,
            "定義層が皆無で外形 0×0 の退化データ: EmptyComposition"
        );
        return Err(ComposeError::EmptyComposition(surface_id));
    }

    // 有効 bind 依存の命令列を積む（描画可能命令ゼロ＝空 ops でもよい）。visited は derive_ops
    // 入口で clear され、外形走査と共有される（要件 10.3）。
    derive_ops(out_ops, visited, world, atlas, surface_id, binds);

    // 分類2: surface 存在＋外形非ゼロ → 正常系。out_ops が空でも（全透明・空 bind 集合）Err に
    // しない（要件 6.6・議題2裁定: 静的外形どおりの空命令列＝全透明返却）。
    Ok(extent)
}

/// [`compute_extent`] の再帰ワーカ: 当 surface の全定義層を走査し `max_x`/`max_y` を更新する。
///
/// [`flatten_surface`]（ops 経路）と同一の入れ子 flatten 構造（オフセット累積＋visited 祖先スタック
/// による循環検出）を持つが、**bind の母集合が異なる**: ops は有効 bind（`binds` ∩ bind animation）
/// のみ、本関数は**全 bind animation の pattern0**（`binds` 非依存・有効/非有効を問わない）を辿る。
/// これが「外形は有効 bind 集合に依存しない静的量」（要件 6.5）の実装核心である。
///
/// 各静的 element については `AtlasBinding` が `Some(ElementId)` のもののみ、`atlas.entry(id).original`
/// を「累積オフセット＋原寸」として外形へ寄与させる（未束縛 None は原寸不明ゆえ寄与しない）。
/// `placement` が None でも `original` は既知ゆえ寄与する（ops ではスキップされる層も外形は数える）。
///
/// 引数は max_x/max_y/visited のスクラッチ3本＋world/atlas＋surface_id＋累積 offset(x,y) の計8本。
/// [`flatten_surface`] と同型の再帰 walker ゆえ全引数が各段で必要（スクラッチ構造体化は将来余地）。
#[allow(clippy::too_many_arguments)]
fn flatten_extent(
    max_x: &mut i64,
    max_y: &mut i64,
    visited: &mut Vec<u32>,
    world: &EmoWorld,
    atlas: &AtlasTable,
    surface_id: u32,
    offset_x: i64,
    offset_y: i64,
) {
    // 循環検出（ops 経路と同一規律・要件 7.2/7.3）: 現在の祖先経路に既出なら打ち切り。
    if visited.contains(&surface_id) {
        tracing::warn!(
            target: "areka_emo_compose",
            surface_id,
            "外形算出: 入れ子参照の循環を検出・枝を打ち切り"
        );
        return;
    }
    visited.push(surface_id);

    if let Some((master, binding)) = surface_and_binding(world, surface_id) {
        // 当 surface の静的 element を外形へ寄与させる（束縛済み・placement 有無を問わず原寸で数える）。
        for (i, _element) in master.elements.iter().enumerate() {
            let Some(element_id) = binding.0.get(i).copied().flatten() else {
                // 未束縛（原寸不明）は外形に寄与できない。ops 側でも skip 済み。
                continue;
            };
            let original = atlas.entry(element_id).original;
            // 負オフセットは原点でクリップ（外形は (0,0) を左上に固定・負方向はみ出しは転写時クリップ）。
            *max_x = (*max_x).max((offset_x + original.w as i64).max(0));
            *max_y = (*max_y).max((offset_y + original.h as i64).max(0));
        }

        // **全** bind animation の pattern0 を母集合として辿る（有効/非有効を問わない・6.5 の核心）。
        // 描画順は外形に無関係（max の和集合ゆえ順序不変）だが、決定性のため id 昇順で走査する。
        let mut bind_ids: Vec<u32> = master
            .animations
            .iter()
            .filter(|a| is_bind_interval(&a.interval))
            .map(|a| a.id)
            .collect();
        bind_ids.sort_unstable();
        bind_ids.dedup();

        for id in bind_ids {
            let Some(anim) = master.animations.iter().find(|a| a.id == id) else {
                continue;
            };
            let Some(pattern0) = anim.patterns.iter().min_by_key(|p| p.index) else {
                continue;
            };
            if pattern0.surface_id < 0 {
                // センチネル（非描画）は外形に寄与しない（ops 経路と一致）。
                continue;
            }
            let nested_id = pattern0.surface_id as u32;
            flatten_extent(
                max_x,
                max_y,
                visited,
                world,
                atlas,
                nested_id,
                offset_x + pattern0.x,
                offset_y + pattern0.y,
            );
        }
    }

    // 枝離脱: 祖先スタックから pop（非循環の重複参照は別経路で再走査可能・要件 7.1）。
    let popped = visited.pop();
    debug_assert_eq!(popped, Some(surface_id), "外形算出: visited は祖先スタック（LIFO）");
}

/// interval が bind 種（`Interval::Bind`/`Interval::BindRandom`）か（要件 5.2・design 層列挙）。
///
/// `Interval::Random`（純ランダム・非 bind）は有効 bind の対象にしない。`Interval` は
/// `#[non_exhaustive]` ゆえ未知 variant は bind でないものとして扱う（非パニック）。
fn is_bind_interval(interval: &Interval) -> bool {
    matches!(interval, Interval::Bind | Interval::BindRandom { .. })
}

/// 指定 surface id の [`SurfaceMaster`] と [`AtlasBinding`] を同時に読む（本 module 内補助）。
///
/// `SurfaceIndex`（疎 id→Entity・O(1)）で entity を引き、`SurfaceMaster` と `AtlasBinding` を
/// 併せて借用する。いずれか（surface 不在・binding 未挿入）が欠けると `None`。既存の公開
/// `world()` アクセサ経由で読むため、公開 API を広げない。
///
/// [`push_static_element_ops`]／[`derive_ops`] 経由で使う本 module 内補助。
fn surface_and_binding(
    world: &EmoWorld,
    surface_id: u32,
) -> Option<(&SurfaceMaster, &AtlasBinding)> {
    let w = world.world();
    let entity: Entity = *w.resource::<SurfaceIndex>().0.get(&surface_id)?;
    let master = w.get::<SurfaceMaster>(entity)?;
    let binding = w.get::<AtlasBinding>(entity)?;
    Some((master, binding))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::EmoWorld;
    use areka_emo_atlas::{
        AlphaParams, AtlasTable, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
    };
    use areka_parsers::shell::{
        Animation, AppendTarget, DefRef, Element, ElementPath, Interval, Pattern, Shell, SortOrder,
        Surface,
    };
    use std::path::Path;

    // ---- 合成モデルビルダ（task 4 テストと同一パターン・実 fixture パースは統合 task 8 送り）----

    /// element 1 本（layer/path/x/y 指定）。
    fn elem(layer: u32, path: &str, x: i64, y: i64) -> Element {
        Element {
            layer,
            path: ElementPath::new(path.to_string()),
            x,
            y,
        }
    }

    /// element のみを持つ最小 surface（collision/animation 空・単一ターゲット）。
    fn surface(id: u32, elements: Vec<Element>) -> Surface {
        surface_with_anims(id, elements, Vec::new())
    }

    /// element＋animation を持つ surface（collision 空・単一ターゲット）。
    fn surface_with_anims(id: u32, elements: Vec<Element>, animations: Vec<Animation>) -> Surface {
        Surface {
            id,
            targets: vec![AppendTarget::Single(id)],
            elements,
            collisions: Vec::new(),
            animations,
        }
    }

    /// bind animation 1 本（interval=`Interval::Bind`・pattern0 が入れ子 surface_id を x,y で参照）。
    ///
    /// pattern index は 0 を pattern0（先頭）とし、識別用に index=5 の余分な pattern も加えて
    /// 「pattern0＝最小 index の pattern」を選ぶ実装を突く（疎 index 許容・model 契約）。
    fn bind_anim(id: u32, ref_surface_id: i64, x: i64, y: i64) -> Animation {
        Animation {
            id,
            interval: Interval::Bind,
            patterns: vec![
                Pattern {
                    index: 0,
                    surface_id: ref_surface_id,
                    wait: 0,
                    x,
                    y,
                },
                // pattern0 以外は本 task では未使用（pattern0 のみ合成対象・要件 5.2）。
                Pattern {
                    index: 5,
                    surface_id: 999_999,
                    wait: 0,
                    x: 7,
                    y: 7,
                },
            ],
        }
    }

    /// 与えた surface 群を登場順 definitions（plain のみ）で包む `Shell`（animation-sort 未指定＝既定 descend）。
    fn shell_of(surfaces: Vec<Surface>) -> Shell {
        shell_of_with_sort(surfaces, None)
    }

    /// 与えた surface 群を `animation-sort` 指定つきで包む `Shell`。
    fn shell_of_with_sort(surfaces: Vec<Surface>, animation_sort: Option<SortOrder>) -> Shell {
        let definitions = (0..surfaces.len()).map(DefRef::Surface).collect();
        Shell {
            surfaces,
            appends: Vec::new(),
            aliases: Vec::new(),
            animation_sort,
            collision_sort: None,
            definitions,
        }
    }

    /// tightly-packed 2×2 の premultiplied BGRA・不透明（α=255）画像スペック。
    fn opaque_2x2() -> (u32, u32, u32, Vec<u8>, bool) {
        let (w, h) = (2u32, 2u32);
        let stride = w * 4;
        let bgra = vec![
            10, 20, 30, 255, //
            40, 50, 60, 255, //
            70, 80, 90, 255, //
            100, 110, 120, 255,
        ];
        (w, h, stride, bgra, true)
    }

    /// `MemoryDecoder` へ base.join(rel) の実パスで不透明画像を登録する。
    fn register(dec: &mut MemoryDecoder, base: &Path, rel: &str) {
        let (w, h, stride, bgra, has_alpha) = opaque_2x2();
        dec.insert(base.join(rel), w, h, stride, bgra, has_alpha);
    }

    /// 指定 rel_path 群を含む単一 SurfaceSet を bake し `AtlasTable` を得る（COM/WIC 非依存・SetId(0)）。
    fn bake_atlas(base: &Path, rels: &[&str]) -> AtlasTable {
        let elements: Vec<Element> = rels.iter().map(|r| elem(0, r, 0, 0)).collect();
        let surfaces = vec![surface(0, elements)];
        let mut dec = MemoryDecoder::new();
        for r in rels {
            register(&mut dec, base, r);
        }
        let set = SurfaceSet {
            surfaces: &surfaces,
            base_dir: base,
            alpha_params: AlphaParams {
                use_self_alpha: UseSelfAlpha::On,
            },
        };
        let result = bake(&[set], &dec, PackConfig::default());
        assert!(result.errors.is_empty(), "bake セットアップは失敗しない");
        result.table
    }

    /// element を、その `path` の ElementId から逆引きするための対応表を作る（テスト補助）。
    ///
    /// BlitOp は ElementId のみを保持するため、命令列の順序を element の識別子（ここでは path）
    /// へ写像して検証する。bake 済み atlas は path→ElementId が一意ゆえ、ElementId→path も一意。
    fn id_to_path<'a>(atlas: &AtlasTable, rels: &[&'a str]) -> Vec<(ElementId, &'a str)> {
        rels.iter()
            .map(|r| {
                let id = atlas
                    .resolve(SetId(0), r)
                    .unwrap_or_else(|| panic!("{r} は bake 済みで resolve できる"));
                (id, *r)
            })
            .collect()
    }

    /// テスト①（受入基準・要件 4.1）: layer [2,0,1]（登場順）→ ops は layer 昇順 0,1,2。
    ///
    /// 各 op を ElementId 経由で element path へ逆写像し、命令列が layer 昇順に整列することを
    /// 検証する（登場順は 2→0→1 だが、layer 昇順で 0(b)→1(c)→2(a)）。
    #[test]
    fn ops_ordered_by_layer_ascending() {
        let base = Path::new("shell/master");
        let rels = ["a.png", "b.png", "c.png"];
        let atlas = bake_atlas(base, &rels);
        let map = id_to_path(&atlas, &rels);

        // 登場順 [layer2=a, layer0=b, layer1=c]。
        let shell = shell_of(vec![surface(
            1000,
            vec![
                elem(2, "a.png", 0, 0),
                elem(0, "b.png", 0, 0),
                elem(1, "c.png", 0, 0),
            ],
        )]);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));

        let mut ops = Vec::new();
        push_static_element_ops(&mut ops, &world, &atlas, 1000, 0, 0);

        let paths: Vec<&str> = ops
            .iter()
            .map(|op| {
                map.iter()
                    .find(|(id, _)| *id == op.element)
                    .map(|(_, p)| *p)
                    .expect("op の ElementId は既知")
            })
            .collect();
        // layer 昇順: b(0) → c(1) → a(2)。
        assert_eq!(paths, vec!["b.png", "c.png", "a.png"]);
    }

    /// テスト②（要件 4.1）: 同一 layer は登場（定義）順を保つ。
    #[test]
    fn same_layer_keeps_appearance_order() {
        let base = Path::new("shell/master");
        let rels = ["first.png", "second.png"];
        let atlas = bake_atlas(base, &rels);
        let map = id_to_path(&atlas, &rels);

        // 両者 layer=5。登場順は first → second。
        let shell = shell_of(vec![surface(
            1,
            vec![elem(5, "first.png", 0, 0), elem(5, "second.png", 0, 0)],
        )]);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));

        let mut ops = Vec::new();
        push_static_element_ops(&mut ops, &world, &atlas, 1, 0, 0);

        let paths: Vec<&str> = ops
            .iter()
            .map(|op| map.iter().find(|(id, _)| *id == op.element).unwrap().1)
            .collect();
        assert_eq!(paths, vec!["first.png", "second.png"]);
    }

    /// テスト③（要件 4.5/10.1）: 同一 World／surface から2回導出→命令列がバイト等価。
    #[test]
    fn derivation_is_deterministic() {
        let base = Path::new("shell/master");
        let rels = ["a.png", "b.png", "c.png"];
        let atlas = bake_atlas(base, &rels);

        let shell = shell_of(vec![surface(
            1000,
            vec![
                elem(2, "a.png", 1, 2),
                elem(0, "b.png", 3, 4),
                elem(1, "c.png", 5, 6),
            ],
        )]);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));

        let mut ops1 = Vec::new();
        push_static_element_ops(&mut ops1, &world, &atlas, 1000, 0, 0);
        let mut ops2 = Vec::new();
        push_static_element_ops(&mut ops2, &world, &atlas, 1000, 0, 0);

        assert_eq!(ops1, ops2, "同一入力→同一 ops（バイト等価）");
        assert_eq!(ops1.len(), 3);
    }

    /// テスト④（要件 4.3/6.3 前段）: 未束縛（None）element は命令化されずスキップされる（非パニック）。
    #[test]
    fn unresolved_binding_is_skipped() {
        let base = Path::new("shell/master");
        // atlas には known.png のみ焼く（bogus.png は未束縛＝None になる）。
        let atlas = bake_atlas(base, &["known.png"]);
        let known_id = atlas.resolve(SetId(0), "known.png").expect("known 解決");

        let shell = shell_of(vec![surface(
            1000,
            vec![elem(0, "known.png", 0, 0), elem(1, "bogus.png", 0, 0)],
        )]);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));

        let mut ops = Vec::new();
        push_static_element_ops(&mut ops, &world, &atlas, 1000, 0, 0);

        // bogus.png は None ゆえスキップ＝命令は known.png の1本のみ。
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].element, known_id);
    }

    /// テスト⑤（要件 4.2）: 命令の Transform は element の translate(x,y) と一致し、純平行移動である。
    #[test]
    fn transform_propagates_as_translation() {
        let base = Path::new("shell/master");
        let atlas = bake_atlas(base, &["p.png"]);

        let shell = shell_of(vec![surface(7, vec![elem(0, "p.png", 12, -8)])]);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));

        let mut ops = Vec::new();
        push_static_element_ops(&mut ops, &world, &atlas, 7, 0, 0);

        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].transform, Transform::translate(12, -8));
        assert_eq!(ops[0].transform.offset(), (12, -8));
        assert!(
            ops[0].transform.is_translation(),
            "M1 は単位行列＋平行移動（要件 4.2）"
        );
        assert_eq!(ops[0].method, ComposeMethod::Overlay);
    }

    /// 追記形の確認: 既存 ops を clear せず末尾追記する（スクラッチ再利用意図・要件 10.3）。
    #[test]
    fn appends_without_clearing() {
        let base = Path::new("shell/master");
        let atlas = bake_atlas(base, &["p.png"]);
        let shell = shell_of(vec![surface(1, vec![elem(0, "p.png", 0, 0)])]);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));

        let sentinel = BlitOp {
            element: ElementId(u32::MAX),
            transform: Transform::identity(),
            method: ComposeMethod::Overlay,
        };
        let mut ops = vec![sentinel.clone()];
        push_static_element_ops(&mut ops, &world, &atlas, 1, 0, 0);

        // 先頭 sentinel が残り、末尾へ element 命令が追記される。
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0], sentinel);
    }

    /// surface 不在では何も積まない（後続 task が SurfaceNotFound 分類を担う・本 task は非追記）。
    #[test]
    fn missing_surface_pushes_nothing() {
        let world = EmoWorld::build(&shell_of(Vec::new()));
        // surface 不在ゆえ atlas は引かれない（空 atlas で足りる）。
        let atlas = bake_atlas(Path::new("shell/master"), &["dummy.png"]);
        let mut ops = Vec::new();
        push_static_element_ops(&mut ops, &world, &atlas, 9999, 0, 0);
        assert!(ops.is_empty());
    }

    // ── task 5.2: 有効 bind pattern0 の合成対象化＋animation-sort→ID 順の2段規則 ──────────

    /// 命令列を element path 列へ逆写像する（ElementId→path が一意な bake 前提・テスト補助）。
    fn ops_to_paths(ops: &[BlitOp], map: &[(ElementId, &str)]) -> Vec<String> {
        ops.iter()
            .map(|op| {
                map.iter()
                    .find(|(id, _)| *id == op.element)
                    .map(|(_, p)| p.to_string())
                    .expect("op の ElementId は既知")
            })
            .collect()
    }

    /// 各 bind パーツ surface（element 1 本・path で識別可能）を持つ Shell を組む共通土台。
    ///
    /// 合成対象 surface `host_id` は静的 element を任意本持ち、各 bind animation の pattern0 が
    /// `bind_surfaces`（(animation_id, ref_surface_id, path) の並び）の各入れ子 surface を参照する。
    /// 戻り値は (world, atlas, id_to_path 表)。
    fn build_bind_world(
        host_id: u32,
        host_elements: Vec<Element>,
        host_element_rels: &[&str],
        bind_surfaces: &[(u32, u32, &str)], // (animation_id, ref_surface_id, part_path)
        animation_sort: Option<SortOrder>,
    ) -> (EmoWorld, AtlasTable) {
        let base = Path::new("shell/master");

        // 全 element path（host の静的分＋各 bind パーツ分）を bake する。
        let mut all_rels: Vec<&str> = host_element_rels.to_vec();
        for (_, _, path) in bind_surfaces {
            all_rels.push(path);
        }
        let atlas = bake_atlas(base, &all_rels);

        // host surface: 静的 element ＋ bind animation 群。
        let anims: Vec<Animation> = bind_surfaces
            .iter()
            .map(|(aid, ref_id, _)| bind_anim(*aid, *ref_id as i64, 0, 0))
            .collect();
        let host = surface_with_anims(host_id, host_elements, anims);

        // 各 bind パーツ surface: element 1 本（path で識別）。
        let parts: Vec<Surface> = bind_surfaces
            .iter()
            .map(|(_, ref_id, path)| surface(*ref_id, vec![elem(0, path, 0, 0)]))
            .collect();

        let mut surfaces = vec![host];
        surfaces.extend(parts);

        let shell = shell_of_with_sort(surfaces, animation_sort);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));
        (world, atlas)
    }

    /// テスト①（受入基準・要件 5.2/5.3/5.6）: 複数有効 bind・sort 既定（descend）→ ID 昇順に描画。
    ///
    /// animation id [3,1,2]（登場順）の各 pattern0 が別 surface（part3/part1/part2）を参照。
    /// animation-sort 未指定（既定 descend）ゆえ ID 昇順描画で bind 層は part1→part2→part3 の順。
    #[test]
    fn active_binds_descend_default_draws_in_id_ascending_order() {
        let (world, atlas) = build_bind_world(
            1000,
            Vec::new(),
            &[],
            // 登場順は 3,1,2（描画順とは別）。各 pattern0 が別 surface を参照。
            &[(3, 1300, "part3.png"), (1, 1100, "part1.png"), (2, 1200, "part2.png")],
            None, // 未指定＝既定 descend。
        );
        let map = id_to_path(&atlas, &["part1.png", "part2.png", "part3.png"]);

        let binds = BindSet::from_ids([1, 2, 3]);
        let mut ops = Vec::new();
        derive_ops(&mut ops, &mut Vec::new(), &world, &atlas, 1000, &binds);

        // descend（既定）→ ID 昇順描画: part1(1) → part2(2) → part3(3)。
        assert_eq!(ops_to_paths(&ops, &map), vec!["part1.png", "part2.png", "part3.png"]);
    }

    /// テスト②（要件 5.3）: animation-sort=ascend → ID 降順に描画（小 ID が上）。
    #[test]
    fn active_binds_ascend_draws_in_id_descending_order() {
        let (world, atlas) = build_bind_world(
            1000,
            Vec::new(),
            &[],
            &[(3, 1300, "part3.png"), (1, 1100, "part1.png"), (2, 1200, "part2.png")],
            Some(SortOrder::Ascend),
        );
        let map = id_to_path(&atlas, &["part1.png", "part2.png", "part3.png"]);

        let binds = BindSet::from_ids([1, 2, 3]);
        let mut ops = Vec::new();
        derive_ops(&mut ops, &mut Vec::new(), &world, &atlas, 1000, &binds);

        // ascend → ID 降順描画: part3(3) → part2(2) → part1(1)。
        assert_eq!(ops_to_paths(&ops, &map), vec!["part3.png", "part2.png", "part1.png"]);
    }

    /// テスト③（要件 5.2）: BindSet に含まれない bind animation は合成対象から除外される。
    #[test]
    fn only_binds_in_bindset_are_included() {
        let (world, atlas) = build_bind_world(
            1000,
            Vec::new(),
            &[],
            &[(1, 1100, "part1.png"), (2, 1200, "part2.png"), (3, 1300, "part3.png")],
            None,
        );
        let map = id_to_path(&atlas, &["part1.png", "part2.png", "part3.png"]);

        // id=2 のみ有効（1,3 は非活性）。
        let binds = BindSet::from_ids([2]);
        let mut ops = Vec::new();
        derive_ops(&mut ops, &mut Vec::new(), &world, &atlas, 1000, &binds);

        // part2 のみ命令化される。
        assert_eq!(ops_to_paths(&ops, &map), vec!["part2.png"]);
    }

    /// テスト④（要件 5.4）: 静的 element ゼロ・全パーツ bind の surface でも非空 bind 集合→非空 ops。
    /// 空 bind 集合では bind 命令ゼロ（非パニック・全透明処理は 5.5/6.6）。
    #[test]
    fn bind_only_surface_produces_layers_from_nonempty_bindset() {
        let (world, atlas) = build_bind_world(
            1000, // emo2 surface1000 相当（static element なし・全 bind）。
            Vec::new(),
            &[],
            &[(1, 1100, "part1.png"), (2, 1200, "part2.png")],
            None,
        );
        let map = id_to_path(&atlas, &["part1.png", "part2.png"]);

        // 非空 bind 集合 → 可視層が生成される（空白にしない）。
        let binds = BindSet::from_ids([1, 2]);
        let mut ops = Vec::new();
        derive_ops(&mut ops, &mut Vec::new(), &world, &atlas, 1000, &binds);
        assert!(!ops.is_empty(), "全 bind surface でも非空 bind 集合から可視層を生む");
        assert_eq!(ops_to_paths(&ops, &map), vec!["part1.png", "part2.png"]);

        // 空 bind 集合 → bind 命令なし（静的 element も無いので空・非パニック）。
        let empty = BindSet::default();
        let mut ops_empty = Vec::new();
        derive_ops(&mut ops_empty, &mut Vec::new(), &world, &atlas, 1000, &empty);
        assert!(ops_empty.is_empty(), "空 bind 集合では bind 命令ゼロ（非パニック）");
    }

    /// テスト⑤（要件 5.2・design 層列挙 i/ii）: 静的 element 層が bind 層の**前（下）**に来る。
    #[test]
    fn static_elements_precede_bind_layers() {
        let (world, atlas) = build_bind_world(
            1000,
            vec![elem(0, "base.png", 0, 0)], // 静的 element 1 本（基底）。
            &["base.png"],
            &[(1, 1100, "part1.png")],
            None,
        );
        let map = id_to_path(&atlas, &["base.png", "part1.png"]);

        let binds = BindSet::from_ids([1]);
        let mut ops = Vec::new();
        derive_ops(&mut ops, &mut Vec::new(), &world, &atlas, 1000, &binds);

        // 静的 base（下）→ bind part1（上）。
        assert_eq!(ops_to_paths(&ops, &map), vec!["base.png", "part1.png"]);
    }

    /// テスト⑥（要件 5.5 前段）: pattern0 の surface_id<0 はセンチネル＝命令を積まず skip（非パニック）。
    #[test]
    fn sentinel_pattern0_is_skipped() {
        let base = Path::new("shell/master");
        // part1 は正常参照、bind id=2 の pattern0 は surface_id=-2（センチネル）。
        let atlas = bake_atlas(base, &["part1.png"]);
        let map = id_to_path(&atlas, &["part1.png"]);

        let host = surface_with_anims(
            1000,
            Vec::new(),
            vec![
                bind_anim(1, 1100, 0, 0),  // 正常参照。
                bind_anim(2, -2, 0, 0),    // センチネル（非描画）。
            ],
        );
        let part1 = surface(1100, vec![elem(0, "part1.png", 0, 0)]);
        let shell = shell_of(vec![host, part1]);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));

        let binds = BindSet::from_ids([1, 2]);
        let mut ops = Vec::new();
        derive_ops(&mut ops, &mut Vec::new(), &world, &atlas, 1000, &binds);

        // センチネル bind id=2 は積まれず、part1 のみ（非パニック）。
        assert_eq!(ops_to_paths(&ops, &map), vec!["part1.png"]);
    }

    /// テスト⑦（要件 4.5/10.1）: 同一入力で 2 回導出→命令列がバイト等価（bind 経路の決定性）。
    #[test]
    fn bind_derivation_is_deterministic() {
        let (world, atlas) = build_bind_world(
            1000,
            vec![elem(0, "base.png", 0, 0)],
            &["base.png"],
            &[(3, 1300, "part3.png"), (1, 1100, "part1.png"), (2, 1200, "part2.png")],
            None,
        );
        let _ = &atlas; // atlas は bind_atlas 済み（以降の resolve は不要）。

        let binds = BindSet::from_ids([1, 2, 3]);
        let mut ops1 = Vec::new();
        derive_ops(&mut ops1, &mut Vec::new(), &world, &atlas, 1000, &binds);
        let mut ops2 = Vec::new();
        derive_ops(&mut ops2, &mut Vec::new(), &world, &atlas, 1000, &binds);

        assert_eq!(ops1, ops2, "同一入力→同一 ops（バイト等価）");
        // base(静的1) ＋ bind 3 本＝4 命令。
        assert_eq!(ops1.len(), 4);
    }

    /// pattern0 の (x,y) が入れ子参照 element の配置へオフセット加算される（要件 5.2・1 段 inline 展開）。
    #[test]
    fn nested_pattern0_offset_is_applied_to_element_transform() {
        let base = Path::new("shell/master");
        let atlas = bake_atlas(base, &["part.png"]);
        let part_id = atlas.resolve(SetId(0), "part.png").expect("part 解決");

        // bind id=1 の pattern0 が surface 1100 を (30, -20) で参照。part の element は自 (5, 6)。
        let host = surface_with_anims(1000, Vec::new(), vec![bind_anim(1, 1100, 30, -20)]);
        let part = surface(1100, vec![elem(0, "part.png", 5, 6)]);
        let shell = shell_of(vec![host, part]);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));

        let binds = BindSet::from_ids([1]);
        let mut ops = Vec::new();
        derive_ops(&mut ops, &mut Vec::new(), &world, &atlas, 1000, &binds);

        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].element, part_id);
        // element 自 (5,6) ＋ pattern0 offset (30,-20) ＝ (35, -14)。
        assert_eq!(ops[0].transform.offset(), (35, -14));
    }

    // ── task 5.3: 入れ子 surface 参照の多段 flatten と循環検出（visited 集合） ─────────────

    /// bind animation を任意 (x,y) つきで組む（bind_anim の x,y=0 固定を拡張・pattern0 のみ有意）。
    fn bind_anim_xy(id: u32, ref_surface_id: i64, x: i64, y: i64) -> Animation {
        bind_anim(id, ref_surface_id, x, y)
    }

    /// テスト5.3-①（受入基準・要件 7.1/7.2/7.3）: 自己参照 bind はスタックオーバーフローせず、
    /// 部分結果（自 surface の静的 element）が得られ、循環枝は打ち切られる（warn・非パニック）。
    ///
    /// surface 1000 は静的 element `self.png` を持ち、かつ自身の bind pattern0 が **surface 1000
    /// 自身**を参照する（自己参照 = 循環）。derive_ops は無限再帰せず（テストが完走すれば O.K.）、
    /// 静的層 self.png は積まれ、自己参照枝は visited で打ち切られる。
    #[test]
    fn self_reference_does_not_overflow_and_yields_partial_result() {
        let base = Path::new("shell/master");
        let atlas = bake_atlas(base, &["self.png"]);
        let self_id = atlas.resolve(SetId(0), "self.png").expect("self 解決");

        // surface 1000: 静的 element self.png ＋ bind id=1 が surface 1000 自身を参照。
        let host = surface_with_anims(
            1000,
            vec![elem(0, "self.png", 0, 0)],
            vec![bind_anim(1, 1000, 0, 0)], // 自己参照。
        );
        let shell = shell_of(vec![host]);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));

        let binds = BindSet::from_ids([1]);
        let mut ops = Vec::new();
        // これが無限再帰すると本テストは完走できない（スタックオーバーフローで abort）。
        derive_ops(&mut ops, &mut Vec::new(), &world, &atlas, 1000, &binds);

        // 静的層 self.png は積まれる（部分結果）。
        assert!(!ops.is_empty(), "自己参照でも静的層の部分結果が得られる");
        // ops は有界（自己参照枝は打ち切られる）。静的 self.png（1本）＋ bind pattern0 が
        // surface 1000 を参照→ visited で 1000 は既訪問ゆえ枝打ち切り＝追加 element なし。
        // よって self.png 1本のみ。
        let paths = ops_to_paths(&ops, &[(self_id, "self.png")]);
        assert_eq!(paths, vec!["self.png"], "自己参照枝は打ち切り＝静的層のみ");
    }

    /// テスト5.3-②（要件 7.2）: 相互参照（A→B→A）はスタックオーバーフローせず、A 再入で
    /// 打ち切られ部分結果が得られる（非パニック）。
    ///
    /// surface A=1000 の bind→B=1100、B の bind→A=1000。A から derive すると
    /// A 静的 → B 静的（B の bind 経由で A 再入は visited で打ち切り）。無限再帰しない。
    #[test]
    fn mutual_reference_cycle_is_pruned_without_overflow() {
        let base = Path::new("shell/master");
        let atlas = bake_atlas(base, &["a.png", "b.png"]);
        let a_id = atlas.resolve(SetId(0), "a.png").expect("a 解決");
        let b_id = atlas.resolve(SetId(0), "b.png").expect("b 解決");

        // A=1000: 静的 a.png ＋ bind id=1 → B=1100。
        let a = surface_with_anims(
            1000,
            vec![elem(0, "a.png", 0, 0)],
            vec![bind_anim(1, 1100, 0, 0)],
        );
        // B=1100: 静的 b.png ＋ bind id=1 → A=1000（相互参照）。
        let b = surface_with_anims(
            1100,
            vec![elem(0, "b.png", 0, 0)],
            vec![bind_anim(1, 1000, 0, 0)],
        );
        let shell = shell_of(vec![a, b]);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));

        let binds = BindSet::from_ids([1]);
        let mut ops = Vec::new();
        derive_ops(&mut ops, &mut Vec::new(), &world, &atlas, 1000, &binds);

        // A 静的 → B 静的まで到達し、B→A の再入は打ち切り。a.png と b.png の各1本。
        let map = [(a_id, "a.png"), (b_id, "b.png")];
        let paths = ops_to_paths(&ops, &map);
        assert_eq!(paths, vec!["a.png", "b.png"], "相互参照は A 再入で打ち切り＝有界");
    }

    /// テスト5.3-③（要件 7.1・オフセット累積）: 非循環の多段入れ子 A→B→C でオフセットが累積する。
    ///
    /// A=1000 の bind→B=1100 を (10,5)、B=1100 の bind→C=1200 を (100,50)、C=1200 の element は
    /// 自 (2,3)。C の element の最終オフセット＝(10+100+2, 5+50+3)＝(112, 58)。多段累積を検証。
    #[test]
    fn offset_accumulates_across_multilevel_nesting() {
        let base = Path::new("shell/master");
        let atlas = bake_atlas(base, &["c.png"]);
        let c_id = atlas.resolve(SetId(0), "c.png").expect("c 解決");

        // A=1000: 静的 element なし・bind id=1 → B=1100 at (10,5)。
        let a = surface_with_anims(1000, Vec::new(), vec![bind_anim_xy(1, 1100, 10, 5)]);
        // B=1100: 静的 element なし・bind id=1 → C=1200 at (100,50)。
        let b = surface_with_anims(1100, Vec::new(), vec![bind_anim_xy(1, 1200, 100, 50)]);
        // C=1200: 静的 element c.png at (2,3)。
        let c = surface(1200, vec![elem(0, "c.png", 2, 3)]);
        let shell = shell_of(vec![a, b, c]);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));

        let binds = BindSet::from_ids([1]);
        let mut ops = Vec::new();
        derive_ops(&mut ops, &mut Vec::new(), &world, &atlas, 1000, &binds);

        assert_eq!(ops.len(), 1, "C の element 1 本のみ（A/B は静的 element なし）");
        assert_eq!(ops[0].element, c_id);
        // 累積: A→B (10,5) ＋ B→C (100,50) ＋ C element 自 (2,3) ＝ (112, 58)。
        assert_eq!(ops[0].transform.offset(), (112, 58), "多段オフセット累積");
    }

    /// テスト5.3-④（要件 7.1・ancestor-stack 規律）: 非循環で同一 surface を2回参照すると、
    /// 双方とも展開される（visited は祖先スタックゆえ枝離脱で pop・偽陽性打ち切りをしない）。
    ///
    /// A=1000 の bind id=1 → 共有子 S=1300 at (10,0)、bind id=2 → 中間 B=1100（B の bind→S=1300 at
    /// (0,20)）。S は祖先でない2経路（A 直下・A→B 経由）から参照される。S の element は各経路で
    /// 展開され、offset は (10,0) と (0,20) で異なる2命令になる（誤って一度きりに刈られない）。
    #[test]
    fn noncyclic_shared_child_expands_on_each_path() {
        let base = Path::new("shell/master");
        let atlas = bake_atlas(base, &["s.png"]);
        let s_id = atlas.resolve(SetId(0), "s.png").expect("s 解決");

        // A=1000: bind id=1 → S=1300 at (10,0)、bind id=2 → B=1100 at (0,0)。
        let a = surface_with_anims(
            1000,
            Vec::new(),
            vec![bind_anim_xy(1, 1300, 10, 0), bind_anim_xy(2, 1100, 0, 0)],
        );
        // B=1100: bind id=1 → S=1300 at (0,20)。
        let b = surface_with_anims(1100, Vec::new(), vec![bind_anim_xy(1, 1300, 0, 20)]);
        // S=1300: 共有子 element s.png at (0,0)。
        let s = surface(1300, vec![elem(0, "s.png", 0, 0)]);
        let shell = shell_of(vec![a, b, s]);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));

        let binds = BindSet::from_ids([1, 2]);
        let mut ops = Vec::new();
        derive_ops(&mut ops, &mut Vec::new(), &world, &atlas, 1000, &binds);

        // S は2経路で展開＝s.png が 2 命令。祖先スタック（pop-on-exit）ゆえ非循環重複は刈られない。
        assert_eq!(ops.len(), 2, "非循環の共有子は各経路で展開（祖先スタック規律）");
        assert!(ops.iter().all(|op| op.element == s_id));
        let offsets: Vec<(i64, i64)> = ops.iter().map(|op| op.transform.offset()).collect();
        // id 昇順描画（既定 descend）: id=1（S 直下 at (10,0)）→ id=2（B→S at (0,20)）。
        assert_eq!(offsets, vec![(10, 0), (0, 20)], "各経路で別オフセット＝重複展開");
    }

    /// テスト5.3-⑤（要件 4.5/10.1）: 循環を含む入力でも 2 回導出→バイト等価（決定性・有界）。
    #[test]
    fn cyclic_derivation_is_deterministic() {
        let base = Path::new("shell/master");
        let atlas = bake_atlas(base, &["a.png", "b.png"]);

        let a = surface_with_anims(
            1000,
            vec![elem(0, "a.png", 0, 0)],
            vec![bind_anim(1, 1100, 0, 0)],
        );
        let b = surface_with_anims(
            1100,
            vec![elem(0, "b.png", 0, 0)],
            vec![bind_anim(1, 1000, 0, 0)], // B→A 相互参照。
        );
        let shell = shell_of(vec![a, b]);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));

        let binds = BindSet::from_ids([1]);
        let mut ops1 = Vec::new();
        derive_ops(&mut ops1, &mut Vec::new(), &world, &atlas, 1000, &binds);
        let mut ops2 = Vec::new();
        derive_ops(&mut ops2, &mut Vec::new(), &world, &atlas, 1000, &binds);

        assert_eq!(ops1, ops2, "循環入力でも同一入力→同一 ops（バイト等価・有界）");
    }

    // ── task 5.4: placement None スキップ＋静的キャンバス外形算出（有効 bind 非依存） ─────────
    //
    // 外形は `compute_extent` が全定義層（全 element ＋全 bind animation の pattern0＝有効 bind
    // 非依存）の「配置オフセット＋原寸」の和集合として算出する（要件 6.5・原点 (0,0) 固定・負方向
    // クリップ）。`original` は bake が「登録画像の原寸」をそのまま記録する（trim.rs）ため、各 element
    // の寄与サイズは register 時の (w,h) で制御できる。以下の土台ヘルパはサイズ可変・全透明を扱う。

    /// tightly-packed w×h の premultiplied BGRA・不透明（α=255）画像スペックを組む。
    ///
    /// bake は原寸（w,h）を `AtlasEntry.original` にそのまま記録するため、外形寄与サイズを
    /// 登録側で制御できる（trim は α>0 の bbox＝ここでは全面ゆえ placement は Some）。
    fn opaque_wxh(w: u32, h: u32) -> (u32, u32, u32, Vec<u8>, bool) {
        let stride = w * 4;
        // 全画素不透明（BGR は識別不要ゆえ定数・α=255）。
        let bgra = vec![64u8; (stride * h) as usize];
        (w, h, stride, bgra, true)
    }

    /// tightly-packed w×h の premultiplied BGRA・**全透明**（α=0）画像スペックを組む。
    ///
    /// 全 α==0 ゆえ trim は bbox を得られず `placement: None`（空エントリ）にするが、`original`
    /// は (w,h) を記録する（trim.rs「原寸は全透明でも記録」）。命令からはスキップされ（要件 6.3）、
    /// 外形へは (w,h) で寄与する（要件 6.5）ことを突く。
    fn transparent_wxh(w: u32, h: u32) -> (u32, u32, u32, Vec<u8>, bool) {
        let stride = w * 4;
        // premultiplied ゆえ α=0 なら BGR も 0（完全透明）。
        let bgra = vec![0u8; (stride * h) as usize];
        (w, h, stride, bgra, true)
    }

    /// (rel_path, (w,h), opaque?) 群から `AtlasTable` を bake する（サイズ・透過を個別指定）。
    ///
    /// `bake_atlas` は全 element 2×2・不透明固定だが、本ヘルパは外形テスト用に原寸と透過を制御する。
    fn bake_atlas_sized(base: &Path, specs: &[(&str, (u32, u32), bool)]) -> AtlasTable {
        let elements: Vec<Element> = specs.iter().map(|(r, _, _)| elem(0, r, 0, 0)).collect();
        let surfaces = vec![surface(0, elements)];
        let mut dec = MemoryDecoder::new();
        for (rel, (w, h), opaque) in specs {
            let (iw, ih, stride, bgra, has_alpha) = if *opaque {
                opaque_wxh(*w, *h)
            } else {
                transparent_wxh(*w, *h)
            };
            dec.insert(base.join(rel), iw, ih, stride, bgra, has_alpha);
        }
        let set = SurfaceSet {
            surfaces: &surfaces,
            base_dir: base,
            alpha_params: AlphaParams {
                use_self_alpha: UseSelfAlpha::On,
            },
        };
        let result = bake(&[set], &dec, PackConfig::default());
        assert!(result.errors.is_empty(), "bake セットアップは失敗しない");
        result.table
    }

    /// テスト5.4-①（**受入基準**・要件 6.5）: 同一 surface へ空／部分／全 BindSet を渡しても
    /// `compute_extent` が返す [`Extent`] は不変（有効 bind 集合に依存しない静的量）。
    ///
    /// host=1000 は静的 element base(40×30) を持ち、bind id=1→part1(200×10)、id=2→part2(10×150)、
    /// id=3→part3(60×60) を各 pattern0 で参照。外形は全 element＋**全 bind pattern0**の和集合ゆえ
    /// max(base.w, part1.w)=200・max(base.h, part2.h)=150＝Extent{200,150}。BindSet を空/部分/全に
    /// 変えても同一でなければ「有効 bind 依存」バグ（6.5 違反）。これが本 task の核心。
    #[test]
    fn extent_is_independent_of_bindset() {
        let base = Path::new("shell/master");
        let atlas = bake_atlas_sized(
            base,
            &[
                ("base.png", (40, 30), true),
                ("part1.png", (200, 10), true),
                ("part2.png", (10, 150), true),
                ("part3.png", (60, 60), true),
            ],
        );

        // host=1000: 静的 base ＋ bind id=1/2/3 が part1/part2/part3 を (0,0) 参照。
        let host = surface_with_anims(
            1000,
            vec![elem(0, "base.png", 0, 0)],
            vec![
                bind_anim(1, 1100, 0, 0),
                bind_anim(2, 1200, 0, 0),
                bind_anim(3, 1300, 0, 0),
            ],
        );
        let part1 = surface(1100, vec![elem(0, "part1.png", 0, 0)]);
        let part2 = surface(1200, vec![elem(0, "part2.png", 0, 0)]);
        let part3 = surface(1300, vec![elem(0, "part3.png", 0, 0)]);
        let shell = shell_of(vec![host, part1, part2, part3]);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));

        // 全 element＋全 bind pattern0 の和集合: w=max(40,200,10,60)=200・h=max(30,10,150,60)=150。
        let expected = Extent { w: 200, h: 150 };

        let empty = compute_extent(&mut Vec::new(), &world, &atlas, 1000);
        let partial = {
            let _b = BindSet::from_ids([2]);
            compute_extent(&mut Vec::new(), &world, &atlas, 1000)
        };
        let full = {
            let _b = BindSet::from_ids([1, 2, 3]);
            compute_extent(&mut Vec::new(), &world, &atlas, 1000)
        };

        // compute_extent は BindSet を引数に取らない（＝構造的に有効 bind 非依存）。三者一致＋期待値一致。
        assert_eq!(empty, expected, "空 BindSet 相当でも全 bind pattern0 を母集合に外形算出");
        assert_eq!(partial, expected, "部分 BindSet でも外形不変");
        assert_eq!(full, expected, "全 BindSet でも外形不変");
        assert_eq!(empty, partial);
        assert_eq!(partial, full);
    }

    /// テスト5.4-②（要件 6.5・全 bind 母集合）: 非活性 bind（どの BindSet にも入れない）の
    /// pattern0 が参照する巨大入れ子 surface が、それでも外形を支配する。
    ///
    /// host=2000 の静的 element は小さい tiny(4×4) のみ。bind id=9 は**一度も activate されない**が、
    /// その pattern0 が huge(500×400) を参照する。もし外形が「有効 bind のみ」を数えるバグなら tiny
    /// の 4×4 になるはずだが、正しくは全 bind pattern0 母集合ゆえ 500×400。空 BindSet でも huge が支配。
    #[test]
    fn extent_unions_inactive_bind_contribution() {
        let base = Path::new("shell/master");
        let atlas = bake_atlas_sized(
            base,
            &[("tiny.png", (4, 4), true), ("huge.png", (500, 400), true)],
        );

        let host = surface_with_anims(
            2000,
            vec![elem(0, "tiny.png", 0, 0)],
            vec![bind_anim(9, 2900, 0, 0)], // id=9 は以降どの BindSet にも入れない。
        );
        let huge = surface(2900, vec![elem(0, "huge.png", 0, 0)]);
        let shell = shell_of(vec![host, huge]);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));

        // 有効 bind 非依存ゆえ、activate されない id=9 の huge も外形へ寄与＝500×400 が支配。
        let extent = compute_extent(&mut Vec::new(), &world, &atlas, 2000);
        assert_eq!(
            extent,
            Extent { w: 500, h: 400 },
            "非活性 bind の入れ子巨大 surface も外形を支配（全 bind 母集合・6.5）"
        );
    }

    /// テスト5.4-③（要件 6.3＋6.5）: placement None（全透明）element は命令からスキップされるが、
    /// 外形には原寸で寄与する。
    ///
    /// surface=3000 は静的 element を 2 本持つ: solid(20×20・不透明) と ghost(300×300・**全透明**＝
    /// placement None)。`derive_ops` は ghost をスキップし solid の 1 命令のみ（要件 6.3）。一方
    /// `compute_extent` は ghost の原寸 300×300 を数えて 300×300（要件 6.5・全透明でも原寸寄与）。
    #[test]
    fn placement_none_skipped_from_ops_but_counted_in_extent() {
        let base = Path::new("shell/master");
        let atlas = bake_atlas_sized(
            base,
            &[
                ("solid.png", (20, 20), true),
                ("ghost.png", (300, 300), false), // 全透明→ placement None・original は 300×300。
            ],
        );
        let solid_id = atlas.resolve(SetId(0), "solid.png").expect("solid 解決");
        let ghost_id = atlas.resolve(SetId(0), "ghost.png").expect("ghost 解決");

        // 全透明 ghost は bake で placement None・原寸は保持（前提の実証）。
        assert!(
            atlas.entry(ghost_id).placement.is_none(),
            "全透明 element は bake で placement None（前提）"
        );
        assert_eq!(atlas.entry(ghost_id).original, areka_emo_atlas::Size { w: 300, h: 300 });
        assert!(atlas.entry(solid_id).placement.is_some(), "不透明 element は placement Some");

        let surf = surface(3000, vec![elem(0, "solid.png", 0, 0), elem(1, "ghost.png", 0, 0)]);
        let shell = shell_of(vec![surf]);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));

        // 命令: ghost はスキップされ solid の 1 本のみ（要件 6.3）。
        let binds = BindSet::default();
        let mut ops = Vec::new();
        derive_ops(&mut ops, &mut Vec::new(), &world, &atlas, 3000, &binds);
        assert_eq!(ops.len(), 1, "placement None（ghost）は命令からスキップ（要件 6.3）");
        assert_eq!(ops[0].element, solid_id, "残る命令は不透明 solid のみ");

        // 外形: ghost の原寸 300×300 を数える（要件 6.5・全透明でも寄与）。
        let extent = compute_extent(&mut Vec::new(), &world, &atlas, 3000);
        assert_eq!(
            extent,
            Extent { w: 300, h: 300 },
            "placement None element も原寸で外形へ寄与（要件 6.5）"
        );
    }

    /// テスト5.4-④（要件 6.5・原点固定／負方向クリップ）: 負オフセットの入れ子層は原点 (0,0) を
    /// 上へ動かさず、外形を基底より縮めない。
    ///
    /// host=4000 の静的 base(50×40)。bind id=1 の pattern0 が small(10×10) を **(-100,-100)** で参照。
    /// 負オフセット層の寄与は max(0, -100+10)=0 ゆえ外形へ効かず、原点は (0,0) のまま・外形は base の
    /// 50×40 を下回らない（負方向はみ出しは転写時クリップ・議題2裁定 (A)）。
    #[test]
    fn negative_offset_is_clipped_and_origin_stays_fixed() {
        let base = Path::new("shell/master");
        let atlas = bake_atlas_sized(
            base,
            &[("base.png", (50, 40), true), ("small.png", (10, 10), true)],
        );

        let host = surface_with_anims(
            4000,
            vec![elem(0, "base.png", 0, 0)],
            vec![bind_anim(1, 4100, -100, -100)], // 負オフセット参照。
        );
        let small = surface(4100, vec![elem(0, "small.png", 0, 0)]);
        let shell = shell_of(vec![host, small]);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));

        // small は (-100+10, -100+10)=(-90,-90) ゆえ max(0,·)=0 で寄与せず、base の 50×40 が残る。
        let extent = compute_extent(&mut Vec::new(), &world, &atlas, 4000);
        assert_eq!(
            extent,
            Extent { w: 50, h: 40 },
            "負オフセット層は原点クリップ＝外形を縮めない（要件 6.5・原点 (0,0) 固定）"
        );
    }

    /// テスト5.4-⑤（要件 6.5・基底一致）: (0,0) 原寸 (W,H) の単一 element・より大きい層なし→
    /// 外形は原寸ちょうど。
    #[test]
    fn extent_equals_base_original_when_no_larger_layers() {
        let base = Path::new("shell/master");
        let atlas = bake_atlas_sized(base, &[("only.png", (123, 45), true)]);

        let surf = surface(5000, vec![elem(0, "only.png", 0, 0)]);
        let shell = shell_of(vec![surf]);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));

        let extent = compute_extent(&mut Vec::new(), &world, &atlas, 5000);
        assert_eq!(extent, Extent { w: 123, h: 45 }, "単一 element・(0,0)→外形＝原寸");
    }

    /// テスト5.4-⑥（要件 10.1・決定性）: 同一入力で 2 回算出→ [`Extent`] が同値。
    #[test]
    fn extent_is_deterministic() {
        let base = Path::new("shell/master");
        let atlas = bake_atlas_sized(
            base,
            &[
                ("base.png", (40, 30), true),
                ("part.png", (200, 150), true),
            ],
        );
        let host = surface_with_anims(
            6000,
            vec![elem(0, "base.png", 0, 0)],
            vec![bind_anim(1, 6100, 5, 7)],
        );
        let part = surface(6100, vec![elem(0, "part.png", 0, 0)]);
        let shell = shell_of(vec![host, part]);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));

        let e1 = compute_extent(&mut Vec::new(), &world, &atlas, 6000);
        let e2 = compute_extent(&mut Vec::new(), &world, &atlas, 6000);
        assert_eq!(e1, e2, "同一入力→同一 Extent（決定的）");
        // 参考: part は (5,7) 参照ゆえ w=max(40, 5+200)=205・h=max(30, 7+150)=157。
        assert_eq!(e1, Extent { w: 205, h: 157 });
    }

    // ── task 5.5: 命令ゼロ時の 3 分類（正常空合成／対象不在／退化データ）─────────────────
    //
    // build_plan は derive_ops（有効 bind 依存の命令列）＋ compute_extent（有効 bind 非依存の
    // 静的外形）を wrap し、design「Error Handling」表の 3 状態を厳密に区別する:
    //   - 対象 surface 不在        → Err(SurfaceNotFound)（error ログ・要件 10.5）。
    //   - surface 存在・命令ゼロでも → Ok(非ゼロ Extent)＋空 ops（正常全透明・要件 6.6・議題2裁定）。
    //   - 定義層皆無で外形 0×0      → Err(EmptyComposition)（error ログ・要件 10.5・唯一の真の失敗退化）。
    // 非パニック（要件 1.4）。out_ops はエントリで clear する再利用スクラッチ（要件 10.3）。

    use crate::error::ComposeError;

    /// テスト5.5-①（要件 10.5）: 対象 surface 不在 → `Err(SurfaceNotFound)`・ops 空・非パニック。
    #[test]
    fn build_plan_absent_surface_is_surface_not_found() {
        // surface を一切持たない World（9999 は不在）。
        let world = EmoWorld::build(&shell_of(Vec::new()));
        let atlas = bake_atlas(Path::new("shell/master"), &["dummy.png"]);

        let binds = BindSet::default();
        let mut ops = Vec::new();
        let result = build_plan(&mut ops, &mut Vec::new(), &world, &atlas, 9999, &binds);

        assert_eq!(
            result,
            Err(ComposeError::SurfaceNotFound(9999)),
            "不在 surface は SurfaceNotFound（要件 10.5）"
        );
        assert!(ops.is_empty(), "不在 surface では命令を積まない");
    }

    /// テスト5.5-②（**受入基準**・要件 6.6）: 全 element が全透明の surface → `Ok(非ゼロ Extent)`＋空 ops。
    ///
    /// surface=3000 は element を 2 本持つがいずれも全透明（α=0 → placement None）。描画可能命令は
    /// ゼロだが、外形は原寸（300×300 が支配）で非ゼロ。これはエラーでなく **正常系**（議題2裁定）。
    #[test]
    fn build_plan_all_transparent_is_ok_empty_ops_nonzero_extent() {
        let base = Path::new("shell/master");
        let atlas = bake_atlas_sized(
            base,
            &[
                ("ghost1.png", (300, 300), false), // 全透明→ placement None。
                ("ghost2.png", (20, 20), false),   // 全透明→ placement None。
            ],
        );

        let surf = surface(3000, vec![elem(0, "ghost1.png", 0, 0), elem(1, "ghost2.png", 0, 0)]);
        let shell = shell_of(vec![surf]);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));

        let binds = BindSet::default();
        let mut ops = Vec::new();
        let result = build_plan(&mut ops, &mut Vec::new(), &world, &atlas, 3000, &binds);

        // 描画可能命令ゼロでも Err にしない（要件 6.6・議題2裁定）。
        let extent = result.expect("全透明でもエラーにせず Ok（要件 6.6）");
        assert!(ops.is_empty(), "全 element 全透明 → 空 ops（描画可能命令ゼロ）");
        // 外形は原寸で非ゼロ（全透明でも original で寄与＝300×300 が支配）。
        assert_eq!(extent, Extent { w: 300, h: 300 }, "非ゼロの静的外形を返す（要件 6.6）");
        assert_ne!(extent.w, 0);
        assert_ne!(extent.h, 0);
    }

    /// テスト5.5-③（要件 6.6）: bind のみ surface＋空 BindSet → `Ok(非ゼロ Extent)`＋空 ops。
    ///
    /// host=1000 は静的 element なし・全パーツ bind。空 BindSet ゆえ有効 bind ゼロ＝命令ゼロだが、
    /// 外形は全 bind pattern0 母集合（有効 bind 非依存）ゆえ非ゼロ。エラーでなく正常空合成。
    #[test]
    fn build_plan_empty_bindset_bind_only_is_ok_empty_ops_nonzero_extent() {
        let base = Path::new("shell/master");
        let atlas = bake_atlas_sized(
            base,
            &[("part1.png", (200, 10), true), ("part2.png", (10, 150), true)],
        );

        // 静的 element なし・bind id=1/2 が part1/part2 を参照。
        let host = surface_with_anims(
            1000,
            Vec::new(),
            vec![bind_anim(1, 1100, 0, 0), bind_anim(2, 1200, 0, 0)],
        );
        let part1 = surface(1100, vec![elem(0, "part1.png", 0, 0)]);
        let part2 = surface(1200, vec![elem(0, "part2.png", 0, 0)]);
        let shell = shell_of(vec![host, part1, part2]);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));

        // 空 BindSet → 有効 bind ゼロ＝描画可能命令ゼロ。
        let binds = BindSet::default();
        let mut ops = Vec::new();
        let result = build_plan(&mut ops, &mut Vec::new(), &world, &atlas, 1000, &binds);

        let extent = result.expect("空 BindSet でも正常（描画可能命令ゼロは失敗でない・要件 6.6）");
        assert!(ops.is_empty(), "空 BindSet → bind 命令ゼロ・静的 element も無し → 空 ops");
        // 外形は全 bind pattern0 の和集合（有効 bind 非依存）: w=max(200,10)=200・h=max(10,150)=150。
        assert_eq!(extent, Extent { w: 200, h: 150 }, "有効 bind 非依存の非ゼロ外形");
    }

    /// テスト5.5-④（要件 10.5・議題2裁定）: 定義層皆無で外形 0×0 → `Err(EmptyComposition)`。
    ///
    /// surface=7000 は EXISTS するが element ゼロ・bind ゼロ（外形へ寄与する層が皆無）。
    /// compute_extent が {0,0} を返す唯一の真の失敗退化ケース。
    #[test]
    fn build_plan_no_layers_degenerate_is_empty_composition() {
        // element ゼロ・animation ゼロの surface（存在はするが定義層が皆無）。
        let surf = surface(7000, Vec::new());
        let shell = shell_of(vec![surf]);
        let mut world = EmoWorld::build(&shell);
        // atlas は引かれない（element がない）が bind_atlas は呼ぶ（binding 挿入）。
        let atlas = bake_atlas(Path::new("shell/master"), &["dummy.png"]);
        world.bind_atlas(&atlas, SetId(0));

        // 前提の実証: 外形が 0×0（定義層皆無）。
        assert_eq!(
            compute_extent(&mut Vec::new(), &world, &atlas, 7000),
            Extent { w: 0, h: 0 },
            "定義層皆無 → 外形 0×0（前提）"
        );

        let binds = BindSet::default();
        let mut ops = Vec::new();
        let result = build_plan(&mut ops, &mut Vec::new(), &world, &atlas, 7000, &binds);

        assert_eq!(
            result,
            Err(ComposeError::EmptyComposition(7000)),
            "定義層皆無で 0×0 の退化のみ EmptyComposition（要件 10.5・議題2裁定）"
        );
        assert!(ops.is_empty());
    }

    /// テスト5.5-⑤（sanity・要件 6.6 対比）: 可視 element を持つ通常 surface → `Ok`＋非空 ops。
    #[test]
    fn build_plan_populated_surface_is_ok_with_nonempty_ops() {
        let base = Path::new("shell/master");
        let atlas = bake_atlas_sized(base, &[("visible.png", (80, 60), true)]);
        let visible_id = atlas.resolve(SetId(0), "visible.png").expect("visible 解決");

        let surf = surface(5000, vec![elem(0, "visible.png", 0, 0)]);
        let shell = shell_of(vec![surf]);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));

        let binds = BindSet::default();
        let mut ops = Vec::new();
        let result = build_plan(&mut ops, &mut Vec::new(), &world, &atlas, 5000, &binds);

        let extent = result.expect("通常 surface は Ok");
        assert_eq!(ops.len(), 1, "可視 element 1 本 → 命令 1 本");
        assert_eq!(ops[0].element, visible_id);
        assert_eq!(extent, Extent { w: 80, h: 60 }, "外形＝element 原寸");
    }

    /// テスト5.5-⑥（要件 10.3・10.1）: out_ops はエントリで clear される（スクラッチ再利用）・決定的。
    #[test]
    fn build_plan_clears_scratch_and_is_deterministic() {
        let base = Path::new("shell/master");
        let atlas = bake_atlas_sized(base, &[("visible.png", (80, 60), true)]);
        let visible_id = atlas.resolve(SetId(0), "visible.png").expect("visible 解決");

        let surf = surface(5000, vec![elem(0, "visible.png", 0, 0)]);
        let shell = shell_of(vec![surf]);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));

        let binds = BindSet::default();

        // 事前にゴミを詰めた out_ops → build_plan がエントリで clear する（ゴミは消える）。
        let junk = BlitOp {
            element: ElementId(u32::MAX),
            transform: Transform::identity(),
            method: ComposeMethod::Overlay,
        };
        let mut ops = vec![junk.clone(), junk.clone(), junk];
        let e1 = build_plan(&mut ops, &mut Vec::new(), &world, &atlas, 5000, &binds).expect("Ok");

        assert_eq!(ops.len(), 1, "エントリで clear ＝ この surface の命令のみが残る");
        assert_eq!(ops[0].element, visible_id, "ゴミは残らない");

        // 2 回目（別スクラッチ）→ バイト等価・同一 Extent（決定性）。
        let mut ops2 = Vec::new();
        let e2 = build_plan(&mut ops2, &mut Vec::new(), &world, &atlas, 5000, &binds).expect("Ok");
        assert_eq!(ops, ops2, "同一入力→同一 ops（バイト等価）");
        assert_eq!(e1, e2, "同一入力→同一 Extent");
    }
}
