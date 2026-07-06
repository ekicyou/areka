//! 合成入力発行モジュール（発行層）。
//!
//! per-scope surface 状態の確定結果を emo（⑥）への **表示指令** [`DisplayCommand`] へ写し、
//! 発行先抽象 [`SurfaceOutput`] へ送る層。指令は `Send` 所有データ（要件 5.4）で、他スレッド
//! ／後続 spec（emo-present）へ安全に受け渡せる。emo-present の表示指令 API が未完成でも、
//! 本 spec 定義の観測用 [`MockSurfaceOutput`] を発行先にすれば単体観測が閉じる（要件 5.5）。

use areka_emo_compose::BindSet;
use areka_sakura::ActorKey;
use std::sync::{Arc, Mutex};

/// emo（⑥）への表示指令（`Send` 所有データ・要件 5.4）。
///
/// - [`DisplayCommand::Show`]: あるスコープの surface 状態が表示 surface へ確定した（要件 5.1）。
///   解決済み `surface_id` と現在の bind 集合 `binds` を伴う。
/// - [`DisplayCommand::Hide`]: あるスコープの状態が非表示へ遷移した（要件 5.2・DD5）。
///
/// `ActorKey`（`String` 内包）・`u32`・`BindSet`（`Vec<u32>` 内包）はいずれも `Send` ゆえ、
/// `DisplayCommand` は自動的に `Send`（test の `_assert_send` でコンパイル時固定・要件 5.4）。
#[derive(Clone, Debug, PartialEq)]
pub enum DisplayCommand {
    /// 表示: 解決済み surface id と現在の bind 集合を伴う（要件 5.1）。
    Show {
        scope: ActorKey,
        surface_id: u32,
        binds: BindSet,
    },
    /// 非表示遷移（要件 5.2）。
    Hide { scope: ActorKey },
}

/// emo への表示指令の発行先抽象（emo-present 完了を待たない・要件 5.5）。
///
/// 本番の発行先（emo-present の `show_surface`/`hide` 相当への橋渡し）は `ghost-setup` の
/// 領分。本 spec 内では単体観測用に [`MockSurfaceOutput`] を実装として用いる。`send` は
/// infallible（送出失敗のログ観測は本番アダプタの責務）。
pub trait SurfaceOutput {
    /// 1 表示指令を発行先へ届ける（到着順＝FIFO を保つ）。
    fn send(&mut self, command: DisplayCommand);
}

/// 観測用 mock 出力先（`MockSink` 流儀・要件 5.5/7.1）。
///
/// 発行を `Arc<Mutex<Vec<DisplayCommand>>>` の共有蓄積へ FIFO push し、[`records`] が返す
/// Arc クローンを通じて、アクタースレッドへ move した後でもテスト側が発行列を照合できる。
/// `Send + 'static`（`Arc<Mutex<..>>` は `Send + Sync`・`DisplayCommand` は `Send`）。
///
/// [`records`]: MockSurfaceOutput::records
pub struct MockSurfaceOutput {
    records: Arc<Mutex<Vec<DisplayCommand>>>,
}

impl MockSurfaceOutput {
    /// 空の共有蓄積を持つ mock を生成する。
    pub fn new() -> Self {
        Self {
            records: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 共有蓄積の Arc クローンを返す。mock をアクタースレッドへ move した後も、
    /// テストスレッドがこのハンドルから発行列を送信順どおりに照合できる。
    pub fn records(&self) -> Arc<Mutex<Vec<DisplayCommand>>> {
        Arc::clone(&self.records)
    }
}

impl Default for MockSurfaceOutput {
    fn default() -> Self {
        Self::new()
    }
}

impl SurfaceOutput for MockSurfaceOutput {
    fn send(&mut self, command: DisplayCommand) {
        self.records
            .lock()
            .expect("MockSurfaceOutput records mutex poisoned")
            .push(command);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use areka_emo_compose::BindSet;
    use areka_sakura::ActorKey;

    /// `DisplayCommand` が `Send` 所有データであること（要件 5.4）をコンパイル時に固定する。
    fn _assert_send<T: Send>() {}

    /// 観測用出力先へ表示指令列（`Show`/`Hide` 混在）を送ると、記録が送信順どおりに
    /// 一致する（要件 5.1/5.2/5.5・observable: 発行列が送信順と一致）。
    #[test]
    fn mock_records_display_commands_in_send_order() {
        // 5.4: Send 所有データであることを固定。
        _assert_send::<DisplayCommand>();

        let mut out = MockSurfaceOutput::new();
        let handle = out.records();

        // Show / Hide を混在させ、distinct な scope・surface_id・binds で送る。
        let commands = vec![
            DisplayCommand::Show {
                scope: ActorKey::from("0"),
                surface_id: 2100,
                binds: BindSet::from_ids([48, 100]),
            },
            DisplayCommand::Hide {
                scope: ActorKey::from("1"),
            },
            DisplayCommand::Show {
                scope: ActorKey::from("1"),
                surface_id: 2106,
                binds: BindSet::from_ids([]),
            },
            DisplayCommand::Hide {
                scope: ActorKey::from("0"),
            },
        ];

        for command in commands.clone() {
            out.send(command);
        }

        // records() の Arc ハンドルから発行列を照合する（送信順どおり・全件）。
        let recorded = handle.lock().expect("records mutex poisoned");
        assert_eq!(recorded.len(), commands.len(), "全件記録されること");
        assert_eq!(&*recorded, &commands, "発行列が送信順どおりに一致すること");
    }

    /// 記録が空の初期状態から始まる（vacuous pass 防止の反証）。
    #[test]
    fn mock_starts_empty() {
        let out = MockSurfaceOutput::new();
        assert!(out.records().lock().unwrap().is_empty());
    }
}
