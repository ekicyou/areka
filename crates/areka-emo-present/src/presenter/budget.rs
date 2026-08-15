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
//! | マスクの輪番（`Option<Arc<AlphaMask>>`・空き 1 枚） | A4/A7 | [`FrameBudget::regenerate_mask`] |
//!
//! 表示バッファ（A2/A6）だけは本器が所有しない——所有はキャッシュ側にあり、追い出しエントリの
//! 容量を [`ComposeCache::take_recycled`] で回収して回す。本器は
//! [`FrameBudget::display_buffer`] でその仲介と計数だけを担う。
//!
//! # 回る実体の本数（キャッシュ容量 3 での形）
//!
//! キャッシュが最大 3 エントリを保持するため、毎コマ経路を回る実体の本数は容量 1 の頃から増えた:
//!
//! - **表示バッファ**: キャッシュの 3 本 ＋（恒等 k では合成先席の 1 本）＝ 最大 4 本
//! - **マスク**: キャッシュの 3 本 ＋ 輪番の空き 1 枚 ＝ 4 本
//!
//! いずれも本数が有限で、追い出し 1 件が次の適用の入力になるという形は変わらない。**本器が
//! 実体ごとの状態を覚えないのはこのためである**——覚える形は本数が増えた瞬間に破綻する
//! （§なぜ「到達済み寸法（高水位）を器が覚える」形をやめたのか）。立ち上がりに要する適用数は
//! 「本数ぶん」であり、Requirement 3.2 の「一度だけ」は**確保対象ごとに一度**の意味で読む。
//!
//! # 何をもって「確保した」と数えるか（**全ての席で 1 通り**＝実体の容量差）
//!
//! 席（あるいは席を通り抜けるバッファ）そのものの `Vec` 容量を、呼び出しの**前後で読み比べる**。
//! `Vec` の容量は再確保でしか増えず、`clear`／`truncate`／`resize` で縮みもしないため、
//! 「増えた＝この呼び出しで確保が起きた／変わらない＝起きていない」が厳密に決まる。過大にも
//! 過小にも振れない。読み口は 3 つで、いずれも本 spec が additive に足した観測専用の getter である:
//!
//! | 対象 | 読み口 |
//! |---|---|
//! | リサンプル作業領域（x 軸写像表） | [`ResampleScratch::capacity`] |
//! | 合成先席・表示バッファ | [`ComposedSurface::bytes_capacity`] |
//! | 当たり判定マスクの詰めバイト列 | [`AlphaMask::packed_capacity`] |
//!
//! ## なぜ「到達済み寸法（高水位）を器が覚える」形をやめたのか
//!
//! 高水位を器のフィールドとして持つ形は、**高水位が役割に紐づき実体に紐づかない**。バッファが
//! 1 本しか無いうちは両者が一致するので成立していたが、次の 2 つの理由で成立しなくなった:
//!
//! 1. **キャッシュ容量が 1 → 3 になった**（要件 7.1・2026-08-15 開発者裁定）。表示バッファは
//!    3 本＋合成先席 1 本の計 4 本が入れ替わりながら回る。器が持つ 1 個の高水位はそのうちどれの
//!    値なのかを言えず、**寸法変化の途中で他の本の高水位を当てて成長を見逃す**（＝黙って確保する）
//! 2. 実体を差し替えつつ高水位を持ち越す改変（`display_buffer` が回収バッファを捨てて空を返す・
//!    `SurfaceSeat::lend` が席を作り直す）が**計数へ 1 件も現れなかった**。番地の一致という
//!    走ごとに揺れる傍証しか検出器が無く、決定論的に殺せていなかった
//!
//! 容量を実体から直接読む形は 2 つとも構造で殺す——差し替えられた実体の容量は 0 から始まるので、
//! 同じ寸法へ伸びた時点で必ず 1 件計数される。
//!
//! ## 誤差の向き（安全側）
//!
//! この形の誤差は片側だけである。縮小したバッファは容量を保つため、後で元の寸法へ戻す要求では
//! 容量が動かず**確保なし**と正しく判定される。逆に「新しい実体を同じ寸法で確保し直す」改変は、
//! 容量が 0 から伸びるぶんを必ず計数する。見逃し（＝定常アロケーション 0 の偽の成立）は
//! 構造的に起こらない。
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
//! # design.md との突合（**task 5.3 で解消済み**）
//!
//! design.md「FrameBudget（`presenter/budget.rs`・新設）」の Service Interface ブロックは
//! task 5.2 実装より前に書かれており、席の実形と 3 点で食い違っていた（閉包貸しの
//! [`FrameBudget::native_scratch`]・第 1 引数に `retired` を取る [`FrameBudget::regenerate_mask`]・
//! ブロックに無かった [`FrameBudget::swap_native_scratch`]）。いずれも実装側の判断で形を変えた
//! ものであり **正しいのは実装の形**である（理由は各メソッドの doc に書いてある）。task 5.3 が
//! design.md の当該ブロックを実形へ更新して差異を解消したため、本モジュールが単独で持つ
//! 未解決の乖離は無い。
//!
//! 併せて design.md の `alloc_resample_dst` の説明も実挙動へ揃えた: 恒等 k の交代経路で容量回収が
//! 不成立になった適用は**その回に確保を 1 件も起こさず**、代金は次の適用の `alloc_compose_dst` に
//! 載る（空バッファが合成先席へ、容量のある方が表示バッファへ出るため）。取りこぼしは無く、
//! 判定式⑶は 4 フィールドの合計ゆえ機械判定も動かない。
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
///
/// **本モジュール私有**である。計数シームが 1 箇所（D6）であるためには、発生点の名前を外から
/// 持ち出せないことが条件になる——外に出ていれば席を経由しない計数が書けてしまう。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AllocSite {
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
    /// 読み手は本モジュールのテスト（`budget_tests.rs`）のみである。**製品コードにも presenter 側の
    /// 檻にも走査の消費者は無い**——本列挙は本モジュール私有ゆえ外へ持ち出せず、presenter 経由の檻
    /// （`presenter_budget_steady_state_tests.rs`）は [`BudgetCounters`] の名前つき 4 フィールドを
    /// 直接読む。列挙を増減させると perf サマリ行のフィールド集合が変わるため、「4 つある」ことを
    /// 固定する走査は本モジュール内に置く。
    #[allow(dead_code)]
    const ALL: [AllocSite; 4] = [
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
    fn count(&self, site: AllocSite) -> u32 {
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
    /// 読み手は本モジュールのテストのみ（presenter 経由の檻は名前つきフィールドを直接読む）。
    #[allow(dead_code)]
    fn count(&self, site: AllocSite) -> u64 {
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
/// 観測を席と別の場所に置くと、**席だけを差し替える改変が計数へ現れない**経路ができる。ゆえに
/// 本入れ物はフィールドを私有にし、**席へ触れる唯一の道**を観測込みのメソッドにする。
///
/// 観測の形は 3 席とも同一で、**実体そのものの `Vec` 容量を呼び出しの前後で読み比べる**
/// （本モジュール冒頭 §何をもって「確保した」と数えるか）。到達済み寸法（高水位）を器が覚える
/// 形は 1 つも残っていない——覚える形は実体が複数本で入れ替わる経路（キャッシュ容量 3）で
/// 破綻し、実体の差し替えを見逃す。
mod seat {
    use std::sync::Arc;

    use wintf::ecs::widget::bitmap_source::AlphaMask;

    use super::{ComposedSurface, ResampleScratch, ScaleRatio, resample_with};

    /// 合成先の常設席（A1）。**高水位を持たない**——観測は席の実体の容量そのもので行う。
    #[derive(Debug, Default)]
    pub(super) struct SurfaceSeat {
        surface: ComposedSurface,
    }

    impl SurfaceSeat {
        /// 席を閉包へ貸す。閉包の中で**席の容量が増えたら** `true`（＝確保が起きた）。
        ///
        /// 容量は再確保でしか増えないため、この観測は厳密である。席の実体を差し替える改変は
        /// 容量 0 から始まる新品を作ることになり、外形まで伸びた時点で必ず計数される。
        pub(super) fn lend<R>(
            &mut self,
            borrow: impl FnOnce(&mut ComposedSurface) -> R,
        ) -> (R, bool) {
            let before = self.surface.bytes_capacity();
            let produced = borrow(&mut self.surface);
            (produced, self.surface.bytes_capacity() > before)
        }

        /// 読み取り用の参照（リサンプル元）。
        pub(super) fn get(&self) -> &ComposedSurface {
            &self.surface
        }

        /// 恒等 k の交代（design.md Flow 2）。観測は実体の容量から読むため、席と一緒に
        /// 動かすべき付随状態は 1 つも無い。
        pub(super) fn swap(&mut self, buffer: &mut ComposedSurface) {
            std::mem::swap(&mut self.surface, buffer);
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

    /// マスクの輪番（A4/A7）。器が握るのは**空き 1 枚**だけで、残りはキャッシュ側に在る。
    ///
    /// # 輪番の形（キャッシュ容量 3）
    ///
    /// 1 適用の流れは「追い出しエントリのマスクを空きとして受け取り、いま在る空きを再生成先として
    /// 取り出す」である。ゆえに実体はキャッシュの 3 本＋空き 1 枚の計 4 本が順に回る。取り出す
    /// 空きは**下流が既に手放している**（新しいマスクが `set_shared` で置き換えたため）ので
    /// 単独所有が成立し、`Arc::get_mut` による in-place 再生成が通る。
    ///
    /// # 確保の観測は実体の容量そのもの（高水位を持たない）
    ///
    /// `AlphaMask::regenerate_from_pbgra32` は `clear`＋`resize(詰め長, 0)` である。詰め長が
    /// そのバッファの容量を超えれば、これは**紛れもない新規確保**である（例: 40×20 → 60×30 の
    /// k=2/1 で詰め長 400→900 バイト）。[`AlphaMask::packed_capacity`] を再生成の前後で読み
    /// 比べれば、それが厳密に決まる。器側で到達済み詰めバイト長を覚える形は採らない——回る実体が
    /// 4 本ある以上、器が持つ 1 個の値は**どの実体のものでもない**（本モジュール冒頭 §なぜ
    /// 「到達済み寸法（高水位）を器が覚える」形をやめたのか）。
    ///
    /// **寸法拡大の代金は回る本数ぶんの適用に分かれて 1 件ずつ現れる**。Requirement 3.2 の
    /// 「一度だけ」は確保対象ごとに一度、という読みである。
    ///
    /// [`AlphaMask::packed_capacity`]: wintf::ecs::widget::bitmap_source::AlphaMask::packed_capacity
    #[derive(Debug, Default)]
    pub(super) struct MaskRotation {
        /// 空きスロット（下流が手放し済みのマスク）。
        spare: Option<Arc<AlphaMask>>,
    }

    impl MaskRotation {
        /// 輪番を 1 つ進めてマスクを作り直す。第 2 要素が `true` なら確保が起きた。
        ///
        /// `retired` は追い出しエントリが束ねていたマスク。これを次の空きスロットとして受け取り、
        /// 代わりにいま在る空きを再生成先に取り出す。
        ///
        /// 確保になるのは 3 つの場合で、いずれも `true` を返す（黙って確保しない）:
        /// ⑴ 空きスロットがまだ無い（輪番の立ち上がり） ⑵ 空きスロットが単独所有でない
        /// ⑶ 要求する詰め長がそのバッファの容量を超える（`resize` が伸びる）。
        pub(super) fn regenerate(
            &mut self,
            retired: Option<Arc<AlphaMask>>,
            pixels: &[u8],
            width: u32,
            height: u32,
            stride: u32,
        ) -> (Arc<AlphaMask>, bool) {
            let slot = self.spare.take();
            // 追い出しエントリのマスクが次の空きスロットへ回る。
            self.spare = retired;

            let fresh = || {
                (
                    Arc::new(AlphaMask::from_pbgra32(pixels, width, height, stride)),
                    true,
                )
            };

            match slot {
                Some(mut shared) => match Arc::get_mut(&mut shared) {
                    Some(mask) => {
                        // `clear`＋`resize` は容量を超えるときだけ確保する（容量は縮まない）。
                        let before = mask.packed_capacity();
                        mask.regenerate_from_pbgra32(pixels, width, height, stride);
                        let grew = mask.packed_capacity() > before;
                        (shared, grew)
                    }
                    // 単独所有が不成立（第三者が参照を握っている）: 確保するが黙ってはいない。
                    None => fresh(),
                },
                // 空きスロットがまだ無い（輪番の立ち上がり）: 確保して計数する。
                None => fresh(),
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
    /// # 可視性は**私有**である（シーム 1 箇所の実効化・task 5.3）
    ///
    /// 席が入った今、正しい呼び手は**本モジュールの席メソッドだけ**である（D6）。`show.rs` が
    /// 席メソッドへ移った時点で `pub(super)` を落とした——外から直接叩ける限り「シーム 1 箇所」は
    /// 形骸化し、席を通さない確保が黙って計数へ紛れ込む経路が残るためである。テスト
    /// （`budget_tests.rs`）は子モジュールゆえ私有のまま到達でき、計数の意味論を単独で固定できる。
    fn note_alloc(&mut self, site: AllocSite) {
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
    /// 付随して動かす状態は 1 つも無い——確保の観測は実体の容量から直接読むため、交代で
    /// 取り違える値が存在しない（本モジュール冒頭 §何をもって「確保した」と数えるか）。
    pub(super) fn swap_native_scratch(&mut self, display: &mut ComposedSurface) {
        self.native.swap(display);
    }

    /// 表示バッファを整える（A2/A6・design.md D2⑵）。
    ///
    /// `recycled` は [`ComposeCache::take_recycled`] が返す追い出しエントリで、**合成成功後に
    /// のみ**取ること（Flow 2 の規律・呼び手の責務）。回収できていればその容量をそのまま表示
    /// バッファとして返し、エントリが束ねていたマスクを第 2 要素で返す——呼び手はそれを
    /// [`FrameBudget::regenerate_mask`] へ渡し、輪番の空きスロットとして戻す。
    ///
    /// 回収が成立しなかった場合（キャッシュにまだ空きがある暖機中・[`invalidate_all`] 直後）は
    /// 空のバッファから始める。このバッファは以後の k 適用で外形まで伸びる＝**そこで必ず確保が
    /// 起きる**が、その計数は伸長が起きる席メソッド（[`resample_native_into`]／
    /// [`swap_native_scratch`] 経由の [`native_scratch`]）側が実体の容量差として拾う。ここで
    /// 先回りして数えると 1 適用で二重に数えることになる。
    ///
    /// 本メソッドが状態を持たない（`&mut self` を使わない）のは、確保の観測が**実体の容量**から
    /// 読めるようになったためである。以前は「新しい実体で回り直す」ことを器のフィールドへ
    /// 記録していたが、回るバッファが 4 本になった今それは取り違えの元でしかない。
    ///
    /// [`ComposeCache::take_recycled`]: crate::cache::ComposeCache::take_recycled
    /// [`invalidate_all`]: crate::cache::ComposeCache::invalidate_all
    /// [`resample_native_into`]: FrameBudget::resample_native_into
    /// [`native_scratch`]: FrameBudget::native_scratch
    /// [`swap_native_scratch`]: FrameBudget::swap_native_scratch
    pub(super) fn display_buffer(
        &mut self,
        recycled: Option<CacheEntry>,
    ) -> (ComposedSurface, Option<Arc<AlphaMask>>) {
        match recycled {
            // 追い出しエントリの native 原寸は使わない——新しいエントリの原寸は今回の合成が
            // 決めるので、回収するのはバッファの確保だけである。
            Some(CacheEntry {
                composed,
                mask,
                native: _,
            }) => (composed, Some(mask)),
            None => (ComposedSurface::default(), None),
        }
    }

    /// 合成先席の中身を `scale` 倍して `out`（回収した表示バッファ）へ転写する（A3・design.md D2⑶）。
    ///
    /// x 軸写像表は常設席から借りる——`resample`（使い捨ての作業領域を毎回起こす形）ではなく
    /// `resample_with` を使う唯一の理由がこれである。出力バイトは `resample` と 1 バイトも
    /// 違わない（emo-compose 側の等価檻が固定している）。
    ///
    /// 計数は 2 系統で、どちらも**実体の容量を呼び出しの前後で読み比べる**同じ形である:
    /// [`AllocSite::ResampleDst`] は `out` のバイト列の容量が増えたとき、[`AllocSite::Xmap`] は
    /// 写像表の容量が増えたとき。写像表は恒等 k と外形ゼロでは触られない（`resample_with` が
    /// 先に復帰する）ため容量が動かず、その 2 経路は自動的に計数されない——触っていない席の
    /// 確保を数えると定常ゼロの主張が濁る。
    ///
    /// `out` は毎回同じ実体とは限らない（キャッシュの追い出しバッファが順に回ってくる）。ゆえに
    /// 器側で到達済みを覚える形は採れず、**その回に渡された実体の容量**だけを見る。回収されずに
    /// 空バッファが渡ってきた回は容量 0 から伸びるので、必ず 1 件計数される。
    pub(super) fn resample_native_into(&mut self, scale: ScaleRatio, out: &mut ComposedSurface) {
        let before = out.bytes_capacity();
        let grew_x_map = self.xmap.resample(self.native.get(), scale, out);

        if out.bytes_capacity() > before {
            self.note_alloc(AllocSite::ResampleDst);
        }
        if grew_x_map {
            self.note_alloc(AllocSite::Xmap);
        }
    }

    /// マスクを再生成して `Arc` で返す（A4/A7・design.md D3・Flow 2 の輪番）。
    ///
    /// `retired` は [`display_buffer`] が回収エントリから外したマスク——**追い出されたエントリが
    /// 束ねていたマスク**であり、下流（`AlphaMaskResource`）は既により新しいマスクへ差し替わって
    /// いるため単独所有である。これを次の空きスロットとして受け取り、代わりにいま在る空きを
    /// 再生成先として取り出す。これで輪番が決定論的に回る（回る本数はキャッシュ容量＋1）。
    ///
    /// 単独所有が成立すれば `Arc::get_mut` で中身を借りて `regenerate_from_pbgra32` により
    /// 内容を作り直す。成立しない境界——立ち上がり（空きスロットがまだ無い）・第三者が参照を
    /// 握っている——では新規に確保し、**必ず [`AllocSite::Mask`] を計数する**
    /// （design.md §Error Handling・黙って確保しない）。
    ///
    /// # in-place 再生成も「確保しない」とは限らない
    ///
    /// `regenerate_from_pbgra32` は `clear`＋`resize(詰め長, 0)` である。詰め長がそのバッファの
    /// **容量**を超えれば `resize` は伸び、それは新規確保そのものである（寸法拡大で必ず起きる）。
    /// ゆえに輪番は再生成の前後で [`AlphaMask::packed_capacity`] を読み比べ、増えたときは
    /// in-place 経路でも [`AllocSite::Mask`] を計数する（[`seat::MaskRotation`]）。**回る実体は
    /// 1 適用ずつずれて再生成される**ため、寸法拡大の代金は本数ぶんの適用に分かれて 1 件ずつ現れる。
    ///
    /// 本器が保持する `Arc` は常に高々 1 本（空きスロット）であり、残りはキャッシュのエントリが
    /// 握る。単独所有が不成立だったスロットは輪番から外れる（所有者は他に居るので解放はされない）
    /// ため、次の適用で `retired` が新しい空きになり、輪番は自力で立ち直る。
    ///
    /// [`display_buffer`]: FrameBudget::display_buffer
    /// [`AlphaMask::packed_capacity`]: wintf::ecs::widget::bitmap_source::AlphaMask::packed_capacity
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
    /// 読み手はテストのみ（`budget_tests.rs` が席の単体で、
    /// `presenter_budget_steady_state_tests.rs` が実 `apply_show` を通した定常状態で読む）。
    #[allow(dead_code)]
    pub(super) fn cumulative(&self) -> &BudgetCounters {
        &self.total
    }

    /// 合成先の常設席がいま抱えているバイト列の先頭位置（**テストの観測口・製品経路に消費者なし**）。
    ///
    /// # なぜ口が要るのか（席を私有にした代償）
    ///
    /// design.md §Testing Strategy「Integration Tests」項目 1 は、定常アロケーション 0 の檻に
    /// 「③native scratch ポインタ不変」を課している。ところが席（[`seat::SurfaceSeat`]）は
    /// 本モジュール私有であり、**計数シームが 1 箇所である**（D6）ためにその私有性は落とせない。
    /// presenter 側の檻（`presenter_budget_steady_state_tests.rs`）は本モジュールの子ではないので、
    /// 席の実体へ届く道が他に無い。
    ///
    /// 読めるのは番地だけで、席の中身にも計数にも触れない。`#[cfg(test)]` ゆえ製品ビルドには
    /// 存在せず、[`AllocSite`] も perf サマリ行のフィールド集合も動かさない
    /// （`tools/perf/judge-perf.py` との契約は不変）。
    ///
    /// # 検出力はこの口には無い
    ///
    /// 返すのは席が抱えるバイト列の先頭位置だけである。番地を渡すだけのこの口自体に検出力は
    /// 無く、検出力を持つのはこの値を読む側の assert である。どの assert がどの誤実装をどれだけ
    /// 赤にしたかの実測は `presenter_budget_steady_state_tests.rs` の
    /// §番地の主張はどこまで効くか に一箇所だけ登記してある。
    #[cfg(test)]
    pub(super) fn native_scratch_ptr(&self) -> *const u8 {
        self.native.get().bytes().as_ptr()
    }
}

#[cfg(test)]
#[path = "budget_tests.rs"]
mod tests;
