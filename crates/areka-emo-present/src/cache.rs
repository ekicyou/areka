//! 合成メモ（`ComposeCache`）＝ **合成入力（surface id ＋ bind 集合 ＋ pattern 状態）**
//! → 原寸の合成面・その原寸バイト由来の当たり判定マスク・表示記録の三つ組の**容量 3・LRU 置換**の
//! メモ化スロット表。
//!
//! 上流 `areka-emo-compose` の合成は `(surface_id, BindSet)` の純粋関数であり、**キーが合成入力の
//! 全体を捕捉しない限りキャッシュは正しくない**（surface id のみをキーにすると、同一 surface で
//! bind 集合だけ異なる着せ替え・まばたきが古い合成結果に衝突する）。本表は合成入力と結果の対を
//! 最大 [`CAPACITY`] 件保持し、**同一入力なら再利用・1 ビットでも異なれば必ずミス（＝再合成）**を
//! 構造で担保する。
//!
//! # 容量は 1 → 3（2026-08-15 開発者裁定・要件 7.1）
//!
//! 初版（`completed/areka-P0-emo-present` R4.1）は容量 1 の「直前 1 件メモ」だった。理由は
//! 「seriko のアニメ pattern 状態を合成入力へ加えると状態空間が膨張し、全保持はメモリ堆積と
//! 低ヒット率の二重苦になる」という見積もりである。**この見積もりのうち低ヒット率の側が実測で
//! 否定された**（`areka-P0-recompose-budget` task 7.1／7.2・`remeasure-2026-08-15.md`）:
//!
//! - キャラ面（`TargetId(0)`／`surface_id=1000`）は容量 1 では **1066 適用すべて引き当て外れ**
//!   （25 分の長時間走行・命中率 0.0%）。1 コマ適用の **80%** が毎コマの作り直しだった
//! - 実走行の適用列を LRU で再生した命中率は 1→**0.0%**／2→10.5%／**3→56.2%**／4→56.5%／
//!   8→57.6%／64→68.5%。**膝は容量 3 にある**（emo2 のまばたきが 3 コマの繰り返しであるため）
//!
//! ゆえに容量を 3・置換方式を **LRU** とする（命中率の材料が LRU 再生であるため、置換方式を
//! 変えると裁定の根拠そのものが成立しない）。**残る 2 点の承認済み意味論——キー完全一致のみ
//! ヒット・表示バッファとマスクの原子対——は一切変わらない。** 代金はメモリで、1 対象あたり
//! 原寸 3 件ぶん（例: 382×547×4＝835,816 バイト × 3 ＋ 原寸の詰めマスク 3 枚）である。
//!
//! 全保持（無制限）を採らない理由は初版のまま生きている——pattern 状態込みのキー空間は
//! 走行全体で 100 個規模へ膨らみ（実測 103 個）、無制限保持は大きな堆積になる。
//! 上限つきの LRU は「状態が変わらない間だけ前回画像を継続する」初版の戦略を、まばたきの
//! **1 周期ぶん**へ広げたものである。
//!
//! # 表示スケール k はキーに参加しない（`areka-P0-present-gpu-transform-scale`・要件 5.1／5.2）
//!
//! 保持する [`CacheEntry::composed`] は **native 原寸**の合成結果であり、[`CacheEntry::mask`] は
//! **その原寸バイト由来**である。拡大率 k は wintf の描画経路（`render_surface` の `SetTransform`）
//! が変換行列の係数として掛けるものであって、面の中身にも表示記録にも現れない。ゆえに k は
//! キー要素ではなく、**窓の DPI が変わって k だけが変わった適用は同じエントリにヒットする**
//! （要件 5.3・再合成しない）。「k 変化後に旧 k の絵が表示に載らない」はエントリが k を持たない
//! 構造そのもので担保される（要件 5.6）——載せ替わるのは変換の係数だけである。
//!
//! かつて（完了 spec `areka-P0-emo-dpi-scaling` の設計 D6）は k 適用済みの面を保持していたため
//! k をキー要素にしていたが、本仕様（裁定 D・2026-09-11）でその形は撤去された。
//!
//! # 表示記録（[`CacheEntry::display`]）も同じ入れ物に束ねる
//!
//! エントリは絵・マスクに加えて**表示記録**——原寸バイトを描く閉じた [`GraphicsCommandList`]——を
//! 保持する。本層は記録を**生成しない**（GPU を知らない）: [`insert`] で与えられた閉じたリストを
//! そのまま持つだけであり、記録内容は表示スケール k を含まない。ゆえに GPU の無い檻では
//! [`GraphicsCommandList::empty`] を渡せばよく、保持・引き当て・追い出し・全無効化の意味論は
//! 記録の有無に一切依らない。
//!
//! [`AlphaMask`] を表示バッファと同一エントリ（[`CacheEntry`]）へ束ねる純粋な状態層である点は
//! 従来どおり。表示バッファと当たり判定マスクを 1 エントリに封じることで、対の入替がスロット
//! 操作 1 回で原子的に起きる（R2.4・要件 4.4）。
//!
//! # マスク生成点は挿入の外（`areka-P0-recompose-budget` 設計 D4）
//!
//! マスク生成（[`AlphaMask::from_pbgra32`]）は挿入 API の内側から**呼び手側の予算シームへ移った**。
//! 本層は生成済みの [`Arc<AlphaMask>`] を [`insert`] の引数で受け取り、表示バッファと対で束ねる
//! だけである。「1 apply につきマスク 1 回生成・表示バッファと原子対で挿入」の契約は apply 単位で
//! 不変であり（[`insert`] が表示バッファと `Arc` マスクを**同時に**受け取るため対の崩れは起き得ない）、
//! 「表示のたびに再生成しない」（R2.1）は本層側では「[`insert`] はミス時にしか呼ばれない」という
//! 引き当てフローの形がそのまま担保する。エントリ側を [`Arc`] にしたのは、下流（hit-test）への
//! 供給を複製から参照カウント増へ落とすためのクレート内部の表現変更であり、原子対の意味論は不変。
//!
//! # 責務分界（合成は持たない・純粋状態層）
//!
//! 本表は合成器（`Composer`）を所有しない。ミス時に合成し、表示を記録してから [`insert`] を
//! 呼ぶのは提示段（`presenter`）の責務であり、本層は「保持・引き当て・追い出し・全無効化」だけを
//! 担う純粋な状態（設計 §State Management）である。UI スレッド専有（`EmoPresenter` が NonSend）ゆえ
//! ロックを持たない。無効化はアトラス再構築・ghost 再読込用の [`invalidate_all`] のみ提供する
//! （R4.3）——k 変化はキーに現れないため無効化も再合成も要さない。
//!
//! [`insert`]: ComposeCache::insert
//! [`invalidate_all`]: ComposeCache::invalidate_all

use std::sync::Arc;

use areka_emo_compose::{BindSet, ComposedSurface, PatternState};
use wintf::ecs::GraphicsCommandList;
use wintf::ecs::widget::bitmap_source::AlphaMask;

/// キャッシュエントリ＝原寸の合成面・当たり判定マスク・表示記録の原子対（R2.4 の構造的担保）。
///
/// `composed` は表示の真実源（原寸・記録の元）、`mask` はさわり判定の真実源であり、両者は
/// **同一 `composed.bytes()` 由来**である（呼び手が対で作り [`ComposeCache::insert`] へ同時に渡す）。
/// 1 エントリへ束ねてあるため、surface 切替に伴う対の入替は表の操作 1 回で原子的に起きる。
///
/// 原寸の外形は `composed.width()`／`composed.height()` そのものであり、別フィールドで二重に
/// 持たない（要件 5.1）。
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// premultiplied BGRA・**native 原寸**の合成結果（表示の真実源・照会契約の原寸の供給源）。
    pub composed: ComposedSurface,
    /// `composed.bytes()`（原寸バイト）から呼び手が 1 回だけ生成した当たり判定マスク・
    /// さわり判定の真実源（要件 4.1）。÷k の写像は wintf の `alpha_mask_hit`（物理寸の境界に
    /// 対する比例写像）が担うため、マスク側に k は入らない。
    ///
    /// [`Arc`] 共有形なのは下流（hit-test）への供給を複製から参照カウント増へ落とすためであり
    /// （設計 D3・クレート内部の表現変更）、`composed` との原子対という意味論は不変である。
    pub mask: Arc<AlphaMask>,
    /// このエントリの**表示記録**＝原寸バイトを描く閉じた [`GraphicsCommandList`]。
    ///
    /// 本層は記録を生成せず（GPU を知らない）、[`ComposeCache::insert`] で与えられたリストを
    /// 保持するだけである。記録内容に表示スケール k は含まれない——拡大は wintf の描画経路が
    /// 変換行列で掛けるため、同じ記録が任意の k の表示に使える。GPU の無い檻では
    /// [`GraphicsCommandList::empty`] を渡す。
    pub display: GraphicsCommandList,
}

/// キャッシュキー＝エントリを一意に定める全体（合成入力 ＝ surface id ＋ bind 集合 ＋ pattern 状態）。
///
/// `EmoWorld`／`AtlasTable` は target 構築時に固定（変わるときは [`ComposeCache::invalidate_all`] が
/// 走る契約）ゆえキーに含めない。seriko のアニメ pattern 状態（[`PatternState`]）は合成入力の第一級
/// 要素として本キーに含める（R5.2）。表示スケール k は**含めない**（本モジュール冒頭 §表示スケール k
/// はキーに参加しない）。
///
/// `PatternState` の等価は内部 `BTreeMap` の正準（昇順）順序で安定する（task 2）ため、挿入順に
/// 依存せず決定論的にヒット判定できる。
#[derive(Debug, Clone, PartialEq, Eq)]
struct ComposeKey {
    surface_id: u32,
    binds: BindSet,
    pattern: PatternState,
}

/// 保持するエントリ数の上限（**開発者裁定 2026-08-15**・要件 7.1）。
///
/// 実測の根拠は本モジュール冒頭 §容量は 1 → 3 と `remeasure-2026-08-15.md` §4。命中率の膝が
/// ここにあり、4 以上へ広げても伸びは 1 ポイント未満である（3→64 でも 12 ポイント）。
///
/// **この定数の変更は要件 7.1 の裁定ゲートを通す。** 自律ループが単独で動かしてよい値ではない。
const CAPACITY: usize = 3;

/// 合成入力 → [`CacheEntry`] の**容量 [`CAPACITY`]・LRU 置換**メモ化表。
///
/// UI スレッド専有の純粋な状態容器で、内部ロックを持たない。キー（合成入力）と結果の対の
/// 保持・完全一致引き当て、追い出し（[`take_recycled`]）、およびアトラス再構築・ghost 再読込時の
/// 破棄（[`invalidate_all`]）だけを担う。合成器は所有せず、ミス時の合成・表示記録・挿入は
/// 提示段の責務である（本モジュール冒頭 §責務分界）。
///
/// # 置換方式は LRU（最近最も使われていないものから追い出す）
///
/// 内部表 `entries` は**最近使用の昇順**で並ぶ——先頭が最も古い引き当て（次の追い出し対象）、
/// 末尾が直近である。順序を動かすのは [`touch`]（ヒット）と [`insert`]（挿入）の 2 つだけで、
/// [`get`] は**順序を動かさない**。読み取りだけの引き当てで置換順が変わると、観測（テスト・
/// 同一適用内の再照会）が置換順を書き換えてしまうためである。
///
/// LRU であることは裁定の根拠そのものである: 容量 3 で命中率 56.2% という材料は実走行の適用列を
/// **LRU で再生**して得たもので、FIFO（挿入順で追い出す）へ替えると数字の出所が消える。
///
/// 表の確保は構築時の 1 回だけ（`Vec::with_capacity(CAPACITY)`）で、以後 [`CAPACITY`] を
/// 超えないため**毎コマ経路で伸びない**。要素の入れ替えは `Vec` 内の move のみで確保を伴わない。
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

impl Default for ComposeCache {
    fn default() -> Self {
        Self::new()
    }
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

    /// 表が満杯なら**最も古い引き当ての 1 件だけ**を追い出して返す（容量を回収する）。
    ///
    /// 追い出されるエントリの表示バッファ（`Vec<u8>` の確保）を呼び手が次の表示バッファとして
    /// 使い回すための口である（要件 3.1・設計 D2⑵）。**空きがあるあいだは `None` を返し、
    /// 表には一切触れない**——生きているエントリを剥がすと命中率が落ち、容量 3 の裁定の前提が
    /// 崩れる。ゆえに暖機（表が埋まるまで）は回収が成立せず、その適用では呼び手が新しい
    /// バッファを起こして計数する（要件 3.2 の「一度だけ」の形）。
    ///
    /// 返り値を捨てれば従来どおり解放される。**本メソッドは完全一致・原子対という承認済み
    /// 意味論を一切変えない**（容量そのものは要件 7.1 の裁定で 1 → 3 になった・[`CAPACITY`]）。
    ///
    /// # 呼出は合成成功後に限る（設計 Flow 2 の規律）
    ///
    /// 合成が失敗し得る位置でこれを呼ぶと、失敗時に生きているエントリが 1 件消えたまま残り
    /// 「合成失敗時の表示・キャッシュは適用前のまま」という現行挙動が壊れる。**合成が成功した
    /// 後にのみ呼ぶ**ことは呼び手（`presenter/show.rs` のミス経路）が構造で保証する契約であり、
    /// 本層は強制しない（本層は合成器を持たず成否を知り得ないため・本モジュール冒頭 §責務分界）。
    ///
    /// 空の表（未挿入・[`invalidate_all`] 後）に対しては `None` を返し、状態は変わらない。
    ///
    /// [`invalidate_all`]: ComposeCache::invalidate_all
    pub fn take_recycled(&mut self) -> Option<CacheEntry> {
        if self.entries.len() < CAPACITY {
            return None;
        }
        // 先頭＝最近使用の昇順の先頭＝最も古い引き当て。キーは破棄しエントリだけを渡す。
        Some(self.entries.remove(0).1)
    }

    /// 原寸の合成面・**生成済みの**当たり判定マスク・表示記録を、合成入力（surface id ＋ bind 集合 ＋
    /// pattern 状態）鍵の原子対として挿入し、**最近使用の末尾**へ置く。
    ///
    /// 同一キーが既に在れば対ごと置き換える（重複エントリは作らない・R2.4）。表が満杯で新しい
    /// キーなら、最も古い引き当ての 1 件を捨てて席を空ける——**捨てるだけ**なので、その確保を
    /// 回収したい呼び手は先に [`take_recycled`] を呼ぶ（設計 Flow 2 の順序）。
    ///
    /// [`take_recycled`]: ComposeCache::take_recycled
    ///
    /// マスク生成（[`AlphaMask::from_pbgra32`]）は呼び手側の予算シームで行う（`recompose-budget`
    /// 設計 D4）。本メソッドは表示バッファと `Arc` マスクを**同時に**受け取るため、「1 apply に
    /// つきマスク 1 回生成・表示バッファと原子対で挿入」の契約は apply 単位で不変である。渡す
    /// `mask` は必ず**同じ `composed` の bytes 由来**でなければならない——別出所のマスクを渡すと
    /// 絵とさわり判定が食い違い、原子対の意味が失われる。
    ///
    /// `pattern` は seriko のアニメ pattern 状態（[`PatternState`]）で、`binds` と同格の合成入力
    /// キー要素である（R5.2）。`composed` は **native 原寸**の合成結果でなければならない
    /// （要件 5.1）——拡大は wintf の描画経路が掛けるため、k 適用済みの面を渡す形は存在しない。
    ///
    /// `display` は呼び手が原寸バイトから記録した**閉じた**コマンドリスト（[`CacheEntry::display`]）
    /// で、本層はそれを保持するだけである（生成しない・GPU を知らない）。GPU の無い檻では
    /// [`GraphicsCommandList::empty`] を渡す。
    ///
    /// 挿入したエントリへの共有参照を返す（提示段がそのまま表示・マスク同期へ用いる）。
    pub fn insert(
        &mut self,
        surface_id: u32,
        binds: BindSet,
        pattern: PatternState,
        composed: ComposedSurface,
        mask: Arc<AlphaMask>,
        display: GraphicsCommandList,
    ) -> &CacheEntry {
        // 同一キーの再挿入は重複を作らずその席を外す（以下の push で末尾＝最近使用へ戻る）。
        if let Some(at) = self.position(surface_id, &binds, &pattern) {
            self.entries.remove(at);
        } else if self.entries.len() >= CAPACITY {
            // 満杯かつ新しいキー: 最も古い引き当てを捨てる（回収したい呼び手は先に take_recycled）。
            self.entries.remove(0);
        }
        let key = ComposeKey {
            surface_id,
            binds,
            pattern,
        };
        self.entries.push((
            key,
            CacheEntry {
                composed,
                mask,
                display,
            },
        ));
        // 直前に押し込んだ末尾は必ず存在する。
        &self.entries.last().expect("entry was just inserted").1
    }

    /// 合成入力で引き当て、**ヒットしたらそのエントリを最近使用の末尾へ引き上げる**。
    /// 戻り値はヒットしたか否か。
    ///
    /// LRU の順序を動かす**唯一の引き当て口**であり、`presenter/show.rs` の 1 適用 1 回の
    /// 引き当て点がここを通る。ここが [`get`]（順序を動かさない読み取り）へ差し替わると、置換は
    /// 挿入順（FIFO）へ静かに退化し、容量 3 の裁定の根拠である LRU 再生の数字が実装と対応しなく
    /// なる——**その退化を計数もバイト等価も検出しない**ため、専用の檻で固定してある
    /// （`cache_tests.rs` の LRU/FIFO 弁別檻と `presenter_cache_capacity_tests.rs`）。
    ///
    /// 引き当ての判定規則は [`get`] と 1 ビットも違わない（同一の [`Self::position`] を使う）。
    /// エントリ参照ではなく `bool` を返すのは、呼び手が可変借用を持ち越さずに済むようにする
    /// ためである（同一適用内の再照会は [`get`] で足りる）。
    ///
    /// [`get`]: ComposeCache::get
    pub fn touch(&mut self, surface_id: u32, binds: &BindSet, pattern: &PatternState) -> bool {
        match self.position(surface_id, binds, pattern) {
            Some(at) => {
                // 末尾＝最近使用。要素の move のみで確保は起きない（毎コマ経路の上を走る）。
                let entry = self.entries.remove(at);
                self.entries.push(entry);
                true
            }
            None => false,
        }
    }

    /// 合成入力（surface id ＋ bind 集合 ＋ pattern 状態）が保持中のどれかと**完全一致**するときのみ
    /// エントリを返す。**最近使用順は動かさない**（動かすのは [`touch`]）。
    ///
    /// 順序を動かさないのは、同一適用内の再照会（`show.rs` の装着・反映の直前・`read.rs` の
    /// 読み戻し）と観測（檻）がこの口を使うためである——読み取りが置換順を書き換えると、檻は
    /// 自分の観測で LRU の状態を壊し、本番は 1 適用で何度も「最近使用」を打ち直すことになる。
    ///
    /// [`touch`]: ComposeCache::touch
    ///
    /// surface id・bind 集合・pattern 状態のいずれかが 1 ビットでも異なればミス（＝呼び手は
    /// 再合成する）。これが「同一 surface の着せ替え切替・アニメ pattern 進行で古い絵を返さない」
    /// （R5.2）ことの構造的担保である。表示スケール k は判定に関与しない（要件 5.3）。
    ///
    /// `pattern` 等価は [`PatternState`] の `Eq`（正準順序で安定・task 2）に従う。
    pub fn get(
        &self,
        surface_id: u32,
        binds: &BindSet,
        pattern: &PatternState,
    ) -> Option<&CacheEntry> {
        self.position(surface_id, binds, pattern)
            .map(|at| &self.entries[at].1)
    }

    /// 保持中のエントリを**全て**破棄する（アトラス再構築・ghost 再読込時の唯一の無効化口・R4.3）。
    ///
    /// 以後あらゆるキーがミスし、提示段が再合成して再挿入する。**k 変化はここを通さない**——
    /// k はキーにも面にも現れないため、無効化も再合成も要さない。表そのものの確保は保持する
    /// （`clear` は容量を縮めない）ため、無効化の後も毎コマ経路で表が伸び直すことはない。
    pub fn invalidate_all(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
#[path = "cache_tests.rs"]
mod tests;
