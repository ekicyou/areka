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

// task 7.1/7.2 が純粋な型＋parse＋basepos シーム＋座標算出、task 7.3 が talk スレッド側消費
// `MoveCueSink` を載せる。残る UI 末端の消費点（`apply_move_directive`＝7.4・`mod.rs` の channel
// 配線＝9.1）は後続タスクが足す。それまで `MoveCueSink`／`resolve_move_target_position` 等は
// 非 test ビルド（bin 本体）から未参照ゆえ dead_code が出るが、これは段階実装の想定内であり、
// 後続タスクの結線（9.1）で解消される（本 allow は wiring 着地時に撤去する）。
#![allow(dead_code)]

use std::sync::mpsc::Sender;

use bevy_ecs::prelude::World;
use dola::cue::{CueCommand, TalkCue};
use tracing::{debug, info, warn};
use wintf::ecs::{SizeI, WindowPos};

use crate::placement::follow::move_window_to;
use crate::placement::resolver::PointPx;
use crate::placement::spawn::GhostWindows;

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

// =============================================================================
// MoveCueSink（talk スレッド純粋解釈）— `\![move]` 名前選別消費の最初の実消費者
// （task 7.3・R4.5・R8.5）
// =============================================================================

/// `\![move]` の talk スレッド側消費 sink（design「MoveCueSink＋純粋解釈＋UI 適用」・R4.5）。
///
/// broadcast された全 cue のうち、キャリア正準形（`Custom` の String Array）であり自らの
/// コマンド名リテラル `"move"` を運ぶもの**だけ**を名前自己選別して解釈し、
/// [`parse_move_directive`] の結果を mpsc で UI スレッド（frame 相の
/// `apply_move_directive`＝task 7.4／channel 配線＝task 9.1）へ送出する。それ以外
/// （非キャリア・担当外コマンド名・非数値 scope・parse 縮退）は**記録付き良性スキップ**へ
/// 縮退する——無音破棄でも panic でもない（R4.5／R8.5・log-first）。
///
/// # 名前自己選別（R4.5・高々 1 消費者）
///
/// 担当判定は本 sink が自らのコマンド名リテラル `name == "move"` で行う——dola はコマンド名の
/// 語彙も名前写像 API も持たず、`MoveCueSink` は中央権威表に依存せず `"move"` を自己選別する。
/// 「1 名前=高々 1 消費者」の一意性は結線層（areka）の消費者台帳（task 2.5）が保証する
/// （dola の権威表ではない）。
///
/// # duration honor 不変（R4.5 後段）
///
/// 本 sink は cue を**観測**するのみで envelope の `duration` に一切触れない——名前で担当が引けても
/// 引けなくても、duration honor 契約（全演者が任意 cue の `duration` を尊重する）に影響を与えない。
///
/// # Clone + Send（boot 型境界）
///
/// dispatcher は talk ごとに sink を clone する（`GhostBootOptions.sinks` は
/// `dola::cue::CueSink + Clone + Send + 'static`）。内側 [`Sender`] は常に `Clone` で、全 clone は
/// 単一の受信端（`Emo2Wiring` の `Receiver`）への送信端＝配送意味は同一ゆえ、そのまま `derive(Clone)`
/// が成り立つ（`MoveDirective: Send` により `Sender<MoveDirective>: Send`）。
#[derive(Clone)]
pub struct MoveCueSink {
    /// UI スレッド（frame 相 drain）への送出端（Clone 可・全 clone は単一受信端へ配送）。
    tx: Sender<MoveDirective>,
}

impl MoveCueSink {
    /// mpsc 送信端（`Emo2Wiring` の `Receiver` と対・task 9.1 が生成）から sink を構築する。
    pub fn new(tx: Sender<MoveDirective>) -> Self {
        Self { tx }
    }
}

/// 演者非依存の単一出力契約 [`dola::cue::CueSink`] を実装する（`GhostBootOptions.sinks` の
/// broadcast 登録先が要求する形・task 7.3）。broadcast 下では担当外 cue（`Text`／`Emote`／
/// 他コマンド名のキャリア等）も本 sink へ届くが、名前選別で `"move"` 以外は記録付き良性スキップへ
/// 縮退する（duration honor には触れない・R4.5/R8.5）。
impl dola::cue::CueSink for MoveCueSink {
    fn emit(&mut self, cue: TalkCue) {
        // 1) キャリア抽出。非キャリア（Text/Emote 等の担当外 broadcast）は良性スキップ。
        //    Custom なのに非正準 params のときの severity は**宛名規律**（D8④）で分ける:
        //    宛名（`Custom{command}` フィールド・params 非正準でも読める）が "move"＝自分宛の
        //    壊れ物ゆえ warn・他人宛/未知名＝担当外ゆえ debug（報告責任は宛名の担当者）。
        let Some((name, tokens)) = cue.command.as_command_carrier() else {
            match &cue.command {
                CueCommand::Custom { command, .. } if command == "move" => warn!(
                    command = ?cue.command,
                    "MoveCueSink: 自分宛（move）の非正準 Custom params を良性スキップ（D8④）"
                ),
                CueCommand::Custom { .. } => debug!(
                    command = ?cue.command,
                    "MoveCueSink: 他人宛/未知名の非正準 Custom params を良性スキップ（担当外・D8④）"
                ),
                _ => debug!(
                    command = ?cue.command,
                    "MoveCueSink: 非キャリア cue を良性スキップ（担当外・R8.5）"
                ),
            }
            return;
        };

        // 2) 名前自己選別（消費者自身のコマンド名リテラル）。"move" のみ解釈する。
        //    dola は名前語彙を持たず、一意性（1 名前=高々 1 消費者）は areka 消費者台帳（task 2.5）
        //    が保証する——本 sink は中央権威表に依存せず自らの名前で自己選別する（R4.5）。
        if name != "move" {
            debug!(
                name,
                "MoveCueSink: 担当外コマンド名を良性スキップ（名前自己選別・R4.5/R8.5）"
            );
            return;
        }

        // 3) scope は cue.actor（"0"/"1"）の u32 parse。非数値は warn＋スキップ（design「破損・異常」）。
        let scope = match cue.actor.as_str().parse::<u32>() {
            Ok(scope) => scope,
            Err(_) => {
                warn!(
                    actor = cue.actor.as_str(),
                    "MoveCueSink: cue.actor が非数値 scope のため \\![move] をスキップ（R5.5）"
                );
                return;
            }
        };

        // 4) 純粋解釈（決定論・no I/O）。Err（名前付き -- 形等）は記録付き良性スキップ（非 panic・R5.4）。
        let tokens: Vec<String> = tokens.iter().map(|s| s.to_string()).collect();
        match parse_move_directive(scope, &tokens) {
            Ok(directive) => {
                if self.tx.send(directive).is_err() {
                    // 受信端（Emo2Wiring）切断は talk を殺さない（log-first・非 panic・R5.5）。
                    warn!("MoveCueSink: MoveDirective の送出に失敗（受信端切断）");
                }
            }
            Err(degradation) => {
                warn!(
                    ?degradation,
                    "MoveCueSink: \\![move] の縮退を記録付き良性スキップ（語彙保持・R5.4）"
                );
            }
        }
    }
}

// =============================================================================
// apply_move_directive（UI スレッド適用）— frame 相 drain 先の実窓反映
// （task 7.4・R5.1/5.3/5.5/R6/9.5）
// =============================================================================

/// `\![move]` の [`MoveDirective`] を UI スレッド上で実窓へ反映する（design「MoveCueSink＋
/// 純粋解釈＋UI 適用」・R5.1/5.3/5.5/6.1/6.2/9.5）。
///
/// `directive.scope`／基準スコープ→[`GhostWindows`]（Resource・World から読む）で対象・基準の
/// キャラ窓 entity を解決し、両窓の [`WindowPos`]（物理 px）から basepos シーム
/// （[`CanonDefaultBasepos`]・M1 実導出）経由で最終座標を算出し（[`resolve_move_target_position`]）、
/// [`move_window_to`] **のみ**を呼んで反映する。`&mut World` 署名で UI スレッド専有を型担保する。
///
/// # 永続分離（R6/6.1/6.2/9.5）
///
/// 本関数は唯一の位置ライター [`move_window_to`] だけを呼び、[`Anchored`](crate::placement::follow::Anchored)
/// （ドラッグ確定系の単一真実源）にも DragEnd 観測点（`on_char_drag_end`/`on_balloon_drag`）にも
/// 構造的に触れない——表示位置のみを動かし永続確定値を更新せず（R6.1）、第二の位置ライターを
/// 新設しない（R6.2）。統合檻は「適用前後で `Anchored` がビット同一」を直接 assert する。
/// バルーン随伴の offset 維持（R5.3）は `move_window_to` が内部で担う。
///
/// # 縮退（warn＋継続・`false`・R5.5）
///
/// 非スコープ基準（screen/primaryscreen/me/global＝M1 非実導出・emo2 未使用）・`GhostWindows`
/// 未挿入・対象/基準 scope の窓不在・`WindowPos` 不在・座標算出不能（位置/寸法欠落）はいずれも
/// warn＋`false`——silent no-op でも panic でもなく talk を殺さない（log-first・
/// [[areka-log-first-no-silent-failure]]）。
pub fn apply_move_directive(world: &mut World, directive: &MoveDirective) -> bool {
    // 基準は M1 では数値スコープのみ実導出。非スコープ（screen 等）は語彙保持のうえ warn＋スキップ。
    let MoveBase::Scope(base_scope) = directive.base else {
        warn!(
            base = ?directive.base,
            scope = directive.scope,
            "apply_move_directive: 非スコープ基準は M1 非実導出のためスキップ（R5.5）"
        );
        return false;
    };

    // scope→GhostWindows（Resource）で対象・基準のキャラ窓 entity を解決する。
    let Some(ghost_windows) = world.get_resource::<GhostWindows>() else {
        warn!("apply_move_directive: GhostWindows 未挿入のため \\![move] をスキップ（R5.5）");
        return false;
    };
    let Some(target) = ghost_windows.char_window(directive.scope as usize) else {
        warn!(
            scope = directive.scope,
            "apply_move_directive: 対象 scope の char 窓が GhostWindows に無い（R5.5）"
        );
        return false;
    };
    let Some(base_window) = ghost_windows.char_window(base_scope as usize) else {
        warn!(
            base_scope,
            "apply_move_directive: 基準 scope の char 窓が GhostWindows に無い（R5.5）"
        );
        return false;
    };

    // 両窓の WindowPos（物理 px・`Copy`）を読む。move_window_to は `&mut World` を要するため、
    // ここで値コピーして共有 borrow を解放する。不在（窓生成前の異常系）は warn＋false。
    let Some(base_pos) = world.get::<WindowPos>(base_window).copied() else {
        warn!(
            ?base_window,
            "apply_move_directive: 基準窓の WindowPos 不在（窓生成前）のためスキップ（R5.5）"
        );
        return false;
    };
    let Some(target_pos) = world.get::<WindowPos>(target).copied() else {
        warn!(
            ?target,
            "apply_move_directive: 対象窓の WindowPos 不在（窓生成前）のためスキップ（R5.5）"
        );
        return false;
    };

    // basepos シーム（M1 は CanonDefaultBasepos）経由で最終座標を算出（全て物理 px・R-6 対策）。
    // 位置/寸法欠落で算出不能なら None＝warn＋継続（R5.5）。
    let Some(pos) =
        resolve_move_target_position(&CanonDefaultBasepos, &base_pos, &target_pos, directive)
    else {
        warn!(
            scope = directive.scope,
            "apply_move_directive: 位置・寸法欠落で座標算出不能のためスキップ（R5.5）"
        );
        return false;
    };

    // 反映は move_window_to のみ（唯一の位置ライター・バルーン随伴 offset 維持を内包・R5.3/6.2）。
    // Anchored・DragEnd 観測点には触れない（R6/9.5）。
    let moved = move_window_to(world, target, pos.x, pos.y);

    // 成功経路の positive ログ（実機 OnFirstBoot サインオフ／R9.6 の grep 拠点）。degradation は
    // 全て上で warn＋false 済みゆえ、ここは唯一の真の成功点——move_window_to が実際に窓を動かした
    // （`true`）ときのみ 1 本出す（`false`＝窓ハンドル未生成の縮退では出さない）。target_pos.position
    // は resolve 成功が Some を保証（[`resolve_move_target_position`] が None で早期 return）。
    if moved {
        let (from_x, from_y) = target_pos.position.map(|p| (p.x, p.y)).unwrap_or_default();
        info!(
            scope = directive.scope,
            base_scope,
            from_x,
            from_y,
            to_x = pos.x,
            to_y = pos.y,
            "apply_move_directive: move 適用完了（scope→物理px移動）"
        );
    }

    moved
}

#[cfg(test)]
#[path = "move_cue_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "move_cue_move_sink_tests.rs"]
mod move_sink_tests;

#[cfg(test)]
#[path = "move_cue_apply_move_tests.rs"]
mod apply_move_tests;

#[cfg(test)]
#[path = "move_cue_move_severity_log_tests.rs"]
mod move_severity_log_tests;
