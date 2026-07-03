# Requirements Document

## Introduction

areka の M1 並行モデルは「各エンジン（kanade/sakura/seriko/emo など）＝独立スレッドのアクター・相互通信はチャンネル・I/O 契約＝チャンネルのメッセージ型」と確定している。しかし全エンジンが共有する通信の**原語（プリミティブ）が存在しない**ため、放置すると各エンジンが独自の channel 流儀（メッセージ型・生存期間・停止手順・エラー伝搬）を発明し、M-boot 統合で噛み合わなくなる。特に UI スレッド（emo/render・窓）は message pump 中で `recv()` ブロックできず、queue＋wakeup による**配送ブリッジ**という実装物を要する。

本ユニット（⓪ ghost 帰属の横断基盤・`areka-P0-parser-foundation` の並行モデル版）は、責務三分のうち「**機構**」——アクター原語（envelope 規約・spawn/join・停止手順・UI 配送ブリッジ）を提供する。特定エンジンの知識を持たず、経路（kanade＝最大の消費者）と結線（ghost / ghost-setup）は下流ユニットの領分である。

成果物は「全エンジンが同じ原語で会話できる**最小のアクター基盤**」であり、フレームワーク化（トレイトだらけの actor framework）はしない。抽象は 2 例目の実物が要求するまで作らない。単一の pass/fail 観測は toy アクター試験——(a) worker⇄worker の request/reply と Close→join の決定的完走、(b) worker→UI スレッド（message pump 実走）への配送ブリッジが echo を返す——で満たす。既存エンジンの改修は本ユニットでは行わない。

## Boundary Context

- **In scope**:
  - アクター spawn/join の原語（名前付きスレッド・JoinHandle 引き渡し・panic 検出）
  - inbox 規約（アクターごと単一 Receiver・メッセージ＝enum）
  - request/reply 規約（返信 Sender をメッセージに同梱＝oneshot 相当）
  - 停止手順（Close メッセージ→即時停止（積み残し破棄）→join の全経路。積み残しの reply Sender は drop され要求側は切断を観測）
  - UI スレッド配送ブリッジ（queue＋wakeup で message pump スレッドへ届ける・pump 内 drain）
  - backpressure 方針と大型データ手渡し規約の明文化
  - toy アクター試験（worker⇄worker・worker→UI pump 実走）
  - tracing 統合（スレッド名・アクター名を span に載せる）
- **Out of scope**:
  - 各エンジンの実アクター化（kanade/sakura/seriko/emo 各ユニットの領分）
  - I/O 契約 4 クラスタ（撫で／選択肢／二人立ち／移動）のメッセージ型の中身定義（本基盤は器のみ）
  - crossbeam-channel 導入（select/MPMC が実需になるまで凍結・新規依存＝要承認）
  - 監督ツリー・再起動戦略（M2 以降・実需駆動）
  - async runtime（tokio 禁止）
  - tracing Subscriber の初期化（アプリ層の責務）
- **Adjacent expectations**:
  - 消費者（kanade／sakura／seriko／emo-present／ghost-setup／shiori）は本基盤の envelope 規約の上に各自のメッセージ型を定義する。
  - emo-present の指令口（`show_surface` 級）は本基盤の envelope 規約に載る前提（M-boot は直接呼出で開始し、結線時に channel 化）。
  - UI スレッドは MTA（`WinApp::new`＝COINIT_MULTITHREADED）かつ render/window 固定・D2D 単一スレッド前提を破らない。wintf 既存の pump／tick 資産（`VsyncEventBridge`・`event_listener`・`wintf-winmsg-executor`）と整合する起床方式を用いる。
  - host-32（別プロセス＝天然のアクター境界・WM_COPYDATA が channel）は本基盤の直接対象外だが、親窓 pump スレッド統合の同型課題を既に解いた参照実装として矛盾しない。

## Requirements

### Requirement 1: アクター spawn / join の原語

**Objective:** As an areka エンジン結線層（ghost / ghost-setup）, I want 名前付きスレッドとしてアクターを起動し JoinHandle を受け取れること, so that 各エンジンを独立スレッドのアクターとして起こし、終了時に確実に回収できる

#### Acceptance Criteria

1. When 結線層がアクターを spawn したとき, the Actor Foundation shall そのアクターへメッセージを送るための送信端（Sender）と、当該アクターの JoinHandle を呼び出し側へ返す。
2. When アクターを spawn したとき, the Actor Foundation shall そのアクターの実行スレッドへ呼び出し側指定のアクター名を付与する。
3. If アクターの本体処理がパニックしたとき, then the Actor Foundation shall そのパニックを join 時に観測可能な失敗として呼び出し側へ伝搬する（パニックを黙って握り潰さない）。
4. The Actor Foundation shall アクター 1 個の spawn・停止・join を短時間かつ低コストで行える手段を提供する（sakura の per-talk transient 生成・破棄に耐える軽量性）。
5. The Actor Foundation shall アクターごとに単一の受信端（inbox）を持つ構造を規約として定める。

### Requirement 2: inbox 規約と request/reply

**Objective:** As an areka エンジン実装者（consumer）, I want アクターごとの単一 inbox と、返信端を同梱した request/reply の規約, so that 各エンジンが同じ流儀で相互通信でき、M-boot 統合で噛み合う

#### Acceptance Criteria

1. The Actor Foundation shall 各アクターの inbox を単一の受信端とし、そのアクター宛メッセージをアクターごとの enum 型（命名 `XxxMsg`）で表す規約を定める。
2. Where request/reply が必要なメッセージであるとき, the Actor Foundation shall そのメッセージに返信用の送信端（reply Sender）を同梱する規約を提供する（oneshot 相当・応答を 1 回返す）。
3. When 送信側が reply 付きメッセージを送り、受信側が応答を返したとき, the Actor Foundation shall 送信側が対応する応答を受け取れることを保証する。
4. The Actor Foundation shall アクター間を渡るメッセージを Send な所有データとし、借用（参照）を跨がせない規約を定める。
5. Where メッセージが大型データ（画素バッファ等）を含むとき, the Actor Foundation shall コピーを避けた手渡し規約（`Arc` もしくは共有バッファの受け渡し）を明文化する。

### Requirement 3: 停止手順（Close → 破棄 → join）

**Objective:** As an areka エンジン結線層, I want 各アクターに共通する明確な停止手順, so that 終了時にアクターを確実に落とせ、停止時の曖昧さに起因する統合バグを排除できる

#### Acceptance Criteria

1. The Actor Foundation shall 各アクターの inbox enum に横断制御としての停止メッセージ（Close 相当）を含める規約を定める。
2. When アクターが Close 相当のメッセージを受け取ったとき, the Actor Foundation shall そのアクターの受信ループを終了へ導く。
3. The Actor Foundation shall Close 到達時点で inbox に残る未処理メッセージを**破棄する**（drain 処理しない）方針に固定し、Close は「即時停止」（受信ループを直ちに抜ける）を意味する規約として明文化する。積み残しを処理し切ってから止めたい送信側は、後続メッセージが無いことを確認したうえで Close を送る運用で対応する（graceful 停止は本原語の上に送信側が構築する）。
4. When アクターが停止し join されたとき, the Actor Foundation shall その停止・回収が決定的に完了することを保証する（停止が永久にハングしない）。
5. If アクターの送信端がすべて破棄され inbox が閉じたとき, then the Actor Foundation shall アクターの受信ループを正常終了させる（明示 Close と同様に受信ループを抜ける）。
6. When アクターが停止する（Close 受信・全 Sender drop・panic のいずれか）時点で inbox に request/reply メッセージが未処理で残っていたとき, the Actor Foundation shall それらのメッセージを drop することで同梱された返信用送信端（reply Sender）も drop され、対応する要求側の応答受信が**切断（`Err`）として観測され永久ブロックしないこと**を保証する（＝要求のキャンセル／アクター終了は reply 側の切断で伝わる規約とし、要求側は応答受信が `Err` を返し得ることを許容する）。

### Requirement 4: UI スレッド配送ブリッジ

**Objective:** As an areka の UI スレッド上のアクター（emo/render・窓）, I want message pump を止めずに他スレッドからのメッセージを受け取る配送ブリッジ, so that pump 中でブロックできない UI スレッドへも同じ原語で指令を届けられる

#### Acceptance Criteria

1. The Actor Foundation shall message pump 上で動作するアクターへ、他スレッドからメッセージを届ける配送ブリッジ（queue＋wakeup）を提供する。
2. When worker スレッドが UI スレッド宛にメッセージを送ったとき, the Actor Foundation shall UI スレッドの pump をブロックさせずに、そのメッセージを queue へ積み UI スレッドを起床させる。
3. While UI スレッドが message pump を実行しているとき, the Actor Foundation shall 起床契機ごとに queue に積まれたメッセージを UI スレッド側で drain して処理させる。
4. The Actor Foundation shall UI スレッドが MTA・render/window 固定・D2D 単一スレッド前提を保ったまま動作するブリッジとし、wintf 既存の pump／tick 起床資産（`VsyncEventBridge` / `event_listener` / `wintf-winmsg-executor`）と整合する起床方式を用いる。
5. The Actor Foundation shall この配送ブリッジを、emo-present の指令 API および窓移動指令の将来の搬送路として利用可能な形で提供する（本ユニットは搬送路の器を提供し、各指令メッセージの型定義は下流ユニットが行う）。

### Requirement 5: backpressure と流量方針

**Objective:** As an areka エンジン実装者, I want 制御メッセージと大量データを取り違えない流量規約, so that channel を詰まらせず、毎フレームのデータ洪水を避けられる

#### Acceptance Criteria

1. The Actor Foundation shall 制御メッセージ経路を unbounded（低レート前提）とする方針を明文化する。
2. The Actor Foundation shall 毎フレーム大量に発生するデータ（画素バッファ等）を channel に直接流さず、共有バッファ／`Arc` 手渡し等で受け渡す規約を明文化する。
3. Where 将来 select／MPMC／有界キューが実需となったとき, the Actor Foundation shall その導入を crossbeam-channel 等の新規依存追加（開発者承認要）へ委ねる拡張シームとして残す（本ユニットでは導入しない）。

### Requirement 6: tracing 統合

**Objective:** As an areka 開発者, I want アクター単位で追跡可能な構造化ログ, so that どのアクター・スレッドが何を処理したかを steering の logging 規約に沿って観測できる

#### Acceptance Criteria

1. The Actor Foundation shall アクターの処理を tracing の span に載せ、スレッド名およびアクター名を span に含める。
2. The Actor Foundation shall tracing Subscriber の初期化を行わない（初期化はアプリ層の責務とする）。

### Requirement 7: 最小性の維持（フレームワーク化の禁止）

**Objective:** As an areka プロジェクト（spec 工場・過剰抽象の禁止）, I want 基盤を「規約＋薄いヘルパ＋ブリッジ」までに留めること, so that 未検証の抽象が下流ユニットを縛らない

#### Acceptance Criteria

1. The Actor Foundation shall 提供物を「規約＋薄いヘルパ＋UI 配送ブリッジ」までに限定し、共通トレイトによる過剰抽象（actor framework 化）を行わない。
2. Where ある抽象（監督ツリー・再起動戦略・select・MPMC 等）が 1 例のみを根拠に導入されそうなとき, the Actor Foundation shall その抽象の導入を、2 例目の実物が要求するまで見送る。
3. The Actor Foundation shall 既存エンジン（kanade/sakura/seriko/emo）の実アクター化を本ユニットの成果物に含めない。

### Requirement 8: 観測（受け入れ試験）

**Objective:** As an areka 開発者, I want 基盤の正しさを示す単一 pass/fail の toy アクター試験, so that 下流の消費開始前に原語の健全性を機械的に確認できる

#### Acceptance Criteria

1. When toy アクター試験を実行したとき, the Actor Foundation shall worker⇄worker 間の request/reply が正しい応答を返し、Close→join が決定的に完走することを検証する。
2. When toy アクター試験を実行したとき, the Actor Foundation shall worker から UI スレッド（実走する message pump 上）への配送ブリッジが echo を返すことを、wintf の pump 上で検証する。
3. If いずれかの toy アクター試験ケースが停止・応答・echo のいずれかで期待結果を満たさないとき, then the Actor Foundation shall その試験を失敗（fail）として観測させる。
