//! WintfTaskPool - 非同期タスク実行用リソース
//!
//! TaskPoolでバックグラウンド実行し、mpsc channelでコマンドをECSへ送信。
//! Input scheduleの`drain_task_pool_commands`で受け取って適用する。

use bevy_ecs::prelude::*;
use bevy_tasks::{TaskPool, TaskPoolBuilder};
use std::future::Future;
use std::sync::Mutex;
use std::sync::mpsc;

/// Box化されたECSコマンド型（クロージャベース）
pub type BoxedCommand = Box<dyn FnOnce(&mut World) + Send>;

/// コマンド送信用チャネル型
pub type CommandSender = mpsc::Sender<BoxedCommand>;

/// 非同期タスク実行用リソース
///
/// # Design
/// - TaskPoolでバックグラウンド実行
/// - mpsc channelで`BoxedCommand`（`Box<dyn FnOnce(&mut World) + Send>`）をECSへ送信
/// - Input scheduleの`drain_task_pool_commands`で適用
///
/// # Example
/// ```ignore
/// task_pool.spawn(|tx| async move {
///     let result = some_async_work().await;
///     let cmd: BoxedCommand = Box::new(move |world: &mut World| {
///         world.insert_resource(MyResult(result));
///     });
///     let _ = tx.send(cmd);
/// });
/// ```
#[derive(Resource)]
pub struct WintfTaskPool {
    pool: TaskPool,
    sender: mpsc::Sender<BoxedCommand>,
    // MutexでラップしてSyncを満たす
    receiver: Mutex<mpsc::Receiver<BoxedCommand>>,
}

impl WintfTaskPool {
    /// 新しいWintfTaskPoolを作成
    pub fn new() -> Self {
        Self::from_pool(TaskPool::new())
    }

    /// スレッドの数を指定してWintfTaskPoolを作成
    ///
    /// `new()` は論理CPUの数だけスレッドを作る。閉包を走らせないテストの足場など、
    /// スレッドを多く持つ必要が無い呼び手が数を絞るための作り口。
    pub fn with_threads(n: usize) -> Self {
        Self::from_pool(TaskPoolBuilder::new().num_threads(n).build())
    }

    fn from_pool(pool: TaskPool) -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            pool,
            sender,
            receiver: Mutex::new(receiver),
        }
    }

    /// 非同期タスクを生成（CommandSenderが自動で渡される）
    pub fn spawn<F, Fut>(&self, f: F)
    where
        F: FnOnce(CommandSender) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send,
    {
        let tx = self.sender.clone();
        self.pool
            .spawn(async move {
                f(tx).await;
            })
            .detach();
    }

    /// 受信したコマンドをすべて収集して返す（Worldに適用しない）
    ///
    /// `drain_task_pool_commands`で使用。
    /// コマンド実行中もWintfTaskPoolがWorld内に存在する必要があるため、
    /// コマンドを収集してから別途実行する。
    pub fn drain_commands(&self) -> Vec<BoxedCommand> {
        if let Ok(receiver) = self.receiver.lock() {
            receiver.try_iter().collect()
        } else {
            Vec::new()
        }
    }
}

impl Default for WintfTaskPool {
    fn default() -> Self {
        Self::new()
    }
}

/// `with_threads` の檻（structure.md のテスト分離規約）
#[cfg(test)]
#[path = "task_pool_tests.rs"]
mod tests;
