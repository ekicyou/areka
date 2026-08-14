//! 読み取り専用の照会契約——text 層スロット view（`TextSlotView`）・物理寸・実適用 k・現サーフェス id・
//! 表示画素の読み戻し。いずれも表示成立点で確定した値を返すのみで、表示状態を変更しない。

use super::{EmoPresenter, Entity, PresentError, ScaleRatio, TargetId};

/// 予約 text 層スロットへの読み取り専用の到達手段（emo-text-layer が消費する additive 公開増分・R9.1/9.2）。
///
/// [`EmoPresenter::text_slot_view`] が返すスナップショット値。フィールドは非公開（`#[non_exhaustive]`
/// 相当）で accessor のみを公開し、スロット状態の変更手段を一切持たない（読み取り専用 view）。
/// 装着 API 形（emo-present が描画物を受け取る）は emo-text の描画型が本 crate へ逆流し依存方向
/// （emo-present → emo-text 禁止）と衝突するため採らない（design §TextSlotView）。
///
/// mount は初回 `ShowSurface` で遅延生成されるため、それ以前は取得できない（`text_slot_view` が
/// `None`）。呼び手は表示確立後に取得するか再取得を試みる（runtime 前提条件）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextSlotView {
    /// 予約済み text 層スロット entity（`Name("emo-text-layer-slot")`・内容なしの seam）。
    slot: Entity,
    /// スロットが属する装着先の窓 Entity。
    window: Entity,
    /// バルーン/シェル surface の **native 原寸**（k 適用前・取得時点のスナップショット）。
    surface_size: (u32, u32),
    /// 表示中の**物理寸**＝`scaled_extent(applied, surface_size)`（丸め権威を通した値・要件 3.1）。
    ///
    /// 構築時に [`ScaleRatio::scaled_extent`] で確定させる（`scale` からの再計算を呼び手に許さない）。
    physical_size: (u32, u32),
    /// バルーン surface と同一の合成スケール k（**実適用値**・要件 1.2）。
    scale: f32,
}

impl TextSlotView {
    /// 予約済み text 層スロット entity（emo-text-layer が描画を装着する先）。
    pub fn slot(&self) -> Entity {
        self.slot
    }

    /// スロットが属する装着先の窓 Entity。
    pub fn window(&self) -> Entity {
        self.window
    }

    /// バルーン surface の **native 原寸**（k 適用**前**の合成外形・要件 1.2 の照会契約）。
    ///
    /// 表示中の**物理寸ではない**。物理寸は丸め権威 [`ScaleRatio::scaled_extent`] を通した
    /// `scaled_extent(scale(), surface_size())` である（下流の照合式
    /// `GetClientRect ≒ surface_size × scale`・design §State Management）。k=1.0 の窓では両者が
    /// 一致するため、k 導入前の観測値（＝供給面寸）とも等しい。
    pub fn surface_size(&self) -> (u32, u32) {
        self.surface_size
    }

    /// 表示中の**物理寸**（窓 client がこの値と一致すべき唯一の寸法・要件 3.1/4.2）。
    ///
    /// `scaled_extent(applied, surface_size())` を**丸め権威**
    /// [`ScaleRatio::scaled_extent`] 経由で構築時に確定させた値である。呼び手が
    /// [`scale`] と [`surface_size`] から掛け算で復元することを想定していない——
    /// [`ScaleRatio::as_f32`] は照会用の出口ビューであり、その doc が明記するとおり
    /// **寸法・画素演算に使ってはならない**。既約有理 `num/den` が f32 で非厳密になる k
    /// （例 112dpi／author 96 ＝ 7/6）では積が端数ちょうど 0.5 の直下へ落ち、
    /// round half away from zero が切り下がって権威と 1px 食い違う（27px → 権威 32・f32 経由 31）。
    /// 窓 client を 1px 小さく書くとべき等 skip がその誤差を恒久化するため、消費点は必ず本値を使う。
    ///
    /// この禁止の**唯一の既知の例外は emo-text `ScaleContract::physical_extent`（文字供給面の
    /// 確保寸）**であり、2026-08-14 の裁定（spec `areka-P0-scale-exact-rational`・登記は emo-text
    /// `region.rs`）に基づく。**この例外を他の用途へ拡大してはならない**——例外は供給面寸の 1 点に
    /// 限られ、本値のような**窓 client 寸**を [`scale`] から掛け算で復元することは従来どおり禁止である。
    ///
    /// k=1.0 の窓では [`surface_size`] と一致する（恒等）。
    ///
    /// [`scale`]: Self::scale
    /// [`surface_size`]: Self::surface_size
    pub fn physical_size(&self) -> (u32, u32) {
        self.physical_size
    }

    /// バルーン surface と同一の合成スケール k（**実際に表示へ適用中の値**・要件 1.2）。
    ///
    /// かつてはコンパイル時定数 1.0（`CURRENT_COMPOSE_SCALE`）を恒常で返していたが、DPI 追従表示の
    /// 導入で **`PresentTarget.applied`（表示成立点でのみ更新される単一真実源）の写し**へ変わった。
    /// 窓 DPI ＝ author_dpi なら 1.0、192dpi／author 96 なら 2.0 を返す。下流（
    /// `collision-dpi-hittest` の ÷k・`emo-text-layer` の行寸）はこの値を実適用 k として参照してよい。
    pub fn scale(&self) -> f32 {
        self.scale
    }
}

impl EmoPresenter {
    /// target の予約 text 層スロットへの読み取り専用の到達手段（mount 未生成なら `None`・R9.1/9.2）。
    ///
    /// mount（と供給面）は初回 `ShowSurface` で原寸確定後に遅延生成されるため、未登録 target・
    /// 初回表示確立前は取得不可（`None`）である。呼び手（結線側）は表示確立後に取得するか再取得を
    /// 試みる。返る値はスナップショット（読み取り専用 view）で、スロット状態は変更できない。
    ///
    /// # 取得条件（k 導入後・要件 1.2）
    ///
    /// mount／供給面の存在に加えて **表示が一度成立していること**（`applied`／`native_size` が確定
    /// していること）を条件とする。`scale()` は実適用 k、`surface_size()` は native 原寸を返す契約で
    /// あり、いずれも表示成立点でしか確定しないためである——供給面だけ生成できて upload に失敗した
    /// ような中間状態で「k=1.0・供給面寸」という**実態のない値**を返さない（無言の縮退を作らない）。
    pub fn text_slot_view(&self, target: TargetId) -> Option<TextSlotView> {
        let t = self.targets.get(&target)?;
        let mount = t.mount.as_ref()?;
        // 供給面の遅延生成前は表示未確立（既存契約の維持）。
        t.chain.as_ref()?;
        // 実適用 k と native 原寸は表示成立点でのみ確定する（照会値＝実適用値の担保）。
        let applied = t.applied?;
        let surface_size = t.native_size?;
        Some(TextSlotView {
            slot: mount.text_slot(),
            window: t.window,
            surface_size,
            // 物理寸は**丸め権威**で確定させる（`as_f32` 経由の掛け算は 1px 食い違う・D4）。
            physical_size: applied.scaled_extent(surface_size.0, surface_size.1),
            scale: applied.as_f32(),
        })
    }

    /// target の**窓 client 物理寸**（＝実適用 k を丸め単一権威に通した値・要件 3.1/4.2）。
    ///
    /// `scaled_extent(applied, native_size)` を [`ScaleRatio::scaled_extent`] 経由で計算する。
    /// 窓 client を合わせる消費点（`emo2_boot` の resnap／DPI 追従フェーズ）はこの照会だけを見れば
    /// よく、native 原寸と物理寸を**取り違えようがない**——[`TextSlotView`] 経由だと
    /// [`TextSlotView::surface_size`]（native）と [`TextSlotView::physical_size`]（物理）が
    /// 隣り合って生えており、消費点での 1 トークンの取り違えが「窓が原寸へ引き戻される」
    /// 静かな欠陥になる。本照会はその選択肢を消費点から取り除く。
    ///
    /// **[`applied_scale`] から掛け算で復元してはならない**——`as_f32` は照会用の出口ビューであり、
    /// 既約分母が 2 冪でない k（例 112dpi／author 96 ＝ 7/6）では f32 の積が端数ちょうど 0.5 の
    /// 直下へ落ち、権威と 1px 食い違う（native 27px → 権威 32・f32 経由 31）。
    ///
    /// 表示が一度も成立していない target・未登録 target は `None`（[`applied_scale`] と同じ規律で、
    /// 未確定を原寸や 1.0 倍で塗り潰さない）。値は [`TextSlotView::physical_size`] と常に一致する
    /// （同一の `applied`／`native_size` から同一の権威で導くため）。
    ///
    /// [`applied_scale`]: Self::applied_scale
    pub fn target_physical_size(&self, target: TargetId) -> Option<(u32, u32)> {
        let t = self.targets.get(&target)?;
        let applied = t.applied?;
        let native = t.native_size?;
        Some(applied.scaled_extent(native.0, native.1))
    }

    /// 下流照会契約（要件 1.2）: いま**実際に表示へ適用中**の k。
    ///
    /// 単一真実源は [`PresentTarget::applied`] で、その更新点は表示成立点ただ 1 箇所である——ゆえに本照会が
    /// 返す値は常に「いま画面に載っている絵に実際に掛かった k」であり、「導出したが適用に失敗した k」が
    /// 漏れることはない（要件 4.4 の失敗経路は `applied` を書かずに early return する）。
    ///
    /// 表示が一度も成立していない target・未登録 target は `None`。「まだ何も適用していない」を 1.0 で
    /// 塗り潰さない（1.0 は等倍という**適用結果**であって未確定の別名ではない）。
    ///
    /// 同じ値は [`TextSlotView::scale`] からも読める（あちらは text 層向けにスロット情報と束ねた
    /// スナップショット経路）。
    ///
    /// # 当たり判定の ÷k はこの f32 を経由しない
    ///
    /// `as_f32` は照会用の**出口ビュー**であり、寸法・画素演算に使ってはならない（[`target_physical_size`]
    /// の doc が述べる 1px 食い違いと同じ理由）。`areka-P0-collision-dpi-hittest` の点÷k は f32 を一切
    /// 経由せず、有理値のまま [`Self::hit_region_client`] が内部で厳密に消費する。厳密値そのものが要る
    /// 呼び手（実機 probe の期待ゲート等）は [`Self::applied_ratio`] を使う。
    ///
    /// この禁止の**唯一の既知の例外は emo-text `ScaleContract::physical_extent`（文字供給面の
    /// 確保寸）**であり、2026-08-14 の裁定（spec `areka-P0-scale-exact-rational`・登記は emo-text
    /// `region.rs`）に基づく。**この例外を他の用途へ拡大してはならない**——例外は供給面寸の 1 点に
    /// 限られ、**窓 client 寸**をこの f32 から復元する禁止（[`target_physical_size`]）は据え置く。
    ///
    /// [`target_physical_size`]: Self::target_physical_size
    pub fn applied_scale(&self, target: TargetId) -> Option<f32> {
        Some(self.targets.get(&target)?.applied?.as_f32())
    }

    /// 実適用 k の**厳密照会**（既約有理のまま返す・f32 版 [`Self::applied_scale`] と併存）。
    ///
    /// 真実源・確定点・`None` の意味はすべて [`Self::applied_scale`] と同一（同じ `applied` を読む）で、
    /// 違いは表現だけである——本照会は丸めも近似も挟まない [`ScaleRatio`] を返す。実機サインオフの
    /// **期待ゲート**（「この水準では k がちょうど 5/4 であること」を hard assert する用途・要件 4.1）は
    /// f32 比較では 1 ulp の揺れを議論する羽目になるため、こちらを使う。
    ///
    /// 判定経路（[`Self::hit_region_client`]）は本照会を**経由しない**——判定は私有 `applied` を直読する
    /// （公開面を判定の依存に据えると、照会と判定で別の k を見る余地が生まれる）。
    pub fn applied_ratio(&self, target: TargetId) -> Option<ScaleRatio> {
        self.targets.get(&target)?.applied
    }

    /// target が**最後に確立した**サーフェス id（CurrentSurfaceRead・R3.1-3.3）。
    ///
    /// 画面の絵ではなく表示確立の結果を返す（α 非依存）。未表示（一度も `ShowSurface` していない）・
    /// `Hide` 済み・空合成へ縮退した場合、および未登録 target は `None`。単一真実源は
    /// `PresentTarget.current_surface_id`（`ComposeKey` から導出しない・design §CurrentSurfaceRead
    /// State Management）。既存の表示ロジックへ分岐を足さない additive な読み取りのみ（R3.4）。
    ///
    /// # 「確立」は「可視」を意味しない（`areka-P0-balloon-visibility` Requirement 6.2/6.8）
    ///
    /// [`VisibilityOwnership::External`] の target では `ShowSurface` が確立のみを行い可視化しないため、
    /// 本照会が `Some` でも画面には出ていないことがある。可視か否かは [`Self::target_visible`] が答える
    /// ——面 ID の所有（本照会）と可視性の所有（`target_visible`）は直交する別軸である。
    ///
    /// [`VisibilityOwnership::External`]: super::VisibilityOwnership::External
    pub fn current_surface_id(&self, target: TargetId) -> Option<u32> {
        self.targets.get(&target)?.current_surface_id
    }

    /// target がいま**可視か**（`areka-P0-balloon-visibility` Requirement 6.8 の可視性の単一真実源）。
    ///
    /// 未登録 target は `None`、登録済みなら `Some(可視か)` を返す。真実源は `PresentTarget.visible`
    /// で、その更新点は「表示確立点の可視化手順（[`VisibilityOwnership::CommandDriven`] のみ）」・
    /// [`Self::show_target`]・`Hide`／全透明退化の 3 系統だけである。
    ///
    /// # なぜ照会を生やすのか（第 2 の帳簿を作らせない）
    ///
    /// 可視性を判断する層（バルーン可視性コントローラ）が「自分が最後に何を発行したか」を自前で覚えると、
    /// 明示指令（`\b[-1]`）や全透明退化のような**自分が発行していない遷移**で帳簿が実状態から乖離する。
    /// 判断側は毎フレーム本照会を読み、presenter 1 箇所を真実源とする。
    ///
    /// 読み取り専用であり、表示状態を一切変更しない（additive な照会）。
    ///
    /// [`VisibilityOwnership::CommandDriven`]: super::VisibilityOwnership::CommandDriven
    pub fn target_visible(&self, target: TargetId) -> Option<bool> {
        Some(self.targets.get(&target)?.visible)
    }

    /// target の表示中画素を CPU へ読み戻す（R6.2/R8.3・検証・将来の直読みヒットテスト基盤）。
    ///
    /// 未装着、または供給面未生成（未表示）なら [`PresentError::TargetNotAttached`] を返す。
    pub fn read_back(&self, target: TargetId) -> Result<Vec<u8>, PresentError> {
        let Some(t) = self.targets.get(&target) else {
            tracing::error!(?target, "read_back: 未装着ターゲット");
            return Err(PresentError::TargetNotAttached(target));
        };
        match t.chain.as_ref() {
            Some(chain) => chain.read_back(),
            None => {
                tracing::error!(?target, "read_back: 供給面が未生成（未だ表示していない）");
                Err(PresentError::TargetNotAttached(target))
            }
        }
    }
}
