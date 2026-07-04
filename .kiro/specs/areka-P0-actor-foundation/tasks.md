# Implementation Plan

- [ ] 1. Foundation: `areka-actor` クレート基盤とconventions正本
- [x] 1.1 クレート雛形とモジュール骨格の作成
  - crates/areka-actor を新設し、Cargo.toml に依存（tracing/thiserror/async-channel/wintf-winmsg-executor、dev-dependencies に windows）を宣言する
  - ルート Cargo.toml の workspace.dependencies に async-channel を追記する
  - src/spawn.rs・src/reply.rs・src/ui.rs・tests/ を空実装（コンパイル可能な骨格）で用意する
  - 観測可能な完了条件: `cargo check -p areka-actor` が新設クレート単体で成功する
  - _Requirements: 6.2, 7.1, 7.3_
  - _Boundary: クレート基盤_

- [ ] 1.2 lib.rs への規約正本（envelope/停止/流量/拡張シーム）の明文化
  - inbox規約（単一Receiver・XxxMsg命名）・envelope規約（reply Sender同梱・Send所有データ・Arc大型手渡し）・停止規約（Close=即時停止・積み残し破棄・受信ループはErrで終了しない）・流量規約（unbounded制御・大型データ非流通）・拡張シーム（crossbeam等は承認まで凍結）をcrate rustdocへ規範文で記述する
  - 1.1で用意したモジュールの公開シンボルをlib.rsからre-exportする（規約が定める公開面のみに限定する）
  - 観測可能な完了条件: `cargo doc -p areka-actor` が上記5つの規約セクションを含むrustdocを生成する
  - _Requirements: 1.5, 2.1, 2.4, 2.5, 3.1, 3.3, 3.7, 5.1, 5.2, 5.3, 7.1_
  - _Boundary: conventions（lib.rs）_

- [ ] 2. Core: 純粋層の実装とUIブリッジ実現性の検証
- [ ] 2.1 (P) 名前付きアクターspawnと受信ループヘルパの実装
  - 名前付きスレッドとしてアクターを起動しinbox送信端とjoinハンドルを返す機能を実装する
  - 受信ループヘルパを実装する: 通常メッセージはhandlerへ渡す・handlerがErrを返した場合はエラーをtracingへ記録したうえで受信を継続する・Close相当（Break）受信で即時終了する・全送信端drop（切断）で正常終了する
  - panicをjoin時に観測可能な失敗として伝搬させる
  - 観測可能な完了条件: 単体テストで「スレッド名=アクター名」「panic時のjoinがErr」「Break即時終了」「切断で正常終了」「handlerのErr後も受信継続」の5ケースがgreenになる
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 3.2, 3.4, 3.5, 3.7, 6.1_
  - _Boundary: spawn_

- [ ] 2.2 (P) request/reply（oneshot相当）の実装
  - リクエストごとに一度だけ応答可能な返信チャンネル対を生成する機能を実装する
  - 応答未送信のまま返信端がdropされた場合に要求側が切断として観測できるようにする
  - 上限時間付き待機（timeoutとdroppedを区別）を提供する
  - 観測可能な完了条件: 単体テストで「送受信往復」「送信端drop→Dropped」「timeout→Timeout」の3ケースがgreenになる
  - _Requirements: 2.2, 2.3, 3.6_
  - _Boundary: reply_

- [ ] 2.3 (P) pump実走環境でのタスク駆動組合せ検証（スパイク）
  - spawn_local・MessageLoop::run・スレッドメッセージ起床（heartbeat）を同時使用した最小限のecho往復を試作し、組合せが成立するか確認する（spawn/replyとは独立の検証であり、それらへのコード依存を持たない）
  - 成立しない場合は実証済みの代替手段（block_on方式またはmessage-only窓＋PostMessageW方式）を選定し記録する
  - 観測可能な完了条件: 最小echoスパイクが成功する、または代替方式が選定され後続タスクの実装方針として確定している
  - _Requirements: 8.2, 8.3_
  - _Boundary: ui（検証専用）_

- [ ] 3. UIアクター配送ブリッジ（queue+wakeup+pump内drain）の実装
  - UIスレッド上でUIアクターを起動し、他スレッドからのメッセージを非同期チャンネル経由でpumpを塞がずに配送する機能を実装する
  - 受信ループはspawnモジュールへのコード依存を持たず、独立に同一の耐障害規約（handlerのErrは記録して継続・Break/切断でのみ終了）を踏襲する
  - UIスレッド以外からの誤った呼び出しを検出した場合はエラーを記録したうえで戻り値のエラーとして返す（安易なpanicにしない）
  - 観測可能な完了条件: 単体テストでqueueへの格納とUIアクター停止後の送信エラーが確認できる
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 3.2, 3.3, 3.5, 3.7, 6.1_
  - _Boundary: ui_
  - _Depends: 2.3_

- [ ] 4. Integration: toyアクター試験による基盤原語の結線検証
- [ ] 4.1 (P) worker⇄worker往復試験（toy試験a）の実装
  - request/replyの往復・Close→join決定的完走・Close後続メッセージの破棄と要求側切断観測・全送信端dropでの正常終了・panicのjoin観測・handlerのErr後も受信継続、を単一の試験群として実装する
  - 観測可能な完了条件: `cargo test -p areka-actor`実行でtoy試験aの全ケースがpassする
  - _Requirements: 8.1, 1.3, 2.3, 3.3, 3.4, 3.5, 3.6, 3.7_
  - _Boundary: toy tests（worker）_
  - _Depends: 2.1, 2.2_

- [ ] 4.2 (P) worker→UI pump実走echo試験（toy試験b）の実装
  - 2.3で確定した組合せ方式でMessageLoopを bounded 実走させ、workerからUIアクターへのecho往復を検証する試験を実装する
  - 応答不達・期限超過・応答不一致は試験失敗として観測されるようにする
  - 観測可能な完了条件: `cargo test -p areka-actor`実行でtoy試験bが実際のpump上でpassする（無限ブロックしない）
  - _Requirements: 8.2, 8.3, 4.1, 4.2, 4.3, 4.4, 4.5_
  - _Boundary: toy tests（ui）_
  - _Depends: 2.3, 3_

- [ ] 5. Validation: 公開面の最小性とクレート全体検証
  - 公開re-exportの数と型が規約正本（lib.rs）の宣言と一致し、過剰な抽象（共通トレイト・監督ツリー・select等）が追加されていないことを確認する
  - tracing-subscriber初期化コードが本クレートに含まれないことを確認する
  - 観測可能な完了条件: `cargo test -p areka-actor`がクレート全体で全green、かつ公開API一覧が設計のComponents節と一致する
  - _Requirements: 6.2, 7.1, 7.2, 7.3_
  - _Boundary: クレート全体_
  - _Depends: 4.1, 4.2_
