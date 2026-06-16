# A1-T: areka エントリポイント × テスト網羅性

- status: completed
- commit: test(A1): main.rs の純粋ロジックに headless ユニットテスト21件を追加

## findings

### モジュール×テスト対応表（改善前 → 改善後）

`crates/areka/src/main.rs`（単一ファイル 399 LOC、改善前テスト 0 件）:

| 区分 | 対象 | 改善前 | 改善後 | 備考 |
|------|------|--------|--------|------|
| 純関数 | `build_typewriter_tokens` | なし | 5件 | 行分割・改行トークン・段落ポーズ・空入力・BALLOON_TEXT 構造 |
| 定数/アセット | `SHELL_IMAGE_PATH` | なし | 1件 | コンパイル時埋め込みパスの実在検証（アセット移動の回帰検知） |
| Entity構築 | `create_shell_window` | なし | 2件 | headless `World` で検証（コンポーネント構成・画像子Entity） |
| Entity構築 | `create_balloon_window` | なし | 4件 | 位置オフセット・階層構造・Typewriter設定・FrameTime 連携 |
| 非同期セットアップ | `run_setup` | なし | 1件 | mpsc チャネル経由のコマンド送信→World 適用まで headless 検証 |
| ハンドラ | `on_shell_pressed` | なし | 3件 | 左ダブルクリック despawn / 非該当クリック / Tunnel 無視 |
| ハンドラ | `on_shell_drag` | なし | 5件 | Tunnel / WindowPos 無し / position 未設定 / バルーン handle 無し / 正常系スモーク |
| GUI/COM 依存 | `main` | なし | なし | 下記「テスト化できない箇所」参照 |

追加テストは S9（steering structure.md テスト命名規約）の Inline 方式に従い、`main.rs` 末尾の `#[cfg(test)] mod tests` に配置（バイナリクレートのため統合テスト不可、in-source が唯一の選択肢）。既存テストの除外は 0 件（除外対象なし）。

### headless テスト可能と判断した根拠（深掘り解析）

- `create_shell_window` / `create_balloon_window` は `World::spawn` によるコンポーネント挿入のみで、実ウィンドウ生成（`CreateWindowExW`）は wintf のシステム実行時にのみ発生する。
- 関与する wintf コンポーネントフックを全数確認した:
  - `on_window_add`: `GetDpiForSystem()`（プロセス安全な syscall）+ `SetWindowParentToLayoutRoot`（LayoutRoot 不在時は no-op）のみ。
  - `on_bitmap_source_add`: `WicCore` リソース不在時は warn して return（COM 不要）。
  - `on_typewriter_add` / `on_rectangle_add`: コンポーネント自動挿入のみ。
  - `on_window_handle_add/remove`: `GetDpiForWindow(null)`→0→デフォルト DPI、`PostMessageW(null hwnd)` はスレッドメッセージ投函のみで headless テストで無害。
- `run_setup` は `CommandSender`（`std::sync::mpsc::Sender<BoxedCommand>`）への送信のみの即時完了 Future であり、noop waker の 1 回 poll（`Waker::noop()`、テスト内ヘルパ `poll_ready`）で駆動可能。

### テスト化できない箇所の深掘り所見

1. **`main()`（約30行）** — `human_panic::setup_panic!`、tracing-subscriber 初期化（`RUST_LOG` フォールバック "info"）、`WinThreadMgr::new()`（COM/メッセージループ初期化）、`mgr.run()`（ブロッキングループ）。プロセスグローバル初期化とブロッキングループのため unit テスト不可。S7 最終起動テストが回帰検知を担う。初期化順序（tracing → WinThreadMgr → spawn → run）に問題は認められない。
2. **`on_shell_drag` 正常系の SetWindowPos 内容検証** — `SetWindowPosCommand::enqueue` は wintf のスレッドローカルキューへの push であり、wintf にキュー検査 API（test 用 drain 等）が存在しないため、enqueue された座標（`pos + BALLOON_OFFSET`）をテストから観測できない。本セルでは「パニックせず false を返す」スモークテストに留めた（P1 として提案記録）。
3. **`create_balloon_window` の `_shell_entity` 引数未使用** — バルーン初期位置はシェルの実位置ではなく定数（`SHELL_INITIAL_X + BALLOON_OFFSET_X`）から導出される。現挙動として正しくテストで固定化した（変更は挙動変更となるため本ループ対象外）。
4. **`SHELL_IMAGE_PATH` の `CARGO_MANIFEST_DIR` 依存** — インストール配布時には解決不能なパスだが、現状はモック実装（開発実行前提）であり挙動変更を伴うため対処せず所見のみ。

### 検証（S2）

- BEFORE: `cargo build --workspace` 成功 / `cargo test --workspace` 全グリーン（areka ユニットスイート "running 0 tests"、計 963 passed / 0 failed）
- AFTER: `cargo build --workspace` 成功 / `cargo test --workspace` 全グリーン（areka 21 passed / 0 failed、既存スイートはベースラインと同一結果、失敗 0）
- RED フェーズ: 既存挙動の特性化テストのため N/A（欠落状態の証跡はベースラインの "running 0 tests"）

## proposals

- P1（report/proposals.md へ追記）: wintf `SetWindowPosCommand` キューのテスト検査 API 追加
