# A1-S: areka エントリポイント × シンプル化

- status: completed
- commit: refactor(A1): main.rs の挙動非破壊な簡素化（不要割当・未使用引数・重複クエリの除去）

## findings

### S6（karpathy-guidelines）基準で検証した候補一覧

`crates/areka/src/main.rs`（境界内唯一のファイル、A1-T 由来の 21 テストが回帰検知器）:

| # | 候補 | S6 根拠 | 判定 |
|---|------|---------|------|
| 1 | `build_typewriter_tokens` の中間 `Vec<&str>` 割当 | 不要な中間構造（最小コード原則） | **適用** |
| 2 | `create_balloon_window` の未使用引数 `_shell_entity` | 使われない「柔軟性」＝投機的結合 | **適用** |
| 3 | `on_shell_pressed` のシェル/バルーン 2 連クエリ＋`chain` | 自明な重複（ほぼ同一のクエリ・収集ブロック×2） | **適用** |
| 4 | `on_shell_drag` の未使用束縛 `_event` と 2 段階 Option 展開 | 不要な中間変数 | **適用** |
| 5 | マーカー構造体の `pub` と derive 一式（`PartialEq, Hash` 等） | 未使用 derive だがマーカーの慣例で、削除利得僅少 | 見送り（churn 回避） |
| 6 | `BALLOON_OFFSET_Y = 0` の加算除去 | +0 は冗長だがオフセット概念の対称性・意図表明を担う | 見送り |
| 7 | `Brushes` 色リテラルのヘルパ抽出 | 単一用途コードへの抽象化禁止（S6 #2）に抵触 | 見送り |
| 8 | `main()`（テスト保護外: COM 初期化・メッセージループ） | 構造的整理のみ可（R5.5）。命名・コメント・重複に指摘事項なし | 変更なし |
| 9 | バルーン位置導出の一元化（定数依存→シェル実体依存） | 重複知識の除去だがロジック変更を要する | 見送り→ **P2 提案記録** |

### 適用した簡素化と根拠

1. **`build_typewriter_tokens`**: `text.split('\n').collect::<Vec<_>>()` の中間割当を除去し、イテレータを直接 `enumerate`。トークン列の出力は不変（純関数テスト 5 件で固定）。
2. **`create_balloon_window(world, _shell_entity)` → `create_balloon_window(world)`**: 引数は関数内で一切参照されておらず（バルーン初期位置は定数 `SHELL_INITIAL_* + BALLOON_OFFSET_*` から導出）、削除はロジック非変更。`run_setup` 内の不要になった `shell_entity` 束縛も削除（自分の変更が生んだ孤児の掃除）。テスト 4 件の呼び出し箇所を機械的に追従（アサーションは無変更）。
3. **`on_shell_pressed`**: `With<ShellWindowMarker>` / `With<BalloonWindowMarker>` の重複クエリ 2 ブロック＋`chain` を、単一の `Or<(With<...>, With<...>)>` フィルタクエリへ統合。despawn される集合は同一（`double_click_left_despawns_all_marked_windows` が集合の意味論＝マーカー保持エンティティ全消去・非保持温存を固定）。despawn の列挙順は内部詳細であり、外部観測可能な挙動（ダブルクリック→全ウィンドウ終了）は不変。
4. **`on_shell_drag`**: 未使用の `Phase::Bubble(_event)` を `Phase::Bubble(_)` に、`shell_pos` 中間変数を let-else への直接展開に統合。分岐 5 件のテストで固定済み。

diff 合計: main.rs のみ 19 insertions / 28 deletions。

### 適用見送りと根拠

- 候補 5・6・7: 上表のとおり。挙動非破壊で適用可能だが、S6「最小 diff・既存スタイル尊重・壊れていないものを直さない」に照らし利得が薄く churn と判断。
- 候補 8（`main()` 約 30 行）: テスト保護外（R5.5 対象）。深掘り解析の結果、初期化順序（human_panic → tracing → WinThreadMgr → spawn → run）・操作ガイド出力・コメントのいずれにも構造的整理に値する指摘なし。ロジック変更を要する簡素化候補も検出されず。
- 候補 9: 下記 proposals 参照。
- 非推奨コード（R2.9/R2.10）: `crates/areka` 配下に `deprecated` 指定は 0 件（grep で実証）。削除対象なし。

### 検証（S2）

- BEFORE: `cargo build --workspace` 成功 / `cargo test --workspace` 全グリーン（18 スイート、993 passed / 0 failed / 32 ignored、areka 21 passed）
- AFTER: `cargo build --workspace` 成功（警告 0）/ `cargo test --workspace` 全グリーン（BEFORE と同一の 18 スイート・993 passed / 0 failed / 32 ignored、areka 21 passed）。既存テストの失敗 0、アサーション変更 0（変更はシグネチャ追従の呼び出し箇所のみ）

### 再検証（タスク 2.2 再実行ラン）

- 本セルは先行ランで完遂済み（`refactor(A1): エントリポイントの挙動非破壊な簡素化4件を適用` = `f49ae86`）。現行 `crates/areka/src/main.rs` は 4 件の簡素化（中間 Vec 除去・未使用引数 `_shell_entity` 除去・`Or` 単一クエリ統合・let-else 統合）を既に反映している。
- 再実行ランで S6 基準の再点検を実施: 適用可能な新規の挙動非破壊簡素化は検出されず（候補表 1–9 の判定は不変）。`deprecated` / `unwrap()` / `.clone()` / `#[allow]` / `TODO` は areka 境界内に 0 件（grep 実証）。karpathy-guidelines「churn 回避」に従い追加変更なし。
- S2 再検証（このラン）: `cargo build -p areka` 成功 → `cargo test -p areka` **22 passed / 0 failed**（A1-V 由来の overflow テスト追加で 21→22）。`cargo build --workspace` 成功 → `cargo test --workspace` 全スイート **0 failed**（areka 22 passed、wintf cue_performance_test 含め本ラン全グリーン）。
- 注: 作業ツリーに `crates/wintf/` の未コミット変更が存在するが他セルの所有であり本境界外。areka 境界は本ランで変更なし（`git status` で main.rs は clean）。

## flaky

なし（wintf cue_performance_test は BEFORE / AFTER とも全実行内で安定パス。隔離再実行は不要だった）

## proposals

- P2（report/proposals.md へ追記）: バルーンウィンドウ位置導出のシェル実体への一元化
