//! `FrameBudget`: 毎フレーム経路における新規確保・容量成長の**唯一の計数シーム**。
//!
//! 「定常状態で表示バッファの新規確保が起きていない」を数値で言い切るための計数器。確保の発生点は
//! 2 つ（席の一覧と 1:1）:
//!
//! | 発生点 | 増えるフィールド | 何の確保か |
//! |---|---|---|
//! | [`AllocSite::ComposeDst`] | `alloc_compose_dst` | native 合成先（＝交代で表示バッファにもなる） |
//! | [`AllocSite::Mask`] | `alloc_mask` | 当たり判定マスク |
//!
//! # なぜ 1 箇所で数えるのか
//!
//! プロセス全体の確保を捕まえる `#[global_allocator]` 差し替えは、emo-compose の既存予算檻が
//! **棄却済み**の方針である（`areka-emo-compose/src/golden_tests_determinism_budget_tests.rs`
//! の「アプローチ (B)」——プロセス全体を汚染し既存テストを不安定化しうる）。代わりに、
//! 確保が起き得る発生点そのものを名前で数える。
//!
//! # 席の一覧
//!
//! 器は計数に加えて**再利用席**を持つ。[`FrameBudget::note_alloc`] は「席の再利用が成立せず結局
//! 確保したとき」に席メソッドの内部から呼ばれる。
//!
//! | 席 | 貸し出し口 |
//! |---|---|
//! | 合成先の常設席（`ComposedSurface`） | [`FrameBudget::native_scratch`]・表示バッファとの交代は [`FrameBudget::swap_native_scratch`] |
//! | マスクの輪番（`Option<Arc<AlphaMask>>`・空き 1 枚） | [`FrameBudget::regenerate_mask`] |
//!
//! 表示バッファだけは本器が所有しない——所有はキャッシュ側にあり、追い出しエントリの容量を
//! [`ComposeCache::take_recycled`] で回収して回す。本器は [`FrameBudget::display_buffer`] で
//! その仲介と計数だけを担う。
//!
//! # 回る実体の本数（キャッシュ容量 3 での形）
//!
//! キャッシュが最大 3 エントリを保持するため、毎コマ経路を回る実体の本数は次のとおり:
//!
//! - **表示バッファ**: キャッシュの 3 本 ＋ 合成先席の 1 本 ＝ 最大 4 本
//! - **マスク**: キャッシュの 3 本 ＋ 輪番の空き 1 枚 ＝ 4 本
//!
//! 追い出し 1 件が次の適用の入力になる。**本器が実体ごとの状態を覚えないのはこのためである**——
//! 覚える形は本数が増えた瞬間に破綻する（下の「なぜ「到達済み寸法（高水位）を器が覚える」形をやめたのか」節）。
//! 立ち上がりに要する適用数は「本数ぶん」であり、「一度だけ」は**確保対象ごとに一度**の意味で読む。
//!
//! # 何をもって「確保した」と数えるか（**全ての席で 1 通り**＝実体の容量差）
//!
//! 席（あるいは席を通り抜けるバッファ）そのものの `Vec` 容量を、呼び出しの**前後で読み比べる**。
//! `Vec` の容量は再確保でしか増えず、`clear`／`truncate`／`resize` で縮みもしないため、
//! 「増えた＝この呼び出しで確保が起きた」が厳密に決まる。読み口は 2 つ:
//!
//! | 対象 | 読み口 |
//! |---|---|
//! | 合成先席・表示バッファ | [`ComposedSurface::bytes_capacity`] |
//! | 当たり判定マスクの詰めバイト列 | [`AlphaMask::packed_capacity`] |
//!
//! ## なぜ「到達済み寸法（高水位）を器が覚える」形をやめたのか
//!
//! 高水位を器のフィールドとして持つと、**高水位が役割に紐づき実体に紐づかない**。バッファが
//! 1 本しか無いうちは成立するが、次の 2 つで成立しなくなる:
//!
//! 1. **キャッシュ容量が 1 → 3 になった**（2026-08-15 開発者裁定）。表示バッファは 4 本が入れ替わり
//!    ながら回り、器が持つ 1 個の高水位はどの本の値かを言えず、**寸法変化の途中で他の本の高水位を
//!    当てて成長を見逃す**（＝黙って確保する）
//! 2. 実体を差し替えつつ高水位を持ち越す改変（`display_buffer` が回収バッファを捨てて空を返す・
//!    `SurfaceSeat::lend` が席を作り直す）が計数へ 1 件も現れなかった。番地の一致という走ごとに
//!    揺れる傍証しか検出器が無く、決定論的に殺せなかった
//!
//! 容量を実体から直接読めば、差し替えられた実体の容量は 0 から始まるので、同じ寸法へ伸びた
//! 時点で必ず 1 件計数される。
//!
//! ## 誤差の向き（安全側）
//!
//! この形の誤差は片側だけである。縮小したバッファは容量を保つため、元の寸法へ戻す要求は
//! **確保なし**と正しく判定される。「新しい実体を同じ寸法で確保し直す」改変は、容量が 0 から
//! 伸びるぶんを必ず計数する。見逃し（＝定常アロケーション 0 の偽の成立）は起こらない。
//!
//! # 席が期待どおり再利用できない境界
//!
//! 回収エントリが無い・マスク輪番の空きスロットが単独所有でない、といった境界では**確保する**が、
//! **黙っては確保しない**——必ず該当カウンタを増やす（ログ無し失敗経路の禁止を計数フィールドで
//! 担保している）。
//!
//! # 増分と累積を両方出す理由
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
//! 計数は整数の飽和加算のみで、確保もログもロックも行わず、ログ設定も参照しない。
//!
//! 交代経路で容量回収が不成立になった適用は**その回に確保を 1 件も起こさず**、代金は次の適用の
//! `alloc_compose_dst` に載る（空バッファが合成先席へ、容量のある方が表示バッファへ出るため）。
//! 取りこぼしは無く、判定式⑶は 2 フィールドの合計ゆえ機械判定も動かない。
//!
//! [`ComposeCache::take_recycled`]: crate::cache::ComposeCache::take_recycled

use std::sync::Arc;

use wintf::ecs::widget::bitmap_source::AlphaMask;

use super::ComposedSurface;
use crate::cache::CacheEntry;

/// 確保の発生点（perf サマリ行の `alloc_*` と 1:1）。
///
/// 列挙を増減させると perf サマリ行のフィールド集合が変わる＝判定スクリプトとの契約変更である。
///
/// **本モジュール私有**: 計数シームが 1 箇所であるには発生点の名前を外へ出せないことが条件で、
/// 外に出ていれば席を経由しない計数が書けてしまう。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AllocSite {
    /// native 合成先の新規確保／容量成長（交代で表示バッファへ出る側の確保もここで数える）。
    ComposeDst,
    /// 当たり判定マスクの新規確保（輪番スロットの単独所有が不成立の場合を含む）。
    Mask,
}

impl AllocSite {
    /// 全発生点（走査の正本。檻はこれを回して発生点ごとの計数を確かめる）。
    ///
    /// 読み手は本モジュールのテスト（`budget_tests.rs`）のみ。presenter 経由の檻は [`BudgetCounters`]
    /// の名前つき 2 フィールドを直接読む。増減は perf サマリ行のフィールド集合の照合が捕まえる。
    #[cfg(test)]
    const ALL: [AllocSite; 2] = [AllocSite::ComposeDst, AllocSite::Mask];
}

/// 1 適用分の確保計数スナップショット（perf サマリ行の `alloc_*` フィールドの供給源）。
///
/// [`FrameBudget::take_delta`] が取り出しと同時に器側をリセットするため、この値は常に直前の取り出しから
/// 今回までの 1 適用分を表す。定常状態では全フィールドが 0 になる。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct BudgetDelta {
    /// native 合成先の新規確保／容量成長。
    pub(super) alloc_compose_dst: u32,
    /// 当たり判定マスクの新規確保。
    pub(super) alloc_mask: u32,
}

impl BudgetDelta {
    /// 発生点で引く（全発生点の走査用。名前つきフィールドと同じ値を返す）。
    ///
    /// 現時点の読み手はテストのみ（[`AllocSite::ALL`] を回す檻）。製品コードの
    /// perf サマリ行は名前つきフィールドを直接使う。
    #[cfg(test)]
    fn count(&self, site: AllocSite) -> u32 {
        match site {
            AllocSite::ComposeDst => self.alloc_compose_dst,
            AllocSite::Mask => self.alloc_mask,
        }
    }

    /// 1 件加算する（飽和。1 適用で u32 を溢れさせる経路は無いが、値を巻き戻さない）。
    fn bump(&mut self, site: AllocSite) {
        let slot = match site {
            AllocSite::ComposeDst => &mut self.alloc_compose_dst,
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
    /// 当たり判定マスクの累積。
    pub(super) alloc_mask: u64,
}

impl BudgetCounters {
    /// 発生点で引く（全発生点の走査用）。
    ///
    /// 読み手は本モジュールのテストのみ（presenter 経由の檻は名前つきフィールドを直接読む）。
    #[cfg(test)]
    fn count(&self, site: AllocSite) -> u64 {
        match site {
            AllocSite::ComposeDst => self.alloc_compose_dst,
            AllocSite::Mask => self.alloc_mask,
        }
    }

    /// 1 件加算する（飽和。溢れても値を巻き戻さない＝計数を捏造しない）。
    fn bump(&mut self, site: AllocSite) {
        let slot = match site {
            AllocSite::ComposeDst => &mut self.alloc_compose_dst,
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
/// フィールドを私有にし、**席へ触れる唯一の道**を観測込みのメソッドにする。観測は 2 席とも、実体の
/// `Vec` 容量を呼び出しの前後で読み比べる（本モジュール冒頭の「何をもって「確保した」と数えるか」節）。
mod seat {
    use std::sync::Arc;

    use wintf::ecs::widget::bitmap_source::AlphaMask;

    use super::ComposedSurface;

    /// 合成先の常設席。**高水位を持たない**——観測は席の実体の容量そのもので行う。
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

        /// 読み取り用の参照（席の番地を読む観測口 `FrameBudget::native_scratch_ptr` 専用）。製品経路に
        /// 読み手は無いので `#[cfg(test)]` で閉じる。
        #[cfg(test)]
        pub(super) fn get(&self) -> &ComposedSurface {
            &self.surface
        }

        /// 表示バッファとの交代（**k の値に依らず常にこの経路**）。観測は
        /// 実体の容量から読むため、席と一緒に動かすべき付随状態は 1 つも無い。
        pub(super) fn swap(&mut self, buffer: &mut ComposedSurface) {
            std::mem::swap(&mut self.surface, buffer);
        }
    }

    /// マスクの輪番。器が握るのは**空き 1 枚**だけで、残りはキャッシュ側に在る。
    ///
    /// # 輪番の形（キャッシュ容量 3）
    ///
    /// 1 適用の流れは「追い出しエントリのマスクを空きとして受け取り、いま在る空きを再生成先として
    /// 取り出す」。実体はキャッシュの 3 本＋空き 1 枚の計 4 本が順に回る。取り出す空きは**下流が既に
    /// 手放している**（新しいマスクが `set_shared` で置き換えたため）ので単独所有が成立し、
    /// `Arc::get_mut` による in-place 再生成が通る。
    ///
    /// # 確保の観測は実体の容量そのもの
    ///
    /// `AlphaMask::regenerate_from_pbgra32` は `clear`＋`resize(詰め長, 0)` なので、詰め長がそのバッファの
    /// 容量を超えれば新規確保になる（例: 40×20 → 60×30 で詰め長 100→240 バイト・行はバイト境界まで詰める）。
    /// [`AlphaMask::packed_capacity`] を再生成の前後で読み比べて決める。回る実体が 4 本ある以上、器が
    /// 覚える 1 個の値は**どの実体のものでもない**（本モジュール冒頭の「なぜ「到達済み寸法（高水位）を器が
    /// 覚える」形をやめたのか」節）ので、寸法拡大の代金は回る本数ぶんの適用に分かれて 1 件ずつ現れる。
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
                None => fresh(),
            }
        }
    }
}

/// 毎フレーム経路の確保計数の所有者。適用をまたいで存続し、累積を保持する。
///
/// 再利用席（合成先の常設席・マスクの輪番）は本型のフィールドである。
///
/// - Preconditions: 適用対象（表示 target）ごとに 1 つ持ち、使い回す。
/// - Postconditions: [`take_delta`](FrameBudget::take_delta) は増分を返して器側を 0 に戻す。
///   [`cumulative`](FrameBudget::cumulative) は読み取りのみでリセット手段を持たない。
/// - Invariants: 適用ごとの増分の総和は累積に一致する。定常状態では
///   [`take_delta`](FrameBudget::take_delta) の全フィールドが 0 で、寸法変化時は席ごとに増えて
///   また 0 へ戻る。「一度だけ」は**確保対象ごとに一度**の意で、複数のバッファが回る席（最大 4 本）
///   では代金が回る本数ぶんの適用に分かれて 1 件ずつ現れる。
#[derive(Debug, Default)]
pub(super) struct FrameBudget {
    /// 取り出し待ちの適用単位の増分。
    pending: BudgetDelta,
    /// run 全体の累積（リセットしない）。
    total: BudgetCounters,
    /// 合成先の常設席（`compose_into` の出力先）。
    native: seat::SurfaceSeat,
    /// マスクの輪番（下流が手放した空き 1 枚）。
    mask: seat::MaskRotation,
}

impl FrameBudget {
    /// 空の計数器を作る（全カウンタ 0）。
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// 新規確保・容量成長が **1 回起きた**ことを記録する（計数シームの唯一の入口）。
    ///
    /// 増分と累積を同時に進める。確保が起きなかった経路（席の再利用が成立した場合）では呼ばない——
    /// 「確保した回数」を数えることが、再利用の成立・不成立の判定材料になる条件である。確保もログも
    /// ロックも行わないため、毎フレーム経路の上で無条件に呼んでよい。
    ///
    /// # 可視性は**私有**である
    ///
    /// 正しい呼び手は本モジュールの席メソッドだけ。外から直接叩けると「シーム 1 箇所」が形骸化し、
    /// 席を通さない確保が黙って計数へ紛れ込む。テスト（`budget_tests.rs`）は子モジュールゆえ私有の
    /// まま到達できる。
    fn note_alloc(&mut self, site: AllocSite) {
        self.pending.bump(site);
        self.total.bump(site);
    }

    /// 合成先の常設席を貸し出す（`compose_into` の出力先）。
    ///
    /// 席は貸し出しの前後で同じ実体であり、閉包の中で外形へ伸びる。席の `Vec` 容量を貸し出しの前後で
    /// 読み比べ、増えたときにだけ [`AllocSite::ComposeDst`] を計数する。
    ///
    /// # なぜ閉包で貸すのか
    ///
    /// 合成の外形は `compose_into` の内側（`build_plan`）で決まり、呼び手も本器も**事前には知らない**ので、
    /// 伸長の観測は席を使い終えた直後にしか置けない。`&mut` を裸で返すと観測点が呼び手の記憶頼みになり、
    /// 忘れれば**黙って確保する**経路ができる。閉包で囲めば貸し出しと計数が構造的に対になる。
    pub(super) fn native_scratch<R>(
        &mut self,
        borrow: impl FnOnce(&mut ComposedSurface) -> R,
    ) -> R {
        let (produced, grew) = self.native.lend(borrow);
        if grew {
            self.note_alloc(AllocSite::ComposeDst);
        }
        produced
    }

    /// 表示バッファとの交代——合成先席と表示バッファを入れ替える。
    ///
    /// **k の値に依らず常にこの経路**で、表示へ載せるのは原寸そのもの。コピーも確保も起きない
    /// （`Vec` の所有ごと交換する）。交代後は「いま合成した中身」が表示バッファに、「回収した容量」が
    /// 合成先席になる。付随して動かす状態は 1 つも無い——確保の観測は実体の容量から直接読むため、
    /// 交代で取り違える値が存在しない。
    pub(super) fn swap_native_scratch(&mut self, display: &mut ComposedSurface) {
        self.native.swap(display);
    }

    /// 表示バッファを整える。
    ///
    /// `recycled` は [`ComposeCache::take_recycled`] が返す追い出しエントリで、**合成成功後にのみ**取る
    /// こと（呼び手の責務）。回収できていればその容量をそのまま表示バッファとして返し、エントリが
    /// 束ねていたマスクを第 2 要素で返す——呼び手はそれを [`FrameBudget::regenerate_mask`] へ渡し、
    /// 輪番の空きスロットとして戻す。
    ///
    /// 回収が成立しなかった場合（キャッシュにまだ空きがある暖機中・[`invalidate_all`] 直後）は空の
    /// バッファから始める。これは交代で合成先席へ入り、次の適用の合成で外形まで伸びる＝**そこで必ず
    /// 確保が起きる**が、計数は伸長が起きる席メソッド（[`swap_native_scratch`] 経由の [`native_scratch`]）
    /// 側が容量差として拾う。ここで先回りして数えると二重になる。
    ///
    /// 状態を持たない（`self` を取らない関連関数）のは、確保の観測が**実体の容量**から読めるため。
    /// 回るバッファは 4 本あり、「新しい実体で回り直す」ことを器のフィールドへ記録すると取り違えの
    /// 元になる。
    ///
    /// [`ComposeCache::take_recycled`]: crate::cache::ComposeCache::take_recycled
    /// [`invalidate_all`]: crate::cache::ComposeCache::invalidate_all
    /// [`native_scratch`]: FrameBudget::native_scratch
    /// [`swap_native_scratch`]: FrameBudget::swap_native_scratch
    pub(super) fn display_buffer(
        recycled: Option<CacheEntry>,
    ) -> (ComposedSurface, Option<Arc<AlphaMask>>) {
        match recycled {
            // 回収するのはバッファの確保だけ（新しいエントリの原寸は今回の合成が決める）。表示記録
            // （`display`）は回収の対象外——原寸バイトと 1 対 1 ゆえ次の合成が作り直し、保持しているのは
            // COM 参照だけで確保の使い回しには関与しない。
            Some(CacheEntry {
                composed,
                mask,
                display: _,
            }) => (composed, Some(mask)),
            None => (ComposedSurface::default(), None),
        }
    }

    /// マスクを再生成して `Arc` で返す。
    ///
    /// `retired` は [`display_buffer`] が回収エントリから外したマスク——追い出されたエントリが束ねて
    /// いたマスクで、下流（`AlphaMaskResource`）は既により新しいマスクへ差し替わっているため単独所有
    /// である。これを次の空きスロットとして受け取り、代わりにいま在る空きを再生成先として取り出す。
    /// これで輪番が決定論的に回る（回る本数はキャッシュ容量＋1）。
    ///
    /// 単独所有が成立すれば `Arc::get_mut` で中身を借りて `regenerate_from_pbgra32` で作り直す。
    /// 成立しない境界——立ち上がり（空きスロットがまだ無い）・第三者が参照を握っている——では新規に
    /// 確保し、**必ず [`AllocSite::Mask`] を計数する**（黙って確保しない）。
    ///
    /// # in-place 再生成も「確保しない」とは限らない
    ///
    /// `regenerate_from_pbgra32` の `resize(詰め長, 0)` は詰め長がバッファの**容量**を超えれば伸び、
    /// それは新規確保そのもの（寸法拡大で必ず起きる）。ゆえに輪番は再生成の前後で
    /// [`AlphaMask::packed_capacity`] を読み比べ、増えたときは in-place 経路でも [`AllocSite::Mask`] を
    /// 計数する（[`seat::MaskRotation`]）。回る実体は 1 適用ずつずれて再生成されるため、寸法拡大の
    /// 代金は本数ぶんの適用に分かれて 1 件ずつ現れる。
    ///
    /// 本器が保持する `Arc` は常に高々 1 本（空きスロット）で、残りはキャッシュのエントリが握る。
    /// 単独所有が不成立だったスロットは輪番から外れ（所有者は他に居るので解放はされない）、次の適用で
    /// `retired` が新しい空きになって輪番は自力で立ち直る。
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
        let (mask, allocated) = self.mask.regenerate(retired, pixels, width, height, stride);
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
    #[cfg(test)]
    pub(super) fn cumulative(&self) -> &BudgetCounters {
        &self.total
    }

    /// 合成先の常設席がいま抱えているバイト列の先頭位置（**テストの観測口・製品経路に消費者なし**）。
    ///
    /// 定常アロケーション 0 の檻の「native scratch ポインタ不変」のための口。席（[`seat::SurfaceSeat`]）は
    /// 計数シームを 1 箇所に保つため私有で、presenter 側の檻（`presenter_budget_steady_state_tests.rs`）は
    /// 本モジュールの子ではないので、席の実体へ届く道が他に無い。読めるのは番地だけで、席の中身にも計数
    /// にも触れない。`#[cfg(test)]` ゆえ製品ビルドには存在せず、[`AllocSite`] も perf サマリ行の
    /// フィールド集合も動かさない（`tools/perf/judge-perf.py` との契約は不変）。
    ///
    /// この口自体に検出力は無く、検出力を持つのはこの値を読む側の assert である。どの assert がどの
    /// 誤実装をどれだけ赤にしたかの実測は `presenter_budget_steady_state_tests.rs` の
    /// 「番地の主張はどこまで効くか」節に登記してある。
    #[cfg(test)]
    pub(super) fn native_scratch_ptr(&self) -> *const u8 {
        self.native.get().bytes().as_ptr()
    }
}

#[cfg(test)]
#[path = "budget_tests.rs"]
mod tests;
