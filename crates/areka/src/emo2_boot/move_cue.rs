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
use dola::cue::{command_target_of, CueCommand, CueTarget, TalkCue};
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
/// broadcast された全 cue のうち、キャリア正準形（`Custom` の String Array）でありコマンド名
/// レベルの単一権威表 [`command_target_of`] が `"move"` を [`CueTarget::Window`] へ割り当てる
/// もの**だけ**を解釈し、[`parse_move_directive`] の結果を mpsc で UI スレッド（frame 相の
/// `apply_move_directive`＝task 7.4／channel 配線＝task 9.1）へ送出する。それ以外
/// （非キャリア・担当外コマンド名・非数値 scope・parse 縮退）は**記録付き良性スキップ**へ
/// 縮退する——無音破棄でも panic でもない（R4.5／R8.5・log-first）。
///
/// # 名前選別（R4.5・高々 1 消費者）
///
/// 担当判定は [`command_target_of`] 単一が権威表であり、`MoveCueSink` は私的名前リストを持たない。
/// `command_target_of(name) == Some(Window)` かつ `name == "move"` の 2 条件で自らの担当を限定する
/// （`Window` 割り当てが将来 `"move"` 以外へ拡張されても本 sink は `"move"` 専任＝高々 1 消費者を保つ）。
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
        //    Custom なのに非正準 params のときのみ異常として warn（design「破損・異常」・R8.5）。
        let Some((name, tokens)) = cue.command.as_command_carrier() else {
            if matches!(cue.command, CueCommand::Custom { .. }) {
                warn!(
                    command = ?cue.command,
                    "MoveCueSink: 非正準 Custom params（as_command_carrier=None）を良性スキップ（R8.5）"
                );
            } else {
                debug!(
                    command = ?cue.command,
                    "MoveCueSink: 非キャリア cue を良性スキップ（担当外・R8.5）"
                );
            }
            return;
        };

        // 2) 名前レベル選別（単一権威表 command_target_of）。"move"→Window のみ解釈する。
        //    高々 1 消費者: MoveCueSink は "move" 専任ゆえ name=="move" も併せ確認する（R4.5）。
        if command_target_of(name) != Some(CueTarget::Window) || name != "move" {
            debug!(
                name,
                "MoveCueSink: 担当外コマンド名を良性スキップ（名前選別・R4.5/R8.5）"
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

// =============================================================================
// task 7.5 — move 決定論檻 全網羅 監査（audit + gap-fill）
//
// design.md Testing Strategy の 2 項目を既存檻へ 1:1 で写像し、全項目が具体的
// （fail-if-broken）に固定済みであることを監査した結果、追加檻は不要（gap ゼロ）と確定。
// 監査時点の写像（項目 → 檻）:
//
// Unit Tests item 4「parse_move_directive 檻（R5.2/5.4）」（下記 `mod tests`）:
//   - 正典省略既定〔fix/fix/0/screen/left.top〕 → `canon_omission_defaults`
//     （空 positional／空トークン埋めの両形で MoveDirective 構造体を厳密一致 assert）
//   - 裸 base≡base.base                        → `bare_base_equals_base_base`
//   - time>0 縮退                              → `timed_move_kept_and_recorded`
//   - 名前付き形縮退                            → `named_form_is_degraded_err`（純名前形＋混在検出）
//   - 基準語彙**全種**の受理/縮退分類          → `base_vocab_acceptance_and_classification`
//     数値スコープ 0/1/2＝受理（is_m1_derived∧UnsupportedBase 記録なし）／
//     screen・primaryscreen・me・global の 4 語＝各 variant 受理＋UnsupportedBase 記録／
//     未知語＝Err(UnknownBase)。MoveBase の全 5 variant＋未知を網羅。
//   - （fixture parse 検算）                    → `fixture_move_353_scope1`
//   - （防御的 Err）                            → `unparsable_axis_is_err`
//
// Integration Tests item 4「move 経路檻（R5.1/5.3/5.5/R6/9.5）」（`mod apply_move_tests`）:
//   - fixture 検算物理座標                      → `apply_moves_target_to_fixture_position`（Point{697,800}）
//   - バルーン随伴 offset 維持                  → `apply_keeps_balloon_offset`
//   - 対象不在 warn+false                       → `apply_target_absent_returns_false_without_mutation`
//   - Anchored 不変（対象**と**基準の両窓）     → `apply_leaves_anchored_bit_identical`
//   - （非スコープ基準 warn+false）             → `apply_non_scope_base_returns_false`
//
// 監査結論: 両項目とも欠落・弱檻（トートロジー/部分網羅）なし。全 26 檻が具体 assert で緑。
// =============================================================================

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

// =============================================================================
// MoveCueSink 名前選別 sink 檻（task 7.3・R4.5・R8.5）
// =============================================================================

#[cfg(test)]
mod move_sink_tests {
    use super::*;
    use dola::cue::{ActorKey, CueCommand, CueSink, TalkCue};
    use std::sync::mpsc::{channel, TryRecvError};

    /// `\![name,tokens...]` 汎用キャリア cue を組む（正準形＝`Custom` の String Array）。
    fn carrier_cue(actor: &str, name: &str, tokens: &[&str]) -> TalkCue {
        let toks: Vec<String> = tokens.iter().map(|s| s.to_string()).collect();
        TalkCue {
            at: 0.0,
            actor: ActorKey::from(actor),
            command: CueCommand::command_carrier(name, toks),
            duration: 0.0,
        }
    }

    /// 名前選別: `"move"` キャリアのみ解釈して MoveDirective を送出する（R4.5・最初の実消費者）。
    #[test]
    fn move_carrier_sends_directive() {
        let (tx, rx) = channel::<MoveDirective>();
        let mut sink = MoveCueSink::new(tx);
        sink.emit(carrier_cue("0", "move", &["-353", "", "", "0", "base", "base"]));
        let d = rx.try_recv().expect("move キャリアは MoveDirective を送出する");
        assert_eq!(d.scope, 0);
        assert_eq!(d.x, AxisSpec::Px(-353));
        assert_eq!(d.base, MoveBase::Scope(0));
        assert_eq!(rx.try_recv(), Err(TryRecvError::Empty), "送出は 1 件のみ");
    }

    /// 担当外キャリア（`bind`/`raise`/未知名）は良性スキップ＝何も送出しない
    /// （高々 1 消費者・`command_target_of` 単一権威表・R4.5）。
    #[test]
    fn non_move_carrier_is_benign_skip() {
        for name in ["bind", "raise", "unknownfoo"] {
            let (tx, rx) = channel::<MoveDirective>();
            let mut sink = MoveCueSink::new(tx);
            sink.emit(carrier_cue("0", name, &["1", "2"]));
            assert_eq!(
                rx.try_recv(),
                Err(TryRecvError::Empty),
                "担当外キャリア {name} は何も送出しない（良性スキップ・R8.5）"
            );
        }
    }

    /// 非キャリア cue（`Text` 等の担当外 broadcast）は良性スキップ＝何も送出しない（R8.5）。
    #[test]
    fn non_carrier_cue_is_benign_skip() {
        let (tx, rx) = channel::<MoveDirective>();
        let mut sink = MoveCueSink::new(tx);
        sink.emit(TalkCue {
            at: 0.0,
            actor: ActorKey::from("0"),
            command: CueCommand::Text("アヒル".into()),
            duration: 0.0,
        });
        assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
    }

    /// scope 抽出: `\1` actor → `MoveDirective.scope == 1`（scope は cue.actor 由来）。
    #[test]
    fn scope_reflects_actor() {
        let (tx, rx) = channel::<MoveDirective>();
        let mut sink = MoveCueSink::new(tx);
        sink.emit(carrier_cue("1", "move", &["-353", "", "", "0", "base", "base"]));
        let d = rx.try_recv().expect("送出される");
        assert_eq!(d.scope, 1, "scope は cue.actor（\\1）由来");
    }

    /// 非数値 actor（`sakura` 等）は warn＋スキップ＝非 panic・何も送出しない（design 破損・異常）。
    #[test]
    fn non_numeric_actor_is_skipped() {
        let (tx, rx) = channel::<MoveDirective>();
        let mut sink = MoveCueSink::new(tx);
        sink.emit(carrier_cue("sakura", "move", &["-353"]));
        assert_eq!(
            rx.try_recv(),
            Err(TryRecvError::Empty),
            "非数値 scope はスキップ（非 panic）"
        );
    }

    /// parse の `Err`（名前付き `--` 形）→ 記録付き良性スキップ・何も送出しない・非 panic（R5.4）。
    #[test]
    fn parse_err_named_form_is_skipped() {
        let (tx, rx) = channel::<MoveDirective>();
        let mut sink = MoveCueSink::new(tx);
        sink.emit(carrier_cue("0", "move", &["--X=80", "--Y=-400"]));
        assert_eq!(
            rx.try_recv(),
            Err(TryRecvError::Empty),
            "名前付き -- 形は Err 縮退で送出しない"
        );
    }

    /// `MoveCueSink` は Clone: clone 側から送出しても同一受信端へ届く
    /// （dispatcher が talk ごとに sink を clone する前提・boot 型境界）。
    #[test]
    fn clone_reaches_same_receiver() {
        let (tx, rx) = channel::<MoveDirective>();
        let sink = MoveCueSink::new(tx);
        let mut clone = sink.clone();
        clone.emit(carrier_cue("0", "move", &["10", "", "", "0", "base", "base"]));
        let d = rx.try_recv().expect("clone からの送出も同一受信端へ届く");
        assert_eq!(d.x, AxisSpec::Px(10));
        drop(sink); // 元 sink 生存の確認（Clone は独立ハンドル）。
    }

    /// boot 型境界（`dola::cue::CueSink + Clone + Send + 'static`）をコンパイル時に固定する。
    #[test]
    fn satisfies_boot_sink_bounds() {
        fn require<T: dola::cue::CueSink + Clone + Send + 'static>() {}
        require::<MoveCueSink>();
    }
}

// =============================================================================
// apply_move_directive（UI スレッド適用）統合檻（task 7.4・R5.1/5.3/5.5/R6/9.5）
//
// 偽装境界: 実 HWND なしの headless World（`fake_handle` パターン・follow.rs 流儀）で
// `spawn_ghost_windows` により実 GhostWindows＋char/balloon 窓（Anchored/WindowPos/
// BalloonFollow 付き）を組み、既知 WindowPos に対し apply_move_directive を駆動して
// fixture 検算式どおりの物理座標・バルーン随伴 offset 維持・対象不在 warn+false・
// Anchored ビット同一（第二位置ライター非混入の構造檻）を固定する。
// =============================================================================

#[cfg(test)]
mod apply_move_tests {
    use super::*;
    use bevy_ecs::prelude::{Entity, World};
    use windows::Win32::Foundation::{HINSTANCE, HWND};

    use crate::placement::follow::{Anchored, BalloonFollow};
    use crate::placement::resolver::{Anchor, ScopePlacement, SizePx};
    use crate::placement::source::GhostTitles;
    use crate::placement::spawn::{spawn_ghost_windows, GhostWindows};
    use wintf::ecs::{Point, WindowHandle, WindowPos};

    /// 偽 HWND の WindowHandle（実窓なし・headless 決定論シーム）。
    fn fake_handle(raw: usize) -> WindowHandle {
        WindowHandle {
            hwnd: HWND(raw as *mut _),
            instance: HINSTANCE::default(),
        }
    }

    /// 既知位置・寸法の解決済み配置（balloon_offset は char 窓へ BalloonFollow.offset として転写）。
    fn placement(scope: usize, cx: i32, cy: i32, cw: i32, ch: i32, boff: PointPx) -> ScopePlacement {
        ScopePlacement {
            scope,
            char_pos: PointPx { x: cx, y: cy },
            char_size: SizePx { w: cw, h: ch },
            balloon_pos: PointPx {
                x: cx + boff.x,
                y: cy + boff.y,
            },
            balloon_size: SizePx { w: 200, h: 150 },
            balloon_offset: boff,
            anchor: Anchor::Bottom,
        }
    }

    /// 各窓へ偽 WindowHandle を付与する（move_window_to の反映口 enqueue_window_set_pos が
    /// WindowPos を書ける条件＝WindowHandle 実在）。
    fn attach_fake_handles(world: &mut World, gw: &GhostWindows) {
        let mut raw = 0x100usize;
        for scope in gw.scopes().collect::<Vec<_>>() {
            for e in [
                gw.char_window(scope).unwrap(),
                gw.balloon_window(scope).unwrap(),
            ] {
                world.entity_mut(e).insert(fake_handle(raw));
                raw += 0x10;
            }
        }
    }

    /// base=scope0 (1000,500,400,687)・target=scope1 (1200,800,300,434) の headless World＋
    /// GhostWindows（全窓に偽 WindowHandle 付与済み）。
    fn move_world() -> (World, GhostWindows) {
        let placements = vec![
            placement(0, 1000, 500, 400, 687, PointPx { x: 285, y: -19 }),
            placement(1, 1200, 800, 300, 434, PointPx { x: 285, y: -19 }),
        ];
        let mut world = World::new();
        let gw = spawn_ghost_windows(
            &mut world,
            &placements,
            &GhostTitles::from_scope_titles([(0, "a".to_string()), (1, "b".to_string())]),
        );
        attach_fake_handles(&mut world, &gw);
        (world, gw)
    }

    fn pos_of(world: &World, e: Entity) -> Point {
        world
            .get::<WindowPos>(e)
            .expect("WindowPos があるはず")
            .position
            .expect("position があるはず")
    }

    /// fixture `\1\![move,-353,,,0,base,base]`（scope 1・base scope0）。
    fn fixture_directive() -> MoveDirective {
        parse_move_directive(
            1,
            &["-353", "", "", "0", "base", "base"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>(),
        )
        .expect("fixture move は Ok")
    }

    /// R5.1: fixture の即時移動が対象窓へ反映される。
    /// x' = pos0.x + w0/2 − 353 − w1/2 = 1000 + 200 − 353 − 150 = 697・y は Fix ゆえ現状維持（800）。
    #[test]
    fn apply_moves_target_to_fixture_position() {
        let (mut world, gw) = move_world();
        let target = gw.char_window(1).unwrap();

        assert!(
            apply_move_directive(&mut world, &fixture_directive()),
            "対象・基準窓が揃うので適用は成功する"
        );
        assert_eq!(
            pos_of(&world, target),
            Point { x: 697, y: 800 },
            "x'=pos0.x+w0/2−353−w1/2=697・y=Fix は現状維持"
        );
    }

    /// R5.3: バルーン随伴——移動後も balloon_pos − char_pos ≡ offset（move_window_to が内包）。
    #[test]
    fn apply_keeps_balloon_offset() {
        let (mut world, gw) = move_world();
        let target = gw.char_window(1).unwrap();
        let balloon = gw.balloon_window(1).unwrap();
        let offset = world
            .get::<BalloonFollow>(target)
            .copied()
            .expect("target に BalloonFollow")
            .offset;

        assert!(apply_move_directive(&mut world, &fixture_directive()));

        let cpos = pos_of(&world, target);
        let bpos = pos_of(&world, balloon);
        assert_eq!(bpos.x - cpos.x, offset.x, "offset x が移動後も維持される");
        assert_eq!(bpos.y - cpos.y, offset.y, "offset y が移動後も維持される");
    }

    /// R5.5: 対象 scope の窓が GhostWindows に無い→warn＋false・state 不変（panic しない）。
    #[test]
    fn apply_target_absent_returns_false_without_mutation() {
        // GhostWindows に scope0（基準）のみ。fixture の対象 scope1 は不在。
        let placements = vec![placement(0, 1000, 500, 400, 687, PointPx { x: 285, y: -19 })];
        let mut world = World::new();
        let gw = spawn_ghost_windows(
            &mut world,
            &placements,
            &GhostTitles::from_scope_titles([(0, "a".to_string())]),
        );
        attach_fake_handles(&mut world, &gw);
        let base = gw.char_window(0).unwrap();
        let before = pos_of(&world, base);

        assert!(
            !apply_move_directive(&mut world, &fixture_directive()),
            "対象 scope1 不在は false（R5.5）"
        );
        assert_eq!(pos_of(&world, base), before, "適用不成立で state は不変");
    }

    /// R6/9.5: apply の前後で対象・基準窓の `Anchored`（ドラッグ確定系の単一真実源）が
    /// ビット同一であること（apply が move_window_to のみを呼び第二の位置ライターを混入しない構造檻）。
    #[test]
    fn apply_leaves_anchored_bit_identical() {
        let (mut world, gw) = move_world();
        let target = gw.char_window(1).unwrap();
        let base = gw.char_window(0).unwrap();
        let target_before = world
            .get::<Anchored>(target)
            .copied()
            .expect("target に Anchored（spawn が付与）");
        let base_before = world
            .get::<Anchored>(base)
            .copied()
            .expect("base に Anchored（spawn が付与）");

        assert!(apply_move_directive(&mut world, &fixture_directive()));

        assert_eq!(
            world.get::<Anchored>(target).copied(),
            Some(target_before),
            "対象窓の Anchored はビット同一（永続確定系へ触れない・R6/9.5）"
        );
        assert_eq!(
            world.get::<Anchored>(base).copied(),
            Some(base_before),
            "基準窓の Anchored もビット同一"
        );
    }

    /// R5.5: 非スコープ基準（M1 非実導出）は warn＋false（座標算出へ進まない）。
    #[test]
    fn apply_non_scope_base_returns_false() {
        let (mut world, gw) = move_world();
        let target = gw.char_window(1).unwrap();
        let before = pos_of(&world, target);
        // base=screen（非スコープ・M1 縮退）。x=Px(-353)・y=Fix。
        let directive = parse_move_directive(
            1,
            &["-353", "", "", "screen", "base", "base"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>(),
        )
        .expect("screen 基準は語彙保持で Ok");
        assert_eq!(directive.base, MoveBase::Screen);

        assert!(
            !apply_move_directive(&mut world, &directive),
            "非スコープ基準は M1 非実導出のため false（R5.5）"
        );
        assert_eq!(pos_of(&world, target), before, "適用不成立で対象窓は不動");
    }

    // -------------------------------------------------------------------------
    // task 10.4 — MoveCueSink→channel→apply_move_directive→move_window_to の
    // 末端 pipeline を **一続きの unit** として駆動する統合檻（R5.3/5.5/6.1/6.2/9.5）。
    //
    // 既存檻との差分（重複回避）:
    //   - 7.4 `apply_move_tests`: `apply_move_directive` を **単体**で駆動（directive を
    //     parse で直接構築・sink/channel を経ない apply-in-isolation）。
    //   - 9.1 `move_cue_sink_reaches_emo2_wiring_receiver`（frame.rs）: sink→channel→
    //     `drain_move_directives` まで（**apply を呼ばない**＝窓を動かさない）。
    //   - 9.2 `run_move_drain_phase_applies_directive_when_ghost_windows_present`（frame.rs）:
    //     **生 `tx.send(directive)`**（sink を経ない）→drain→apply（channel→apply 結線）。
    //   - 9.3 `spine_move_cue_drives_window_move_end_to_end`（spine.rs）: full async spine
    //     （cue script→compile→dispatch→broadcast→実 MoveCueSink→channel→drain→apply）だが
    //     **座標のみ** assert（balloon 随伴・Anchored ビット同一・対象不在は檻に入れていない）。
    //
    // 10.4 の固有価値: `MoveCueSink::emit(キャリア cue)` の **名前選別＋actor→scope＋parse＋
    // channel handoff** と `apply_move_directive` の **座標算出＋move_window_to 反映** が
    // headless 単一スレッドで **正しく合成する** ことを、R5.3（balloon 随伴 offset 維持）・
    // R6/9.5（Anchored ビット同一）・R5.5（対象不在 warn+false・no mutation）まで **pipeline 越し**
    // に固定する（9.3 の full boot を要さない move-only の焦点檻）。

    use dola::cue::{ActorKey, CueCommand, CueSink, TalkCue};
    use std::sync::mpsc::channel;

    /// `\![move]` キャリア cue を **実 `MoveCueSink`→mpsc channel** に通し、drain した
    /// `MoveDirective` を返す（sink の名前選別＋`cue.actor`→scope＋`parse_move_directive`＋
    /// channel handoff を実経路で通す＝末端 pipeline の前段そのもの）。
    fn directive_via_sink(actor: &str, tokens: &[&str]) -> MoveDirective {
        let (tx, rx) = channel::<MoveDirective>();
        let mut sink = MoveCueSink::new(tx);
        sink.emit(TalkCue {
            at: 0.0,
            actor: ActorKey::from(actor),
            command: CueCommand::command_carrier(
                "move",
                tokens.iter().map(|s| s.to_string()).collect(),
            ),
            duration: 0.0,
        });
        rx.try_recv()
            .expect("move キャリアは sink→channel を通って MoveDirective を送出する")
    }

    /// R5.3/6.1/6.2/9.5: fixture `\1\![move,-353,,,0,base,base]` を **sink→channel→apply**
    /// の一続きに通し、①対象窓が fixture 検算座標へ移動②バルーン随伴 offset 維持③対象・基準の
    /// `Anchored` がビット同一——を **pipeline 越し** に同時固定する（sink 名前選別＋parse＋
    /// handoff と apply の座標算出＋move_window_to が正しく合成する証明）。
    #[test]
    fn pipeline_sink_to_apply_moves_keeps_balloon_and_anchored() {
        let (mut world, gw) = move_world();
        let target = gw.char_window(1).unwrap();
        let base = gw.char_window(0).unwrap();
        let balloon = gw.balloon_window(1).unwrap();

        let offset = world
            .get::<BalloonFollow>(target)
            .copied()
            .expect("target に BalloonFollow")
            .offset;
        let target_anchored_before = world
            .get::<Anchored>(target)
            .copied()
            .expect("target に Anchored（spawn が付与）");
        let base_anchored_before = world
            .get::<Anchored>(base)
            .copied()
            .expect("base に Anchored（spawn が付与）");

        // 末端 pipeline を一続きに駆動: キャリア cue→MoveCueSink::emit→channel→drain→apply。
        let directive = directive_via_sink("1", &["-353", "", "", "0", "base", "base"]);
        assert_eq!(
            directive.scope, 1,
            "sink が cue.actor（\\1）から scope=1 を導出する（pipeline 前段）"
        );
        assert!(
            apply_move_directive(&mut world, &directive),
            "対象・基準窓が揃うので pipeline 越しの適用は成功する"
        );

        // ① fixture 検算座標（x'=1000+200−353−150=697・y=Fix は現状維持 800）。
        assert_eq!(
            pos_of(&world, target),
            Point { x: 697, y: 800 },
            "sink→channel→apply の一続きで対象窓が fixture 座標へ移動する"
        );

        // ② バルーン随伴 offset 維持（R5.3）——pipeline 越しに balloon_pos − char_pos ≡ offset。
        let cpos = pos_of(&world, target);
        let bpos = pos_of(&world, balloon);
        assert_eq!(bpos.x - cpos.x, offset.x, "offset x が pipeline 越しに維持される");
        assert_eq!(bpos.y - cpos.y, offset.y, "offset y が pipeline 越しに維持される");

        // ③ Anchored ビット同一（R6.1/6.2/9.5）——sink 経由でも永続確定系へ触れない。
        assert_eq!(
            world.get::<Anchored>(target).copied(),
            Some(target_anchored_before),
            "対象窓の Anchored はビット同一（pipeline 越しに永続確定系へ触れない）"
        );
        assert_eq!(
            world.get::<Anchored>(base).copied(),
            Some(base_anchored_before),
            "基準窓の Anchored もビット同一"
        );
    }

    /// R5.5: 対象 scope 不在の move キャリアを **sink→channel→apply** に通しても、apply は
    /// warn＋`false` を返し World は不変（Anchored 含む）——非 panic・talk を殺さない縮退が
    /// pipeline 越しに成立する。sink 段は正常に scope=1 の directive を送出し（名前選別・parse は
    /// 通る）、対象不在の縮退は apply 段でのみ起きる分離を固定する。
    #[test]
    fn pipeline_target_absent_warns_false_no_mutation() {
        // GhostWindows に scope0（基準）のみ挿入・fixture の対象 scope1 は不在。
        let placements = vec![placement(0, 1000, 500, 400, 687, PointPx { x: 285, y: -19 })];
        let mut world = World::new();
        let gw = spawn_ghost_windows(
            &mut world,
            &placements,
            &GhostTitles::from_scope_titles([(0, "a".to_string())]),
        );
        attach_fake_handles(&mut world, &gw);
        let base = gw.char_window(0).unwrap();
        let before = pos_of(&world, base);
        let base_anchored_before = world
            .get::<Anchored>(base)
            .copied()
            .expect("base に Anchored");

        // sink 段は正常送出（actor=1・parse は通る）。対象不在の縮退は apply 段でのみ起きる。
        let directive = directive_via_sink("1", &["-353", "", "", "0", "base", "base"]);
        assert_eq!(directive.scope, 1, "sink 段は対象不在を知らず directive を送出する");

        assert!(
            !apply_move_directive(&mut world, &directive),
            "対象 scope1 不在は pipeline 越しでも warn＋false（R5.5）"
        );
        assert_eq!(
            pos_of(&world, base),
            before,
            "適用不成立で基準窓の位置は不変（no mutation）"
        );
        assert_eq!(
            world.get::<Anchored>(base).copied(),
            Some(base_anchored_before),
            "適用不成立で Anchored も不変（永続確定系へ触れない・R6/9.5）"
        );
    }
}
