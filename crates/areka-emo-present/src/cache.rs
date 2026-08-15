//! 合成メモ（`ComposeCache`）＝ **合成入力（surface id ＋ bind 集合 ＋ pattern 状態）＋表示スケール k**
//! → 表示バッファ・当たり判定マスク対の**容量 3・LRU 置換**のメモ化スロット表。
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
//! 約 3.44MB → 約 10.3MB（1 エントリ＝表示寸バッファ 764×1094×4＝3,343,264 バイト＋詰めマスク
//! 約 105KB）である。
//!
//! 全保持（無制限）を採らない理由は初版のまま生きている——pattern 状態込みのキー空間は
//! 走行全体で 100 個規模へ膨らみ（実測 103 個）、無制限保持は 340MB 級の堆積になる。
//! 上限つきの LRU は「状態が変わらない間だけ前回画像を継続する」初版の戦略を、まばたきの
//! **1 周期ぶん**へ広げたものである。
//!
//! # 表示スケール k のキー参加（要件 2.4/4.1・設計 D6）
//!
//! 保持する [`CacheEntry::composed`] は **k 適用済みの表示用サーフェス**（原寸合成結果を
//! [`ScaleRatio`] 倍へリサンプルしたもの）であり、[`CacheEntry::mask`] は**その k 寸バイト由来**である。
//! ゆえに k は合成入力と同格のキー要素でなければならない——さもなくば DPI の異なるモニタへ窓を移した
//! 直後（要件 4.1）や、k 変化を跨ぐ surface／pattern 切替（要件 2.4）に、**旧 k の絵とマスク**が
//! ヒットしてしまう。「キー＝合成入力の全体」不変条件は「**合成入力＋表示スケール**」へ拡張され、
//! 1 ビットでも異なれば必ずミスという規律そのものは不変である。
//!
//! k 変化は**キー相違＝ミス**として表現し、命令的な全無効化（`invalidate_all`）で二重化しない
//! （設計 D6）。ミスした呼び手（提示段）が再合成＋再サンプルして再挿入する——k 変化は稀イベント
//! ゆえこの再計算は許容される。k を**キー要素として弁別する**形は容量 3 でも不変である（同 D6）:
//! 旧 k のエントリは別キーとして表に残り得るが、引き当ては完全一致のみゆえ**旧 k の絵が新しい k の
//! 表示に載ることは無い**。残った旧 k のエントリは LRU でいずれ追い出される（DPI を戻したときに
//! 命中し得るのは副次的な利得であって、正しさはキー完全一致だけに依っている）。
//!
//! [`AlphaMask`] を表示バッファと同一エントリ（[`CacheEntry`]）へ束ねる純粋な状態層である点は
//! 従来どおり。マスクが k 適用済みバイト由来になることで、[`AlphaMask`] の物理 px 契約は
//! **マスク生成コードを一切変更せずに** k 追従と整合する（設計「emo-present / cache.rs」）。表示
//! バッファと当たり判定マスクを 1 エントリに封じることで、対の入替がスロット操作 1 回で原子的に
//! 起きる（R2.4）。
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
//! 本表は合成器（`Composer`）もリサンプラも所有しない。ミス時に合成し、k 倍へリサンプル
//! してから [`insert`] を呼ぶのは提示段（`presenter`）の責務であり、本層は「保持・引き当て・
//! 追い出し・全無効化」だけを担う純粋な状態（設計 §State Management）である。k の導出・適用も本層は行わず、
//! 与えられた [`ScaleRatio`] を**キー要素として弁別するだけ**である。UI スレッド専有
//! （`EmoPresenter` が NonSend）ゆえロックを持たない。無効化はアトラス再構築・ghost 再読込用の
//! [`invalidate_all`] のみ提供する（R4.3）——k 変化はキー相違で表現されるため無効化を要さない。
//!
//! [`insert`]: ComposeCache::insert
//! [`invalidate_all`]: ComposeCache::invalidate_all

use std::sync::Arc;

use areka_emo_compose::{BindSet, ComposedSurface, PatternState, ScaleRatio};
use wintf::ecs::widget::bitmap_source::AlphaMask;

/// キャッシュエントリ＝表示バッファと当たり判定マスクの原子対（R2.4 の構造的担保）。
///
/// `composed` は表示（WUC アップロード）の真実源、`mask` はさわり判定の真実源であり、両者は
/// **同一 `composed.bytes()` 由来**である（呼び手が対で作り [`ComposeCache::insert`] へ同時に渡す）。
/// 1 エントリへ束ねてあるため、surface 切替に伴う対の入替は表の操作 1 回で原子的に起きる。
///
/// 保持されるのは**キーの `scale` を適用済みの表示用サーフェス**（＝物理 px 寸）であり、`mask` も
/// その k 寸バイト由来である（要件 2.4/4.1・設計 D6）。エントリの構造・生成コードは k 導入で
/// 一切変わらない——k は原寸を差し替えるのでなく、キーで別エントリとして弁別される。
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// premultiplied BGRA・表示の真実源（k 適用済みの表示寸）。
    pub composed: ComposedSurface,
    /// `composed.bytes()`（＝k 寸バイト）から呼び手が 1 回だけ生成した当たり判定マスク・
    /// さわり判定の真実源。`AlphaMask` の物理 px 契約と無修正で整合する。
    ///
    /// [`Arc`] 共有形なのは下流（hit-test）への供給を複製から参照カウント増へ落とすためであり
    /// （設計 D3・クレート内部の表現変更）、`composed` との原子対という意味論は不変である。
    pub mask: Arc<AlphaMask>,
    /// このエントリの **k 適用前**の合成外形（`(width, height)`）＝照会契約の native 原寸。
    ///
    /// `composed`／`mask` が k 適用済みの表示寸を持つのに対し、こちらは原寸である。両者は
    /// `scaled_extent(native) == composed の外形` の関係にあるが、丸めを含むため逆算はできない
    /// ——ゆえに値として控える。
    ///
    /// # なぜ**エントリの中**なのか（容量 3 で移した・要件 7.1）
    ///
    /// 容量 1 の頃は「保持しているエントリ＝直前に挿入したエントリ」だったため、提示段が
    /// target 側の 1 個のフィールド（`cached_native`）へ挿入と同時に控えれば対が保てた。容量 3 では
    /// **ヒットしたエントリが直前の挿入とは限らない**——別の面のエントリに命中した回に、直前に
    /// 挿入した面の原寸を照会契約へ返すことになる（原寸が面ごとに違えば画面と乖離する）。
    /// 原寸を絵・マスクと同じ入れ物へ移すことで、対の維持が構造で決まる。
    pub native: (u32, u32),
}

/// キャッシュキー＝エントリを一意に定める全体（合成入力 ＝ surface id ＋ bind 集合 ＋ pattern 状態、
/// ＋ 表示スケール k）。
///
/// `EmoWorld`／`AtlasTable` は target 構築時に固定（変わるときは [`ComposeCache::invalidate_all`] が
/// 走る契約）ゆえキーに含めない。seriko のアニメ pattern 状態（[`PatternState`]）は合成入力の第一級
/// 要素として本キーに含める（R5.2）。さらに表示スケール `scale`（[`ScaleRatio`]）も同格のキー要素で
/// ある（要件 2.4/4.1）——エントリが保持するのは k 適用済みサーフェスとその bytes 由来マスクゆえ、
/// k が違えば別の絵・別のマスクだからである。すなわち「キー＝合成入力の全体」不変条件は
/// 「**合成入力＋表示スケール**」へ拡張され、1 ビットでも異なれば（surface id・binds・pattern・
/// scale のいずれか）ミスして再合成する規律は不変である。
///
/// `PatternState` の等価は内部 `BTreeMap` の正準（昇順）順序で安定する（task 2）ため、挿入順に
/// 依存せず決定論的にヒット判定できる。`ScaleRatio` の等価は**既約正準形で厳密**（構築時に gcd
/// 約分・`ScaleRatio::new`）ゆえ、`120/96` と `5/4` のように表記が異なるだけの同値 k は同一キーへ
/// 畳まれる——k の作り方（DPI 比のまま渡すか約分済みで渡すか）でヒット/ミスが揺れない。
#[derive(Debug, Clone, PartialEq, Eq)]
struct ComposeKey {
    surface_id: u32,
    binds: BindSet,
    pattern: PatternState,
    scale: ScaleRatio,
}

/// 保持するエントリ数の上限（**開発者裁定 2026-08-15**・要件 7.1）。
///
/// 実測の根拠は本モジュール冒頭 §容量は 1 → 3 と `remeasure-2026-08-15.md` §4。命中率の膝が
/// ここにあり、4 以上へ広げても伸びは 1 ポイント未満である（3→64 でも 12 ポイント）。
///
/// **この定数の変更は要件 7.1 の裁定ゲートを通す。** 自律ループが単独で動かしてよい値ではない。
const CAPACITY: usize = 3;

/// 合成入力＋表示スケール → [`CacheEntry`] の**容量 [`CAPACITY`]・LRU 置換**メモ化表。
///
/// UI スレッド専有の純粋な状態容器で、内部ロックを持たない。キー（合成入力＋k）と結果の対の
/// 保持・完全一致引き当て、追い出し（[`take_recycled`]）、およびアトラス再構築・ghost 再読込時の
/// 破棄（[`invalidate_all`]）だけを担う。合成器・リサンプラは所有せず、ミス時の合成・k 倍
/// リサンプル・挿入は提示段の責務である（本モジュール冒頭 §責務分界）。
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
    /// キー（合成入力＋表示スケール）と結果の対を**最近使用の昇順**で持つ（先頭＝次の追い出し）。
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
    fn position(
        &self,
        surface_id: u32,
        binds: &BindSet,
        pattern: &PatternState,
        scale: ScaleRatio,
    ) -> Option<usize> {
        self.entries.iter().position(|(key, _)| {
            key.surface_id == surface_id
                && key.binds == *binds
                && key.pattern == *pattern
                && key.scale == scale
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

    /// 表示用サーフェスと**生成済みの**当たり判定マスクを、合成入力（surface id ＋ bind 集合 ＋
    /// pattern 状態）＋表示スケール `scale` 鍵の原子対として挿入し、**最近使用の末尾**へ置く。
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
    /// キー要素である（R5.2）。`scale` は表示スケール k（要件 2.4/4.1）で、渡す `composed` は
    /// **その k を適用済みの表示用サーフェス**でなければならない——マスクはそのバイト由来である
    /// ため、k と `composed` の不一致はそのまま「絵とさわり判定の寸法不一致」になる。本層は合成器も
    /// リサンプラも持たない（k の適用は提示段の責務・本モジュール冒頭 §責務分界）。
    ///
    /// `native` は **k 適用前**の合成外形で、絵・マスクと同じエントリへ束ねて保持する
    /// （[`CacheEntry::native`]・照会契約の原寸がヒットしたエントリと必ず対になるため）。
    ///
    /// 挿入したエントリへの共有参照を返す（提示段がそのまま表示・マスク同期へ用いる）。
    pub fn insert(
        &mut self,
        surface_id: u32,
        binds: BindSet,
        pattern: PatternState,
        scale: ScaleRatio,
        composed: ComposedSurface,
        mask: Arc<AlphaMask>,
        native: (u32, u32),
    ) -> &CacheEntry {
        // 同一キーの再挿入は重複を作らずその席を外す（以下の push で末尾＝最近使用へ戻る）。
        if let Some(at) = self.position(surface_id, &binds, &pattern, scale) {
            self.entries.remove(at);
        } else if self.entries.len() >= CAPACITY {
            // 満杯かつ新しいキー: 最も古い引き当てを捨てる（回収したい呼び手は先に take_recycled）。
            self.entries.remove(0);
        }
        let key = ComposeKey {
            surface_id,
            binds,
            pattern,
            scale,
        };
        self.entries.push((
            key,
            CacheEntry {
                composed,
                mask,
                native,
            },
        ));
        // 直前に押し込んだ末尾は必ず存在する。
        &self.entries.last().expect("entry was just inserted").1
    }

    /// 合成入力＋表示スケールで引き当て、**ヒットしたらそのエントリを最近使用の末尾へ引き上げる**。
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
    pub fn touch(
        &mut self,
        surface_id: u32,
        binds: &BindSet,
        pattern: &PatternState,
        scale: ScaleRatio,
    ) -> bool {
        match self.position(surface_id, binds, pattern, scale) {
            Some(at) => {
                // 末尾＝最近使用。要素の move のみで確保は起きない（毎コマ経路の上を走る）。
                let entry = self.entries.remove(at);
                self.entries.push(entry);
                true
            }
            None => false,
        }
    }

    /// 合成入力（surface id ＋ bind 集合 ＋ pattern 状態）と表示スケール `scale` が保持中のどれかと
    /// **完全一致**するときのみエントリを返す。**最近使用順は動かさない**（動かすのは [`touch`]）。
    ///
    /// 順序を動かさないのは、同一適用内の再照会（`show.rs` の供給面生成・アップロード直前）と
    /// 観測（檻）がこの口を使うためである——読み取りが置換順を書き換えると、檻は自分の観測で
    /// LRU の状態を壊し、本番は 1 適用で何度も「最近使用」を打ち直すことになる。
    ///
    /// [`touch`]: ComposeCache::touch
    ///
    /// surface id・bind 集合・pattern 状態・表示スケールのいずれかが 1 ビットでも異なればミス
    /// （＝呼び手は再合成＋再サンプルする）。これが「同一 surface の着せ替え切替・アニメ pattern
    /// 進行で古い絵を返さない」（R5.2）ことに加え、「**k 変化後に旧 k の絵とマスクを返さない**」
    /// （要件 2.4/4.1・設計 D6）ことの構造的担保である。k 変化はここでのキー相違だけで表現し、
    /// [`invalidate_all`] による命令的な二重化は行わない。
    ///
    /// `pattern` 等価は [`PatternState`] の `Eq`（正準順序で安定・task 2）、`scale` 等価は
    /// [`ScaleRatio`] の `Eq`（既約正準形で厳密）に従う。
    ///
    /// [`invalidate_all`]: ComposeCache::invalidate_all
    pub fn get(
        &self,
        surface_id: u32,
        binds: &BindSet,
        pattern: &PatternState,
        scale: ScaleRatio,
    ) -> Option<&CacheEntry> {
        self.position(surface_id, binds, pattern, scale)
            .map(|at| &self.entries[at].1)
    }

    /// 保持中のエントリを**全て**破棄する（アトラス再構築・ghost 再読込時の唯一の無効化口・R4.3）。
    ///
    /// 以後あらゆるキーがミスし、提示段が再合成して再挿入する。**k 変化はここを通さない**——
    /// キー等価で表現できるものを命令で二重化しない（設計 D6）。表そのものの確保は保持する
    /// （`clear` は容量を縮めない）ため、無効化の後も毎コマ経路で表が伸び直すことはない。
    pub fn invalidate_all(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
#[path = "cache_tests.rs"]
mod tests;
