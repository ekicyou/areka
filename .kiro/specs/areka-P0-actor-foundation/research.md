# Gap Analysis — areka-P0-actor-foundation

> 実施日: 2026-07-03 / 対象: 確定済み requirements.md（Req1〜8）+ brief.md / 言語: ja
> 目的: 確定要件と既存コードベースの差分を洗い出し、設計フェーズの実装戦略判断へ材料を渡す（情報提供であって決定ではない）。

## 1. 分析サマリ（3〜5点）

- **既存パターンは"ほぼ揃っている"**: 本ユニットが求める原語（名前付きスレッド spawn＋`Arc<AtomicBool>` stop→join の RAII・`event_listener::Event` によるスレッド跨ぎ起床・`spawn_local` による UI スレッド async・listen-before-work の notify 取りこぼし防止・`Weak` upgrade による shutdown 終了）は、`wintf` の `VsyncEventBridge`/`CursorMonitorBridge`/`ClickThroughController` に**実証済みの生きた実装として既存**する。本ユニットは新技術の導入ではなく、これら散在パターンの**規約化・最小ヘルパ化・"エンジン非依存"への一般化**が主眼。
- **UI 配送ブリッジは wintf 依存が不可避**: Req4 の配送ブリッジは `wintf_winmsg_executor::spawn_local`／`event_listener` に密結合し、UI スレッド（MTA・`WinApp`）・message pump の存在が前提。これは `std::sync::mpsc`＋`std::thread` だけの純粋層には収まらず、**規約/純粋ヘルパ層（wintf 非依存）と UI ブリッジ層（wintf 依存）の二層分割**という brief の Boundary Candidate が構造的に妥当。ただし配置クレート（新設 `areka-actor` か wintf 内か）は要決定（下記 DD-1/DD-2）。
- **request/reply（oneshot 相当）と "Close→drain→join" の既製品が無い**: 既存は「起床通知（データ非搬送の `Event::notify`）」中心で、**メッセージ enum＋返信 Sender 同梱の request/reply**、および **inbox の Close→drain→join 停止規約**を体現した再利用可能ヒルパは存在しない（host-32 は WM_COPYDATA の `ResponseSlot` で single-in-flight reply を実現するが、これはプロセス跨ぎ・window message 専用で in-proc channel の写像ではない）。ここが本ユニットの**新規実装の核**。
- **依存ゼロ方針は成立する**: `std::sync::mpsc`（`WintfTaskPool` が既に本番採用・`mpsc::Sender<BoxedCommand>`）で inbox は賄える。`recv_timeout` で tick 相当も可能。`crossbeam-channel`・`tokio` は不要（要件で凍結/禁止）。純粋層の追加依存はゼロで達成可能。UI ブリッジ層のみ `event_listener`＋`wintf-winmsg-executor` を要する（いずれも既存 workspace 依存）。
- **toy 試験(b)（worker→UI pump 実走 echo）の実行環境が論点**: Req8.2 は「wintf の pump 上で」echo を検証すると明記。既存の UI スレッド pump 実走テスト（`WinApp::run` の `block_on`）は実 HWND／メッセージループを要し、`tick_bridge`/`clickthrough` のユニットテストは**pump を回さず同期コアだけを検証**する回避策（headless）を採っている。toy 試験(b) を "pump 実走" で機械 pass/fail 化する具体手段（integration test か example か・bounded 終了規律）は要設計（DD-5）。

## 2. Requirement → 既存資産マップ（Missing / Reuse / Constraint タグ付き）

| Req | 技術的必要物 | 既存資産（再利用元） | ギャップ判定 |
| --- | --- | --- | --- |
| **R1 spawn/join** | 名前付きスレッド起動・`Sender`＋`JoinHandle` 返却・panic を join で観測・軽量 spawn/破棄 | `std::thread::Builder::new().name(..).spawn(..)`（`VsyncEventBridge::new`/`CursorMonitorBridge::spawn` で実証）・`JoinHandle` 保持と `join()`（`stop()` の `join().is_err()` で panic 観測パターンあり） | **Reuse＋薄い一般化**。既存は「名前付き spawn＋stop_flag→join」を各所でコピー。これを "inbox Sender も返す spawn ヘルパ" へ一般化。panic 伝搬は `join()` の `Result::Err` をそのまま返す規約化で足りる。 |
| **R2 inbox＋request/reply** | アクターごと単一 Receiver・`XxxMsg` enum 規約・reply Sender 同梱（oneshot 相当）・Send 所有データ・`Arc` 大型手渡し | `WintfTaskPool`（`mpsc::Sender/Receiver`・`Mutex<Receiver>`）が in-proc channel の既存採用例。host-32 `ResponseSlot`（single-in-flight reply の別実装＝window message 用） | **Missing（新規核）**。oneshot 相当の "reply Sender をメッセージに同梱" した in-proc request/reply ヘルパは無い。std mpsc で `Sender<Reply>` を enum variant に持たせる規約＋薄いヘルパを新規に定義。 |
| **R3 停止 Close→drain→join** | inbox enum に Close variant・受信ループ終了・drain/破棄を一方に固定・決定的 join・全 Sender drop で正常終了 | `stop_flag(AtomicBool)→join` の RAII（`VsyncEventBridge`/`CursorMonitorBridge`）は "停止" の実証だが**メッセージ経由の Close ではない**。mpsc の "全 Sender drop→`recv()` が `Err`→ループ終了" は std 標準挙動 | **Missing（規約化）**。Close variant＋drain 方針（処理する/破棄する）の**固定**が未定＝brief/要件が "どちらか一方に固定" と要求。std mpsc の Sender-drop 正常終了（R3.5）は標準挙動で自然に満たせる。 |
| **R4 UI 配送ブリッジ** | pump 上アクターへ queue＋wakeup 配送・pump 非ブロック・起床ごと drain・MTA/render固定/D2D単一維持・`VsyncEventBridge`/`event_listener`/`wintf-winmsg-executor` と整合・emo-present/窓移動の搬送路 | `ClickThroughController`＝**worker→UI スレッドの二重起床ブリッジの完成実装**（`spawn_local` async ループ＋`event_listener::Event` wake＋listen-before-work＋`Weak` upgrade shutdown）。`AsyncTickTask`（`spawn_local` で pump に相乗り tick）。`CommandSender`＋`drain_and_apply`（worker→World の queue drain 前例） | **Reuse＋一般化（構造は既存）**。"queue（mpsc）＋wakeup（`event_listener` notify）＋pump 内 drain" は clickthrough/tick で実証済み。本ユニットは対象を "任意 UI アクターの inbox" へ一般化する薄いブリッジを切り出す。wintf 依存が確定する層。 |
| **R5 backpressure/流量** | 制御 unbounded 明文化・大型データを channel に流さず `Arc`/共有バッファ・select/MPMC/有界は crossbeam 拡張シームへ | std `mpsc::channel()`＝unbounded（既存 `WintfTaskPool` が採用）。`Arc` 手渡しは workspace 全域で常用（例: `Arc<event_listener::Event>`） | **Missing（文書化のみ）**。実装物ではなく "規約の明文化"。std mpsc unbounded がそのまま R5.1 を満たす。R5.3 は "導入しない" を拡張シームとして残すだけ。 |
| **R6 tracing** | span にスレッド名・アクター名・Subscriber 初期化しない | `tracing`（全クレート規約）・既存 bridge が `debug!/trace!` でスレッド名を含むメッセージを出力。areka 側で `tracing_subscriber::fmt().with_env_filter(..).init()`（アプリ層初期化＝要件通り）。 | **Reuse＋薄い追加**。`#[tracing::instrument]` もしくは `span!` でアクター名フィールドを載せるだけ。Subscriber 非初期化は既存慣行と一致（`shiori_demo` はテスト内で `with_default`）。 |
| **R7 最小性（framework 禁止）** | 規約＋薄いヘルパ＋ブリッジに限定・共通トレイト過剰抽象なし・2 例目まで抽象保留・既存エンジン実アクター化しない | プロジェクト規律（roadmap "spec 工場の禁止"・"抽象は2例目の実物が要求してから"）。既存 parser 群も "転記層に徹する" 同型の最小主義 | **Constraint（設計指針）**。実装物ではなく設計制約。API 表面を最小に保つ判断基準。 |
| **R8 観測（toy 試験）** | (a) worker⇄worker request/reply＋Close→join 決定的完走 (b) worker→UI pump 実走 echo (c) 失敗は fail 観測 | (a) は純粋層で `#[cfg(test)]` により決定的検証可（thread＋mpsc）。(b) は既存の "pump 実走テスト" 前例が乏しく headless 回避が主流 | **(a) Reuse / (b) Missing（要設計）**。(a) は既存パターンで容易。(b) の pump 実走 echo の機械 pass/fail 化手段は要決定（DD-5）。 |

## 3. 実装アプローチ選択肢（A/B/C・トレードオフ）

### Option A: 既存 wintf クレート内に新設モジュール（`wintf::actor` 等）として実装
- **範囲**: 純粋層＋UI ブリッジ層を両方 wintf 内の新規モジュールに置く。
- **論拠**: UI ブリッジは `spawn_local`／`event_listener`／`WinApp` に密結合し、そもそも wintf を触る唯一の層。同居させれば依存追加ゼロ・既存 bridge 群（`runtime/tick_bridge.rs`・`ecs/clickthrough/`）と近接配置でき、パターン共有が容易。
- **トレードオフ**: ✅ 依存追加ゼロ・既存資産に隣接・即着手。❌ 純粋層（エンジン非依存・wintf 非依存であるべき部分）まで wintf に閉じ込めると、将来 host-32（別クレート・std-only）や areka 側から再利用しづらい。brief の "⓪ ghost 帰属の横断基盤" という位置づけと、"parser-foundation の並行版"（＝独立クレート `areka-parsers` に対応）という類推から外れる。

### Option B: 新設クレート `areka-actor`（純粋層）＋ UI ブリッジは wintf 内（分割配置）
- **範囲**: 規約＋純粋ヘルパ（spawn/join・inbox・request/reply・Close→drain→join・std mpsc のみ・**依存ゼロ or tracing のみ**）を新設独立クレート `areka-actor` に。UI 配送ブリッジ（wintf 依存）は wintf 内の新設モジュールに置き、`areka-actor` の envelope 規約型を実装/consume する。
- **論拠**: brief の Boundary Candidate（"規約＋ヘルパ（純粋・単体テスト可）／UI 配送ブリッジ（wintf を知る唯一の層）の二層"）に最も忠実。純粋層は `areka-parsers`（`encoding_rs`＋`tracing` のみの最小依存独立クレート）と同型で、kanade/sakura/seriko/ghost-setup が wintf に引きずられず consume 可能。host-32（std-only）とも思想整合。
- **トレードオフ**: ✅ 層分離が構造的・単体テスト容易・下流の依存グラフが素直・"parser-foundation の並行版" の類推が実体化。❌ クレート 1 個の新設コスト（Cargo.toml・workspace member 追加）。純粋層と UI 層で envelope 型を跨ぐ設計を要する（型の所有はどちらか要決定＝DD-3）。

### Option C: ハイブリッド（純粋層は最初 wintf 内モジュール、2 例目消費時にクレート抽出）
- **範囲**: まず全体を wintf 内に置き（Option A）、kanade など 2 例目の実消費が現れた時点で純粋層を `areka-actor` へ抽出（Option B へ移行）。
- **論拠**: roadmap の "抽象は 2 例目の実物が要求してから"（R7.2）の精神に沿い、クレート境界という抽象すら実需まで遅延。
- **トレードオフ**: ✅ 初期コスト最小・YAGNI 徹底。❌ kanade は**本ユニットの直後の先行依存**（roadmap: "kanade＝actor-foundation 先行依存"）ゆえ 2 例目がすぐ来る＝抽出移行コストを近い将来必ず払う。抽出時に kanade/sakura 等の import path 変更が波及する。"横断基盤" が最初から複数クレートに consume される見込みが濃厚なため、遅延の利得が小さい可能性。

> 分析上の所見（決定ではない）: brief が明示的に "parser-foundation の並行版"・"二層"・"新設モジュール/小クレート（`areka-actor` 等・design 判断）" と述べており、Option B が最も要件・brief の意図に整合的に見える。ただし "新設モジュール**または**小クレート" と両論併記されているため、純粋層のクレート化是非は設計判断として明示的に俎上へ載せるべき（DD-1）。

## 4. Effort / Risk

| 項目 | 見積 | 一行根拠 |
| --- | --- | --- |
| 純粋層（spawn/join・inbox・request/reply・Close→drain→join・R1/2/3/5/6/8a） | **Effort S（1〜3日）／Risk Low** | 既存パターン（thread＋mpsc＋join）を規約化する範囲。std のみ・決定的単体テスト可・新技術なし。 |
| UI 配送ブリッジ（R4・R8b） | **Effort M（3〜7日）／Risk Medium** | 構造は clickthrough/tick に実証済みだが "任意 UI アクター inbox への一般化" と "pump 実走 echo の機械 pass/fail 化"（DD-5）に設計余地。MTA/D2D 単一/pump 非ブロックの不変条件維持を要検証。 |
| クレート境界・envelope 型の所有決定＋配置（DD群） | **Effort S／Risk Low〜Medium** | 純粋実装より "どこに置き誰が型を所有するか" の設計判断が主。決めれば実装は軽い。 |
| 全体 | **Effort M（合算 4〜8日相当）／Risk Low〜Medium** | 新規核は request/reply と Close 規約のみ。残りは既存資産の一般化と文書化。最大の不確実は R8b の pump 実走観測手段。 |

## 5. 設計フェーズへの申し送り（Research Needed / 推奨）

### Research Needed（設計フェーズで詰める）
- **RN-1**: `wintf-winmsg-executor`（=0.0.5）の `spawn_local`／`JoinHandle`／`MessageLoop` の公開 API が、"任意 UI アクター inbox の起床 drain" を wintf 外へ露出できるか（現状は wintf 内でのみ利用）。UI ブリッジを別クレートに切る場合の API 境界。
- **RN-2**: toy 試験(b) の "wintf の pump 実走上での echo" を機械 pass/fail で回す手段（integration test で bounded な `MessageLoop`＋heartbeat 終了 or example の手動検証）。host-32 `parent_window.rs` の `pump_until_hello_or`（別スレッド WM_NULL heartbeat＋deadline quit の bounded pump）が有力な写経元。
- **RN-3**: request/reply の reply channel 実体（`std::sync::mpsc::channel()` を per-request 生成 or `oneshot` 専用軽量型を自作）。std に oneshot は無いため mpsc の 1 回受信で代替する規約の是非。
- **RN-4**: ~~Close→drain の "drain して処理 / 破棄" の**どちらに固定するか**（R3.3・要件が一方固定を要求）。~~ → **【解決 2026-07-03 要件ディスカッション #1】「破棄（Close=即時停止）」に固定**。積み残しメッセージの drop により同梱 reply Sender も drop され、要求側は `reply.recv()` を `Err`（切断）で観測＝ハングしない（std mpsc の drop 意味論が自己シグナル化）。graceful 停止は送信側が「後続なしを確認後に Close」で本原語の上に構築。R3.3 更新＋R3.6 追加で反映済み。

### 設計判断アイテム（要件ディスカッションへ供給・番号付き）
- **DD-1**: 純粋層を独立クレート `areka-actor` にするか、wintf 内新設モジュールに留めるか（Option A/B/C）。"parser-foundation の並行版" の類推・kanade が即・先行依存である点・host-32(std-only) 再利用性が判断材料。
- **DD-2**: UI 配送ブリッジ層の配置（wintf 内で確定か／純粋層と同一クレートに寄せるか）。wintf 依存が不可避な唯一の層である前提。
- **DD-3**: envelope 規約型（メッセージ enum の共通シェイプ・reply Sender 同梱の型・Close の載せ方）を "どの層が所有" するか。純粋層が型を持ち UI ブリッジが consume する形が二層分離に自然だが、共通トレイトの過剰抽象（R7.1）を避ける線引きが要る。
- **DD-4**: request/reply の oneshot 相当実装（per-request `mpsc::channel()` か軽量自作 oneshot か）。std のみ・依存ゼロ制約下での選択。
- **DD-5**: toy 試験(b) の観測形態（bounded integration test か example か）と、pump 実走の終了規律（deadline＋heartbeat か shutdown Event か）。R8.2 の "機械 pass/fail" 要求を満たす具体手段。
- **DD-6**: ~~Close 到達時の未処理メッセージ方針（drain 処理 / 破棄）の固定（R3.3）。~~ → **【解決済み・上記 RN-4 参照】破棄に固定**（reply Sender drop により要求側は `Err` 観測＝ハングなし）。requirements.md R3.3 更新＋R3.6 追加で確定。設計フェーズへの持ち越しなし。
- **DD-7**: spawn ヘルパの API 形（返却は `(Sender<XxxMsg>, JoinHandle<..>)` か newtype ハンドルか）。R1.1 "Sender と JoinHandle を返す" を最小表面で満たす形。既存 `ClickThroughHandle`/`CursorMonitorBridge` の RAII ハンドル流儀を踏襲するか否か。
- **DD-8**: panic 伝搬の粒度（R1.3）。`join()` の `Err` をそのまま呼び出し側へ返すだけか、`JoinHandle` を包む薄い型で "panic を失敗として観測可能化" するか（監督ツリーは作らない＝R7.2 との線引き）。

## 6. 既存資産の具体参照（設計の写経元・全て絶対パス）

- `C:\home\maz\git\areka\.claude\worktrees\confident-gauss-6ffd16\crates\wintf\src\runtime\tick_bridge.rs` — `VsyncEventBridge`（名前付き spawn＋`Arc<AtomicBool>` stop→join RAII・`event_listener::Event` notify）／`AsyncTickTask`（`spawn_local` で pump 相乗り・listen-before-work・`Weak` upgrade shutdown）。R1/R4 の写経元。
- `C:\home\maz\git\areka\.claude\worktrees\confident-gauss-6ffd16\crates\wintf\src\ecs\clickthrough\controller.rs` — `ClickThroughController::start`／`ClickThroughHandle`（RAII）／`run_click_through`（worker→UI 二重起床 async ループ）。**R4 UI 配送ブリッジの最良の完成前例**。
- `C:\home\maz\git\areka\.claude\worktrees\confident-gauss-6ffd16\crates\wintf\src\ecs\clickthrough\monitor.rs` — `CursorMonitorBridge`（worker スレッド＋`event_listener` 起床＋store→notify 順序不変＋RAII join）。R1/R4 worker 側の写経元。
- `C:\home\maz\git\areka\.claude\worktrees\confident-gauss-6ffd16\crates\wintf\src\runtime\message_loop.rs` — `ShutdownPolicy`（`event_listener::Event` による block_on 先行 quit 回避・tail race 補填 notify）。停止規律（R3）と UI ループ終了規律の参照。
- `C:\home\maz\git\areka\.claude\worktrees\confident-gauss-6ffd16\crates\wintf\src\runtime\mod.rs` — `WinApp`（MTA 初期化・`spawn_local` facade・run 全結線・`wire_click_through` の worker＋wake relay 結線）。UI ブリッジがどこに繋がるかの結線例。
- `C:\home\maz\git\areka\.claude\worktrees\confident-gauss-6ffd16\crates\wintf\src\ecs\widget\bitmap_source\task_pool.rs` — `WintfTaskPool`（`mpsc::Sender<BoxedCommand>`＋`Mutex<Receiver>`＋`drain_and_apply`）。**in-proc mpsc channel＋queue drain の既存本番採用例**。R2/R5 の写経元。
- `C:\home\maz\git\areka\.claude\worktrees\confident-gauss-6ffd16\crates\shiori-host32-host\src\parent_window.rs` — `ParentMessageWindow::pump_until_hello_or`（bounded pump＋別スレッド heartbeat＋deadline quit）／`ResponseSlot` の single-in-flight reply。**プロセス跨ぎ actor 境界の参照実装**・R8b の bounded pump 実走テストの写経元。
- `C:\home\maz\git\areka\.claude\worktrees\confident-gauss-6ffd16\crates\shiori-host32-host\src\process_host.rs` — helper プロセス spawn＋非ブロッキング生存監視（`poll_exit`/`ExitKind::classify`）。別プロセス＝天然 actor 境界の lifecycle 参照。
- `C:\home\maz\git\areka\.claude\worktrees\confident-gauss-6ffd16\crates\areka-parsers\Cargo.toml` — 最小依存独立クレート（`encoding_rs`＋`tracing`）の型。純粋層クレート化（DD-1）時の Cargo.toml 雛形。
- `C:\home\maz\git\areka\.claude\worktrees\confident-gauss-6ffd16\crates\areka\src\main.rs`（L119-123）— `tracing_subscriber::fmt().with_env_filter(..).init()`。R6 の "Subscriber 初期化はアプリ層" の既存実例。

---
*本 gap 分析は情報提供であり実装決定ではない。DD-1〜DD-8 は要件ディスカッション／設計フェーズで解決される設計判断項目である。*
