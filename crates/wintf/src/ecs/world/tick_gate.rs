//! tick の門——「この画面更新でスケジュールを全部回すか、省略するか」を決める純関数。
//!
//! # 何のためか
//!
//! 画面更新のたびに 13 本のスケジュールを回すと、見た目が 1 画素も変わらない回でも
//! CPU を使う。そこで、回す前に一度だけこの門を通す。門は状態を持たず、時計も読まず、
//! 何も書き換えない——渡された値だけを見て `Run(理由)` か `Skip` を返す。旗を集めるのは
//! 兄弟の [`tick_wake`](super::tick_wake)、両者をつないで数を数えるのは
//! `EcsWorld::decide_tick` で、判断そのものはここに閉じている。
//!
//! # 回す 5 つの理由（この順に上から見る）
//!
//! 上にある理由が 1 つでも当たれば、下は見ない。
//!
//! ⒈ [`RunReason::Disabled`]——**門が無効**。`AREKA_TICK_GATE=0` や既定 OFF のとき。
//!    門を切ったのだから、以後の判断は一切せず必ず回す（＝門を入れる前と同じ挙動）。
//! ⒉ [`RunReason::Warmup`]——**起動直後**。起動から [`TICK_GATE_WARMUP_FRAMES`] 回の
//!    画面更新の間は無条件で回す。立ち上げの途中は「何が変わったか」を旗で言い切れる
//!    段階にないので、判断そのものを持ち込まない。
//! ⒊ [`RunReason::Wake`]——**旗が立っている**。誰かが「変わった」と申告した。どの旗かは
//!    そのまま持ち回るので、ログに理由の中身まで出せる（[`wake_names`]）。
//! ⒋ [`RunReason::Deadline`]——**預けた期限が来た**。「◯ミリ秒後にもう一度見たい」と
//!    頼まれていた時刻を過ぎた。
//! ⒌ [`RunReason::Heartbeat`]——**心拍**。旗も期限も無いまま
//!    [`TICK_HEARTBEAT_FRAMES`] 回ぶん画面更新が過ぎたら、理由が無くても 1 回回す。
//!
//! どれにも当たらないときだけ [`TickDecision::Skip`] になる。言い換えると、省略になるのは
//! **旗ゼロ・期限なし・心拍未満・起動直後でない・門が有効**の 5 つが同時に成り立つとき
//! だけであり、決定論テストはこの必要十分条件を全組合せで突き合わせている。
//!
//! # 疑わしいときは回す
//!
//! 5 つの理由はどれも「回す」側に倒れる。判断の材料が足りない・分からない・怪しいときに
//! 省略へ倒れる経路は 1 本も無い。
//!
//! - 門を切れば回す（⒈）。立ち上げ中は回す（⒉）。素性の分からないウィンドウメッセージは
//!   [`FORCE`](super::tick_wake::FORCE) の旗になって回る（⒊）。旗を立て忘れても心拍が拾う（⒌）。
//! - 旗の取得や観測の組立に失敗した場合、呼び出し側は `Run(Disabled)` へ倒す
//!   （黙って省略しない＝要件 3.7）。
//!
//! 非対称なのは費用が非対称だからである。余分に 1 回回す損は画面更新 1 回ぶんの CPU に
//! すぎないが、回すべき回を省略する損は「反応しないアプリ」になって現れる。心拍・
//! 起動直後・未知は全走、の三重の網はこの一点のために置いてある。

use std::fmt::Write as _;

use super::tick_wake::{ALL, WakeSnapshot};

/// 旗も期限も無いまま省略を続けてよい上限（画面更新の回数）。
///
/// 120Hz なら約 4 回/秒は必ず回る計算になる。旗の立て忘れがあっても、反応が止まりきる
/// ことはない——という安全側の網である。
pub const TICK_HEARTBEAT_FRAMES: u32 = 30;

/// 起動から無条件で全走する画面更新の回数。
///
/// 立ち上げ中は「何が変わったか」を旗で言い切れる段階にないため、判断を持ち込まない。
pub const TICK_GATE_WARMUP_FRAMES: u32 = 600;

/// 門が見る入力（これが全部・他には何も見ない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickGateInputs {
    /// この画面更新までに立っていた旗（[`tick_wake`](super::tick_wake) のビット集合）。
    pub bits: u32,
    /// 預けてあった期限が到来していたか。
    pub deadline_due: bool,
    /// 前回**回した** tick からの画面更新の回数（心拍の物差し）。
    pub frames_since_run: u32,
    /// 起動からの画面更新の回数（起動直後かどうかの物差し）。
    pub frames_since_boot: u32,
    /// 門そのものが有効か（無効なら判断せず必ず回す）。
    pub gate_enabled: bool,
}

impl TickGateInputs {
    /// 旗の読み取り結果に、数えている 2 つの回数と門の有効を添えて入力を組む。
    ///
    /// 写すだけで、何も判断しない。
    #[inline]
    pub fn from_snapshot(
        snapshot: &WakeSnapshot,
        frames_since_run: u32,
        frames_since_boot: u32,
        gate_enabled: bool,
    ) -> TickGateInputs {
        TickGateInputs {
            bits: snapshot.bits,
            deadline_due: snapshot.deadline_due,
            frames_since_run,
            frames_since_boot,
            gate_enabled,
        }
    }
}

/// 回す理由（モジュール doc の ⒈〜⒌ と同じ並び＝優先順位が高い順）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunReason {
    /// 門が無効（判断せず必ず回す）。
    Disabled,
    /// 起動直後（[`TICK_GATE_WARMUP_FRAMES`] 未満）。
    Warmup,
    /// 旗が立っていた（立っていた旗をそのまま持ち回る）。
    Wake(u32),
    /// 預けた期限が到来した。
    Deadline,
    /// 旗も期限も無いが心拍（[`TICK_HEARTBEAT_FRAMES`] 到達）。
    Heartbeat,
}

impl RunReason {
    /// ログの `reason=` に載せる名前。
    ///
    /// [`Wake`](RunReason::Wake) は旗の中身に関わらず `"wake"` を返す。旗の内訳は
    /// [`wake_names`] で別のフィールドに出す（1 つのフィールドに 2 つの意味を混ぜない）。
    #[inline]
    pub fn as_str(&self) -> &'static str {
        match self {
            RunReason::Disabled => "disabled",
            RunReason::Warmup => "warmup",
            RunReason::Wake(_) => "wake",
            RunReason::Deadline => "deadline",
            RunReason::Heartbeat => "heartbeat",
        }
    }
}

/// 門の答え。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickDecision {
    /// 回す（理由つき）。
    Run(RunReason),
    /// 省略する。
    Skip,
}

impl TickDecision {
    /// 回す側か。
    #[inline]
    pub fn is_run(&self) -> bool {
        matches!(self, TickDecision::Run(_))
    }
}

/// 回すか省略するかを決める純関数。
///
/// 副作用は無く、同じ入力からは必ず同じ答えが出る。優先順位はモジュール doc の ⒈〜⒌ の
/// とおりで、上の理由が当たれば下は見ない。
///
/// 不変条件: `gate_enabled == false` ／ `frames_since_boot < TICK_GATE_WARMUP_FRAMES` ／
/// `bits != 0` ／ `deadline_due` ／ `frames_since_run >= TICK_HEARTBEAT_FRAMES` の
/// いずれか 1 つでも成り立てば、必ず [`TickDecision::Run`] を返す。
#[inline]
pub fn should_run(i: &TickGateInputs) -> TickDecision {
    // ⒈ 門を切ってあるなら、判断そのものをしない。
    if !i.gate_enabled {
        return TickDecision::Run(RunReason::Disabled);
    }

    // ⒉ 起動直後は無条件で全走。
    if i.frames_since_boot < TICK_GATE_WARMUP_FRAMES {
        return TickDecision::Run(RunReason::Warmup);
    }

    // ⒊ 誰かが「変わった」と申告している。
    if i.bits != 0 {
        return TickDecision::Run(RunReason::Wake(i.bits));
    }

    // ⒋ 預けた期限が来た。
    if i.deadline_due {
        return TickDecision::Run(RunReason::Deadline);
    }

    // ⒌ 理由は無いが、間が空きすぎたので 1 回回す。
    if i.frames_since_run >= TICK_HEARTBEAT_FRAMES {
        return TickDecision::Run(RunReason::Heartbeat);
    }

    // ここに来るのは 5 つの理由が 1 つも当たらなかったときだけ。
    TickDecision::Skip
}

/// 旗の内訳を人が読める 1 語にする（ログ用）。
///
/// 並びは [`ALL`](super::tick_wake::ALL) の順＝ビットの若い順で、区切りは `|` である
/// （例: `POINTER|PRESENT`）。旗が 1 本も立っていなければ `NONE`。
///
/// 表に無いビットは黙って捨てず、残りをまとめて 16 進で末尾に付ける（例:
/// `POINTER|0x800`）。捨ててしまうと「旗ゼロなのに回った」と読める行が出てしまい、
/// ログから旗の立て忘れを追えなくなる。
pub fn wake_names(bits: u32) -> String {
    if bits == 0 {
        return "NONE".to_string();
    }

    let mut out = String::new();
    let mut rest = bits;
    for (name, flag) in ALL {
        if bits & flag.bits() != 0 {
            if !out.is_empty() {
                out.push('|');
            }
            out.push_str(name);
            rest &= !flag.bits();
        }
    }

    if rest != 0 {
        if !out.is_empty() {
            out.push('|');
        }
        // 書き込み先が String なので失敗しない（`write!` の Err は起きえない）。
        let _ = write!(out, "{rest:#x}");
    }

    out
}

#[cfg(test)]
#[path = "tick_gate_tests.rs"]
mod tests;
