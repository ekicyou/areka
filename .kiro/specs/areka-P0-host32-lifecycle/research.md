# ギャップ分析: areka-P0-host32-lifecycle

> **フェーズ**: validate-gap（要件確定後・design 前）。本書は情報提供であり最終決定ではない。
> **調査日**: 2026-07-05。requirements.md・brief.md・上流実シンボル（`shiori-host32-host`／`-ipc`／`-testdll`／`-helper`）・steering を実地精査。
> **言語**: ja（spec.json.language）。

## 1. サマリ（3〜5点）

- **死活検出 seam は完成済み・監視ループ化と統一報告だけが未実装**。`poll_exit`／`poll_exit_kind`／`ExitKind{Clean/Abnormal(i32)/Terminated}`／`HelperHandle::terminate()`（冪等）／`RequestError{Handshake/Timeout/Ipc/Shiori}` はすべて既存（`process_host.rs`／`error.rs`／`client.rs`）。本仕様は**新規機構をほぼ生まず、既存 seam を常設監視・統一報告・周期/kill 試験へ「増分」する**性格が強い。effort は控えめ・risk は低〜中。
- **統一報告型（R2）が唯一の新規 API 設計判断**。R2 は「helper 死亡／タイムアウト／SHIORI エラー」を単一語彙で区別できる `Send` な報告データを求めるが、現状 `RequestError` は `!Send` 要素を含まず既に `Send` 相当・区別も保持。`ExitKind`（死活）と `RequestError`（request 失敗）は**別型で並存**しており、両者を突合する「統一報告型」が未在。これが design の中核判断（拡張 vs 包む型 vs 突合ヘルパ）。
- **ログ規律に既存資産との齟齬**。要件 R4.4／R5.5／R7.6 は失敗経路を `error!`＋`Err` で surface することを求めるが、**x64 host クレート（`shiori-host32-host`）は現状ログ機構を一切持たない**（`tracing` 依存なし・本体コードに `error!`／`eprintln!` 皆無、helper 側のみ `eprintln!`）。steering tech.md は「`tracing` を全体規約・subscriber 初期化はアプリ層」と規定。**`error!` の実体（`tracing` 導入）と host クレートへの依存追加が design 判断**。
- **周期運転・強制 kill 試験は既存 e2e パターンの合成で成立可能**。`shiori_request_e2e.rs`（HELLO pump → LOAD ack[1] → GET/NOTIFY → `poll_exit_kind`→None）と `error_paths.rs`（spawn → terminate → `poll_exit_kind` が非 Clean を bounded に観測）が、周期連打（R3）・kill 注入（R4）の**ほぼ完成した骨格**を提供。testdll fixture・helper 解決規約（silent-skip 禁止 panic）・HelperGuard・1 窓制約対処もそのまま流用できる。
- **凍結境界は自然に守られる構造**。`shiori-host32-ipc`（wire/framing/MsgTag/ResponseSlot/timeout）は cargo 依存で不透明利用され、本仕様のスコープは host クレート上位のみ。有限復帰は既存 `SMTO_ABORTIFHUNG`＋`AREKA_SHIORI_REQUEST_TIMEOUT_MS`（既定 60s・"0"=無限 debug）に乗る。改変誘因が構造的に無い。

## 2. 現状コードベース調査

### 2.1 対象クレート構成

`crates/shiori-host32-host/`（x64/arm64 ホスト側 transport・本仕様の主戦場）:

| ファイル | 役割 | 本仕様との関係 |
|---|---|---|
| `src/process_host.rs` | `spawn`／`HelperHandle`／`poll_exit`／`poll_exit_kind`／`ExitKind`／`terminate`／timeout 定数・env 解決 | **死活監視の土台（R1・R4・R5 の seam）** |
| `src/error.rs` | `SpawnError`／`HandshakeError`／`ShioriError`／`RequestError` | **統一報告語彙の土台（R2）** |
| `src/client.rs` | `Shiori3Client::get`／`notify`・`map_send_error`／`map_get_result` | **request 失敗語彙の出口（R2 で消費）** |
| `src/parent_window.rs` | `ParentMessageWindow`・`pump_until_hello_or`・`send_request` | 周期運転・kill 試験の親窓（R3・R4） |
| `src/shiori3.rs` | SHIORI/3.0 codec | 消費のみ（不改変） |
| `tests/shiori_request_e2e.rs` | 決定的 GET/NOTIFY e2e＋env-gate 実 pasta | **R3/R6 試験の原型** |
| `tests/error_paths.rs` | ハンドシェイク timeout／wedge／**helper 異常終了検出**／不正フレーム隔離 | **R4 kill 試験の原型** |
| `tests/shiori_load_e2e.rs` | LOAD e2e | 解決規約・1 窓制約の参照 |

隣接クレート: `shiori-host32-ipc`（**凍結**・cargo 依存）／`shiori-host32-helper`（i686 実 helper・msg loop）／`shiori-host32-testdll`（i686 fixture・固定 200/204/400 応答）。

### 2.2 既存の実シンボル（本仕様が立脚する契約）

**死活 seam（`process_host.rs`）— そのまま常設化対象**:
```
pub enum ExitKind { Clean, Abnormal(i32), Terminated }
impl ExitKind { pub fn classify(status: &ExitStatus) -> ExitKind }   // 純関数・単体テスト可
pub struct HelperHandle { .. }
impl HelperHandle { pub fn pid(&self) -> u32; pub fn terminate(&mut self) -> io::Result<()>  /* 冪等 */ }
pub fn spawn(helper_exe, load_dir, shiori_name, parent_hwnd) -> Result<HelperHandle, SpawnError>
pub fn poll_exit(handle: &mut HelperHandle) -> Option<i32>            // 非ブロッキング・try_wait ベース
pub fn poll_exit_kind(handle: &mut HelperHandle) -> Option<ExitKind>  // 非ブロッキング・分類付き
```
- `poll_exit*` は `try_wait` ベースで**呼び手スレッドを一切ブロックしない**（R1.1/1.2 の非ブロッキング要件を既に満たす seam）。
- `classify` は `Some(0)→Clean`／`Some(n)→Abnormal(n)`／`None→Terminated`（R1.3/1.4/1.5 の分類を既に実装）。
- `terminate` は `ErrorKind::InvalidInput`（終了済み kill）を `Ok` に畳む**冪等**実装（R5.2 の二重 kill 安全を既に満たす）。

**request 失敗語彙（`error.rs`／`client.rs`）— R2 で消費**:
```
pub enum RequestError {
    Handshake(HandshakeError),  // 未ハンドシェイク（構造上通常起きない）
    Timeout,                    // wire timeout（IpcError::Timeout 由来・R2.2）
    Ipc(IpcError),              // 送出失敗＝helper 応答不能の一態様（R2.1）  ※ #[from] を意図的に持たない
    Shiori(ShioriError),        // SHIORI エラー応答 400/500/ErrorLevel（R2.3）
}
```
- `map_send_error`（`client.rs`）が **`IpcError::Timeout`→`RequestError::Timeout`、その他 IpcError→`Ipc` を手動振り分け**（timeout を Ipc へ潰さない区別保持が load-bearing・既にテスト済み）。
- **これは R2.1〜R2.4 の「単一不透明失敗へ潰さない」区別保持を request 経路については既に達成している**。本仕様の増分は「この request 失敗語彙と `ExitKind`（死活）を突合し、呼び手が死活起因を明示的に読める報告面へ整理する」点にある。

### 2.3 試験パターン（既存・流用可能）

- **周期運転の原型**（`shiori_request_e2e.rs::request_e2e_get_value_and_notify_discard`）: 親窓 create → helper spawn → `pump_until_hello_or` → `send_request(Load)` ack[1] → `client.get`／`notify` → **`poll_exit_kind`→None で生存確認**。ここに GET/NOTIFY 連打ループを被せれば R3（周期運転耐性）が成立する。
- **強制 kill の原型**（`error_paths.rs::helper_abnormal_exit_is_detected_nonblocking`）: spawn → `poll_exit_kind`→None（稼働中）→ `terminate` → bounded ループで **非 Clean（Abnormal/Terminated）を非ブロッキング観測**。これは R4.1（異常検出）をほぼそのまま満たす。R4.2（kill 後の request が観測可能エラーで有限復帰）を足すには、kill 後に親窓経由で `send_request`/`client.get` を撃ち `Err`（`Ipc`/`Timeout`）が bounded に返ることを観測する結線が要る。
- **共通インフラ（そのまま流用）**: `resolve_helper_exe`／`resolve_testdll`（env override → `target/i686-pc-windows-msvc/{debug,release}` 探索 → 不在は**明示 panic**＝silent-skip 禁止・R7.4 を体現）／`HelperGuard`（Drop で冪等 terminate・panic 時もリーク防止）／**1 窓制約**（同一プロセスで message-only 窓 2 組独立生成は 2 組目失敗＝窓要る経路は単一関数へ集約、窓不要の kill 経路は別関数）／env-gate 実 pasta（`HOST32_PASTA_DLL` 設定時のみ実行・未設定は skip・設定済み不在は明示 fail＝R6 の原型が既に `request_e2e_real_pasta_optional` に存在）。

### 2.4 ログ機構の現状（ギャップ）

- steering `tech.md`: 「**構造化ログ**: `tracing` を全体規約とし、subscriber 初期化はアプリ層で行う」。
- 実態: `shiori-host32-host/Cargo.toml` は `tracing` に依存せず、`src/` 本体に `error!`／`eprintln!` は**皆無**（helper 側 main.rs のみ観測 `eprintln!`）。テストの `eprintln!` は e2e の観測出力。
- MEMORY「ログ無し失敗経路の禁止」＝ `error!`＋`Err` 戻り値・panic は致命限定＋直前ログ（2026-07-04 開発者指示・actor-foundation design が正本）。
- **帰結**: R4.4／R5.5／R7.6 が要求する `error!` を満たすには、host クレートへ `tracing` を導入するか、既存 helper 流の `eprintln!` を許容とみなすかの**方針確定が design 判断**（下記 決定項目 5）。

## 3. 要件→資産マップ（ギャップタグ: 済 / 増分 / 新規 / 制約）

| 要件 | 必要能力 | 立脚する既存資産 | ギャップ |
|---|---|---|---|
| R1.1〜1.6 死活の常設監視 | 非ブロッキング死活問い合わせ・終了種別分類・seam 上での常設化 | `poll_exit`／`poll_exit_kind`／`ExitKind::classify`／`terminate` | **増分**: seam は完成。「常設監視として運用する呼び出し規律／駆動タイミング」の器（メソッド or ヘルパ or ループ）が未在 |
| R2.1〜2.7 統一報告語彙 | 死活・timeout・SHIORI エラーを区別できる `Send` 報告データ | `RequestError`（区別保持・`map_send_error`）／`ExitKind` | **新規**: request 失敗（`RequestError`）と死活（`ExitKind`）を突合する**統一報告型**が未在。`Send` 制約の明示的担保も要確認 |
| R3.1〜3.6 周期運転耐性 | 実 i686 helper へ連打する決定的ハーネス・生存確認・leak/handle/slot 無巻き込み | `request_e2e` 骨格／testdll 固定応答／解決規約 | **増分**: 連打ループ・反復回数・決定性担保（sleep 最小化）を design が確定 |
| R4.1〜4.4 強制 kill 注入 | kill 後の異常検出＋request 観測可能エラー＋有限復帰＋silent failure 禁止 | `error_paths` の kill 検出／`terminate`／`send_request` の bounded 復帰 | **増分**: kill 後 request の `Err` 観測を単一 run に結線。R4.4 の `error!` は**新規**（ログ機構） |
| R5.1〜5.5 shutdown 全経路 | 通常終了→Clean／異常後後始末→冪等・二重 kill 安全／決定的検証／再起動なし | `terminate`（冪等）／`poll_exit_kind`／HelperGuard Drop | **増分**: 「通常 shutdown」の明示経路（helper へ終了を促す作法）と決定的検証。R5.5 の `error!` は**新規** |
| R6.1〜6.3 env-gate 実 pasta 追験 | env 設定時のみ長時間相当連打・未設定 skip・設定済み不在は明示 fail | `request_e2e_real_pasta_optional`（同型 env-gate が既存） | **増分**: 周期連打版へ拡張（既存は単発 OnBoot） |
| R7.1〜7.7 横断規律 | 凍結不改変・codec 意味論不変・PowerShell i686 test・silent-skip 禁止・sleep 最小・log-first・actor 非結線かつ `Send` | cargo 依存の凍結利用／解決 panic／HelperGuard | **制約**: R7.6（`error!`）はログ機構の新規要素に依存。他は既存規律の踏襲 |

## 4. 実装アプローチの選択肢

本仕様の設計核は **(a) 統一報告型（R2）** と **(b) 死活監視の常設化器と駆動タイミング（R1）** の 2 点。ログ規律は横断の別軸（決定項目 5）。

### 4.1 統一報告型（R2）の選択肢

#### Option A: 既存 `RequestError` を拡張（死活バリアント追加）
`RequestError` に `HelperDead(ExitKind)` 等を足し、request 経路と死活検出を同一 enum で表す。
- ✅ 呼び手（kanade）が単一型を `match` で読める・`?` 一貫。
- ✅ 新型を増やさない。
- ❌ `RequestError` は「request 出口 API の失敗」という単一責務。死活（request していなくても発生する事象）を混ぜると責務が肥大。上流 `host32-request` の凍結意味論（R7.2「`RequestError` 語彙の意味論を変更しない」）と**衝突懸念**——既存バリアント意味論は保つが型の外延を広げるのは「消費に留める」精神から逸脱し得る。design で凍結解釈の確認が要る。

#### Option B: 新規「統一報告型」で両者を包む（推奨候補・要検討）
`enum LifecycleReport { Alive, Exited(ExitKind), RequestFailed(RequestError) }` 等、死活と request 失敗を**別軸として保持**する新型を host クレートに新設。`RequestError` は不変のまま内包。
- ✅ `RequestError`（凍結・消費のみ）を一切改変しない（R7.2 遵守が明快）。
- ✅ 死活（`ExitKind`）と request 失敗（`RequestError`）の**二軸を潰さず**呼び手へ渡せる（R2.4 の区別保持に素直）。
- ✅ `ExitKind`（`Copy`）・`RequestError`（所有データ）とも `Send` 相当ゆえ新型も `Send` に切りやすい（R2.6）。
- ❌ 呼び手が 2 段（報告型→内側の RequestError）で読む場面が生じ得る。API 形状の設計が要る。

#### Option C: 突合ヘルパ関数（型を増やさず判定を提供）
新型を作らず、`fn classify_failure(req_err: &RequestError, exit: Option<ExitKind>) -> FailureKind` のような**純関数**で「helper 死亡起因か／単なる無応答か／SHIORI エラーか」を判定する語彙（小さな `FailureKind` enum）だけ提供。
- ✅ 純関数ゆえ単体テスト容易・既存型を一切触らない。
- ✅ 「request 失敗＋その時点の死活」を突合するという R2 の本質（helper 死亡起因の切り分け）に直接応える。
- ❌ 呼び手が「request 実行時に死活も採って一緒に渡す」呼び出し規律を持つ必要。器（誰が exit を採るか）は R1 側の設計と連動。

**評価**: Option B か C（またはその折衷）が R7.2 の凍結遵守に素直。A は凍結意味論との整合を design で厳密確認しない限りリスク。**報告型は本仕様が正本**（kanade は再定義しない）という brief 制約ゆえ、型/純関数の形と `Send` 担保は本仕様で確定させる。

### 4.2 死活監視の常設化器と駆動タイミング（R1）の選択肢

要件は「非ブロッキング・`Send` な報告データ」の観測可能制約のみ固定し、**専用スレッド要否は design 判断**と明言（requirements Boundary Context・brief Approach 1）。

#### Option A: request 前後 poll（親窓＝pump スレッド内・スレッド追加なし）
`client.get`／`notify` の前後で `poll_exit_kind` を呼び、request 失敗と死活を同一スレッドで突合。
- ✅ 専用スレッド不要＝actor 化の先取りをしない（brief「areka-actor 非依存・先行可」に忠実）。
- ✅ 親窓は元来 pump スレッド専有（`Shiori3Client` は `!Send`・専用スレッド駆動前提）。同一スレッドで完結。
- ❌ request していない間の死活変化は次 request まで観測されない（毎秒 pump 前提なら実害小）。

#### Option B: 周期チェック（定期 poll）を pump ループへ織り込む
`pump_until_hello_or` 系の bounded ループ意匠を流用し、定期 `poll_exit_kind` を回す。
- ✅ request と独立に死活を観測できる。
- ❌ 常駐 pump ループの器を本仕様が持つ必要。kanade の毎秒 pump との責務境界が曖昧化し得る（brief は「周期 request は耐性試験の負荷・イベント意味論なし」と切っており、常駐運行は kanade の領分）。

#### Option C: 専用監視スレッド
`HelperHandle` を監視スレッドへ渡し `poll_exit_kind` を回して報告を channel で流す。
- ✅ 呼び手を完全に非ブロッキング化。
- ❌ requirements/brief が明確に「専用監視スレッドは actor 化の先取りをしない」と牽制。`HelperHandle`（`Child` 保持）の所有権をスレッドへ渡す設計は monitoring と request の handle 共有問題を生む。**過剰設計リスク高**。

**評価**: Option A（request 前後 poll・スレッド追加なし）が brief の「actor 非依存・先行可・親窓 pump スレッド内で足るか design 判断」に最も忠実で risk 最小。B/C は kanade 領分への越境・過剰設計の懸念。design は「A で足りることの論証」または「A では取りこぼす死活事象の具体」を明示して選ぶべき。

### 4.3 試験ハーネス（R3/R4/R6）の選択肢

- **Option A（推奨）: 既存 e2e パターンの合成**。`request_e2e`（周期連打の土台）＋`error_paths` の kill 検出（R4 の土台）を、testdll 固定応答・解決 panic・HelperGuard・1 窓制約対処ごと流用。R6 は既存 `request_e2e_real_pasta_optional` の env-gate 意匠を連打版へ拡張。
- **Option B: 新規ハーネスクレート/モジュール**。連打ロジックを共通化。ここまでの試験規模では過剰（既存 tests/*.rs への追加で足りる）。

**評価**: Option A。**新規機構を生まず既存資産の合成で成立する**のが本仕様の特徴。決定性は「反復回数を定数化・実時間 sleep を bounded ループ内の最小 poll 間隔に留める」で担保（既存 `wait_kind`／bounded ループ意匠を踏襲）。

## 5. リサーチ要（design で確定）

1. **`error!` の実体**（R4.4/R5.5/R7.6 × steering `tracing` 規約）: host クレートへ `tracing` を導入するか、helper 流 `eprintln!` を許容とするか。導入するなら意図的依存追加（MEMORY「encoding_rs 承認済」同様の承認事項）。テストクレートでの subscriber 初期化要否も。→ **Research Needed（依存追加の可否）**。
2. **`ExitKind`/`RequestError` の `Send` 実測**: 両型が実際に `Send` を満たすか（`RequestError::Ipc(IpcError)` の中身・`ShioriError` の `Option<String>` 含め）を型レベルで確認し、統一報告型に `Send` 境界を明示できるか。→ 静的確認で足る（外部リサーチ不要）。
3. **周期運転の「leak/handle 枯渇/ResponseSlot 巻き込みなし」の決定的観測方法**（R3.5）: 反復往復後に何を assert すれば leak/枯渇を実証できるか（pid 生存＋全往復成功＋`poll_exit_kind`→None が最小か、追加のハンドル計数が要るか）。→ design 判断（過剰計測は避ける）。
4. **「通常 shutdown」で helper を終わらせる作法**（R5.1→Clean）: 現状 helper は UNLOAD 停止経路を持たず（main.rs「常駐 lifecycle の終了条件は下流が結線」・`MessageLoop::run` は無停止）。`terminate` は非 0/None＝Clean にならない。**Clean(0) を観測するには helper に正常終了経路が要る**——(a) helper へ終了トリガ（例: UNLOAD/専用メッセージで `msg_loop.quit()`→exit(0)）を足すか、(b) R5.1 の「Clean 観測」を testdll/stand-in の `exit(0)` 経路で満たすか。→ **design の要検討点（helper 側の増分要否）**。

## 6. 見積り（effort / risk）

| 領域 | Effort | Risk | 一言根拠 |
|---|---|---|---|
| 死活監視の常設化器（R1） | S | Low | seam 完成・request 前後 poll なら新機構ほぼ不要 |
| 統一報告型（R2） | S〜M | Low〜Med | 新型/純関数の形と `Send` 境界の設計。凍結 `RequestError` 不改変（Opt B/C）なら Low |
| 周期運転試験（R3） | M | Low | e2e 骨格の合成。決定性担保の設計が主 |
| 強制 kill 試験（R4） | S | Low | `error_paths` の kill 検出がほぼ完成。request 撃ちの結線と `error!` 追加 |
| shutdown 全経路（R5） | S〜M | Med | 通常終了→Clean 観測に helper の正常終了経路が要る場合 effort/risk 上昇（決定項目 4） |
| env-gate 実 pasta（R6） | S | Low | 既存 env-gate 意匠の連打版拡張 |
| ログ規律（R7.6） | S | Med | `tracing` 導入可否の承認次第（決定項目 1・依存追加は方針事項） |

**総括**: 全体 **M（3〜7 日規模）・risk 低〜中**。新機構をほとんど生まず既存 seam/試験骨格の合成が主。risk の芯は (i) 統一報告型の凍結遵守形、(ii) ログ実体（`tracing` 導入可否）、(iii) 通常 shutdown→Clean 観測のための helper 正常終了経路の要否——いずれも**要件確定済みゆえ design で closable**。

## 7. design への申し送り（推奨方針・決定は design/discussion）

- **統一報告型は Option B（包む型）か C（突合純関数）を軸に**、`RequestError`（凍結・消費のみ・R7.2）を改変しない形で。`Send` 境界を型で明示し「本仕様が正本・kanade は再定義しない」を担保。
- **死活監視は Option A（request 前後 poll・スレッド追加なし）を第一候補に**、A で取りこぼす死活事象があるかを論証して確定。actor 非結線・`Send` 化で将来の shiori アクター inbox からの非ブロッキング参照を阻害しない。
- **試験は既存 e2e/error_paths パターンの合成（Option A）**。testdll 固定応答・解決 panic（silent-skip 禁止）・HelperGuard・1 窓制約対処・env-gate を流用。反復回数定数化＋sleep 最小で決定性。
- **ログ実体と通常 shutdown の 2 点（決定項目 1・4）は要件確定後も未定**——`tracing` 依存追加の可否（承認事項）と、Clean(0) 観測のための helper 正常終了経路の要否を design で明示的に閉じる。

---

## 決定判断項目（requirements discussion へ供給）

1. **統一報告型の形**（R2）: A=`RequestError` 拡張／B=両者を包む新型／C=突合純関数＋小 enum。凍結 `RequestError` の意味論不変（R7.2）を最も素直に守るのは B/C。**本仕様が報告型の正本**（kanade 消費・非再定義）ゆえ形と `Send` 境界を本仕様で確定する必要。
2. **死活監視の駆動タイミング／スレッド要否**（R1・Boundary で design 送り明言）: A=request 前後 poll（スレッド追加なし・brief 忠実）／B=pump ループ内周期チェック／C=専用監視スレッド（requirements が牽制）。A で足りるかの論証が要る。
3. **`error!` の実体とログ依存追加**（R4.4/R5.5/R7.6 × steering `tracing` 規約）: host クレートは現状ログ機構ゼロ（`tracing` 依存なし・本体に `error!`/`eprintln!` 皆無）。`tracing` 導入（意図的依存追加＝承認事項）か helper 流 `eprintln!` 許容か。silent-failure 禁止（MEMORY）を満たす最小形の確定。
4. **通常 shutdown→`ExitKind::Clean`(0) の観測経路**（R5.1）: 現 helper は正常終了経路を持たず（`MessageLoop::run` 無停止・main.rs「終了条件は下流が結線」）、`terminate` は Clean にならない。Clean 観測に (a) helper へ終了トリガ増分／(b) stand-in `exit(0)` 経路で代替、のいずれを採るか。helper 側増分の要否は本仕様スコープ判断。
5. **周期運転の「leak/handle 枯渇/ResponseSlot 巻き込みなし」の決定的観測基準**（R3.5）: 最小 assert（全往復成功＋`poll_exit_kind`→None＋pid 生存）で足るか、追加のハンドル/リソース計数を課すか。過剰計測を避けつつ決定性を担保する基準の確定。
6. **周期連打の反復回数と決定性担保**（R3.5/7.5）: 「OnSecondChange 相当の頻度」を実時間依存なしにどう定数化するか（回数固定＋bounded poll・実 sleep 排除）。CI 再現性の担保方法。
