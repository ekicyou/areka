# W1-S: wintf レガシー・プロセス × シンプル化と非推奨コードの実証付き削除

- status: completed
- commit: refactor(W1): 非推奨モジュールの利用実証調査（削除なし・P27 記録）と残存コードの挙動非破壊な簡素化6件

## findings

### 利用実証調査（削除判定・タスク完了条件の必須記録）

調査方法: ワークスペース全域（crates / examples / tests）を対象に、モジュール名・公開シンボル（トレイト名・型名・static・定数）で grep し、`cargo build --workspace` + `cargo build --examples -p wintf` で結線を確認した。

| モジュール | `#![deprecated]` | grep 範囲と結果 | 判定 |
|------------|------------------|-----------------|------|
| `win_message_handler.rs` | **あり**（3モジュール中唯一） | `win_message_handler\|WinMessageHandler\|BaseWinMessageHandler\|WinNcCreate` で全 `*.rs` を検索。利用3件: `winproc.rs:6,16-43`（dispatch 経路のトレイトオブジェクト）、`win_thread_mgr.rs:142`（`create_window` の `Arc<dyn BaseWinMessageHandler>` 引数型）、`examples/dcomp_demo.rs:94`（`impl WinMessageHandler for DemoWindow`） | **削除不可**（R2.9 の「利用ゼロ」を実証できず）。R2.10 に従い削除候補として **P27** に記録（移行が必要な削除セットを明記） |
| `winproc.rs` | なし（`#![allow(deprecated)]` のみ） | `winproc\|wndproc` で crates 全域を検索。`process_singleton.rs:57` がレガシークラス `wintf_window_class` の `lpfnWndProc` に登録（現役結線）。`WM_LAST_WINDOW_DESTROYED` アーム（winproc.rs:77-82、編集後行番号）は `ecs/app.rs:81` が post する終了通知のモーダルループ中の唯一の処理経路として live | **削除候補に該当しない**（非推奨指定なし・現役） |
| `win_thread_mgr.rs` | なし（`#![allow(deprecated)]` のみ） | `win_thread_mgr\|WinThreadMgr\|WM_LAST_WINDOW_DESTROYED\|VSYNC_TICK_COUNT\|LAST_VSYNC_TICK\|DEBUG_WNDPROC_TICK_COUNT` で全 `*.rs` を検索。`areka/src/main.rs:87`（本体エントリポイント）、examples 12 件、`ecs/app.rs:81`・`ecs/world/mod.rs:494`・`ecs/world/vsync.rs:78`（static / 定数依存） | **削除候補に該当しない**（非推奨指定なし・現役の常駐基盤） |

結論: **削除を実施したモジュールはゼロ**。R2.9 の削除条件（deprecated かつ利用ゼロの実証）を満たすモジュールが存在しなかったため、唯一の deprecated モジュール `win_message_handler` を R2.10 に従い P27（削除セット仕様）として記録した。W1-T 所見「steering structure.md は3モジュールすべてを deprecated と記載しているが実態は1モジュールのみ」を再実証し、steering はセル境界外のため P29 として記録した。

### 適用した簡素化（S6 基準・挙動非破壊、5 ファイル +11/-57 行）

W1-T からの申し送り（所見3/4/6/7）を含む6件。テスト未保護の unsafe / Win32 領域（winproc）は R5.5 に従い構造的整理（コメント・自明な整理）に限定した。

1. **`win_style.rs`: 未使用 private 関数 `set_ex2` の削除**（W1-T 所見7）— 呼び出し箇所ゼロを grep で実証（定義と W1-T 申し送り記録のみがヒット）。
2. **`win_style.rs`: no-op メソッド `WS_TILED(self, flag)` の削除**（W1-T 所見3）— `WS_TILED` 定数は値 0 のため ON/OFF とも no-op で、誤解を招く API。ワークスペース内利用ゼロを grep で実証。`publish = false` のため後方互換性の考慮は不要（要件 Adjacent expectations）。同メソッドの no-op を特性化していた W1-T のテスト `ws_tiled_is_noop_because_bit_value_is_zero` は対象消滅のため同時に削除（−1 件。機械的な追随変更）。コンストラクタ `WS_TILEDWINDOW()` / `WS_OVERLAPPED()` は実動作するため温存。
3. **`win_style.rs`: `WS_EX_WINDOWEDGE()` の doc コメント修正**（W1-T 所見4）— 「WS_EX_CLIENTEDGE との組み合わせ」という誤記（WS_EX_OVERLAPPEDWINDOW の説明の転記ミス）を実ビット（0x100 単独）に合わせて修正。特性化テスト `ws_ex_windowedge_sets_only_windowedge_bit` のコメントも追随更新（アサーションは不変）。
4. **`process_singleton.rs`: 常に `None` の dead code `hidden_window` フィールド + アクセサの削除**（W1-T 所見6）— `#[allow(dead_code)]` 付きで一度も `Some` にならないフィールド。W1-T テストの `hidden_window().is_none()` アサーション1行を機械的に削除（テスト関数自体は存続）。
5. **`winproc.rs`: 構造的整理（R5.5 限定）** — (a) `into_boxed_ptr` の冗長な4行を `Box::into_raw(Box::new(self)) as _` の1式へ集約（同一動作の機械的整理）、(b) コメントアウトされたデバッグ `eprintln!` 2行と、それのみが理由で存在した `rc` 束縛の除去、(c) `get_boxed_ptr` へ健全性違反（P28）を明示する NOTE コメントを付与。ロジック・制御フローの変更なし。
6. **ブランケット lint 抑制の削減（4 ファイル）** — 実態に不要な `#![allow(non_snake_case)] / #![allow(unused_variables)] / #![allow(dead_code)]` を `api.rs`（3件全部）・`winproc.rs`（3件。`#![allow(deprecated)]` は必要なため温存）・`win_style.rs`（unused_variables / dead_code。non_snake_case は WS_* メソッド名に必要なため温存）・`win_state.rs`（non_snake_case。unused_variables はトレイトデフォルト実装の未使用引数に必要なため温存）から削除。ビルドで警告ゼロを確認。

### 見送った簡素化

- **`winproc.rs` の健全性修正（トレイト型混同 + mutable transmute）** — ロジック変更を要するテスト未保護 unsafe 領域のため R5.5/R2.8 に従い見送り、NOTE コメント + **P28** として記録。
- **`win_message_handler.rs`（1,400 行）内部の整理** — モジュール全体が削除候補（P27）であり、deprecated コードへの磨き込みは無価値なため一切手を入れず。
- **`win_thread_mgr.rs` の整理** — テスト未保護の現役メッセージループ / VSync スレッド基盤。構造的にも自明な重複・dead code がなく、変更リスクに見合う簡素化対象なしと判断（`try_tick_normal` の単一利用ラッパーは命名による意図表明として妥当と判断し温存）。

### 検証（S2）

- BEFORE: `cargo build --workspace` + `cargo build --examples -p wintf` 成功 / `cargo test --workspace` **1210 passed / 0 failed**（親指示ベースラインと一致。既知フレーキーも本実行では合格）
- AFTER: `cargo build --workspace` + `cargo build --examples -p wintf` 成功（警告 0 — lint 抑制削除後も警告が出ないことを確認）/ `cargo test --workspace --no-fail-fast` **1209 passed / 0 failed**（−1 は簡素化2で対象メソッドと同時に削除した特性化テスト。それ以外の既存テストの変更なし）
- プロダクション変更は dead code / no-op API の削除・コメント・lint 抑制削減のみで、実行経路のロジック変更ゼロ＝外部観測可能な挙動の変更なし（R5.1）。削除 API（`WS_TILED` / `hidden_window` / `set_ex2`）はいずれもワークスペース内利用ゼロを実証済みのため破壊なし（R5.3）

## flaky

- AFTER の全体実行で既知フレーキー `wintf tests/ecs cue_performance_test::bench_pop_ready_empty_queue` が2回 fail（10,000 回空 pop_ready 1.75ms > 閾値 1ms。並行 cargo プロセスの負荷下）。隔離再実行で即合格（0.00s）、続く `--no-fail-fast` 全体実行でも 1209/0 で安定合格 → **パススルー判定**（W1-T と同一パターンの壁時計ベンチマーク負荷依存。境界外のため対処せず記録のみ）。

## proposals

- **P27**（新規): 非推奨モジュール `win_message_handler` の削除セット仕様（利用3件の一括移行。R2.10 による記録）
- **P28**（新規): `winproc::get_boxed_ptr` の健全性違反修正（型混同 + mutable transmute。P27 実施で経路ごと消滅するため P27 優先を推奨。7.3 W1-V の脆弱性観点にも関連）
- **P29**（新規): steering `structure.md` の deprecated 記載（3モジュール一括）と実態（1モジュールのみ）の乖離修正（steering は境界外のため記録のみ）
