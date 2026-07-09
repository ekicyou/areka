# ギャップ分析: areka-P0-ghost-setup

> 対象: 確定済み requirements.md（R1〜R8）と既存コードベースの差分分析。
> 調査日: 2026-07-05。実シンボルは worktree `stoic-jemison-0363b2` の実コードで検証済み。
> 目的: 実装戦略の情報提供（**決定はしない**・設計判断項目は要件ディスカッションへ送る）。

## 0. サマリ（3〜5 点）

- **既存パターン**: `areka-actor`（`spawn_actor`／`ActorHandle`／`run_inbox`／`ReplySender`）が全アクターの正準基盤。kanade・sakura・shiori actor はいずれもこの上に建っており、本仕様の dispatcher／ticker／終了統括も同基盤で書ける。std mpsc・tokio 禁止・スレッド独立という並行モデルは確立済み。
- **契約フォークは実測どおり確定**: kanade（`talk.rs`）は `StartTalk{talk_id, script}`／`TalkDone{talk_id, quit:bool}`・TalkDone は自 inbox（`KanadeMsg::TalkDone`）受領。sakura（`contract.rs`）は `StartTalk{script, talk_id, reply: ReplySender<TalkDone>}`／`TalkDone{talk_id, reason: TalkEndReason}`（**3 値** Ended/Quit/Interrupted）・TalkDone は reply oneshot。**両クレート間に依存辺が無い**（kanade→sakura も sakura→kanade も Cargo 依存なし）——これが「kanade 正本化」の実装形態を決める最大の制約。
- **欠落能力（新規実装が要る中核）**: (1) **ghost 結線層そのものが存在しない**（`ghost` クレート無し・areka バイナリは kanade/sakura/host/mount のいずれにも依存していない）、(2) **sakura dispatcher**（永続 inbox↔per-talk transient の橋渡し・単一 slot・stale 棄却・Close funnel）、(3) **ticker**（差し替え可能な時刻供給）、(4) **正規 clean shutdown 結線**（現状 `ShioriMsg::Unload` は `Unloaded` を返すだけのスタブ・`request_clean_shutdown` 未結線）、(5) **helper 死活監視**（`HelperLifecycle::status()`／`report_failure` は kanade 内で未使用）。
- **凍結面は健在**: `TalkCue`／`SurfaceSink`／`TextSink`／`cue_target_of`／dola cue 型は sakura で実装済み・seriko 消費面ゆえ不改変（WS-A の触ってよい面は `StartTalk`／`TalkDone`／`SakuraMsg::Start` の授受面のみ）。
- **Research Needed（設計フェーズ持越し）**: WS-A の型所有位置（kanade 直参照 vs areka-actor へ昇格 vs ghost 内変換）、reply 配送方式（oneshot 同梱維持 vs kanade inbox 転送へ一本化）、`HelperHandle` と `HelperLifecycle` の役割再配置、ticker の per-talk elapsed 供給経路、ghost クレートを新設するか areka バイナリへ結線するか。

---

## 1. 現状調査（Current State）

### 1.1 クレート配置の実測（brief の呼称 → 実体）

| brief の呼称 | 実体の所在 | 備考 |
|---|---|---|
| areka-kanade | `crates/areka-kanade/`（独立クレート） | 依存: `areka-actor`, `shiori-host32-host`。**sakura 非依存** |
| areka-sakura | `crates/areka-sakura/`（独立クレート） | 依存: `dola`, `areka-actor`, `areka-parsers`。**kanade 非依存** |
| shiori-host32-host | `crates/shiori-host32-host/`（独立クレート） | `HelperLifecycle`/`Shiori3Client`/`request_clean_shutdown`/`ExitKind` |
| **app-shell** | **`crates/areka/src/main.rs`（バイナリ）内の private 関数群** | 独立クレートでない。`resolve_config_inputs`/`open_startup_window`/`ConfigInputs`/`DummyWindowMarker` は**すべて非 pub** |
| **package-mount / MountModel** | **`crates/areka-parsers/src/package/`（module）** | 独立クレートでない。`resolve(ghost_root, DefaultEncoding) -> Result<MountModel, MountError>` |
| **ghost（結線層）** | **存在しない** | 新規に所有先を決める必要（クレート新設 or areka へ結線） |

**重要**: `crates/areka`（バイナリ）の `[dependencies]` は `wintf`/`shiori-abi`/windows 系のみ。**kanade・sakura・host32・areka-parsers への依存が一切無い**。結線層はこれらの依存辺を新設し、`main.rs` の private シーム（`open_startup_window`）を消費または昇格する必要がある。

### 1.2 既存の並行/アクター基盤（`areka-actor`・凍結せず流用）

- `spawn_actor<M,F>(name, body) -> (Sender<M>, ActorHandle)`: 名前付きスレッド・inbox 1 本・`tracing` span。
- `run_inbox<M,E>(rx, handler)`: `Ok(Continue)`/`Ok(Break)`/`Err`（log して継続）の正準受信ループ。停止経路は Break＋全 Sender drop の 2 経路。
- `ActorHandle::join() -> Result<(), ActorError>`（panic を `Panicked` に写像・非 RAII detach）。
- `reply_channel<T>() -> (ReplySender<T>, ReplyReceiver<T>)`: per-request の std mpsc oneshot 相当。`ReplySender::send(self)` が move-consume で「高々 1 回」を型強制。`recv_timeout` は Timeout/Dropped を区別。
- **含意**: dispatcher・ticker・終了統括はいずれもこの原語だけで実装可能（新規外部依存不要）。ticker の周期 tick は `run_inbox` でなく `recv_timeout` 自前ループになる可能性が高い（Req 3.7 の規律は踏襲）。

### 1.3 kanade（③運行・完成済み）実シンボル

- `spawn_kanade(config: KanadeConfig, shiori: Sender<ShioriMsg>, sakura: Sender<StartTalk>) -> (Sender<KanadeMsg>, ActorHandle)`
  - `KanadeConfig{ shell_name, baseware_version, baseware_name, close_talk_deadline_ms:u64 }`（`new(shell_name, baseware_version)` は baseware_name="areka"・deadline=30_000 を既定）。
  - `KanadeMsg::{ Boot, Tick{now:MonotonicMs}, TalkDone(talk::TalkDone), CloseRequest{reason}, ForceQuit{reason}, ShioriDown{reason:String}, Close }`。
  - **`sakura: Sender<StartTalk>` が「永続 channel へ送る」端**——kanade は StartTalk を送るだけで、その先が per-talk であることを知らない。送出失敗は `error!`＋運行継続（sakura 切断耐性テスト済み）。
  - `MonotonicMs(pub u64)`（OS 稼働ミリ秒・注入時刻）。
- `spawn_shiori_actor(connect: impl FnOnce() -> Result<ShioriConnection, String> + Send + 'static, on_down: Sender<KanadeMsg>) -> (Sender<ShioriMsg>, ActorHandle)`（`crates/areka-kanade/src/shiori/real.rs`）。
  - `ShioriConnection{ window: ParentMessageWindow, helper: HelperHandle }`——**`helper` は `HelperHandle`（生ハンドル）であって `HelperLifecycle` ではない**。
  - `connect` はアクタースレッド上で 1 回だけ実行（`ParentMessageWindow` が `!Send`）。失敗時 `ShioriDown` 死活報告。成功後 `on_down` は即 drop（Req 4.9）。
  - `ShioriMsg::{ Request{call, reply:ReplySender<ShioriOutcome>}, Unload{reply}, Close }`。
- close 握手: `close_talk_deadline_ms` は「最後の Tick の now＋deadline」で武装（最初の Tick が None なら初 Tick で武装）。now≧deadline で `Unloading{DeadlineExceeded}`＋error log。

### 1.4 sakura（④再生・完成済み）実シンボル

- `spawn_talk(start: StartTalk, surface_sink: impl SurfaceSink+Send+'static, text_sink: impl TextSink+Send+'static) -> TalkHandle`。
  - `StartTalk{ script:String, talk_id:TalkId, reply: areka_actor::ReplySender<TalkDone> }`——**reply 同梱**。
  - `TalkDone{ talk_id:TalkId, reason: TalkEndReason }`・`TalkEndReason::{ Ended, Quit, Interrupted }`（**3 値**）。
  - `TalkHandle{ inbox: Sender<SakuraMsg>, actor: ActorHandle }`。`spawn_talk` が `SakuraMsg::Start` を自己投函（外部は Tick/Close のみ送る）。
  - `SakuraMsg::{ Start(StartTalk), Tick(f64), Close }`。**Tick(f64) は talk 起点からの経過秒**（0 起点・単調非減少・有限）。「本番は外部 ticker が `dola::runtime::clock::now()` から elapsed 算出」と明記。
- 終端は `state.take()→reply.send(TalkDone)→Break` の対（高々 1 回機構）。Close→`Interrupted`。空/quit-only スクリプトは Tick 不要で即 `TalkDone`。
- **凍結面（不改変）**: `SurfaceSink::emit(&mut self, TalkCue)`／`TextSink::emit(...)`／`TalkCue{at,actor,command}`／`cue_target_of(&CueCommand)->Option<CueTarget>`／dola cue 型。`MockSink` が録音 sink の既存資産（spine e2e の記録 sink に流用可）。
- 再エクスポート: `lib.rs` で `pub use contract::*`（DD-1 の import 安定化・下流は `areka_sakura::contract::*` 参照）。

### 1.5 shiori-host32-host（①・凍結消費）実シンボル

- `HelperLifecycle{ new(HelperHandle), status()->HelperStatus, terminate()->io::Result<()>, request_clean_shutdown(&mut self, &ParentMessageWindow)->Result<ExitKind, ShutdownError>, pid(), report_failure(RequestError)->LifecycleReport }`。
- `HelperStatus::{ Running, Exited(ExitKind) }`・`ExitKind::{ Clean, Abnormal(i32), Terminated }`（Req 7.3 の `ExitKind::Clean` はここ）。
- `Shiori3Client<'a>{ new(&'a ParentMessageWindow), with_sender(...), get(id,&[String])->Result<Option<String>,RequestError>, notify(...)->Result<(),RequestError> }`——**window を借用（lifetime 束縛・スレッド固定）**。
- `classify_failure(&RequestError, Option<ExitKind>)->FailureClass`・`ShutdownError::{ Unload, UnexpectedAck, ExitTimeout }`。

### 1.6 app-shell（`crates/areka/src/main.rs`）実シンボル

- `resolve_config_inputs(args:&[String]) -> ConfigInputs{ ghost_root, balloon_root }`（純粋・既定は `CARGO_MANIFEST_DIR` 相対）。
- `open_startup_window(&WinApp)`——**replace-me シーム**。現状はダミー窓 spawn＋`AREKA_APP_SMOKE_EXIT_MS` ゲート自動 close。**private fn**（クレート外から呼べない）。
- `main()`: log init → `resolve_config_inputs` → `WinApp::new()` → `shiori_demo::run_demo_if_enabled()`（env gate）→ `open_startup_window(&app)` → `app.run()`。
- smoke 統合テスト `tests/smoke_boot_loop_exit.rs`: `CARGO_BIN_EXE_areka` を子プロセス起動・`AREKA_APP_SMOKE_EXIT_MS=500`・exit 0 のみ判定・60s 番犬。**この smoke を緑のまま維持**が R8.2。

### 1.7 package（②・`areka-parsers::package`）実シンボル

- `resolve(ghost_root:&Path, default_encoding: DefaultEncoding) -> Result<MountModel, MountError>`。
  - `MountModel{ names:GhostNames{name,sakura_name,kero_name}, shiori:ShioriMount{dir:PathBuf, file:Option<String>}, shell:ShellMount{dir:PathBuf} }`。
  - `MountError::{ StartPointMissing{expected}, StartPointUnreadable{path,kind}, ShellDirMissing{expected} }`（R2.5 の 3 致命失敗と一致）。
  - **`default_encoding: DefaultEncoding` 引数が必須**（既定をハードコードしない設計・SSP 準拠は ANSI）——ghost 結線層が既定を供給する必要（設計判断）。

---

## 2. 要件実現性分析（Requirement → Asset マップ）

| 要件 | 必要な技術 | 既存資産 | ギャップ種別 |
|---|---|---|---|
| **R1** talk 契約統一 | 単一正本の `StartTalk`/`TalkDone`/中断理由（3 値） | kanade `talk.rs`＝2 値 quit:bool／sakura `contract.rs`＝3 値 reason・両者に依存辺なし | **Constraint**（型所有位置の決定が必要）＋ **Missing**（3 値化に伴う kanade quit:bool 消費部の改稿） |
| R1.4 import パス不変 | `areka_sakura::contract::*` 経由の参照互換 | sakura `pub use contract::*` 既存 | **Constraint**（再エクスポート差し替えで維持） |
| R1.5 凍結面不改変 | `TalkCue`/sink/`cue_target_of`/dola cue 不変 | sakura で実装済み・不改変で足りる | 既存流用 |
| R1.6 両クレートテスト追随 | 新契約に既存テストを追随・緑維持 | kanade/sakura とも網羅テスト有 | **Missing**（テスト改稿） |
| **R2** descript 起点起動統括 | mount→shiori→kanade→sakura dispatcher→ticker の順で結線 | 各エンジン spawn API 完成・結線層は無 | **Missing**（ghost 結線層の新規実装） |
| R2.3 KanadeConfig 値源 | shell 名＝shell descript／baseware＝areka 定数 | `MountModel.names`／`KanadeConfig::new` 既存 | **Missing**（値源解決の結線・shell 名の出所確定＝設計判断） |
| **R3** shiori 結線＋死活監視 | connect closure に `Shiori3Client`＋`HelperLifecycle` 監視／死活を kanade へ | `spawn_shiori_actor` は `HelperHandle` のみ・`HelperLifecycle` 未結線・監視ループ無 | **Missing**（死活監視結線）＋ **Constraint**（`HelperHandle`↔`HelperLifecycle` 再配置） |
| **R4** sakura dispatcher | 永続 inbox で StartTalk 受領→per-talk spawn・単一 slot・TalkDone 転送・stale 棄却・Close funnel・sink 注入口 | `spawn_talk`（per-talk）・`kanade Sender<StartTalk>`（永続端）・`MockSink` | **Missing**（dispatcher 常駐アクター全体が新規） |
| **R5** ticker | kanade へ毎秒 Tick／active talk へ経過秒 Tick／差し替え可能 | `MonotonicMs`／`SakuraMsg::Tick(f64)`／`dola::runtime::clock` | **Missing**（ticker 全体が新規・差し替え抽象の設計） |
| **R6** 終了統括 | Close→drain→join／kanade 停止観測→Unload→`request_clean_shutdown`／exit 0 | `request_clean_shutdown` 存在・但し `ShioriMsg::Unload` はスタブ | **Missing**（正規 unload 結線・現状スタブ差し替え） |
| **R7** 決定論 spine e2e | testdll fixture＋記録 sink・注入 Tick・sleep 不使用・主要経路網羅 | kanade/sakura の決定論テスト作法・`MockSink`・testdll fixture 資産 | **Missing**（結線層 e2e の新規作成）＋ **Unknown**（testdll を x64 で扱う可否＝i686 成果物前提） |
| **R8** env gate 実 pasta＋app smoke | 実 emo2 OnBoot 一周／`open_startup_window` シーム経由で smoke 維持／ダミー窓維持 | smoke テスト・実 pasta 資産（emo2）・`AREKA_APP_SMOKE_EXIT_MS` | **Missing**（結線を smoke へ非破壊で挿す）＋ **Constraint**（private シームの昇格 or 内部結線） |

### 複雑度シグナル

- 単純 CRUD ではなく **アクター結線ワークフロー＋契約整合**。外部連携（32bit helper・実 pasta）は既存資産で吸収済みだが、ghost 層が初めてそれらを 1 本に束ねる。
- 決定論テストの網羅（注入 Tick・sleep 不使用・全断線経路）が記憶 `deterministic-test-coverage-mandate` で必達——テスト設計が実装と同等の重み。

---

## 3. 実装アプローチの選択肢（決定しない・トレードオフ提示）

### 3.A WS-A: talk 契約の一本化

本仕様の最初の地雷。**両クレートに依存辺が無い**ため「kanade 正本化」の具体形は自明でなく、以下の直交する 2 軸で分岐する。

#### 軸1: 統一型の物理的所有位置

- **A1: kanade を正本にし sakura が kanade へ依存**（`areka_sakura::contract` が `pub use areka_kanade::talk::{StartTalk, TalkDone, ...}`）。
  - ✅ roadmap の「kanade 正本」に文字通り従う。単一定義。
  - ❌ **新しい依存辺 sakura→kanade を作る**（kanade は運行系・sakura は再生系で、依存方向としては逆に見える／循環リスクは無いが層順が不自然）。sakura が kanade のビルドを引き込む。
- **A2: 共有型を下位クレート（`areka-actor` 等）へ昇格**し、kanade・sakura とも同一定義を参照。
  - ✅ 依存方向が自然（両者とも既に `areka-actor` 依存）。層としても talk 授受契約は「アクター基盤の語彙」に近い。
  - ❌ `areka-actor`（純粋アクター原語）にドメイン型（talk）が混じる——責務境界の議論が要る。`ReplySender` は既にここにあるので親和性はある。
- **A3: ghost 結線層に変換アダプタを置き、両クレートの型は据え置き**（統一せず dispatcher が翻訳）。
  - ✅ kanade/sakura を一切改変しない（凍結最小）。
  - ❌ **R1「単一正本へ一本化」に反する**（二重定義が残る）。要件が明示的に統一を要求しているため、この案は要件不適合の疑い＝要件ディスカッションで確認対象。

> 記憶 `areka-commit-as-you-go`／`canonical-not-minimal-lifecycle` は「隣接クレート増分を厭わない・小細工を避け正規実装」を支持——A1/A2 が整合、A3 は非推奨寄り。

#### 軸2: TalkDone の配送方式（reply oneshot 同梱 vs kanade inbox 転送）

現状: sakura は `StartTalk.reply: ReplySender<TalkDone>` へ oneshot 返信。kanade は `KanadeMsg::TalkDone` を自 inbox で受領（reply 概念なし）。

- **B1: kanade inbox 転送へ一本化**（`StartTalk` から reply を外し、dispatcher が sakura の TalkDone を `KanadeMsg::TalkDone` へ転送）。
  - ✅ kanade の既存受領経路（`KanadeMsg::TalkDone`）をそのまま使える。dispatcher が「per-talk reply を受けて kanade inbox へ橋渡し」する自然な役割（R4.3 と一致）。
  - ✅ 単一 slot・stale 棄却（R4.4）を dispatcher が talk_id で判断しやすい（inbox に集約）。
  - ❌ sakura の `StartTalk` から reply を除去＝sakura 側の授受面改稿・テスト追随。
- **B2: oneshot 同梱を維持し dispatcher が reply を受けて転送**。
  - ✅ sakura の `StartTalk`/`spawn_talk` を無改変（reply 同梱のまま）。
  - ❌ kanade の `Sender<StartTalk>` には reply を積めない（kanade の StartTalk に reply が無い）＝**dispatcher が reply を合成して spawn_talk へ渡す**必要。kanade→dispatcher は reply 無し・dispatcher→sakura は reply 有り、という非対称を dispatcher が吸収。
  - ❌ reply の `ReplyReceiver` を dispatcher スレッドで待つ形になり、単一 slot・並行 Tick 供給との両立（ブロッキング recv の位置）が設計論点。

> **観察**: B2 は「kanade の StartTalk に reply が無い」ため、いずれにせよ dispatcher が reply を**生成して** spawn_talk へ渡す。よって reply の存在は sakura↔dispatcher 間のローカル事情に収まり、kanade 契約からは隠せる。B1 は sakura 契約も揃える（より全体整合）。**どちらでも dispatcher が非対称の吸収点**になる点が共通。

**推奨の方向（決定でなく設計判断の材料）**: 軸1=A2（共有型を下位へ）× 軸2=B1（inbox 転送・sakura reply 除去）が「単一正本・自然な依存方向・kanade 既存受領経路の温存」を最も満たす。ただし A1（kanade 直参照）＋ B2（reply 温存）は sakura 改変を最小化できる。要件ディスカッションで軸ごとに確定すべき。

### 3.B sakura dispatcher（新規常駐アクター）

**Option B（新規コンポーネント）で確定的**——既存に相当物が無く、責務（永続 inbox↔per-talk 非対称吸収・単一 slot・転送・Close funnel）が明確に独立。

- 構造: `spawn_actor("sakura-dispatcher", ...)` で常駐。inbox は kanade の `Sender<StartTalk>` が指す先。
- 単一 slot: `Option<TalkHandle>`（active talk）を body ローカルに保持。新 StartTalk 受領時に前の active を Close＋join してから spawn（記憶 `areka-interrupt-single-close-funnel` の単一 Close funnel）。
- TalkDone 転送: 配送方式（3.A 軸2）に従う。stale は talk_id 比較で棄却（R4.4・kanade R6.6 と対）。
- sink 注入口: 構築時注入（`spawn_dispatcher(surface_sink, text_sink, ...)`）か setter か——**構築時注入が MockSink／実 sink の差し替えに素直**（seriko/emo-text-layer が後で同じ口に挿す）。setter は途中差し替えを許すが本仕様では不要。→ 設計判断（brief クロスユニット契約が「構築時注入 or setter」を design 判断と明記）。
- Tick 供給との関係: dispatcher は active talk の `TalkHandle.inbox` を握る。ticker が per-talk Tick を送るには active talk の inbox が要る——**dispatcher が保持する inbox を ticker がどう参照するか**が設計論点（3.C 参照）。

### 3.C ticker（差し替え可能な時刻供給）

**Option B（新規）**。2 系統（kanade 毎秒 `MonotonicMs` Tick／active talk への `f64` 経過秒 Tick）を養う。

- **C1: 単一 ticker スレッドで両方を養う**。1 秒 cadence で `KanadeMsg::Tick{now}` を送り、同時に active talk へ経過秒。
  - ❌ per-talk elapsed の粒度が 1 秒では粗い可能性（brief 指摘: `\w` は ms 級・OnSecondChange は 1 秒）。sakura Tick の解像度要求を design で確定（記憶なし＝要調査）。
- **C2: 2 系統を分離**（kanade 用 1 秒 ticker＋talk 用高解像度 ticker）。
  - ✅ 解像度を系統ごとに最適化。❌ スレッド 2 本・active talk 切替時の talk ticker 再結線が複雑。
- **差し替え抽象（R5.3/5.4 必達）**: 決定論テストが Tick を外部注入するため、ticker は「実クロック実装」と「注入実装」を差し替え可能な抽象（trait or channel）として公開。
  - **C-inject-A**: ticker を trait 化し本番＝実クロック／テスト＝手動 step。
  - **C-inject-B**: 「ticker は Tick を送る channel の産出者」に留め、テストは ticker を起動せず `KanadeMsg::Tick`／`SakuraMsg::Tick` を直接 inbox へ注入。
    - ✅ 既存の kanade/sakura テスト作法（inbox へ直接注入）と同型・追加抽象が最小。spine e2e は「ticker を起動しない」だけで sleep 不使用が成立。→ **C-inject-B が既存決定論テスト資産と最も整合**（設計判断の材料）。
- **per-talk elapsed の供給経路**: active talk の inbox は dispatcher が握る。ticker→dispatcher→talk か、ticker が dispatcher から inbox を借りるか、dispatcher 自身が talk へ Tick を中継するか——**dispatcher が Tick 中継を兼ねる**と ticker は「dispatcher と kanade へ 1 種類ずつ Tick を送る」だけに単純化できる（active talk の所在を知るのは dispatcher のみ）。要 design。

### 3.D shiori 結線＋死活監視（R3・既存 `spawn_shiori_actor` の隣接増分）

- connect closure: `mount(MountModel.shiori.dir/file)` → helper 起動 → `Shiori3Client`＋`HelperLifecycle` を `ShioriConnection` に格納。
  - **ギャップ**: 現 `ShioriConnection` は `helper: HelperHandle`。`HelperLifecycle::new(HelperHandle)` で包む必要がある。`request_clean_shutdown` と `status()`/`report_failure` は `HelperLifecycle` の API。
  - **Option A（既存 spawn_shiori_actor を拡張）**: `ShioriConnection` に `HelperLifecycle` を持たせ、`ShioriMsg::Unload` アームで `request_clean_shutdown` を呼ぶ／受信ループ内で `status()` を周期観測し `Exited(Abnormal|Terminated)` を `ShioriDown` として kanade へ通知（R3.4）。
    - ✅ 既存 actor の唯一の host32 import 点（`real.rs`）に閉じる。
    - ❌ kanade クレートに手を入れる（隣接増分・凍結境界外＝許容）。受信ループへの死活ポーリング混入は `run_shiori_loop` の blocking recv と両立させる設計が要る（`recv_timeout` 化 or 別監視スレッド）。
  - **Option B（ghost 層で監視を別アクター化）**: shiori actor は現状維持し、ghost が `HelperLifecycle` を別途保持して死活監視スレッドを回す。
    - ❌ `HelperHandle`／`HelperLifecycle` の所有が actor と ghost に割れ、drop/teardown の責務が二重化。connect closure がスレッド内で helper を生成する現構造（`!Send`）と相性が悪い。
  - **観察**: R3.5「プロトコル・IPC・語彙を変更せず消費のみ」は守れる（`request_clean_shutdown` 等は既存 API のまま）。ただし `spawn_shiori_actor` の**シグネチャ／`ShioriConnection` の構造**は本仕様が触ってよい面（授受面ではなく結線面）——Option A が正規実装に近い。**現 `ShioriMsg::Unload` スタブ（`Unloaded` 即返し）を正規経路へ差し替えるのが R6.2/R6.3 の中核**（記憶 `canonical-not-minimal-lifecycle`）。

### 3.E ghost 結線層の所有先（R2 全体・R8）

- **E1: 新規 `areka-ghost` クレート**（`crates/areka-ghost/`）を作り、areka バイナリがそれに依存して `open_startup_window` から呼ぶ。
  - ✅ 責務分離（結線ロジックが独立テスト可能・spine e2e をクレート内 `tests/` に置ける）。areka バイナリは薄いまま。
  - ✅ `main.rs` の private シームは「ghost 結線を呼ぶ」1 行に置換——smoke 非破壊で挿しやすい。
  - ❌ クレート新設・workspace member 追加。ただし本 workspace は `crates/*` glob ゆえ追加は容易。
- **E2: areka バイナリ内に結線モジュールを追加**（`crates/areka/src/ghost/` 等）。
  - ✅ 新クレート無し。`open_startup_window` の private 性のまま内部結線できる。
  - ❌ 結線層の独立テストが binary crate 制約（`tests/` は統合テストのみ・in-source 中心）に縛られる。spine e2e の testdll fixture 結線が binary crate では扱いにくい可能性。
- **観察**: 決定論 spine e2e（R7）を独立した integration test として組みやすいのは E1。kanade/sakura が独立クレート＋`tests/` を持つ既存パターンとも整合。ただし `open_startup_window`/`WinApp`/`ConfigInputs` は areka バイナリ private ゆえ、E1 でも「バイナリ側の薄い結線（シーム消費）」は areka に残る。→ **ハイブリッド**（結線ロジック＝新クレート／シーム消費＝areka main.rs）が現実的。設計判断。
- **R8 の私有シーム問題**: `open_startup_window(&WinApp)` は private。ghost 結線を挿すには (i) main.rs 内で ghost クレートを呼ぶ、(ii) シームを pub 化して外から差し替え、のいずれか。smoke（`AREKA_APP_SMOKE_EXIT_MS`・ダミー窓維持）を壊さないため、**ghost 起動と smoke ゲートの共存**（ダミー窓は維持しつつエンジンを起動）を設計で確定。

### 3.F 終了統括（R6）

- Close→drain→join を全エンジンへ（`areka-actor` 規約・`ActorHandle::join`）。順序: kanade 停止観測→shiori Unload（`request_clean_shutdown`）→全 join→exit 0。
- **Option A（ghost 層が停止オーケストレータを所有）**: ghost が各 `ActorHandle` と kanade/dispatcher/ticker/shiori の Sender を保持し、停止シーケンスを駆動。
  - kanade の停止観測（`KanadeMsg` 経由で kanade が終端に達したこと）をどう ghost が知るか——kanade が停止時に何かを ghost へ通知する経路が要る（現 `KanadeMsg` に「kanade 停止完了」外向き通知は見当たらない＝要確認/設計）。
- **正規実装の原則**: stand-in exit(0) 禁止・`request_clean_shutdown` の正規経路のみ（記憶 `canonical-not-minimal-lifecycle`）。現状スタブ Unload の差し替えが前提。失敗は log-first＋`Err`（R6.5・記憶 `areka-log-first-no-silent-failure`）。

### 3.G 決定論 spine e2e（R7）

- 記録 sink＝sakura の `MockSink`（`Arc<Mutex<Vec<TalkCue>>>`・クロススレッド FIFO 観測）を流用可能。
- testdll fixture: host32 の testdll 資産で SHIORI 応答をスクリプト化。**注意（記憶 `workspace-test-needs-i686-host32-artifacts`）**: workspace test 緑化には host-32 の i686 成果物の事前ビルドが要る。in-process 32bit DLL load 系は x64 不可・別プロセス spawn の host e2e は x64 可。spine e2e が testdll をどのプロセスモデルで駆動するか（in-proc か spawn か）で i686 前提が変わる＝**Research Needed**。
- 注入 Tick・sleep 不使用は 3.C の C-inject-B（ticker を起動せず inbox へ直接注入）で成立。主要経路（boot 成功／SHIORI 死活／close 握手／close deadline／全断線）を実行テスト化（記憶 `deterministic-test-coverage-mandate`）。

---

## 4. Out-of-Scope（設計フェーズへ繰越す Research Needed）

1. **WS-A 型所有位置**（3.A 軸1: kanade 直参照 A1／areka-actor 昇格 A2／変換アダプタ A3）——依存辺の方向と責務境界。要件ディスカッションで確定。
2. **TalkDone 配送方式**（3.A 軸2: inbox 転送 B1／oneshot 温存 B2）——sakura 授受面をどこまで改稿するか。
3. **`HelperHandle`↔`HelperLifecycle` の再配置と `ShioriMsg::Unload` スタブの正規化**（3.D/3.F）——`request_clean_shutdown`/`status()`/`report_failure` の結線点。
4. **ticker の系統構成と per-talk elapsed 供給経路・解像度**（3.C）——sakura Tick(f64) の解像度要求（`\w` ms 級 vs 1 秒 cadence）。ukadoc `list_shiori_event` の OnSecondChange は 1 秒周期の確認のみ。
5. **ghost 層の所有先**（3.E: 新クレート E1／areka 内モジュール E2／ハイブリッド）と **private シーム `open_startup_window` の扱い**（内部呼び出し vs pub 化）。
6. **kanade 停止完了の外向き通知経路**（3.F）——ghost が「kanade が終端に達した」ことを観測する手段。
7. **spine e2e の testdll プロセスモデルと i686 成果物前提**（3.G・記憶 `workspace-test-needs-i686-host32-artifacts`）。
   → **要件ディスカッション #2 で決着（2026-07-06）**: 決定論 spine e2e は **偽 SHIORI アクター境界**（`spawn_shiori_actor` の connect closure に台本化した偽 `ShioriConnection` を注入）で駆動し、**純 x64・i686 非依存**で全経路を網羅する（R7.1/R7.6）。実 32bit helper・実 testdll・実 pasta は **env ゲート下の opt-in 追験**に限定（R8.4）。設計に残るのは「偽 SHIORI アクターの具体形（台本化 API・死活/断線の演出）」のみ。
8. **KanadeConfig の shell 名の出所**（R2.3）——`MountModel.names`（name/sakura.name）のどれを shell_name（OnBoot Ref0）に写すか。brief は「shell descript の name 系」と示すが、`resolve` が返す `GhostNames` の具体対応を design で確定。
9. **`resolve` の `default_encoding` 供給**（R2.1）——ghost が `DefaultEncoding`（SSP 準拠 ANSI 既定）をどう決めるか。

---

## 5. 実装複雑度・リスク

| ワークストリーム | 工数 | リスク | 一言根拠 |
|---|---|---|---|
| WS-A 契約統一 | **M**（3〜7 日） | **Medium** | 型統一自体は機械的だが、両クレート無依存ゆえ所有位置の設計判断＋両クレートのテスト追随が必要。3 値化で kanade の quit:bool 消費部（close 握手）を改稿。 |
| sakura dispatcher | **M** | **Medium** | 新規常駐アクターだが既存 `spawn_actor`/`spawn_talk` 原語で組める。単一 slot・stale 棄却・Close funnel・sink 注入・Tick 中継の結線が肝。 |
| ticker | **S〜M** | **Medium** | 差し替え抽象は C-inject-B で最小化可能。per-talk elapsed 解像度が未確定リスク。 |
| shiori 死活監視＋正規 unload | **M** | **High** | 現 Unload スタブの正規化・`HelperLifecycle` 再配置・blocking recv と死活ポーリングの両立・helper 実プロセス絡み。凍結語彙は守れるが結線面の改変が広い。 |
| 終了統括 | **M** | **Medium** | 停止順序は規約通りだが kanade 停止観測経路が未確定。正規 clean shutdown 必達。 |
| 決定論 spine e2e | **M〜L** | **High** | 全断線・deadline・死活を注入 Tick で網羅・sleep 不使用。testdll の i686 前提・プロセスモデルが未確定リスク。 |
| env gate 実 pasta＋smoke 維持 | **S〜M** | **Medium** | 実 pasta 資産は既存。private シームへ非破壊で結線する形の確定が要。 |

**全体**: 工数 **L（1〜2 週）**、リスク **Medium〜High**（結線面が広く、shiori 正規化と spine e2e の i686/プロセスモデルが最大の不確実性）。ただし全アクター API・凍結面・記録 sink・決定論テスト作法が既存資産として揃っており、**新規外部依存はゼロ**（std mpsc・既存クレート内結線）で成立する見込み。

## 6. 設計フェーズへの申し送り

- **推奨の初期方向（要確認）**: WS-A は 3.A 軸1=A2（共有型を `areka-actor` へ昇格）× 軸2=B1（inbox 転送・sakura reply 除去）が単一正本・自然な依存方向・kanade 既存受領経路温存を最も満たす。dispatcher は構築時 sink 注入＋Tick 中継兼務。ticker は C-inject-B（テストは inbox 直接注入）。ghost 層は E1/E2 ハイブリッド（結線ロジック新クレート＋areka main.rs の薄いシーム消費）。shiori は 3.D Option A（`spawn_shiori_actor` 拡張・Unload 正規化）。
- **持越し研究項目**: 上記 §4 の 1〜9。特に (3) 現 Unload スタブの正規化と (7) spine e2e の i686 プロセスモデルはリスク源として design で先に潰す。
- **凍結遵守**: `TalkCue`/`SurfaceSink`/`TextSink`/`cue_target_of`/dola cue／`Shiori3Client`/`RequestError`/`LifecycleReport` 語彙は不改変。触るのは talk 授受面（`StartTalk`/`TalkDone`/`SakuraMsg::Start`）と shiori 結線面（`ShioriConnection`/`spawn_shiori_actor`/`ShioriMsg::Unload`）と ghost 新規結線。

---

## 7. 設計フェーズ: ディスカバリ追補と設計決定（2026-07-06・design 生成時）

> §4 の持越し研究項目 1〜9 を本節で決着した。設計の正本は design.md（本節は決定の背景・比較記録）。

### 7.1 ディスカバリ追補（実コード検証・light discovery）

- **kanade `talk.rs` は切り出し前提で設計済み**（rustdoc 実測）: 「本ファイルは std のみに依存…契約クレートへ切り出す作業は、このファイルの機械的な移動だけで完結する」（DD-1）。本仕様が想定されていた「2 例目の契約消費者」であり、切り出しの執行が kanade 自身の設計意図に一致する。
- **`ShioriMsg::Unload` スタブの差し替え点も設計済み**（`real.rs` 実測）: 「境界契約（ShioriMsg::Unload／Unloaded）は不変ゆえ、正規経路確立時にこの 1 アームのみ差し替えればよい」。差し替え先 `HelperLifecycle::request_clean_shutdown(&ParentMessageWindow) -> Result<ExitKind, ShutdownError>` は `&mut self` を要する（trait 側 `&mut` 化が必要）。
- **`real.rs` には private の `ShioriBackend` trait が既在**: `get`/`notify` を持ち、fake backend が同一 runner（`run_shiori_loop`）で検証済み。公開化＋`unload`/`status` 追加が最小増分。
- **`ShioriConnection` の構築は実 helper 子プロセスを要する**: `HelperHandle` は `Child` を所有し、公開コンストラクタは `spawn()`（プロセス起動）のみ。**「偽 ShioriConnection」を literal に作るには子プロセスが要る＝純 x64 決定論と両立しない**（§7.2 DD-D の根拠）。
- **接続手順の実証資産**: `areka-kanade/tests/kanade/real_helper_test.rs` の `connect_real_helper`（`ParentMessageWindow::create` → `spawn` → `pump_until_hello_or` → LOAD ack）が実 pasta で GO 済み。env gate 慣行（`HOST32_PASTA_DLL` silent skip／DLL 不在 fail／`HOST32_HELPER_EXE`→target 探索）も同ファイルが正本。
- **shell descript 読解の道具は揃っている**: `charset::decode(&[u8], DefaultEncoding) -> String`＋`kv::parse_kv(&str) -> BTreeMap`（areka-parsers foundation・公開済み）。
- **ukadoc 確認**: OnBoot Reference0＝「起動時のシェル名」（`ukadoc:list_shiori_event:OnBoot:1`）。`GhostNames`（ゴースト側 descript の name 系）はシェル名ではない → shell/master/descript.txt の `name` が値源（§7.2 DD-H）。
- **`spawn_actor` は inbox channel を内部生成**する（外部 Receiver 注入不可・`ActorHandle` は外部構築不可）——相互 Sender 要求の循環（kanade⇄dispatcher・kanade⇄shiori）は spawn 原語の外で中継チャンネルを作って解くしかない（§7.2 DD-C の根拠）。
- **`MockSink` は Clone 非実装**（凍結面 sink.rs）——dispatcher の per-talk sink 注入（Clone 要求）には e2e 側で Clone な RecordingSink を別定義する（sink.rs 不改変）。

### 7.2 設計決定（§4 持越し 1〜9 の決着）

#### DD-A（§4-1）: 統一型の所有位置＝新規契約クレート `areka-talk`（A2 の実現形）

- 代替案: A1（sakura→kanade 依存）／A2-actor（areka-actor へ昇格）／A2-new（新規契約クレート）／A3（変換アダプタ・要件 1.7 で除外済み）。
- 選択: **A2-new**。根拠: (i) A1 は再生系→運行系の逆向き依存辺と host32 ビルド引き込みを生む、(ii) A2-actor は actor-foundation 仕様が凍結した公開面（「これ以外の公開面を持たない」）に違反する、(iii) kanade DD-1 が「契約クレートへの機械的移動」を明示的に予定しており、A2-new はその執行である。「kanade 正本」（R1.1）は意味論（kanade の形状＋reason 3 値）の正本性として実現し、物理所在は areka-talk とする。
- トレードオフ: workspace クレート +1（`crates/*` glob ゆえ追加容易）。

#### DD-B（§4-2）: TalkDone 配送＝B1（inbox 転送・reply 撤去）＋ done ポートは汎用 Sender

- `StartTalk` から `reply` を撤去（kanade 形状に統一）。sakura の完了通知は `spawn_talk<D: From<TalkDone>>(start, done: Sender<D>, …)` の**汎用 Sender ポート**へ移す。dispatcher は `D = DispatcherMsg` で自身の inbox へ巻き取り、`KanadeMsg::TalkDone` へ転送する（R4.3）。sakura テストは `D = TalkDone`（std の恒等 `From`）で追随。
- 高々 1 回保証は `ReplySender` の move-consume から `Option<TalkState>::take()`（既存機構）へ等価移行。
- 却下: B2（oneshot 温存）——dispatcher が単一 inbox で `ReplyReceiver` を同時待ちできず、per-talk 監視スレッドが要る（スレッド浪費・停止複雑化）。

#### DD-C（§4-6 併合）: 結線循環の解消＝中継チャンネル＋汎用 relay／kanade 停止観測＝join

- `spawn_kanade`（`sakura: Sender<StartTalk>`）と `spawn_shiori_actor`（`on_down: Sender<KanadeMsg>`）のシグネチャを**不変**に保ったまま、ghost が素の mpsc を 2 本張り `spawn_relay<A, B: From<A>>` で変換転送する（start-relay／down-relay）。relay は上流全 Sender drop で自然終了（明示 Close 不要）。
- **kanade 停止完了の観測（§4-6）は `ActorHandle::join`**。新たな外向き通知経路は作らない——shutdown は ghost が能動的に駆動する系列であり、join が停止規約の正準観測である。kanade 自発停止（quit talk）済みの場合、shutdown の ForceQuit 送出は Err になるが kanade は自身の終了系列で Unload 済み＝冪等に join 工程へ進む。
- dispatcher の per-talk done ポート用 self-sender は `reply_channel` で spawn 時に body へ受け渡す（enum を汚さない）。

#### DD-D（§4-3・§4-7）: 偽装シーム＝`ShioriBackend` 公開化＋`Box<dyn ShioriBackend>` connect／Unload 正規化・死活監視は同一 runner 内

- `spawn_shiori_actor` の connect を `FnOnce() -> Result<Box<dyn ShioriBackend>, String>` へ一般化する。**R7.1 の「偽 ShioriConnection」の実現形**: 実 `ShioriConnection` は `Child` 所有ゆえプロセスなしで構築不能——注入点（connect closure）と呼出形は要件どおり保ち、**注入される型を backend 抽象へ持ち上げる**ことで純 x64・プロセス spawn ゼロの台本 fake（ScriptedShioriBackend）を成立させる（R7.6）。本番は `impl ShioriBackend for ShioriConnection`（`helper: HelperLifecycle` 化・`ConnectionBackend` 中間構造は廃止）。
- `ShioriBackend` に `unload(&mut self) -> Result<ExitKind, ShutdownError>`・`status(&mut self) -> HelperStatus` を追加。Unload アームは `request_clean_shutdown` の正規経路へ差し替え（`Ok(Clean)`→info＋`Unloaded`／`Ok(他)`→warn＋`Unloaded`／`Err`→error＋`Failed(Ipc)`）。
- 死活監視（§4-3・**設計ディスカッション #2 で簡素化**）: タイマー poll（recv_timeout 500ms 案）を廃し、受信ループは blocking recv のまま**メッセージ到達時のみ `status()` を確認**。到達間隔は kanade Steady/Closing の Tick pump（OnSecondChange 毎秒・steady.rs/close.rs 実測）が ≤1s を構造保証し、request 失敗（`classify_failure`）が第二の網。`Exited` 初回観測で `ShioriDown` を一度だけ送る（sticky・unload 成功後は発火しない）。検出機構が 1 本＝テスト経路と本番経路が完全一致。`on_down` は接続成功後もループ中保持へ変更（**是正・設計ディスカッション #1**: 保持は kanade→shiori→down-relay→kanade の Sender 環を作り「全 Sender drop 停止」は環の解体後にのみ成立——「down-relay 仲介で解消」は誤りだった。kanade rustdoc は解体後伝播の旨へ更新・design「アクター別の停止経路」マトリクスが正本）。
- 却下: 監視の別アクター化——`HelperLifecycle` の所有が actor と ghost に割れ、`!Send` 窓との teardown 責務が二重化する。

#### DD-E（§4-4）: ticker＝単一スレッド・2 cadence・C-inject-B（テストは ticker 不起動）

- 単一 ticker スレッドが `base_interval`（既定 **50ms**＝`\w` の 50ms 単位に一致）で `DispatcherMsg::Tick{now}` を、`kanade_interval`（既定 **1000ms**＝OnSecondChange 周期）で `KanadeMsg::Tick{now}` を送る。clock は注入可能（既定 GetTickCount64＝`MonotonicMs` rustdoc の正準）。
- **per-talk 経過秒の供給経路は dispatcher が中継**: active talk の inbox を知るのは dispatcher のみ。dispatcher が `Tick{now}` 受領時に base（初回 Tick で確定・elapsed=0.0 起点）からの経過秒を `SakuraMsg::Tick(f64)` へ換算して送る。
- 決定論（C-inject-B）: spine e2e は `TickerMode::Disabled` で ticker を起動せず、`KanadeMsg::Tick`／`DispatcherMsg::Tick` を inbox へ直接注入する（既存 kanade/sakura テスト作法と同型・sleep 不使用が構造的に成立）。
- 却下: C2（talk 用高解像度 ticker の分離）——スレッド 2 本と active 切替時の再結線が複雑で、50ms 単一 cadence で M1 要求（記録 sink）に足りる。
- **絶対境界スケジューリング（設計ディスカッション #2）**: 発火は OS 時計の絶対グリッド（50ms／1000ms 境界）へ整列——相対経過でなく境界計算＝累積ドリフトなし（OS 時計に正確・開発者要求）。catch-up は境界スキップで各系統 1 発（burst なし）。副次効果: グリッド整列により複数ゴーストの ticker は共有コンポーネント無しで自然同期＝**上位コンダクター不要・kanade の役割にもしない**（kanade は注入時刻の消費者・OnSecondChange の意味論的発行者は kanade の Tick pump のまま）。

#### DD-F（§4-5）: ghost 層の所有先＝新規クレート `areka-ghost`＋areka main の薄い結線（ハイブリッド E1）

- 結線ロジック（boot/shutdown/dispatcher/ticker/relay/config/wiring/sink）と spine e2e は `crates/areka-ghost/` に置く（integration test を kanade/sakura と同型の `tests/` に組める）。areka main は `boot`（**非致命**: 失敗は warn/error＋骨格継続）と `app.run()` 復帰後の `shutdown(System)` のみ。
- **`open_startup_window` シームは不改変**（pub 化しない・ダミー窓と smoke ゲート維持＝R8.2/8.3）。ghost 結線はシームの外（main 本体）に置く——「シームの周りの結線を所有」という brief の境界に一致。

#### DD-G（§4-7 続き）: spine e2e のプロセスモデル＝プロセスレス（確定執行）

- 要件ディスカッション #2 の決着（偽 SHIORI 境界・純 x64）を DD-D のシームで執行。シナリオは S1 boot 成功／S2 接続失敗／S3 helper 死活／S4 close 握手（`ExitKind::Clean` 相当）／S5 close deadline／S6 全断線＝**段階的解体**（設計ディスカッション #1 で再定義: dispatcher Close→kanade Close→残 senders drop→切断伝播の有界 join。純粋な全 drop 一斉解放は Sender 環ゆえ構造的に不成立）。
- **駆動口の確定（設計ディスカッション #2）**: S3 駆動＝kanade へ Tick 注入→Steady pump の OnSecondChange 到達時チェックで検出（本番同一経路・実時間ゼロ）。S5＝注入 now で既定 deadline 30_000ms を数値的に跨ぐ（短縮構成不要・config への override 注入点は設けない）。`GhostParts`＝`{kanade, dispatcher, ticker(Option), 全 handles}`・shiori 投函端なし（runtime 非保持の正本どおり）。i686 成果物は `cargo test --workspace` の前提から外れる（実 helper は R8.1 の env gate のみ）。

#### DD-H（§4-8）: `KanadeConfig.shell_name`＝shell descript の `name`（フォールバック＝shell ディレクトリ名）

- ukadoc: OnBoot Reference0＝「起動時のシェル名」。値源は `MountModel.shell.dir` 直下 `descript.txt` の `name` キー（`charset::decode`＋`parse_kv`）。読取不能・欠落は warn＋shell ディレクトリ名（通常 "master"）へフォールバック（boot を落とさない）。`GhostNames`（ゴースト name 系）は誤りゆえ使わない。
- baseware 情報: `baseware_name = "areka"`（`KanadeConfig::new` 既定）・`baseware_version = env!("CARGO_PKG_VERSION")`（workspace 統一 version）。

#### DD-I（§4-9）: `DefaultEncoding` の供給＝`GhostBootOptions` の呼び出し側指定・既定 `Ansi`

- SSP 準拠の既定は ANSI（記憶 areka-descript-encoding-ishiori-utf8: 既定ハードコード禁止→resolve 引数は呼び出し側供給）。ghost は options の既定値として `Ansi` を置き、テスト・将来の設定 UI が上書きできる。emo2（UTF-8・charset 宣言あり）は prescan 宣言優先で正しく読める（宣言なしレガシー＝ANSI とみなす既定に一致）。

#### DD-J（追加決定）: kanade の 3 値写像＝`Quit`→quit 経路／`Ended`・`Interrupted`→非 quit 経路

- 意味論保存の最小写像。`Interrupted` はM1 ではユーザー中断結線が存在せず、dispatcher の slot 差し替え由来分は stale として棄却されるため kanade へ実質届かない——防御的に非 quit＋`info!` 観測とし、中断意味論の精緻化は input-events/idle-talk へ委ねる。既存テストは `quit:true→Quit`/`quit:false→Ended` の機械的置換。

#### DD-K（追加決定）: sink 注入＝構築時注入＋`Clone` 制約／本番既定は `LogSink`

- dispatcher は `S: SurfaceSink + Clone + Send + 'static`（text 同様）を構築時注入し talk ごとに clone する（setter なし——途中差し替えの実需なし）。後続 M-boot 統合の実 sink（channel Sender ベース）は Clone を自然に満たす。本番既定は ghost 提供の `LogSink`（tracing 出力・無蓄積）——`MockSink`（無限蓄積）を本番に置かない。e2e は Clone な RecordingSink をテスト側で定義（凍結 sink.rs 不改変）。

### 7.3 シンセシス記録

- **一般化**: 転送問題 2 箇所（StartTalk→dispatcher・ShioriDown→kanade）を単一の `spawn_relay<A, B: From<A>>` へ一般化（実装 1 つ・実需 2 例）。`From` ベースの投函変換は spawn_talk done ポートとも同型で、契約を汚さない受け渡しの統一原語になる。
- **build vs adopt**: 新規外部依存ゼロ。全て workspace 既存資産（areka-actor 原語・parsers foundation・host32 API・real_helper_test の connect 手順）の組み合わせで成立。
- **単純化**: (i) `ConnectionBackend` 中間構造を廃止し `ShioriConnection` に直接 impl、(ii) ticker を trait 化せず「起動しない」ことで決定論を成立（C-inject-B・追加抽象ゼロ）、(iii) kanade/sakura/actor の公開シグネチャ変更を relay で回避、(iv) dispatcher の graceful drain は「Close funnel＋join」のみ（監督ツリー等は導入しない）。

### 7.4 リスクと緩和（設計後更新）

- **shiori actor 改稿の広さ**（High→Medium）: 変更は real.rs 1 ファイルに閉じ、既存 fake-backend テスト機構が同一 runner 検証を継承する。Unload 正規化・死活監視は scripted backend 単体テスト＋spine e2e S3/S4 で檻に入れる。
- **shutdown のハング**: 全 join は有界待機のテスト（spine e2e・独立レビューの複数回実行）で検出。設計上、各アクターの停止経路（Close／全 Sender drop）を系列に明記済み。
- **`Interrupted` の意味論保留**: DD-J の防御写像＋`info!` 観測で M1 を閉じ、Revalidation Trigger（areka-talk 形状・reason 消費）として記録。
- **app 起動時の ghost boot 失敗ログ**: 既定プレースホルダ ghost_root は不在＝毎回 warn が出る。骨格モード（ghost なし）を明示する 1 行ログで意図を可視化（silent skip にしない）。

### 7.5 参照

- ukadoc `list_shiori_event` OnBoot（Reference0＝起動時のシェル名）／OnSecondChange（1 秒周期）
- `crates/areka-kanade/src/talk.rs`（DD-1 切り出し前提）・`src/shiori/real.rs`（Unload スタブ差し替え点・ShioriBackend）・`tests/kanade/real_helper_test.rs`（connect 手順・env gate 慣行）
- `crates/areka-sakura/src/contract.rs`／`drive.rs`（授受面・高々 1 回機構）・`src/sink.rs`（凍結面・MockSink）
- `crates/shiori-host32-host/src/lifecycle.rs`（`HelperLifecycle`／`request_clean_shutdown`／`ExitKind`）
- `crates/areka-parsers/src/package/`（`resolve`／`MountModel`）・`charset`／`kv` foundation
- `.kiro/steering/`（tech/structure/product）・記憶: areka-interrupt-single-close-funnel／canonical-not-minimal-lifecycle／deterministic-test-coverage-mandate／areka-log-first-no-silent-failure／areka-runtime-env-naming
