//! `\![move]`（キャラクタ移動）の型と受け口（design.md「PositionSink」「MoveArgs」）。
//!
//! 本ファイルは指令の型（[`MoveDirective`]・[`AxisSpec`]・[`MoveBase`]・[`RefPoint`]・線を流れる
//! [`PositionCue`]）と、talk スレッドの受け口 [`MoveCueSink`] を持つ。引数の読みは兄弟の `move_cue_args.rs`
//! （子モジュール `args`）にあり、[`parse_move_directive`] として再輸出する。移動先の計算と
//! 基準位置の解き手（`BaseposResolver`／`CanonDefaultBasepos`）は兄弟の `move_cue_geometry.rs`
//! （子モジュール `geometry`）に、UI スレッドでの適用（`apply_move_directive`）は兄弟の
//! `move_cue_place.rs`（子モジュール `place`）にある。
//!
//! 名前付きの形と旧い位置の形は同じ [`MoveDirective`] へ読まれ、読めない値・知らない語は
//! SSP 2.9.18 の実測と同じ値へ置き換えて `MoveNote` に記録される（受け口が
//! `move_arg_substituted` の `warn!` で出す）。裸 `base`（ドット無し）≡ `base.base`。

use std::sync::mpsc::Sender;

use dola::cue::{CueCommand, TalkCue};
use tracing::{debug, warn};
use wintf::ecs::world::tick_wake;

use crate::placement::script_alignment::{AlignmentWord, parse_alignment_word};

#[path = "move_cue_args.rs"]
mod args;
pub use args::parse_move_directive;

#[path = "move_cue_geometry.rs"]
mod geometry;
pub use geometry::{BaseposResolver, CanonDefaultBasepos};

#[path = "move_cue_place.rs"]
mod place;
pub use place::apply_move_directive;

/// 台本から UI スレッドへ運ぶ窓の位置の指令（design.md「PositionSink」の Event Contract）。
///
/// [`MoveCueSink`] が送り、frame 相の move drain が種類ごとに適用する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PositionCue {
    /// `\![move]`——1 つの窓を動かす（`apply_move_directive`）。
    Move(MoveDirective),
    /// `\![execute,resetwindowpos]`——すべての窓を既定の位置へ並べ直す
    /// （`placement::reset_pos::reset_window_positions`・`OnResetWindowPos` は送らない＝要件 8.7）。
    ResetWindowPos,
    /// `\![set,alignmenttodesktop,方向]`／`\![set,alignmentondesktop,方向]`——実行者のスコープの
    /// キャラクターの窓の吸着の向きを変える（`placement::script_alignment`・要件 1）。
    SetAlignment { scope: u32, word: AlignmentWord },
    /// `\![set,sticky-window,…]`——ID の並びを読まずに運ぶ（読みと拒否は帳簿が持つ・要件 3.1）。
    SetSticky { tokens: Vec<String> },
    /// `\![reset,sticky-window]`——台本で作った組を解く（要件 3.5）。
    ResetSticky,
    /// `\![execute,resetballoonpos]`——すべてのバルーンを標準の位置へ戻す（要件 7.1）。
    ResetBalloonPos,
    /// `\![lock,balloonmove]`（`locked: true`）／`\![unlock,balloonmove]`（`false`）——実行者のスコープの
    /// バルーンを利用者のドラッグで動かさない指定を当てる・解く（`placement::balloon_lock`・要件 5.1・5.2）。
    BalloonMoveLock { scope: u32, locked: bool },
}

/// `\![move]` の 1 件の意味（design.md「MoveArgs」）。
///
/// 両方の形から [`parse_move_directive`] が読み、置き換え済みの値だけを持つ（どの欄も正典の語彙の中）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveDirective {
    /// 移動対象スコープ（cue.actor 由来・`\0`=0／`\1`=1）。
    pub scope: u32,
    /// X 軸指定（省略/"fix"＝現状維持・数値＝物理 px）。
    pub x: AxisSpec,
    /// Y 軸指定（同上）。
    pub y: AxisSpec,
    /// 移動時間 ms（省略=0）。`>0` は補間せず即時に置く（読みが記録する・要件 5.1）。
    pub duration_ms: u32,
    /// 基準（解き方は適用の側 `move_cue_place.rs`）。
    pub base: MoveBase,
    /// 基準側の参照点（省略=left.top）。
    pub base_offset: RefPoint,
    /// 自窓側の参照点（省略=left.top）。
    pub move_offset: RefPoint,
    /// `--option=ignore-sticky-window`。真なら組の相手を連れ動かさない（`move_cue_place.rs`）。
    pub ignore_sticky_window: bool,
}

/// 軸指定（X/Y 共通）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisSpec {
    /// 省略または `"fix"`＝現状維持。
    Fix,
    /// **作者基準 px**（台本に書かれた値そのまま・転記層は解釈しない）。
    ///
    /// 物理 px への換算（k 倍）は移動先の計算（`move_cue_geometry.rs` の `compute_move_target`）の責務である。
    /// `windowposition.x/y` と同種の「作者基準で書かれた画面オフセット」であり、同じ
    /// 写像（`placement::scale_signed`）を通す。差分/座標の解釈は基準解決に委ねる（R-6）。
    Px(i32),
}

/// 移動基準（design.md `MoveBase`・要件 3.1〜3.7）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveBase {
    /// 数値スコープの窓（無ければ `screen` へ倒す）。
    Scope(u32),
    /// 動かす窓の中心のモニタの作業領域（省略時既定・外れたら最近傍）。
    Screen,
    /// 主モニタの作業領域。
    PrimaryScreen,
    /// 動かす前の自分の窓。
    Me,
    /// システムの座標の原点（主モニタの左上）。
    Global,
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
    /// 裸 `base` の展開先（`base.base`・R5.2 の等価則）。テストが期待値に使う。
    #[cfg(test)]
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

/// 軸の識別（読みの記録・基準の置き換えの記録が使う）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
}

// =============================================================================
// MoveCueSink（talk スレッド純粋解釈）— `\![move]` 名前選別消費の最初の実消費者
// （task 7.3・R4.5・R8.5）
// =============================================================================

/// 窓の位置の指令（`\![move]` と `\![execute,resetwindowpos]`）の talk スレッド側消費 sink
/// （design「PositionSink」・R4.5）。
///
/// broadcast された全 cue のうち、キャリア正準形（`Custom` の String Array）であり自らの
/// コマンド名リテラル `"move"`、または `"execute"` で第 1 引数が `resetwindowpos` を運ぶもの**だけ**を
/// 名前自己選別して解釈し、[`PositionCue`]（`move` は [`parse_move_directive`] の結果を包んだ
/// `Move`・`execute,resetwindowpos` は残りの引数を読まない `ResetWindowPos`）を mpsc で UI スレッド（frame 相の
/// `apply_move_directive`＝task 7.4／channel 配線＝task 9.1）へ送出する。それ以外
/// （非キャリア・担当外コマンド名・非数値 scope）は**記録付き良性スキップ**へ
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
/// が成り立つ（`PositionCue: Send` により `Sender<PositionCue>: Send`）。
#[derive(Clone)]
pub struct MoveCueSink {
    /// UI スレッド（frame 相 drain）への送出端（Clone 可・全 clone は単一受信端へ配送）。
    tx: Sender<PositionCue>,
}

impl MoveCueSink {
    /// mpsc 送信端（`Emo2Wiring` の `Receiver` と対・task 9.1 が生成）から sink を構築する。
    pub fn new(tx: Sender<PositionCue>) -> Self {
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
        //    "execute" 宛も debug——第 1 引数が読めず resetwindowpos か分からないうえ、execute 宛の
        //    壊れ物の warn は InstallCueSink が持つ（同じ荷物に warn を 2 行出さない）。
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

        // 2) 名前自己選別（消費者自身のコマンド名リテラル）。"move" と (execute,resetwindowpos) のみ。
        //    dola は名前語彙を持たず、一意性（1 名前=高々 1 消費者）は areka 消費者台帳（task 2.5）
        //    が保証する——本 sink は中央権威表に依存せず自らの名前で自己選別する（R4.5）。
        //    並べ直しは残りの引数を読まず、scope も使わない（全スコープが対象・要件 8.7）。
        //    窓の組・吸着・バルーン位置初期化の 5 組もここで選ぶ（1 本の線で台本の並びを保つ・設計 議題 8）。
        //    組と初期化は実行者を読まない。吸着だけは実行者のスコープへ当てる（`move` と同じ読み）。
        match (name, tokens.first().copied()) {
            ("execute", Some("resetwindowpos")) => {
                self.send(PositionCue::ResetWindowPos);
                return;
            }
            ("execute", Some("resetballoonpos")) => {
                self.send(PositionCue::ResetBalloonPos);
                return;
            }
            ("set", Some("sticky-window")) => {
                let tokens = tokens[1..].iter().map(|s| s.to_string()).collect();
                self.send(PositionCue::SetSticky { tokens });
                return;
            }
            ("reset", Some("sticky-window")) => {
                self.send(PositionCue::ResetSticky);
                return;
            }
            ("set", Some(selector @ ("alignmenttodesktop" | "alignmentondesktop"))) => {
                if let Some(scope) = actor_scope(&cue, &format!("set,{selector}")) {
                    let word = parse_alignment_word(&tokens[1..]);
                    self.send(PositionCue::SetAlignment { scope, word });
                }
                return;
            }
            // ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5block_2cballoonmove_5d:1
            // ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bunlock_2cballoonmove_5d:1
            // 動かせない指定は実行者のスコープへ当てる。余分な引数は読まない（SSP 2.9 も誤りにしない）。
            (cmd @ ("lock" | "unlock"), Some("balloonmove")) => {
                if let Some(scope) = actor_scope(&cue, &format!("{cmd},balloonmove")) {
                    self.send(PositionCue::BalloonMoveLock {
                        scope,
                        locked: cmd == "lock",
                    });
                }
                return;
            }
            _ => {}
        }
        if name != "move" {
            debug!(
                name,
                "MoveCueSink: 担当外コマンド名を良性スキップ（名前自己選別・R4.5/R8.5）"
            );
            return;
        }

        // 3) scope は cue.actor（"0"/"1"）の u32 parse。非数値は warn＋スキップ（design「破損・異常」）。
        let Some(scope) = actor_scope(&cue, "move") else {
            return;
        };

        // 4) 純粋な読み（決定論・no I/O）。常に指令を返し、置き換えは 1 件ずつ記録する（要件 4.7・5.1）。
        let tokens: Vec<String> = tokens.iter().map(|s| s.to_string()).collect();
        let parsed = parse_move_directive(scope, &tokens);
        for note in &parsed.notes {
            warn!(
                event = "move_arg_substituted",
                scope,
                ?note,
                "MoveCueSink: \\![move] の引数を置き換えて受理"
            );
        }
        self.send(PositionCue::Move(parsed.directive));
    }
}

/// 実行者（`cue.actor` の `"0"`/`"1"`）をスコープの数として読む。数でなければ `warn!` で捨てる
/// （design「破損・異常」・R5.5）。`tag` は記録に出す指令の名前。
fn actor_scope(cue: &TalkCue, tag: &str) -> Option<u32> {
    let scope = cue.actor.as_str().parse::<u32>().ok();
    if scope.is_none() {
        warn!(
            actor = cue.actor.as_str(),
            tag, "MoveCueSink: cue.actor が非数値 scope のため指令をスキップ（R5.5）"
        );
    }
    scope
}

impl MoveCueSink {
    /// 線へ 1 件送り、次の画面更新を起こす。envelope の `duration` には触れない（要件 5.2）。
    fn send(&self, cue: PositionCue) {
        if self.tx.send(cue).is_err() {
            // 受信端（Emo2Wiring）切断は talk を殺さない（log-first・非 panic・R5.5）。
            warn!("MoveCueSink: PositionCue の送出に失敗（受信端切断）");
        }
        // 窓を動かす指令が UI へ届いた＝次の画面更新に仕事がある（設計 C16 の
        // `PRESENT`）。送出の後に立てる理由は `PresentBridge::send` と同じ。
        tick_wake::mark(tick_wake::PRESENT);
    }
}

#[cfg(test)]
#[path = "move_cue_move_sink_tests.rs"]
mod move_sink_tests;

#[cfg(test)]
#[path = "move_cue_apply_move_tests.rs"]
mod apply_move_tests;

#[cfg(test)]
#[path = "move_cue_apply_move_sticky_tests.rs"]
mod apply_move_sticky_tests;

#[cfg(test)]
#[path = "move_cue_move_severity_log_tests.rs"]
mod move_severity_log_tests;

#[cfg(test)]
#[path = "move_cue_balloon_lock_tests.rs"]
mod balloon_lock_tests;
