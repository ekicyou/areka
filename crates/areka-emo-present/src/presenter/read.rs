//! 読み取り専用の照会契約——text 層スロット view（`TextSlotView`）・物理寸・実適用 k・現サーフェス id・
//! 表示画素の読み戻し。いずれも表示成立点で確定した値を返すのみで、表示状態を変更しない。

use super::{EmoPresenter, Entity, NeverShown, PresentError, ScaleRatio, TargetId};

/// 予約 text 層スロットへの読み取り専用の到達手段（emo-text-layer が消費する）。
///
/// [`EmoPresenter::text_slot_view`] が返すスナップショット値。フィールドは非公開で accessor のみを
/// 公開し、スロット状態の変更手段を持たない。装着 API 形（emo-present が描画物を受け取る）は emo-text
/// の描画型が本 crate へ逆流して依存方向（emo-present → emo-text 禁止）と衝突するため採らない。
///
/// mount は初回 `ShowSurface` で遅延生成されるため、それ以前は取得できない（`text_slot_view` が
/// `None`）。呼び手は表示確立後に取得するか再取得を試みる。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextSlotView {
    /// 予約済み text 層スロット entity（`Name("emo-text-layer-slot")`・内容なしの seam）。
    slot: Entity,
    /// スロットが属する装着先の窓 Entity。
    window: Entity,
    /// バルーン/シェル surface の **native 原寸**（k 適用前・取得時点のスナップショット）。
    surface_size: (u32, u32),
    /// 表示中の**物理寸**＝`scaled_extent(applied, surface_size)`（丸め権威を通した値）。
    ///
    /// 構築時に [`ScaleRatio::scaled_extent`] で確定させる（`scale` からの再計算を呼び手に許さない）。
    physical_size: (u32, u32),
    /// バルーン surface と同一の合成スケール k（**実適用値**）。
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

    /// バルーン surface の **native 原寸**（k 適用**前**の合成外形）。
    ///
    /// 表示中の物理寸ではない。物理寸は丸め権威 [`ScaleRatio::scaled_extent`] を通した
    /// `scaled_extent(scale(), surface_size())` である（下流の照合式
    /// `GetClientRect ≒ surface_size × scale`）。k=1.0 の窓では両者が一致する。
    pub fn surface_size(&self) -> (u32, u32) {
        self.surface_size
    }

    /// 表示中の**物理寸**（窓 client がこの値と一致すべき唯一の寸法）。
    ///
    /// `scaled_extent(applied, surface_size())` を丸め権威 [`ScaleRatio::scaled_extent`] 経由で構築時に
    /// 確定させた値。呼び手が [`scale`] と [`surface_size`] から掛け算で復元してはならない:
    /// [`ScaleRatio::as_f32`] は照会用の出口ビューで、寸法・画素演算に使えない。既約有理 `num/den` が
    /// f32 で非厳密になる k（例 112dpi／author 96 ＝ 7/6）では積が端数ちょうど 0.5 の直下へ落ち、
    /// round half away from zero が切り下がって権威と 1px 食い違う（27px → 権威 32・f32 経由 31）。
    /// 窓 client を 1px 小さく書くとべき等 skip がその誤差を恒久化するため、消費点は必ず本値を使う。
    ///
    /// この禁止の唯一の既知の例外は emo-text `ScaleContract::physical_extent`（文字供給面の確保寸）で、
    /// 登記は emo-text `region.rs`。例外は供給面寸の 1 点に限り、窓 client 寸を [`scale`] から掛け算で
    /// 復元することは禁止のままである。
    ///
    /// k=1.0 の窓では [`surface_size`] と一致する（恒等）。
    ///
    /// [`scale`]: Self::scale
    /// [`surface_size`]: Self::surface_size
    pub fn physical_size(&self) -> (u32, u32) {
        self.physical_size
    }

    /// バルーン surface と同一の合成スケール k（**実際に表示へ適用中の値**）。
    /// 窓 DPI ＝ author_dpi なら 1.0、192dpi／author 96 なら 2.0 を返す。下流（`emo-text-layer` の
    /// 行寸）はこの値を実適用 k として参照してよい。
    pub fn scale(&self) -> f32 {
        self.scale
    }
}

impl EmoPresenter {
    /// target の予約 text 層スロットへの読み取り専用の到達手段（mount 未生成なら `None`）。
    ///
    /// mount は初回 `ShowSurface` で原寸確定後に遅延生成されるため、未登録 target・初回表示確立前は
    /// 取得不可。返る値はスナップショットで、スロット状態は変更できない。
    ///
    /// # 取得条件
    ///
    /// 装着の存在に加えて表示が一度成立していること（`applied`／`native_size` が確定していること）を
    /// 条件とする。`scale()` は実適用 k、`surface_size()` は native 原寸を返す契約で、いずれも表示成立点
    /// でしか確定しない。中間状態で「k=1.0・原寸」という実態のない値を返さない（無言の縮退を作らない）。
    pub fn text_slot_view(&self, target: TargetId) -> Option<TextSlotView> {
        let t = self.targets.get(&target)?;
        // 装着の遅延生成前は表示未確立。
        let mount = t.mount.as_ref()?;
        // 実適用 k と native 原寸は表示成立点でのみ確定する（照会値＝実適用値の担保）。
        let applied = t.applied?;
        let surface_size = t.native_size?;
        Some(TextSlotView {
            slot: mount.text_slot(),
            window: t.window,
            surface_size,
            // 物理寸は**丸め権威**で確定させる（`as_f32` 経由の掛け算は 1px 食い違う）。
            physical_size: applied.scaled_extent(surface_size.0, surface_size.1),
            scale: applied.as_f32(),
        })
    }

    /// target の**窓 client 物理寸**（＝実適用 k を丸め単一権威に通した値）。
    ///
    /// `scaled_extent(applied, native_size)` を [`ScaleRatio::scaled_extent`] 経由で計算する。窓 client を
    /// 合わせる消費点（`areka` の `runtime::frame` の resnap／DPI 追従フェーズ）はこの照会だけを見ればよく、
    /// native 原寸と物理寸を取り違えようがない。[`TextSlotView`] 経由だと
    /// [`TextSlotView::surface_size`]（native）と [`TextSlotView::physical_size`]（物理）が隣り合っており、
    /// 1 トークンの取り違えが「窓が原寸へ引き戻される」静かな欠陥になる。
    ///
    /// [`applied_scale`] から掛け算で復元してはならない。`as_f32` は照会用の出口ビューで、既約分母が
    /// 2 冪でない k（例 112dpi／author 96 ＝ 7/6）では f32 の積が端数ちょうど 0.5 の直下へ落ち、権威と
    /// 1px 食い違う（native 27px → 権威 32・f32 経由 31）。
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

    /// 下流照会契約: いま**実際に表示へ適用中**の k。
    ///
    /// 単一真実源は `PresentTarget::applied` で、更新点は表示成立点ただ 1 箇所。ゆえに返す値は常に
    /// 「いま画面に載っている絵に実際に掛かった k」で、「導出したが適用に失敗した k」が漏れることはない
    /// （失敗経路は `applied` を書かずに early return する）。
    ///
    /// 表示が一度も成立していない target・未登録 target は `None`。「まだ何も適用していない」を 1.0 で
    /// 塗り潰さない（1.0 は等倍という適用結果であって未確定の別名ではない）。同じ値は
    /// [`TextSlotView::scale`] からも読める（text 層向けにスロット情報と束ねたスナップショット経路）。
    ///
    /// # 当たり判定の ÷k はこの f32 を経由しない
    ///
    /// `as_f32` は照会用の出口ビューで、寸法・画素演算に使ってはならない（[`target_physical_size`] の
    /// doc が述べる 1px 食い違いと同じ理由）。当たり判定の点÷k は f32 を経由せず、有理値のまま
    /// [`Self::hit_region_client`] が内部で厳密に消費する。厳密値そのものが要る呼び手（実機 probe の
    /// 期待ゲート等）は [`Self::applied_ratio`] を使う。
    ///
    /// この禁止の唯一の既知の例外は emo-text `ScaleContract::physical_extent`（文字供給面の確保寸）で、
    /// 登記は emo-text `region.rs`。例外は供給面寸の 1 点に限り、窓 client 寸をこの f32 から復元する禁止
    /// （[`target_physical_size`]）は据え置く。
    ///
    /// [`target_physical_size`]: Self::target_physical_size
    pub fn applied_scale(&self, target: TargetId) -> Option<f32> {
        Some(self.targets.get(&target)?.applied?.as_f32())
    }

    /// 実適用 k の**厳密照会**（既約有理のまま返す・f32 版 [`Self::applied_scale`] と併存）。
    ///
    /// 真実源・確定点・`None` の意味はすべて [`Self::applied_scale`] と同一（同じ `applied` を読む）で、
    /// 違いは表現だけである——本照会は丸めも近似も挟まない [`ScaleRatio`] を返す。実機サインオフの
    /// **期待ゲート**（「この水準では k がちょうど 5/4 であること」を hard assert する用途）は
    /// f32 比較では 1 ulp の揺れを議論する羽目になるため、こちらを使う。
    ///
    /// 判定経路（[`Self::hit_region_client`]）は本照会を**経由しない**——判定は私有 `applied` を直読する
    /// （公開面を判定の依存に据えると、照会と判定で別の k を見る余地が生まれる）。
    pub fn applied_ratio(&self, target: TargetId) -> Option<ScaleRatio> {
        self.targets.get(&target)?.applied
    }

    /// target が最初の面を表示しないまま決まった理由。
    ///
    /// 表示が一度成り立った target と、面の指令を受けて結果がまだの target は `None`。
    /// 未登録と面の指令を受けていない target は `Some(NotRequested)`。
    pub fn never_shown(&self, target: TargetId) -> Option<NeverShown> {
        let Some(t) = self.targets.get(&target) else {
            return Some(NeverShown::NotRequested);
        };
        if t.applied.is_some() {
            None
        } else if !t.surface_requested {
            Some(NeverShown::NotRequested)
        } else {
            t.never_shown
        }
    }

    /// target が**最後に確立した**サーフェス id。
    ///
    /// 画面の絵ではなく表示確立の結果を返す（α 非依存）。未表示（一度も `ShowSurface` していない）・
    /// `Hide` 済み・空合成へ縮退した場合、および未登録 target は `None`。単一真実源は
    /// `PresentTarget.current_surface_id`（`ComposeKey` から導出しない）。
    ///
    /// # 「確立」は「可視」を意味しない
    ///
    /// [`VisibilityOwnership::External`] の target では `ShowSurface` が確立のみを行い可視化しないため、
    /// 本照会が `Some` でも画面には出ていないことがある。可視か否かは [`Self::target_visible`] が答える
    /// ——面 ID の所有（本照会）と可視性の所有（`target_visible`）は直交する別軸である。
    ///
    /// [`VisibilityOwnership::External`]: super::VisibilityOwnership::External
    pub fn current_surface_id(&self, target: TargetId) -> Option<u32> {
        self.targets.get(&target)?.current_surface_id
    }

    /// target の**出番の世代**。未登録は `None`。
    ///
    /// 外から所有される対象を見えていない状態から [`Self::show_target`] で出すたびに 1 つ進む
    /// （命令で見える対象は 0 のまま）。届けの相が窓の知らせに載せて seriko へ渡す。
    pub fn stage_generation(&self, target: TargetId) -> Option<u64> {
        Some(self.targets.get(&target)?.stage_generation)
    }

    /// target の**ずれ**（合図 `MoveShift` で最後に受けた値・作者の座標）。未登録・まだ受けていない target は `(0, 0)`。
    pub fn move_shift(&self, target: TargetId) -> (i64, i64) {
        self.targets.get(&target).map_or((0, 0), |t| t.move_shift)
    }

    /// target がいま**可視か**（可視性の単一真実源）。
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
    /// [`VisibilityOwnership::CommandDriven`]: super::VisibilityOwnership::CommandDriven
    pub fn target_visible(&self, target: TargetId) -> Option<bool> {
        Some(self.targets.get(&target)?.visible)
    }

    /// target が表示に渡した**原寸の合成バイト列**（表示中エントリの `ComposedSurface`・premultiplied
    /// BGRA・`stride=width*4`）を返す。
    ///
    /// 表示面（wintf の `CompositionDrawingSurface`）は書き込み専用で読み戻さない。返るのは合成メモが
    /// 保持する原寸バイト列そのもので、k の値に依らず同一。寿命はメモの寿命に等しいので、消費者は表示
    /// 直後に読む。失敗はどれも `error!` ＋ `Err`: 未装着は [`PresentError::TargetNotAttached`]・
    /// 未表示は [`PresentError::NotShown`]・表示中エントリの消失（LRU 追い出し・`InvalidateCache` 後）は
    /// [`PresentError::EntryEvicted`]。
    pub fn read_back(&self, target: TargetId) -> Result<Vec<u8>, PresentError> {
        let Some(t) = self.targets.get(&target) else {
            tracing::error!(?target, "read_back: 未装着ターゲット");
            return Err(PresentError::TargetNotAttached(target));
        };
        let Some((surface_id, binds, pattern)) = t.last_show.as_ref() else {
            tracing::error!(
                ?target,
                "read_back: 未だ表示していない（表示成立点を通っていない）"
            );
            return Err(PresentError::NotShown(target));
        };
        // `get` は最近使用順を動かさない（LRU を打ち直すのは `show.rs` の `touch` だけ）。
        match t.cache.get(*surface_id, binds, pattern) {
            Some(entry) => Ok(entry.composed.bytes().to_vec()),
            None => {
                tracing::error!(
                    ?target,
                    surface_id,
                    "read_back: 表示中エントリがメモに無い（追い出し／InvalidateCache 後）"
                );
                Err(PresentError::EntryEvicted {
                    target,
                    surface_id: *surface_id,
                })
            }
        }
    }
}

#[cfg(test)]
#[path = "read_move_shift_tests.rs"]
mod move_shift_tests;
