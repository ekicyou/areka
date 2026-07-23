# Requirements Document

## Introduction

areka-kanade のテスト専用ログ捕捉基盤 `crates/areka-kanade/src/schedule/log_capture.rs`（`capture()` + `assert_logged()`・**32 個の回帰檻が共有**）は、`cargo test --workspace` の並列負荷下で **~1/10〜1/20 の確率で捕捉対象イベントを取りこぼす**。この確率的失敗は当該クレート単体（`cargo test -p areka-kanade --lib`）や隔離実行では発現せず、workspace 並列実行時のみ現れる。

根本原因は確定済み（tracing-core 0.1.36 ソースレベル・再調査不要）: `capture()` が呼び出しごとに `with_default` で transient dispatcher を登録/破棄するため、**live dispatcher が 0 個の瞬間に callsite Interest キャッシュの rebuild が走ると `Interest::never` が sticky に焼き付く**（次の dispatcher 登録まで復活しない）。並列テストで transient dispatcher が全滅した瞬間に別スレッドの callsite が初回 register される窓で発症する。

この欠陥は input-events 由来ではなく areka-P0-kanade 時代から **main に存在**しており、input-events ブランチが並列負荷を上げて露呈させただけである。確率的失敗は開発者の決定論テスト方針（deterministic-test-coverage-mandate）に違反し、**全 spec の kiro-complete DoD Test Gate を確率的に赤くする**プロジェクト横断の毒となっている。

本仕様は log_capture 基盤にプロセスグローバルな interest-keeper を導入し、並列負荷下でも決定論的に緑になるようにする。`capture`/`assert_logged` の **API・意味論・32 檻は無改変**のまま保つ。areka-kanade 本体（非テスト）コードには一切触れない（Req 1〜6）。

**第二の workspace ゲート flake（4.4 検証で判明・Req 9）**: 本仕様の受け入れ検証（Task 4.4・`cargo test --workspace` 反復）の実測で、log 捕捉とは**別の第二の確率的クラッシュ**が判明した——`cargo test --workspace` 並列負荷下で wintf `--test layout` テストバイナリが **~13% の確率で 0xC0000005 アクセス違反により即死**する。根本原因は確定済み（一次実証）: layout テストバイナリは `CoInitializeEx` を一切呼ばないが、`EcsWorld::new()` が `WicCore::new()` で `CoCreateInstance(CLSID_WICImagingFactory2)` を呼ぶ。通常は `CO_E_NOTINITIALIZED` で失敗し `if let Ok`（`world/mod.rs:62`）が無言スキップするが、並走テスト（可視ウィンドウ生成に伴う MSCTF/TSF ロード）が副作用で**一時的な MTA を発生**させる窓では生成が成功し WIC factory を取得してしまう。その借り物 MTA が解体されると COM ランタイムごと factory が解放され、`EcsWorld` drop 時の `IUnknown::Release` が解放済みメモリへ仮想呼出して即死する（全 8 クラッシュの WER フォールトオフセットが `windows_core::unknown::IUnknown::Drop` で一致・容疑 2 テスト両 skip でクラッシュ完全消失を実証）。この第二 flake も log_capture 欠陥と同じく**全 spec の kiro-complete workspace DoD Test Gate を確率的に赤くする**プロジェクト横断の毒であり、本セッションの開発者判断により本仕様のスコープへ取り込む。処方は Req 1 の interest-keeper と**同型**——`WicCore` がプロセス寿命の MTA キーパーを自ら確立し、借り物寿命依存を構造的に根絶する（Req 9）。

## Boundary Context

- **In scope**:
  - `crates/areka-kanade/src/schedule/log_capture.rs` への interest-keeper 導入と、モジュール doc（PITFALL 節）の更新
  - RED→GREEN 検証（並列プロセス・ストレス実行 ＋ `cargo test --workspace` 反復）
  - （同一境界の二次修正・討議#1で確定）areka-kanade テスト内の「反復回数上限を時間の代用にする協調ループ」の上限非依存決定論化: 同型 `'drive` ループ 3 箇所（`steady_test.rs:821`・`close_test.rs:170`・`close_test.rs:806`）＋ `close_test.rs:57` の `wait_until` ヘルパー（100,000 yield 有界）
  - （4.4 検証で判明した第二の workspace ゲート flake・本セッションの開発者判断で本 spec 取込・Req 9）wintf の `WicCore`（`crates/wintf/src/ecs/widget/bitmap_source/wic_core.rs`）が COM アパートメントの借り物寿命に依拠して並列負荷下で確率的にクラッシュ（0xC0000005）する本番欠陥の根治: プロセス寿命 MTA キーパー（`CoIncrementMTAUsage`）の自己確立 ＋ `world/mod.rs:62` の無言スキップへの `error!` 付与
- **Out of scope**:
  - `capture`/`assert_logged` の API 変更・シグネチャ変更・32 檻の書き換え
  - areka-kanade 本体（非テスト）コードへの変更、kanade 本体の挙動・ログ語彙の変更（Req 6）
  - tracing / tracing-subscriber のバージョン更新・差し替え
  - 他クレート（wintf 等）のテスト基盤（テストハーネス／檻）への横展開・wintf の COM 寿命修正（Req 9）を超える wintf 本体挙動の変更
  - cargo-deny advisories への新規 allow 追加
- **Adjacent expectations**:
  - areka-P0-input-events（未マージブランチ・全タスク完了・実機サインオフ済み・開発者承認済み）の kiro-complete は本仕様のマージによって Test Gate が決定論的に緑となり再開・完了できる。input-events 側の機能変更は本仕様のスコープ外であり、マージ解除は本仕様の帰結にすぎない。
  - バージョンは Cargo.lock 固定（tracing 0.1.44 / tracing-core 0.1.36 / tracing-subscriber 0.3.23）を前提とする。
  - `cargo test --workspace` は i686 前提成果物（`shiori-host32-helper` / `shiori-host32-testdll`）のビルドを前提とする。ビルド・テストは PowerShell で実行する。

## Requirements

### Requirement 1: 並列負荷下での決定論的なログ捕捉
**Objective:** テストスイートを実行する開発者として、`cargo test --workspace` を並列負荷下で実行しても log 捕捉檻が確率的に失敗しないことを求める。それにより全 spec の kiro-complete DoD Test Gate が安定的に緑になる。

#### Acceptance Criteria
1. When `cargo test --workspace` が並列負荷下で実行される, the ログ捕捉基盤 shall `capture` のクロージャ内でテストスレッドが発行した対象イベントを取りこぼさず捕捉する。
2. While 複数のテストスレッドが `capture` を並行して出入りしている, the ログ捕捉基盤 shall callsite Interest が `Never` に焼き付く事象を発生させない。
3. When `capture` が初めて呼び出される, the ログ捕捉基盤 shall プロセス内で永続する interest-keeper を一度だけ確立し、以降の callsite Interest を常に有効（イベントが捨てられない状態）に固定する。
4. The ログ捕捉基盤 shall `cargo test --workspace` を連続 5 回以上（i686 前提成果物ビルド後）実行しても log 捕捉檻の失敗が 0 件である。

### Requirement 2: 既存 API・意味論・呼出箇所の不変性
**Objective:** 32 個の回帰檻を保守する開発者として、本修正が `capture`/`assert_logged` の API と意味論を変えないことを求める。それにより既存の檻・呼出箇所を無改変で緑のまま保てる。

#### Acceptance Criteria
1. The ログ捕捉基盤 shall `capture` と `assert_logged` の公開シグネチャ・引数・戻り値型を変更しない。
2. Where `capture` の呼出箇所（`actor.rs` ×4・`schedule/mod.rs` ×6・`schedule/boot.rs` ×1 の計 10 箇所）が既存のまま残る, the ログ捕捉基盤 shall それらを無改変でコンパイル・成功させる。
3. The ログ捕捉基盤 shall `capture` のスレッドローカル捕捉の意味論を不変に保ち、クロージャ外または他スレッドで発行されたイベントを捕捉しない。
4. If interest-keeper がプロセスグローバルに常駐する, then the ログ捕捉基盤 shall スレッドローカルの捕捉がグローバルを shadow する挙動によって各 `capture` 呼び出しの捕捉列を相互に混在させない。

### Requirement 3: TRACE を含む全レベルの捕捉
**Objective:** ログ檻を書く開発者として、ERROR/WARN だけでなく TRACE レベルの檻も捕捉できることを求める。それにより `actor.rs:647` の TRACE 檻（`shiori_request`）が安全に緑となる。

#### Acceptance Criteria
1. When `capture` のクロージャ内で TRACE レベルのイベントが発行される, the ログ捕捉基盤 shall そのイベントを捕捉し `assert_logged` で照合可能にする。
2. The ログ捕捉基盤 shall interest-keeper 導入後も TRACE・WARN・ERROR のいずれのレベルもフィルタで除外しない。

### Requirement 4: 回帰檻の保全（特に不在表明檻）
**Objective:** ログ規律を検証する開発者として、既存 32 檻が本修正で無改変のまま真に機能することを求める。特にイベント欠落で偽 PASS しうる不在表明檻が本修正の受益者となることを保証する。

#### Acceptance Criteria
1. The ログ捕捉基盤 shall 既存 32 個の回帰檻すべてを無改変で緑に保つ。
2. While `schedule/boot.rs` の不在表明檻が実行される, the ログ捕捉基盤 shall イベントの取りこぼしに起因する偽 PASS を発生させず、検証すべきイベントが実際に発行された場合のみ檻を通過させる。

### Requirement 5: 誤設定に対する明示的失敗（fail-loud・log-first）
**Objective:** 将来の回帰を防ぎたい開発者として、interest-keeper の前提が破られた場合に静かに縮退せず大声で落ちることを求める。それにより flake の静かな再発を防ぐ。

#### Acceptance Criteria
1. If 本基盤の interest-keeper より先に他のグローバル subscriber が設定されている, then the ログ捕捉基盤 shall 静かに縮退せず、原因を説明する明示的なメッセージで panic する。
2. The ログ捕捉基盤 shall interest-keeper の確立をログ無しの失敗経路（silent failure）にしない。

### Requirement 6: areka-kanade 本番コード非改変とスコープ境界
**Objective:** kanade 本体の挙動を保ちたい開発者として、log 捕捉修正が areka-kanade のテスト基盤に限定され areka-kanade 本番コードに一切触れないことを求める。それにより修正の副作用範囲を最小に保てる。

> **スコープ注記**: 本 Requirement 6 は areka-kanade の log 捕捉修正（Req 1〜5）を統べる。Task 4.4 検証で判明した wintf の WIC/COM 借り物寿命欠陥の**本番修正**は Req 9 が統べる（本セッションの開発者判断で取り込んだ意図的かつ最小の本番変更・wic_core.rs の COM 寿命に限定）。

#### Acceptance Criteria
1. The 修正 shall 本番（非テスト）コードを一切変更しない。
2. The 修正 shall kanade 本体の挙動・ログ語彙・イベント語彙を変更しない。
3. The 修正 shall tracing / tracing-subscriber のバージョン更新・差し替えを行わない。
4. The 修正 shall 他クレート（wintf 等）のテスト基盤へ変更を横展開しない。
5. The 修正 shall cargo-deny advisories へ新規 allow を追加しない。

### Requirement 7: 復帰駆動ループの上限非依存決定論化（同一境界・二次修正）
**Objective:** 同一境界内の潜在 flake を除きたい開発者として、復帰後 pump を駆動する Tick 供給ループが反復回数上限（時間の代用）に依存せず決定論的に完了することを求める。それにより CPU 飢餓（Defender 再スキャン等）下でも偽赤せず、input-events レビューで指摘済みの潜在 flake を同一 PR で一掃できる。

> **討議#1 帰結（2026-07-23）**: brief 記載の「park-barrier 化（mouse cage 10 手本）」は構造的に写像不能と調査で確定——mouse cage の `expected_holds` バリアは release-before-park race を閉じるもので既に `SakuraGate` 実装済み・復帰系 cage の固有構造（復帰点に観測可能シグナルが無く Tick 供給が構造的に必須）には効かない。真の病名は「反復上限＝時間の代用」（協調ループの CPU 飢餓・ghost e2e で根治済みの既知の病）であり、処方は「意味論的完了バリア＋壁時計 deadline」。

#### Acceptance Criteria
1. The 対象テストの同型 `'drive` ループ 3 箇所（`steady_test.rs` の `talk_completion_resumes_get_pump_ref3_one_status_none`・`close_test.rs:170` 付近・`close_test.rs:806` 付近）shall 反復回数上限（500×64 yield 等）に依存せず、意味論的完了シグナル（quit:true talk の帰結である inbox 切断＝send Err）をループ終了バリアとして駆動される。
2. The 対象テスト shall 復帰（pump 再開）の表明を、kanade の join 完了後の最終記録列に対して評価する（ループ内観測 race の排除・表明の強化）。
3. If kanade が復帰せず終了しない（欠陥）, then the 対象テスト shall 壁時計 deadline（`join_bounded`・`wait_until_blocked` と同系の Instant deadline）によりハングでなく失敗として検出する。
4. The `close_test.rs` の `wait_until` ヘルパー（100,000 yield 有界・壁時計なし）shall 壁時計 deadline ベースへ置換される。
5. When 二次修正が適用される, the 対象テスト shall 既存の検証意味論（非空虚性を含む）を変えずに緑を維持する。

### Requirement 8: 受け入れ検証（病の性質別の証拠形式）
**Objective:** 修正の有効性を確認する開発者として、修正前後の再現・解消を実行可能な証拠で示すことを求める。それにより恒久解であることを客観的に確認できる。

> **討議#2 帰結（2026-07-23）**: 証拠形式は病の性質に合わせる。keeper（R1〜R6）は「確率再現可能な病」ゆえ RED→GREEN ストレス（lib exe 対象）。R7' の対象ループ（integration exe 在住）は「反復上限＝時間の代用」という構造的な病で、飢餓由来 flake は原理的に統計再現できない（Defender 再スキャン等は制御不能）ゆえ、構造証明＋回帰緑で判定する。

#### Acceptance Criteria
1. When 修正前に lib テスト実行ファイル（mtime 最新）を 4 プロセス並列 × 25 ラウンドで起動する, the 検証 shall ≥1 件の失敗で keeper 欠陥を再現するか、~100 実行で未再現の場合はその旨を記録して workspace 反復判定へ委ねる。
2. When 修正後に同一のストレス実行を行う, the 検証 shall 失敗 0 件を示す。
3. When 修正後に `cargo test -p areka-kanade` と `cargo test --workspace`（連続 5 回以上）を実行する, the 検証 shall 全回で失敗 0 件を示す。
4. The R7' 対象ループの検証 shall 反復回数上限の不在（意味論的完了バリアと壁時計 deadline で駆動される構造的性質）をコードレビューで確認し、`cargo test -p areka-kanade` と workspace 反復（AC 8.3）で対象テストが緑であることをもって足りるものとし、飢餓の人工再現や integration exe への RED→GREEN ストレスは要求しない。
5. The R7' の非空虚性 shall kanade が復帰しない欠陥時に壁時計 deadline がハングでなく失敗として検出する経路の存在をレビュー観点で担保する。

### Requirement 9: wintf WIC factory の COM アパートメント寿命の決定論化（第二の workspace ゲート flake の根治・本セッション取込）
**Objective:** テストスイートを実行する開発者として、`cargo test --workspace` 並列負荷下で wintf テストバイナリが COM アパートメントの借り物寿命に起因して確率的にクラッシュ（0xC0000005）しないことを求める。それにより Req 1 の log 捕捉修正と併せて workspace DoD ゲートが真に決定論的に緑になる。

> **根本原因（一次実証・確定）**: `EcsWorld::new()` → `WicCore::new()` → `CoCreateInstance(CLSID_WICImagingFactory2, CLSCTX_INPROC_SERVER)`。layout テストバイナリは `CoInitializeEx` を呼ばないため通常は生成失敗（無言スキップ）だが、並走テストの副作用（可視ウィンドウ生成の MSCTF/TSF ロード）で一時的 MTA が立つ窓では生成が成功し、その借り物 MTA 解体で COM ランタイムごと factory が解放され、`EcsWorld` drop の `IUnknown::Release` が use-after-free で即死する。並列プロセス負荷（`cargo test --workspace` 相当）が発症窓の重なり確率を上げる（単独プロセスでは 135 実行 0 クラッシュ・8 プロセス同時負荷で 8/40 再現・容疑 2 テスト両 skip で 0/48）。

#### Acceptance Criteria
1. When `WicCore::new()` が WIC factory を生成する, the wintf shall 生成に先立ってプロセス寿命の MTA を自ら確立し、生成後に COM ランタイム／factory が呼び元の借り物アパートメントの解体で解放される事象を発生させない。
2. The wintf shall プロセス内で一度だけ MTA キーパー（`CoIncrementMTAUsage` の cookie をプロセス生存期間保持＝decrement しない）を確立し、以降 `WicCore` の COM ポインタが常に生存する COM ランタイム上に在ることを保証する。
3. When `cargo test --workspace` が並列負荷下で連続実行される, the wintf `--test layout` バイナリ shall 0xC0000005 アクセス違反でクラッシュしない。
4. If `WicCore::new()` の factory 生成が失敗する, then the wintf shall 無言スキップにせず失敗を `error!`（log-first・steering: areka-log-first-no-silent-failure）で記録する（縮退挙動自体は維持してよい）。
5. The 修正 shall 本番挙動の意味論を変えない（本番は `WinApp::new` が既に `CoInitializeEx(MTA)` をプロセス寿命で確立済ゆえ MTA キーパーは参照カウント増分のみ・無害）・新規依存を追加しない（`windows::Win32::System::Com` は使用済）・wintf の他テスト基盤や他クレートへ変更を横展開しない。
6. When 修正の有効性を確認する, the 検証 shall 修正前に並列プロセス負荷で 0xC0000005 を再現し（RED）、修正後に同一のストレスで 0 クラッシュを示す（GREEN）——本 flake は「確率再現可能な病」ゆえ RED→GREEN ストレスで判定する。
