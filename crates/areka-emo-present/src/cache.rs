//! 合成メモ（`ComposeCache`）＝合成入力（surface id ＋ bind 集合 ＋ pattern 状態）→ 描く計画・
//! 書き出しの控え（原寸の合成面）の組の、容量 3・LRU 置換のメモ化スロット表。
//!
//! 上流 `areka-emo-compose` の合成は `(surface_id, BindSet)` の純粋関数なので、キーが合成入力の全体を
//! 捕捉しないとキャッシュは正しくない（surface id だけをキーにすると、同一 surface で bind 集合だけ
//! 異なる着せ替え・まばたきが古い合成結果に衝突する）。同一入力なら再利用し、1 ビットでも違えば必ずミスする。
//!
//! # 容量 3・LRU
//!
//! 容量 1 では、キャラ面（`TargetId(0)`／`surface_id=1000`）の 1066 適用がすべて引き当て外れだった
//! （25 分の長時間走行・命中率 0.0%）。実走行の適用列を LRU で再生した命中率は 1→0.0%／2→10.5%／
//! 3→56.2%／4→56.5%／8→57.6%／64→68.5% で、膝は 3（emo2 のまばたきが 3 コマの繰り返しのため）。
//! この数字は LRU 再生で得たので、置換方式を変えると根拠が成り立たない。全保持は、pattern 状態込みの
//! キー空間が走行全体で 100 個規模（実測 103 個）になり堆積するので採らない。メモリは 1 対象あたり
//! 原寸 3 件ぶん（例: 382×547×4＝835,816 バイト × 3）。
//!
//! # 表示スケール k はキーに参加しない
//!
//! [`CacheEntry::plan`] と [`CacheEntry::composed`] は native 原寸で、拡大率 k は wintf の描画経路
//! （`render_surface` の `SetTransform`）が変換行列で掛ける。
//! よって DPI 変化で k だけが変わった適用は同じエントリにヒットし（再合成しない）、k は無効化の契機にもならない。
//!
//! # 責務
//!
//! 保持・引き当て・追い出し・全無効化だけを担う純粋な状態層。GPU に触れない。ミス時の計画・合成は
//! 提示段（`presenter`）の責務で、[`insert`] は計画と合成面を同時に受け取るので組は崩れない。UI スレッド専有（`EmoPresenter` が NonSend）でロックを持たない。
//! 無効化はアトラス再構築・ghost 再読込用の [`invalidate_all`] のみ。
//!
//! [`insert`]: ComposeCache::insert
//! [`invalidate_all`]: ComposeCache::invalidate_all

use areka_emo_compose::{BindSet, ComposedSurface, DrawPlan, PatternState};

/// キャッシュエントリ＝描く計画と原寸の合成面の原子の組。
///
/// 2 つは同じ合成入力から作る。1 エントリへ束ねてあるので、surface 切替に伴う組の入替は表の操作 1 回で原子的に起きる。
/// 原寸の外形は `composed.width()`／`composed.height()` で、別フィールドに持たない。
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// 画面の節へ写す描く計画（表示の真実源）。
    pub plan: DrawPlan,
    /// premultiplied BGRA・native 原寸の合成結果（書き出しの出どころ）。
    pub composed: ComposedSurface,
}

/// キャッシュキー＝合成入力（surface id ＋ bind 集合 ＋ pattern 状態）。
///
/// `EmoWorld`／`AtlasTable` は target 構築時に固定（変わるときは [`ComposeCache::invalidate_all`] が走る）
/// なのでキーに含めない。表示スケール k も含めない。`PatternState` の等価は内部 `BTreeMap` の昇順で
/// 安定するので、挿入順に依らない。
#[derive(Debug, Clone, PartialEq, Eq)]
struct ComposeKey {
    surface_id: u32,
    binds: BindSet,
    pattern: PatternState,
}

/// 保持するエントリ数の上限。
///
/// 命中率の膝がここにあり、4 へ広げても伸びは 0.3 ポイント（8 でも 1.4・64 でも 12 ポイント）。
/// 変更は開発者の裁定を通す。
pub(crate) const CAPACITY: usize = 3;

/// 合成入力 → [`CacheEntry`] の容量 [`CAPACITY`]・LRU 置換メモ化表。
///
/// 内部ロックを持たない。ミス時の計画・合成・挿入は提示段の責務。
///
/// `entries` は最近使用の昇順（先頭が次の追い出し対象）。順序を動かすのは [`touch`]（ヒット）と
/// [`insert`] だけで、[`get`] は動かさない（読み取りだけの照会で置換順が変わると、観測が置換順を書き換えてしまう）。
/// FIFO へ替えると、容量 3 の命中率 56.2% の根拠（LRU 再生）が消える。
///
/// 表の確保は構築時の 1 回だけで、以後 [`CAPACITY`] を超えないので毎コマ経路で伸びない。
///
/// [`take_recycled`]: ComposeCache::take_recycled
/// [`invalidate_all`]: ComposeCache::invalidate_all
/// [`touch`]: ComposeCache::touch
/// [`get`]: ComposeCache::get
/// [`insert`]: ComposeCache::insert
#[derive(Debug)]
pub struct ComposeCache {
    /// キー（合成入力）と結果の対を**最近使用の昇順**で持つ（先頭＝次の追い出し）。
    entries: Vec<(ComposeKey, CacheEntry)>,
}

impl ComposeCache {
    /// 空の表を構築する（容量 [`CAPACITY`] 件ぶんの席を 1 回だけ確保する）。
    pub fn new() -> Self {
        Self {
            entries: Vec::with_capacity(CAPACITY),
        }
    }

    /// キーの位置（最近使用の昇順の添字）を引く。
    fn position(&self, surface_id: u32, binds: &BindSet, pattern: &PatternState) -> Option<usize> {
        self.entries.iter().position(|(key, _)| {
            key.surface_id == surface_id && key.binds == *binds && key.pattern == *pattern
        })
    }

    /// 保持しているエントリ数（**テストの観測口・製品経路に消費者なし**）。
    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries.len()
    }

    /// 表が満杯なら最も古い引き当ての 1 件だけを追い出して返す（呼び手が計画と合成面の確保を
    /// 使い回すための口）。
    ///
    /// 空きがあるあいだは `None` を返し表に触れない（生きているエントリを剥がすと命中率が落ちる）。
    /// 暖機中は回収が成立せず、呼び手が新しいバッファを起こす。
    ///
    /// # 呼出は合成成功後に限る
    ///
    /// 合成が失敗し得る位置で呼ぶと、失敗時に生きているエントリが 1 件消えたまま残り、
    /// 「合成失敗時の表示・キャッシュは適用前のまま」が壊れる。呼び手（`presenter/show.rs` のミス経路）が
    /// 守る契約で、本層は成否を知り得ないので強制しない。
    pub fn take_recycled(&mut self) -> Option<CacheEntry> {
        if self.entries.len() < CAPACITY {
            return None;
        }
        Some(self.entries.remove(0).1)
    }

    /// 描く計画と原寸の合成面を、合成入力鍵の原子の組として挿入し、最近使用の末尾へ置く。
    ///
    /// 同一キーが在れば対ごと置き換える。満杯で新しいキーなら最も古い 1 件を捨てる（捨てるだけなので、
    /// 確保を回収したい呼び手は先に [`take_recycled`] を呼ぶ）。
    ///
    /// `plan` と `composed` は native 原寸でなければならない（拡大は wintf の描画経路が掛ける）。
    ///
    /// [`take_recycled`]: ComposeCache::take_recycled
    pub fn insert(
        &mut self,
        surface_id: u32,
        binds: BindSet,
        pattern: PatternState,
        plan: DrawPlan,
        composed: ComposedSurface,
    ) {
        // 同一キーの再挿入は席を外し、以下の push で末尾へ戻す。
        if let Some(at) = self.position(surface_id, &binds, &pattern) {
            self.entries.remove(at);
        } else if self.entries.len() >= CAPACITY {
            // 満杯かつ新しいキー: 最も古い引き当てを捨てる。
            self.entries.remove(0);
        }
        let key = ComposeKey {
            surface_id,
            binds,
            pattern,
        };
        self.entries.push((key, CacheEntry { plan, composed }));
    }

    /// 合成入力で引き当て、ヒットしたらそのエントリを最近使用の末尾へ引き上げる。戻り値はヒットしたか否か。
    ///
    /// LRU の順序を動かす唯一の引き当て口で、`presenter/show.rs` の 1 適用 1 回の引き当て点が通る。
    /// ここが [`get`] に替わると置換は静かに FIFO へ退化し、計数もバイト等価も検出しないので、
    /// 専用の檻（`cache_tests.rs` の LRU/FIFO 弁別檻と `presenter_cache_capacity_tests.rs`）で固定してある。
    /// 判定規則は [`get`] と同じ [`Self::position`]。`bool` を返すのは、呼び手が可変借用を持ち越さずに済むため。
    ///
    /// [`get`]: ComposeCache::get
    pub fn touch(&mut self, surface_id: u32, binds: &BindSet, pattern: &PatternState) -> bool {
        match self.position(surface_id, binds, pattern) {
            Some(at) => {
                // 要素の move のみで確保は起きない（毎コマ経路）。
                let entry = self.entries.remove(at);
                self.entries.push(entry);
                true
            }
            None => false,
        }
    }

    /// 合成入力が保持中のどれかと完全一致するときのみエントリを返す。最近使用順は動かさない（動かすのは [`touch`]）。
    ///
    /// 同一適用内の再照会（`show.rs` の装着・反映の直前・`read.rs` の読み戻し）と観測（檻）がこの口を使うので、
    /// 読み取りが置換順を書き換えてはならない。surface id・bind 集合・pattern 状態のいずれかが 1 ビットでも
    /// 違えばミス（呼び手は再合成する）。表示スケール k は判定に関与しない。
    ///
    /// [`touch`]: ComposeCache::touch
    pub fn get(
        &self,
        surface_id: u32,
        binds: &BindSet,
        pattern: &PatternState,
    ) -> Option<&CacheEntry> {
        self.position(surface_id, binds, pattern)
            .map(|at| &self.entries[at].1)
    }

    /// 保持中のエントリを全て破棄する（アトラス再構築・ghost 再読込時の唯一の無効化口）。
    ///
    /// k 変化はここを通さない（k はキーにも面にも現れない）。`clear` は容量を縮めないので、
    /// 以後も毎コマ経路で表が伸び直さない。
    pub fn invalidate_all(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
#[path = "cache_tests.rs"]
mod tests;
