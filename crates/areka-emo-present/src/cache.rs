//! 合成メモ（`ComposeCache`）＝ **合成入力（surface id ＋ bind 集合 ＋ pattern 状態）＋表示スケール k**
//! → 表示バッファ・当たり判定マスク対の容量 1 メモ化スロット。
//!
//! 上流 `areka-emo-compose` の合成は `(surface_id, BindSet)` の純粋関数であり、**キーが合成入力の
//! 全体を捕捉しない限りキャッシュは正しくない**（surface id のみをキーにすると、同一 surface で
//! bind 集合だけ異なる着せ替え・まばたきが古い合成結果に衝突する）。本スロットは直前の合成入力と
//! その結果を 1 件だけ保持し、**同一入力なら再利用・1 ビットでも異なれば必ずミス（＝再合成）**を
//! 構造で担保する。多エントリ保持は採らない: seriko のアニメ pattern 状態を合成入力へ加えると
//! 状態空間が膨張し、全保持はメモリ堆積（1 エントリ＝表示寸ビットマップ）と低ヒット率の二重苦になる
//! ため、「状態が変わらない間だけ前回画像を継続する」直近 1 件こそが正しい戦略である。
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
//! ゆえこの再計算は許容される。容量 1 スロットも維持し、k 別の多エントリ保持は行わない（同 D6）。
//!
//! 挿入時に [`AlphaMask`] を **1 回だけ**生成して同一エントリ（[`CacheEntry`]）へ束ねる純粋な状態層で
//! ある点は従来どおり。マスクが k 適用済みバイト由来になることで、[`AlphaMask`] の物理 px 契約は
//! **マスク生成コードを一切変更せずに** k 追従と整合する（設計「emo-present / cache.rs」）。表示
//! バッファと当たり判定マスクを 1 エントリに封じることで、対の入替がスロット操作 1 回で原子的に
//! 起きる（R2.4）。表示のたびにマスクを再生成しない（R2.1）ことを、マスク生成を挿入 API の内側へ
//! 隠して**構造で**担保する（呼び手はマスク生成を書けない・忘れられない）。
//!
//! # 責務分界（合成は持たない・純粋状態層）
//!
//! 本スロットは合成器（`Composer`）もリサンプラも所有しない。ミス時に合成し、k 倍へリサンプル
//! してから [`insert`] を呼ぶのは提示段（`presenter`）の責務であり、本層は「保持・引き当て・
//! 全無効化」だけを担う純粋な状態（設計 §State Management）である。k の導出・適用も本層は行わず、
//! 与えられた [`ScaleRatio`] を**キー要素として弁別するだけ**である。UI スレッド専有
//! （`EmoPresenter` が NonSend）ゆえロックを持たない。無効化はアトラス再構築・ghost 再読込用の
//! [`invalidate_all`] のみ提供する（R4.3）——k 変化はキー相違で表現されるため無効化を要さない。
//!
//! [`insert`]: ComposeCache::insert
//! [`invalidate_all`]: ComposeCache::invalidate_all

use areka_emo_compose::{BindSet, ComposedSurface, PatternState, ScaleRatio};
use wintf::ecs::widget::bitmap_source::AlphaMask;

/// キャッシュエントリ＝表示バッファと当たり判定マスクの原子対（R2.4 の構造的担保）。
///
/// `composed` は表示（WUC アップロード）の真実源、`mask` はさわり判定の真実源であり、両者は
/// **同一 `composed.bytes()` 由来**である（[`ComposeCache::insert`] が挿入時に一度だけ生成し束ねる）。
/// 1 エントリへ束ねてあるため、surface 切替に伴う対の入替はスロット操作 1 回で原子的に起きる。
///
/// 保持されるのは**キーの `scale` を適用済みの表示用サーフェス**（＝物理 px 寸）であり、`mask` も
/// その k 寸バイト由来である（要件 2.4/4.1・設計 D6）。エントリの構造・生成コードは k 導入で
/// 一切変わらない——k は原寸を差し替えるのでなく、キーで別エントリとして弁別される。
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// premultiplied BGRA・表示の真実源（k 適用済みの表示寸）。
    pub composed: ComposedSurface,
    /// `composed.bytes()`（＝k 寸バイト）から挿入時に 1 回だけ生成した当たり判定マスク・
    /// さわり判定の真実源。`AlphaMask` の物理 px 契約と無修正で整合する。
    pub mask: AlphaMask,
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

/// 合成入力＋表示スケール → [`CacheEntry`] の容量 1 メモ化スロット（直前の表示結果のみ保持）。
///
/// UI スレッド専有の純粋な状態容器で、内部ロックを持たない。直前のキー（合成入力＋k）と結果の
/// 保持・完全一致引き当て、およびアトラス再構築・ghost 再読込時の破棄（[`invalidate_all`]）だけを
/// 担う。合成器・リサンプラは所有せず、ミス時の合成・k 倍リサンプル・挿入は提示段の責務である
/// （本モジュール冒頭 §責務分界）。k 別の多エントリ保持は行わない——k 変化は稀イベントであり、
/// キー相違によるミス → 再合成＋再サンプルで足りる（設計 D6）。
///
/// [`invalidate_all`]: ComposeCache::invalidate_all
#[derive(Debug, Default)]
pub struct ComposeCache {
    /// 直前のキー（合成入力＋表示スケール）と結果の対。`None` は未合成／無効化済み。
    slot: Option<(ComposeKey, CacheEntry)>,
}

impl ComposeCache {
    /// 空のスロットを構築する。
    pub fn new() -> Self {
        Self { slot: None }
    }

    /// 表示用サーフェスを合成入力（surface id ＋ bind 集合 ＋ pattern 状態）＋表示スケール `scale`
    /// 鍵で挿入し、[`AlphaMask`] を **挿入時に 1 回だけ**生成して束ねる。既存スロットは対ごと置換
    /// する（直前 1 件のみ保持・R2.4）。
    ///
    /// 呼び手は合成済み [`ComposedSurface`] を渡すだけでよく、マスク生成
    /// （[`AlphaMask::from_pbgra32`]）は本メソッド内部で `composed.bytes()`／`width`／`height`／
    /// `stride` から一度だけ行う。マスク生成 API を挿入の内側へ隠すことで、「表示のたびに再生成
    /// しない」（R2.1）を呼び手が破れない構造にする。
    ///
    /// `pattern` は seriko のアニメ pattern 状態（[`PatternState`]）で、`binds` と同格の合成入力
    /// キー要素である（R5.2）。`scale` は表示スケール k（要件 2.4/4.1）で、渡す `composed` は
    /// **その k を適用済みの表示用サーフェス**でなければならない——マスクはそのバイトから生成される
    /// ため、k と `composed` の不一致はそのまま「絵とさわり判定の寸法不一致」になる。本層は合成器も
    /// リサンプラも持たない（k の適用は提示段の責務・本モジュール冒頭 §責務分界）。
    ///
    /// 挿入したエントリへの共有参照を返す（提示段がそのまま表示・マスク同期へ用いる）。
    pub fn insert(
        &mut self,
        surface_id: u32,
        binds: BindSet,
        pattern: PatternState,
        scale: ScaleRatio,
        composed: ComposedSurface,
    ) -> &CacheEntry {
        // マスクは挿入時に 1 回だけ生成し（R2.1）、表示バッファと同一 bytes 由来で束ねる（R2.4）。
        // `composed` は k 適用済みゆえ、このマスクも k 寸（物理 px）で生成される（設計 D6）。
        let mask = AlphaMask::from_pbgra32(
            composed.bytes(),
            composed.width(),
            composed.height(),
            composed.stride(),
        );
        let key = ComposeKey {
            surface_id,
            binds,
            pattern,
            scale,
        };
        self.slot = Some((key, CacheEntry { composed, mask }));
        // 直前に挿入したスロットは必ず存在する。
        &self.slot.as_ref().expect("slot was just inserted").1
    }

    /// 合成入力（surface id ＋ bind 集合 ＋ pattern 状態）と表示スケール `scale` が直前のエントリと
    /// **完全一致**するときのみエントリを返す。
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
        match &self.slot {
            Some((key, entry))
                if key.surface_id == surface_id
                    && key.binds == *binds
                    && key.pattern == *pattern
                    && key.scale == scale =>
            {
                Some(entry)
            }
            _ => None,
        }
    }

    /// スロットを破棄する（アトラス再構築・ghost 再読込時の唯一の無効化口・R4.3）。
    ///
    /// 以後あらゆるキーがミスし、提示段が再合成して再挿入する。**k 変化はここを通さない**——
    /// キー等価で表現できるものを命令で二重化しない（設計 D6）。
    pub fn invalidate_all(&mut self) {
        self.slot = None;
    }
}

#[cfg(test)]
#[path = "cache_tests.rs"]
mod tests;
