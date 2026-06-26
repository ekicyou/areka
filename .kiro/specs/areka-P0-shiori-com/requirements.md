# Requirements Document

## Project Description (Input)
脳（SHIORI）と areka の境界に、ネイティブ脳（pasta）と過去互換DLLの両方を同一視できる安定したABIが必要である。現状 areka は windows-rs 経由で COM を多用（DComp/D2D/DWrite）しているが、SHIORI 抽象は未定義であり、呼び出し側に「ネイティブ／過去互換」の分岐が露出してしまう懸念がある。

本仕様は、areka の**内部唯一の SHIORI ABI = `IShiori`（COM, 文字列は HSTRING/UTF-16）**を定義し、ネイティブ脳が in-proc COM で直結できるようにする。あわせて push 用の `IShioriHost`（sink）を定義し、ネイティブ脳の能動的 wakeup を可能にする。上位設計の正本は `doc/COMPAT_ARCHITECTURE.md` §5。

## Introduction
本仕様は、areka 本体と「脳（SHIORI）」の境界となる**内部唯一の抽象境界 `IShiori`（COM インターフェイス）**と、その能動通知経路 `IShioriHost`（sink インターフェイス）を定義する。狙いは、ネイティブ脳と過去互換 DLL という性質の異なる実装を、呼び出し側から見て**完全に同一視**できるようにし、areka 本体のコードに「ネイティブ／過去互換」の分岐を一切露出させないことである。

本仕様のスコープは **ABI（インターフェイス契約）の定義と、ネイティブ脳の in-proc アクティベーション・ライフサイクル・リクエスト・push 経路**に限定する。過去互換 DLL のホスティング、さくらスクリプト解釈、毎秒ポーリング駆動の上位ロジックは、隣接する別仕様の責務であり本仕様には含めない。

## Boundary Context
- **In scope（本仕様が責務を持つ範囲）**:
  - `IShiori` インターフェイスの面（ライフサイクル load/unload、リクエスト hrequest（同期呼び出し＋遅延応答・相関トークン）、文字列引数・戻り値の取り回し、エラー報告）の定義
  - `IShioriHost`（sink）インターフェイスの面（脳からの能動通知 Raise、および遅延リクエストの完了応答）の定義
  - ネイティブ脳の **in-proc アクティベーション**（追加 IPC を介さずに同一プロセス内で `IShiori` 実装へ到達する経路）
  - 文字列を **HSTRING/UTF-16** で取り回す契約と、その WinRT ランタイム非依存という制約
  - 呼び出し側に native／過去互換の区別を露出させないという**呼び出し面の不変条件**
- **Out of scope（本仕様が責務を持たない範囲）**:
  - 過去互換 DLL（32bit shiori.dll 等）のホスティング → `areka-P0-shiori-host-32`
  - さくらスクリプトの解釈・実行 → 別仕様（sakura-script）
  - 毎秒ポーリング（OnSecondChange）等、SHIORI を駆動する上位タイミングロジック
  - SAORI、過去互換のための独自 IPC、OOP 自動マーシャリング
  - x86（32bit）でのネイティブ脳直結（本仕様は x64／CPU ネイティブ前提）
- **Adjacent expectations（隣接仕様・既存資産への期待）**:
  - `areka-P0-shiori-host-32` は、過去互換 DLL を本仕様で定義する**同じ `IShiori` を実装する一実装**として areka 本体へ提供されること（呼び出し側は区別しない）
  - ネイティブ旗艦脳「ぱすたさん（pasta）」は、本仕様の `IShiori` を実装する受け皿として in-proc 経路で接続されること
  - COM 基盤（windows-rs）および COM 命名規約（完了済み `com-resource-naming-unification`）に整合すること

## Requirements

### Requirement 1: 内部唯一の SHIORI ABI（呼び出し面の同一視）
**Objective:** areka 本体の開発者として、ネイティブ脳でも過去互換 DLL でも同一の抽象境界を通じて脳を扱いたい。これにより、呼び出し側コードに実装種別の分岐を持ち込まずに済む。

#### Acceptance Criteria
1. The IShiori ABI shall be areka 本体が脳とやり取りするための唯一の内部インターフェイス契約として定義される。
2. When areka 本体が脳に対して操作（ロード・アンロード・リクエスト）を行うとき, the IShiori ABI shall すべての操作を `IShiori` のメソッド呼び出しとして表現する。
3. While 脳がネイティブ実装であるか過去互換 DLL 実装であるかにかかわらず, the IShiori ABI shall 呼び出し側へ同一のメソッド面と呼び出し規約を提供する。
4. The IShiori ABI shall 実装種別（ネイティブ／過去互換）を呼び出し側に区別させる分岐をインターフェイス面に持たない。
5. Where 実装種別による差異が存在する場合, the IShiori ABI shall その差異を生成（アクティベーション）経路にのみ局所化し、確立済みの `IShiori` 利用面には波及させない。

### Requirement 2: ライフサイクル（ロード／アンロード）
**Objective:** areka 本体の開発者として、脳の初期化と終了を明示的に制御したい。これにより、脳のリソースを確実に確保・解放できる。

#### Acceptance Criteria
1. When areka 本体が脳を利用開始するとき, the IShiori ABI shall ロード操作を提供し、脳に初期化の機会を与える。
2. When areka 本体が脳の利用を終了するとき, the IShiori ABI shall アンロード操作を提供し、脳に終了処理の機会を与える。
3. If ロード操作が失敗したとき, then the IShiori ABI shall 失敗を呼び出し側へ判別可能な形で報告する。
4. While 脳がロードされていない状態のとき, the IShiori ABI shall リクエスト操作を有効な処理として受理しない。

### Requirement 3: リクエスト処理（hrequest 相当・同期呼び出し＋遅延応答）
**Objective:** areka 本体の開発者として、脳へリクエストを送り応答を受け取りたい。即時応答できない問い合わせも、呼び出しをブロックせずに後から応答を受け取りたい。

#### Acceptance Criteria
1. When areka 本体が脳へリクエストを送るとき, the IShiori ABI shall リクエストを同期的なメソッド呼び出しとして受け取り、結果を呼び出しの戻りとして返す。
2. When 脳がリクエストに即時応答するとき, the IShiori ABI shall 応答文字列を伴う即時応答として結果を返す。
3. When 脳がリクエストに即時応答せず後で応答するとき, the IShiori ABI shall 即時の応答文字列を伴わない遅延結果を返し、後続の応答と突き合わせるための相関トークンを発行する。
4. The IShiori ABI shall リクエスト引数および即時応答文字列を HSTRING（UTF-16）として取り回す。
5. The IShiori ABI shall 即時応答・遅延・失敗の各結果を呼び出し側が判別可能な形で返す。
6. If リクエスト処理が失敗したとき, then the IShiori ABI shall 失敗を呼び出し側へ判別可能な形で報告する。

### Requirement 4: 文字列の取り回し（HSTRING/UTF-16, WinRT 非依存）
**Objective:** 統合者として、脳とのすべての文字列受け渡しを単一の文字列表現で扱い、かつ WinRT ランタイムへの依存を持ち込みたくない。これにより、ネイティブ・過去互換の双方で同一の文字列契約を成立させられる。

#### Acceptance Criteria
1. The IShiori ABI shall 脳との間で受け渡すすべての文字列引数・戻り値を HSTRING（UTF-16）として定義する。
2. While 文字列のプロセス内取り回し（生成・読み取り・解放）を行うとき, the IShiori ABI shall WinRT ランタイムの初期化を前提としない。
3. The IShiori ABI shall HSTRING 型引数の OOP 自動マーシャリングを要求しない設計上の不変条件を満たす。

### Requirement 5: ネイティブ脳の in-proc アクティベーション
**Objective:** ネイティブ脳の実装者として、自分の脳を areka と同一プロセス内で `IShiori` として直結させたい。これにより、追加のプロセス間通信やマーシャリングを介さずに最短経路で連携できる。

#### Acceptance Criteria
1. Where 脳がネイティブ実装である場合, the IShiori ABI shall その脳実装へ in-proc（同一プロセス内）で到達するアクティベーション経路を提供する。
2. While ネイティブ経路で脳とやり取りするとき, the IShiori ABI shall 文字列・呼び出しに OOP マーシャリングを介在させない。
3. The IShiori ABI shall x64／CPU ネイティブを前提とし、x86（32bit）でのネイティブ直結を本仕様の対象としない。
4. Where 脳がネイティブ実装である場合, the IShiori ABI shall その脳が `IShiori` を直接実装することで areka 本体へ接続できるようにする。

### Requirement 6: push 経路・遅延応答（IShioriHost sink による能動 wakeup と遅延リクエスト完了）
**Objective:** ネイティブ脳の実装者として、areka 本体からの問い合わせを待たずに能動的に areka へ通知（wakeup）したい。あわせて、遅延扱いとしたリクエストの応答を後から areka へ届けたい。

#### Acceptance Criteria
1. The IShioriHost ABI shall areka 本体側が実装し脳へ渡す単一の sink インターフェイスとして定義され、能動通知と遅延リクエスト応答の双方を受け取る。
2. When 脳がロードされるとき, the IShiori ABI shall 脳が能動通知および遅延応答に使用できる `IShioriHost`（sink）を脳へ受け渡す機会を提供する。
3. When 脳が能動的に areka へ通知するとき, the IShioriHost ABI shall 通知内容（スクリプト相当の文字列）を受け取る Raise 操作を提供する。
4. When 脳が遅延扱いとしたリクエストの応答を届けるとき, the IShioriHost ABI shall Raise とは別の完了操作を提供し、対応する相関トークンと応答文字列を受け取る。
5. The IShioriHost ABI shall Raise の通知内容および遅延応答の完了内容を HSTRING（UTF-16）として取り回す。

### Requirement 7: エラー報告規約
**Objective:** areka 本体の開発者として、脳操作の成否を一貫した方法で判別したい。これにより、失敗時に呼び出し側で適切に分岐・回復できる。

#### Acceptance Criteria
1. The IShiori ABI shall 各操作の成否を COM 呼び出し規約に沿った形で呼び出し側へ報告する。
2. If 脳の操作が失敗したとき, then the IShiori ABI shall 成功時と区別可能な失敗結果を返す。
3. The IShiori ABI shall 既存の areka COM 命名規約（`com-resource-naming-unification`）と整合した命名・規約を用いる。
