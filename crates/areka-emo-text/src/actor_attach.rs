//! actor の子: 登録と再追従（装着先の解決・大きさの再追従・粗いバルーン定義の警告・
//! 潰れた描画範囲の状態の変わり目の記録）。
//! 足す予定の spec: balloon-font-file。

use areka_emo_present::TextSlotView;
use areka_parsers::balloon::BalloonModel;
use areka_sakura::contract::ActorKey;
use tracing::{debug, info, trace, warn};

use crate::choice::CursorFallback;
use crate::place::PlaceKey;
use crate::region::{TextRegion, inline_axis_name};
#[cfg(doc)]
use crate::state::TextLayerState;

use super::decoration;
#[cfg(doc)]
use super::present_frame;
use super::{ResolvedBalloonText, TextLayerRuntime, TextSlotBinding};

/// 解き直しの入力（[`ResolvedBalloonText::resolve_with_background`] の引数と同じ 3 つ）。
/// 普通のバルーンの場所ごとに `TextLayerRuntime` が覚え、再追従は入力も結線も前と同じなら
/// 解き直さない（解き直しは純粋な関数なので、同じ入力からは `layout_input` と同じ値が出る）。
///
/// **解き直しの引数を足したら、この欄も足す**——3 つのほかを読むようになると、省いたときの
/// 答えが解き直したときの答えと食い違う。
#[derive(Clone, Debug, PartialEq)]
pub(super) struct ResolveInputs {
    model: BalloonModel,
    image_size: (u32, u32),
    background: (u8, u8, u8),
}

impl ResolveInputs {
    /// 覚えた入力が、いま解き直すなら渡す入力と同じか（毎フレームの比べ・写しを作らない）。
    fn is(&self, model: &BalloonModel, image_size: (u32, u32), background: (u8, u8, u8)) -> bool {
        self.image_size == image_size && self.background == background && self.model == *model
    }

    /// 覚えた入力の選択肢の飾りの縮退の値（飾りの警告の「前の値」・要件 2.3）。
    fn cursor_fallback(&self) -> CursorFallback {
        CursorFallback::of(self.model.cursor())
    }
}

/// 粗いバルーン定義（折返し基準が描画範囲の遠辺の外）を「装着ごとに 1 件」だけ知らせる
/// （spec `areka-P0-emo2-conformance-e2e` 要件 14.1〜14.3・design D14・2026-09-06）。
///
/// # なぜ解決する側ではなくここなのか
///
/// この `warn!` は以前 [`TextRegion::resolve`] の中にあった。しかし解決は**毎フレーム**走る
/// ——再追従シーム（[`TextLayerRuntime::refresh_actor_binding`]）が churn ガードの判定キーを
/// 得るために解き直すためで、実機の一周走行では生ログ 30,837 行のうち **27,908 行**がこの
/// 1 種類の警告になった（走行 A の実測）。件数が意味を失い、生ログが読めなくなる。
/// （以上は当時の経緯。今の再追従は入力が前と同じなら解き直さないので、解決が走るのは装着と
/// 入力の変わり目だけである・spec `real-run-log-noise` 要件 1.1。）
///
/// 「読み込み（装着）1 回につき 1 件」という意味を持つのは actor の登録口である。ゆえに
/// 解決からは粗さの記録を外し、記録は本関数が担う。文言と 4 つの欄（`balloon`・`axis`・
/// `wrap_threshold`・`inline_limit`）は移動の前後で 1 文字も変えていない——実機走行の判定は
/// この語を grep する（要件 14.3・手順書 §5.7）。`balloon` の欄は場所の名前（普通のバルーンは
/// 結線が入れた名前か `スコープ{番号}のバルーン`、箱は `balloon.*`ブレスの名前・shell-balloon
/// 要件 3.11・3.12）で、呼び手が渡す。
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
/// 箱の登録（`actor_box.rs` の `register_box`）は前の配置の入力を持たないので `None` を渡し、
/// 箱の名前ごとに 1 度だけ呼ぶ。
///
/// 檻は `actor_region_warn_tests.rs`（装着 1 件・4 欄・3 回の同値再追従で 0 件・再構築するが
/// 領域同値で 0 件・値の変わる再追従で 1 件・遠辺の内で 0 件・縦書きの軸欄）。
pub(super) fn warn_coarse_wrap_threshold(
    balloon: &str,
    resolved: &ResolvedBalloonText,
    previous: Option<TextRegion>,
) {
    let region = resolved.region;
    if region.wrap_threshold() <= region.inline_limit() || previous == Some(region) {
        return;
    }
    warn!(
        balloon,
        axis = inline_axis_name(resolved.mode),
        wrap_threshold = region.wrap_threshold(),
        inline_limit = region.inline_limit(),
        "折返し基準が描画範囲の外に解決された——実効の折返し位置は描画範囲の辺になる（バルーン定義側の粗さ）"
    );
}

impl TextLayerRuntime {
    /// 描画範囲の潰れを場所の記憶と突き合わせ、状態の変わり目に 1 回だけ記録する
    /// （件 2・要件 2.3〜2.5・2.7）。呼び手は登録口——普通のバルーンの
    /// [`register_actor`](Self::register_actor)（装着・再追従が合流する）と箱の `register_box`。
    ///
    /// - 潰れに入った（記憶に無く、いま潰れている）: `warn!` 1 回。どのバルーンのどの値かが
    ///   読めるよう、場所の名前と validrect の 4 辺と画像の大きさを載せる。
    /// - 戻った（記憶に在り、いま幅・高さとも正）: `info!` 1 回（幅と高さ）。
    /// - それ以外（潰れたまま・正のまま）: 記録しない。
    ///
    /// 記憶（`collapsed_regions`）は登録を外しても箱の束を差し替えても消さないので、隠す・出す・
    /// 付け直しは「潰れたまま」に当たり記録が増えない。名前の欄は粗い定義の警告と同じ
    /// （普通のバルーンは結線が入れた名前か `スコープ{番号}のバルーン`、箱は `balloon.*`ブレスの名前）。
    pub(super) fn note_region_collapse(
        &mut self,
        place: &PlaceKey,
        label: &str,
        resolved: &ResolvedBalloonText,
    ) {
        let region = resolved.region;
        if region.is_collapsed() {
            if self.collapsed_regions.insert(place.clone()) {
                warn!(
                    balloon = label,
                    left = region.left(),
                    top = region.top(),
                    right = region.right(),
                    bottom = region.bottom(),
                    image_size = ?region.image_size(),
                    "文字の描画範囲が潰れている（幅か高さが 0 以下）——このバルーンの文字は描かない"
                );
            }
        } else if self.collapsed_regions.remove(place) {
            info!(
                balloon = label,
                width = region.right() - region.left(),
                height = region.bottom() - region.top(),
                "文字の描画範囲が戻った——次のフレームから文字を描く"
            );
        }
    }

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
    ///
    /// 描画範囲の潰れの状態の変わり目の記録（件 2）も本口が書く（`note_region_collapse`）。
    pub fn register_actor(
        &mut self,
        actor: ActorKey,
        binding: TextSlotBinding,
        resolved: ResolvedBalloonText,
    ) {
        debug!(actor = %actor, slot = ?binding.slot, "actor の装着先（予約スロット）を登録した");
        // 前回の解決済み領域（未登録＝装着なら None＝「値が新しく決まった」側）と突き合わせる。
        let place = PlaceKey::balloon(&actor);
        // 呼び手が入力を知らない直の道なので、記憶を消して不変条件を保つ（入力を知っている
        // 装着と再追従は、この後に記憶を入れ直す）。
        self.resolve_inputs.remove(&place);
        let previous = self.layout_input.get(&place).map(|it| it.region);
        let label = self.balloon_label_of(&actor);
        warn_coarse_wrap_threshold(&label, &resolved, previous);
        decoration::warn_ignored_origin(&label, &resolved, previous);
        self.note_region_collapse(&place, &label, &resolved);
        // 2 層（既定・無効表示・選択肢文字色）を純粋状態へ差し込む唯一の点（要件 3.1／4.6）。
        // 装着も再追従もここへ合流するので、`\f[default]`／`\f[disable]` の戻し先は
        // 常に「今装着されているバルーン定義」で解決した値になる。
        self.state
            .set_look_layers(&actor, resolved.font.looks.clone());
        // 既定の見た目が変わった＝空回しの出発点が変わったので、区間の全文を求め直す（要件 2.2）。
        self.rehearse_again();
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
        let background = self.background_of(&actor);
        let mut resolved =
            ResolvedBalloonText::resolve_with_background(model, binding.image_size, background);
        self.stamp_inline_marker(&actor, &mut resolved);
        let place = PlaceKey::balloon(&actor);
        // 飾りの警告の前の値は、記憶を消す直の登録口より前に取り出す（要件 2.3）。
        let previous = self.cursor_fallback_at(&place);
        self.register_actor(actor.clone(), binding, resolved);
        self.remember_inputs(
            &actor,
            place,
            model,
            binding.image_size,
            background,
            previous,
        );
    }

    /// 場所の記憶の入力から取り出した飾りの縮退の値（記憶が無ければ `None`）。
    fn cursor_fallback_at(&self, place: &PlaceKey) -> Option<CursorFallback> {
        self.resolve_inputs
            .get(place)
            .map(ResolveInputs::cursor_fallback)
    }

    /// 解き直しの入力を場所の記憶へ入れ直す（装着・再追従の作り直しは `register_actor` の後、
    /// 再追従の中の段は解いた値が `layout_input` に等しいと確かめた後に呼ぶ＝不変条件を保つ）。
    ///
    /// 入れ直すときに、選択肢の飾りの縮退の値を `previous`（入れ直す前の記憶の値・呼び手が
    /// `register_actor` の前に取り出す）と比べて警告する（`warn_cursor_fallback`・要件 2.1〜2.4）。
    fn remember_inputs(
        &mut self,
        actor: &ActorKey,
        place: PlaceKey,
        model: &BalloonModel,
        image_size: (u32, u32),
        background: (u8, u8, u8),
        previous: Option<CursorFallback>,
    ) {
        decoration::warn_cursor_fallback(
            &self.balloon_label_of(actor),
            &CursorFallback::of(model.cursor()),
            previous.as_ref(),
        );
        let inputs = ResolveInputs {
            model: model.clone(),
            image_size,
            background,
        };
        self.resolve_inputs.insert(place, inputs);
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
    ///
    /// 判定は 3 段（spec `real-run-log-noise` 要件 1.1〜1.7）:
    ///
    /// - **上の段**: binding も解き直しの入力（[`ResolveInputs`]）も記憶と同じ → 解き直さずに
    ///   `false`。解き直しの中の記録（領域の `debug!`・作者向けの警告）は出ない。
    /// - **中の段**: 入力は変わったが、解き直した値が binding ともども今と同じ → 記憶だけを
    ///   新しい入力へ替えて `false`（次のフレームから上の段で止まる）。
    /// - **下の段**: 値が違う → 今の作り直し（`register_actor` の後に記憶を入れ直す）。
    ///
    /// 上の段の答えが解き直した場合と同じなのは、不変条件「場所の記憶が在れば、その入力から
    /// 解いた値が `layout_input` に等しい」による（直の登録口 `register_actor` が記憶を消し、
    /// 装着と再追従が入れ直す）。記憶の無い場所は今と同じく解き直して比べる。
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
        let background = self.background_of(actor);
        // 上の段: 入力も結線も前と同じ——解き直さない（不変条件により答えは解き直した場合と同じ）。
        if current == binding
            && self
                .resolve_inputs
                .get(&place)
                .is_some_and(|it| it.is(model, binding.image_size, background))
            // 印の見た目は解き直しの入力でなく解いた後に押す値なので、今の値と直に比べる。
            && self
                .layout_input
                .get(&place)
                .is_some_and(|it| it.font.inline_marker == self.inline_marker_of(actor))
        {
            trace!(
                actor = %actor,
                k = k_new,
                "文字層の再追従: 適用 k・面実寸・文字描画領域がいずれも同値のため再結線・再生成を行わない（churn ガード・R4.5）"
            );
            return false;
        }
        // 判定キー＝binding 全体（k・物理寸・image 原寸・slot・window）と、その image 原寸で
        // model から解き直した文字描画領域の**連言**（D3・R4.4）。k の同値のみを根拠に再追従を
        // 省略すると、同 k のまま面実寸や scope 別 `validrect` が変わったときに旧寸の文字層が
        // 残る。k は双方とも ScaleContract 正規化済み（TextSlotBinding::new 経由）の同一表現
        // ゆえ、（derive した `PartialEq` 経由の）厳密一致で「変化なし」を判定してよい
        // （f32 は出口ビュー——ここでは比較にのみ使い、寸法演算には一切用いない・D4）。
        //
        // 再解決は純関数ゆえ判定前にここで **1 回だけ**行い、再構築側でもこの値をそのまま使う
        // （二重 resolve・第 2 の構築流儀を作らない・R4.3）。
        let mut resolved =
            ResolvedBalloonText::resolve_with_background(model, binding.image_size, background);
        self.stamp_inline_marker(actor, &mut resolved);
        if current == binding && self.layout_input.get(&place) == Some(&resolved) {
            // 中の段: 解いた値が今と同じ——記憶だけを新しい入力へ替える（不変条件は保たれる）。
            let previous = self.cursor_fallback_at(&place);
            self.remember_inputs(
                actor,
                place,
                model,
                binding.image_size,
                background,
                previous,
            );
            trace!(
                actor = %actor,
                k = k_new,
                "文字層の再追従: 適用 k・面実寸・文字描画領域がいずれも同値のため再結線・再生成を行わない（churn ガード・R4.5）"
            );
            return false;
        }

        // 装着と同一の導出（`ResolvedBalloonText::resolve` → `register_actor`）で binding／
        // layout 入力を再構築する（単一構築経路・R4.3）。純粋状態（TextLayerState）には
        // 触れない＝リビール進行・確定行は保存される（R4.7）。
        let collapsed = resolved.region.is_collapsed();
        // 飾りの警告の前の値は、記憶を消す直の登録口より前に取り出す（要件 2.3）。
        let previous = self.cursor_fallback_at(&place);
        self.register_actor(actor.clone(), binding, resolved);
        self.remember_inputs(
            actor,
            place.clone(),
            model,
            binding.image_size,
            background,
            previous,
        );
        // 描画資源だけを破棄する。次 present_frame の初回解決分岐が新しい k・面実寸の物理寸で
        // 再生成し、空の行 TextLayout キャッシュから保存済み状態を全再描画する（R4.3/R8.4）。
        // 新しい領域が潰れているときは捨てずに残す: World を持つ提示が同じフレームの下流で
        // 差し込み口から外す（ここで捨てると前の字の絵が差し込み口に残る・件 2 要件 2.9）。
        if collapsed {
            info!(
                actor = %actor,
                k_old,
                k_new,
                image_size = ?binding.image_size,
                surface_size = ?binding.surface_size,
                "文字層の再追従: binding と文字描画領域を再構築した——描画範囲が潰れているので描画資源は残し、同じフレームの提示が外す（戻るまで再生成しない・リビール状態は保存）"
            );
            return true;
        }
        // 印の面は窓の子なので、World を持つ次の提示の頭で消す（描画資源と一緒に捨てると残る）。
        if let Some(markers) = self.surfaces.remove(&place).and_then(|r| r.markers) {
            self.retired_markers.push(markers);
        }
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
