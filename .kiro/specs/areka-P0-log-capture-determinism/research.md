# ギャップ分析: areka-P0-log-capture-determinism

> 本書は kiro-validate-gap フェーズの成果物。確定済み要件（requirements.md）と**既存コードベースの実測**との差分を示し、設計フェーズと要件ディスカッションへ判断材料を供給する。**決定は下さない**（情報と選択肢の提示に徹する）。
> 作成: 2026-07-23 / spec.json language=ja / phase=requirements-generated

---

## 0. サマリ（3〜5 行）

- 本仕様は**単一テスト専用ファイル** `crates/areka-kanade/src/schedule/log_capture.rs`（98 行・`capture()`＋`assert_logged()`）への **~15 行の追加**でありコード境界は 1 ファイル。本番コードは不改変。既存パターン（`tracing_subscriber` Layer＋`with_default` スレッドローカル捕捉）を保ったまま、プロセスグローバル interest-keeper を前置する形の**拡張（Option A）**が素直に成立する。
- ブリーフ埋め込みの確定アプローチ（`OnceLock`＋`set_global_default(registry())`＋`rebuild_interest_cache()`）は既存構造と衝突しない: kanade の lib/integration に**他のグローバル subscriber 初期化は存在しない**（grep で確認済み）。
- 検証負荷（RED→GREEN の 4 プロセス×25 ラウンド ＋ workspace 5 回反復）と i686 前提成果物ビルドが**実装本体より重い**。決定論再現の証拠取りが本仕様の実質的コスト中心。
- Req 7（steady_test の 500-bound ループ park-barrier 化）は**同一境界の二次修正**だが、参照手本（mouse_test cage 8/9/10）とは構造が異なる（steady cage は「Tick 駆動で resume を起こす」ループであり、mouse cage の `join_bounded` 一発とは別種）。ここは設計で具体バリア設計を要する**唯一の非自明点**。
- 主要リスクは Low〜Medium。技術リスクは「bare `registry()` のグローバル常駐が capture 外で走る他テストの挙動へ副作用を持たないか」の一点（設計で確認要）。

---

## 1. 現状調査（Current State Investigation）

### 1.1 対象ファイルと構造

`crates/areka-kanade/src/schedule/log_capture.rs`（98 行・`#[cfg(test)]` 配下 pub(crate)）:

| 要素 | 役割 | 行 |
|---|---|---|
| `CapturedEvent { target, event, level }` | 捕捉 1 イベント（target/event/level のみ照合） | 24-30 |
| `EventFieldVisitor` | 構造化フィールド `event` 抽出（record_str＋record_debug 保険） | 33-50 |
| `CaptureLayer { sink: Arc<Mutex<Vec<_>>> }` | `on_event` で sink へ push する `Layer<S>` | 53-71 |
| `capture<F>(f) -> Vec<CapturedEvent>` | `registry().with(layer)` を `with_default` でスレッドローカル差込・`mem::take` で回収 | 74-85 |
| `assert_logged(events, level, event_name)` | `target=="kanade"` かつ level/event 一致を `any` で表明 | 90-98 |

**既存の PITFALL 注記**（79-83 行）: 旧 flaky（`Arc::try_unwrap` 競合・~1/10〜1/20）を `mem::take` へ切替えた修正注記が既にある。今回の欠陥はその**より深層の同族**（callsite Interest 焼き付き）であり、モジュール doc（9-13 行「決定性の要（PITFALL）」節）へ統合追記する対象。

### 1.2 呼出箇所（grep 実測・ブリーフ記載と一致）

- **`actor.rs`**: `capture` ×4（566/598/630/661 行）＋ `assert_logged` ×4（576/608/**647**/681 行）。647 行が **`Level::TRACE` の `shiori_request` 檻**（Req 3 の直接受益者）。import は 546 行 `use crate::schedule::log_capture::{assert_logged, capture};`。
- **`schedule/boot.rs`**: `capture` ×1（687 行）。630 行の `boot_greeting_talkdone_correlates_without_unknown_error` が **不在表明檻**——709 行 `events.iter().any(...)`＋714 行 `assert!(!unknown_fired, ...)`。**イベント取りこぼしで `any` が空振りし偽 PASS しうる最脆弱檻**（Req 4.2 の主対象・ブリーフの記述を実コードで確認）。
- **`schedule/mod.rs`**: `capture`/`assert_logged` 系 31 出現（capture ×6・assert_logged 約 30）。

呼出総数はブリーフの「actor ×4・mod ×6・boot ×1＝10 capture」と整合。「32 檻」は assert 系檻の総数（enumerate は設計/実装で確定・本分析では総量把握で足りる）。

**グローバル subscriber 初期化の非在**: `areka-kanade` の src/tests とも `set_global_default`／`set_default`／`init()` の既存呼出は本ファイル以外に無い（grep）。interest-keeper 前置の衝突相手は現状ゼロ。

### 1.3 二次修正対象（Req 7）

`crates/areka-kanade/tests/kanade/steady_test.rs` の `talk_completion_resumes_get_pump_ref3_one_status_none`（780 行〜）:

- 821 行 `'drive: for i in 3..=500u64 { ... }`: Tick を送りつつ、内側 `for _ in 0..64 { yield_now(); resumed_get_after_active_window(...).is_some() }`（833-839 行）で resume を polling し打ち切る**壁時計非依存だが上限依存の有界ループ**。
- 終了自体は既に 855 行 `join_bounded("kanade resume join", DEFAULT_TIMEOUT, kanade)` のバリアで駆動。**500-bound ループは「復帰後 pump を起こす Tick を供給する」ためだけに存在**する点が mouse cage と異なる。

参照手本（`mouse_test.rs`）の park-barrier イディオム:
- `spawn_harness_gated(cfg, fixture, QuitPolicy::PerTalk(...), hold_indices)` が `(Harness, SakuraGate)` を返す（common/mod.rs 1037 行）。
- `SakuraGate` は `GateInner.expected_holds`（park 総数）を持ち、releaser スレッドが「`released` かつ `parked.len() >= expected_holds`」まで待つ（common/mod.rs 883 行）——**park-count バリアで race を閉じる**。
- cage 8/9/10 は保留窓を作り、終了は `join_bounded` の quit:true talk で一発駆動（retry ループ皆無）。

**差分の要点**: mouse cage は「保留→置換→quit talk→join」で Tick 反復が不要。steady cage は「保留 talk 解放→**復帰後**の GET pump を起こすため追加 Tick が要る」。この追加 Tick 供給を上限非依存のバリアへどう写すかが設計論点（§4 の DD-7）。

### 1.4 バージョン・前提（Cargo.lock 実測）

- `tracing` 0.1.44（2433-2434 行）・`tracing-core` 0.1.36（2455-2456 行）・`tracing-subscriber` 0.3.23（2476-2477 行）——**ブリーフ／要件記載と完全一致**。固定済み。
- `tracing-core` 0.1.36 のソースは本ワークツリーの cargo registry キャッシュに**未展開**（`~/.cargo/registry/src` 走査で NOT-FOUND）。ゆえにブリーフ引用の行番号（callsite.rs:505 / dispatcher.rs:314-319 / sharded.rs:222-235）は**本分析では独立再導出していない**（タスク指示どおり再導出不要）。**内部整合性は確認**: 引用バージョンが Cargo.lock 固定値と一致し、主張する機構（live dispatcher 0 個での rebuild → `Interest::never` sticky → 素の registry の leak 済み registrar で Interest ≥ Sometimes 固定）は要件 1/2/3 の受入基準と論理的に無矛盾。**行番号の一次確認は実装/レビュー時（kiro-review A-3）に registry 展開後行う**（Research Needed R-1）。

---

## 2. 要件充足性分析（Requirements Feasibility）

### 2.1 要件→資産マップ（ギャップタグ: 追加=Missing / 不明=Unknown / 制約=Constraint）

| 要件 | 必要な技術要素 | 既存資産 | ギャップ |
|---|---|---|---|
| R1 並列決定論捕捉 | プロセスグローバル interest-keeper（一度だけ確立） | 無（capture は毎回 transient dispatcher） | **Missing**: `OnceLock`＋`set_global_default`（~15 行） |
| R1.4 workspace 5 回反復 0 失敗 | 反復検証手順・i686 前提ビルド | steering 既知（PowerShell・i686 helper/testdll） | **Constraint**: 検証コストが実装より重い |
| R2 API/意味論不変 | `capture`/`assert_logged` シグネチャ不改変 | 現行シグネチャ | ギャップ無（前置のみ・呼出 10 箇所不改変） |
| R2.4 thread-local が global を shadow | `with_default` の既存挙動 | 現行 `with_default` | **Unknown**: global 常駐下の shadow が全 capture で混在しないこと（設計確認 DD-4） |
| R3 TRACE 含む全レベル捕捉 | 素の registry は per-layer filter 無し（Interest::always） | actor.rs:647 の TRACE 檻が現存 | **Unknown**: bare registry グローバルが TRACE を落とさないこと（R-2） |
| R4 32 檻保全・不在表明檻 | 檻不改変で真動作 | boot.rs 不在表明檻（709-714 行） | ギャップ無（本修正の最大受益者） |
| R5 fail-loud（先行 global で panic） | `set_global_default` の Result を expect/明示 panic | 無 | **Missing**: expect メッセージ（DD-3） |
| R6 本番不改変・スコープ境界 | test 専用ファイルのみ変更 | log_capture.rs は `#[cfg(test)]` | ギャップ無 |
| R7 steady_test park-barrier 化 | 上限非依存バリア | mouse cage の `expected_holds` バリア（手本） | **Missing/Unknown**: 追加 Tick 供給のバリア写像（DD-7） |
| R8 RED→GREEN 証拠 | 4 プロセス×25 ラウンド stress・mtime 最新 exe 選択 | 無（手順のみブリーフ A-2） | **Missing**: 検証スクリプト/手順（DD-6） |

### 2.2 非機能・複雑度シグナル

- **信頼性**: 本仕様の主目的そのもの（確率的 flake の恒久解）。決定論テスト方針（steering: deterministic-test-coverage-mandate）に整合。
- **並行性**: `OnceLock::get_or_init` は競合安全（複数スレッド同時初回 capture でも set_global_default は 1 回）。
- **複雑度**: アルゴリズム的には単純（グローバル一度確立）。難所は**tracing 内部意味論の正しさ確認**と**確率的欠陥の証拠取り**——コード量でなく検証設計に複雑度が寄る。

---

## 3. 実装アプローチ選択肢

### Option A: 既存 log_capture.rs を拡張（ブリーフ確定案）

`capture()` 先頭で `install_interest_keeper()` を呼び、`OnceLock` で一度だけ `set_global_default(tracing_subscriber::registry())`＋`rebuild_interest_cache()`。

- **対象ファイル**: log_capture.rs のみ（~15 行追加＋module doc 更新）。
- **互換性**: `capture`/`assert_logged` シグネチャ・呼出 10 箇所・32 檻すべて不改変（Req 2/4 直接充足）。
- **トレードオフ**: ✅ 最小差分・既存パターン踏襲・境界 1 ファイル。✅ 隔離テスト容易（本ファイル内で完結）。❌ グローバル常駐の副作用検証（DD-4/R-2）が要る。

### Option B: 専用初期化フック新設

keeper を別 `pub(crate) fn init_capture_environment()` 等へ切り出し、テストハーネス側または module 初期化（`ctor`/lazy static）で確立。

- **根拠**: 「初回 capture 呼出」に依存せず、capture を通らず `step()` だけ呼ぶテストにも keeper を効かせられる。
- **トレードオフ**: ✅ 確立契機を capture から分離。❌ 新規依存（`ctor` 等）or 呼出規約の追加＝Req 1.3「capture が初めて呼び出される時に確立」の文言と**衝突**（要件変更が必要になる）。❌ 境界が広がる。**要件との齟齬ゆえ非推奨**だが、DD-4 で「capture 外の step 発火」が問題化した場合の退避路として記録。

### Option C: ハイブリッド（A ＋ Req 7 二次修正を同 PR）

Option A の keeper 導入に加え、steady_test の 500-bound ループを `expected_holds` バリアベースへ改修。

- **組合せ**: keeper（src 側・R1〜R6）＋ steady cage 改修（tests 側・R7）は**独立差分**だが同一境界（log_capture 決定論化）ゆえ同 PR が経済的（ブリーフ Scope「同 PR が経済的」）。
- **段階**: フェーズ 1＝keeper 導入＋RED→GREEN、フェーズ 2＝steady cage 改修（回帰意味論不変を維持）。
- **トレードオフ**: ✅ 潜在 flake を同一 PR で一掃。❌ steady cage のバリア設計（DD-7）が唯一の非自明作業を持ち込む。**Req 7 が「推奨」ゆえ、Req 7 を GREEN 化できない場合に A へ縮退可能**（Req 7 は分離可能と要件・ブリーフ双方が明記）。

**却下済み代替**（ブリーフ・妥当性確認済み）:
- ①テスト直列化 mutex: capture 外で `step()` を呼ぶテストが NoSubscriber 下で callsite を焼くため不十分。
- ②リトライ/bound 拡大: 確率を下げるだけで恒久解でない（Req 8 の「恒久解」要件に反する）。

---

## 4. 設計判断項目（要件ディスカッションへ供給・DD リスト）

1. **DD-1 keeper 確立契機**: 要件 1.3 は「capture が初めて呼び出される時」に固定＝Option A（lazy in-capture）を前提化。この文言を維持するか、capture 外 step テスト対策（DD-4）次第で見直すか。→ 現状は A を推奨・要件文言と整合。
2. **DD-2 `rebuild_interest_cache()` の要否**: 導入前に Never 焼き付き済みの callsite への「保険」。`tracing 0.1.44` で `tracing::callsite::rebuild_interest_cache` が公開 API か要確認（R-3）。保険を残すか、set_global_default 単独で十分かを設計で判断。
3. **DD-3 fail-loud の実装形**: `set_global_default` は `Result`。`OnceLock::get_or_init` 内での失敗は「先行する外部 global subscriber」のみ（自分は 1 回しか呼ばない）。`.expect(<原因説明>)` 一発か、`match`＋明示 `panic!` で区別メッセージを出すか（Req 5.1/5.2・log-first 規律）。
4. **DD-4 グローバル常駐の副作用範囲**: bare `registry()` が `set_global_default` 後、**capture クロージャ外**（例: spawn したアクタースレッドの `tracing::error!`、`step()` を直接呼ぶ非 capture テスト）で走るイベントに対し、on_event no-op で無害か。他テストの挙動・出力を変えないことを設計で確認（R-2 と連動）。thread-local が global を shadow する意味論（Req 2.4）の実挙動確認を含む。
5. **DD-5 module doc（PITFALL 節）更新方針**: 既存 79-83 行の旧 flaky 注記（`Arc::try_unwrap`）と 9-13 行の「決定性の要」節へ、callsite Interest 焼き付きの根本原因＋keeper 機構を統合追記する範囲・粒度。
6. **DD-6 RED→GREEN 検証の実行形態**: 4 プロセス×25 ラウンド stress（mtime 最新 exe 選択・stale 複数残存対策）を PowerShell スクリプト化するか手順記載に留めるか。RED 未再現時（~100 実行）は「記録の上 workspace 反復へ委ねる」を要件 8.1 が許容——GREEN 判定の一次証拠を workspace 5 回反復（Req 1.4/8.3）へ寄せる設計。
   - **討議#2 追記（2026-07-23）**: keeper の証拠形式（RED→GREEN ストレス）と R7' の証拠形式を分離。R7' 対象ループは integration exe 在住ゆえ lib exe ストレスの対象外・かつ飢餓由来 flake は統計再現不能（Defender 再スキャン等は制御不能）。R7' の判定は「反復上限の不在」という構造的性質のコードレビュー確認＋対象テストの回帰緑（`cargo test -p areka-kanade`／workspace 反復）＋非空虚性のレビュー観点担保とする（R8.4/8.5 に確定）。integration exe への RED→GREEN ストレスや飢餓の人工再現は偽の安心／新たな非決定性源ゆえ要求しない。
7. **DD-7 steady_test park-barrier 設計（Req 7）— ✅ 討議#1 で解決（2026-07-23）**: 追加調査により機構・範囲とも確定、要件 R7 を全面書き換え済み。
   - **確定事実**: 同型 `'drive` ループは 3 箇所（steady_test.rs:821 / close_test.rs:170 / close_test.rs:806）＋同病亜種 `wait_until`（close_test.rs:57・100,000 yield 有界）。すべて「反復上限＝時間の代用」の協調ループで、CPU 飢餓（Defender 再スキャン等・steering 既知の病）で空回りし尽くして偽赤する。
   - **park-barrier 棄却の明白な理由**: ①mouse cage の `expected_holds` バリアは release-before-park race 用で既に `SakuraGate` 実装済み（復帰系も享受済み）②復帰点（TalkDone 処理）は SHIORI 呼出を発行せず観測可能シグナルが無い（新設は本番コード変更＝R6 違反）③talk 1 も hold する案は単段 gate ではデッドロック・多段化しても Tick 供給 polling は消えず共有ハーネスの表面積だけ増える。
   - **採用機構（本質解）**: (a) 意味論的完了バリア＝quit:true talk の帰結である inbox 切断（send Err）をループ終了条件に（因果連鎖: 復帰→GET→Value→talk quit:true→終了→切断が必然）(b) 復帰表明は join 後の最終記録列で評価（ループ内観測 race 排除）(c) ハング→失敗変換は壁時計 Instant deadline（`join_bounded`/`wait_until_blocked`(common/mod.rs:561) と同系＝ハーネス内に定石実装済み）(d) `wait_until` も同 deadline 化。縮退条項は撤廃（確定要件）。
8. **DD-8 i686 前提ビルドと workspace gate**: `cargo test --workspace` 5 回反復の前に `cargo build --target i686-pc-windows-msvc -p shiori-host32-helper -p shiori-host32-testdll`（steering: workspace-test-needs-i686-host32-artifacts）。ビルド/テストは PowerShell（Git Bash の link.exe 遮蔽罠）。検証手順へ明記。

---

## 5. Research Needed（設計フェーズへ持ち越す確認項目）

- **R-1**: `tracing-core` 0.1.36 の行番号引用（callsite.rs:505 `unwrap_or_else(Interest::never)` / dispatcher.rs:314-319 の subscriber Arc leak / sharded.rs:222-235 の Interest::always・on_event no-op）を registry 展開後に一次確認（kiro-review A-3 でレビュアー自身が読む）。本ワークツリー未展開ゆえ本分析では未実施（バージョン整合は確認済み）。
- **R-2**: bare `tracing_subscriber::registry()` が per-layer filter 無しで全 callsite `Interest::always` を返し、TRACE を含む全レベルを落とさない（Req 3）ことのソース/挙動確認。
- **R-3**: `tracing::callsite::rebuild_interest_cache`（DD-2）が `tracing` 0.1.44 で公開 API として存在するか（`tracing::subscriber::set_global_default` は公開 API 確定）。
- **R-4**: `set_global_default` が subscriber Arc を leak し registrar が永久生存する挙動（dispatcher.rs:314-319）の一次確認——Interest ≥ Sometimes 固定の構造的根拠（R-1 と一体）。

---

## 6. 工数・リスク

| 項目 | 評価 | 根拠 |
|---|---|---|
| **工数** | **S（1〜3 日）** | 実装は 1 ファイル ~15 行。既存パターン踏襲・新規依存なし。工数の大半は RED→GREEN 証拠取り（4×25 stress・workspace 5 反復・i686 ビルド）と Req 7 のバリア設計。 |
| **リスク** | **Low〜Medium** | Low: 境界 1 ファイル・API 不変・本番不改変・グローバル衝突相手ゼロ。Medium 要素: (a) tracing 内部意味論の正しさ（R-1〜R-4）が誤れば根治にならない、(b) bare registry グローバル常駐の capture 外副作用（DD-4）、(c) Req 7 バリア設計（DD-7）。いずれも縮退路（Req 7 分離・行番号レビュー時確認）が用意され致命化しない。 |

---

## 7. 設計フェーズへの推奨

- **推奨アプローチ**: **Option C（A＋Req 7 同 PR）**を第一候補とし、Req 7 が GREEN 化困難な場合に **Option A（keeper のみ）**へ縮退。Option B は要件 1.3 文言と衝突するため退避路としてのみ記録。
- **設計で確定すべき鍵**: DD-2（rebuild_interest_cache 要否）・DD-3（fail-loud 実装形）・DD-4（グローバル常駐の副作用範囲）・DD-7（steady cage バリア設計）。
- **持ち越す研究項目**: R-1〜R-4（tracing 内部の一次確認・実装/レビュー時）。
- **次工程**: `/kiro-spec-design areka-P0-log-capture-determinism`（要件ディスカッションで DD-1〜DD-8 を解消後）。
