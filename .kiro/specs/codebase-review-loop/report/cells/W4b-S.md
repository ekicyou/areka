# W4b-S: wintf ヒットテスト・計測 × シンプル化

- status: completed
- commit: （親が S10 準拠でコミット）

## 観点
- セルID: W4b-S（領域 W4b「wintf ヒットテスト・計測」 × 観点 S「シンプル化」）
- 性質: 非挙動変更（リファクタリング／簡素化。R5.1）。Feature Flag Protocol 不要
- requirements: 2.2, 2.5, 2.7, 2.8, 4.1, 5.1, 5.3
- design: S6（karpathy-guidelines）、S2 検証、S10 コミット規約、W4b 領域定義、S 観点別規則（design.md:337 — テスト保護外の unsafe/COM/GUI は構造的整理に限定 R5.5）、セル断片様式、提案記録様式

## 調査範囲（対象ファイル一覧・W4b 分担）

W4a-T.md / W4b-T.md のファイル分担定義に従い、W4b 担当分のみを対象として精読・S6 点検した:

- `crates/wintf/src/ecs/layout/hit_test/mod.rs`（560 LOC）
- `crates/wintf/src/ecs/layout/hit_region/mod.rs`（505 LOC）
- `crates/wintf/src/ecs/layout/metrics.rs`（93 LOC）
- `crates/wintf/src/ecs/layout/rect.rs`（208 LOC）
- `crates/wintf/src/ecs/layout/systems/monitor_systems.rs`（278 LOC）
- `crates/wintf/src/ecs/layout/systems/window_pos_systems.rs`（170 LOC）

W4a 分担ファイル（taffy/arrangement/box_style/dimension 系・mod.rs の LayoutRoot）には一切触れていない。直前の 12.1 W4b-T が追加した特性化テスト23件（hit_test ex 3・hit_region 4・metrics 9・rect 5・monitor 2）が本セルの回帰検知器として機能する。

## 適用した簡素化（2 件・いずれも comment-only）

S6（karpathy）基準で点検し、**挙動を変えない構造的整理 2 件**のみを適用した。両者ともコメント帯（テストモジュール宣言の直前バナー）の重複・崩れの是正であり、コンパイル対象トークン列に一切影響しない。

### 適用 1: hit_region/mod.rs の重複・崩れたテストバナーコメントの整理

mod.rs:496-501 に「`// ===` → `// テスト` →（閉じ `===` なし・空行）→ `// === / // テスト / // ===` の完全な二重バナー」という崩れたコメント残骸があった。これを単一の正規バナー（`// === / // テスト / // ===`）へ整理した（コメント 3 行削減）。

- 挙動非破壊根拠: **コメントのみの変更**。Rust のレキサはコメントを破棄するため、生成コードのトークン列・AST はバイト等価。`mod tests;` 宣言およびテスト本体は一切不変。`--lib layout::hit` の hit_region 全テスト（in-source 34件）が緑であることが構造非破壊を裏づける。

### 適用 2: hit_test/mod.rs の重複した区切りコメント行の整理

mod.rs:550-551 に `// ====...` の区切り行が 2 連続（バナーの 1 行重複）していた。重複した 1 行を削除し単一の正規バナーへ整理した（コメント 1 行削減）。

- 挙動非破壊根拠: 適用 1 と同じく**コメントのみの変更**でトークン列バイト等価。`mod tests;` / `mod tests_ex;` 宣言と本体は不変。`hit_test` 系テスト（in-source tests 17 + tests_ex 25 = 42件）が緑。

## proposals へ回した候補（P52）

- **P52**（新規記録）: `hit_test_entity` / `hit_test_entity_ex` の重複統合（ロジック変更を要する簡素化）。
  - 両関数はモード解決・None早期return・GlobalArrangement取得・`bounds.contains`・Bounds 合成α判定・AlphaMask 座標変換まで本体がほぼ完全重複（差分は戻り値 `bool` vs `RegionHit` のみ、約 60 行）。本来 `hit_test_entity` を ex への薄いラッパーへ縮約可能。
  - **しかし観測可能な挙動差が 1 点ある**: 非ex版は `mode == NamedRegions` 時に `if mode == Bounds` を素通りして **AlphaMask 経路**（mod.rs:218-249、BitmapSourceResource 不在で `true` フォールバック）を実行するのに対し、ex版は NamedRegions を独立アームで `HitRegionMap` 判定（mod.rs:378-416）する。委譲化すると NamedRegions エンティティの判定結果が変わり得る（R5.1 違反）。この経路を固定するテストは現状なし（`hit_test/tests.rs` に NamedRegions ケース不在）だが、未テストでも挙動を変えない原則に従い統合を見送り、挙動整合を前提とする小規模仕様として P52 に記録した。

## 適用しなかった候補とその理由（churn / 未テスト経路 / R5.5）

1. **Bounds 合成α判定ヘルパ・AlphaMask 座標変換ヘルパの抽出**（hit_test/mod.rs の 2 関数間で重複する各ブロック） — 抽出自体はトークン等価で挙動非破壊にできるが、(a) NamedRegions 差異により 2 関数は分離維持が必要で、ヘルパは各 2 呼び出し箇所のみの抽象となり karpathy 基準（単一/少数用途への抽象の純増は不採用、churn 回避）に反する、(b) AlphaMask 変換ブロックは WIC/COM 依存で**未到達（W4b-T 所見1/P51）**であり R5.5（テスト保護外はロジック構造の変更を避け構造的整理に限定）の趣旨に照らし不要な手出しを避けた。重複の根本除去は P52 の委譲化で構造的に解消する方が筋が良いため、本セルでは部分抽出によるchurnを作らず P52 へ一本化した。

2. **hit_region/mod.rs:190 の `for i in 0..pixel_count` → `iter_mut().enumerate()`（clippy `needless_range_loop`）** — 当該ループは `buffer[i*4..]`（PBGRA 読み取り）と `index_map[i]`（書き込み）を併用する `ColorMapData::from_image` 内の処理で、**実 PNG/WIC デコードを要する COM 依存の未テスト構築経路**（W4b-T: ColorMap 構築は WIC 依存、テストは hit_test/width/height の消費側のみ）。イテレータ形への書き換えはループ形状のロジック変更であり、R5.5（テスト保護外は構造的整理に限定・ロジック変更を避ける）に従い不適用とした。clippy 警告は S3 記録に留める。

3. **window_pos_systems.rs:22/134 の `type_complexity`（clippy）** — 警告対象は Bevy の `Query<(...), (...)>` システム引数タプル。`type` 別名への分解はシグネチャを難読化し可読性が下がる（システムパラメータの意図が読み取りづらくなる）ため不採用。S3 記録のみ。

4. **hit_test/mod.rs:322 の `HitTestMode::None => unreachable!()`** — None は match 手前（mod.rs:306-308）で早期 return 済みのため到達不能だが、これは exhaustive match の防御アームとして意図的に存置されたもの。除去は match の再構成（明示的な防御の削除）を要し、可読性向上もないため churn と判断し不変。

5. **monitor_systems.rs の `BoxStyle { size: Px(w/h), position: Absolute, inset: Px(left/top)+Auto }` 構築の 3 重複**（initialize_layout_root:59-72 / :112-125、detect_display_change_system:233-246）と update_monitor_layout_system:156-165 の同型代入 — ヘルパ化候補だが、これらの spawn 経路（initialize / detect_display_change）は**実モニタ列挙・実 Win32 API に依存する未テスト経路**（W4b-T 所見2: デバイス依存で決定的検証不能。update_monitor_layout_system のみ合成 Monitor で特性化済み）。テスト保護外でのロジック共通化は R5.5 の趣旨に反し、また回帰検知器が無いため不適用。命名・コメントの整理余地も特に無く無変更とした。

6. **metrics.rs / rect.rs** — 精読の結果、簡素化の余地なし。`LayoutScale`/`Opacity`/`TextLayoutMetrics`（metrics.rs）、`D2DRectExt` 各メソッド・`transform_rect_axis_aligned`（rect.rs）はいずれも O(1) の自明実装で重複・冗長分岐・デッドコードなし。無変更。

## verification (S2)

- BEFORE: 親のベースライン（クリーンワークツリー HEAD `5b38635`、1446 passed / 0 failed）を信頼して流用。
- AFTER: `cargo build --workspace` 成功、`cargo test --workspace` **1446 passed / 0 failed**（ベースライン完全一致 — テストの増減ゼロ、簡素化はコメントのみで挙動・テスト構成不変）。
  - 反復検証: `cargo test -p wintf --test layout` 170 passed / 0 failed、`cargo test -p wintf --lib layout` 82 passed / 0 failed（hit_test/hit_region の in-source 特性化テスト含む全件緑）。
  - 差分: 2 ファイル（hit_test/mod.rs・hit_region/mod.rs）、コメント -4 行（実行コード変更なし）+ 提案記録（proposals.md に P52 追記）+ 本断片。

## clippy（S3・記録のみ・非ブロッカー）

- `cargo clippy -p wintf --lib` の W4b 境界ファイル警告は**変更前後で同一の 3 件**（`hit_region/mod.rs:190` needless_range_loop、`window_pos_systems.rs:22`・`:134` type_complexity）。いずれも既存プロダクションコード由来で、本セルのコメント整理による新規警告の導入なし。3 件とも上記「適用しなかった候補」2/3 で不適用理由を記録済み。wintf lib 全体の警告数 156 件は本セル境界外を含む既存警告。S3 規定によりブロッカーとせず記録に留める。

## flaky

- `cue_performance_test::bench_pop_ready_empty_queue`（既知・負荷依存・W4b 境界外 `tests/ecs`）: 本セルの `cargo test --workspace` 実行（2 回）でいずれも初回から pass。隔離再実行不要だった。本セルの変更（コメント整理のみ）とは無関係。

## RED フェーズ代替の検証

非挙動変更の簡素化タスクのため RED は N/A、テスト追加もなし。適用 2 件はともにコメントのみの変更でトークン列バイト等価であり、AFTER S2 がベースライン（1446/0）と件数・合否ともに完全一致したことが挙動非破壊の証拠。W4b-T の特性化テスト23件（回帰検知器）が全件緑のまま維持された。
