//! actor の子: 登録と再追従（装着先の解決・大きさの再追従・粗いバルーン定義の警告）。
//! 足す予定の spec: shell-balloon（登録の口）・balloon-font-file。

use areka_emo_present::TextSlotView;
use areka_parsers::balloon::BalloonModel;
use areka_sakura::contract::ActorKey;
use tracing::{debug, info, warn};

use crate::place::PlaceKey;
use crate::region::{BALLOON_NAME_PLACEHOLDER, TextRegion, inline_axis_name};
#[cfg(doc)]
use crate::state::TextLayerState;

use super::decoration;
#[cfg(doc)]
use super::present_frame;
use super::{ResolvedBalloonText, TextLayerRuntime, TextSlotBinding};

/// 粗いバルーン定義（折返し基準が描画範囲の遠辺の外）を「装着ごとに 1 件」だけ知らせる
/// （spec `areka-P0-emo2-conformance-e2e` 要件 14.1〜14.3・design D14・2026-09-06）。
///
/// # なぜ解決する側ではなくここなのか
///
/// この `warn!` は以前 [`TextRegion::resolve`] の中にあった。しかし解決は**毎フレーム**走る
/// ——再追従シーム（[`TextLayerRuntime::refresh_actor_binding`]）が churn ガードの判定キーを
/// 得るために解き直すためで、実機の一周走行では生ログ 30,837 行のうち **27,908 行**がこの
/// 1 種類の警告になった（走行 A の実測）。件数が意味を失い、生ログが読めなくなる。
///
/// 「読み込み（装着）1 回につき 1 件」という意味を持つのは actor の登録口である。ゆえに
/// 解決からは粗さの記録を外し、記録は本関数が担う。文言と 4 つの欄（`balloon`・`axis`・
/// `wrap_threshold`・`inline_limit`）は移動の前後で 1 文字も変えていない——実機走行の判定は
/// この語を grep する（要件 14.3・手順書 §5.7）。バルーン名の代替値は解決側と同一の
/// [`BALLOON_NAME_PLACEHOLDER`] を共有する。
///
/// # 件数（要件 14.2）
///
/// `previous` は当該 actor の**前回の**解決済み領域（未登録＝装着なら `None`）。値が新しく
/// 決まったとき——初回、または前回と異なる領域——だけ 1 件書く:
///
/// - 装着（`register_actor_binding`）は必ず初回ゆえ **1 件**、
/// - 値の同じ再追従は churn ガードで [`TextLayerRuntime::register_actor`] に達しない **0 件**、
/// - binding だけが変わって領域が同値の再追従は、登録口に達しても **0 件**、
/// - 領域の値が変わる再追従は **1 件**（新しい値で）。
///
/// 檻は `actor_region_warn_tests.rs`（装着 1 件・4 欄・3 回の同値再追従で 0 件・再構築するが
/// 領域同値で 0 件・値の変わる再追従で 1 件・遠辺の内で 0 件・縦書きの軸欄）。
fn warn_coarse_wrap_threshold(resolved: &ResolvedBalloonText, previous: Option<TextRegion>) {
    let region = resolved.region;
    if region.wrap_threshold() <= region.inline_limit() || previous == Some(region) {
        return;
    }
    warn!(
        balloon = BALLOON_NAME_PLACEHOLDER,
        axis = inline_axis_name(resolved.mode),
        wrap_threshold = region.wrap_threshold(),
        inline_limit = region.inline_limit(),
        "折返し基準が描画範囲の外に解決された——実効の折返し位置は描画範囲の辺になる（バルーン定義側の粗さ）"
    );
}

impl TextLayerRuntime {
    /// actor と装着先（予約スロット）＋layout 入力の対応を登録する（結線側の口——
    /// `ActorKey → TargetId` の対応は結線側が所有し、emo-present `TextSlotView` の読み値
    /// から [`TextSlotBinding::new`]／[`ResolvedBalloonText::resolve`] で組んで渡す・R9.5）。
    ///
    /// 未解決のまま蓄積していた actor は次の [`present_frame`] で再試行され装着される。
    ///
    /// # 粗いバルーン定義の警告はここが書く（要件 14.2・2026-09-06）
    ///
    /// 折返し基準が描画範囲の遠辺の外に解決されたことを知らせる `warn!` は本口が書く——
    /// 「読み込み（装着）1 回につき 1 件」という意味を持つ層がここだからである
    /// （本ファイルの `warn_coarse_wrap_threshold` の doc に経緯）。
    ///
    /// 範囲外ゆえ無視した `origin` 宣言の警告（本 spec 要件 3.1〜3.5）も同じ理由で本口が書く。
    /// 関数の本体だけは `actor_decoration.rs` に置く（本ファイルの行数の余白のため）。
    pub fn register_actor(
        &mut self,
        actor: ActorKey,
        binding: TextSlotBinding,
        resolved: ResolvedBalloonText,
    ) {
        debug!(actor = %actor, slot = ?binding.slot, "actor の装着先（予約スロット）を登録した");
        // 前回の解決済み領域（未登録＝装着なら None＝「値が新しく決まった」側）と突き合わせる。
        let place = PlaceKey::balloon(&actor);
        let previous = self.layout_input.get(&place).map(|it| it.region);
        warn_coarse_wrap_threshold(&resolved, previous);
        decoration::warn_ignored_origin(&resolved, previous);
        // 2 層（既定・無効表示・選択肢文字色）を純粋状態へ差し込む唯一の点（要件 3.1／4.6）。
        // 装着も再追従もここへ合流するので、`\f[default]`／`\f[disable]` の戻し先は
        // 常に「今装着されているバルーン定義」で解決した値になる。
        self.state
            .set_look_layers(&actor, resolved.font.looks.clone());
        self.routing.insert(place.clone(), binding);
        self.layout_input.insert(place, resolved);
    }

    /// 統合配線の一点口（R9.5・task 8）: `ActorKey → TargetId` 対応の解決結果
    /// （`EmoPresenter::text_slot_view(target)` の view——対応関係の所有は結線側・
    /// example/emo2-boot）と 2 層マージ済み balloon model から、binding
    /// （[`TextSlotBinding::from_view`]）と layout 入力（[`ResolvedBalloonText::resolve`]・
    /// 入力は必ず binding の `image_size`＝**image px**）を導出して
    /// [`register_actor`](Self::register_actor) へ登録する。
    ///
    /// 物理 px を領域解決へ渡す誤配線をこの口で構造閉塞する（2 空間モデル——design.md
    /// 「DPI/スケール契約」）。行レイアウト（`PositionedLine`）・クリック可能範囲の
    /// choice-render 再利用シーム（R9.4）には手を触れない——本口は入力の供給のみで、
    /// レイアウト出力の形を変えない。
    ///
    /// `text_slot_view` は初回 `ShowSurface` 前は `None` を返すため、呼び手は表示確立後に
    /// 本口で登録する（未登録の間に届いた cue は蓄積され、登録後の次フレームで装着・描画される）。
    pub fn register_actor_view(
        &mut self,
        actor: ActorKey,
        view: &TextSlotView,
        model: &BalloonModel,
    ) {
        self.register_actor_binding(actor, TextSlotBinding::from_view(view), model);
    }

    /// **単一構築経路の内側**（binding 直渡し・装着側）: view からの読み取りは呼び手側の
    /// [`TextSlotBinding::from_view`] 一点で完結し、以降の導出——`binding.image_size` を入力と
    /// する [`ResolvedBalloonText::resolve`] と [`register_actor`](Self::register_actor) への
    /// 登録——は装着でも再追従でも完全に同一である（第 2 の構築流儀を作らない・R4.3/R8.1）。
    ///
    /// 再追従側（[`refresh_actor_binding`](Self::refresh_actor_binding)）は判定キーに解決済み
    /// 領域を含む都合で `resolve` を**判定前に自分で 1 回だけ**呼び、その値のまま
    /// `register_actor` へ合流する（二重 resolve の回避——導出そのものは本メソッドと同一）。
    pub(super) fn register_actor_binding(
        &mut self,
        actor: ActorKey,
        binding: TextSlotBinding,
        model: &BalloonModel,
    ) {
        let resolved = ResolvedBalloonText::resolve_with_background(
            model,
            binding.image_size,
            self.background_of(&actor),
        );
        self.register_actor(actor, binding, resolved);
    }

    /// balloon target の**再追従シーム**（適用 k・面実寸・文字描画領域の変化に追従する・
    /// R4.3〜R4.7／R8.1/8.2/8.3/8.5/8.7・design D11/D3）。
    ///
    /// 窓 DPI 変化（モニタ跨ぎ移動・表示スケール変更）で emo-present の適用 k が変わったとき、
    /// あるいは同じ k のままバルーン面の実寸や当該 scope の `validrect` が変わったとき、
    /// 結線側（`emo2_boot` の DPI フェーズ）が新しい [`TextSlotView`] を携えて呼ぶ。view から
    /// binding を再構築し（[`register_actor_view`](Self::register_actor_view) と同一の導出）、
    /// 当該 actor の `ActorRender`（供給面・描画実行部・実測 metrics）を破棄する。次の
    /// [`present_frame`] の初回解決分岐が**新 k の物理寸**（`ceil(validrect 寸 × k)`／
    /// `validrect 原点 × k`）で再生成する——既存の生成式をそのまま再利用し、旧寸供給面は
    /// 再利用しない（R8.2）。
    ///
    /// # リビール状態は保存される（R8.3・`Clear`/`ClearAll` と別物）
    ///
    /// 破棄するのは描画資源だけで、純粋状態（[`TextLayerState`]——typewriter 進行・確定行）には
    /// 触れない（[`register_actor`](Self::register_actor) が `routing`＋`layout_input` しか
    /// 上書きしない既存構造がこれを担保する）。確定行 TextLayout キャッシュは `ActorRender` に
    /// 宿るため、破棄→再生成で次フレームは保存済み状態から**全再描画**される（R8.4）。
    ///
    /// # 戻り値（churn ガード・R4.5/R8.5）
    ///
    /// 再追従を行ったとき `true`、行わなかったとき `false`。判定キー——binding 全体
    /// （k・物理寸・image 原寸・slot・window）と `model`×`image_size` から解き直した
    /// [`ResolvedBalloonText`]——が**すべて同値**（identity 再導出を含む）なら
    /// **no-op で `false`**：毎フレーム再結線・再生成を構造的に禁じる。逆に k が同値でも
    /// 面実寸や文字描画領域が違えば再構築する（R4.4——k の同値のみを根拠に省略しない）。
    /// 未登録 actor（まだ装着されていない）も `false`——装着は `register_actor_view` の
    /// 領分であり、本口が第 2 の装着経路にならない（R4.6）。
    pub fn refresh_actor_scale(
        &mut self,
        actor: &ActorKey,
        view: &TextSlotView,
        model: &BalloonModel,
    ) -> bool {
        self.refresh_actor_binding(actor, TextSlotBinding::from_view(view), model)
    }

    /// [`refresh_actor_scale`](Self::refresh_actor_scale) の内側（binding 直渡し・判断分岐の本体）。
    /// `TextSlotView` は emo-present 私有フィールド型ゆえ in-crate 檻から構築できないため、
    /// 判断分岐をこの層で檻に入れられるよう分けてある（公開口との差は view 読み取りの有無のみ）。
    pub(super) fn refresh_actor_binding(
        &mut self,
        actor: &ActorKey,
        binding: TextSlotBinding,
        model: &BalloonModel,
    ) -> bool {
        let place = PlaceKey::balloon(actor);
        let Some(&current) = self.routing.get(&place) else {
            // 未装着 actor（`text_slot_view` が None のまま等）——再構築すべき binding が無い。
            // 失敗ではなく「対象なし」の静穏 skip（装着は register_actor_view の領分・R4.6）。
            debug!(
                actor = %actor,
                k = binding.scale,
                "文字層の再追従: 未登録 actor のため何もしない（装着は register_actor_view の領分）"
            );
            return false;
        };
        let (k_old, k_new) = (current.scale, binding.scale);
        // 判定キー＝binding 全体（k・物理寸・image 原寸・slot・window）と、その image 原寸で
        // model から解き直した文字描画領域の**連言**（D3・R4.4）。k の同値のみを根拠に再追従を
        // 省略すると、同 k のまま面実寸や scope 別 `validrect` が変わったときに旧寸の文字層が
        // 残る。k は双方とも ScaleContract 正規化済み（TextSlotBinding::new 経由）の同一表現
        // ゆえ、（derive した `PartialEq` 経由の）厳密一致で「変化なし」を判定してよい
        // （f32 は出口ビュー——ここでは比較にのみ使い、寸法演算には一切用いない・D4）。
        //
        // 再解決は純関数ゆえ判定前にここで **1 回だけ**行い、再構築側でもこの値をそのまま使う
        // （二重 resolve・第 2 の構築流儀を作らない・R4.3）。
        let resolved = ResolvedBalloonText::resolve_with_background(
            model,
            binding.image_size,
            self.background_of(actor),
        );
        if current == binding && self.layout_input.get(&place) == Some(&resolved) {
            debug!(
                actor = %actor,
                k = k_new,
                "文字層の再追従: 適用 k・面実寸・文字描画領域がいずれも同値のため再結線・再生成を行わない（churn ガード・R4.5）"
            );
            return false;
        }

        // 装着と同一の導出（`ResolvedBalloonText::resolve` → `register_actor`）で binding／
        // layout 入力を再構築する（単一構築経路・R4.3）。純粋状態（TextLayerState）には
        // 触れない＝リビール進行・確定行は保存される（R4.7）。
        self.register_actor(actor.clone(), binding, resolved);
        // 描画資源だけを破棄する。次 present_frame の初回解決分岐が新しい k・面実寸の物理寸で
        // 再生成し、空の行 TextLayout キャッシュから保存済み状態を全再描画する（R4.3/R8.4）。
        self.surfaces.remove(&place);
        info!(
            actor = %actor,
            k_old,
            k_new,
            image_size = ?binding.image_size,
            surface_size = ?binding.surface_size,
            "文字層の再追従: binding と文字描画領域を再構築し描画資源を破棄した（次フレームで新しい物理寸へ再生成・リビール状態は保存）"
        );
        true
    }
}
