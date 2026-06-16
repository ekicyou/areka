# W7a-S: wintf ウィンドウ・メッセージ × シンプル化（unsafe 保守則適用）

- status: completed
- commit: （親が S10 準拠でコミット）

## 観点
- セルID: W7a-S（領域 W7a「wintf ウィンドウ・メッセージ」 × 観点 S「シンプル化」）。調査範囲は **`ecs/window/` + `ecs/window_proc/` 全体**。
- 性質: 非挙動変更（リファクタリング／簡素化）。Feature Flag Protocol 不要。
- requirements: 2.2, 2.5, 2.7, 2.8, 4.1, 5.1, 5.3, 5.5（特に **R5.5: テスト保護外の unsafe/GUI/Win32 は構造的整理に限定**）
- design: S6（karpathy 基準）、S2 検証、S10 コミット規約、W7a 領域定義、観点列 S（L177）、unsafe 保守則（L337）、セル断片様式（L440）、提案記録様式（L453）
- 回帰検知器: 直前 17.1 W7a-T1（`ecs/window/` に43件）+ 17.2 W7a-T2（`ecs/window_proc/` に14件、計57件）の特性化テスト。本セルの簡素化後に S2 全量がベースライン（1625/0）と一致することが挙動非破壊の証拠。

## 調査範囲（boundary = `crates/wintf/src/ecs/window/` + `ecs/window_proc/`）

両ディレクトリ全16ファイルを精読し、`cargo clippy -p wintf --lib` の simplification 系 lint を各候補の起点とした。各候補を「テスト保護下か」「unsafe/GUI/Win32 域か」で分類した。

### clippy（simplification 系）境界内ヒットの分類（BEFORE 実測）

`cargo clippy -p wintf --lib --message-format=short` の境界内ヒットを lint 種別ごとに集計（BEFORE）:

| lint 種別 | 境界内件数 | クレート全域 | 判定 |
|-----------|-----------|-------------|------|
| `useless_conversion`（同型 `.into()`） | **8** | 8（**全件が W7a 境界内**） | **適用**（同型恒等変換の除去・構造整理） |
| `derivable_impls`（`impl Default`） | **1**（ZOrder） | 5（他4は arrangement/dimension/app = 別境界） | **適用**（テスト保護下の純粋型・boilerplate 削減） |
| `collapsible_if` | 29 | 68 | 見送り（クレート全域容認・R5.5 域・churn） |
| `question_mark`（`let...else`→`?`） | 8 | 8（全件 W7a 境界内） | 見送り（実 WndProc ハンドラ域・R5.5・churn） |
| `drop_non_drop`（`mem::drop`） | 3 | 3 | 見送り（**借用解放のため load-bearing**・削除でコンパイル不能） |
| `type_complexity`（Bevy Query/SystemState） | 1 | 30 | 見送り（クレート全域の典型偽陽性・churn） |
| `default_constructed_unit_structs` | 1 | 1 | 見送り（実 CreateWindowExW システム内・R5.5・単発 churn） |

## 適用した簡素化（2種・計9件の clippy lint を解消、いずれも挙動非破壊）

### 適用1: `ZOrder` の手書き `impl Default` → `#[derive(Default)]` + `#[default]`（window/window_pos.rs）

`window/window_pos.rs:24-44` の `ZOrder` enum は `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` + 手書き `impl Default { fn default() -> Self { ZOrder::NoChange } }` だった。最初のバリアント `NoChange` に `#[default]` 属性を付け、derive へ `Default` を追加して手書き impl（6行）を削除した（+1属性 / −6行 = net −5行）。

- **挙動非破壊根拠**: `#[derive(Default)]` + `#[default] NoChange` が生成する `Default::default()` は手書き実装と完全に同一（`ZOrder::NoChange` を返す）。コンパイラが生成する関数本体は等価。
- **テスト保護**: `ZOrder` は **W7a-T1 の `test_zorder_default_is_no_change`（window_pos.rs in-source）でテスト保護下**の純粋型。`ZOrder::default() == ZOrder::NoChange` を直接アサートしており、簡素化後も同テストが緑（window:: 43/0）。
- **R5.5 整合**: `ZOrder` は純粋な enum（unsafe impl Send/Sync は付くが Default ロジックに無関係）。テスト保護下のロジックゆえ R5.5 の「構造整理限定」制約の対象外（より踏み込んだ簡素化が許される領域）だが、本変更は実質 boilerplate 除去に留まる。clippy `derivable_impls` を解消。

### 適用2: 同型恒等 `.into()` の除去（8件・window/window_pos.rs ×2 + window_proc/window_pos.rs ×6）

clippy `useless_conversion`（"useless conversion to the same type"）が指摘する**同型恒等変換**8件を除去した。すべて `Point → Point` / `SizeI → SizeI` の自己変換（reflexive blanket impl）であり、clippy が「同型」と証明済み。

- **window/window_pos.rs:332**（`to_window_coords`）: `client_to_window_coords(position.into(), size.into())` → `(position, size)`。`position: Point` / `size: SizeI` を同型のまま渡すだけ。
- **window_proc/window_pos.rs:108-109**（`WM_WINDOWPOSCHANGED` の bypass 経路）: `bypass.position = Some(client_pos.into())` → `Some(client_pos)`、`bypass.size = Some(client_size.into())` → `Some(client_size)`。
- **window_proc/window_pos.rs:136-142**（同ハンドラの DerefMut 経路）: `Some(corrected_pos.into())` ×2 → `Some(corrected_pos)`、`Some(client_size.into())` ×2 → `Some(client_size)`。`correct_position_for_dpi_center_preserve` は `Point` を返し（dpi_helpers.rs:68）、`window_to_client_coords` は `(Point, SizeI)` を返す（window_handle.rs:174）ため、`WindowPos.position: Option<Point>` / `size: Option<SizeI>` への代入は同型恒等。

- **挙動非破壊根拠**: `useless_conversion` lint は `From`/`Into` が **reflexive 恒等（`T: Into<T>`）の場合にのみ発火**する。除去後の生成コードは `.into()` 付きと完全に同一（恒等変換は no-op）。`cargo build --workspace` 成功が「型が一致し変換が不要だった」ことを実証（コンパイラが bare 値を受理）。
- **テスト保護**: window/window_pos.rs:332 の `to_window_coords` は実 HWND 経路（`client_to_window_coords` が AdjustWindowRectExForDpi を呼ぶ）でユニット保護外。window_proc/window_pos.rs の6件は**実 WM_WINDOWPOSCHANGED ハンドラ内**（R5.5 域、ユニット保護外）。ただし `.into()` 除去はトークンレベルの恒等変換除去であり**ロジック・制御フロー・型に一切触れない構造整理**。S2 全量（1625/0 ベースライン一致）と実起動 S7 が最終的な回帰検知器。
- **R5.5 整合**: R5.5 は「ロジック変更を伴う簡素化」を禁じるが、**同型恒等トークンの除去は命名・表記の構造的整理に該当**しロジック非介入。前例 **W2-S 適用5**（COM 域の `windows_core::Result<()>` → `Result<()>` 型表記揺れ統一）と同種の「同型ノイズ除去」。8件全てが W7a 境界内に集中（クレート全域でも 8/8）するため、解消してもクレート内の不整合 churn を生まない（むしろ `useless_conversion` lint カテゴリを wintf lib から一掃する contained な解消）。

## R5.5 で構造整理に限定／見送った unsafe・GUI・Win32 域の候補

- **`collapsible_if`（境界内29件・クレート全域68件）**: keyboard.rs/lifecycle.rs/mouse_click.rs/mouse_dblclick_wheel.rs/mouse_move.rs/window_proc/window_pos.rs の実 WndProc ハンドラ内、および window/window_pos.rs:425（`SetWindowParentToLayoutRoot::apply` の Command/World 操作）に分布。(a) いずれも**実 WndProc / Command 域**（R5.5 でロジック非介入の構造整理に限定）、(b) クレート全域が collapsible_if を68件容認し let-chain を一切採用していない（採用すれば W7a だけ不整合な churn・karpathy 3 違反）ため**見送り**。前例 W5a-S の collapsible_if 見送り判断と同一基準。
- **`question_mark`（`let...else { return None }` → `?`、境界内8件）**: mouse_move.rs:50/55/105/439・mouse_click.rs:28・mouse_dblclick_wheel.rs:35/194/213 の各ハンドラ冒頭の bailout イディオム。すべて**実 WndProc ハンドラ（`HandlerResult = Option<LRESULT>`）の本体先頭**で、`get_entity_from_hwnd` / `try_get_ecs_world` が None なら `DefWindowProcW` へ委譲する明示的早期 return。`?` への書き換えは挙動等価だが、(a) 6ハンドラファイルで一貫して使われる**確立済みの可読 bailout イディオム**、(b) R5.5 域（テスト保護外の実ハンドラ本体の制御フロー）、(c) 明確な可読性向上のない stylistic churn のため**見送り**（R5.5 + karpathy 2/3）。
- **`drop_non_drop`（`mem::drop`、境界内3件）**: mouse_move.rs:146/319/377 の `drop(entity_ref)`。clippy は「Drop 未実装の値の drop は無意味」と指摘するが、**`entity_ref`（Bevy `EntityWorldMut`）は `world_borrow.world_mut()` を可変借用しており、直後の `entity_mut(...)` 再借用のために借用を NLL で明示終了させる load-bearing な drop**。除去するとコンパイル不能（借用競合）。**見送り必須**（除去は挙動変更どころかビルド破壊）。
- **`type_complexity`（window/window_system.rs:24）**: `create_windows` の `SystemState<(Query<...>, Res<...>)>` 型。Bevy 慣用クエリの典型的偽陽性でクレート全域30件発生。`type` エイリアス化は W7a だけ局所適用すると不整合・可読性も実質低下のため見送り（W5a-S の type_complexity 見送りと同一基準・churn 回避）。
- **`default_constructed_unit_structs`（window/window_system.rs:146）**: `HasGraphicsResources::default()`（unit struct）を bare path にする提案。**実 CreateWindowExW 排他システム内**（R5.5 域・ユニット保護外）の単発の cosmetic 変更で、可読性向上に乏しく churn のみのため見送り（R5.5 保守的判断）。
- **window_system.rs:79-85 の CompositionMode→ex_style 分岐の純粋関数抽出（W7a-T1 申し送り）**: ULW/DComp による ex_style 調整は純粋写像だが、`create_windows`（実 CreateWindowExW + WinProcessSingleton 密結合・ユニット保護外）の本体に埋め込まれている。抽出は**実 GUI システムへのロジック構造変更**（R5.5 で禁ずる「ロジックに踏み込む簡素化」）かつ実起動 S7 でしか回帰検知できないため、本 S セルでは適用せず **P65 として記録**（後述）。

## P64（window_proc メッセージパラメータ抽出）の判断 — 維持・見送り

W7a-T2 が記録した **P64**（LPARAM 座標の lo/hi ワード抽出・WPARAM 修飾キー/XBUTTON/wheel delta 抽出・DoubleClick→PointerButton マッピングが各ハンドラ本体にインライン埋め込みで純粋ヘルパ未抽出、LPARAM 座標式は3ファイル複製）について、本 S セルで抽出可否を慎重に検討した結果、**P64 を維持し抽出を見送る**。

- **抽出対象の精査**: 純粋な重複は (a) `let x = (lparam.0 & 0xFFFF) as i16 as i32; let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;`（mouse_move.rs:28-29/110-111・mouse_click.rs:33-34・mouse_dblclick_wheel.rs:40-41 の4箇所）、(b) `let shift = (wparam_val & 0x04) != 0; let ctrl = (wparam_val & 0x08) != 0;`（mouse_move/mouse_click/mouse_dblclick_wheel の3箇所）。これらは純粋ビット演算で抽出自体は機械的に可能。
- **見送り理由**: (1) **W7a-T2 自身が P64 を「4ファイルにまたがるプロダクション構造変更（判断に迷う構造変更）」と明示的に分類**して proposals へ回している。本タスク指示は「**判断に迷う/挙動影響の疑義があれば P64 維持で見送り**」と規定。先行 T セルが既に不確実性を表明している候補を S セルで強行するのは過剰介入リスク（過去セルの REJECTED 要因）。(2) 抽出は4つの**実 WndProc ハンドラ（テスト保護外・R5.5 域）の本体**に触れ、新規共有モジュール面を作る構造変更。(3) DoubleClick→PointerButton マッピング（mouse_dblclick_wheel.rs:49-56）は `None => return Some(LRESULT(0))` という**ハンドラの制御フロー**を内包し、純粋値写像として綺麗に切り出せない（抽出すればハンドラ構造変更）。(4) 残る重複（4コピーの2行イディオム）は低害・自己文書的で、除去の可読性向上が cross-file 構造 churn に見合わない（karpathy 2/3）。
- **結論**: 純粋デコーダ抽出（DRY + ユニット到達）の価値は認めるが、R5.5（テスト保護外の構造整理限定）+ 先行 T セルの不確実性表明 + churn 回避の総合判断で **P64 維持**。proposals への新規追記なし（P64 はそのまま有効）。

## proposals へ回した候補

- **P65**（新規）: `create_windows` の CompositionMode→ex_style 分岐（window_system.rs:79-85）の純粋関数抽出。ULW=WS_EX_LAYERED / DComp=(ex_style & !WS_EX_LAYERED) | WS_EX_NOREDIRECTIONBITMAP の写像は純粋だが、実 CreateWindowExW + WinProcessSingleton 密結合システムの本体に埋め込まれ、抽出は実 GUI システムへのロジック構造変更（R5.5 で本ループ実施不可）。W7a-T1 所見5の申し送りを提案化。

## 適用しなかった候補と理由（churn 回避等）

- 上記「R5.5 で構造整理に限定／見送った候補」セクションの全 lint（collapsible_if 29・let-else 8・mem::drop 3・type_complexity 1・unit-struct-default 1）。
- **P64 の解析ヘルパ抽出**（上記「P64 の判断」参照・維持）。
- **command.rs / components.rs / dpi.rs / monitor.rs / window_handle.rs / mod.rs**: simplification 系 clippy ヒットゼロ。精読の結果、いずれも S6「最小コード」を既に満たす（command.rs の `find_owner_window`・dpi.rs の変換群・dpi_helpers.rs の補正群はテスト保護下の純粋ロジックだが既に最小で踏み込んだ簡素化の余地なし。window_handle.rs は全面 Win32 でロジック抽出箇所なし）。変更なし。

## S6（karpathy-guidelines）適合確認

- 適用2種（計9 lint 解消）はすべて「既存の冗長（手書き Default boilerplate・同型恒等 `.into()` ノイズ）の除去」であり、新規抽象・投機的柔軟性・不要なエラー処理の追加はゼロ（rule 2 Surgical Changes）。各変更は clippy 診断または W7a-T1 申し送りにトレースでき、自分の変更で孤児化したものなし（rule 3）。
- 成功基準: 「S2 全量がベースライン 1625/0 と一致＝挙動非破壊」を満たした（rule 4）。挙動を変える簡素化候補（P65・P64）はロジック変更につき proposals/維持へ退避し、本ループでは実装しない（R5.2/R5.5）。

## verification (S2)

- BEFORE: 親検証済みベースライン（HEAD = W7a-T2 コミット・クリーンツリー・**1625 passed / 0 failed**）を信頼し省略（親指示・design フェーズ0 規定に従う）。
- AFTER: `cargo build --workspace` 成功（exit 0）/ `cargo test --workspace` **1625 passed / 0 failed**（ignored 32、全20本の `test result:` 行を awk で合算して実測。`error[`/`error`/`panicked`/`FAILED` 行ゼロ）。
  - **ベースラインと完全一致（1625/0）= テストの追加・変更・削除ゼロで全1625既存テストが簡素化後コードをそのまま通過 = 挙動非破壊の裏付け**。グローバル件数変動なし（±0）。
  - 反復検証: `cargo test -p wintf --lib window::` で **43 passed / 0 failed**（W7a-T1 回帰検知器全件緑）、`--lib window_proc::` で **23 passed / 0 failed**（W7a-T2 回帰検知器全件緑、既存9 + 追加14）、`--test window` で統合 **30 passed / 0 failed**（既存維持）。
- 変更ファイル: `crates/wintf/src/ecs/window/window_pos.rs`（+3/−8、net −5）・`crates/wintf/src/ecs/window_proc/window_pos.rs`（+6/−7、net −1）の2ファイル（git diff --numstat 実測）。net −6 LOC。boundary（window/ + window_proc/）内に収束。tests/ 不変・新規テストファイルなし・プロダクションロジック変更ゼロ。

## flaky

- 既知フレーキー `cue_performance_test::bench_pop_ready_empty_queue`（W7a 境界外 `tests/ecs`）: `cargo test --workspace` の全量実行で `tests/ecs` バイナリ合格（隔離再実行不要）。本セルの変更（同型 `.into()` 除去・Default derive 化）とは無関係。

## clippy（S3・記録のみ・非ブロッカー）

- `cargo clippy -p wintf --lib` の boundary（window/ + window_proc/）simplification 系 lint:
  - **`useless_conversion`: 8 → 0**（適用2で全件解消。クレート全域でも 8/8 が W7a 境界内だったため wintf lib から本 lint カテゴリを一掃）。
  - **`derivable_impls`（window/window_pos.rs:40 ZOrder）: 1 → 0**（適用1で解消）。
  - 解消合計 **9 lint**（before/after を `--message-format=short` で実測）。
  - 据え置き（R5.5/churn で意図的に未適用）: collapsible_if 29・let-else 8・mem::drop 3・type_complexity 1・default-unit-struct 1。
  - **新規 clippy 警告/error の導入はゼロ**（適用後 boundary を再 lint し、新規ヒットなしを確認）。wintf lib 全体の警告は 122 → 113（−9＝解消分）。
- **error 20件はすべて `com/d2d/command_sink.rs`**（`clippy::not_unsafe_ptr_arg_deref`= COM vtable コールバックの生ポインタ引数）であり、**boundary 外**・本セル以前から存在（W7a-T1/T2 所見と一致）。boundary 内に error ゼロを実測確認。S3 規定により記録のみ・非ブロッカー。
