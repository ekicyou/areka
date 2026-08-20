//! 遷移観測の**サーフェス記録**（`kind=surface`）——語彙・レコード純関数・刻印の取り出し口。
//!
//! design.md の C3（`transition_record`＋`show.rs`／`refresh.rs`／`timing.rs`）が正本である。
//! 表示 1 コマの適用で供給面の寸が変わった回と、その寸で可視化した回、および再表示を見送った回を
//! wintf の観測チャネル（[`transition_diag::TRANSITION_TARGET`]）へ 1 行ずつ出し、窓書込・モニタ表
//! 更新・メッセージ受理と**同一のフレーム番号系列**へ並べる（Requirement 2.2）。
//!
//! # 行の形
//!
//! ```text
//! [transition] frame=<u32> t_us=<u64> kind=surface stage=<s> target_id=<u32> w=<w> h=<h> resized=<b> reason=<r>
//! ```
//!
//! 接頭語（`frame`／`t_us`／`kind`）は [`transition_diag::record_prefix`] が組む。段階ごとに意味を
//! 持たないフィールドは**落とさず**番兵 [`transition_diag::MISSING`]（`-`）で埋める——落とすと
//! 「記録が出ていない」と「その段階にはその値が無い」の区別が事後に付かない（C1 と同じ規律）。
//!
//! # `target_id` は数値で出す（`Debug` 表現ではない）
//!
//! [`TargetId`] の `Debug` は `TargetId(7)` だが、行には**内側の u32 だけ**を載せる。判定側
//! （areka の `transition_judge`）は `target_id` から窓の scope と種別を規則
//! 「shell = 2·scope ／ balloon = 2·scope+1」で写像するため、値が算術で読めることが要件である。
//! scope・窓種別そのものを本レコードが運ばないのは、それが areka の語彙であり emo-present は
//! 知らないからである（design C3・依存方向 wintf ← areka-emo-present ← areka）。
//!
//! # 既定 OFF・前置ガード
//!
//! 発行点（`show.rs`／`refresh.rs`）は [`transition_diag::is_enabled`] が真のときだけレコードを
//! **組む**。`recompose-budget` が成立させた定常状態のアロケーション 0（Requirement 10.4）は
//! 「既定運転で新たな確保が 1 バイトも起きない」ことを含むため、ガードが行の組立より外側に
//! 無ければならない。発行が `debug!` である以上、濾過テストではこの退行を検出できない——
//! 固定するのは `transition_record_tests.rs` の**本文走査**である。

use wintf::ecs::FrameCount;
use wintf::ecs::window::transition_diag::{self, FIELD_STAGE, MISSING, Stamp, TickStart};

use super::World;
use crate::command::TargetId;

// ---------------------------------------------------------------------------
// レコード種別（kind 語）
// ---------------------------------------------------------------------------

/// サーフェス更新のレコード種別。
///
/// wintf の [`transition_diag::KIND_ALL`] とは交わらない（上位 crate が足す語であり、wintf は
/// 上位の語彙を持たない）。交わっていないことは語彙テストが固定している。
pub const KIND_SURFACE: &str = "surface";

// ---------------------------------------------------------------------------
// 段階語（stage）
// ---------------------------------------------------------------------------

/// 供給面へのアップロードが成立した（バッファ寸の変更を含み得る）。
pub const SURFACE_STAGE_UPLOAD: &str = "upload";
/// 新しい寸で可視化した（`set_bounds`・可視性付与の後）。
pub const SURFACE_STAGE_VISUALIZE: &str = "visualize";
/// 再表示を見送った（拡大率不変・不可視）。
pub const SURFACE_STAGE_SKIPPED: &str = "skipped";

/// emo-present が発行する `stage` 語の全体（判定側の語彙照合・語の一意性テストが参照する）。
pub const SURFACE_STAGE_ALL: &[&str] = &[
    SURFACE_STAGE_UPLOAD,
    SURFACE_STAGE_VISUALIZE,
    SURFACE_STAGE_SKIPPED,
];

// ---------------------------------------------------------------------------
// 見送り理由（reason）
// ---------------------------------------------------------------------------

/// 再導出した拡大率が前回適用値と等しい。
pub const SURFACE_REASON_K_UNCHANGED: &str = "k-unchanged";
/// 対象が不可視である（Hide／全透明退化を再表示で蘇らせない）。
pub const SURFACE_REASON_INVISIBLE: &str = "invisible";

/// emo-present が発行する `reason` 語の全体。
///
/// 発行側は 2 語とも記録するが、**判定側が Requirement 4.6（再導出結果が得られない窓は現状維持）の
/// 対象として合否から除外するのは [`SURFACE_REASON_INVISIBLE`] だけ**である。
/// [`SURFACE_REASON_K_UNCHANGED`] は記録されるが除外しない——あれは拡大率が変わらない限り定常
/// フレームで全窓が出し続ける空振りであり、判定側の遷移区間は次の起点まで伸びるので、除外に使うと
/// 毎遷移で全窓が除外側へ入って窓ごとの判定量が静かに消える（task 4.2 の実機採取で判明。裁定は
/// 仕様 `areka-P0-dpi-transition-atomicity` の requirements.md 4.6 直下の注記〔2026-08-20〕。
/// 除外の選り分けの実装は `areka` の `placement::transition_judge::summarize`）。
///
/// 語が増えたら除外の意味が変わるため、一覧を単一の定義元に置く。
pub const SURFACE_REASON_ALL: &[&str] = &[SURFACE_REASON_K_UNCHANGED, SURFACE_REASON_INVISIBLE];

// ---------------------------------------------------------------------------
// フィールド名
// ---------------------------------------------------------------------------

/// 適用対象（[`TargetId`] の内側の u32）。
pub const SURFACE_FIELD_TARGET_ID: &str = "target_id";
/// 供給面の物理幅（見送りでは番兵）。
pub const SURFACE_FIELD_W: &str = "w";
/// 供給面の物理高さ（見送りでは番兵）。
pub const SURFACE_FIELD_H: &str = "h";
/// この適用で供給面のバッファ寸が変わったか（`stage=upload` のみ・他は番兵）。
pub const SURFACE_FIELD_RESIZED: &str = "resized";
/// 見送りの理由（`stage=skipped` のみ・他は番兵）。
pub const SURFACE_FIELD_REASON: &str = "reason";

/// `kind=surface` 行の必須フィールド（接頭語を除く）。
pub const SURFACE_FIELDS: &[&str] = &[
    FIELD_STAGE,
    SURFACE_FIELD_TARGET_ID,
    SURFACE_FIELD_W,
    SURFACE_FIELD_H,
    SURFACE_FIELD_RESIZED,
    SURFACE_FIELD_REASON,
];

// ---------------------------------------------------------------------------
// レコード
// ---------------------------------------------------------------------------

/// サーフェス更新の段階。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SurfaceStage {
    /// アップロード成立。
    Upload,
    /// 新しい寸での可視化。
    Visualize,
    /// 再表示の見送り。
    Skipped,
}

impl SurfaceStage {
    /// 行に載る段階語。
    pub(super) fn as_str(self) -> &'static str {
        match self {
            SurfaceStage::Upload => SURFACE_STAGE_UPLOAD,
            SurfaceStage::Visualize => SURFACE_STAGE_VISUALIZE,
            SurfaceStage::Skipped => SURFACE_STAGE_SKIPPED,
        }
    }
}

/// サーフェス更新 1 件。
#[derive(Clone, Copy, Debug)]
pub(super) struct SurfaceRecord {
    /// 刻印（[`stamp_of`] が World 資源から組む）。
    pub(super) stamp: Stamp,
    /// 段階。
    pub(super) stage: SurfaceStage,
    /// 適用対象。
    pub(super) target_id: TargetId,
    /// 供給面の物理寸。見送りでは寸そのものが無いので `None`（2 フィールドとも番兵）。
    pub(super) size: Option<(u32, u32)>,
    /// バッファ寸が変わったか。`stage=upload` 以外では意味を持たないので `None`（番兵）。
    ///
    /// `false` へ潰さないのは、「寸は変わらなかった」と「この段階では測っていない」を同じ
    /// 字面にしないためである（[`transition_diag::FlushRecord::total_us`] と同じ規律）。
    pub(super) resized: Option<bool>,
    /// 見送りの理由（[`SURFACE_REASON_ALL`] のいずれか）。見送り以外は `None`（番兵）。
    pub(super) reason: Option<&'static str>,
}

// ---------------------------------------------------------------------------
// レコード純関数（時刻を読まない・ログに触れない）
// ---------------------------------------------------------------------------

/// 省略可能な値を表現へ（不在は番兵）。
fn opt_field<T: std::fmt::Display>(value: Option<T>) -> String {
    match value {
        Some(v) => v.to_string(),
        None => MISSING.to_string(),
    }
}

/// サーフェス更新 1 件の記録行。
pub(super) fn surface_line(record: &SurfaceRecord) -> String {
    let (w, h) = match record.size {
        Some((w, h)) => (w.to_string(), h.to_string()),
        None => (MISSING.to_string(), MISSING.to_string()),
    };
    format!(
        "{prefix} {FIELD_STAGE}={stage} {SURFACE_FIELD_TARGET_ID}={target_id} \
         {SURFACE_FIELD_W}={w} {SURFACE_FIELD_H}={h} {SURFACE_FIELD_RESIZED}={resized} \
         {SURFACE_FIELD_REASON}={reason}",
        prefix = transition_diag::record_prefix(record.stamp, KIND_SURFACE),
        stage = record.stage.as_str(),
        // `Debug`（`TargetId(7)`）ではなく内側の u32。判定側が scope を算術で写像する（module doc）。
        target_id = record.target_id.0,
        resized = opt_field(record.resized),
        reason = opt_field(record.reason),
    )
}

// ---------------------------------------------------------------------------
// 刻印の取り出し（D1＝World 資源が正）
// ---------------------------------------------------------------------------

/// World 資源（`FrameCount`＋`TickStart`）から刻印を組む。
///
/// # スレッド局所ミラーを読まない（D1）
///
/// `apply_show`／`refresh_scale` はいずれも `&mut World` を受け取る**World を借りられる観測点**
/// である。[`transition_diag::stamp`]（スレッド局所ミラー）を読んでよいのは World を借りられない
/// 点（一括 flush・wndproc）だけであり、ライブラリである本 crate が「呼び手は UI スレッドに
/// 居る」という**呼び手側の性質**へ寄りかかるのは退行である。資源から組めば、どのスレッドに
/// 載っても同一 tick の全レコードが同じ `frame` を持つ。
///
/// # 資源が無い World（tick 前・テスト World）
///
/// `EcsWorld::new` 以外で組まれた World には両資源が無い。その場合は `frame=0`・`t_us=0` を返す
/// ——[`transition_diag::current_frame`] が tick 前に `0` を返すのと同じ意味であり、値を捏造
/// しない。判定は `wrapping_sub` の差分だけで行う（D14）ため、この 0 は絶対値比較に晒されない。
pub(super) fn stamp_of(world: &World) -> Stamp {
    match (
        world.get_resource::<FrameCount>(),
        world.get_resource::<TickStart>(),
    ) {
        (Some(frame), Some(tick_start)) => transition_diag::stamp_from_world(frame, tick_start),
        _ => Stamp { frame: 0, t_us: 0 },
    }
}

/// 段階別計時（perf サマリ行）へ載せるフレーム番号（Requirement 2.8・D12）。
///
/// 刻印と**同じ資源**から読む——perf 行と `surface` 行が同一フレーム番号で突合できることが
/// 本フィールドの唯一の存在理由であり、供給元が 2 つになると突合が静かに壊れる。
pub(super) fn frame_of(world: &World) -> u32 {
    world
        .get_resource::<FrameCount>()
        .map_or(0, |frame| frame.0)
}

#[cfg(test)]
#[path = "transition_record_tests.rs"]
mod transition_record_tests;
