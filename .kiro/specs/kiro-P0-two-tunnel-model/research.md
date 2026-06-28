# Gap Analysis & Research — kiro-P0-two-tunnel-model

---

| 項目 | 内容 |
|------|------|
| **Feature** | kiro-P0-two-tunnel-model |
| **Type** | プロセス支援仕様（前例: `completed/kiro-P0-roadmap-management`） |
| **分析日** | 2026-06-28 |
| **Language** | ja |
| **要件状態** | requirements-generated（未承認・本ギャップ分析が要件ディスカッションへ供給） |

> 本書は要件と既存コードベースの**ギャップ分析**であり、最終決定ではなく**選択肢と論点の提示**である（kiro-validate-gap 原則: Information over Decisions）。要件の付録 C で design へ委譲された 3 論点（CI 依存方向ガード実装 / pilot worktree ライフサイクル / テンプレ example の形）に、既存コードベースの実地調査で根拠を与える。

---

## Summary（3-5 bullets）

- **本仕様は技術機能ではなくプロセス支援仕様**。成果物は steering 文書（Markdown）＋新クレート `crates/pilot`（Rust・葉ノード）＋CI 設定＋workflow 統合の 4 種。前例 `kiro-P0-roadmap-management` と同型だが、**Rust クレート新設と CI 新設**が加わる点で重量級。
- **CI は現在ゼロから新設**。`.github/workflows/` も `deny.toml` も `xtask` も存在しない。要件 4（examples ビルド＋依存方向ガード）は「既存 CI に統合」と書くが、統合先の CI 自体が無く、**CI 基盤の bootstrap が事実上の前提作業**になる（要件のスコープ拡大ではなく、設計で扱うべき隠れ依存）。
- **`crates/pilot` の構造は既存パターンで実現可能**。葉ノード（`shiori-abi` が `publish=false` の先例）＋`examples/<dir>/main.rs`（`wintf/examples/taffy_flex_demo/main.rs` が「サブフォルダ＋main.rs」の実証済み前例）。Cargo の `[[example]]` 明示登録 vs 自動検出の選択が設計論点。
- **🚩 重大リスク: ワークツリーで `cargo metadata`/`cargo build`/`cargo-deny` が即失敗する**。`[patch.crates-io] pasta_core = vendors/pasta/...` のサブモジュールが worktree で未populate のため。CI と依存ガードは **`git submodule update --init --recursive` を先行**しないと一切動かない（既知メモリ `harness-shell-quirks` と一致・本調査で実地再現）。
- **依存方向ガードの実装候補は複数現実的**。`cargo-deny`（0.19.9 が当環境に導入済・`bans.deny` で edge 禁止）／`cargo metadata` + 自前スクリプト／`xtask`。go ゲート記法（`_Depends(confirmed): pilot`）は spec.json/roadmap.md の既存 `dependencies` 配列との整合が論点。

---

## 1. 既存コードベースの現状（Current State）

### 1.1 ワークスペース構成

ルート `Cargo.toml`: `members = ["crates/*"]`, `resolver = "2"`, `edition = "2024"`, workspace 全体 `publish = false`（ただし各クレートで個別上書き）。

| クレート | publish | 役割 | 依存（抜粋） |
|----------|:-------:|------|--------------|
| `areka` | **true** | アプリ統合（bin） | wintf, shiori-abi, windows, bevy_ecs |
| `wintf` | **true** | Windows UI 基盤（lib） | dola, bevy_*, windows, taffy。`examples/` 多数 |
| `dola` | **true** | 演出定義（lib） | serde, interpolation, rand, **pasta_core** |
| `shiori-abi` | **false** | SHIORI ABI（最小依存 lib） | windows-core, windows, thiserror |

出荷グラフ（依存方向）: `areka → wintf → dola → pasta_core(patched)`、`areka → shiori-abi`。**現状の葉ノード（誰にも依存されない末端）= `areka`（bin）**。`pilot` を新設すれば、誰からも依存されないもう一つの葉ノードになる。

### 1.2 「葉ノード `publish=false`」の先例 = `shiori-abi`

要件 2.2（`publish = false`）・2.3（葉ノード）・NFR-2（最小依存・32bit 可搬性）は、**`shiori-abi` がほぼそのまま範例**。`shiori-abi` は `publish = false`・最小依存（windows-core/windows/thiserror のみ）で確立済み。`pilot` は同じ規律を踏襲できる。ただし `shiori-abi` は「他から依存される ABI クレート」である一方、`pilot` は「誰からも依存されない」点が逆で、ここを CI で機械保証するのが要件 4-2/4-3 の核心。

### 1.3 `examples/<dir>/main.rs` の実証済み前例 = `wintf/examples/taffy_flex_demo/`

要件 2.4（`examples/<spec-name>/{main.rs, README.md}`・1 仕様 1 フォルダ）は、**`wintf/examples/taffy_flex_demo/`（`main.rs` + `diagnostics.rs` 等の複数ファイル構成）が既に同型**。Cargo の examples は「単一 `foo.rs`」と「`foo/main.rs`（サブディレクトリ）」の両方を自動認識する。`pilot/examples/<spec>/main.rs` は追加設定なしで `cargo build --examples` / `cargo run -p pilot --example <spec>` が成立する見込み（要件 3.4 の実行法と一致）。

> ⚠️ 注意: example のサブフォルダ自動検出で `main.rs` 以外のファイル（`README.md`）は Cargo に無視されるため共存に問題はない。ただし `<spec>` フォルダに `main.rs` が無いと example として認識されない → CI の腐敗検出（要件 4-1）が「README だけのフォルダ」を見逃す可能性。テンプレ規約で `main.rs` 必須を担保する必要がある（設計論点）。

### 1.4 CI は現状「存在しない」

`.github/` ディレクトリ自体が無い。`deny.toml` 無し。`xtask` クレート無し。`Makefile`/`justfile` 無し。**つまり要件 4「既存 CI ワークフローに統合」の“既存 CI”が存在しない。** リモートは GitHub（`github.com/ekicyou/areka`）で GitHub Actions が自然な選択だが、ワークフロー新設そのものが本仕様の実装範囲に含まれる（要件 4-5 の文言「既存ワークフローに統合」は、CI bootstrap を含意すると解釈するのが妥当）。

### 1.5 ツールチェーン実地確認

- `cargo 1.96.0` / `rustc 1.96.0`（2026-05-25）。`cargo metadata --format-version 1` 動作（`--no-deps` なら patch 未解決でも packages は取れる）。
- **`cargo-deny 0.19.9` が当開発環境に導入済**（要件 4 の依存ガード候補として即利用可能だが、CI ランナーには別途インストールが要る）。

### 1.6 spec.json / roadmap.md の既存「依存」表現

- `kiro-spec-schema.md`: 子仕様に `dependencies: ["other-spec"]`（feature_name の配列）と `tier`/`priority` が定義済み。**go ゲート（`_Depends(confirmed): pilot`）という「確定前提依存」概念は現状スキーマに無い** → 要件 6-4 は schema 拡張 or roadmap 記法の新設を要する。
- `roadmap.md`: `Dependencies: <spec>, <spec>` を**自由テキストで**各 spec 行に併記する現行慣行あり（例: `areka-P0-shiori-host-32 -- ... Dependencies: areka-P0-shiori-com, areka-P0-shiori-reference`）。go ゲート種別（pilot/main 区別・confirmed 種）を**この自由テキスト記法の拡張**として乗せるのが最小コスト（roadmap.md は `inclusion: manual`・要件の Adjacent expectations で「拡張余地を持つ」と明記済）。

### 1.7 workflow.md の現状（拡張対象）

`inclusion: always`・約 200 行。ブランチ＆マージ戦略（PR ベース・main 直 push 禁止）、完了時アクション（DoD ゲート → archive → PR squash）、タスク完了アクション、仕様フェーズフロー（`discovery → start → design → tasks → implementation → complete`）を持つ。要件 8 は**この末尾/中間に二坑フェーズ・go ゲート・依存マップ検証・削除/隔離規律を上乗せ**する（要件 8-5: 既存ブランチ/完了規約は不変）。**`inclusion: always` ゆえコンテキスト常駐コスト**があり、要件 1-5（詳細を別文書へ委譲）の動機と直結 → 二坑詳細は別 steering 文書に切り出し、workflow.md からは参照に留めるのが整合的（要件 1-5 と 8 の協調設計論点）。

---

## 2. 要件→アセット対応マップ（gap tags: Missing / Reuse / Constraint）

| 要件 | 必要な技術要素 | 既存アセット | Gap |
|------|----------------|--------------|-----|
| R1 二坑 steering 文書化 | `.kiro/steering/` 新規 Markdown ＋ workflow からの参照 | steering 群・`focus.md` の lean ポインタ慣行 | **Reuse**（前例 roadmap-management の steering 確立型） |
| R1-5 詳細委譲でコンテキスト抑制 | `inclusion: manual` 分離文書 | `roadmap.md`（manual）の前例 | **Reuse** |
| R2 `crates/pilot` 新設・葉/publish=false | 新 Cargo.toml・workspace member | `shiori-abi`（publish=false 葉）, `members=["crates/*"]` 自動取り込み | **Reuse**（範例あり） |
| R2-4 `examples/<spec>/{main,README}` | サブフォルダ example 規約 | `wintf/examples/taffy_flex_demo/main.rs` | **Reuse**（実証済） |
| R2-6 テンプレ example | 雛形 main.rs ＋ README 雛形 | なし | **Missing**（新規作成・設計論点 C3） |
| R2-7 / NFR-2 最小依存・32bit 可搬 | 依存を持たせない | workspace 32bit 方針（roadmap/tech） | **Constraint**（pilot に重い dep を持ち込まない規律） |
| R3 README 3 幕一次記録 | Markdown 規約 | なし（新規規約） | **Missing**（規約文書化） |
| R3-6 subagent が .md 書けない代替手順 | 運用手順記述 | 既知メモリ `harness-shell-quirks`（PowerShell here-string / 親書き込み） | **Reuse**（手順は既知・文書化が gap） |
| R4-1 examples ビルド | `cargo build --examples`（pilot 対象） | cargo 動作・examples 認識 | **Constraint**（🚩 submodule init 前提・後述） |
| R4-2/3 production→pilot 依存禁止の機械検証 | 依存方向ガード | `cargo-deny` 導入済 / `cargo metadata` | **Missing**（ガード実装・設計論点 C1） |
| R4-5 既存 CI へ統合 | CI ワークフロー | **CI 不在** | **Missing**（CI 基盤 bootstrap） |
| R5 命綱・削除/隔離規律 | steering 規約 | karpathy-guidelines（add-only 抑制思想） | **Missing**（規約文書化・思想は援用可） |
| R6 ハードゲート go 依存 | spec 上の記法 `_Depends(confirmed)` | spec.json `dependencies` / roadmap 自由テキスト Dependencies | **Missing/Constraint**（記法新設 or 既存拡張・設計論点 C4） |
| R7 依存マップ重点検証 | 被覆/孤児/DAG/合否基準の検証手順 | roadmap の依存順記述・`/kiro-spec-batch` の wave 概念 | **Missing**（検証ルール文書化） |
| R8 workflow 統合 | workflow.md 拡張 | workflow.md（拡張対象・always） | **Reuse/Constraint**（不変部尊重・常駐コスト） |
| NFR-4 completed/ 不変 | 改変しない規律 | `completed/kiro-P0-roadmap-management` | **Constraint** |

---

## 3. 🚩 重大リスク: ワークツリーでの submodule 未populate

### 事象（本調査で実地再現）
worktree 上で `cargo deny list` を実行すると即失敗:
```
[ERROR] failed to gather crates: `cargo metadata` exited with an error:
error: failed to load source for dependency `pasta_core`
  unable to update .../vendors/pasta/crates/pasta_core
```
原因: ルート `Cargo.toml` の `[patch.crates-io] pasta_core = { path = "vendors/pasta/crates/pasta_core" }`。git worktree では submodule が自動 populate されないため、patch 先のパスが空。

### 影響（設計に必須の制約）
- 要件 4-1 `cargo build --examples`、4-2/4-3 の `cargo metadata` ベース依存ガード、`cargo-deny` の**いずれも submodule 未init では一切走らない**。
- CI ジョブと、ローカル/worktree 双方で、**`git submodule update --init --recursive` を依存ガード/ビルドの前段に必須化**する必要がある。既知メモリ `harness-shell-quirks`（「git submodules NOT populated in worktrees → workspace cargo の前に init」）と完全一致。
- pilot クレート自体は最小依存（pasta 非依存）だが、`cargo metadata`/`cargo build` は**ワークスペース全体を解決**するため、pilot のビルドであっても patch 解決に巻き込まれる。→ pilot を独立解決させる回避策（後述 Option A2）も検討余地あり。

---

## 4. 実装アプローチの選択肢

本仕様は性質の異なる 4 成果物（steering / pilot crate / CI / workflow）の束であり、成果物ごとに A/B/C を評価する。

### 4.A CI 依存方向ガードの実装（要件 4-2/4-3・設計論点 C1）

| Option | 内容 | ✅ | ❌ |
|--------|------|----|----|
| **A1: cargo-deny `bans.deny`** | `deny.toml` で `pilot` クレートへの依存を禁止 edge として宣言 | 宣言的・当環境に 0.19.9 導入済・将来の他依存規律にも転用可 | CI ランナーへの別インストール要・submodule init 必須・edge 単位の禁止表現の検証要 |
| **A2: `cargo metadata` + 自前スクリプト** | metadata の resolve グラフを走査し「pilot を依存に含む出荷クレート」を検出して fail | 追加ツール不要・ロジックを完全制御・pilot 限定の独立解決も組める | スクリプト保守・JSON 走査の自作・submodule init 依然必須（metadata 全体解決のため） |
| **A3: xtask クレート** | `cargo xtask check-isolation` を新設しガードを Rust で実装 | リポジトリ内に検証ロジックを内包・ローカルでも同一実行・他 xtask へ発展可 | xtask クレート新設のコスト（本仕様スコープを広げる）・xtask 自体が workspace member |

> 補助観点（A1/A2 共通の検証強度）: 「pilot が production に依存される」だけでなく「pilot が**他クレートに依存する**ことは許容（pilot は何に依存してもよい）」を区別する必要がある。禁止すべきは **inbound edge（誰かが pilot を依存に持つ）のみ**。cargo-deny の `bans` は通常 outbound（あるクレートを誰も使うな）を表現でき、pilot への inbound 禁止＝「pilot を deny 対象に置く」で表現可能。要 PoC 確認（design 段階）。

### 4.B CI 基盤（要件 4-5・隠れ前提）

| Option | 内容 | ✅ | ❌ |
|--------|------|----|----|
| **B1: GitHub Actions 新設（最小）** | `.github/workflows/ci.yml` を新設し submodule init → `cargo build --examples` → 依存ガードのみ | リモートが GitHub・自然・将来の test job 母体になる | Windows ランナー前提（windows-rs 依存）でコスト/時間・本仕様で CI を初導入する重み |
| **B2: 依存ガードのみの軽量 job** | フル build はせず metadata 解析のみ（依存方向）＋ pilot examples build だけ別 job | 高速・スコープ最小 | フルワークスペース CI は別途要・部分的 |
| **B3: ローカル pre-commit/手動スクリプト先行、CI は後続** | まず `cargo xtask`/スクリプトをローカル規律にし、CI 化は段階導入 | 着手が軽い | 要件 4-5「自動実行」「機械的厳守」(NFR-3) を満たさない → 不採用寄り |

> 制約: windows-rs 全面依存ゆえ、production フル build は実質 Windows ランナー必須。一方 pilot は最小依存なので、**pilot examples build だけなら Linux ランナーでも通る可能性**（pilot が windows/pasta に非依存なら）。依存方向ガード（metadata 解析）は OS 非依存。→ B2 的な「OS 非依存ジョブで依存ガード＋pilot build、重い production build は別レーン」が現実的（design で切り分け）。

### 4.C steering 文書の配置（要件 1・5・8・設計論点）

| Option | 内容 | ✅ | ❌ |
|--------|------|----|----|
| **C1: 単一新規文書 `two-tunnel.md`（`inclusion: manual`）＋ workflow から参照** | 詳細を manual 文書に集約、workflow.md は短い参照節のみ追加 | 常駐コスト最小（要件 1-5）・workflow.md 肥大回避・roadmap.md の manual 前例に倣う | 参照を辿る手間・always でないと AI が読み落とすリスク |
| **C2: workflow.md に全部インライン** | 二坑規律を workflow.md 本体に書く | 1 ファイルで完結・always で常時参照 | workflow.md（always）が肥大しコンテキスト常駐コスト増（要件 1-5 に反する） |
| **C3: lean ポインタ（focus.md 型）+ manual 詳細文書の二層** | always な薄いポインタ ＋ manual な詳細、の focus.md/roadmap.md と同じ二層 | プロジェクト既存パターンと完全整合・常駐最小かつ発見性確保 | ファイル数増 |

> プロジェクトは既に「`focus.md`(always・lean) → `roadmap.md`(manual・詳細)」の二層を確立済。**C3 がプロジェクト規約と最も整合**。

### 4.D テンプレ example の形（要件 2-6・設計論点 C3）

- 最小 `main.rs`（`fn main() { println!("pilot template: replace me"); }` 程度・依存ゼロ）＋ 3 幕 README 雛形。
- 配置: `crates/pilot/examples/_template/{main.rs, README.md}`（`_` 前置で実 spec と区別・CI build 対象に含めるか除外するかは設計論点）。または `crates/pilot/templates/` に置き examples 外にする案（CI build 対象外にできる）。
- 論点: テンプレを examples 配下に置くと `cargo build --examples` がテンプレもビルド（腐敗検出に入る＝望ましい）が、`<spec>` 命名規約と混在する。examples 外に置くと build 検証外になる。

---

## 5. 工数・リスク

| 成果物 | Effort | Risk | 一言根拠 |
|--------|:------:|:----:|----------|
| R1/R5 steering 文書（二坑規律・命綱・隔離） | S | Low | 前例 roadmap-management と同型・思想は karpathy 援用 |
| R2/R3 `crates/pilot`＋README 規約＋テンプレ | S–M | Low | `shiori-abi`＋`wintf/examples` の組合せで範例完備 |
| R4 CI（examples build＋依存ガード） | **M–L** | **Medium–High** | 🚩 CI ゼロからの新設＋submodule init 必須＋依存ガード PoC＋Windows ランナー検討 |
| R6/R7 ハードゲート記法＋依存マップ検証 | M | Medium | spec.json/roadmap への記法新設・検証は文書規律（自動化は別） |
| R8 workflow 統合 | S | Low | 既存拡張・不変部尊重 |

**総合: M–L / Medium。** クリティカルは要件 4（CI）一点に集中。他は文書/範例で軽い。

---

## 6. design フェーズへ持ち越す Research / 決定事項（要件ディスカッションへの供給）

要件付録 C の 3 論点を、実地調査で具体化した上で再掲・拡張する。

1. **【最優先・C1】CI 依存方向ガードの実装手段**: A1 cargo-deny（導入済）/ A2 cargo metadata 自前 / A3 xtask の三択。inbound-edge 禁止（誰も pilot に依存しない）の表現可否を design で PoC 確認。**Research Needed: cargo-deny `bans` で「特定クレートへの被依存禁止」を表現できるか実証。**
2. **【解決済み・議題1】機械チェックの乗り物 = ローカル workflow ゲート（GitHub Actions 新設は対象外）**: 未リリース repo を GitHub CI で重くしない開発者判断。マージは本チャット駆動の `/kiro-complete` に集約されるため、隔離チェック（`cargo build --examples -p pilot` ＋ 依存方向ガード ＋ `git submodule update --init --recursive` 前段）を `/kiro-complete` の DoD ゲート（既存の `cargo test --workspace` に並置）へ統合する形で NFR-3「機械的厳守」を満たす。要件 R4 を「CI Pipeline」→「Isolation Gate（ローカル workflow 統合）」へ改稿済（R4-5/4-6/4-7・NFR-3）。リモート CI 化は repo がリリースに近づいた際の後続候補。**残る design 論点は実装手段（cargo-deny / cargo metadata / xtask）の選定のみ（下記 §6-1 / §4.A）。**
3. **【C2】pilot worktree のライフサイクル（いつ捨てるか）**: 要件外（運用詳細）だが、worktree＋submodule 制約と絡む。pilot crate は永続（隔離保全）だが、探索用 throwaway worktree の破棄タイミングは運用記述として workflow へ。
4. **【C3】テンプレ example の配置**: `examples/_template/` （build 対象・腐敗検出に入る）vs `templates/`（build 対象外）。`main.rs` 必須規約で「README だけのフォルダが example 認識されない」問題を担保。
5. **go ゲート記法の表現先**: `_Depends(confirmed): pilot` を (a) spec.json スキーマ拡張（kiro-spec-schema.md に新フィールド）／(b) roadmap.md の既存自由テキスト `Dependencies:` の拡張／(c) tasks.md の `_Requirements:` 類似記法、のどこに置くか。既存 `dependencies` 配列との二重管理回避が論点。
6. **steering 二層化**: 二坑詳細を `two-tunnel.md`（manual）に切り出し、`focus.md`/`workflow.md`（always）からは lean 参照に留める（既存 focus→roadmap 二層と整合）。要件 1-5 と 8-1 の協調設計。
7. **依存マップ検証（要件 7）の自動 vs 手動**: 被覆/孤児/DAG/合否基準を、`/kiro-spec-batch` の wave 概念と接続した自動チェックにするか、ディスカッション時の手動 checklist にするか。本仕様スコープ（モデル確立）では手動 checklist 規約が下限、自動化は後続候補。
8. **NFR-4 整合**: `completed/kiro-P0-roadmap-management` は不変。二坑規律はそれを置換せず steering 群へ追補する形を design で明示。

---

## 7. 推奨（design への申し送り・決定ではない）

- steering: **C3 二層**（既存 focus→roadmap 規約と整合）。
- pilot crate: `shiori-abi`（publish=false 葉）＋`wintf/examples/<dir>/main.rs` の合成で範例どおり。
- 機械チェックの乗り物: **ローカル workflow 完了ゲート（`/kiro-complete` DoD）に確定（議題1）**。GitHub Actions 新設は対象外。`git submodule update --init --recursive` → `cargo build --examples -p pilot` → 依存方向ガード を DoD ゲートに統合（既存 `cargo test --workspace` に並置）。
- ガード手段（design 送り）: 当環境に `cargo-deny` 導入済ゆえ A1 を第一候補に PoC、不適なら A2（cargo metadata 自前）へフォールバック。乗り物がローカルゆえ「CI ランナーへの別インストール」懸念は当面解消（ローカル実行環境に導入済）。
- go ゲート記法: roadmap.md の既存 `Dependencies:` 自由テキスト拡張（最小コスト）を第一候補、厳密化が要れば spec.json スキーマ拡張。

---

## Next Steps

1. 本ギャップ分析を要件ディスカッション（kiro-requirements-discussion）で精査し、§6 の論点を設計判断として収集する。
2. requirements 承認後 `/kiro-design kiro-P0-two-tunnel-model` で design 生成 → §4 の Option を確定。
3. design で特に **要件 4（CI bootstrap＋依存ガード PoC＋submodule init）** を最優先に詰める。
