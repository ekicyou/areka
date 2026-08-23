//! 純粋層: 名前付きアクタースレッドの spawn／`ActorHandle`／受信ループヘルパ。
//!
//! `std`（[`std::sync::mpsc`]＋[`std::thread`]）のみを用い、wintf/executor 非依存で
//! 実装される。提供物は次の 5 つ:
//!
//! - [`spawn_actor`] — アクターを名前付きスレッドとして起動し、inbox の送信端と
//!   [`ActorHandle`] を返す原語。
//! - [`run_inbox`] — worker 側の正準受信ループヘルパ（使用は任意）。
//! - [`ActorHandle`] — 非 RAII（detach）な join ハンドル。
//! - [`ActorError`] — join 時に panic を観測可能な失敗として写像するエラー型。
//! - [`install_thread_start_hook`] — 生成されたスレッドの走り始めに 1 度呼ばれる
//!   [`ThreadStartHook`] の呼ばれ口（何を行うかは統合側が決める・依存の向きは不変）。

use std::any::Any;
use std::ops::ControlFlow;
use std::sync::OnceLock;
use std::sync::mpsc::{self, Receiver, RecvError, Sender};
use std::thread::{self, JoinHandle};

/// アクタースレッドの走り始めに 1 度だけ呼ばれるフック。引数はアクター名（＝スレッド名）。
///
/// # なぜフックなのか（依存の向き）
///
/// スレッド 1 本ごとの CPU を役割名で読むために、生成されたスレッドは自分を wintf の
/// スレッド名簿へ登録する必要がある（areka-P0-draw-load-parity 要件 2.3）。ところが本
/// クレートも `areka-ghost` も **wintf に依存していない**し、依存を足すこともできない
/// （`Cargo.toml` 非接触＝同 spec 要件 8.6）。そこで本クレートは登録の**呼ばれ口**だけを
/// 持ち、実際に何を登録するかは wintf と本クレートの双方を知る統合側（実行体 `areka` の
/// `thread_roles`）が 1 箇所で宣言する。本クレートは相手を知らないままで済む。
pub type ThreadStartHook = fn(&str);

/// フックが既に導入済みだったことを表すエラー（最初の導入だけが効く）。
#[derive(Debug, thiserror::Error)]
#[error("thread start hook is already installed")]
pub struct ThreadStartHookAlreadyInstalled;

/// プロセス共有のスレッド開始フック。未導入なら [`spawn_actor`] は何もしない
/// （費用は `OnceLock::get` 1 回だけ）。
static THREAD_START_HOOK: OnceLock<ThreadStartHook> = OnceLock::new();

/// スレッド開始フックを導入する（プロセスに 1 度きり・最初の導入が勝つ）。
///
/// 2 度目以降は導入されず [`ThreadStartHookAlreadyInstalled`] を返す。黙って上書きも
/// panic もしない——導入の取り合いは記録に残す（`warn!`）だけで、アクターの挙動は
/// 一切変わらない。
///
/// 導入前に起動済みのスレッドは対象外である（フックは spawn の時点で読まれる）。
/// よって導入は起動のできるだけ早い段階で 1 度行うこと。
pub fn install_thread_start_hook(
    hook: ThreadStartHook,
) -> Result<(), ThreadStartHookAlreadyInstalled> {
    match THREAD_START_HOOK.set(hook) {
        Ok(()) => Ok(()),
        Err(_) => {
            tracing::warn!("[actor] thread start hook is already installed; keeping the first one");
            Err(ThreadStartHookAlreadyInstalled)
        }
    }
}

/// アクターを名前付きスレッドとして起動し、inbox の送信端と join ハンドルを返す。
///
/// inbox は本関数内部で [`mpsc::channel`] を生成し、`Receiver` を `body` へ move する
/// （body が単独所有＝inbox 1 本の構造保証・Req 1.5）。`Sender` を呼び出し側へ返す
/// （Req 1.1）。スレッドは [`std::thread::Builder::new`]`.name(name)` で名前付き起動され
/// （Req 1.2）、body は [`tracing::info_span!`]`("actor", actor = %name)` の中で実行される
/// ため、スレッド名＝アクター名が span に載る（Req 6.1）。
///
/// 追加の常駐機構（レジストリ・監督・stop_flag）は持たない素の thread spawn である
/// （Req 1.4）。停止はメッセージ Close／全 Sender drop の 2 経路が原語（Req 3.2/3.5）。
///
/// [`install_thread_start_hook`] でフックが導入されている場合のみ、生成されたスレッドの中
/// で `body` より前に 1 度フックを呼ぶ（引数はアクター名）。未導入なら何もしない。
///
/// # 運用規約（デッドロック注意）
///
/// `join` は body の終了後に呼ぶ。呼び出し側が `Sender` を握ったまま Close も送らずに
/// [`ActorHandle::join`] すると body は永久に受信待ちとなりデッドロックし得る。停止駆動
/// （Close 送信／全 Sender drop）を先に行うこと。
///
/// # Panics
///
/// [`std::thread::Builder::spawn`] が OS リソース枯渇等で失敗した場合、`tracing::error!`
/// でアクター名と原因を記録したうえで panic する（スレッド起動不能はプロセス継続不能級・
/// 既存 wintf 資産の `expect` 慣行に整合）。
pub fn spawn_actor<M, F>(name: &str, body: F) -> (Sender<M>, ActorHandle)
where
    M: Send + 'static,
    F: FnOnce(Receiver<M>) + Send + 'static,
{
    let (tx, rx) = mpsc::channel::<M>();
    let name: Box<str> = name.into();
    let span_name: String = name.to_string();

    let spawn_result = thread::Builder::new()
        .name(name.to_string())
        .spawn(move || {
            let span = tracing::info_span!("actor", actor = %span_name);
            let _enter = span.enter();
            // 走り始めに 1 度だけフックを呼ぶ（body より前・生成された当のスレッドの中）。
            // 未導入なら何も起きない。
            if let Some(hook) = THREAD_START_HOOK.get() {
                hook(&span_name);
            }
            body(rx);
        });

    let handle = match spawn_result {
        Ok(handle) => handle,
        Err(e) => {
            tracing::error!(actor = %name, cause = %e, "failed to spawn actor thread");
            panic!("failed to spawn actor thread '{name}': {e}");
        }
    };

    (tx, ActorHandle { name, handle })
}

/// 正準受信ループ。`handler` の戻りで挙動を固定する（Req 3.7・受信ループ耐障害規約）:
///
/// - `Ok(ControlFlow::Continue(()))` → 次の recv へ。
/// - `Ok(ControlFlow::Break(()))` → 即時 `return`（Close 受領時・積み残しは `rx` の drop で
///   破棄される）。
/// - `Err(e)` → [`tracing::error!`]（`%e`）を記録して**ループ継続**（個別メッセージ処理の
///   失敗はループを殺さない）。
///
/// 全 `Sender` drop（[`RecvError`]／Disconnected）でも `return`（正常終了）。
///
/// ＝ループの終了経路は `Ok(Break)` と Disconnected の 2 経路のみ（型と実装で強制される）。
///
/// 周期 tick 等で自前ループ（`recv_timeout` 利用）を書く場合も、本規約（Req 3.7）に従うこと。
///
/// 失敗しない `handler` は `E = `[`std::convert::Infallible`] を使う。
pub fn run_inbox<M, E>(rx: Receiver<M>, mut handler: impl FnMut(M) -> Result<ControlFlow<()>, E>)
where
    E: std::fmt::Display,
{
    loop {
        match rx.recv() {
            Ok(msg) => match handler(msg) {
                Ok(ControlFlow::Continue(())) => continue,
                Ok(ControlFlow::Break(())) => return,
                Err(e) => {
                    tracing::error!(error = %e, "actor message handler returned error; continuing");
                    continue;
                }
            },
            Err(RecvError) => return,
        }
    }
}

/// アクタースレッドの join ハンドル。
///
/// 非 RAII である: `drop` しても join しない（detach）。body の終了を待って回収したい場合は
/// [`join`](ActorHandle::join) を明示的に呼ぶ。
#[derive(Debug)]
pub struct ActorHandle {
    name: Box<str>,
    handle: JoinHandle<()>,
}

impl ActorHandle {
    /// アクター名（＝スレッド名）を返す。
    pub fn name(&self) -> &str {
        &self.name
    }

    /// body の終了を待つ。正常終了なら `Ok(())`。body が panic していた場合は panic を握り
    /// 潰さず [`ActorError::Panicked`]（アクター名＋payload 文字列）へ写像して返す（Req 1.3）。
    ///
    /// panic を観測した際は返す前に [`tracing::warn!`] でアクター名と payload を記録する
    /// （失敗経路のログ規律・黙って握り潰さない）。
    pub fn join(self) -> Result<(), ActorError> {
        match self.handle.join() {
            Ok(()) => Ok(()),
            Err(payload) => {
                let message = panic_message(&payload);
                tracing::warn!(actor = %self.name, panic = %message, "actor thread panicked");
                Err(ActorError::Panicked {
                    actor: self.name.into_string(),
                    message,
                })
            }
        }
    }

    /// アクタースレッドが既に終了しているかを返す（join せず状態のみ観測）。
    pub fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }
}

/// panic payload（`Box<dyn Any>`）から表示用文字列を取り出す。`&str`／`String` を優先し、
/// いずれでもない場合は総称メッセージへフォールバックする。
fn panic_message(payload: &Box<dyn Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_owned()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".to_owned()
    }
}

/// アクター関連のエラー。
#[derive(Debug, thiserror::Error)]
pub enum ActorError {
    /// アクタースレッドが panic した（join 時に観測）。
    #[error("actor '{actor}' panicked: {message}")]
    Panicked {
        /// panic したアクターの名前。
        actor: String,
        /// panic payload の表示文字列。
        message: String,
    },
}

#[cfg(test)]
#[path = "spawn_hook_tests.rs"]
mod hook_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::Infallible;
    use std::sync::mpsc::sync_channel;
    use std::time::Duration;

    /// テスト用の有界待機ヘルパ: 別スレッドで `f` を走らせ、期限内に完了しなければ
    /// テストを失敗させる（どのテストもハングしないことを保証する）。
    fn run_bounded<F: FnOnce() + Send + 'static>(what: &str, timeout: Duration, f: F) {
        let (done_tx, done_rx) = sync_channel::<()>(0);
        thread::spawn(move || {
            f();
            let _ = done_tx.send(());
        });
        assert!(
            done_rx.recv_timeout(timeout).is_ok(),
            "'{what}' did not complete within {timeout:?} (possible hang)"
        );
    }

    // ケース1: スレッド名＝アクター名。
    #[test]
    fn thread_name_equals_actor_name() {
        let (name_tx, name_rx) = mpsc::channel::<Option<String>>();
        let (_tx, handle) = spawn_actor::<(), _>("kanade-test", move |_rx| {
            let observed = thread::current().name().map(|s| s.to_owned());
            name_tx.send(observed).expect("send observed thread name");
        });

        let observed = name_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("actor should report its thread name");
        assert_eq!(observed.as_deref(), Some("kanade-test"));
        assert_eq!(handle.name(), "kanade-test");
        handle.join().expect("body completes normally");
    }

    // ケース2: panic 時の join が Err(Panicked)。
    #[test]
    fn join_returns_err_on_panic() {
        let (_tx, handle) = spawn_actor::<(), _>("panicky", move |_rx| {
            panic!("boom in actor");
        });

        let err = handle
            .join()
            .expect_err("panicking body must surface as Err");
        match err {
            ActorError::Panicked { actor, message } => {
                assert_eq!(actor, "panicky");
                assert!(
                    message.contains("boom in actor"),
                    "panic message should carry payload, got: {message}"
                );
            }
        }
    }

    // ケース3: Break で即時終了。
    #[test]
    fn run_inbox_breaks_immediately_on_break() {
        let (tx, rx) = mpsc::channel::<&'static str>();
        // Close 相当メッセージ＋その後ろに積み残しを 2 件。
        tx.send("close").expect("send close");
        tx.send("backlog-1").expect("send backlog-1");
        tx.send("backlog-2").expect("send backlog-2");

        let (seen_tx, seen_rx) = mpsc::channel::<&'static str>();
        run_bounded("run_inbox break", Duration::from_secs(5), move || {
            run_inbox::<&'static str, Infallible>(rx, move |msg| {
                if msg == "close" {
                    Ok(ControlFlow::Break(()))
                } else {
                    seen_tx.send(msg).expect("record seen");
                    Ok(ControlFlow::Continue(()))
                }
            });
        });

        // Break 即時終了ゆえ積み残しは処理されない。
        assert!(
            seen_rx.try_recv().is_err(),
            "backlog after Break must NOT be processed"
        );
    }

    // ケース4: 切断（全 Sender drop）で正常終了。
    #[test]
    fn run_inbox_returns_on_disconnect() {
        let (tx, rx) = mpsc::channel::<u32>();
        drop(tx); // 全 Sender drop → Disconnected。

        run_bounded("run_inbox disconnect", Duration::from_secs(5), move || {
            run_inbox::<u32, Infallible>(rx, |_msg| Ok(ControlFlow::Continue(())));
        });
        // ハングせず復帰すればテスト成功（run_bounded が保証）。
    }

    // ケース5: handler の Err 後も受信継続。
    #[test]
    fn run_inbox_continues_after_handler_err() {
        #[derive(Debug)]
        struct HandlerErr(&'static str);
        impl std::fmt::Display for HandlerErr {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "handler err: {}", self.0)
            }
        }

        let (tx, rx) = mpsc::channel::<&'static str>();
        tx.send("will-error").expect("send will-error");
        tx.send("after-error").expect("send after-error");
        tx.send("close").expect("send close");

        let (seen_tx, seen_rx) = mpsc::channel::<&'static str>();
        run_bounded(
            "run_inbox continue-after-err",
            Duration::from_secs(5),
            move || {
                run_inbox::<&'static str, HandlerErr>(rx, move |msg| match msg {
                    "will-error" => Err(HandlerErr("intentional")),
                    "close" => Ok(ControlFlow::Break(())),
                    other => {
                        seen_tx.send(other).expect("record seen");
                        Ok(ControlFlow::Continue(()))
                    }
                });
            },
        );

        // Err を返したメッセージの後続が処理されていること＝ループは殺されていない。
        let seen = seen_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("message after handler Err must still be processed");
        assert_eq!(seen, "after-error");
    }
}
