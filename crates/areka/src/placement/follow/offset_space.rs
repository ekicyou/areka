//! バルーン位置オフセットの**単位空間契約の定義元**と、その契約の上で働く変換規則
//! （areka-P0-balloon-offset-dpi・design C1／D1〜D5・要件 1.1〜1.5／2.1〜2.5／3.1／3.3／
//! 4.2／4.4／5.3／9.3）。
//!
//! # 単位空間契約（唯一の権威・要件 1.1／1.3）
//!
//! **実行時の合流欄と追従オフセットが保持する値は「現在の表示 DPI における物理 px」である。**
//!
//! 対象は `ScopeConfig.balloon_offset`／`ScopePlacement.balloon_offset`／
//! 追従 Component が持つオフセットの 3 つで、すべて同じ空間の値として合流する。
//! 作者空間（作者基準 DPI で意味を持つ生値）を持つ供給元は、**合流の前に**本モジュールの
//! [`scale_author_offset`] を通って物理 px へ換算される——換算前の値と換算後の値が同じ欄へ
//! 混ざることは無い（要件 1.2）。「単位空間の混在は意図的な暫定」という従来の記述は本契約が
//! 置き換える。
//!
//! # 供給元ごとの換算軸の割り当て（要件 1.4）
//!
//! 作者基準 DPI はシェル（`seriko.dpi`）とバルーン（`dpi`）で別々に宣言され得るため、
//! 拡大率は 1 つではなく 2 つある。どちらで換算するかは供給元ごとに一意に定める。
//!
//! | 供給元 | 換算軸 | 出所 |
//! |---|---|---|
//! | `descript` の `balloon.offsetx`／`offsety` | **シェル軸**（`MeasureScaling::shell`） | 語彙がゴースト／シェルの `descript.txt`＝シェル作者の空間（design D2・本仕様の決定） |
//! | `windowposition` 由来の調整量 | **バルーン軸** | バルーン作者の空間（既存確定・本仕様は温存して記録するだけ） |
//! | 移動台本 `\![move]` の `dx`／`dy` | **シェル軸** | 既存確定（互換記録へ登記済み）・本仕様は温存して記録するだけ |
//!
//! **遷移の追随には軸の選択が生じない**（design D5・要件 4.4）。
//! `k_axis(d) = app_scale × (d ÷ author_dpi_axis)` ゆえ
//! `k_axis(d₁) ÷ k_axis(d₀) = d₁ ÷ d₀` となり、作者基準 DPI が約分で消えるからである。
//! [`rescale_follow_offset`] は表示 DPI の整数 2 つから比を直に組む。
//!
//! # 永続値は契約の明示の例外（要件 5.1／5.3）
//!
//! 永続化されるバルーンオフセットは物理 px の生値のみで、**保存時の表示 DPI を記録しない**。
//! 保存値は換算せずそのまま採用する（拡大率をまたぐ保存位置の追従はしない・開発者裁定の踏襲）。
//! この「どの表示 DPI に属するか分からない」状態は [`OffsetBase::dpi`] の `None`＝**未係留**
//! として表現する。未係留の基準は最初の観測で**値を変えずに**その時の表示 DPI へ係留され、
//! 以後は通常の追随規則が効く（要件 5.4）。
//!
//! # 丸めと記録の規律
//!
//! 本モジュールは**新しい丸め規約を 1 つも導入しない**（要件 9.3）——大きさの丸めは
//! `ScaleRatio::scale_len`（round half away from zero・非ゼロ長は最小 1px・恒等は素通し）へ、
//! 符号の保存は [`scale_signed`] へ委譲する。
//!
//! また `World`・`Entity`・ログ機構のいずれにも触れない。縮退・飽和は**判定結果の値**として
//! 返し、警告の発行は呼び手の責務である（要件 1.5／2.5／3.6／9.4 の記録は呼び手が出す）。

// scaffold: 消費者の結線は段階的に入る。確立の 2 本（[`BalloonFollow::new`]・
// [`BalloonFollow::reestablish`]）と読取の [`BalloonFollow::offset`]、および供給時の換算
// （[`scale_author_offset`]・[`ScaledAxis`]・`scale_axis`＝task 4.1 で
// `placement::apply_author_balloon_offset_scale` が結線）は結線済みだが、
// 追随相専用の 2 本（[`BalloonFollow::anchor_base_dpi`]・[`BalloonFollow::apply_rescaled`]）
// と基準対の読取 [`BalloonFollow::base`]、および遷移の変換規則（[`rescale_follow_offset`]と
// その判定型）は、追随相（task 6.x）が結線するまで非テストビルドで未使用に見える
// ——areka は lib target を持たない bin crate ゆえ `pub` でも dead_code 免除されない。
// 実測（本 allow を外した `cargo build -p areka --bins`・task 4.1 時点）で残る未使用は
// **6 項目**（`base`・`anchor_base_dpi`・`apply_rescaled`・`UnresolvedScale`・
// `OffsetRescale`・`rescale_follow_offset`）であり、項目ごとの許可を 6 枚貼るより
// 1 枚に集約するほうが、以後の**真の** dead code を隠さない。
// （task 3.1 時点の本注記は「7 項目」と書いていたが、当時の実測値も 6 項目で誤りだった
// ——数は結線が進むたびに動くので、変えたときは必ず measure し直すこと。
// task 4.1 では 9 → 6 へ減った。task 6.x の結線後は本 allow ごと撤去できる見込み。）
#![allow(dead_code)]

use areka_emo_compose::ScaleRatio;
use bevy_ecs::prelude::{Component, Entity};
use wintf::ecs::DPI;

use crate::placement::resolver::PointPx;
use crate::placement::scale_signed;

/// キャラ窓に付与するバルーン追従 Component（4.2/4.4/4.8）。
///
/// 現在のオフセットと、その値が導かれた**基準対**（[`OffsetBase`]）を 1 つに持つ。
/// どちらも**私有**であり、読取は [`BalloonFollow::offset`]／[`BalloonFollow::base`] に、
/// 書込は次の 4 本だけに閉じる（areka-P0-balloon-offset-dpi・design D14・要件 3.3／3.5）。
///
/// | 種別 | メソッド | 基準対の扱い |
/// |---|---|---|
/// | 確立 | [`BalloonFollow::new`]（配置解決の既定／保存値の復元） | 与えられた基準対を置く |
/// | 確立 | [`BalloonFollow::reestablish`]（ドラッグ結果／キーワード再導出） | 新しい値で焼き直す |
/// | 追随 | [`BalloonFollow::anchor_base_dpi`]（未係留の基準を係留する） | 値は不変・表示 DPI だけ刻む |
/// | 追随 | [`BalloonFollow::apply_rescaled`]（基準から引き直した値を反映） | **変えない** |
///
/// 欄を私有にする目的は、「基準対の書き手を 1 つでも取りこぼすと基準が古いまま残り、
/// 次の遷移で静かにずれる」危険を**型で潰す**ことである——定義モジュールの外から欄へ
/// 直接代入すると、実行時にずれる代わりにコンパイルエラーになる（design D14）。
///
/// `offset` の初期値は配置時に確定する暫定 offset（物理 px・
/// `ScopePlacement.balloon_offset` の転写＝P5 幾何の暫定規則。正式な配置規則は
/// balloon 表示系の後続が所有する・4.4）。バルーン単独ドラッグでユーザーが
/// ずらすと [`super::on_balloon_drag`] が `balloon_pos − char_pos` へ**記憶更新**し、
/// 以後のキャラ窓ドラッグ・[`super::move_window_to`] は調整後 offset で追従する
/// （4.8・セッション内のみ・永続化は M-life の領分。
/// 旧挙動「次のキャラ窓ドラッグで初期 offset へスナップバック」は
/// 2026-07-11 要件 4.8 により仕様退役）。
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BalloonFollow {
    /// 追従して動かすバルーン窓 entity。
    pub balloon: Entity,
    /// キャラ窓左上からバルーン窓左上への相対 offset（物理 px・配置時確定）。
    /// **私有**——上表の 4 本だけが書ける。
    offset: PointPx,
    /// 基準対（この offset が導かれた元の値と、その値が属する表示 DPI）。**私有**。
    base: OffsetBase,
}

impl BalloonFollow {
    /// **確立点**——基準対を与えて組む（配置解決の既定／保存値の復元）。
    ///
    /// 現在値は基準値そのものから始まる（まだ一度も追随していない状態）。
    pub fn new(balloon: Entity, base: OffsetBase) -> Self {
        Self {
            balloon,
            offset: base.offset,
            base,
        }
    }

    /// 現在の相対位置（キャラ窓左上相対・物理 px・読取専用・要件 9.1）。
    pub fn offset(&self) -> PointPx {
        self.offset
    }

    /// 基準対（読取専用・追随ステップと決定論テストが読む）。
    pub fn base(&self) -> OffsetBase {
        self.base
    }

    /// **確立点**——新しい相対位置を基準として焼き直す（ドラッグ結果・キーワード再導出）。
    ///
    /// 現在値と基準値の双方が `offset` になり、基準 DPI はその時点の表示 DPI へ
    /// **係留済み**（`Some(dpi)`）になる。利用者のドラッグ由来にも作者指定と同一の
    /// 追随規則が効くのは、この 1 本が基準を確立するからである（要件 3.5）。
    pub fn reestablish(&mut self, offset: PointPx, dpi: DPI) {
        self.offset = offset;
        self.base = OffsetBase {
            offset,
            dpi: Some(dpi),
        };
    }

    /// **係留**——未係留の基準へ現在の表示 DPI を刻む（要件 5.2）。
    ///
    /// 基準値も現在値も **1 bit も変えない**。永続値の腕が「どの表示 DPI に属するか
    /// 分からない」状態から通常の追随規則へ入る唯一の口である（要件 5.4）。
    pub fn anchor_base_dpi(&mut self, dpi: DPI) {
        self.base.dpi = Some(dpi);
    }

    /// **追随**——基準から引き直した値を反映する（要件 3.1）。
    ///
    /// **基準対は変えない**。出力を入力へ戻さないので誤差が連鎖せず、拡大率が往復して
    /// 元へ戻れば同じ値へ戻る（[`rescale_follow_offset`] の純関数性がここで守られる）。
    pub fn apply_rescaled(&mut self, offset: PointPx) {
        self.offset = offset;
    }
}

/// 追従オフセットの基準対——値と、その値が属する表示 DPI。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OffsetBase {
    /// 基準値（キャラ窓左上相対・物理 px）。
    pub offset: PointPx,
    /// 基準値が属する表示 DPI。`None` は**未係留**＝
    /// 「最初に観測した表示 DPI の空間に属する」と読む（永続値の腕・要件 5.2）。
    pub dpi: Option<DPI>,
}

impl OffsetBase {
    /// **未係留**の基準対を組む（表示 DPI が記録されていない値・要件 5.2）。
    ///
    /// 永続値の腕がこの形を採るほか、追従の基準を対象にしない檻が合成の
    /// `ScopePlacement` を組むときの既定でもある——「どの表示 DPI に属するか分からない」
    /// を正直に表す 1 ビットであり、最初の観測で値を変えずに係留される。
    pub fn unpinned(offset: PointPx) -> Self {
        Self { offset, dpi: None }
    }
}

/// 換算の 1 軸ぶんの結果（飽和したかを呼び手へ伝える・要件 2.5）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScaledAxis {
    /// 換算後の値（物理 px・飽和時は `±i32::MAX`）。
    pub value: i32,
    /// `i32` 域を超えて飽和したか（回り込みは起こさない）。
    pub saturated: bool,
}

/// 拡大率を解決できなかった理由（要件 9.4: 縮退は必ず語を持つ）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnresolvedScale {
    /// 基準 DPI が 0（構築子を通れば起きないが、発明せず縮退する）。
    ZeroBaseDpi,
    /// 現在 DPI が 0。
    ZeroCurrentDpi,
}

/// 追随の判定結果。呼び手はこの 4 腕を網羅して書込と記録を決める。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetRescale {
    /// 未係留の基準を現在の DPI へ係留した——**値は変えない**（要件 5.2）。
    Anchored {
        /// 係留した表示 DPI（＝観測した現在値）。
        base_dpi: DPI,
    },
    /// 基準 DPI と現在 DPI が同一——値も基準も変えない（要件 3.3 の bit 同一）。
    Unchanged,
    /// 追随した。
    Rescaled {
        /// 追随後のオフセット（物理 px）。
        offset: PointPx,
        /// いずれかの軸が飽和したか（要件 2.5 と同型）。
        saturated: bool,
    },
    /// 拡大率を解決できない——値も基準も変えない（要件 3.6）。
    Unresolved {
        /// 解決できなかった理由。
        reason: UnresolvedScale,
    },
}

/// 遷移時の唯一の変換規則（純関数・要件 3.1／3.3／4.2／4.4）。
///
/// 入力は**基準対と現在の表示 DPI だけ**であり、前回の結果を入力にしない。出力が入力へ
/// 戻らないため誤差が連鎖せず、一度訪れた表示 DPI へ戻れば常に同じ値になる（往復無誤差）。
///
/// 比は表示 DPI の整数 2 つから直接組む——`k(d) = app_scale × (d ÷ author_dpi)` ゆえ
/// `k(d₁) ÷ k(d₀) = d₁ ÷ d₀` で作者基準 DPI が約分で消え、シェル軸／バルーン軸の選択が
/// 生じない（要件 4.4 の「どちらを用いるか」への答え）。
///
/// `base` は変更しない（純関数）。基準が変わるのは確立点と係留だけである。
pub fn rescale_follow_offset(base: OffsetBase, current: DPI) -> OffsetRescale {
    let Some(base_dpi) = base.dpi else {
        // 未係留＝保存値の腕。値を 1 bit も動かさずに現在の表示 DPI へ係留する（要件 5.2）。
        return OffsetRescale::Anchored { base_dpi: current };
    };
    if base_dpi == current {
        // 恒等——値も基準も動かさない（要件 2.2／3.3）。
        return OffsetRescale::Unchanged;
    }
    if base_dpi.dpi_x == 0 || base_dpi.dpi_y == 0 {
        return OffsetRescale::Unresolved {
            reason: UnresolvedScale::ZeroBaseDpi,
        };
    }
    if current.dpi_x == 0 || current.dpi_y == 0 {
        return OffsetRescale::Unresolved {
            reason: UnresolvedScale::ZeroCurrentDpi,
        };
    }
    // 0 は上で弾いたので `ScaleRatio::new` は必ず `Some`——それでも `unwrap` せず、
    // 万一の縮退にも語を与える（記録の無い縮退経路を作らない・要件 9.4）。
    let (Some(kx), Some(ky)) = (
        ScaleRatio::new(current.dpi_x as u32, base_dpi.dpi_x as u32),
        ScaleRatio::new(current.dpi_y as u32, base_dpi.dpi_y as u32),
    ) else {
        return OffsetRescale::Unresolved {
            reason: UnresolvedScale::ZeroCurrentDpi,
        };
    };
    let x = scale_axis(base.offset.x, kx);
    let y = scale_axis(base.offset.y, ky);
    OffsetRescale::Rescaled {
        offset: PointPx {
            x: x.value,
            y: y.value,
        },
        saturated: x.saturated || y.saturated,
    }
}

/// 作者空間のオフセットを合流欄の空間（物理 px）へ換算する（要件 2.1／2.4／2.5）。
///
/// `k` は供給元の作者空間に対応する軸の表示スケール。`balloon.offsetx`／`offsety` は
/// シェル作者の空間ゆえ [`MeasureScaling::shell`](crate::placement::measure::MeasureScaling)
/// を渡す（design D2・上表「供給元ごとの換算軸の割り当て」）。
///
/// 恒等比（`ScaleRatio::ONE`）では生値がそのまま返る（要件 2.2）。
pub fn scale_author_offset(raw: (i32, i32), k: ScaleRatio) -> (ScaledAxis, ScaledAxis) {
    (scale_axis(raw.0, k), scale_axis(raw.1, k))
}

/// 1 軸ぶんの符号付き換算＋飽和の判定。
///
/// 値は [`scale_signed`]（既存の単一権威）が出す。飽和の判定だけを本層が足す——
/// `scale_len` は `u32` 域で答えるので、物理 px 通貨（`i32`）へ収まらなかったことは
/// 同じ権威の出力を `i32` の上限と比べて判る。**新しい丸めは 1 つも導入しない。**
fn scale_axis(v: i32, k: ScaleRatio) -> ScaledAxis {
    ScaledAxis {
        value: scale_signed(v, k),
        saturated: k.scale_len(v.unsigned_abs()) > i32::MAX as u32,
    }
}
