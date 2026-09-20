//! `\![enter,nouserbreakmode]`／`\![leave,nouserbreakmode]` の受け口（areka-P0-balloon-break）。
//!
//! 中断を禁じる区間の出入りとトークの始まりを、1 本の線で UI の側へ流す受け口がここへ入る。
//!
//! # 何を運ぶか（design「NoUserBreakCueSink」・要件 5.1／5.2／5.4）
//!
//! 台本の指示は演者ごとに振り分けられずに全員へ配られるので、本受け口にも文字も演技も
//! 他人宛のコマンドも届く。ここで行うのは次の 2 つに限る。
//!
//! 1. **自己選別**——コマンド名と第 1 引数の組が `("enter", "nouserbreakmode")`／
//!    `("leave", "nouserbreakmode")` のものだけを受理し、それ以外（`\![enter,onlinemode]`
//!    など別の第 1 引数・他の名前・運搬でない cue）は担当外として読み飛ばす（要件 8.2）。
//! 2. **トークの境界の自己検出**——複製で状態を初期化し、複製後の最初の指示（どの cue でも
//!    よい）で [`NoUserBreakSignal::TalkStarted`] を 1 回だけ流す。
//!
//! # なぜトークの始まりも同じ線に流すのか（design「旗の順序」・要件 5.4）
//!
//! 正典の記述例は `\![enter,nouserbreakmode]` を台本の先頭に置く。先頭のタグは台本の最初の
//! 指示と同じ時刻に配られるので、旗を「表示の合図の線で届くトークの始まりで解く」形にすると、
//! 旗の線（入力の段で取り出す）と表示の合図の線（更新の段で取り出す）の取り出し順の違いから、
//! 同じ巡に届いた 2 つが逆順で処理され、入ったばかりの区間が解かれてしまう。そこで本受け口が
//! 自分でトークの境界を検出し、3 つの合図を**同じ 1 本の線**へ順に流す。
//!
//! 旗そのもの（入れ子の扱い・区間外の「出る」の判定）は持たない。旗は UI の側だけが持ち、
//! 本受け口は運ぶだけである（裁定 8）。cue の占有時間にも触れない（観測するだけ）。
//!
//! 骨格は `readme_cue.rs` の受け口（名前＋第 1 引数の自己選別）と `talk_lifecycle.rs` の
//! `BalloonLifecycleSink`（複製によるトークの境界の自己検出）をそのまま組み合わせたものである。
//!
//! # 黙って諦めない（要件 6）
//!
//! 読み飛ばし・送出の失敗のいずれの経路も、必ず理由を記録してから抜ける。受け口が閉じていても
//! 台本は殺さない（記録して継続する）。

use std::sync::mpsc::Sender;

use dola::cue::TalkCue;
use tracing::{debug, warn};

/// 区間へ入るコマンド名（`\![enter,…]`）。第 1 引数まで見ないと担当は決まらない。
const NAME_ENTER: &str = "enter";
/// 区間から出るコマンド名（`\![leave,…]`）。同上。
const NAME_LEAVE: &str = "leave";
/// 第 1 引数（選別子）——本受け口が担当するのはこの選別子を伴う 2 組だけである。
const SELECTOR_NO_USER_BREAK: &str = "nouserbreakmode";

/// talk スレッドから UI スレッドへ流れる、中断の無効化の合図（design「NoUserBreakCueSink」）。
///
/// 3 値は同じ 1 本の線を流れ、1 つのトークの中では台本の順に並ぶ。トークをまたぐ順序は
/// 配送が「古いトークのスレッドの合流 → 新しいトークの起動」を守ることで保たれる。
///
/// 受け取った側（UI）が旗を畳む——入れ子は数えず、[`TalkStarted`](Self::TalkStarted) で解く。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NoUserBreakSignal {
    /// このトークの最初の指示が配られた（複製後の最初の `emit` で 1 回だけ）。
    ///
    /// 受け取った側はこれを「閉じ忘れた区間を解く」契機として使う（要件 5.4）。
    TalkStarted,
    /// `\![enter,nouserbreakmode]`——中断の無効化の区間に入る（要件 5.1）。
    Enter,
    /// `\![leave,nouserbreakmode]`——区間から出る（要件 5.2）。
    Leave,
}

/// 中断の無効化の合図を UI へ届ける受け口（`GhostBootOptions.sinks` の 7 本目）。
pub(crate) struct NoUserBreakCueSink {
    /// UI スレッド（入力の段の取り出し）への送出端。全ての複製は単一の受信端へ配送する。
    tx: Sender<NoUserBreakSignal>,
    /// このトークで [`NoUserBreakSignal::TalkStarted`] を送出済みか（複製ごとに `false` から）。
    started: bool,
}

impl NoUserBreakCueSink {
    /// 送信端（受信端は結線の task が UI の持ち物へ渡す）から受け口を組む。
    pub(crate) fn new(tx: Sender<NoUserBreakSignal>) -> Self {
        Self { tx, started: false }
    }

    /// 1 つの合図を非ブロックで送る。受信端が閉じていても台本は殺さない
    /// （記録して継続・既存の受け口 6 本と同じ規律）。
    fn send(&self, signal: NoUserBreakSignal) {
        if self.tx.send(signal).is_err() {
            warn!(
                ?signal,
                "NoUserBreakCueSink: 旗の合図を送り出せなかった（受信端が閉じている）——台本は続ける"
            );
        }
    }
}

/// **状態を引き継がない複製**——`#[derive(Clone)]` ではなく手書きにしてある。
///
/// 配送はトークの起動ごとに登録済みの受け口を 1 回だけ複製し、その複製が当該トーク専用に
/// なる。複製で `started` を初期状態へ戻すことで「複製＝トークの境界」を型の側で保証する
/// （`BalloonLifecycleSink` と同じ手口）。前のトークの `started` を持ち越すと、次のトークで
/// 始まりの合図が出ず、閉じ忘れた区間が解けないまま居残る。
impl Clone for NoUserBreakCueSink {
    fn clone(&self) -> Self {
        Self::new(self.tx.clone())
    }
}

/// 演者非依存の単一出力契約を実装する（配送への登録が要求する形）。
///
/// 全ての cue が届くので、担当外は記録付きの良性な読み飛ばしへ落とす。cue の占有時間には
/// 一切触れない（観測するだけで、待ちの契約に影響を与えない）。
impl dola::cue::CueSink for NoUserBreakCueSink {
    fn emit(&mut self, cue: TalkCue) {
        // ⑴ トークの始まり: 複製後の最初の emit で 1 回だけ。送出の成否に依らず `started` を
        //    立てる——「トークにつき 1 回」は本受け口の契約であり、切断時に毎 cue 再送するのは
        //    無意味な繰り返しになる（失敗自体は send 内で記録済み）。自己選別より前に置くのは、
        //    担当のコマンドを 1 つも持たない台本でも旗を解く契機が要るためである（要件 5.4）。
        if !self.started {
            self.started = true;
            self.send(NoUserBreakSignal::TalkStarted);
        }

        // ⑵ 開封。運搬でない cue（文字・演技など）と開封できない荷物は担当外である。名前だけで
        //    自分宛とみなせないのは、`enter`／`leave` が第 1 引数で担当の分かれる名前だからで
        //    ある（`\![enter,onlinemode]` は本受け口の担当ではない）。
        let Some((name, params)) = cue.command.as_command_carrier() else {
            debug!(
                command = ?cue.command,
                "NoUserBreakCueSink: 運搬でない cue を良性に読み飛ばす（担当外）"
            );
            return;
        };

        // ⑶ 自己選別。担当は「名前＋第 1 引数」の 2 組だけで、別の第 1 引数（`onlinemode` など）も
        //    第 1 引数の無い裸の `\![enter]` も担当外である（報せる責任はその担当者にある）。
        // ukadoc の正典 URL は担当の定義箇所（`CommandConsumer::UserBreakSink` の doc）に置く。
        let selector = params.first().copied().unwrap_or_default();
        let signal = match (name, selector) {
            (NAME_ENTER, SELECTOR_NO_USER_BREAK) => NoUserBreakSignal::Enter,
            (NAME_LEAVE, SELECTOR_NO_USER_BREAK) => NoUserBreakSignal::Leave,
            _ => {
                debug!(
                    name,
                    selector, "NoUserBreakCueSink: 担当外のコマンドを良性に読み飛ばす（自己選別）"
                );
                return;
            }
        };

        // ⑷ 送り出し。旗の状態は持たない（入れ子・区間外の「出る」の判定は UI が下す）。
        self.send(signal);
    }
}

#[cfg(test)]
#[path = "user_break_cue_tests.rs"]
mod tests;
