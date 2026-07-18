//! `\![move]`（キャラクタ移動）の**完全語彙型**と**純粋解釈**（design.md「MoveCueSink＋純粋
//! 解釈＋UI 適用」・R5.2/R5.4）。
//!
//! 本ファイルは task 7.1 の範囲＝**純粋な型＋`parse_move_directive` のみ**。UI 末端結線
//! （`MoveCueSink`／`BaseposResolver`／`CanonDefaultBasepos`／`apply_move_directive`）は
//! 後続タスク（7.2〜7.4・9.1）が同ファイルへ additive に足す（scope 境界）。
//!
//! # 正典 positional 形（ukadoc `\![move]`）
//!
//! `\![move,dx,dy,time,base,X基準.Y基準,...]`——`parse_move_directive` が受け取る `tokens` は
//! キャリア cue の `params`（コマンド名 `move` を除いた引数列）で、位置は次のとおり:
//!
//! | idx | 意味 | 型 | 省略時既定 |
//! |----:|------|----|-----------|
//! | 0 | dx（X 座標/差分） | [`AxisSpec`] | `Fix` |
//! | 1 | dy（Y 座標/差分） | [`AxisSpec`] | `Fix` |
//! | 2 | time（移動時間 ms） | `u32` | `0` |
//! | 3 | base（基準） | [`MoveBase`] | `Screen` |
//! | 4 | base-offset（基準位置） | [`RefPoint`] | `left.top` |
//! | 5 | move-offset（合わせる自窓位置） | [`RefPoint`] | `left.top` |
//!
//! 名前付き `--key=value` 形（ukadoc 記述例）は M1 では positional のみ実導出ゆえ
//! **記録付き縮退（`Err(MoveDegradation::NamedForm)`＝良性スキップ・語彙は将来 additive）**。
//!
//! # 互換裁量（doc/COMPAT_ARCHITECTURE.md §8 対応表に登記）
//!
//! - 裸 `base`（ドット無し）≡ `base.base`（正典形式は `X基準.Y基準`・fixture の de-facto・R5.2 明文）。
//! - 基準 `screen`/`primaryscreen`/`me`/`global` は語彙保持のみ・M1 は数値スコープのみ実導出
//!   （[`MoveDirective::m1_degradations`] が `UnsupportedBase` として surface）。
//! - `time>0` は最終位置へ即時反映＋記録（`Ok` のまま `duration_ms` 保持・R5.4）。

// task 7.1/7.2 は純粋な型＋parse＋basepos シーム＋座標算出を載せる段階で、UI 末端の消費点
// （`MoveCueSink`＝7.3・`apply_move_directive`＝7.4・`mod.rs` の channel 配線＝9.1）は後続
// タスクが足す。それまでは非 test ビルド（bin 本体）から未参照ゆえ dead_code が出るが、これは
// 段階実装の想定内であり、後続タスクの結線で解消される（本 allow は wiring 着地時に撤去する）。
#![allow(dead_code)]

use wintf::ecs::{SizeI, WindowPos};

use crate::placement::resolver::PointPx;

/// `\![move]` の完全語彙型（design.md Service Interface・R5.2）。
///
/// M1 実導出は positional＋数値スコープ基準のみ。残り（名前付き形・非スコープ基準・time>0）は
/// **語彙を第一級で保持**したうえで縮退（Err または [`m1_degradations`](Self::m1_degradations)
/// による記録）へ分類する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveDirective {
    /// 移動対象スコープ（cue.actor 由来・`\0`=0／`\1`=1）。
    pub scope: u32,
    /// X 軸指定（省略/"fix"＝現状維持・数値＝物理 px）。
    pub x: AxisSpec,
    /// Y 軸指定（同上）。
    pub y: AxisSpec,
    /// 移動時間 ms（省略=0）。`>0` は M1 では即時へ縮退（記録・語彙保持・R5.4）。
    pub duration_ms: u32,
    /// 基準（数値スコープのみ M1 実導出・他は語彙保持＋縮退）。
    pub base: MoveBase,
    /// 基準側の参照点（省略=left.top）。
    pub base_offset: RefPoint,
    /// 自窓側の参照点（省略=left.top）。
    pub move_offset: RefPoint,
}

impl MoveDirective {
    /// `Ok` で保持したまま M1 で縮退する語彙を列挙する（記録用・語彙自体は保持・R5.4）。
    ///
    /// - `base` が非スコープ（screen/primaryscreen/me/global）→ [`MoveDegradation::UnsupportedBase`]。
    /// - `duration_ms > 0`（時間付き移動）→ [`MoveDegradation::TimedMoveImmediate`]。
    ///
    /// 名前付き `--` 形はそもそも `parse_move_directive` が `Err` を返すため、ここには現れない。
    pub fn m1_degradations(&self) -> Vec<MoveDegradation> {
        let mut out = Vec::new();
        if !self.base.is_m1_derived() {
            out.push(MoveDegradation::UnsupportedBase(self.base.clone()));
        }
        if self.duration_ms > 0 {
            out.push(MoveDegradation::TimedMoveImmediate {
                duration_ms: self.duration_ms,
            });
        }
        out
    }
}

/// 軸指定（X/Y 共通）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisSpec {
    /// 省略または `"fix"`＝現状維持。
    Fix,
    /// 物理 px（差分/座標は基準解決に委ねる・R-6 物理 px 一元）。
    Px(i32),
}

/// 移動基準（design.md `MoveBase`）。M1 実導出は [`Scope`](MoveBase::Scope) のみ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveBase {
    /// 数値スコープの窓（M1 唯一の実導出経路）。
    Scope(u32),
    /// デスクトップ全体（省略時既定・M1 縮退＝語彙保持）。
    Screen,
    /// プライマリスクリーン（M1 縮退）。
    PrimaryScreen,
    /// 自窓相対（M1 縮退）。
    Me,
    /// 全画面仮想デスクトップ（M1 縮退）。
    Global,
}

impl MoveBase {
    /// M1 で実導出される基準か（数値スコープのみ true・R5.2）。
    pub fn is_m1_derived(&self) -> bool {
        matches!(self, MoveBase::Scope(_))
    }
}

/// 参照点（基準位置／自窓位置）。正典形式 `X基準.Y基準`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefPoint {
    pub x: RefX,
    pub y: RefY,
}

impl RefPoint {
    /// 省略時既定（正典 `left.top`）。
    pub const LEFT_TOP: RefPoint = RefPoint {
        x: RefX::Left,
        y: RefY::Top,
    };
    /// 裸 `base` の展開先（`base.base`・R5.2 の等価則）。
    pub const BASE_BASE: RefPoint = RefPoint {
        x: RefX::Base,
        y: RefY::Base,
    };
}

/// X 基準（left/right/base/center）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefX {
    Left,
    Right,
    Base,
    Center,
}

impl RefX {
    fn parse(token: &str) -> Option<RefX> {
        match token.to_ascii_lowercase().as_str() {
            "left" => Some(RefX::Left),
            "right" => Some(RefX::Right),
            "base" => Some(RefX::Base),
            "center" => Some(RefX::Center),
            _ => None,
        }
    }
}

/// Y 基準（top/bottom/base/center）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefY {
    Top,
    Bottom,
    Base,
    Center,
}

impl RefY {
    fn parse(token: &str) -> Option<RefY> {
        match token.to_ascii_lowercase().as_str() {
            "top" => Some(RefY::Top),
            "bottom" => Some(RefY::Bottom),
            "base" => Some(RefY::Base),
            "center" => Some(RefY::Center),
            _ => None,
        }
    }
}

/// `\![move]` の縮退分類（記録付き・非 panic・語彙保持）。
///
/// **`Err` として返る（parse 不能・良性スキップ）**:
/// - [`NamedForm`](MoveDegradation::NamedForm)（`--key=value` 形・M1 は positional のみ）
/// - [`UnparsableAxis`](MoveDegradation::UnparsableAxis)（X/Y が fix でも i32 でもない）
/// - [`UnparsableDuration`](MoveDegradation::UnparsableDuration)（time が非数値）
/// - [`UnknownBase`](MoveDegradation::UnknownBase)（base が数値でも既知語でもない）
/// - [`UnknownRefPoint`](MoveDegradation::UnknownRefPoint)（基準語が未知）
///
/// **`Ok` のまま [`MoveDirective::m1_degradations`] が記録として surface する（語彙保持）**:
/// - [`UnsupportedBase`](MoveDegradation::UnsupportedBase)（非スコープ基準・M1 非実導出）
/// - [`TimedMoveImmediate`](MoveDegradation::TimedMoveImmediate)（time>0＝即時反映＋記録・R5.4）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveDegradation {
    /// `--key=value` 名前付き形の混入（positional のみ M1 実導出・語彙は将来 additive）。
    NamedForm(String),
    /// X/Y 軸トークンが `fix` でも i32 でもない。
    UnparsableAxis { axis: Axis, token: String },
    /// time トークンが非数値（u32 に読めない）。
    UnparsableDuration(String),
    /// base トークンが数値でも既知の基準語でもない。
    UnknownBase(String),
    /// 基準位置トークン（`X.Y`／裸）が未知の基準語を含む。
    UnknownRefPoint(String),
    /// 非スコープ基準（screen/primaryscreen/me/global）＝M1 非実導出（語彙保持・記録）。
    UnsupportedBase(MoveBase),
    /// time>0 の移動＝最終位置へ即時反映＋記録（語彙保持・R5.4）。
    TimedMoveImmediate { duration_ms: u32 },
}

/// [`MoveDegradation::UnparsableAxis`] の軸識別。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
}

/// `\![move]` の positional 引数列を[`MoveDirective`]へ純粋に解釈する（決定論・no I/O・R5.2）。
///
/// `scope` は移動対象（cue.actor 由来・`\1`=1）。`tokens` はキャリア cue の `params`
/// （コマンド名 `move` を除いた引数列）で、位置意味論はモジュール doc の表に従う。
///
/// 名前付き `--key=value` 形の混入は [`MoveDegradation::NamedForm`] で `Err`（記録付きスキップ・
/// 語彙は将来 additive）。非スコープ基準・time>0 は `Ok` のまま保持し、
/// [`MoveDirective::m1_degradations`] が記録として surface する。
pub fn parse_move_directive(
    scope: u32,
    tokens: &[String],
) -> Result<MoveDirective, MoveDegradation> {
    // 名前付き `--` 形の混入は M1 縮退（positional のみ実導出・R5.2 対応表）。負数 `-353` は
    // `--` 前置でないため誤検出しない。
    if let Some(named) = tokens.iter().find(|t| t.starts_with("--")) {
        return Err(MoveDegradation::NamedForm(named.clone()));
    }

    let tok = |i: usize| tokens.get(i).map(String::as_str).unwrap_or("");

    let x = parse_axis(tok(0), Axis::X)?;
    let y = parse_axis(tok(1), Axis::Y)?;
    let duration_ms = parse_duration(tok(2))?;
    let base = parse_base(tok(3))?;
    let base_offset = parse_ref_point(tok(4))?;
    let move_offset = parse_ref_point(tok(5))?;

    Ok(MoveDirective {
        scope,
        x,
        y,
        duration_ms,
        base,
        base_offset,
        move_offset,
    })
}

/// 軸: 省略/"fix"＝`Fix`・i32＝`Px`・他＝`Err(UnparsableAxis)`。
fn parse_axis(token: &str, axis: Axis) -> Result<AxisSpec, MoveDegradation> {
    if token.is_empty() || token.eq_ignore_ascii_case("fix") {
        Ok(AxisSpec::Fix)
    } else {
        token
            .parse::<i32>()
            .map(AxisSpec::Px)
            .map_err(|_| MoveDegradation::UnparsableAxis {
                axis,
                token: token.to_string(),
            })
    }
}

/// time: 省略=0・u32・他＝`Err(UnparsableDuration)`。
fn parse_duration(token: &str) -> Result<u32, MoveDegradation> {
    if token.is_empty() {
        Ok(0)
    } else {
        token
            .parse::<u32>()
            .map_err(|_| MoveDegradation::UnparsableDuration(token.to_string()))
    }
}

/// base: 省略=`Screen`（正典既定）・数値=`Scope`・既知語=各 variant・他＝`Err(UnknownBase)`。
fn parse_base(token: &str) -> Result<MoveBase, MoveDegradation> {
    if token.is_empty() {
        return Ok(MoveBase::Screen);
    }
    match token.to_ascii_lowercase().as_str() {
        "screen" => Ok(MoveBase::Screen),
        "primaryscreen" => Ok(MoveBase::PrimaryScreen),
        "me" => Ok(MoveBase::Me),
        "global" => Ok(MoveBase::Global),
        _ => token
            .parse::<u32>()
            .map(MoveBase::Scope)
            .map_err(|_| MoveDegradation::UnknownBase(token.to_string())),
    }
}

/// 参照点: 省略=`left.top`・`X.Y`＝各基準・裸トークン＝`token.token`（裸 base≡base.base・R5.2）。
fn parse_ref_point(token: &str) -> Result<RefPoint, MoveDegradation> {
    if token.is_empty() {
        return Ok(RefPoint::LEFT_TOP);
    }
    // ドット無し（裸）は `X基準=Y基準=token` として展開（裸 base≡base.base の一般化）。
    let (xs, ys) = token.split_once('.').unwrap_or((token, token));
    let x = RefX::parse(xs).ok_or_else(|| MoveDegradation::UnknownRefPoint(token.to_string()))?;
    let y = RefY::parse(ys).ok_or_else(|| MoveDegradation::UnknownRefPoint(token.to_string()))?;
    Ok(RefPoint { x, y })
}

// =============================================================================
// basepos 型シーム＋座標算出（task 7.2・R5.2・全て物理 px・R-6 対策）
// =============================================================================

/// basepos（基準位置）解決の**型シーム**（R5.2・design「BaseposResolver」）。
///
/// M1 実導出は [`CanonDefaultBasepos`]（正典既定＝x=サーフェス幅÷2・y=下端）のみ。宣言
/// `point.basepos` の実導出は本 spec の範囲外であり、このトレイトは追跡 spec
/// `areka-P0-surfaces-basepos` が別実装（宣言値を差す resolver）を差し込むための**差替点**
/// として型のみを予約する（doc/COMPAT_ARCHITECTURE.md §8 対応表に登記）。
///
/// 入力は窓寸法（物理 px・`WindowPos.size` 由来）のみ——論理 px 系（BoxStyle）を経由しない
/// ことで 2026-07-05 の二重スケール欠陥（R-6）を型で構造遮断する。
pub trait BaseposResolver {
    /// 窓寸法（物理 px）に対する basepos（窓左上原点相対・物理 px）を返す。
    fn basepos(&self, window_size: SizeI) -> PointPx;
}

/// 正典既定の basepos（x=サーフェス幅÷2・y=下端＝height・R5.2・A-1 裁定）。
///
/// emo2 は `point.basepos` を宣言せず、この正典既定がそのまま適用される正規経路。y の「下端」は
/// 窓左上原点からの相対＝サーフェス高さ（`height`）そのもの。奇数幅は整数除算で切り捨てる。
pub struct CanonDefaultBasepos;

impl BaseposResolver for CanonDefaultBasepos {
    fn basepos(&self, window_size: SizeI) -> PointPx {
        PointPx {
            x: window_size.width / 2,
            y: window_size.height,
        }
    }
}

/// 基準窓・対象窓の [`WindowPos`]（物理 px）と [`MoveDirective`] から、対象窓の**最終位置**を
/// 算出する純粋関数（決定論・no I/O・R5.2）。
///
/// 全て物理 px——窓寸法は `WindowPos.size`（物理）のみを源とし、論理 px 系（BoxStyle）を
/// 経由しない（R-6 対策）。基準位置の解決は `resolver`（M1 は [`CanonDefaultBasepos`]）へ委ね、
/// 宣言 `point.basepos` の追跡 spec がこの引数の差し替えで別 basepos を供給できる。
///
/// # 算出式（design「座標算出」）
///
/// - X: [`AxisSpec::Fix`] なら対象窓の現在 X を**現状維持**。[`AxisSpec::Px`]`(dx)` なら
///   `x' = base_pos.x + basepos(base窓).x + dx − basepos(対象窓).x`。
/// - Y: 同型（`Fix`＝現状維持／`Px(dy)`＝`base_pos.y + basepos(base窓).y + dy − basepos(対象窓).y`）。
///
/// fixture 検算（`\1\![move,-353,,,0,base,base]`・x=Px(-353)・y=Fix）:
/// `x' = pos0.x + w0/2 − 353 − w1/2`・y は現状維持。
///
/// # 縮退（`None`）
///
/// 対象窓の position 欠落、または Px 軸を含むのに基準窓の position／両窓の size が欠落
/// （窓生成前等）で算出できないとき `None`（呼び出し側＝7.4 が warn＋talk 継続・R5.5）。
/// 両軸 [`AxisSpec::Fix`]（現状維持のみ）は basepos 不要ゆえ、基準窓・寸法が欠けても
/// 対象窓の現在位置を返す。極端入力でも panic しない（`saturating_*`・placement と同流儀）。
pub fn resolve_move_target_position(
    resolver: &impl BaseposResolver,
    base: &WindowPos,
    target: &WindowPos,
    directive: &MoveDirective,
) -> Option<PointPx> {
    let target_pos = target.position?;

    // 両軸 Fix（現状維持のみ）は基準窓・寸法を要さない（no-op 移動）。
    if directive.x == AxisSpec::Fix && directive.y == AxisSpec::Fix {
        return Some(PointPx {
            x: target_pos.x,
            y: target_pos.y,
        });
    }

    // Px 軸を含む＝基準窓の位置と両窓の寸法（basepos 算出）が要る。欠落（窓生成前等）は
    // 算出不能ゆえ None（呼び出し側が warn＋継続・R5.5）。
    let base_pos = base.position?;
    let base_bp = resolver.basepos(base.size?);
    let target_bp = resolver.basepos(target.size?);

    let x = match directive.x {
        AxisSpec::Fix => target_pos.x, // 現状維持
        // x' = base_pos.x + basepos(base窓).x + dx − basepos(対象窓).x（全て物理 px）
        AxisSpec::Px(dx) => base_pos
            .x
            .saturating_add(base_bp.x)
            .saturating_add(dx)
            .saturating_sub(target_bp.x),
    };
    let y = match directive.y {
        AxisSpec::Fix => target_pos.y, // 現状維持（Y は Fix なら同型）
        AxisSpec::Px(dy) => base_pos
            .y
            .saturating_add(base_bp.y)
            .saturating_add(dy)
            .saturating_sub(target_bp.y),
    };

    Some(PointPx { x, y })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    /// 正典省略時既定（fix/fix/0/screen/left.top）。空トークン列でも決定論的に既定へ落ちる。
    #[test]
    fn canon_omission_defaults() {
        let d = parse_move_directive(0, &[]).expect("空 positional は既定へ落ちて Ok");
        assert_eq!(
            d,
            MoveDirective {
                scope: 0,
                x: AxisSpec::Fix,
                y: AxisSpec::Fix,
                duration_ms: 0,
                base: MoveBase::Screen,
                base_offset: RefPoint::LEFT_TOP,
                move_offset: RefPoint::LEFT_TOP,
            }
        );
        // 空文字トークンで明示的に埋めても同じ既定（省略と空は同義・R4.2 の空トークン意味論）。
        let d2 = parse_move_directive(0, &toks(&["", "", "", "", "", ""]))
            .expect("空トークン埋めも既定へ落ちる");
        assert_eq!(d, d2);
    }

    /// 裸 `base`（ドット無し）≡ `base.base`（正典形式 `X.Y` の de-facto・R5.2 対応表）。
    #[test]
    fn bare_base_equals_base_base() {
        let bare = parse_move_directive(0, &toks(&["", "", "", "0", "base", "base"]))
            .expect("裸 base は Ok");
        let dotted = parse_move_directive(0, &toks(&["", "", "", "0", "base.base", "base.base"]))
            .expect("base.base は Ok");
        assert_eq!(bare.base_offset, RefPoint::BASE_BASE);
        assert_eq!(bare.move_offset, RefPoint::BASE_BASE);
        assert_eq!(bare, dotted, "裸 base と base.base は完全等価");
    }

    /// time>0 は `Ok` のまま `duration_ms` を保持し、記録として縮退が surface する（R5.4）。
    #[test]
    fn timed_move_kept_and_recorded() {
        let d = parse_move_directive(0, &toks(&["100", "", "2500", "0", "base", "base"]))
            .expect("time>0 は縮退記録付きでも Ok");
        assert_eq!(d.duration_ms, 2500);
        assert!(
            d.m1_degradations()
                .contains(&MoveDegradation::TimedMoveImmediate { duration_ms: 2500 }),
            "time>0 は m1_degradations に TimedMoveImmediate として記録される"
        );
    }

    /// 名前付き `--key=value` 形は M1 縮退＝`Err(NamedForm)`（記録付きスキップ・語彙は将来 additive）。
    #[test]
    fn named_form_is_degraded_err() {
        let err = parse_move_directive(
            0,
            &toks(&["--X=80", "--Y=-400", "--time=2500", "--base=screen"]),
        )
        .expect_err("名前付き形は Err へ縮退");
        assert_eq!(err, MoveDegradation::NamedForm("--X=80".to_string()));

        // positional に 1 トークンだけ名前付きが混入しても検出する。
        let err2 = parse_move_directive(0, &toks(&["-353", "", "", "--base=screen"]))
            .expect_err("混在でも名前付きを検出");
        assert!(matches!(err2, MoveDegradation::NamedForm(_)));
    }

    /// 基準語彙の受理と縮退分類（数値スコープ＝実導出／screen 等＝語彙保持＋UnsupportedBase 記録）。
    #[test]
    fn base_vocab_acceptance_and_classification() {
        // 数値スコープ＝M1 実導出（m1_degradations に基準縮退なし）。
        for scope_str in ["0", "1", "2"] {
            let d = parse_move_directive(0, &toks(&["", "", "", scope_str, "base", "base"]))
                .expect("数値スコープ基準は Ok");
            let n: u32 = scope_str.parse().unwrap();
            assert_eq!(d.base, MoveBase::Scope(n));
            assert!(d.base.is_m1_derived());
            assert!(
                !d.m1_degradations()
                    .iter()
                    .any(|g| matches!(g, MoveDegradation::UnsupportedBase(_))),
                "数値スコープ基準は縮退記録を持たない"
            );
        }

        // 非スコープ語＝語彙保持（Ok）＋UnsupportedBase 記録（M1 非実導出）。
        for (word, expected) in [
            ("screen", MoveBase::Screen),
            ("primaryscreen", MoveBase::PrimaryScreen),
            ("me", MoveBase::Me),
            ("global", MoveBase::Global),
        ] {
            let d = parse_move_directive(0, &toks(&["", "", "", word, "base", "base"]))
                .unwrap_or_else(|_| panic!("基準語 {word} は語彙保持で Ok"));
            assert_eq!(d.base, expected);
            assert!(!d.base.is_m1_derived());
            assert!(
                d.m1_degradations()
                    .contains(&MoveDegradation::UnsupportedBase(expected)),
                "非スコープ基準 {word} は UnsupportedBase として記録される"
            );
        }

        // 未知の基準語は防御的に Err（非 panic）。
        let err = parse_move_directive(0, &toks(&["", "", "", "nonsense"]))
            .expect_err("未知基準は Err");
        assert_eq!(err, MoveDegradation::UnknownBase("nonsense".to_string()));
    }

    /// fixture `\1\![move,-353,,,0,base,base]`（scope 1）の完全一致（R9.3 の直入力檻の parse 部）。
    #[test]
    fn fixture_move_353_scope1() {
        let d = parse_move_directive(1, &toks(&["-353", "", "", "0", "base", "base"]))
            .expect("fixture move は Ok");
        assert_eq!(
            d,
            MoveDirective {
                scope: 1,
                x: AxisSpec::Px(-353),
                y: AxisSpec::Fix,
                duration_ms: 0,
                base: MoveBase::Scope(0),
                base_offset: RefPoint::BASE_BASE,
                move_offset: RefPoint::BASE_BASE,
            }
        );
        // fixture は数値スコープ基準＋time=0 ゆえ M1 縮退記録は空（実導出の正規経路）。
        assert!(d.m1_degradations().is_empty());
    }

    /// 軸トークンが fix でも i32 でもない場合は防御的に Err（非 panic）。
    #[test]
    fn unparsable_axis_is_err() {
        let err = parse_move_directive(0, &toks(&["abc"]))
            .expect_err("非数値・非 fix の軸は Err");
        assert_eq!(
            err,
            MoveDegradation::UnparsableAxis {
                axis: Axis::X,
                token: "abc".to_string(),
            }
        );
    }

    // -------------------------------------------------------------------------
    // basepos 型シーム＋座標算出（task 7.2・R5.2・全て物理 px・R-6 対策）
    // -------------------------------------------------------------------------

    use wintf::ecs::{Point, SizeI, WindowPos};

    /// 位置＋寸法を持つ WindowPos（物理 px・follow.rs のヘルパ流儀）。
    fn win(x: i32, y: i32, w: i32, h: i32) -> WindowPos {
        WindowPos {
            position: Some(Point { x, y }),
            size: Some(SizeI::new(w, h)),
            ..Default::default()
        }
    }

    /// 正典既定 basepos は (幅÷2, 高さ＝下端)（R5.2・A-1）。奇数幅は整数切り捨て。
    #[test]
    fn canon_default_basepos_is_half_width_and_bottom() {
        assert_eq!(
            CanonDefaultBasepos.basepos(SizeI::new(400, 687)),
            PointPx { x: 200, y: 687 },
            "x=幅÷2・y=下端（＝height・窓左上原点相対）"
        );
        // 奇数幅は整数除算で切り捨て（435÷2=217）。
        assert_eq!(
            CanonDefaultBasepos.basepos(SizeI::new(435, 100)),
            PointPx { x: 217, y: 100 }
        );
    }

    /// fixture `\1\![move,-353,,,0,base,base]` の X 検算＝`pos0.x + w0/2 − 353 − w1/2`・Y 現状維持。
    /// base=scope0=むらさき窓 pos0=(1000,500) size0=(400,687)、target=scope1=エモ窓
    /// pos1=(1200,800) size1=(300,434) の具体値で完全一致を固定する。
    #[test]
    fn fixture_move_353_x_and_y_unchanged() {
        let directive = parse_move_directive(1, &toks(&["-353", "", "", "0", "base", "base"]))
            .expect("fixture move は Ok");
        let base = win(1000, 500, 400, 687);
        let target = win(1200, 800, 300, 434);

        let pos = resolve_move_target_position(&CanonDefaultBasepos, &base, &target, &directive)
            .expect("位置・寸法が揃うので算出できる");

        // x' = 1000 + 200 − 353 − 150 = 697
        assert_eq!(pos.x, 697, "x' = pos0.x + w0/2 − 353 − w1/2");
        // Y は Fix ゆえ target の現在 Y を現状維持
        assert_eq!(pos.y, 800, "Y=Fix は対象窓の現在 Y を現状維持");
    }

    /// Y=Px 経路も対称に効く（Y が「常に現状維持」へ hardcode されていないことの檻）。
    /// x=Fix・y=Px(50) → x' は target.x 現状維持、y' = pos0.y + h0 + 50 − h1。
    #[test]
    fn y_px_axis_is_symmetric_not_hardcoded() {
        let directive = parse_move_directive(1, &toks(&["", "50", "", "0", "base", "base"]))
            .expect("y=Px は Ok");
        assert_eq!(directive.x, AxisSpec::Fix);
        assert_eq!(directive.y, AxisSpec::Px(50));

        let base = win(1000, 500, 400, 687);
        let target = win(1200, 800, 300, 434);
        let pos = resolve_move_target_position(&CanonDefaultBasepos, &base, &target, &directive)
            .expect("算出できる");

        // X=Fix → 現状維持
        assert_eq!(pos.x, 1200, "X=Fix は対象窓の現在 X を現状維持");
        // y' = 500 + 687 + 50 − 434 = 803
        assert_eq!(pos.y, 803, "y' = pos0.y + h0 + dy − h1（下端 basepos で対称）");
    }

    /// BaseposResolver はトレイト＝差替シーム。テストダブルが別 basepos を供給すると
    /// 算出結果がその出力を用いて変わる（宣言 point.basepos の追跡 spec 差替点の証明）。
    #[test]
    fn computation_honors_resolver_seam() {
        /// basepos を「幅・高さそのもの（右下端）」とする差替ダブル（正典既定＝中央下端とは別物）。
        struct FullSizeBasepos;
        impl BaseposResolver for FullSizeBasepos {
            fn basepos(&self, window_size: SizeI) -> PointPx {
                PointPx {
                    x: window_size.width,
                    y: window_size.height,
                }
            }
        }

        let directive = parse_move_directive(1, &toks(&["-353", "", "", "0", "base", "base"]))
            .expect("fixture move は Ok");
        let base = win(1000, 500, 400, 687);
        let target = win(1200, 800, 300, 434);

        let pos = resolve_move_target_position(&FullSizeBasepos, &base, &target, &directive)
            .expect("算出できる");
        // x' = pos0.x + w0 − 353 − w1 = 1000 + 400 − 353 − 300 = 747（Canon の 697 とは別）
        assert_eq!(pos.x, 747, "resolver の basepos 出力（幅そのもの）が使われる");
        assert_ne!(pos.x, 697, "正典既定（中央）とは異なる＝シームが効いている");
    }

    /// 両軸 Fix（現状維持のみ）は基準窓の寸法欠落でも対象窓の現在位置を返す（no-op 移動）。
    #[test]
    fn both_fix_returns_current_position() {
        let directive = parse_move_directive(1, &toks(&["", "", "", "0", "base", "base"]))
            .expect("両軸省略は Ok");
        assert_eq!(directive.x, AxisSpec::Fix);
        assert_eq!(directive.y, AxisSpec::Fix);

        // 基準窓は寸法なし（現状維持のみゆえ basepos 不要）。
        let base = WindowPos::default();
        let target = win(1200, 800, 300, 434);
        let pos = resolve_move_target_position(&CanonDefaultBasepos, &base, &target, &directive)
            .expect("両軸 Fix は現状維持で算出できる");
        assert_eq!(pos, PointPx { x: 1200, y: 800 });
    }

    /// 位置・寸法欠落（窓生成前等）で Px 軸を含むと算出不能＝`None`（呼び出し側が warn＋継続・R5.5）。
    #[test]
    fn missing_geometry_with_px_axis_is_none() {
        let directive = parse_move_directive(1, &toks(&["-353", "", "", "0", "base", "base"]))
            .expect("fixture move は Ok");
        // 対象窓に寸法が無い（basepos 算出不能）。
        let base = win(1000, 500, 400, 687);
        let target = WindowPos {
            position: Some(Point { x: 1200, y: 800 }),
            size: None,
            ..Default::default()
        };
        assert!(
            resolve_move_target_position(&CanonDefaultBasepos, &base, &target, &directive).is_none(),
            "Px 軸で寸法欠落は算出不能＝None"
        );
    }
}
