//! `FrameBudget`: 毎フレーム経路における新規確保・容量成長の**唯一の計数シーム**。
//!
//! 「定常状態で表示バッファの新規確保が起きていない」（Requirement 3.1・判定式⑶）を、推測でなく
//! 数値で言い切るための計数器である。確保の発生点は 4 つ（design.md §FrameBudget の席一覧と 1:1）:
//!
//! | 発生点 | 増えるフィールド | 何の確保か |
//! |---|---|---|
//! | [`AllocSite::ComposeDst`] | `alloc_compose_dst` | native 合成先 |
//! | [`AllocSite::ResampleDst`] | `alloc_resample_dst` | 表示バッファ（リサンプル先） |
//! | [`AllocSite::Xmap`] | `alloc_xmap` | リサンプル作業領域（x 軸写像表） |
//! | [`AllocSite::Mask`] | `alloc_mask` | 当たり判定マスク |
//!
//! # なぜ 1 箇所で数えるのか（design.md D6）
//!
//! プロセス全体の確保を捕まえる `#[global_allocator]` 差し替えは、emo-compose の既存予算檻が
//! **棄却済み**の方針である（`areka-emo-compose/src/golden_tests_determinism_budget_tests.rs`
//! の「アプローチ (B)」——プロセス全体を汚染し既存テストを不安定化しうる）。代わりに、
//! 確保が起き得る発生点そのものを名前で数える。数えられるのは「この経路で意図している確保」
//! だけであり、それが観測したい対象と一致する。
//!
//! # 計数の意味は段階で変わらない（増分と累積の関係）
//!
//! 器は計数に加えて**再利用席**（合成先の常設席・リサンプル作業領域の席・マスクの輪番）を持つ
//! （task 5.2）。ゆえに [`FrameBudget::note_alloc`] は「呼び手が確保したと申告したとき」ではなく
//! **「席の再利用が成立せず結局確保したとき」**に席メソッドの内部から呼ばれる。どちらの段でも
//! 計数の意味は「新規確保・容量成長が 1 回起きた」で変わらないため、増分・累積の意味論と
//! 本モジュールの外向き API（[`FrameBudget::take_delta`]・[`FrameBudget::cumulative`]）は
//! 席の導入で動いていない。
//!
//! # 席の一覧（design.md §FrameBudget・D2・Flow 2 と 1:1）
//!
//! | 席 | 是正対象 | 貸し出し口 |
//! |---|---|---|
//! | 合成先の常設席（`ComposedSurface`） | A1 | [`FrameBudget::native_scratch`]・恒等 k の交代は [`FrameBudget::swap_native_scratch`] |
//! | リサンプル作業領域の席（`ResampleScratch`） | A3 | [`FrameBudget::resample_native_into`] |
//! | マスクの輪番（`Option<Arc<AlphaMask>>`・2 スロット） | A4/A7 | [`FrameBudget::regenerate_mask`] |
//!
//! 表示バッファ（A2/A6）だけは本器が所有しない——所有はキャッシュ側にあり、追い出しエントリの
//! 容量を [`ComposeCache::take_recycled`] で回収して回す。本器は
//! [`FrameBudget::display_buffer`] でその仲介と計数だけを担う。
//!
//! # 何をもって「確保した」と数えるか（席ごとに 2 通り）
//!
//! ## ⑴ 容量を席から直接読める席は、呼び出しの前後の差で数える（厳密）
//!
//! リサンプル作業領域は [`ResampleScratch::capacity`] で**席そのものの容量**を読める。`Vec` の
//! 容量は再確保でしか増えないため、`resample_with` の前後で容量を読み比べれば
//! 「増えた＝確保が起きた／変わらない＝起きていない」が厳密に決まる。過大にも過小にも振れない。
//!
//! この形にしてある理由は、器側で高水位を覚える形が**席の差し替えを 1 件も検出できない**
//! ためである（§到達済み寸法の落とし穴）。
//!
//! ## ⑵ 容量を読めない席は、**その実体に紐づけた**到達済み寸法（高水位）で数える
//!
//! `ComposedSurface` のバイト列と `AlphaMask` の詰めバイト列は容量を公開していない。ゆえに
//! これらの席は**到達済みの最大長**を実体と分離不能に束ねて持ち、要求がそれを超えたときにだけ
//! 計数する。
//!
//! この規則は片側にのみ誤差を持つ: `Vec` の容量は一度到達した長さを下回らない（`clear`＋`resize`
//! も `reserve` も容量を縮めない）ため、**到達済み以下の要求は確保が起き得ない**——見逃しは
//! 構造的に起こらない。逆に、到達済みを超える要求は `Vec` の余裕次第で実際には確保が起きない
//! ことがあり、その場合は 1 件多く数える。**安全側（多めに数える）へ倒してある**のは、
//! 定常アロケーション 0（Requirement 3.1）が「数え漏れによる偽の成立」で通ることを防ぐためである。
//!
//! ## 到達済み寸法の落とし穴（レビューの変異検査が暴いた欠陥）
//!
//! 高水位を「実体」ではなく「役割」に紐づけると、**席の実体を差し替える改変が計数へ 1 件も
//! 現れない**——高水位が前の実体の値を覚えたままなので、確保し直した席が同じ寸法へ伸びても
//! 「到達済み以下」と判定されるためである。ゆえに本モジュールでは
//!
//! - 高水位は必ず席の実体と同じ入れ物に置き、実体と一緒に入れ替える（[`seat::SurfaceSeat`]・
//!   [`seat::MaskRotation`]・表示バッファの [`FrameBudget::swap_native_scratch`]）
//! - 容量を読める席は高水位を持たず、席から直接読む（[`seat::XmapSeat`]）
//!
//! の 2 つだけを許す。特にリサンプル作業領域は**高水位を出力幅で代用していた**ため、
//! 「毎回まっさらな作業領域で呼ぶ」改変（席への代入・使い捨てローカルの受け渡しの両方）が
//! 檻を 1 本も赤にしなかった。⑴ の形はその 2 種をどちらも殺す。
//!
//! # 席が期待どおり再利用できない境界（design.md §Error Handling）
//!
//! 回収エントリが無い・マスク輪番の空きスロットが単独所有でない、といった境界では**確保する**。
//! ただし**黙っては確保しない**——必ず該当カウンタを増やす。隠れた縮退を作らないための規律で
//! あり、ログ無し失敗経路の禁止（steering）を本経路では計数フィールドで担保している。
//!
//! # 増分と累積を両方出す理由（Requirement 1.3）
//!
//! - **増分**（[`BudgetDelta`]）: 適用 1 回分。perf サマリ行の `alloc_*` フィールドへそのまま載り、
//!   判定スクリプトが行単位で「この適用で確保が起きたか」を機械集計する
//! - **累積**（[`BudgetCounters`]）: 取り出しでリセットされない run 全体の合計。テストが
//!   「ウォームアップ後の N 反復で 1 件も増えていない」を主張する読み取り口になる
//!
//! 両者は同じ申告から同時に更新されるため、適用ごとの増分の総和は必ず累積に一致する。
//!
//! # 毎フレーム経路の上に載る（計数自体が負荷にならない）
//!
//! 計数は整数の飽和加算のみで、確保もログもロックも行わない。ログ設定も参照しない——
//! 表示経路がログ設定で分岐しないという構造（Requirement 1.5）は計数側でも保つ。
//!
//! # design.md の Service Interface と実装の差異（**task 5.3 が design.md 側を直すこと**）
//!
//! design.md「FrameBudget（`presenter/budget.rs`・新設）」の Service Interface ブロック
//! （design.md:322-345）は task 5.2 実装より前に書かれており、次の 3 点で実形と食い違っている。
//! いずれも実装側の判断で形を変えたものであり、**正しいのは実装の形**である（理由は各メソッドの
//! doc に書いてある）。**design.md を実形へ更新するのは task 5.3 の責務**であり、本 task は
//! design.md を編集しない。
//!
//! | design.md | 宣言されている形 | 実形 | 変えた理由 |
//! |---|---|---|---|
//! | :338 | `fn native_scratch(&mut self) -> &mut ComposedSurface` | `fn native_scratch<R>(&mut self, borrow: impl FnOnce(&mut ComposedSurface) -> R) -> R` | 合成の外形は `compose_into` の内側で決まるため、伸長の観測は「席を使い終えた直後」にしか置けない。`&mut` を裸で返すと観測点が呼び手の記憶任せになり、忘れれば黙って確保する経路ができる（[`FrameBudget::native_scratch`] の doc） |
//! | :342 | `fn regenerate_mask(&mut self, bytes: &[u8], w: u32, h: u32, stride: u32) -> Arc<AlphaMask>` | 第 1 引数に `retired: Option<Arc<AlphaMask>>` を取る | 輪番の空きスロットは「直前の適用で表示に使ったマスク」であり、それは表示バッファと原子対でキャッシュに入っている。回収エントリから外したマスクを器へ戻す口が無いと 2 スロットの輪番が回らない（[`FrameBudget::regenerate_mask`] の doc） |
//! | :336-345 | `swap_native_scratch` が**ブロックに存在しない** | [`FrameBudget::swap_native_scratch`] を持つ | design.md 本文の Flow 2 は恒等 k で合成先席と表示バッファを交代させる（コピーも確保もしない）と定めている。その交代を行う口が Service Interface ブロックにだけ落ちていた |
//!
//! [`ComposeCache::take_recycled`]: crate::cache::ComposeCache::take_recycled

use std::sync::Arc;

use areka_emo_compose::scale::{ResampleScratch, resample_with};
use wintf::ecs::widget::bitmap_source::AlphaMask;

use super::{ComposedSurface, ScaleRatio};
use crate::cache::CacheEntry;

/// 確保の発生点（design.md §FrameBudget の席一覧・perf サマリ行の `alloc_*` と 1:1）。
///
/// 列挙を増減させると perf サマリ行のフィールド集合が変わる＝判定スクリプトとの契約変更である。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AllocSite {
    /// native 合成先の新規確保／容量成長。
    ComposeDst,
    /// 表示バッファ（リサンプル先）の新規確保／容量成長（容量の回収が不成立の場合を含む）。
    ResampleDst,
    /// リサンプル作業領域（x 軸写像表）の新規確保／容量成長。
    Xmap,
    /// 当たり判定マスクの新規確保（輪番スロットの単独所有が不成立の場合を含む）。
    Mask,
}

impl AllocSite {
    /// 全発生点（走査の正本。檻はこれを回して「4 つある」ことごと固定する）。
    ///
    /// 読み手は現時点ではテストのみ（`budget_tests.rs`）。製品コードからの走査は task 6.1
    /// （定常アロケーション 0 の檻）が全発生点を回す形で入る。
    #[allow(dead_code)]
    pub(super) const ALL: [AllocSite; 4] = [
        AllocSite::ComposeDst,
        AllocSite::ResampleDst,
        AllocSite::Xmap,
        AllocSite::Mask,
    ];
}

/// 1 適用分の確保計数スナップショット（perf サマリ行の `alloc_*` フィールドの供給源）。
///
/// [`FrameBudget::take_delta`] が取り出しと同時に器側をリセットするため、この値は常に
/// 「直前の取り出しから今回までの 1 適用分」を表す。定常状態では全フィールドが 0 になる
/// ことが是正後の不変量である（Requirement 3.1）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct BudgetDelta {
    /// native 合成先の新規確保／容量成長。
    pub(super) alloc_compose_dst: u32,
    /// 表示バッファの新規確保／容量成長。
    pub(super) alloc_resample_dst: u32,
    /// リサンプル作業領域の新規確保／容量成長。
    pub(super) alloc_xmap: u32,
    /// 当たり判定マスクの新規確保。
    pub(super) alloc_mask: u32,
}

impl BudgetDelta {
    /// 発生点で引く（全発生点の走査用。名前つきフィールドと同じ値を返す）。
    ///
    /// 現時点の読み手はテストのみ（[`AllocSite::ALL`] を回す檻）。製品コードの
    /// perf サマリ行は名前つきフィールドを直接使う。
    #[allow(dead_code)]
    pub(super) fn count(&self, site: AllocSite) -> u32 {
        match site {
            AllocSite::ComposeDst => self.alloc_compose_dst,
            AllocSite::ResampleDst => self.alloc_resample_dst,
            AllocSite::Xmap => self.alloc_xmap,
            AllocSite::Mask => self.alloc_mask,
        }
    }

    /// 1 件加算する（飽和。1 適用で u32 を溢れさせる経路は無いが、値を巻き戻さない）。
    fn bump(&mut self, site: AllocSite) {
        let slot = match site {
            AllocSite::ComposeDst => &mut self.alloc_compose_dst,
            AllocSite::ResampleDst => &mut self.alloc_resample_dst,
            AllocSite::Xmap => &mut self.alloc_xmap,
            AllocSite::Mask => &mut self.alloc_mask,
        };
        *slot = slot.saturating_add(1);
    }
}

/// run 全体の累積計数（取り出しでリセットされない）。
///
/// 幅が [`BudgetDelta`]（u32）より広い u64 なのは、長時間走行（20 分超）の合計を素直に載せる
/// ためである。増分は 1 適用分ゆえ u32 で溢れない。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct BudgetCounters {
    /// native 合成先の累積。
    pub(super) alloc_compose_dst: u64,
    /// 表示バッファの累積。
    pub(super) alloc_resample_dst: u64,
    /// リサンプル作業領域の累積。
    pub(super) alloc_xmap: u64,
    /// 当たり判定マスクの累積。
    pub(super) alloc_mask: u64,
}

impl BudgetCounters {
    /// 発生点で引く（全発生点の走査用）。
    ///
    /// 現時点の読み手はテストのみ。累積を「全発生点で 0 のまま」と主張する檻は task 6.1 が置く。
    #[allow(dead_code)]
    pub(super) fn count(&self, site: AllocSite) -> u64 {
        match site {
            AllocSite::ComposeDst => self.alloc_compose_dst,
            AllocSite::ResampleDst => self.alloc_resample_dst,
            AllocSite::Xmap => self.alloc_xmap,
            AllocSite::Mask => self.alloc_mask,
        }
    }

    /// 1 件加算する（飽和。溢れても値を巻き戻さない＝計数を捏造しない）。
    fn bump(&mut self, site: AllocSite) {
        let slot = match site {
            AllocSite::ComposeDst => &mut self.alloc_compose_dst,
            AllocSite::ResampleDst => &mut self.alloc_resample_dst,
            AllocSite::Xmap => &mut self.alloc_xmap,
            AllocSite::Mask => &mut self.alloc_mask,
        };
        *slot = slot.saturating_add(1);
    }
}

/// 席と「確保したかどうかの観測」を**分離不能に**束ねる私有入れ物。
///
/// # なぜ入れ物へ閉じ込めるのか（変異検査が暴いた欠陥）
///
/// 到達済み寸法（高水位）を席と別のフィールドに置くと、**席だけを差し替える改変が計数へ 1 件も
/// 現れない**——高水位が前の実体の値を覚えたままなので、確保し直した席が同じ寸法へ伸びても
/// 「到達済み以下」と判定されるためである。実際、分離形で「席を毎回まっさらに起こす」変異を
/// 当てたところ、リサンプル作業領域の側は檻が 1 本も赤にならなかった。
///
/// 本入れ物はフィールドを私有にし、**席へ触れる唯一の道**を観測込みのメソッドにする。
/// 観測の形は席ごとに 2 通りある（本モジュール冒頭 §何をもって「確保した」と数えるか）:
///
/// - [`SurfaceSeat`]・[`MaskRotation`]: 容量を読めないので到達済み長を**実体と同じ入れ物に**
///   置き、実体を入れ替えるときは高水位も一緒に動かす
/// - [`XmapSeat`]: 高水位を**持たない**。[`ResampleScratch::capacity`] で席そのものの容量を
///   呼び出しの前後で読み比べる（`Vec` は再確保でしか容量が増えない＝厳密な観測）
mod seat {
    use std::sync::Arc;

    use wintf::ecs::widget::bitmap_source::AlphaMask;

    use super::{ComposedSurface, ResampleScratch, ScaleRatio, resample_with};

    /// 合成先の常設席（A1）＋到達済みバイト長。
    #[derive(Debug, Default)]
    pub(super) struct SurfaceSeat {
        surface: ComposedSurface,
        /// 到達済みバイト長（高水位）。`Vec` の容量は一度到達した長さを下回らないため、
        /// これ以下の要求では確保が起き得ない。
        reached: usize,
    }

    impl SurfaceSeat {
        /// 席を閉包へ貸す。閉包の中で到達済みを超えて伸びたら `true`（＝確保が起きた）。
        pub(super) fn lend<R>(
            &mut self,
            borrow: impl FnOnce(&mut ComposedSurface) -> R,
        ) -> (R, bool) {
            let produced = borrow(&mut self.surface);
            let len = self.surface.bytes().len();
            let grew = len > self.reached;
            if grew {
                self.reached = len;
            }
            (produced, grew)
        }

        /// 読み取り用の参照（リサンプル元）。
        pub(super) fn get(&self) -> &ComposedSurface {
            &self.surface
        }

        /// 恒等 k の交代（design.md Flow 2）。到達済みは実体に紐づく値ゆえ一緒に入れ替える。
        pub(super) fn swap(&mut self, buffer: &mut ComposedSurface, reached: &mut usize) {
            std::mem::swap(&mut self.surface, buffer);
            std::mem::swap(&mut self.reached, reached);
        }
    }

    /// リサンプル作業領域の席（A3）。**高水位を持たない**。
    ///
    /// # 観測は席そのものの容量で行う（高水位の代用が作った穴）
    ///
    /// 以前の形は到達済み**出力幅**（`out.width()`）を高水位に使っていた。出力幅は
    /// 「出力サーフェスの性質」であって作業領域の性質ではないため、この形は作業領域を
    /// 一度も観測していなかった——次の 2 種の改変がどちらも檻を 1 本も赤にしなかった:
    ///
    /// - `self.scratch = ResampleScratch::default();` を `resample_with` の直前に置く
    /// - 使い捨てのローカル `ResampleScratch` を作って `resample_with` へ渡す（席は不使用）
    ///
    /// 現在の形は [`ResampleScratch::capacity`] を呼び出しの前後で読み比べる。`Vec` の容量は
    /// **再確保でしか増えない**ので、増えた＝この呼び出しで写像表を確保し直した、が厳密に決まる。
    /// 上の 2 種はこれで両方死ぬ: 前者は毎回 0 から伸びるので毎回計数され（定常ゼロが赤）、
    /// 後者は席の容量が永久に 0 のまま動かないので初回の確保が計数されない（初回 1 件が赤）。
    #[derive(Debug, Default)]
    pub(super) struct XmapSeat {
        scratch: ResampleScratch,
    }

    impl XmapSeat {
        /// 常設の作業席で `src` を `scale` 倍して `out` へ転写する。
        ///
        /// 写像表の容量がこの呼び出しで増えたら `true`（＝確保が起きた）。恒等 k と外形ゼロは
        /// `resample_with` が写像表に触れる前に復帰するため容量が動かず、自動的に `false` に
        /// なる（触っていない席の確保を数えない・条件分岐を別に持たない）。
        pub(super) fn resample(
            &mut self,
            src: &ComposedSurface,
            scale: ScaleRatio,
            out: &mut ComposedSurface,
        ) -> bool {
            let before = self.scratch.capacity();
            resample_with(src, scale, out, &mut self.scratch);
            self.scratch.capacity() > before
        }
    }

    /// `AlphaMask` の詰めバイト長（`wintf` の `AlphaMask` と同じ式）。
    ///
    /// 出典: `wintf/src/ecs/widget/bitmap_source/alpha_mask.rs` の `from_pbgra32`
    /// （`row_bytes = width.div_ceil(8)`・`data = vec![0; row_bytes * height]`）と
    /// `regenerate_from_pbgra32`（同式で `clear`＋`resize`）。この式が向こうで変われば
    /// 本器の計数は安全側（多め）に振れるだけで、見逃しにはならない。
    fn packed_len(width: u32, height: u32) -> usize {
        width.div_ceil(8) as usize * height as usize
    }

    /// マスクの輪番（A4/A7・2 スロット）＋**各バッファの**到達済み詰めバイト長。
    ///
    /// # なぜ高水位が要るのか（レビューが暴いた黙った確保）
    ///
    /// `AlphaMask::regenerate_from_pbgra32` は `clear`＋`resize(詰め長, 0)` である。詰め長が
    /// そのバッファの容量を超えれば、これは**紛れもない新規確保**である。以前の形は輪番に
    /// 高水位を持たず「in-place 再生成なら確保なし」と決め打ちしていたため、寸法拡大
    /// （例: 40×20 → 60×30 の k=2/1 で詰め長 400→900 バイト）が 1 件も計数へ現れなかった。
    /// 「黙っては確保しない」（design.md §Error Handling）に反する唯一の席だった。
    ///
    /// # 高水位はスロットと一緒に回る
    ///
    /// 輪番は 2 本の `Arc` が 1 適用ずつずれて空きスロットへ入る。高水位は**その実体**に紐づく
    /// 値ゆえ、`Arc` と同じ組で持ち、実体と一緒に次のスロットへ送る。器が握るのは空きスロット
    /// 1 本だけなので、もう 1 本（呼び手へ返した現行マスク）の高水位を [`Self::live_reached`]
    /// として預かり、それが `retired` として戻ってきたときに組み直す。
    ///
    /// ゆえに**寸法拡大の代金は 2 適用に分かれて 1 件ずつ現れる**（バッファが 2 本あるので
    /// 2 本とも伸びる必要がある）。Requirement 3.2 の「一度だけ」は確保対象ごとに一度、
    /// という読みであり、恒等 k の交代席が立ち上がりに 2 適用ぶんかかるのと同じ形である。
    ///
    /// [`Self::live_reached`]: MaskRotation::live_reached
    #[derive(Debug, Default)]
    pub(super) struct MaskRotation {
        /// 空きスロット（前々回のマスク）と、そのバッファの到達済み詰めバイト長。
        spare: Option<(Arc<AlphaMask>, usize)>,
        /// 直前に返したマスクの到達済み詰めバイト長（次の適用で `retired` として戻ってくる）。
        live_reached: usize,
    }

    impl MaskRotation {
        /// 輪番を 1 つ進めてマスクを作り直す。第 2 要素が `true` なら確保が起きた。
        ///
        /// `retired` は直前の適用で表示に使ったマスク（回収エントリが束ねていたもの）。これを
        /// 次の空きスロットとして受け取り、代わりに前々回のマスクを再生成先に取り出す。
        ///
        /// 確保になるのは 3 つの場合で、いずれも `true` を返す（黙って確保しない）:
        /// ⑴ 空きスロットがまだ無い（輪番の立ち上がり） ⑵ 空きスロットが単独所有でない
        /// ⑶ 要求する詰め長がそのバッファの到達済みを超える（`resize` が伸びる）。
        pub(super) fn regenerate(
            &mut self,
            retired: Option<Arc<AlphaMask>>,
            pixels: &[u8],
            width: u32,
            height: u32,
            stride: u32,
        ) -> (Arc<AlphaMask>, bool) {
            let needed = packed_len(width, height);
            let slot = self.spare.take();
            // 直前に返したマスクが次の空きスロットへ回る（高水位も実体と一緒に動かす）。
            self.spare = retired.map(|mask| (mask, self.live_reached));

            let fresh = |this: &mut Self| {
                this.live_reached = needed;
                (
                    Arc::new(AlphaMask::from_pbgra32(pixels, width, height, stride)),
                    true,
                )
            };

            match slot {
                Some((mut shared, reached)) => match Arc::get_mut(&mut shared) {
                    Some(mask) => {
                        // `clear`＋`resize` は到達済みを超えるときだけ確保する（容量は縮まない）。
                        let grew = needed > reached;
                        mask.regenerate_from_pbgra32(pixels, width, height, stride);
                        self.live_reached = reached.max(needed);
                        (shared, grew)
                    }
                    // 単独所有が不成立（第三者が参照を握っている）: 確保するが黙ってはいない。
                    None => fresh(self),
                },
                // 空きスロットがまだ無い（輪番の立ち上がり 2 回ぶん）: 確保して計数する。
                None => fresh(self),
            }
        }
    }
}

/// 毎フレーム経路の確保計数の所有者。適用をまたいで存続し、累積を保持する。
///
/// 再利用席（合成先の常設席・リサンプル作業領域の席・マスクの輪番）は本型のフィールドである
/// （task 5.2・design.md §FrameBudget）。計数の API 面は席の導入で変わっていない。
///
/// - Preconditions: 適用対象（表示 target）ごとに 1 つ持ち、適用のたびに使い回す。
/// - Postconditions: [`take_delta`](FrameBudget::take_delta) は増分を返して器側を 0 に戻す。
///   [`cumulative`](FrameBudget::cumulative) は読み取りのみでリセット手段を持たない。
/// - Invariants: 適用ごとの増分の総和は累積に一致する。是正後の定常状態では
///   [`take_delta`](FrameBudget::take_delta) の全フィールドが 0（Requirement 3.1）であり、
///   寸法変化時は席ごとに一度だけ増えてまた 0 へ戻る（Requirement 3.2）。「一度だけ」は
///   **確保対象ごとに一度**の意であり、2 本のバッファが交代する席（恒等 k の合成先席・
///   マスクの輪番）では代金が 2 適用に分かれて 1 件ずつ現れる。
#[derive(Debug, Default)]
pub(super) struct FrameBudget {
    /// 取り出し待ちの適用単位の増分。
    pending: BudgetDelta,
    /// run 全体の累積（リセットしない）。
    total: BudgetCounters,
    /// 合成先の常設席（A1・`compose_into` の出力先）。
    native: seat::SurfaceSeat,
    /// 表示バッファが到達済みのバイト長（高水位）。席そのものは持たない——バッファの所有は
    /// キャッシュ側にあり、本器は回収の仲介と計数だけを担う（A2/A6）。ゆえにここだけは
    /// 実体と束ねられない（実体の素通しは `budget_tests.rs` の外形一致の檻が守る）。
    ///
    /// 恒等 k の交代（[`FrameBudget::swap_native_scratch`]）では**バッファと一緒にこの高水位も
    /// 入れ替える**。高水位は「その役割」ではなく「その実体」に紐づく値だからである。
    display_reached: usize,
    /// リサンプル作業領域の席（A3・`resample_with` の x 軸写像表）。
    xmap: seat::XmapSeat,
    /// マスクの輪番（A4/A7・2 スロット＋各バッファの到達済み詰めバイト長）。
    mask: seat::MaskRotation,
}

impl FrameBudget {
    /// 空の計数器を作る（全カウンタ 0）。
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// 新規確保・容量成長が **1 回起きた**ことを記録する（計数シームの唯一の入口・D6）。
    ///
    /// 増分と累積を同時に進める。確保が起きなかった経路（席の再利用が成立した場合）では
    /// 呼ばない——「呼ばれた回数」ではなく「確保した回数」を数えることが、再利用の成立・
    /// 不成立を判定材料にする条件である。
    ///
    /// 確保もログもロックも行わないため、毎フレーム経路の上で無条件に呼んでよい。
    ///
    /// # 可視性は task 5.3 で私有へ落とす
    ///
    /// 席が入った今、正しい呼び手は**本モジュールの席メソッドだけ**である（シーム 1 箇所・D6）。
    /// ただし `show.rs` の現行配線（task 1.3）がまだ本メソッドを直接叩いており、その置き換えは
    /// task 5.3 の領分ゆえ `pub(super)` を維持する。5.3 が `show.rs` を席メソッドへ移し終えたら
    /// **私有へ落とすこと**——外から直接叩ける限り「シーム 1 箇所」は形骸化する。
    pub(super) fn note_alloc(&mut self, site: AllocSite) {
        self.pending.bump(site);
        self.total.bump(site);
    }

    /// 合成先の常設席（A1）を貸し出す（`compose_into` の出力先・design.md D2⑴）。
    ///
    /// 席は貸し出しの前後で同じ実体であり、閉包の中で外形へ伸びる。伸長が**到達済みのバイト長を
    /// 超えた**ときにだけ [`AllocSite::ComposeDst`] を計数する（本モジュール冒頭 §高水位）。
    ///
    /// # なぜ閉包で貸すのか
    ///
    /// 合成の外形は `compose_into` の内側（`build_plan`）で決まるため、呼び手も本器も**事前には
    /// 知らない**。ゆえに伸長の観測は「席を使い終えた直後」にしか置けない。`&mut` を裸で返す形は
    /// この観測点を呼び手の記憶に委ねることになり、忘れれば**黙って確保する**経路ができる。
    /// 閉包で囲めば貸し出しと計数が構造的に対になり、シームが 1 箇所（D6）のまま保たれる。
    #[allow(dead_code)] // 配線は task 5.3（show.rs のミス経路）。
    pub(super) fn native_scratch<R>(&mut self, borrow: impl FnOnce(&mut ComposedSurface) -> R) -> R {
        let (produced, grew) = self.native.lend(borrow);
        if grew {
            self.note_alloc(AllocSite::ComposeDst);
        }
        produced
    }

    /// 恒等 k の交代（design.md Flow 2 の `alt k が恒等`）——合成先席と表示バッファを入れ替える。
    ///
    /// コピーも確保も起きない（`Vec` の所有ごと交換する）。交代後は「いま合成した中身」が表示
    /// バッファに、「回収した容量」が合成先席になり、次の適用はそちらへ合成する。
    ///
    /// 到達済みバイト長も一緒に入れ替える——高水位は役割ではなく**実体**に紐づく値であり、
    /// 交換し忘れると交代先の席で伸長を見逃す（＝黙って確保する）ためである。
    #[allow(dead_code)] // 配線は task 5.3。
    pub(super) fn swap_native_scratch(&mut self, display: &mut ComposedSurface) {
        self.native.swap(display, &mut self.display_reached);
    }

    /// 表示バッファを整える（A2/A6・design.md D2⑵）。
    ///
    /// `recycled` は [`ComposeCache::take_recycled`] が返す追い出しエントリで、**合成成功後に
    /// のみ**取ること（Flow 2 の規律・呼び手の責務）。回収できていればその容量をそのまま表示
    /// バッファとして返し、エントリが束ねていたマスクを第 2 要素で返す——呼び手はそれを
    /// [`FrameBudget::regenerate_mask`] へ渡し、輪番の空きスロットとして戻す。
    ///
    /// 回収が成立しなかった場合（初回・[`invalidate_all`] 直後）は空のバッファから始める。この
    /// バッファは以後の k 適用で外形まで伸びる＝**そこで必ず確保が起きる**ため、到達済みバイト長を
    /// 0 へ戻して次の伸長が漏れなく計数されるようにする（黙って確保しない）。確保そのものを
    /// 計数するのは伸長が起きる席メソッド（[`resample_native_into`]／[`swap_native_scratch`] 経由の
    /// [`native_scratch`]）側であり、ここで先回りして数えると 1 適用で二重に数えることになる。
    ///
    /// [`ComposeCache::take_recycled`]: crate::cache::ComposeCache::take_recycled
    /// [`invalidate_all`]: crate::cache::ComposeCache::invalidate_all
    /// [`resample_native_into`]: FrameBudget::resample_native_into
    /// [`native_scratch`]: FrameBudget::native_scratch
    /// [`swap_native_scratch`]: FrameBudget::swap_native_scratch
    #[allow(dead_code)] // 配線は task 5.3。
    pub(super) fn display_buffer(
        &mut self,
        recycled: Option<CacheEntry>,
    ) -> (ComposedSurface, Option<Arc<AlphaMask>>) {
        match recycled {
            Some(CacheEntry { composed, mask }) => (composed, Some(mask)),
            None => {
                // 新しい実体で回り直す＝この実体の到達済みは 0 から数え直す。
                self.display_reached = 0;
                (ComposedSurface::default(), None)
            }
        }
    }

    /// 合成先席の中身を `scale` 倍して `out`（回収した表示バッファ）へ転写する（A3・design.md D2⑶）。
    ///
    /// x 軸写像表は常設席から借りる——`resample`（使い捨ての作業領域を毎回起こす形）ではなく
    /// `resample_with` を使う唯一の理由がこれである。出力バイトは `resample` と 1 バイトも
    /// 違わない（emo-compose 側の等価檻が固定している）。
    ///
    /// 計数は 2 系統で、観測の形が異なる:
    /// [`AllocSite::ResampleDst`] は `out` のバイト長が到達済みを超えたとき（`out` の容量は読めない）、
    /// [`AllocSite::Xmap`] は写像表の**容量がこの呼び出しで増えたとき**（席から直接読める）。
    /// 写像表は恒等 k と外形ゼロでは触られない（`resample_with` が先に復帰する）ため容量が動かず、
    /// その 2 経路は自動的に計数されない——触っていない席の確保を数えると定常ゼロの主張が濁る。
    #[allow(dead_code)] // 配線は task 5.3。
    pub(super) fn resample_native_into(&mut self, scale: ScaleRatio, out: &mut ComposedSurface) {
        let grew_x_map = self.xmap.resample(self.native.get(), scale, out);

        let reached = out.bytes().len();
        if reached > self.display_reached {
            self.display_reached = reached;
            self.note_alloc(AllocSite::ResampleDst);
        }
        if grew_x_map {
            self.note_alloc(AllocSite::Xmap);
        }
    }

    /// マスクを再生成して `Arc` で返す（A4/A7・design.md D3・Flow 2 の輪番）。
    ///
    /// `retired` は [`display_buffer`] が回収エントリから外したマスク——**直前の適用で表示に
    /// 使ったマスク**であり、下流（`AlphaMaskResource`）がまだ握っているため参照数は 2 である。
    /// これを次の空きスロットとして受け取り、代わりに**前々回**のマスク（下流が既に手放して
    /// いる＝単独所有）を再生成先として取り出す。これで輪番が 2 スロットで決定論的に回る。
    ///
    /// 単独所有が成立すれば `Arc::get_mut` で中身を借りて `regenerate_from_pbgra32` により
    /// 内容を作り直す。成立しない境界——初回（空きスロットがまだ無い）・第三者が参照を握って
    /// いる——では新規に確保し、**必ず [`AllocSite::Mask`] を計数する**
    /// （design.md §Error Handling・黙って確保しない）。
    ///
    /// # in-place 再生成も「確保しない」とは限らない
    ///
    /// `regenerate_from_pbgra32` は `clear`＋`resize(詰め長, 0)` である。詰め長がそのバッファの
    /// 到達済みを超えれば `resize` は伸び、それは新規確保そのものである（寸法拡大で必ず起きる）。
    /// ゆえに輪番はスロットごとに到達済み詰めバイト長を持ち、超えたときは in-place 経路でも
    /// [`AllocSite::Mask`] を計数する（[`seat::MaskRotation`]）。**輪番の 2 本は 1 適用ずつ
    /// ずれて再生成される**ため、寸法拡大の代金は 2 適用に分かれて 1 件ずつ現れる。
    ///
    /// 本器が保持する `Arc` は常に高々 1 本（空きスロット）であり、呼び手へ返した 1 本と合わせて
    /// 輪番は 2 本を超えない（Invariants）。単独所有が不成立だったスロットは輪番から外れる
    /// （所有者は他に居るので解放はされない）ため、次の適用で `retired` が新しい空きになり、
    /// 輪番は自力で 2 本へ戻る。
    ///
    /// [`display_buffer`]: FrameBudget::display_buffer
    #[allow(dead_code)] // 配線は task 5.3。
    pub(super) fn regenerate_mask(
        &mut self,
        retired: Option<Arc<AlphaMask>>,
        pixels: &[u8],
        width: u32,
        height: u32,
        stride: u32,
    ) -> Arc<AlphaMask> {
        let (mask, allocated) = self
            .mask
            .regenerate(retired, pixels, width, height, stride);
        if allocated {
            self.note_alloc(AllocSite::Mask);
        }
        mask
    }

    /// この適用分の増分を取り出してリセットする（perf サマリ行へ載せる値）。
    ///
    /// 累積には触れない。取り出しは**表示成立点 1 箇所**（`show.rs` の perf サマリ行 emit の
    /// 直前）で行う。早期復帰した適用の増分は取り出されず次の適用へ持ち越されるが、確保自体は
    /// 実際に起きているため、成立した次の行に現れるのが正しい（累積は常に厳密）。
    pub(super) fn take_delta(&mut self) -> BudgetDelta {
        std::mem::take(&mut self.pending)
    }

    /// run 全体の累積カウンタ（読み取りのみ）。
    ///
    /// テストが「ウォームアップ後の N 反復で 1 件も増えていない」を主張する口である。
    ///
    /// 現時点の読み手はテストのみ（`budget_tests.rs`）。presenter 経由で累積を主張する檻は
    /// task 6.1（定常アロケーション 0）が置く。
    #[allow(dead_code)]
    pub(super) fn cumulative(&self) -> &BudgetCounters {
        &self.total
    }
}

#[cfg(test)]
#[path = "budget_tests.rs"]
mod tests;
