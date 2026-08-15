//! `FrameTiming`: 表示 1 コマ適用（`apply_show`）の段階別計時と perf サマリ行の emit。
//!
//! 「何が重いか」を推測でなく実測で確定するための観測基盤である（Requirement 1.1/1.2/1.5）。
//! 表示 1 コマの適用を **キャッシュ照会・合成・リサンプル・当たり判定マスク生成・供給面転写**
//! の 5 段へ刻み、合計と併せて 1 適用 = 1 行の固定スキーマで出す。この行だけが
//! `tools/perf/judge-perf.py`（判定スクリプト）とコードの間のデータ契約である。
//!
//! # 計測は無条件・emit だけがフィルタに従う（Requirement 1.5・design.md D5）
//!
//! [`FrameTiming::start`]／[`FrameTiming::mark`] は [`Instant::now`]（段あたり数十 ns）を
//! **ログ設定を一切参照せずに**呼ぶ。ログ水準の判定に触れるのは [`FrameTiming::emit`] 内の
//! `tracing::debug!` だけであり、その中でも「行を出すか」しか変わらない。ゆえに
//! 「計時ログの有効・無効で表示結果が変わらない」は檻ではなく**構造**で成立する——表示経路に
//! ログ設定による分岐が 1 つも無いためである。`if tracing::enabled!(..)` のような前置ガードを
//! 足すと、この構造的保証が失われる（＝入れてはならない）。
//!
//! # 行のスキーマ（design.md §Data Models「perf サマリ行スキーマ」）
//!
//! 固定文言 [`PERF_LINE_MESSAGE`]・`debug` 水準・target は本モジュールのモジュールパス
//! （`areka_emo_present::presenter::timing`）＝既存の `RUST_LOG=areka_emo_present=debug` 流儀で
//! そのまま拾える。フィールドは
//!
//! - `target_id`（Debug）・`surface_id`（u32）・`cache_hit`（bool）: 適用対象と引き当ての可否
//! - `t_cache_us` `t_compose_us` `t_resample_us` `t_mask_us` `t_upload_us` `t_total_us`（u64・µs）:
//!   **全段が常に出る**。実行されなかった段は 0 である。これにより判定スクリプトは
//!   「段フィールドの欠落」を行単位で検出でき、部分集計を黙って出さずに済む（Requirement 2.5）
//! - `alloc_compose_dst` `alloc_resample_dst` `alloc_xmap` `alloc_mask`（u32）: この適用で起きた
//!   新規確保／容量成長の計数（Requirement 1.3・供給元は `FrameBudget`）
//! - `key_hash`（u64）: 合成キーの安定ハッシュ。run 内の異なりキー数＝必要スロット数の実測根拠
//!   （Requirement 7.2 のキャッシュ容量裁定材料）
//!
//! 既存の表示成立点 info!（`"apply(ShowSurface): 表示・マスクを更新"`）とは**別行**であり、
//! 同一適用の対として隣接して出る（配線は show.rs の責務）。文言・フィールド名の変更は
//! `judge-perf.py` と計時檻の再検証トリガである（design.md §Revalidation Triggers）。

use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

use areka_emo_compose::{BindSet, ComposeMethod, PatternState, ScaleRatio};

use super::budget::BudgetDelta;
use crate::command::TargetId;

/// perf サマリ行の固定文言（grep の唯一の足がかり・判定スクリプトとの契約面）。
///
/// `tracing` のメッセージは文字列リテラルでなければならないため、[`FrameTiming::emit_at`] 内の
/// `debug!` には同じ文言を直接書いてある。両者の乖離は単体檻が RED で捕まえる。
///
/// 読み手は檻だけである（`timing_tests.rs`・task 1.4 の計時ログ較正檻）——製品コードは
/// リテラルを直接持つため参照しない。契約面を名前で 1 箇所に据えることが本定数の目的であり、
/// 死蔵ではない。
#[allow(dead_code)]
pub(super) const PERF_LINE_MESSAGE: &str = "perf(apply_show): 段階別計時";

/// 計時対象の段（Requirement 1.1 の列挙と 1:1）。
///
/// 合計は段ではなく「起点から emit までの全区間」として別に取る（段の総和ではない——段の外側で
/// 費やされた時間があれば差として現れ、内訳の取りこぼしが判る）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Stage {
    /// キャッシュ照会（`ComposeCache::get`）。
    CacheLookup,
    /// 合成（`Composer::compose` / `compose_into`）。
    Compose,
    /// リサンプル（k 適用・恒等 k では実行されない）。
    Resample,
    /// 当たり判定マスク生成（`AlphaMask` 生成・引き当て時は実行されない）。
    MaskGen,
    /// 供給面転写（swap chain へのアップロード）。
    Upload,
}

impl Stage {
    /// 段数（配列長の正本）。
    const COUNT: usize = 5;

    /// 記録配列上の位置。
    const fn index(self) -> usize {
        match self {
            Stage::CacheLookup => 0,
            Stage::Compose => 1,
            Stage::Resample => 2,
            Stage::MaskGen => 3,
            Stage::Upload => 4,
        }
    }
}

/// perf サマリ行のうち、計時でも確保計数でもない同定情報（適用対象・引き当て・合成キー）。
#[derive(Debug, Clone, Copy)]
pub(super) struct EmitContext {
    /// 適用対象（既存の表示成立点 info! と同一の識別子）。
    pub(super) target_id: TargetId,
    /// 適用したサーフェス id（同上）。
    pub(super) surface_id: u32,
    /// この適用が引き当てで済んだか。
    pub(super) cache_hit: bool,
    /// 合成キーの安定ハッシュ（[`compose_key_hash`] の値・Requirement 7.2）。
    pub(super) key_hash: u64,
}

/// 1 適用分の段階計時。適用の冒頭で開始し、各段の境界で区間を確定する。
///
/// - Preconditions: [`start`](FrameTiming::start) は 1 適用につき 1 回。
/// - Postconditions: [`emit`](FrameTiming::emit) は `self` を消費する（同じ計時の二重 emit 不可）。
/// - Invariants: 出力行には全段フィールドが必ず載る（実行されなかった段は 0）。
#[derive(Debug)]
pub(super) struct FrameTiming {
    /// 適用の起点（合計の基準）。
    started: Instant,
    /// 直近に確定した段の境界（次の段の区間の始点）。
    last: Instant,
    /// 段別の所要。`None` ＝ その段は実行されなかった（行には 0 で出る）。
    stages: [Option<Duration>; Stage::COUNT],
}

impl FrameTiming {
    /// 適用の冒頭で計時を開始する（ログ設定を参照しない＝D5）。
    pub(super) fn start() -> Self {
        Self::start_at(Instant::now())
    }

    /// 起点を注入して開始する（檻が実時間に依存しないための私有シーム・Requirement 6.2）。
    fn start_at(now: Instant) -> Self {
        Self {
            started: now,
            last: now,
            stages: [None; Stage::COUNT],
        }
    }

    /// 段の完了を記録する（直前の境界からの区間を `stage` へ帰属させ、境界を進める）。
    pub(super) fn mark(&mut self, stage: Stage) {
        self.mark_at(stage, Instant::now());
    }

    /// 境界時刻を注入して段を確定する（私有シーム）。
    ///
    /// 同一段の二重記録は配線の誤り（段の意味が壊れ、内訳が読めなくなる）ゆえ `debug_assert` で
    /// 拒否する。release では区間を上書きし、境界は必ず進める——観測が止まるより、最後の区間で
    /// 続行するほうが被害が小さい（表示は計時に依存しない）。
    fn mark_at(&mut self, stage: Stage, now: Instant) {
        let index = stage.index();
        debug_assert!(
            self.stages[index].is_none(),
            "同一段の二重記録: {stage:?}（1 適用につき各段 1 回）"
        );
        // 単調時計でも境界の逆転は理屈上あり得る（注入時）。飽和で 0 に落として計時を壊さない。
        self.stages[index] = Some(now.saturating_duration_since(self.last));
        self.last = now;
    }

    /// 表示成立点で perf サマリ行を 1 本出す（`self` を消費）。
    ///
    /// 確保計数（[`BudgetDelta`]）は行へ載せるだけで、値は作らない——確保の発生点を知るのは
    /// 計数シームの所有者 `FrameBudget` 側であり、`FrameTiming` はその 1 適用分の増分を
    /// 受け取って同居させる運搬役である（Requirement 1.3）。
    pub(super) fn emit(self, ctx: &EmitContext, allocs: BudgetDelta) {
        self.emit_at(ctx, allocs, Instant::now());
    }

    /// 終端時刻を注入して emit する（私有シーム）。
    ///
    /// 全段フィールドを**常に**載せる: 記録の無い段は 0 である（Requirement 2.5 の行単位欠落
    /// 検出は「段が常に出る」ことに依存する。条件付きでフィールドを落とすと、実行されなかった
    /// 段と欠陥で消えた段が判定スクリプトから区別できなくなる）。
    fn emit_at(self, ctx: &EmitContext, allocs: BudgetDelta, now: Instant) {
        let stage_us = |stage: Stage| -> u64 { duration_us(self.stages[stage.index()]) };
        let t_cache_us = stage_us(Stage::CacheLookup);
        let t_compose_us = stage_us(Stage::Compose);
        let t_resample_us = stage_us(Stage::Resample);
        let t_mask_us = stage_us(Stage::MaskGen);
        let t_upload_us = stage_us(Stage::Upload);
        let t_total_us = as_micros_u64(now.saturating_duration_since(self.started));

        let EmitContext {
            target_id,
            surface_id,
            cache_hit,
            key_hash,
        } = *ctx;
        let BudgetDelta {
            alloc_compose_dst,
            alloc_resample_dst,
            alloc_xmap,
            alloc_mask,
        } = allocs;

        // 文言は PERF_LINE_MESSAGE の写し（リテラル必須）。水準・target（モジュールパス既定）と
        // ともに judge-perf.py との契約面であり、変更は Revalidation Trigger である。
        tracing::debug!(
            ?target_id,
            surface_id,
            cache_hit,
            t_cache_us,
            t_compose_us,
            t_resample_us,
            t_mask_us,
            t_upload_us,
            t_total_us,
            alloc_compose_dst,
            alloc_resample_dst,
            alloc_xmap,
            alloc_mask,
            key_hash,
            "perf(apply_show): 段階別計時"
        );
    }
}

/// 未記録の段は 0（フィールドを落とさない）。
fn duration_us(recorded: Option<Duration>) -> u64 {
    recorded.map_or(0, as_micros_u64)
}

/// µs へ落とす（u128 → u64 の飽和。1 適用が 58 万年を超えることは無いが、値を捏造しない）。
fn as_micros_u64(d: Duration) -> u64 {
    u64::try_from(d.as_micros()).unwrap_or(u64::MAX)
}

// ── 合成キーの安定ハッシュ（Requirement 7.2） ─────────────────────────────────────────

/// FNV-1a 64bit のオフセット基底。
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
/// FNV-1a 64bit の素数。
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// 非ランダム化・実装非依存の 64bit ハッシャ（FNV-1a）。
///
/// `std::collections::hash_map::DefaultHasher` を使わないのは、⑴`RandomState` 経由の既定構築が
/// プロセス毎にシードされ run 間・run 内の比較可能性を落とす経路を持つこと ⑵`DefaultHasher` の
/// アルゴリズム自体が「将来変わり得る」と規定されており、判定スクリプトが数える異なりキー数の
/// 意味がツールチェーン更新で静かに変質しうること、の 2 点による。FNV-1a は仕様が固定で
/// シードを持たないため、**同一 run 内で安定**（Requirement 7.2 の要求）であることに加えて
/// run をまたいでも同一値になる。暗号用途ではない（衝突耐性は集計用途に十分）。
#[derive(Debug, Clone, Copy)]
struct Fnv1a(u64);

impl Fnv1a {
    fn new() -> Self {
        Self(FNV_OFFSET_BASIS)
    }
}

impl Hasher for Fnv1a {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(FNV_PRIME);
        }
    }
}

/// 合成キー（surface id ＋ bind 集合 ＋ pattern 状態 ＋ 表示スケール k）の安定ハッシュ。
///
/// キー要素は `ComposeCache` の `ComposeKey` と同一である——run 中に現れた**異なりキー数**が
/// 「いくつのスロットが要るのか」の実測根拠になるため、要素を 1 つでも取りこぼすと異なり数が
/// 過少になり、裁定材料（Requirement 7.2）として成立しない。この材料をもとに 2026-08-15 の
/// 開発者裁定で容量が 1 → 3 へ改訂された（要件 7.1）。以後も同じ役割を担う——容量を再検討する
/// ときの入力は毎回ここから採る。
///
/// `BindSet` と `PatternState` は `Hash` を実装しないため、公開アクセサ（昇順正準の `ids()`・
/// animation id 昇順の `iter()`）から値等価と同じ順序で流し込む。`ComposeMethod` は
/// `#[non_exhaustive]` で網羅 match が書けないため判別子で混ぜる（同一 run 内で安定）。
/// 走査は借用のみで、確保は行わない（毎フレーム経路で呼ばれる）。
pub(super) fn compose_key_hash(
    surface_id: u32,
    binds: &BindSet,
    pattern: &PatternState,
    scale: ScaleRatio,
) -> u64 {
    let mut hasher = Fnv1a::new();
    surface_id.hash(&mut hasher);
    // スライスの `Hash` は長さを前置するため、bind 集合と後続要素の境界は曖昧にならない。
    binds.ids().hash(&mut hasher);
    // pattern も件数を前置し、コマ列と後続の `scale` が食い違う形の混線を作らない。
    let frame_count = pattern.iter().count() as u64;
    frame_count.hash(&mut hasher);
    for (animation_id, frame) in pattern.iter() {
        animation_id.hash(&mut hasher);
        frame.surface_id.hash(&mut hasher);
        std::mem::discriminant(&frame.method).hash(&mut hasher);
        // `Blend` は**内側の `BlendMode` が意味を分ける**ため判別子だけでは足りない
        // （`Blend(Multiply)` と `Blend(Screen)` が同一値になる系統的衝突＝異なりキー数を
        // 過少に見積もる向き）。`BlendKind` も `#[non_exhaustive]` ゆえ判別子で混ぜ、
        // 直交軸の `fast` を併せて流す。
        if let ComposeMethod::Blend(mode) = &frame.method {
            std::mem::discriminant(&mode.kind).hash(&mut hasher);
            mode.fast.hash(&mut hasher);
        }
        frame.x.hash(&mut hasher);
        frame.y.hash(&mut hasher);
    }
    scale.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
#[path = "timing_tests.rs"]
mod tests;
