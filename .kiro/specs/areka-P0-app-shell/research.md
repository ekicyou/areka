# ギャップ分析: areka-P0-app-shell

> **調査日**: 2026-07-05 / **言語**: ja / **対象**: `crates/areka`（main.rs 骨格化＋モックデモの example 保全）
> 本書は要件（確定済み）と既存コードの差分を分析し、設計フェーズの判断材料を提供する。**決定ではなく選択肢**を提示する。

## 1. 現状調査（Current State）

### 1.1 対象クレート構造

`crates/areka/src/` の実体（`Glob` 実測）:

| ファイル | 帰属 | app-shell での扱い |
| --- | --- | --- |
| `main.rs`（500行・混成） | モック UI＋骨格＋shiori モジュール宣言 | **分割対象**（モック UI→example／骨格＋宣言は残置） |
| `shiori_host.rs` | 本物の資産（completed shiori 系） | **残置**（Req5） |
| `shiori_session.rs` | 同上 | **残置**（Req5） |
| `reference_brain.rs` | 同上（`#[implement(IShiori)]`＋C 入口 `shiori_factory`） | **残置**（Req5） |
| `shiori_demo.rs` | env-gate 実走デモ（`AREKA_SHIORI_DEMO`） | **残置**（呼び口は骨格 main に残す・Req5.3/5.4） |
| `shiori_e2e_tests.rs` | `#[cfg(test)]` e2e | **残置**（`crate::shiori_host`/`shiori_session` のみ参照） |
| `shiori_lifecycle_e2e_tests.rs` | 同上 | **残置** |
| `shiori_reference_e2e_tests.rs` | 同上 | **残置** |
| `tests.rs` | `#[cfg(test)]` **モック UI のユニットテスト** | **モック UI と一緒に移設**（下記 3.1・最大の非自明点） |

`crates/areka/examples/` は `clickthrough_two_rects.rs` のみ（退避先パターンの前例）。

### 1.2 現 main.rs の混成構造（実測）

- **モック UI 部分**（example へ退避すべき塊）:
  - 定数: `BALLOON_OFFSET_X/Y`（335/0）・`SHELL_INITIAL_X/Y`（400/200）・`SHELL_IMAGE_PATH`（`shell/base.png`）・`BALLOON_TEXT`（「ぱすた」詩文）。
  - マーカー: `ShellWindowMarker` / `BalloonWindowMarker`。
  - 生成関数: `create_shell_window` / `create_balloon_window` / `build_typewriter_tokens` / `run_setup`。
  - システム: `register_click_through_windows`（`Added<WindowHandle>` で shell/balloon をクリック透過機構へ登録）。
  - ハンドラ: `on_shell_drag`（バルーン追従・`SetWindowPosCommand`）・`on_shell_pressed`（ダブルクリック→全窓 despawn）。
  - `main()` 内の窓生成結線（`world.spawn(run_setup)`・`add_systems(FrameFinalize, register_...)`・操作ガイド `println!`）。
- **骨格に残すべき塊**:
  - `human_panic::setup_panic!()`（Req2.3 パニックハンドラ）。
  - tracing subscriber 初期化（`EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))`＝RUST_LOG フォールバック・Req2.1/2.2）。
  - `WinApp::new()?` ＋ `mgr.run()?`（Req2.4 UI ランタイム起動）。
  - `shiori_demo::run_demo_if_enabled()` 呼び口（env-gate・失敗しても継続・Req5.3/5.4）。
  - shiori モジュール宣言 `mod shiori_host;` 等 5本＋`#[cfg(test)] mod shiori_*_e2e_tests;` 3本（Req5.1/5.2）。
- **`#[cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`**（1行目）: リリースビルドでコンソール窓を抑止する属性。骨格に必要か、example に必要かは設計判断（下記 6-DD3）。

### 1.3 依存関係の実測（Cargo.toml）

現 deps: `wintf` / `shiori-abi` / `windows-core` / `human-panic` / `thiserror` / `tracing` / `tracing-subscriber` / `async-io` / `bevy_ecs` / `windows`。
**`areka-parsers` への依存は無い**。→ 構成入力（ghost/balloon root path）の解決は `areka-parsers::package::resolve` を呼ばず**骨格内で自己完結**しなければ Req6.1（新規依存なし）に反する（3.2 で詳述）。

### 1.4 ステアリングからの制約

- **logging.md**（`fileMatch **/*.rs`）: subscriber 初期化の正典パターンは現 main.rs と**同一**（`EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))`）。ライブラリは subscriber を初期化せず、アプリ／example が初期化。→ **example 側も独自に subscriber 初期化が必要**（`clickthrough_two_rects` も実際そうしている）。
- **tech.md**: Rust 2024・tokio 禁止・依存は列挙済みのもの。骨格追加で新規 crate を引かない方針と整合。
- **記憶 `areka-runtime-env-naming`**: 本番ランタイムが読む env 変数は `AREKA_` 冠必須・ドメイン語命名。→ 構成入力を env で受ける設計を採るなら `AREKA_GHOST_ROOT` 等（ただし要件は「起動時引数 or 既定」であり env 経路は必須でない・6-DD2）。`shiori_demo` の `AREKA_SHIORI_DEMO` は既にこの規約に従う。
- **記憶 `areka-placement-real-ghost-first`／`window-placement-dpi-...`**: 骨格は**窓を作らない**（座標・配置ロジックを持たない・Req2.5）＝window-placement リジェクトの再発防止。
- **roadmap.md**: app-shell はアプリ組み上げ三段の第一段。ghost-setup が「差し込み口」にエンジン結線を実装（第二段）・emo2-conformance-e2e が適合証明（第三段）。boot/close 発火順序＝kanade・永続化＝position-persist。**M2 送りの口（SSTP/FMO/Plugin/選択 UI）は骨格に持たない**。

## 2. 要件→資産マップ（Requirement-to-Asset Map）

| 要件 | 技術的必要物 | 既存資産 | ギャップ種別 |
| --- | --- | --- | --- |
| **R1** モックデモの example 保全（挙動不変） | 現 main.rs のモック UI 全体＋ユニットテストを `examples/mock-shell.rs` へ機械的移設 | main.rs のモック UI・`tests.rs`・`clickthrough_two_rects` 前例 | **Constraint**（挙動不変が受入基準・純機械的移設） |
| **R1.6** 名前指定でビルド・実行可 | `[[example]]` 登録（or 慣約 `examples/*.rs` 自動認識） | Cargo.toml に既存 example 登録は**無い**（自動認識に依存） | **Unknown**（`name = "mock-shell"` を明示登録すべきか・6-DD4） |
| **R2.1/2.2** 構造化ロギング＋RUST_LOG フォールバック | tracing subscriber 初期化 | 現 main.rs の init コードそのまま | **なし**（既存パターン流用） |
| **R2.3** パニックハンドラ | `human_panic::setup_panic!()` | 現 main.rs にあり | **なし** |
| **R2.4** UI ランタイム起動 | `WinApp::new()` ＋ `mgr.run()` | wintf 既存 API | **なし** |
| **R2.5** 骨格は窓を作らない | 窓生成・座標定数を骨格から除去 | モック UI を除去すれば自動達成 | **なし**（R1 と裏表） |
| **R3.1-3.4** ghost/balloon root path 解決＋ログ | 引数パース（`std::env::args`）＋既定パス＋`tracing::info!` | **既存の解決ロジックは無い**（骨格の新規責務） | **Missing**（新規・自己完結必須・6-DD1/DD2） |
| **R3.5** 実行時選択 UI 無し | 何も作らない（口を持たない） | — | **なし**（非機能・作らないことが達成） |
| **R4.1** 未結線で正常終了 | 構成解決→ログ→正常終了する制御フロー | — | **Missing**（骨格の制御フロー設計・6-DD5） |
| **R4.2** 空の接続点 | ghost-setup が差し込む関数 1個の空実装 or feature 分岐 | — | **Missing**（形の選択・6-DD5） |
| **R4.3** エンジン等を実装しない | 中身を書かない | — | **なし**（境界宣言） |
| **R5.1/5.2** shiori チェーン残置＋e2e green | 5モジュール宣言＋3 e2e テスト宣言を骨格 main.rs に維持 | `shiori_*` は `crate::shiori_host` 等しか参照せず（実測）モック UI 非依存＝分離クリーン | **Constraint**（帰属維持・移動禁止） |
| **R5.3/5.4** shiori_demo env-gate 不変 | `run_demo_if_enabled()` 呼び口を骨格 main に残す | 現 main.rs にあり（`AREKA_SHIORI_DEMO`） | **なし**（呼び口据え置き） |
| **R6.1** 新規依存なし | 構成解決を std のみで実装（areka-parsers 非依存） | 現 deps に areka-parsers 無し | **Constraint**（自己完結必須） |
| **R6.2/6.3** 既存テスト green＋デモ挙動等価 | 移設後もテストが通る／デモ観測不変 | tests.rs（モック UI テスト）・shiori e2e | **Constraint**（回帰ゲート） |

### ギャップ要約
- **Missing（新規実装）**: 構成入力の解決（R3・引数＋既定＋ログ）／未結線正常終了の制御フロー（R4.1）／空の接続点の形（R4.2）。いずれも小粒。
- **Constraint（守るべき制約）**: モック UI＋そのユニットテストの純機械的移設で挙動・テスト不変（R1/R6.2/R6.3）／shiori チェーン残置（R5）／新規依存なし＝構成解決は std 自己完結（R6.1）。
- **Unknown（設計で決める）**: example の Cargo.toml 明示登録要否／構成入力の受け口（引数のみ or env 併用）／既定パスの実体／`windows_subsystem` 属性の帰属。

## 3. 主要な技術論点（詳細）

### 3.1 【最重要】`tests.rs`（モック UI ユニットテスト）の帰属

`src/tests.rs` は現 main.rs の `#[cfg(test)] mod tests;` で結線され、`use super::*;` で **モック UI 関数（`create_shell_window`/`create_balloon_window`/`build_typewriter_tokens`/`run_setup`/`on_shell_drag`/`on_shell_pressed`）とマーカーをテストする**（約25ケース）。`Grep` 実測でこれら関数の参照は `main.rs` と `tests.rs` の 2ファイルのみ。

→ モック UI を example へ移すと、`tests.rs` の対象シンボルが骨格 main.rs から**消える**。したがって `tests.rs` は**モック UI と一緒に example 側へ移設**しなければならない（example 内 `#[cfg(test)] mod` として同居、or example の inline `#[cfg(test)]`）。骨格 main.rs に取り残すとコンパイル不能＝R6.2 違反。

**留意**: Cargo は `examples/*.rs` の `#[cfg(test)]` を `cargo test` で**ビルド・実行しない**（examples のテストはデフォルトで走らない。`cargo test --examples` はドキュメントテスト系の扱いで、example 内 `#[test]` は通常のテストハーネスに載らない）。→ **モック UI のユニットテストを CI の緑判定に載せ続けたいか**が設計判断（6-DD6）。載せたいなら「モック UI ロジックを example と共有ライブラリ（`src/` の pub モジュール）に切り出し、テストは lib 側に置く」等の選択肢が要るが、これは「モック固有アセット・座標定数を本番骨格コードへ持ち込まない」（R1.5）と緊張関係にある。→ **設計フェーズで明示的にトレードオフを裁く必要**。

### 3.2 構成入力（ghost/balloon root path）の解決と「新規依存なし」

- `areka-parsers::package::resolve`（`crates/areka-parsers/src/package/resolve.rs`）は `resolve(ghost_root, default_encoding)` でマウントを解決する既存資産だが、これは **ghost-setup の領分**（descript.txt を読んでマウント）。骨格の R3 は「root **path** を解決してログに出す」だけ＝**パスの決定**であってマウントではない。
- areka は現状 `areka-parsers` に依存していない。R3 のためにここで依存追加すると R6.1（新規依存なし）違反。→ **構成解決は `std::env::args` ＋ 既定パス定数の std 自己完結**で実装するのが素直。
- 既定パスの実体は brief で「引数 or 既定 fixture パス／ukadoc 上ハードコード/引数で正当」とされる。既定を `CARGO_MANIFEST_DIR` 相対の fixture にするか、CWD 相対にするか、実在チェックの有無は設計判断（6-DD1）。**パスの存在検証まで骨格が担うか**も論点（存在しなくてもログして正常終了できる＝R4.1 と整合するが、UX 上は warn したい）。

### 3.3 shiori チェーンの分離クリーンさ（確認済み）

`shiori_e2e_tests.rs` 等は `shiori_abi` と `crate::shiori_host`/`crate::shiori_session` のみを参照（`Grep` 実測）。モック UI シンボルへの依存は**ゼロ**。→ shiori 5モジュール＋3 e2e テストの宣言を骨格 main.rs に残すだけで R5.1/5.2 が満たせる。移設は純粋にモック UI 側だけで完結する（相互汚染なし）。

### 3.4 クリック透過登録システムの帰属

`register_click_through_windows` は shell/balloon マーカー付き窓を機構へ登録する。これは**モック UI 専用**（骨格は窓を作らない）＝example へ移設。骨格側には残さない（Req2.5）。`clickthrough_two_rects` example も同型のシステムを自前で持つ前例あり。

### 3.5 `windows_subsystem` 属性

現 main.rs 冒頭 `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` はクレート属性（`#![...]`）。骨格 main.rs に残るが、モックデモ example もリリース時にコンソール窓を出したくないなら example 側にも同属性が要る（example のクレート属性は example ファイル冒頭 `#![...]` で付与）。前例 `clickthrough_two_rects` はこの属性を持たない（＝コンソール窓が出る）。→ mock-shell example に付けるか否かは「挙動不変」の解釈次第（6-DD3）。

## 4. 実装アプローチ選択肢

### Option A: 純機械的移設（モック UI 一式＋そのテストを example へ、骨格は現 main.rs から引き算）
- **内容**: モック UI コード＋`tests.rs` を `examples/mock-shell.rs`（＋その `#[cfg(test)]`）へ移す。骨格 main.rs は「属性＋shiori 宣言＋subscriber＋panic＋WinApp＋demo 呼び口＋構成解決＋正常終了」に純化。
- **トレードオフ**: ✅ 挙動不変を最も直接的に保証（R1/R6.3）✅ 分離がクリーン（shiori 非汚染は実測済み）✅ 最小変更 ❌ モック UI ユニットテストが `cargo test` の緑判定から外れる可能性（3.1・6-DD6）❌ 骨格の R3/R4 は新規に書く（が小粒）。
- **効果**: R1・R2・R5 を素直に達成。**推奨の土台**。

### Option B: モック UI ロジックを共有 lib へ切り出し（example は薄い実行体・テストは lib に残す）
- **内容**: `src/lib.rs` に mock UI のロジック（token 構築・Entity 構築）を pub で置き、example と `#[cfg(test)]` テストが lib を使う。
- **トレードオフ**: ✅ モック UI ユニットテストが従来通り `cargo test` で走る（R6.2 の「既存の緑テスト維持」を厳格解釈するなら有利）❌ R1.5（モック固有アセット・座標定数を**本番骨格コードへ持ち込まない**）と正面衝突＝mock 資産が src に居座る ❌ バイナリ＋lib の二重構成化で骨格の器の純度が落ちる ❌ 07-05 window-placement リジェクトの教訓（mock を本番コードへ持ち込まない）に逆行。
- **効果**: テスト継続性を買う代わりに境界純度を売る。**非推奨だが「既存テスト維持」の解釈を厳しく取るなら検討対象**（6-DD6 の裁定次第）。

### Option C: ハイブリッド（モック UI は example へ、ただし GUI 非依存の純ロジック（`build_typewriter_tokens` 等）だけ example 内 `#[cfg(test)]` で保持し実行対象化を design で確認）
- **内容**: Option A を基本に、モック UI テストは example 同居の `#[cfg(test)]` として保全（コードとして残す＝資産喪失は防ぐ）。CI 実行対象にするかは `cargo test --examples` 挙動を design で確認して裁く。
- **トレードオフ**: ✅ 挙動不変＋境界純度（A の利点）✅ テストコード資産は喪失しない ❌ example 内テストの CI 実行有無が Cargo 挙動依存で要検証（Research Needed）。
- **効果**: A と B の中間。**「テストはコードとして残すが実行対象化は Cargo 挙動を確認して決める」現実解**。

## 5. 工数・リスク

- **効果規模**: **S（1〜3日）**。純機械的移設（前例 `clickthrough_two_rects` あり）＋小粒な骨格新規コード（構成解決・正常終了・空接続点）。新規依存なし・新規パターンなし。
- **リスク**: **Low**。根拠: (a) shiori チェーンとモック UI の分離が実測でクリーン（相互参照ゼロ）／(b) subscriber・panic・WinApp は既存パターンそのまま／(c) 窓を作らない＝window-placement のような DPI 座標系の落とし穴に触れない。**唯一の非自明点は `tests.rs` の移設先とテスト実行対象性**（3.1）＝設計で裁けば低リスク。

## 6. 設計フェーズへの申し送り（設計判断項目）

要件ディスカッションで確認・裁定すべき論点（番号は DD = Design Decision）:

1. **DD1: 既定 root path の実体と検証**: ghost/balloon の既定パスをどう定めるか（`CARGO_MANIFEST_DIR` 相対 fixture／CWD 相対／絶対）。パス実在チェックを骨格が行うか（不在時 warn してなお正常終了 vs 何もしない）。
2. **DD2: 構成入力の受け口**: 起動時引数のみ（`std::env::args`）か、env 変数（`AREKA_GHOST_ROOT` 等・`AREKA_` 冠規約）も併用か。要件は「引数 or 既定」で env を要求しないが、`AREKA_SHIORI_DEMO` 前例あり。引数フォーマット（位置引数 vs `--ghost`/`--balloon` フラグ）。
3. **DD3: `windows_subsystem` 属性の帰属**: mock-shell example にもリリース時コンソール抑止属性を付けるか（現 main.rs は付与・前例 `clickthrough_two_rects` は非付与）。「挙動不変」の解釈。
4. **DD4: example の Cargo.toml 明示登録**: `[[example]] name = "mock-shell"` を明示するか、`examples/mock-shell.rs` の自動認識に委ねるか（R1.6 の「名前で指定して実行」を満たす最小形）。
5. **DD5: 空の接続点の形**: ghost-setup が差し込む口を「空実装の関数 1個」にするか「feature 分岐」にするか「呼び口コメント＋TODO」にするか。骨格単体で正常終了（R4.1）を壊さない最小形。
6. **DD6【最重要】: モック UI ユニットテスト（`tests.rs`）の帰属と実行対象性**: (a) example 同居 `#[cfg(test)]` としてコード保全（Option A/C）か、(b) lib 切り出しで `cargo test` 実行継続（Option B・ただし R1.5 と緊張）か。`cargo test --examples` が example 内 `#[test]` を実行するかを design で実測確認。R6.2「既存の緑テストを緑のまま維持」をテスト**実行**の維持と解するか、テスト**コード資産**の維持と解するかの裁定が要る。
   - **【要件ディスカッション #1 で裁定済み（解釈①採用・2026-07-05）】**: R6.2 の緑判定対象は **SHIORI 契約チェーンの e2e テスト群**とする。モック UI ユニットテストは **テストコード資産として保全すれば足り**、`cargo test` での実行継続は厳格に要求しない（R1.5「mock 資産を本番骨格へ持ち込まない」＋07-05 window-placement リジェクト教訓を優先）。→ **Option B（lib 切り出し）は棄却**。残る設計判断は「テストコードの移設先（example 同居 `#[cfg(test)]` の形）と、`cargo test --examples` 実行対象化の可否の実測確認」に限定＝Option A を土台に C を検討。requirements.md R6.2/R6.3 に明確化済み。
7. **DD7: 骨格の正常終了経路**: 未結線時、`mgr.run()`（ブロッキングメッセージループ）を呼ぶか、呼ばず構成ログ後に即 return するか。窓を作らないなら `mgr.run()` は即空ループ／即終了になる想定だが、UI ランタイム起動（R2.4）と正常終了（R4.1）の両立を design で確定。
