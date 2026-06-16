# W7b-S: wintf ECS基盤・World × シンプル化（App Default 派生化・clippy 簡素化）

- status: completed
- commit: （親が S10 準拠でコミット）

## 観点
- セルID: W7b-S（領域 W7b「wintf ECS基盤・World」 × 観点 S「シンプル化」）。担当は **`crates/wintf/src/ecs/{common,world}/` ＋ `ecs/app.rs`** 全体。
- 性質: **非挙動変更タスク**（リファクタリング／簡素化。R5.1）。直前の W7b-T1（`ecs/common/` 18件）＋ W7b-T2（`ecs/world/`＋`app.rs` 21件、計39件）が回帰検知器。簡素化後に S2 全量が緑であることが挙動非破壊の証拠。Feature Flag Protocol 不要。
- requirements: 2.2, 2.5, 2.7, 2.8, 4.1, 5.1, 5.3（テスト保護外はロジック変更を避け構造整理優先 R5.5）
- design: S 観点手順（CellExecutor「S」規則：S6 基準で簡素化／テスト保護外 unsafe/COM/GUI は構造的整理に限定 R5.5／非推奨コードは利用ゼロ実証で削除 R2.9・できなければ提案 R2.10）、S6 karpathy（churn 回避・自明な重複除去・壊れていないものをいじらない）、領域定義「W7b: wintf ECS基盤・World / `crates/wintf/src/ecs/{common,world}/`, `ecs/app.rs`」、セル断片様式、提案記録様式
- 参考: `report/cells/W7b-T1.md`・`W7b-T2.md`（テスト済み範囲・簡素化候補の申し送り）、`report/cells/W6b-S.md`（derivable_impls 適用・churn 回避見送りの先例）・`W7a-S.md`（テスト保護外 GUI のロジック介入見送り先例）、`report/proposals.md`（末尾 P65）
- 境界: `crates/wintf/src/ecs/{common,world}/`, `ecs/app.rs`。**境界外ファイルの変更なし**（App は `window_count`/`message_window`/`display_configuration_changed` を private 保持し型シグネチャ不変＝再エクスポートへ無影響。derive 追加は struct 定義の attribute のみで API 表面を変えない）。

## 調査範囲
W7b-T1/T2 が作成したモジュール×テスト対応表（common: tree_iter/tree_system／world: mod/schedule_labels/vsync／app）を基に、領域全体（common/ ＋ world/ ＋ app.rs）の S6 簡素化候補を精読 + `cargo clippy -p wintf --lib` の simplification 系 lint で棚卸しした。最有力候補は W7b-T2 申し送りの `app.rs:18` `impl Default for App`（clippy `derivable_impls`）。

### 境界内 clippy 警告の全数（BEFORE）
`cargo clippy -p wintf --lib` 出力をパスフィルタした結果、**境界（common/＋world/＋app.rs）内の clippy 警告は次の計4件のみ**（dead_code/unused/その他 simplification 系はゼロ）:
- `app.rs:18:1` — `clippy::derivable_impls`（`impl Default for App` が `#[derive(Default)]` で代替可能）×1
- `tree_system.rs:17:16 / 55:25 / 89:16` — `clippy::type_complexity`（3つのジェネリック伝播関数の Query/ParamSet シグネチャ）×3

（注: `cargo clippy -p wintf --lib` の出力には `ecs/window_proc/mouse_move.rs:55` の `let...else`→`?` 提案も現れるが、これは **W7a 領域（境界外）** であり本セル対象外。）

## 適用した簡素化（挙動非破壊根拠・テスト保護の有無を明記）

| # | 適用箇所 | 内容 | テスト保護 | 挙動非破壊根拠 |
|---|----------|------|-----------|----------------|
| 1 | `app.rs:18-26` | 手書き `impl Default for App`（window_count:0 / message_window:None / display_configuration_changed:false）を削除し、struct へ `#[derive(Resource, Default)]` を付与（clippy `derivable_impls`） | **あり**（W7b-T2 追加の `app.rs` in-source 6件が `App::default()`／`App::new()`（new は default へ委譲）経由で初期状態 window_count=0・display フラグ false を直接アサート） | 手書き Default は各フィールドを `0`／`None`／`false` で初期化。これは `usize`／`Option<isize>`／`bool` の標準 `Default::default()`（= `0`／`None`／`false`）と**フィールドごとに完全一致**。derive 生成物は各フィールドの Default を呼ぶため手書きと同値の構造体を生成する。`Resource` derive と `Default` derive は独立で共存可（全フィールドが Default 実装持ち）。観測挙動・型・API 表面は不変 |

差分: **1 ファイル変更、+1 / −11 行（net −10）**。`crates/wintf/src/ecs/app.rs` のみ（`impl Default` ブロック10行＋ derive 行の `Default` 追記）。境界外波及ゼロ。

W6b-S が `context.rs::WindowDragContext` に対して実施した `derivable_impls` 適用と**完全に同型**の挙動非破壊簡素化（手書き Default = 各フィールド型の Default と一致 → derive 化）。

## テスト保護外で構造整理に限定した箇所（R5.5）
本セルでは構造整理（命名・コメント・自明な重複除去）を要する箇所は検出されなかった。テスト保護外の以下はいずれも**現状で構造的に整理済み**であり、ロジックに踏み込まず無変更とした:
- `world/vsync.rs`（`VsyncTick`／`IS_TICK_FLUSH_IN_PROGRESS` 再入ガード／`TickFlushGuard`）— 実 vsync/実 Win32 依存でテスト不能（W7b-T2 所見3）。命名・コメントは既に十分自己文書的で、重複・冗長分岐なし。R5.5 によりロジック介入せず無変更。
- `world/mod.rs::try_tick_on_vsync`／`measure_and_log_framerate`（private・実時間依存）— 実時間/vsync 依存でテスト不能（W7b-T2 所見2/3）。線形な atomic 比較・FPS ログで冗長分岐・重複なし。無変更。
- `world/mod.rs::try_tick_world` の13スケジュール順次実行 — 各 `try_run_schedule` は固定順序の必須シーケンスで重複でない（実 graphics 経路は device 依存・W7b-T2 所見2）。無変更。

## 適用しなかった候補と理由（churn 回避・karpathy）

1. **`tree_system.rs:17/55/89` の `type_complexity`（3件）の `type` エイリアス抽出**: いずれも3つのジェネリック伝播関数（`sync_simple_transforms`／`mark_dirty_trees`／`propagate_parent_transforms`）の Query/ParamSet シグネチャに対する clippy 提案。**見送り**。根拠:
   - **(a) 領域全体で支配的な既存パターン**: `cargo clippy -p wintf --lib` の `type_complexity` 警告は**ワークスペース全体で計30件・18ファイルに分布**（graphics/layout/widget/window の bevy ECS システム群。本境界の tree_system.rs は3件のみ）。bevy の ECS システムは Query 型が本質的に複雑であり、本コードベースは `type_complexity` を**全域で抑制せず受容する規約**（`wintf/src` 全体に `#[allow(clippy::type_complexity)]` も `type_complexity` 抑制も**一切存在しない**ことを grep で確認。W6b-S が記録した「wintf lib 全体 145 警告が残存」と同じく baseline 警告を許容する方針）。境界内3件だけを「修正」すると 18ファイル中3ファイルだけが乖離し、prevailing pattern に反する不整合を生む（karpathy「既存スタイルに合わせる」）。
   - **(b) bevy 上流ミラーのジェネリック API**: 3関数は `common/mod.rs` の doc が「サードパーティプラグインは、このシステムを … と組み合わせて使用する必要がある」と明記する通り、bevy `bevy_transform` の transform 伝播 API（`sync_simple_transforms`／`propagate_parent_transforms`／`mark_dirty_trees`）をミラーしたジェネリック実装。シグネチャを上流形から変えるのは churn かつ上流との差分を増やす（karpathy「壊れていないものをいじらない」）。
   - **(c) SystemParam 派生のリスク**: ジェネリック `<L,G,M>` ＋ `'w`/`'s` ライフタイムを持つ Query/ParamSet の型エイリアス抽出は SystemParam 導出に対し型を厳密一致させる必要があり、naive な抽出は微妙な導出エラーを招き得る（純粋に cosmetic な lint のために本番伝播経路の制御に近いシグネチャを触るのは S6 churn 基準に反する）。
   - これは**ロジック変更を要する候補ではない**（純粋なシグネチャ/cosmetic 整理）ため proposals 化もしない（W6b-S が `collapsible_if` を churn 回避で見送り・proposals 化しなかったのと同方針）。本断片に churn 回避の記録に留める。

2. **`world/mod.rs::has_systems` フィールドの除去**: `has_systems: bool` は constructor で `true`・`add_systems`/`schedules_mut` で `true` に設定され、`try_tick_world`（mod.rs:441）の `if !self.has_systems { return false; }` で**読まれる**。現行の全経路で常に `true` だが、これは「システム未登録 World では tick をスキップし `false` を返す」という**観測可能な戻り値契約のガード**（公開 API `try_tick_world` の挙動）であり、デッドコードではない。除去は `try_tick_world` の戻り値セマンティクス変更（=挙動変更）になるため R5.1 により**無変更**（簡素化対象外）。

## proposals へ回した候補
- **新規 P 採番なし**（P66 以降への追加なし）。本セルで検証した簡素化候補は (a) 挙動非破壊として適用済み（App derive Default 1件）、または (b) ロジック変更を要さず churn 回避で見送り（type_complexity 3件・has_systems）であり、いずれも「ロジック変更を要する簡素化」「挙動変更を伴う脆弱性対策」「非推奨コード削除候補」に該当しないため proposals 追記は不要と判断（churn 抑制）。proposals.md 末尾は **P65**（変更なし。次セルの新規採番は P66 から）。
- **W7b-T2 申し送り（`app.rs:18` derivable_impls）を本セルで解消**: T2 が「S 観点 W7b-S への申し送り」とした `derivable_impls` 候補を、本セルで適用・解消した（上記「適用した簡素化」#1）。

## clippy（S3・記録のみ・非ブロッカー）
- 境界（common/＋world/＋app.rs）を参照する clippy 警告:
  - **BEFORE: 4件** — `app.rs:18`（derivable_impls ×1）、`tree_system.rs:17/55/89`（type_complexity ×3）。
  - **AFTER: 3件** — `tree_system.rs:17/55/89`（type_complexity ×3、上記理由で意図的に見送り）。
  - **解消した lint: 1件**（`derivable_impls` 1〔app.rs:18、Default 派生化に伴い消滅〕）。
  - **新規導入 lint: ゼロ**。`app.rs` を参照する clippy 警告は AFTER で皆無。`common/`（tree_iter.rs）・`world/`（mod/schedule_labels/vsync）を参照する警告は BEFORE/AFTER ともゼロ（type_complexity は tree_system.rs の3件のみ）。
- wintf lib 全体（`cargo clippy -p wintf --lib`）の総警告（warning/error 行）数は **BEFORE 183 → AFTER 182（純減1・新規0）**。減少分は本セルが解消した `derivable_impls` 1件に一致。残存は他セル/他モジュールの既存警告（本セルのスコープ外）。S3 規定によりブロッカーとせず記録に留める。

## verification (S2)
- BEFORE: 親検証済みベースライン（**1667 passed / 0 failed**・W7b-T2 完了時点のクリーンワークツリー）を信頼し全量は省略（design フェーズ0 + 親指示「BEFORE S2 は省略可」）。反復検証ベースラインとして開始時にワークツリーがクリーン（`git status --porcelain` 空）を確認。
- AFTER:
  - `cargo build --workspace` **成功**（`areka` 本体・`wintf` 含む全クレート。App の手書き Default 削除＋derive 化がビルドを壊さないことの確認）。
  - `cargo test --workspace` **1667 passed / 0 failed / 32 ignored**（全 `test result:` 行を awk 合算: passed=1667 / failed=0 / ignored=32。`test result: FAILED`/`error[`/`panicked` 行ゼロ）。
  - **増減内訳**: 1667 → 1667（**±0**）。derive Default 化は既存テスト（App 6件は default/new 経由で初期状態を assert）を一切削除・追加せず通過したため件数不変。
  - 反復検証: `cargo build -p wintf` 成功・`cargo test -p wintf --lib "ecs::app::tests"` で **6 passed / 0 failed**（derive 生成 Default が手書きと等価＝初期状態 window_count=0・display フラグ false を全件 GREEN で再確認）。
  - **回帰検知器の残存確認**: W7b-T1 の18件（tree_iter 走査・tree 伝播）＋ W7b-T2 の21件（App カウント／schedule_labels derive 契約／EcsWorld FrameCount 進行・スケジュール登録・message_window）が全て GREEN。特に App の `Default`/`new` 初期状態を assert する6件が derive 化後も成立し、Default 等価を保証。

## flaky
- 既知フレーキー `cue_performance_test::bench_pop_ready_empty_queue`（W7b 境界外 `tests/ecs`）: `cargo test --workspace` の全量実行で failed=0（当該テスト含む）。さらに隔離再実行 `cargo test -p wintf --test ecs cue_performance_test` で **5 passed / 0 failed**（`bench_pop_ready_empty_queue` 含め全 `... ok`）で安定合格を確認。本セルの変更とは無関係。

## 自己レビュー結論
- 適用した簡素化は `app.rs` の `derivable_impls`（手書き Default → derive Default）1件のみ。手書き Default が各フィールド型の標準 Default と完全一致することを型で裏取り済み（usize=0・Option=None・bool=false）。観測可能な挙動変更なし（R5.1/R5.3）。テスト保護下（W7b-T2 の App 6件が回帰検知器）。
- App は private フィールドのみで型シグネチャ・API 表面が不変ゆえ境界外（ecs/mod.rs 等の再エクスポート）への波及なし。git diff は app.rs 単一ファイル +1/−11。
- テスト保護外の vsync/実時間依存箇所（vsync.rs・try_tick_on_vsync・measure_and_log_framerate）は R5.5 に従いロジック介入せず、構造整理を要する重複・冗長も検出されず無変更。
- churn を要する候補（type_complexity 3件・has_systems）は karpathy「壊れていないものをいじらない」「既存スタイルに合わせる」＋ 領域全体での type_complexity 受容規約（18ファイル30件・抑制ゼロ）＋ bevy 上流ミラー API 保護の観点で見送り。ロジック変更を要さないため proposals 新規採番もなし（P65 末尾不変、W7b-T2 申し送りの derivable_impls を解消）。
- S2 全量 AFTER = 1667 passed / 0 failed（ベースラインと完全一致＝±0、数字は実測）。clippy 境界警告 4→3（純減1・新規0）。
