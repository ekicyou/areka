# Requirements Document

## Introduction

areka の M1 は「アプリ組み上げ三段」の第二段にあたる。第一段（app-shell＝骨格）で `open_startup_window` シームと env ゲート smoke が用意され、上流エンジン（①shiori 通信層・③kanade 運行・④sakura 再生・②package-mount）は個別に完成した。しかし **これらを descript.txt 起点で実際に繋いで起動・終了する結線層（⓪ghost）が存在しない**。

本仕様（areka-P0-ghost-setup）は、その結線層を所有する。結線に着手した時点で最初に踏む地雷が、並走実装の帰結として **talk 契約が kanade と sakura で二重定義（フォーク）** している事実である。したがって本仕様は次の二つのワークストリームを持つ:

- **WS-A（契約統一・先行）**: kanade と sakura に分裂した talk 授受契約（`StartTalk`／`TalkDone`／中断理由）を kanade 正本へ一本化する。
- **WS-B（結線の背骨）**: descript.txt 起点で全エンジンを起動〜終了統括する結線層（mount → shiori actor → kanade → sakura dispatcher → ticker → 終了統括）を構築する。

観測は単一の pass/fail に集約する: (a) 決定論 spine e2e（記録 sink＋注入 Tick・sleep 不使用）、(b) env ゲート実 pasta 追験（実 emo2 OnBoot 一周）、(c) `open_startup_window` シーム経由の app smoke 維持。

## Boundary Context

- **In scope**:
  - WS-A talk 契約統一（kanade 正本化・中断理由の値域統一・sakura の暫定所有型を再エクスポートへ移譲・両クレートのテスト追随。下流 import パス `areka_sakura::contract::*` は不変）。
  - sakura dispatcher（kanade の永続 talk 送出口を受ける常駐アクター・同時 talk 1 の単一 slot・talk 完了を kanade へ転送・停止時は稼働中 talk を終了させて join）。
  - shiori actor 結線（接続をアクタースレッド上で一度だけ確立・helper の死活監視）。
  - ticker（kanade の毎秒 Tick と稼働中 talk への経過秒 Tick を養う・決定論テストで差し替え可能）。
  - KanadeConfig の値源解決（shell 名は shell descript 由来・baseware 情報は areka 定数）。
  - 終了統括（起動順・終了順の統括・正規の clean shutdown 一本・全スレッド join・プロセス終了コード 0）。
  - 決定論 spine e2e＋env ゲート実 pasta 追験＋app smoke 維持。
- **Out of scope**:
  - 表示結線（サーフェス合成 seriko／emo-present／バルーンテキスト emo-text-layer）。sink は録音実装を挿す（実 sink は後続 M-boot 統合が同じ差し込み口へ挿す）。
  - 本物ゴースト窓の生成（window-placement の領分。本仕様はダミー窓を維持）。
  - 窓位置の永続化（position-persist）。
  - OnSecondChange を起点とする自発会話の生成（idle-talk。ticker は Tick を送るのみ・トーク再生不能時の扱いは kanade が既に処理）。
  - SSTP／FMO 等の外部連携（M2）。
- **Adjacent expectations**:
  - kanade（③）・sakura（④）は完成済みであり、本仕様はその授受面の隣接増分としてのみ両クレートに手を入れる。運行表そのものや再生意味論は既存仕様の領分で不改変。
  - shiori 通信層（①）は `Shiori3Client`／`HelperLifecycle`／`request_clean_shutdown` を提供済みであり、本仕様はこれらを消費するのみ（プロトコル・IPC・語彙は不改変）。
  - boot／close の SHIORI イベント発火順序は kanade が正本として実装済みであり、本仕様は再定義しない。
  - OnSecondChange は 1 秒周期（ukadoc）。ticker は 1 秒周期の Tick を kanade へ供給するのみで、周期の意味論は kanade が所有する。
  - 並走する seriko／emo-present とは非衝突。特に sakura の出力契約（`TalkCue`／`SurfaceSink`／`TextSink`／`cue_target_of`）は seriko が消費中のため凍結面として保護する。

## Requirements

### Requirement 1: talk 契約の kanade 正本化（WS-A）

**Objective:** As a 結線層の実装者, I want talk 授受契約が単一の正本に一本化されていること, so that kanade と sakura を型の齟齬なく結線でき、二重定義に起因する不整合を排除できる

#### Acceptance Criteria

1. The ghost-setup 仕様 shall talk 授受契約（`StartTalk`／`TalkDone`／中断理由）を kanade 正本の単一定義へ一本化する。
2. Where 中断理由が talk 完了に含まれる場合, the talk 契約 shall 通常終了・quit・中断を区別できる値域を単一の正本として提供する。
3. The talk 契約 shall sakura が暫定所有していた授受型を正本の再エクスポートへ差し替える。
4. When 下流が `areka_sakura::contract::*` から talk 授受型を参照するとき, the talk 契約 shall 従来と同一の import パスで参照可能であり続ける。
5. When 契約を一本化するとき, the ghost-setup 仕様 shall `TalkCue`／`SurfaceSink`／`TextSink`／`cue_target_of` および dola cue 型を変更しない。
6. When 契約統一を反映するとき, the ghost-setup 仕様 shall kanade・sakura 両クレートの既存テストを新契約に追随させ、緑を維持する。

### Requirement 2: descript.txt 起点の起動統括（WS-B）

**Objective:** As a ゴースト利用者, I want descript.txt を起点に全エンジンが正しい順序で起動されること, so that ゴーストが起動して脳（SHIORI）との会話ループに入れる

#### Acceptance Criteria

1. When ゴースト起動が要求されたとき, the ghost 結線層 shall descript.txt 起点のマウント解決（SHIORI ディレクトリ／ファイル・shell ディレクトリ）を入力として全エンジンを起動する。
2. When 起動を統括するとき, the ghost 結線層 shall マウント解決 → shiori アクター → kanade → sakura dispatcher → ticker の順で各エンジンを結線する。
3. The ghost 結線層 shall kanade の起動に必要な設定値のうち shell 名を shell の記述子から、baseware 情報を areka 定数から解決して供給する。
4. When 起動が完了したとき, the ghost 結線層 shall kanade を起点とする boot 手順（OnBoot 起点のトーク受領〜再生）が動作する状態にする。
5. If マウント解決が起点不在・読取不能・shell 不在で失敗したとき, then the ghost 結線層 shall 失敗をログに記録し、エラーとして呼び出し側へ返す。

### Requirement 3: SHIORI アクターの結線と死活監視

**Objective:** As a ゴースト利用者, I want 脳（SHIORI）が結線され、その死活が監視されること, so that ゴーストが会話でき、脳の停止が観測されて安全に扱われる

#### Acceptance Criteria

1. When shiori アクターを起動するとき, the ghost 結線層 shall SHIORI 接続をアクタースレッド上で一度だけ確立する接続手続きを供給する。
2. While ゴーストが稼働しているとき, the ghost 結線層 shall 32bit SHIORI helper の死活を監視する。
3. If SHIORI 接続の確立に失敗したとき, then the ghost 結線層 shall 失敗を死活報告として kanade へ通知する。
4. If SHIORI helper が異常終了したことを検出したとき, then the ghost 結線層 shall その死活を kanade へ通知する。
5. The ghost 結線層 shall SHIORI 通信層のプロトコル・IPC・語彙（`Shiori3Client`／`HelperLifecycle`／`request_clean_shutdown` 等）を変更せず消費のみ行う。

### Requirement 4: sakura dispatcher（永続—transient 非対称の吸収）

**Objective:** As a 運行系（kanade）, I want トーク再生要求を常駐の窓口が受けて transient な再生アクターへ橋渡しすること, so that 「永続 channel へ送る」kanade 設計と「per-talk transient」sakura 設計の非対称が吸収され、トークが再生される

#### Acceptance Criteria

1. When kanade がトーク開始を送出したとき, the sakura dispatcher shall そのトークの再生アクターを起動して再生させる。
2. While あるトークが再生中のとき, the sakura dispatcher shall 同時に 1 本のみを再生する単一 slot を維持する。
3. When トーク再生が完了したとき, the sakura dispatcher shall 完了通知を kanade のトーク完了受領口（`KanadeMsg::TalkDone`）へ転送する。
4. If 現在の slot と一致しない古いトークの完了通知を受け取ったとき, then the sakura dispatcher shall その通知を talk 識別子に基づいて棄却する。
5. When 停止が要求されたとき, the sakura dispatcher shall 稼働中のトークへ終了（Close）を送り、その再生スレッドの join を待ってから停止する。
6. The sakura dispatcher shall 出力 sink（サーフェス／テキスト）を注入可能な差し込み口として公開し、後続の実 sink 差し替えを可能にする。

### Requirement 5: ticker（差し替え可能な時刻供給）

**Objective:** As a 決定論テストと本番実行の双方, I want 時刻の刻みを供給する仕組みが差し替え可能であること, so that 本番は実クロックで駆動でき、テストは Tick を注入して sleep 無しで全経路を検証できる

#### Acceptance Criteria

1. While ゴーストが稼働しているとき, the ticker shall kanade へ 1 秒周期の Tick を供給する。
2. While トークが再生中のとき, the ticker shall そのトークへ再生起点からの経過秒を Tick として供給する。
3. The ghost 結線層 shall ticker を差し替え可能にし、決定論テストが Tick を外部注入できるようにする。
4. When 決定論テストが Tick を注入するとき, the ghost 結線層 shall 実時間の経過（sleep）に依存せずに時刻駆動の経路を進行させる。

### Requirement 6: 終了統括（正規 clean shutdown・全 join）

**Objective:** As a ゴースト利用者, I want 終了時に全エンジンが正しい順序で確実に停止すること, so that 脳・helper・全スレッドが取り残されずプロセスが正常終了する

#### Acceptance Criteria

1. When 終了が観測されたとき, the ghost 結線層 shall 各エンジンを 終了（Close）→ 残処理の排出（drain）→ スレッド join の順で停止する。
2. When kanade の停止を観測したとき, the ghost 結線層 shall SHIORI へ Unload を送り、helper の正規の clean shutdown 経路を実行する。
3. The ghost 結線層 shall 終了時に stand-in の最小 hack ではなく正規の clean shutdown 経路のみを用いる。
4. When 全エンジンの停止が完了したとき, the ghost 結線層 shall 全スレッドを join し、プロセス終了コード 0 で終了する。
5. If 終了経路の途中で失敗が起きたとき, then the ghost 結線層 shall 失敗をログに記録し、エラーとして扱う（silent failure を許さない）。

### Requirement 7: 決定論 spine e2e 観測

**Objective:** As a 品質保証者, I want boot から close までの結線の背骨が決定論的に検証されること, so that 全経路が実行テストで回帰檻に入り、sleep 依存の非決定性なしに合否が確定する

#### Acceptance Criteria

1. The ghost 結線層 shall testdll fixture と記録 sink（サーフェス／テキスト sink の録音実装）を用いて boot から close までを検証する e2e を提供する。
2. When boot を検証するとき, the spine e2e shall OnBoot 起点のトーク受領 → sakura 再生 → sink への発火列を観測する。
3. When close を検証するとき, the spine e2e shall 正規の clean shutdown が `ExitKind::Clean` 相当で成立し、全スレッドが join されることを観測する。
4. When spine e2e を実行するとき, the spine e2e shall Tick を注入し、sleep を用いずに合否を確定する。
5. The spine e2e shall boot 成功・SHIORI 死活・close 握手・close deadline・全断線を含む主要経路を実行テストで網羅する。

### Requirement 8: env ゲート実 pasta 追験と app smoke 維持

**Objective:** As a 統合検証者, I want 実 pasta による起動と app 骨格の smoke が維持されること, so that 決定論テストに加えて実ブレインでの一周が確認でき、既存の骨格起動が壊れていないことが保証される

#### Acceptance Criteria

1. Where 環境変数によるゲートが有効なとき, the ghost 結線層 shall 実 pasta（実 emo2）の OnBoot 一周を追験として実行する。
2. When 起動形が `open_startup_window` シームを経由するとき, the ghost 結線層 shall app smoke（`AREKA_APP_SMOKE_EXIT_MS` ゲート）が緑のまま維持されるようにする。
3. While app smoke を維持するとき, the ghost 結線層 shall 本物ゴースト窓を生成せずダミー窓を維持する（本物窓生成は window-placement の領分）。
