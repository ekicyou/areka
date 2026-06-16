# A1-V: areka エントリポイント × 脆弱性

- status: completed
- commit: fix(A1): 脆弱性点検に基づく不変条件の debug_assert・安全コメント・境界値テストを追加

## findings

### 点検対象

`crates/areka/src/main.rs`（境界内唯一のファイル。製品コード約390行 + テスト22件）。点検観点: panic 経路（DoS）・外部入力検証（起動引数・環境変数・ファイルパス）・リソースリーク・整数オーバーフロー・unsafe 境界。

### 1. panic 経路（DoS 可能性）

| # | 経路 | 解析結果 | 対応 |
|---|------|---------|------|
| 1 | `panic!` マクロ（実測1箇所、テスト内 `poll_ready`） | `#[cfg(test)] mod tests` 内のみ。製品バイナリに含まれず外部入力から到達不能 → DoS 経路なし | 対応不要 |
| 2 | `tracing_subscriber::fmt().init()` | グローバル subscriber 二重登録時に panic するが、main 冒頭で1回のみ呼ばれるため発火不能 | 発火不能（構造的に保証） |
| 3 | `world.borrow()`（`Rc<RefCell<EcsWorld>>`） | メインスレッド起動時の単発借用で、競合借用が存在し得ず発火不能 | 発火不能（構造的に保証） |
| 4 | `on_shell_drag` の `pos.x + BALLOON_OFFSET_X` / `pos.y + BALLOON_OFFSET_Y` | debug ビルドのみ overflow panic の可能性。ただし `pos` は wndproc が実ウィンドウ位置から更新する論理座標で、Windows 仮想スクリーン範囲内に収まり `i32::MAX - 335` に遠く及ばない → 発火不能 | **debug_assert + 不変条件コメントを投入**（下記） |
| 5 | `println!`（操作ガイド出力） | release（`windows_subsystem = "windows"`）ではコンソール無しだが、Rust std は無効ハンドルへの書き込みを成功扱いにするため panic しない | 対応不要 |
| 6 | `human_panic::setup_panic!()` | panic ハンドラ設定（release のみ有効）。panic 発生源ではなく、発生時の情報整形のみ | 対応不要 |

結論: 外部入力から到達可能な panic 経路（DoS ベクタ）は存在しない。

### 2. 外部入力の検証

| 入力 | 解析結果 | 対応 |
|------|---------|------|
| 起動引数 | `std::env::args` を一切読まない（攻撃面なし） | 対応不要 |
| 環境変数 `RUST_LOG` | `EnvFilter::try_from_default_env()` は未設定・非UTF-8・不正構文のすべてで Err → `"info"` フォールバック。panic・injection 経路なし。ただし不正値が無音で握り潰され設定ミスに気付けない | フォールバック挙動を**安全コメントで文書化**。警告出力の追加はログ挙動変更のため **P4 提案記録** |
| ファイルパス `SHELL_IMAGE_PATH` | コンパイル時定数（実行時の外部入力ではなく path traversal 不能）。ただし `env!("CARGO_MANIFEST_DIR")` によりビルドマシンの絶対パス（ユーザー名等）が配布バイナリへ埋め込まれる（情報開示）＋ビルドマシン外で画像ロード不能（可用性） | 実行時解決への移行は挙動変更のため **P3 提案記録**（R2.4/R5.2） |
| `run_setup` の `let _ = tx.send(...)` | 受信側喪失時に UI 構築コマンドが無音破棄される失敗黙殺。現構成では受信側 `EcsWorld` が常に生存するため発火不能 | 警告ログ追加はログ挙動変更のため **P4 提案記録** |

### 3. リソースリーク

- HWND/COM リソースは wintf 所有。ダブルクリック終了時はマーカー付きウィンドウ entity の despawn → `on_window_handle_remove` フック → `PostMessageW(WM_CLOSE)` → 最終ウィンドウ破棄で `PostQuitMessage(0)` → `mgr.run()` 終了、の確立済み経路（`crates/wintf/src/ecs/window/window_handle.rs:250-271`, `win_thread_mgr.rs:253-254`）。
- 子 entity（Shell-Image / Balloon-Background / Balloon-Typewriter）は bevy_ecs の `ChildOf` 関係により親 despawn で連鎖 despawn され、孤児化しない。
- mpsc チャネルは1回の send のみで無制限成長なし。spawn された非同期タスクは即時完了 Future。
- 結論: main.rs 起因のリーク経路なし。

### 4. 整数オーバーフロー

- 実行時演算は `on_shell_drag` のオフセット加算のみ（上記 panic 経路 #4 で対応）。`SHELL_INITIAL_X + BALLOON_OFFSET_X` はリテラル定数加算（735/200）でコンパイル時に安全確定。f64 演算（`0.08` / `0.3` / `FrameTime`）にオーバーフロー概念なし。トークン列の割当はコンパイル時定数 `BALLOON_TEXT` の長さで上界。

### 5. unsafe 境界

- 製品コードに `unsafe` は0件（grep で実証）。テスト内の `HWND(null)` / `HINSTANCE(null)` は安全なコンストラクタで参照外しなし。

### 投入した挙動非破壊対策（R2.3/R5.1）

1. **`on_shell_drag`**: オフセット加算前に `debug_assert!(pos.x.checked_add(...).is_some() && ...)` と不変条件コメントを投入。debug ビルドでは加算自体が overflow panic するため assert は新たな panic を導入せず（同条件をより明確なメッセージで先取り）、release ではコンパイル除去される — 外部観測可能な挙動は完全不変。
2. **`main()`**: `RUST_LOG` 不正値のフォールバック挙動（panic 経路なし）を安全コメントで文書化。
3. **境界値テスト追加（additive）**: `drag_at_extreme_virtual_screen_coords_does_not_overflow` — マルチモニタ環境で生じうる負座標・大座標（±1,000,000）でバルーン追従がパニックしないことを固定。areka テストは 21 → 22 件。

diff: main.rs 41 insertions / 0 deletions（既存行の変更・削除なし）+ proposals.md 12 insertions。

### 検証（S2）

- BEFORE: `cargo build --workspace` 成功 / `cargo test --workspace` 全グリーン（18 スイート、993 passed / 0 failed / 32 ignored、areka 21 passed）
- AFTER: `cargo build --workspace` 成功 / `cargo test --workspace` 全グリーン（18 スイート、994 passed / 0 failed / 32 ignored、areka 22 passed）。既存テストの失敗 0・変更 0（差分は追加テスト1件のみ）

## flaky

なし（wintf cue_performance_test は BEFORE / AFTER とも全実行内で安定パス。隔離再実行は不要だった）

## proposals

- P3（report/proposals.md へ追記）: SHELL_IMAGE_PATH のビルドマシン絶対パス埋め込みの解消（実行時パス解決への移行）— kind: 挙動変更を伴う脆弱性対策
- P4（report/proposals.md へ追記）: 起動経路の無音失敗の可観測化（不正 RUST_LOG・UI 構築コマンド送信失敗の警告出力）— kind: その他
