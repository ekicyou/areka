# W5b-S: wintf 図形・画像・ブラシ × シンプル化（unsafe 保守則適用）

- status: completed
- commit: （親が S10 準拠でコミット）

## 観点
- セルID: W5b-S（領域 W5b「wintf 図形・画像・ブラシ」 × 観点 S「シンプル化」）
- 性質: 非挙動変更（リファクタリング／簡素化）。Feature Flag Protocol 不要。
- requirements: 2.2, 2.5, 2.7, 2.8, 4.1, 5.1, 5.3, 5.5（特に R5.5: テスト保護外の unsafe/GUI/COM は構造的整理に限定）
- design: S6（karpathy 基準・L136）、S2 検証、S10 コミット規約、W5b 領域定義（L162）、S 観点手順（L337）、unsafe 保守則（L177, L271, L515）、セル断片様式（L440）、提案記録様式（L453）
- 回帰検知器: 直前 14.1 W5b-T が追加したデバイス非依存の特性化テスト18件（brushes 6・rectangle 4・bitmap_source/tests 3・alpha_mask 5）。本セルの簡素化後に S2 が緑であることが挙動非破壊の証拠。

## 調査範囲（boundary = `widget/{shapes,bitmap_source}/` + `widget/brushes.rs`）

W5b-T のモジュール×テスト対応表をもとに3モジュール群（約1,690 LOC）を精読し、`cargo clippy -p wintf --lib` の simplification 系 lint を併用して候補を列挙。各候補を「テスト保護下か」「unsafe/GUI/WIC/COM 域か」で分類した。

clippy（simplification 系 lint）の W5b 境界内ヒットは正確に6件:

| 箇所 | lint | 分類 | 判定 |
|------|------|------|------|
| `bitmap_source/alpha_mask.rs:35` | `manual_div_ceil` | **テスト保護下の純粋ロジック**（`from_pbgra32`） | **適用** |
| `bitmap_source/alpha_mask.rs:73` | `manual_div_ceil` | **テスト保護下の純粋ロジック**（`is_hit`） | **適用** |
| `brushes.rs:85` | `derivable_impls` | **テスト保護下の純粋データ**（`impl Default for Brush`） | **適用** |
| `shapes/rectangle.rs:128` | `type_complexity` | Bevy `Query` 偽陽性（GUI 描画システム） | 見送り（churn） |
| `bitmap_source/systems.rs:139` | `type_complexity` | Bevy `Query` 偽陽性（D2D 描画システム） | 見送り（churn） |
| `bitmap_source/systems.rs:349` | `collapsible_if` | World 操作（`SetAlphaMaskCommand::apply`、let-chain 提案） | 見送り（churn・R5.5 隣接） |
| `bitmap_source/mod.rs:7` | `module_inception` | モジュール構造（`mod bitmap_source` 内包） | 見送り（構造変更・churn） |

（注: clippy 境界内ヒットは上表の通り。`graphics/systems/brushes.rs` は別ファイル・W3b 境界であり本セル対象外。）

## 適用した簡素化（2件・対象2ファイル）

### 適用1: `Brush` の手動 `impl Default` を derive へ移行（derivable_impls 解消）

`brushes.rs` の `Brush` enum は手動の `impl Default for Brush { fn default() -> Self { Brush::Inherit } }`（旧 L85-89、5行）を持っていた。これを `#[derive(..., Default)]` + 既定バリアント `Inherit` への `#[default]` 注記へ置換し、手動 impl ブロックを削除した（+2 / −7、net −5 LOC）。

- **挙動非破壊根拠**: `#[derive(Default)]` + `#[default]` が生成する `Default::default()` は `Brush::Inherit` を返し、手動実装と**完全に同一の値・同一のセマンティクス**。clippy `derivable_impls` 自身が「derive で置換可能（＝等価）」と判定している。`Brush` は `#[derive(Default)]` 追加で既存の他 derive（Clone/Debug/PartialEq）と独立に Default を導出するのみで、他トレイトに影響しない。
- **テスト保護**: W5b-T 追加の `test_brush_default_is_inherit`（`Brush::default() == Brush::Inherit` かつ `is_inherit()` かつ `as_color().is_none()` を固定）が直接の回帰検知器。加えて `Brushes::default()`（`test_brushes_default`）が `Brush::Inherit` を 2 フィールドに用いる経路も保護下。S2 全量が 1486/0 でベースライン一致＝挙動非破壊を実証。
- **R5.5 整合**: `Brush` は純粋データ型（`D2D1_COLOR_F` を保持するのみ・unsafe なし）でテスト保護下。さらに本変更は**確立済みのクレート慣習への整合**である — 同 `widget/text/` 配下の sibling `TextDirection`（label.rs:13）・`typewriter.rs:95`・`typewriter_ir.rs:34` を含むクレート内 7 箇所が既に `#[derive(Default)]` + `#[default]` 形式を採用しており、`Brush` の手動 impl だけが乖離していた。本変更は乖離の解消であり churn を**減らす**方向（karpathy 3「既存スタイルに合わせる」）。

### 適用2: `AlphaMask` の天井除算を `div_ceil` へ（manual_div_ceil 解消・2 箇所）

`alpha_mask.rs` の `from_pbgra32`（L35）と `is_hit`（L73）で行バイト数を `((width + 7) / 8) as usize` と手計算していた箇所を、いずれも `width.div_ceil(8) as usize`（`is_hit` は `self.width.div_ceil(8)`）へ置換した（+2 / −2、net 0 LOC）。

- **挙動非破壊根拠**: `div_ceil(8)` は `(n + 7) / 8` と**数学的に同一の天井除算**で、対象 `u32` の全値で結果が一致する（`0.div_ceil(8) == 0 == (0+7)/8`、`1.div_ceil(8) == 1`、`8.div_ceil(8) == 1`、`9.div_ceil(8) == 2` 等）。むしろ `div_ceil` は `n + 7` の事前加算によるオーバーフローを内部で回避するため厳密には**より安全**（実利用幅では結果不変）。式の意図（8 ピクセル単位の行アラインメント）が標準ライブラリ API で自己文書化される。
- **テスト保護**: W5b-T 追加の `test_zero_width_produces_empty_mask`（width=0 退化→ `row_bytes=0`）・`test_bit_packing_across_byte_boundary`（16px 幅で row_bytes=2 のバイト境界跨ぎ）・`test_padded_stride_reads_correct_rows`・`test_is_hit_at_last_valid_pixel` および既存 `test_wide_image_bit_packing`（10px 幅）が `row_bytes` 計算経路を直接被覆。`from_pbgra32` と `is_hit` の両方が同一式を用いるため、is_hit 系テストが両箇所を保護。focused `cargo test -p wintf --lib widget::` 72/0 + S2 全量 1486/0 で実証。
- **R5.5 整合**: `from_pbgra32`/`is_hit` は WIC/D2D に一切依存しない純粋ビットパック計算で、W5b-T により 13 件（既存8＋追加5）で特性化済みの**テスト保護下ロジック**。R5.5 が構造整理に限定する「テスト保護外の unsafe/GUI」には該当せず、回帰検知器の保護下で標準 API への置換を適用してよい領域（タスク指示「`manual_div_ceil` の `div_ceil` 化はテスト保護下なら適用候補」に合致）。

## R5.5 で構造整理に限定／見送った unsafe・GUI・WIC 域の候補

- **`bitmap_source/systems.rs:349` の collapsible_if（`SetAlphaMaskCommand::apply` 内）**: clippy は `if let Ok(..) { if let Some(..) {..} }` の let-chain 統合（`if let Ok(..) && let Some(..)`）を提案。当該ブロック自体は World 操作（`get_entity_mut` + `get_mut::<BitmapSourceResource>`）で unsafe/WIC ではないが、(a) 当リポジトリは collapsible_if をクレート全域で **71 件**容認し let-chains を**一切採用していない**（採用すれば本箇所だけ不整合な churn・karpathy 3 違反）、(b) 直前 W5a-S が同種の collapsible_if（`typewriter_draw.rs:161`）を同じ根拠で見送り済みで判断の一貫性が必要。**見送り**（churn 回避、可読性改善が乏しい）。
- **`shapes/rectangle.rs:128` / `bitmap_source/systems.rs:139` の type_complexity（Bevy `Query`）**: いずれも `draw_rectangles` / `draw_bitmap_sources`（全面 D2D 依存の描画システム）の Query 型。クレート全域で **31 件**発生する Bevy 慣用クエリの典型的偽陽性。`type` エイリアス化は W5b だけ局所適用すると不整合で、`Query` 型を読み解く可読性も実質下がる。W5a-S が同種 type_complexity を同根拠で見送り済み。**見送り**（churn 回避）。

## 適用しなかった候補と理由（churn 回避等）

- **`bitmap_source/mod.rs:7` の module_inception（`mod bitmap_source` の自己同名内包）**: 内側 `mod bitmap_source` を改名すれば解消するが、`pub use bitmap_source::BitmapSource`（mod.rs:14）の参照パス変更を伴うモジュール構造の再編であり、「自明な重複除去」の範囲を超える純粋 churn（挙動・可読性の改善ゼロ）。クレート全域で 2 件容認されている内部 mod 命名パターンのため**不適用**（karpathy 2/3）。
- **`Brush::TRANSPARENT`（brushes.rs:29）↔ `shapes/rectangle.rs:68` の `TRANSPARENT_COLOR`（private const）の重複**: バイト同一の透明色 `D2D1_COLOR_F{0,0,0,0}` が両ファイルに存在。ただし (a) 型が異なる（`Brush::Solid(..)` 包装 vs 生 `Color` エイリアス）、(b) `rectangle.rs` の `TRANSPARENT_COLOR` は描画前クリア専用の関数外 private const で `Brush` 経由参照に置換すると `as_color().unwrap()` 等のロジック挿入を要し自己文書性が下がる。W5a-S も同型の `TRANSPARENT_COLOR` 重複を境界跨ぎ・低害として見送り済み。自己文書的かつ低害な重複であり churn が勝るため**不適用**（R5.5 GUI 域の保守的判断）。W5b-T もこの2定数を別々に特性化済み（`test_transparent_color_constant_is_fully_transparent` / `test_brush_constants`）で、集約は両テストの統合も要する。
- **`Brushes::with_foreground` / `with_background` / `with_colors` の構築パターン**: 3 コンストラクタは各々 `Brush::Solid(..)` / `Brush::Inherit` の組合せで自明・最小。共通ヘルパ抽出は単一行構築を間接化するだけで可読性を下げる（karpathy 2「単一用途の抽象化を作らない」）。W5b-T の `test_brushes_with_*` 3件が保護下だが、簡素化の価値がないため**不適用**。
- **`draw_rectangles` / `draw_bitmap_sources` の描画手続き（CreateCommandList→SetTarget→BeginDraw→…→EndDraw）**: 全面 D2D 依存・テスト保護外（W5b-T 所見1）。R5.5 によりロジック簡素化は不可。精読の結果、命名・コメント・自明重複の構造整理レベルでも改善余地のある箇所は検出されず（既に各 warn 分岐・trace が一貫した様式）、**構造整理も不要**と判断。色解決フォールバック（`as_color().unwrap_or_else(DEFAULT_FOREGROUND…)`）は唯一の抽出可能な純粋計算だが W5b-T で brushes 側に特性化済みで、システムからの抽出はシグネチャ変更を要するため対象外。

## proposals へ回した候補

- **新規記録なし**。本セルで検出した簡素化候補は「テスト保護下で挙動非破壊適用（2件）」「churn 回避で見送り（5 lint 箇所）」のいずれかに収まり、**ロジック変更・挙動変更・API シグネチャ変更を要する簡素化候補は検出されなかった**。
- 既存提案との関連（新規採番せず参照のみ）: `from_pbgra32`/`is_hit` 周辺の唯一のロジック変更候補は寸法 u32 乗算オーバーフロー（generate_alpha_mask_system, systems.rs:402-403）だが、これは W5b-T が既に **P55** として記録済み（挙動変更＝panic→スキップを伴うため V/提案領域）。本 S セルの `div_ceil` 化は当該オーバーフローとは独立（`row_bytes` 計算であり `stride*height` バッファ計算ではない）で、重複記録しない。

## S6（karpathy-guidelines）適合確認

- 適用2件はいずれも「冗長な手書きを標準 API/derive へ置換」のみで、新規抽象・投機的柔軟性・不要なエラー処理の追加はゼロ（rule 2「最小コード」）。net −5 LOC。
- 変更は自分の作った orphan（手動 `impl Default`）の除去に限定し、隣接コード・コメント・整形の「改善」には踏み込んでいない（rule 3「外科的変更」）。derive 移行は sibling 慣習へのトレースが明確。
- 成功基準: 「S2 全量がベースライン 1486/0 と一致＝挙動非破壊」かつ「focused widget 72/0 で回帰検知器が緑」を満たした（rule 4「検証可能な目標」）。

## verification (S2)

- BEFORE: 親検証済みベースライン（クリーンツリー・1486 passed / 0 failed、HEAD=7675e30 W5b-T コミット）を信頼し省略（親指示「BEFORE S2 は省略可」・design フェーズ0 規定に従う）。
- AFTER: `cargo build --workspace` 成功（exit 0）/ `cargo test --workspace` **1486 passed / 0 failed**（ignored 32、全 `test result` 行の合算で実測。FAILED result 行ゼロ）。
  - ベースラインと**完全一致**（passed 1486 / failed 0、増減ゼロ）＝テストの追加・変更・削除なしで全既存テストが簡素化後コードをそのまま通過＝挙動非破壊の裏付け。
  - 反復検証: `cargo test -p wintf --lib widget::` で **72 passed / 0 failed**（W5b-T の回帰検知器 18 件を含む widget モジュール全件が緑。内訳 brushes 14・shapes 4・bitmap_source 29・他 25）。
- 変更ファイル: `crates/wintf/src/ecs/widget/brushes.rs`（+2/−7）・`crates/wintf/src/ecs/widget/bitmap_source/alpha_mask.rs`（+2/−2）の 2 ファイルのみ。`git diff --numstat` で実測（合計 +4/−9、net −5 LOC）。boundary 内に収束。tests/・他 widget サブモジュール不変。

## flaky

- 既知フレーキー `cue_performance_test::bench_pop_ready_empty_queue`（W5b 境界外 `tests/ecs` の timing ベンチ）: `cargo test --workspace` 全量実行のうち1回で `test result: FAILED` 行が1本観測されたが、(a) 同一全量実行内で当該テストは `... ok` を出力しており、(b) 隔離再実行 `cargo test -p wintf --test ecs cue_performance_test` で **5 passed / 0 failed**（`bench_pop_ready_empty_queue` 含む）と安定合格、(c) 本セルの変更は純粋 αマスク計算と `Brush` の Default derive のみで cue キュー timing と無関係。負荷由来の timing ばらつきであり回帰ではない。flaky 判定によりゲート通過（親指示の隔離再実行プロトコルに準拠）。

## clippy（S3・記録のみ・非ブロッカー）

- 本セルで解消した lint（before → after、`cargo clippy -p wintf --lib` 実測）:
  - `manual_div_ceil`: **3 → 0**（alpha_mask.rs:35/:73 の boundary 2 箇所を解消。wintf lib 全体でゼロに）
  - `derivable_impls`: **8 → 7**（brushes.rs:85 の `Brush` を解消。残 7 は boundary 外）
- 本セルで意図的に**据え置いた** boundary lint（churn/構造変更回避、件数不変）:
  - `type_complexity`: **31 → 31**（rectangle.rs:128・systems.rs:139 は未変更）
  - `collapsible_if`: **71 → 71**（systems.rs:349 は未変更）
  - `module_inception`: **2 → 2**（bitmap_source/mod.rs:7 は未変更）
- AFTER の boundary clippy 出力に `widget/brushes.rs` と `widget/bitmap_source/alpha_mask.rs` は**一切現れない**（標的2 lint を完全解消）。**新規警告の導入はゼロ**。S3 規定によりブロッカーとせず記録に留める。

## RED フェーズ代替の検証

本セルは非挙動変更（簡素化）のため RED は N/A。挙動非破壊の検証は W5b-T が追加した既存特性化テスト（`test_brush_default_is_inherit`・alpha_mask の `row_bytes` 経路被覆テスト群）を回帰検知器とし、簡素化前後で S2 全量が 1486/0 で一致することをもって実証した。両適用とも clippy 自身が等価変換と判定する simplification 系 lint の解消であり、適用後 focused 72/0 + 全量 1486/0 が初回実行で緑（挙動差分ゼロ）。
