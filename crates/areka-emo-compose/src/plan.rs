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
use crate::pattern::PatternState;
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
    pattern: &PatternState,
) {
    // visited 祖先スタック（design: Composer スクラッチの Vec<u32>）は呼び手（[`build_plan`]／
    // Composer ファサード）から借用して再利用する（要件 10.3・定常状態アロケーションなし）。
    // 走査開始前に空へ戻し、祖先スタック規律（push-on-enter／pop-on-exit）で走査後も空に戻す。
    visited.clear();
    // top-level 合成対象。ここでのみ PatternState の現在コマを層(ii) へ合流させる（コマは表示中
    // surface のアニメに属す・design「top-level surface のみ」）。入れ子再帰は is_top_level=false。
    flatten_surface(
        out_ops, visited, world, atlas, surface_id, binds, pattern, 0, 0, true,
    );
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
/// # 合成合流と method ゲート（task 7.2・要件 4.2/4.6/5.3/8.4）
///
/// `is_top_level`（derive_ops 直下の合成対象 surface のみ真）のとき、層(ii) の対象 id 集合へ
/// `pattern`（[`PatternState`]）の現在コマを持つ id を合流する。合流対象 id 集合 =
/// `{ 有効 bind pattern0 を持つ id } ∪ { PatternState に現在コマを持つ id のうち bind 種でないもの、
/// または bind 種でも現在の bind 集合に属するもの }`（bind 所属条件は bindopt D9-3・下記）。整列は既存の
/// animation-sort 2 段規則を**変更せず**適用する（R5.3・画家のアルゴリズム）。各 id で現在コマが
/// あれば**コマが pattern0 静的寄与を置換**する（4.2「各コマは直前コマをリセットしてベースへ」）。
/// コマ・pattern0 いずれも method ゲート（[`ComposeMethod::is_implemented`]＝Overlay のみ）を通し、
/// 非 Overlay は `warn!`（method 名込み）＋不描画（完全形保持のまま非駆動・8.4）。**再帰段（入れ子
/// 参照）は `is_top_level=false` で PatternState を参照しない**（コマは表示中 surface のアニメに属す）。
///
/// # bind 非所属コマの合流拒否（bindopt D9-3・最後の砦・bindopt 7.1/7.4）
///
/// 上記の合流で、**当 surface の bind 種アニメ**（[`is_bind_animation`]）に属する id のコマは
/// `binds.contains(id)` のときだけ合流する。`-1` 終端を持たないアニメは末尾到達後に最終コマを
/// 保持する仕様ゆえ、着せ替え ID が bind 集合から外れても保持コマが残り得る。その掃除は状態側
/// （発行前除去）と再生側（進行相の停止）が担うが、いずれかが漏れても**外れたパーツが表示に
/// 残らない**不変量を合成計画側でも保証する（実機の表情固着の直接の機構）。**bind に属さない
/// アニメ（純 `random` の `interval` 等）のコマは無条件で従来どおり合流する**（bindopt 7.4）。
/// 合成順・重ね順（animation-sort 2 段規則）は本ゲートで一切変わらない。
///
/// 引数は out_ops/visited のスクラッチ2本＋world/atlas の入力2本＋surface_id＋binds＋pattern＋累積
/// offset(x,y)＋is_top_level。全て再帰の各段で必要ゆえ削れない（スクラッチ構造体化は将来の整理余地）。
#[allow(clippy::too_many_arguments)]
fn flatten_surface(
    out_ops: &mut Vec<BlitOp>,
    visited: &mut Vec<u32>,
    world: &EmoWorld,
    atlas: &AtlasTable,
    surface_id: u32,
    binds: &BindSet,
    // task 7.2: 現在コマ集合の層(ii) 合流＋コマ/pattern0 双方への method ゲートで実消費する。
    // 合流は is_top_level のみ（コマは表示中 surface のアニメに属す）。再帰段は pattern を参照しない。
    pattern: &PatternState,
    offset_x: i64,
    offset_y: i64,
    is_top_level: bool,
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
        // 層（ii）: 合流対象 id 集合 = { 有効 bind（interval が bind 種 ∧ id ∈ binds）の pattern0 を
        //   持つ id } ∪ { PatternState に現在コマを持つ id }。後者は **is_top_level のみ**合流する
        //   （コマは表示中 surface のアニメに属す・design「top-level surface のみ」・要件 4.6/5.3）。
        let mut merged_ids: Vec<u32> = master
            .animations
            .iter()
            .filter(|a| is_bind_interval(&a.interval) && binds.contains(a.id))
            .map(|a| a.id)
            .collect();
        if is_top_level {
            // 現在コマの id を合流（有効 bind pattern0 を持たない id も含む）。重複は既存を優先し
            // 二重列挙しない（整列後も 1 回だけ処理される）。
            for (id, _frame) in pattern.iter() {
                if merged_ids.contains(&id) {
                    continue;
                }
                // bindopt D9-3（最後の砦・bindopt 7.1）: **bind 種**のアニメのコマは、現在の bind
                // 集合に属するときだけ合流する。bind から外れた ID の保持コマ（`-1` 終端を持たない
                // アニメが末尾到達後に保つ最終コマ）が、上流の掃除（状態側の発行前除去・再生側の
                // 進行相停止）を漏れても表示へ届かない不変量をここで成立させる。**bind に属さない
                // アニメ（純 `random` の `interval` 等・当 surface に定義の無い id）は無条件で従来
                // どおり合流する**（bindopt 7.4）。合成順・重ね順の規則は不変（合流の可否のみ）。
                if is_bind_animation(master, id) && !binds.contains(id) {
                    tracing::debug!(
                        target: "areka_emo_compose",
                        surface_id,
                        animation_id = id,
                        "bind 集合に属さない bind 種アニメの保持コマ: 合成計画へ合流しない（bindopt 7.1・D9-3）"
                    );
                    continue;
                }
                merged_ids.push(id);
            }
        }

        // 段1/段2: animation-sort の2段規則を**変更せず**合流後 id 集合へ適用する（design 決定5・
        //   画家のアルゴリズム写像・R5.3）。Descend（既定）→ id 昇順に描画（大 id が上）。
        //   Ascend → id 降順に描画（小 id が上）。
        match world.animation_sort() {
            SortOrder::Descend => merged_ids.sort_unstable(),
            SortOrder::Ascend => merged_ids.sort_unstable_by(|a, b| b.cmp(a)),
            // `SortOrder` は `#[non_exhaustive]`。未知値は既定 Descend（id 昇順描画）へ倒す（非パニック）。
            other => {
                tracing::warn!(
                    target: "areka_emo_compose",
                    surface_id,
                    sort = ?other,
                    "未知の animation-sort: 既定 Descend（id 昇順描画）として扱う"
                );
                merged_ids.sort_unstable();
            }
        }

        // 描画順に各 id の寄与（コマ優先・無ければ pattern0）を静的層の後（＝上）へ積む。
        for id in merged_ids {
            // top-level かつ当 id に現在コマがあれば **コマが pattern0 静的寄与を置換**する
            //   （4.2「各コマは直前コマをリセットしてベースへ」）。コマは method ゲートを通す。
            if is_top_level {
                if let Some(frame) = pattern.get(id) {
                    if frame.method.is_implemented() {
                        // Overlay: pattern0 と同様、frame.surface_id へ (x,y) 累積で再帰 flatten
                        //   （transient コマも入れ子参照を許す・循環検出は再帰入口の visited 判定）。
                        flatten_surface(
                            out_ops,
                            visited,
                            world,
                            atlas,
                            frame.surface_id,
                            binds,
                            pattern,
                            offset_x + frame.x,
                            offset_y + frame.y,
                            false,
                        );
                    } else {
                        // 非 Overlay: 完全形は保持しつつ非駆動＝当該コマ不描画（warn・要件 8.4）。
                        tracing::warn!(
                            target: "areka_emo_compose",
                            surface_id,
                            animation_id = id,
                            method = ?frame.method,
                            "非 Overlay method の現在コマ: 不描画 skip（完全形保持・非駆動・要件 8.4）"
                        );
                    }
                    // コマが pattern0 を置換したゆえ、この id の pattern0 静的経路は辿らない。
                    continue;
                }
            }

            // コマ無し（または非 top-level）: 従来の有効 bind pattern0 静的経路。
            // 同 id の animation は fold 段で単一化済み（後勝ち）ゆえ find で足りる。
            let Some(anim) = master.animations.iter().find(|a| a.id == id) else {
                continue;
            };
            // pattern0＝厳密に index==0 の pattern のみ（要件 9.2・D12）。疎 index の最小値へフォールバック
            // しない: pattern0 を持たない bind animation（まばたき等の `interval,bind+random` 再生アニメ・
            // pattern1 以降のみ）は静的土台を持たず、それらのフレームは seriko-loop（M-life）が再生する
            // （要件 9.1）。ここで pattern1 を土台に採ると閉じ目フレームがベース目を覆う（実機第2欠陥）。
            let Some(pattern0) = anim.patterns.iter().find(|p| p.index == 0) else {
                tracing::debug!(
                    target: "areka_emo_compose",
                    surface_id,
                    animation_id = id,
                    "有効 bind に pattern0（index==0）が無い: 静的合成へ寄与なし・skip（再生は seriko-loop 領分・要件 9.1）"
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

            // pattern0 静的経路にも同じ method ゲート（D-5 是正）: parser の overlay フィルタ撤去
            //   （task 1.2）で非 overlay pattern0 がモデルへ流入し得るため、原文 method を解決して
            //   Overlay のみ駆動する。非 Overlay は warn!（method 名込み）＋不描画（8.4）。emo2 は
            //   全 overlay ゆえ golden byte 不変。
            let p0_method = ComposeMethod::from_name(pattern0.method.as_str());
            if !p0_method.is_implemented() {
                tracing::warn!(
                    target: "areka_emo_compose",
                    surface_id,
                    animation_id = id,
                    method = ?p0_method,
                    "非 Overlay method の bind pattern0: 不描画 skip（完全形保持・非駆動・要件 8.4）"
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
                pattern,
                offset_x + pattern0.x,
                offset_y + pattern0.y,
                false,
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
    flatten_extent(
        &mut max_x, &mut max_y, visited, world, atlas, surface_id, 0, 0,
    );
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
    pattern: &PatternState,
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
    // 入口で clear され、外形走査と共有される（要件 10.3）。pattern は derive_ops へ透過する
    // （現在コマ上乗せの実消費は task 7.2・外形 compute_extent は pattern 非依存の静的量）。
    derive_ops(out_ops, visited, world, atlas, surface_id, binds, pattern);

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
    debug_assert_eq!(
        popped,
        Some(surface_id),
        "外形算出: visited は祖先スタック（LIFO）"
    );
}

/// interval が bind 種（`Interval::Bind`/`Interval::BindRandom`）か（要件 5.2・design 層列挙）。
///
/// `Interval::Random`（純ランダム・非 bind）は有効 bind の対象にしない。`Interval` は
/// `#[non_exhaustive]` ゆえ未知 variant は bind でないものとして扱う（非パニック）。
fn is_bind_interval(interval: &Interval) -> bool {
    matches!(interval, Interval::Bind | Interval::BindRandom { .. })
}

/// 指定 animation id が当 surface の **bind 種**アニメか（bindopt D9-3・bindopt 7.1/7.4）。
///
/// [`flatten_surface`] の層(ii) 合流条件が「bind 種のコマは bind 集合に属することを要求する」ため
/// の判定。当 surface の `animations` から id を引き、interval が bind 種（[`is_bind_interval`]）の
/// ものが 1 本でもあれば真。**当 surface に定義の無い id・bind 種でない id（純 `random` の
/// `interval` 等）は偽**で、追加ゲートを受けず従来どおり無条件に合流する（bindopt 7.4）。
///
/// 判定は `emo-compose` が既に持つ [`Interval`]（転記層由来）だけで閉じており、bind 種の語彙を
/// 求めて seriko へ依存を逆流させない（design Allowed Dependencies「parsers → areka → seriko →
/// emo-compose・逆流禁止」）。同 id の animation は fold 段で単一化済みゆえ `any` で足りる。
fn is_bind_animation(master: &SurfaceMaster, animation_id: u32) -> bool {
    master
        .animations
        .iter()
        .any(|a| a.id == animation_id && is_bind_interval(&a.interval))
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
#[path = "plan_test_support.rs"]
mod test_support;

#[cfg(test)]
#[path = "plan_ops_tests.rs"]
mod ops_tests;

#[cfg(test)]
#[path = "plan_extent_tests.rs"]
mod extent_tests;
