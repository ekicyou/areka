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
/// - [`DisplayCommand::ShowBalloon`]: あるバルーンスコープの面が表示面へ確定した（要件 4.1）。
///   シェルの [`DisplayCommand::Show`] と異なり `binds` を持たない（M-boot にバルーン着せ替えは
///   存在しない＝adapter 側で `PresentCommand::ShowSurface{binds: BindSet::default()}` を組む）。
/// - [`DisplayCommand::HideBalloon`]: あるバルーンスコープが非表示へ遷移した（`\b[-1]` 相当・要件 4.2）。
///
/// `ActorKey`（`String` 内包）・`u32`・`BindSet`（`Vec<u32>` 内包）はいずれも `Send` ゆえ、
/// `DisplayCommand` は自動的に `Send`（test の `_assert_send` でコンパイル時固定・要件 5.4）。
///
/// `#[non_exhaustive]` は付けない（workspace 内部契約・variant 追加時にコンパイラが下流 match の
/// 追随を強制する文化を維持する）。
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
    /// バルーン面表示（要件 4.1）。`binds` なし＝M-boot にバルーン着せ替えは存在しない
    /// （adapter は `PresentCommand::ShowSurface{binds: BindSet::default()}` を組む）。
    ShowBalloon { scope: ActorKey, surface_id: u32 },
    /// バルーン非表示（`\b[-1]` 相当・要件 4.2）。
    HideBalloon { scope: ActorKey },
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

    /// 新設のバルーン面指令 `ShowBalloon { scope, surface_id }`／`HideBalloon { scope }` を
    /// 構築し、`PartialEq` で照合できる（要件 4.1/4.2）。`ShowBalloon` は `binds` を持たず
    /// `scope` と `surface_id` のみ、`HideBalloon` は `scope` のみ（M-boot にバルーン着せ替え無し）。
    #[test]
    fn balloon_display_commands_construct_and_match() {
        // 構築と clone 一致（PartialEq で照合可能・要件 4.1）。
        let show = DisplayCommand::ShowBalloon {
            scope: ActorKey::from("0"),
            surface_id: 2,
        };
        assert_eq!(show, show.clone(), "ShowBalloon は clone と一致すること");

        let hide = DisplayCommand::HideBalloon {
            scope: ActorKey::from("0"),
        };
        assert_eq!(hide, hide.clone(), "HideBalloon は clone と一致すること");

        // ShowBalloon と HideBalloon は互いに不一致。
        assert_ne!(show, hide, "ShowBalloon と HideBalloon は不一致であること");

        // distinct な surface_id は不一致（面 id で識別されること・要件 4.1）。
        assert_ne!(
            show,
            DisplayCommand::ShowBalloon {
                scope: ActorKey::from("0"),
                surface_id: 3,
            },
            "surface_id が異なれば ShowBalloon は不一致であること"
        );

        // distinct な scope は不一致（scope で識別されること・要件 4.1/4.2）。
        assert_ne!(
            show,
            DisplayCommand::ShowBalloon {
                scope: ActorKey::from("1"),
                surface_id: 2,
            },
            "scope が異なれば ShowBalloon は不一致であること"
        );
        assert_ne!(
            hide,
            DisplayCommand::HideBalloon {
                scope: ActorKey::from("1"),
            },
            "scope が異なれば HideBalloon は不一致であること"
        );
    }

    /// 観測用出力先へバルーン面指令（`ShowBalloon`/`HideBalloon`）をシェル面指令と混在させて
    /// 送ると、記録が送信順どおりに一致する（要件 4.1/4.2/5.5・FIFO 保持）。既存 `Send` 固定も
    /// 併せて反証する（新 variant 追加後も `DisplayCommand: Send`・要件 5.4）。
    #[test]
    fn mock_records_balloon_display_commands_in_send_order() {
        // 5.4: 新 variant 追加後も Send 所有データであることを固定。
        _assert_send::<DisplayCommand>();

        let mut out = MockSurfaceOutput::new();
        let handle = out.records();

        // シェル面（Show/Hide）とバルーン面（ShowBalloon/HideBalloon）を混在させて送る。
        let commands = vec![
            DisplayCommand::ShowBalloon {
                scope: ActorKey::from("0"),
                surface_id: 2,
            },
            DisplayCommand::Show {
                scope: ActorKey::from("0"),
                surface_id: 2100,
                binds: BindSet::from_ids([48]),
            },
            DisplayCommand::HideBalloon {
                scope: ActorKey::from("0"),
            },
            DisplayCommand::ShowBalloon {
                scope: ActorKey::from("1"),
                surface_id: 6,
            },
            DisplayCommand::HideBalloon {
                scope: ActorKey::from("1"),
            },
        ];

        for command in commands.clone() {
            out.send(command);
        }

        // records() の Arc ハンドルから発行列を照合する（送信順どおり・全件）。
        let recorded = handle.lock().expect("records mutex poisoned");
        assert_eq!(recorded.len(), commands.len(), "全件記録されること");
        assert_eq!(
            &*recorded, &commands,
            "バルーン面指令を含む発行列が送信順どおりに一致すること"
        );
    }
}
