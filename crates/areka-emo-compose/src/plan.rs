//! `BlitOp` / `PlanBuilder`: 正規化定義からバックエンド非依存の合成命令列を導出する。
//!
//! element レイヤ順・有効 bind 集合の `animation-sort`→animation ID 順の2段合成規則を確定し、
//! 各命令へ `AtlasTable` 解決結果（`ElementId`・`Placement`）と変換行列を含める。入れ子 surface
//! 参照はオフセット累積で inline 展開（flatten・明示のスタック）し、祖先の列で循環を検出する。
//! キャンバス外形を算出し、同一入力に対して決定的な命令列を生成する。
//!
//! # 本 module の到達点（task 5.1／5.2／5.3）とシーム
//!
//! - **task 5.1**: 静的 element 層の列挙（[`push_static_element_ops`]・layer 昇順）。
//! - **task 5.2**: 有効 bind（`BindSet`）pattern0 層の合成対象化＋`animation-sort`→
//!   animation ID 順の 2 段規則（[`derive_ops`]）。静的層の後（上）へ、descend（既定）なら ID 昇順・
//!   ascend なら ID 降順で bind pattern0 を積む（design 決定5・画家のアルゴリズム）。全パーツが
//!   bind な surface でも非空 bind 集合から可視層を生成する。
//! - **task 5.3**: 入れ子 surface 参照の**多段 flatten＋循環検出**
//!   （[`flatten_surface`]）。pattern0 の入れ子参照を pattern の (x,y) をオフセット累積して
//!   inline 展開し、visited 祖先スタックで自己参照・相互参照の循環を検出して非パニックで
//!   枝を打ち切る（部分結果＋warn・要件 7.1/7.2/7.3）。再帰でなく作業場（[`PlanScratch`]）の明示の
//!   スタックで回し、合成 1 回の広がりを予算で打ち切る（audit-emo-compose 指摘 3）。
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
//! - **task 7（本 task）**: 走査の作業場（[`PlanScratch`]）を引数化する（[`build_plan`]/[`derive_ops`] が
//!   呼び手＝Composer の作業場を借用・各走査入口で空へ戻す）。これにより毎フレーム経路（`compose_into`）
//!   で再割り当てが起きず、O(elements) の定常状態アロケーションなしを満たす（要件 10.3）。

use std::collections::HashSet;

use areka_emo_atlas::{AtlasTable, ElementId};
use areka_parsers::shell::{Interval, SortOrder};
use bevy_ecs::entity::Entity;

use crate::bind::BindSet;
use crate::budget::{Budget, VISIT_BUDGET};
use crate::error::ComposeError;
use crate::method::ComposeMethod;
use crate::nesting::{ElementKind, PartKey, is_always_interval};
use crate::normalized::{SurfaceMaster, Transform};
use crate::pattern::{PatternFrame, PatternState};
use crate::plan_always::{always_rest_target, film_op};
use crate::world::{AtlasBinding, EmoWorld, SurfaceIndex};

#[path = "plan_extent.rs"]
mod extent;
#[cfg(test)]
pub(crate) use extent::compute_extent;
pub(crate) use extent::{ExtentMemo, MAX_EXTENT, compute_extent_limited};

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

/// 命令列の走査の作業場（指摘 3・8⑷）。合成器が 1 つ持ち、走査のたびに中身を空へ戻して使い回す
/// （定常状態で確保しない・要件 10.3）。
///
/// 段（1 つの面を 1 つの位置に置く単位）の積み荷は `steps` の 1 本の列に足し、段が尽きたら切り詰める。
/// 層 (ii) の id・挿入の数えの列が 1 本ずつで足りるのは、1 つの段の積み荷を全部作り終えてから子の段へ
/// 入るため（段を作る途中で別の段を作らない）。
#[derive(Debug)]
pub(crate) struct PlanScratch {
    /// 祖先の列（今たどっている面の経路・段を出るときに外す順）。
    pub(crate) visited: Vec<u32>,
    /// 経路にある面の集合（循環の確かめを O(1) にする・`visited` と同じ面を持つ）。外形の走査も借りる。
    pub(crate) on_path: HashSet<u32>,
    /// 段の積み荷を足していく 1 本の列。
    pub(crate) steps: Vec<Step>,
    /// 段ごとの印（積み荷の始まりと読み位置）。
    frames: Vec<FrameMark>,
    /// 層 (ii) の描く順の id。
    pub(crate) merged_ids: Vec<u32>,
    /// 段の中の差し込みの決め（(B, A)＝B を A が差し込む）。
    pub(crate) claims: Vec<(u32, u32)>,
    /// 段の中で差し込み済みの B。
    pub(crate) done: Vec<u32>,
    /// 差し込みの数えで手順をたどり済みの着せ替え。
    pub(crate) expanded: Vec<u32>,
    /// 段の中の着せ替えの手順をたどる小さなスタック（(着せ替え, 手順の読み位置)）。差し込みの数えと
    /// 寄与の積みが順に使う（どちらも終わると空に戻る）。
    pub(crate) chain: Vec<(u32, usize)>,
    /// 合成 1 回の広がりの予算（本番は [`VISIT_BUDGET`]）。
    limit: usize,
}

impl Default for PlanScratch {
    fn default() -> Self {
        Self {
            visited: Vec::new(),
            on_path: HashSet::new(),
            steps: Vec::new(),
            frames: Vec::new(),
            merged_ids: Vec::new(),
            claims: Vec::new(),
            done: Vec::new(),
            expanded: Vec::new(),
            chain: Vec::new(),
            limit: VISIT_BUDGET,
        }
    }
}

impl PlanScratch {
    /// 小さな予算を渡す内側の口（テスト用）。
    #[cfg(test)]
    pub(crate) fn with_limit(limit: usize) -> Self {
        Self {
            limit,
            ..Self::default()
        }
    }

    /// 面の段を開く: 祖先に積み、段の積み荷（層 (i) → 層 (ii)）を作る。
    fn open(&mut self, level: &Level<'_>) {
        self.visited.push(level.surface_id);
        self.on_path.insert(level.surface_id);
        let start = self.steps.len();
        self.frames.push(FrameMark { start, read: start });
        push_static_element_ops(self, level);
        push_bind_layer(self, level);
    }
}

/// 段の積み荷。
#[derive(Debug, Clone)]
pub(crate) enum Step {
    /// 命令をそのまま積む。
    Op(BlitOp),
    /// 部品の段（`is_top_level=false`）へ、面 `surface` を位置 (x, y) に置いて入る。
    Enter { surface: u32, x: i64, y: i64 },
}

/// 段の印: 積み荷の始まりと、次に読む位置。
#[derive(Debug)]
struct FrameMark {
    start: usize,
    read: usize,
}

/// 走査 1 回の入力（どの段でも同じ）。
#[derive(Clone, Copy)]
struct Walk<'a> {
    world: &'a EmoWorld,
    atlas: &'a AtlasTable,
    binds: &'a BindSet,
    pattern: &'a PatternState,
}

impl<'a> Walk<'a> {
    /// 面 `surface_id` を (x, y) に置く段。面か束縛が無ければ `None`（何も積まない）。
    fn level(self, surface_id: u32, x: i64, y: i64, is_top_level: bool) -> Option<Level<'a>> {
        let (master, binding) = surface_and_binding(self.world, surface_id)?;
        Some(Level {
            world: self.world,
            atlas: self.atlas,
            master,
            binding,
            surface_id,
            binds: self.binds,
            pattern: self.pattern,
            offset_x: x,
            offset_y: y,
            is_top_level,
        })
    }
}

/// 層 (i): 段の面の静的 element を、layer 昇順（同 layer は登場順）で積み荷にする（要件 4.1/4.5）。
///
/// 束縛が `Some(ElementId)` の画像だけを命令にし、`None`（画像の解決に失敗・bind 時に debug 済み）と
/// `placement` が None（全透明・要件 6.3）は飛ばす。位置は段の位置＋ element の X,Y。
///
/// # element定義の子（surface-element-nesting 要件 2.1〜2.3・3.1〜3.4）
///
/// 欄がサーフェスの番号（[`ElementKind::Surface`]）の element定義は、同じ順の位置に子の段へ入る
/// 積み荷を置く（位置は段の位置＋ element定義の X,Y）。子が先祖（`visited`）に在る・面の表に無い・
/// 範囲を超える数のときは、その element定義だけを飛ばして `debug!` を 1 行出す（警告は読み込みの
/// ときに出ている）。動く絵の子は今のコマ 1 枚の命令を置く。
fn push_static_element_ops(s: &mut PlanScratch, level: &Level<'_>) {
    let (master, surface_id) = (level.master, level.surface_id);
    // layer 昇順・同 layer は登場順（安定ソート）。fold 段で既に整列済み（不変条件）だが、
    // plan は自 module で決定性を保証するため添字と layer を安定ソートし直す。
    let mut order: Vec<usize> = (0..master.elements.len()).collect();
    order.sort_by_key(|&i| master.elements[i].layer);

    for i in order {
        let element = &master.elements[i];
        let (ex, ey) = element.transform.offset();
        let (x, y) = (level.offset_x + ex, level.offset_y + ey);
        let skip_reason = match element.kind {
            ElementKind::Image => None,
            ElementKind::SurfaceOutOfRange => Some("番号として扱える範囲を超える数"),
            // 動く絵の子: 同じ順の位置で今のコマ 1 枚を、置いた element の描画メソッドで描く。
            ElementKind::Film(film) => {
                let transform = Transform::translate(x, y);
                let (world, atlas, pattern) = (level.world, level.atlas, level.pattern);
                let op = film_op(world, atlas, film, pattern, &element.method, transform);
                s.steps.extend(op.map(Step::Op));
                continue;
            }
            ElementKind::Surface(child) if level.world.surface(child).is_none() => {
                Some("面の表に無い番号")
            }
            ElementKind::Surface(child) if s.on_path.contains(&child) => {
                Some("先祖へ戻る参照（循環）")
            }
            ElementKind::Surface(child) => {
                s.steps.push(Step::Enter {
                    surface: child,
                    x,
                    y,
                });
                continue;
            }
        };
        if let Some(reason) = skip_reason {
            tracing::debug!(
                target: "areka_emo_compose",
                surface_id,
                element = element.layer,
                child = element.path.as_str(),
                reason,
                "element定義の子のサーフェスを置かない（読み込みのとき warn 済み）"
            );
            continue;
        }
        // 束縛が Some の画像のみ命令化する。None（画像の解決に失敗・bind 時に debug・脱落は読み込みの error 済み）はスキップ。
        let Some(element_id) = level.binding.0.get(i).copied().flatten() else {
            tracing::trace!(
                target: "areka_emo_compose",
                surface_id,
                layer = element.layer,
                path = element.path.as_str(),
                "未束縛の画像 element を静的命令からスキップ（画像の解決に失敗・bind 時 debug 済み）"
            );
            continue;
        };
        // placement None（全透明＝空エントリ）の element は命令化せずスキップ（要件 6.3）。原寸は
        // 既知ゆえ外形（compute_extent）へは寄与するが、転写命令には現れない（通常系・warn しない）。
        if level.atlas.entry(element_id).placement.is_none() {
            tracing::trace!(
                target: "areka_emo_compose",
                surface_id,
                layer = element.layer,
                path = element.path.as_str(),
                "placement None（全透明）element を命令からスキップ（外形へは寄与）"
            );
            continue;
        }
        s.steps.push(Step::Op(BlitOp {
            element: element_id,
            transform: Transform::translate(x, y),
            method: element.method.clone(),
        }));
    }
}

/// 合成対象 surface の静的 element 層＋有効 bind pattern0 層の転写命令を `out_ops` へ導出する（要件 5.2/5.3/5.4/5.6）。
///
/// design「PlanBuilder（plan.rs）」の層列挙規則に厳密に従う:
///
/// 1. **静的 element 層**（i）: 合成対象 surface 自身の element を layer 昇順・同 layer は登場順で
///    先頭へ積む（**下＝基底層**・[`push_static_element_ops`]）。
/// 2. **有効 bind pattern0 層**（ii）: 合成対象 surface の `SurfaceMaster.animations` のうち、
///    interval が bind 種（[`Interval::Bind`]/[`Interval::BindRandom`]）**かつ** animation id が
///    `binds` に含まれる animation を「有効 bind」とし、その pattern0（index 0 の [`Pattern`]）を
///    **2 段ソート順**で静的層の**後（＝上）**へ積む。ただし pattern0 を静的に積むのは純
///    [`Interval::Bind`] だけで、[`Interval::BindRandom`] はコマがあるときだけそのコマを描く
///    （件 26・`animated-surface-log-flood` 要件 9.1）。
///    - **段1（animation-sort・要件 5.3/5.6・design 決定5）**: [`EmoWorld::animation_sort`] が
///      [`SortOrder::Descend`]（既定）→ animation id **昇順**に描画（大 id が上）。[`SortOrder::Ascend`]
///      → id **降順**に描画（小 id が上）。これが画家のアルゴリズムの画素積層写像。
///    - **段2**: 上記方向の中で animation id をキーに整列する。
///
/// pattern0 の `surface_id >= 0` は**入れ子 surface 参照**として、参照先 [`SurfaceMaster`] の
/// **elements のみ**を pattern の (x,y) をオフセット**累積**して inline 展開する
/// （多段 flatten・design 決定2）。参照先 surface 自身の bind animation は展開しない（design
/// 「`active_binds` の適用範囲は compose 対象 surface のみ」）。`surface_id < 0` はレイヤクリア/停止
/// センチネル＝**非描画 skip**（trace ログ・非パニック・要件 5.5 前段）。
///
/// # 循環検出と広がりの予算（要件 7.1/7.2/7.3・指摘 3）
///
/// 入れ子は明示のスタック（[`PlanScratch`]）で回す（[`flatten_surface`]）。既に**現在の祖先経路**に
/// 存在する surface へ入ろうとすると（自己参照・相互参照の循環）、`debug!` の上その枝のみを打ち切って
/// 部分結果を残す。祖先は段を出るときに外すので、非循環の重複参照（DAG の共有子）は各経路で正しく
/// 展開される。合成 1 回に予算 1 つを渡し、尽きたらまだ入っていない枝を捨てる（根の面の自分の層は
/// 必ず描く・`error!` は 1 度だけ）。
///
/// # 決定性（要件 4.5/10.1）
///
/// 静的層は安定順、bind 層は (animation id → 昇/降順) の全順序で整列し、入れ子も同一走査順で
/// 決定的ゆえ、同一入力（World／surface_id／binds）に対し毎回同一命令列を append する（循環入力でも
/// 有界かつバイト等価）。`out_ops` は clear しない（追記のみ・clear は [`build_plan`]）。
pub(crate) fn derive_ops(
    out_ops: &mut Vec<BlitOp>,
    scratch: &mut PlanScratch,
    world: &EmoWorld,
    atlas: &AtlasTable,
    surface_id: u32,
    binds: &BindSet,
    pattern: &PatternState,
) {
    let walk = Walk {
        world,
        atlas,
        binds,
        pattern,
    };
    flatten_surface(out_ops, scratch, walk, surface_id);
}

/// 根の面 `root` から段を明示のスタックで回し、積み荷の順に命令を `out_ops` へ積む。
///
/// 積み荷を取り出す時点で、入る面が祖先に在れば循環として捨て（`debug!`）、予算が尽きていれば
/// 捨てる。その時点の祖先の列は再帰で書いたときの入口と同じ（子の段は取り出す前に全部終わって
/// 外れている）ので、命令の並びも再帰と同じになる。
fn flatten_surface(out_ops: &mut Vec<BlitOp>, s: &mut PlanScratch, walk: Walk<'_>, root: u32) {
    s.visited.clear();
    s.on_path.clear();
    s.steps.clear();
    s.frames.clear();
    let mut budget = Budget::new(s.limit);
    // 根の面は予算に数えない（自分の層は必ず描く）。一番上は今までの欄のコマを層 (ii) へ合流させる。
    if let Some(level) = walk.level(root, 0, 0, true) {
        s.open(&level);
    }
    while let Some(frame) = s.frames.last_mut() {
        let (surface, x, y) = match s.steps.get(frame.read) {
            // 段が尽きた: 積み荷を切り詰め、祖先から外す。
            None => {
                s.steps.truncate(frame.start);
                s.frames.pop();
                let popped = s.visited.pop();
                debug_assert!(popped.is_some(), "visited は祖先スタック（LIFO）");
                if let Some(surface) = popped {
                    s.on_path.remove(&surface);
                }
                continue;
            }
            Some(Step::Op(op)) => {
                out_ops.push(op.clone());
                frame.read += 1;
                continue;
            }
            Some(&Step::Enter { surface, x, y }) => (surface, x, y),
        };
        frame.read += 1;
        // 循環検出（要件 7.2/7.3）: 現在の祖先経路に既出なら打ち切り（非パニック）。定義だけで決まる
        // 誤りなので、組むたびの知らせは debug（warn は読み込みが `shape_issues` で 1 度だけ出す）。
        if s.on_path.contains(&surface) {
            tracing::debug!(
                target: "areka_emo_compose",
                surface_id = surface,
                "入れ子参照の循環を検出・枝を打ち切り"
            );
            continue;
        }
        if !budget.enter("plan", root) {
            continue;
        }
        // 部品の段（element定義の子・pattern定義の先）は部品の欄を読む。
        if let Some(level) = walk.level(surface, x, y, false) {
            s.open(&level);
        }
    }
}

/// 層 (ii): 段の有効な着せ替えとコマを、animation-sort の 2 段規則の順に積み荷にする。
///
/// # 合成合流と method ゲート（task 7.2・要件 4.2/4.6/5.3/8.4）
///
/// 層(ii) の対象 id 集合へ、この段のコマ（一番上なら [`PatternState`] の今までの欄、部品の段なら
/// 部品の欄の `surface_id` の分）を持つ id を合流する。
/// 合流対象 id 集合 =
/// `{ 有効 bind pattern0 を持つ id } ∪ { この段に現在コマを持つ id のうち bind 種でないもの、
/// または bind 種でも現在の bind 集合に属するもの }`（bind 所属条件は bindopt D9-3・下記）。整列は既存の
/// animation-sort 2 段規則を**変更せず**適用する（R5.3・画家のアルゴリズム）。各 id で現在コマが
/// あれば**コマが pattern0 静的寄与を置換**する（4.2「各コマは直前コマをリセットしてベースへ」）。
/// コマ・pattern0 いずれも method ゲート（[`ComposeMethod::is_implemented`]＝Overlay のみ）を通し、
/// 非 Overlay は `debug!`（method 名込み）＋不描画（完全形保持のまま非駆動・8.4）。一番上と部品の段は
/// 読む欄だけが違い、規則は同じ 1 本の経路を通る（surface-element-nesting 要件 5.1・5.10・5.12）。
///
/// # bind 非所属コマの合流拒否（bindopt D9-3・最後の砦・bindopt 7.1/7.4）
///
/// 上記の合流で、**当 surface の `bind+random`** に属する id のコマは `binds.contains(id)` のときだけ
/// 合流する。`-1` 終端を持たないアニメは末尾到達後に最終コマを保持する仕様ゆえ、着せ替え ID が bind
/// 集合から外れても保持コマが残り得る。上流の掃除が漏れても**外れたパーツが表示に残らない**
/// 不変量を合成計画側でも保証する。**bind に属さないアニメ（純 `random` の `interval` 等）のコマは
/// 無条件で従来どおり合流する**（bindopt 7.4）。合成順・重ね順は本ゲートで一切変わらない。
///
/// `binds` は全段で同じものを参照するが、参照先の bind animation は「その surface の `animations` に
/// 定義され `binds` に含まれる」もののみが有効（emo2 の bind パーツ surface は bind を持たない）。
fn push_bind_layer(s: &mut PlanScratch, level: &Level<'_>) {
    let (master, surface_id) = (level.master, level.surface_id);
    // `always` は欄が無くても経過 0 を描くので、いつも重ねる対象に入れる（順は同じ規則）。
    s.merged_ids.clear();
    s.merged_ids.extend(
        master
            .animations
            .iter()
            .filter(|a| {
                (is_bind_interval(&a.interval) && level.binds.contains(a.id))
                    || is_always_interval(&a.interval)
            })
            .map(|a| a.id),
    );
    let pattern = level.pattern;
    let frames = (pattern.iter().filter(|_| level.is_top_level))
        .chain(pattern.part(surface_id).filter(|_| !level.is_top_level));
    // 現在コマの id を合流（有効 bind pattern0 を持たない id も含む）。重複は既存を優先し
    // 二重列挙しない（整列後も 1 回だけ処理される）。
    for (id, _frame) in frames {
        if s.merged_ids.contains(&id) {
            continue;
        }
        // bindopt D9-3（最後の砦・bindopt 7.1）: `bind+random` のコマは bind 集合に属するときだけ
        // 合流する。純 `bind` のコマは呼ばれて始まったものだけなので、無効でも合流する
        // （seriko-pattern-calls 5.6・M5）。bind に属さないアニメは無条件で合流する（bindopt 7.4）。
        if is_bind_random_animation(master, id) && !level.binds.contains(id) {
            tracing::debug!(
                target: "areka_emo_compose",
                surface_id,
                animation_id = id,
                "bind 集合に属さない bind 種アニメの保持コマ: 合成計画へ合流しない（bindopt 7.1・D9-3）"
            );
            continue;
        }
        s.merged_ids.push(id);
    }

    // 段1/段2: animation-sort の2段規則を**変更せず**合流後 id 集合へ適用する（design 決定5・
    //   画家のアルゴリズム写像・R5.3）。Descend（既定）→ id 昇順に描画（大 id が上）。
    //   Ascend → id 降順に描画（小 id が上）。
    match level.world.animation_sort() {
        SortOrder::Descend => s.merged_ids.sort_unstable(),
        SortOrder::Ascend => s.merged_ids.sort_unstable_by(|a, b| b.cmp(a)),
        // 知らない値（`Other`・`#[non_exhaustive]` の先）は既定 Descend（id 昇順描画）へ倒す。
        // 1 シェル 1 度の warn は読み込み側が `shape_issues` で出す。
        other => {
            tracing::debug!(
                target: "areka_emo_compose",
                surface_id,
                sort = ?other,
                "未知の animation-sort: 既定 Descend（id 昇順描画）として扱う"
            );
            s.merged_ids.sort_unstable();
        }
    }

    // 描画順に各 id の寄与（コマ優先・無ければ `always` の経過 0 か着せ替えの手順）を静的層の
    // 後（＝上）へ積む。`insert` で差し込まれる着せ替えは本来の位置では積まない（6.1）。
    crate::bind_steps::claim_inserts(s, level);
    s.done.clear();
    for k in 0..s.merged_ids.len() {
        let id = s.merged_ids[k];
        if !s.claims.iter().any(|&(b, _)| b == id) && push_direct(&mut s.steps, level, id) {
            crate::bind_steps::push_bind_contribution(s, level, id);
        }
    }
}

/// 段の入力（層 (i)・層 (ii) の積み荷を作る口が共有する）。
pub(crate) struct Level<'a> {
    pub(crate) world: &'a EmoWorld,
    pub(crate) atlas: &'a AtlasTable,
    pub(crate) master: &'a SurfaceMaster,
    pub(crate) binding: &'a AtlasBinding,
    pub(crate) surface_id: u32,
    pub(crate) binds: &'a BindSet,
    pub(crate) pattern: &'a PatternState,
    pub(crate) offset_x: i64,
    pub(crate) offset_y: i64,
    pub(crate) is_top_level: bool,
}

impl Level<'_> {
    /// この段の animation `id` の今のコマ（一番上は今までの欄、部品の段は部品の欄）。
    pub(crate) fn frame(&self, id: u32) -> Option<&PatternFrame> {
        if self.is_top_level {
            self.pattern.get(id)
        } else {
            self.pattern.part_get(self.surface_id, id)
        }
    }

    /// 面 `surface` をこの段の位置＋ (x, y) に置く部品の段へ入る積み荷。
    pub(crate) fn enter(&self, surface: u32, x: i64, y: i64) -> Step {
        Step::Enter {
            surface,
            x: self.offset_x + x,
            y: self.offset_y + y,
        }
    }
}

/// 層 (ii) の animation `id` の寄与のうち、手順の展開の要らないもの（今のコマ・`always` の経過 0）を
/// 積む。今のコマは静的な寄与を置き換える（4.2）。コマの無い有効な着せ替えで描く手順があるなら、
/// 積まずに `true` を返す（呼び手が [`crate::bind_steps::push_bind_contribution`] で手順をたどる）。
pub(crate) fn push_direct(steps: &mut Vec<Step>, level: &Level<'_>, id: u32) -> bool {
    let surface_id = level.surface_id;
    if let Some(frame) = level.frame(id) {
        if frame.method.is_implemented() {
            // Overlay: frame.surface_id へ (x,y) 累積で入る（循環は取り出すときの visited 判定）。
            steps.push(level.enter(frame.surface_id, frame.x, frame.y));
        } else {
            // 非 Overlay: 完全形は保持しつつ非駆動＝当該コマ不描画（要件 8.4）。組むたびの
            // 知らせは debug（warn は読み込みが `shape_issues` で 1 度だけ出す）。
            tracing::debug!(
                target: "areka_emo_compose",
                surface_id,
                animation_id = id,
                method = ?frame.method,
                "非 Overlay method の現在コマ: 不描画 skip（完全形保持・非駆動・要件 8.4）"
            );
        }
        return false;
    }
    // 同 id の animation は fold 段で単一化済み（後勝ち）ゆえ find で足りる。
    let Some(anim) = level.master.animations.iter().find(|a| a.id == id) else {
        return false;
    };
    // `always`: 欄が「載っていない」なら経過 0 の pattern を今のコマと同じやり方で描き、
    // 「消えている」なら描かない（要件 3.1・4.1・4.5・4.6）。
    if is_always_interval(&anim.interval) {
        let key = (!level.is_top_level).then_some(PartKey::Surface(surface_id));
        if let Some(p) = always_rest_target(anim, level.pattern.cell(key, id), surface_id) {
            steps.push(level.enter(p.surface_id as u32, p.x, p.y));
        }
        return false;
    }
    // コマ無しの有効な着せ替え: 読み込みのときに作った手順を積む（合成のたびに語を引き直さない・5.5）。
    if level.world.bind_steps(surface_id, id).is_empty() {
        // `bind+random`（コマが無いあいだは描かない・件 26）か、描くコマが無い: 静的な寄与は無い（再生は seriko の領分）。
        tracing::debug!(
            target: "areka_emo_compose",
            surface_id,
            animation_id = id,
            "有効な着せ替えに描く手順が無い: 静的合成へ寄与なし"
        );
        return false;
    }
    true
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
/// # 作業場（要件 10.3・buffer 再利用形）
///
/// `scratch` は呼び手（Composer ファサード）が保持する走査の作業場（[`PlanScratch`]）を借用する。
/// 外形走査（[`compute_extent`]）は経路の集合を、命令走査（[`derive_ops`]）は作業場の全部を、
/// それぞれ入口で空へ戻して使う。これにより定常状態（毎フレーム経路）で再割り当てが起きない。
pub(crate) fn build_plan(
    out_ops: &mut Vec<BlitOp>,
    scratch: &mut PlanScratch,
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

    // 静的外形（有効 bind 非依存）を先に算出する。0×0 なら定義層皆無の退化データ。祖先の列は
    // compute_extent 入口で clear される（要件 10.3・共有スクラッチ）。予算は命令列と同じ範囲・同じ値。
    let extent = compute_extent_limited(
        &mut scratch.on_path,
        world,
        atlas,
        surface_id,
        scratch.limit,
        None,
    );

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

    // 外形が GPU の面の最大寸を越える → 命令を積まず、絵も確保させずに失敗を返す（壊れたシェルで
    // 巨大な確保を試みて落ちない）。
    if extent.w > MAX_EXTENT || extent.h > MAX_EXTENT {
        tracing::error!(
            target: "areka_emo_compose",
            surface_id,
            w = extent.w,
            h = extent.h,
            max = MAX_EXTENT,
            "外形が GPU の面の最大寸を越える: ExtentTooLarge"
        );
        return Err(ComposeError::ExtentTooLarge {
            surface: surface_id,
            w: extent.w,
            h: extent.h,
        });
    }

    // 有効 bind 依存の命令列を積む（描画可能命令ゼロ＝空 ops でもよい）。作業場は derive_ops
    // 入口で空へ戻る（要件 10.3）。外形 compute_extent は pattern 非依存の静的量。
    derive_ops(out_ops, scratch, world, atlas, surface_id, binds, pattern);

    // 分類2: surface 存在＋外形非ゼロ → 正常系。out_ops が空でも（全透明・空 bind 集合）Err に
    // しない（要件 6.6・議題2裁定: 静的外形どおりの空命令列＝全透明返却）。
    Ok(extent)
}

/// interval が bind 種（`Interval::Bind`/`Interval::BindRandom`）か（要件 5.2・design 層列挙）。
///
/// `Interval::Random`（純ランダム・非 bind）は有効 bind の対象にしない。`Interval` は
/// `#[non_exhaustive]` ゆえ未知 variant は bind でないものとして扱う（非パニック）。
pub(crate) fn is_bind_interval(interval: &Interval) -> bool {
    matches!(interval, Interval::Bind | Interval::BindRandom { .. })
}

/// 指定 animation id が当 surface の **`bind+random`** アニメか（bindopt D9-3・bindopt 7.1/7.4・
/// seriko-pattern-calls 5.6 で純 `bind` を外した）。
///
/// [`flatten_surface`] の層(ii) 合流条件が「`bind+random` のコマは bind 集合に属することを要求する」
/// ための判定。当 surface の `animations` から id を引き、interval が `bind+random` のものが 1 本でも
/// あれば真。**当 surface に定義の無い id・純 `bind`・bind 種でない id（純 `random` の `interval` 等）は
/// 偽**で、追加ゲートを受けず無条件に合流する（bindopt 7.4）。
///
/// 判定は `emo-compose` が既に持つ [`Interval`]（転記層由来）だけで閉じており、bind 種の語彙を
/// 求めて seriko へ依存を逆流させない（design Allowed Dependencies「parsers → areka → seriko →
/// emo-compose・逆流禁止」）。同 id の animation は fold 段で単一化済みゆえ `any` で足りる。
fn is_bind_random_animation(master: &SurfaceMaster, animation_id: u32) -> bool {
    master
        .animations
        .iter()
        .any(|a| a.id == animation_id && matches!(a.interval, Interval::BindRandom { .. }))
}

/// 指定 surface id の [`SurfaceMaster`] と [`AtlasBinding`] を同時に読む（本 module 内補助）。
///
/// `SurfaceIndex`（疎 id→Entity・O(1)）で entity を引き、`SurfaceMaster` と `AtlasBinding` を
/// 併せて借用する。いずれか（surface 不在・binding 未挿入）が欠けると `None`。既存の公開
/// `world()` アクセサ経由で読むため、公開 API を広げない。
///
/// [`push_static_element_ops`]／[`derive_ops`] 経由で使う本 module 内補助。
pub(crate) fn surface_and_binding(
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
#[path = "plan_ops_tests.rs"]
mod ops_tests;

#[cfg(test)]
#[path = "plan_frame_merge_tests.rs"]
mod frame_merge_tests;

#[cfg(test)]
#[path = "plan_nesting_tests.rs"]
mod nesting_tests;

#[cfg(test)]
#[path = "plan_bind_random_tests.rs"]
mod bind_random_tests;

#[cfg(test)]
#[path = "plan_bind_tests.rs"]
mod bind_tests;
