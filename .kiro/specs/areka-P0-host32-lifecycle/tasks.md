# Implementation Plan

- [ ] 1. Foundation: ログ依存追加・常設監視モジュールの型骨格
- [x] 1.1 `tracing` 依存追加と `lifecycle` モジュールの新設
  - `shiori-host32-host/Cargo.toml` に `tracing = { workspace = true }` を追加する（steering `logging.md` 準拠・新規承認不要）
  - `src/lifecycle.rs` を新設し、`lib.rs` に `pub mod lifecycle;` を登録する
  - Observable: `cargo build -p shiori-host32-host` が新規（空）モジュールを含めて成功する
  - _Requirements: 7.6_

- [x] 1.2 統一報告語彙・shutdown 失敗語彙の型骨格と定数を定義
  - 死活状態（稼働中／終了種別付き終了）を表す型、死活・request 失敗の突合結果を表す分類型、突合結果と原因を束ねる報告型、shutdown 経路専用の失敗語彙型（送出失敗／ack 契約違反／終了未観測の3態）を定義する（処分・再起動判断のバリアントは持たない）
  - UNLOAD ack 待機上限（30秒）・終了観測上限（10秒）・bounded poll 刻み（5ミリ秒）の定数を定義する
  - 上記すべての型が `Send` であることを静的アサーションの単体テストで固定する
  - Observable: `cargo test -p shiori-host32-host` で型骨格に対する `Send` アサーションテストが通る
  - _Requirements: 2.4, 2.6, 2.7, 5.4, 5.5, 7.7_
  - _Depends: 1.1_

- [ ] 2. Core: 死活監視の常設化と統一報告語彙（host 側・`lifecycle.rs`）
- [x] 2.1 死活・request 失敗の突合分類ロジックを実装
  - 「終了検出済みなら常に死活起因（種別を保持）」「生存中の応答なしはタイムアウト」「生存中の送出失敗は transport 異常」「SHIORI エラー応答は区別保持」「未ハンドシェイクは別区分」という5パターンの突合ロジックを、既存の request 失敗語彙と死活種別を入力に決定的に判定する純関数として実装する
  - 突合表の5パターンすべてを網羅する単体テストを追加する
  - Observable: `cargo test -p shiori-host32-host` で突合表の全5パターンに対応するテストが個別に通る
  - _Requirements: 2.1, 2.2, 2.3, 2.4_
  - _Boundary: 統一報告語彙（classify_failure）_
  - _Depends: 1.2_

- [x] 2.2 死活監視の器（非ブロッキング・sticky・冪等後始末）を実装
  - 既存の helper ハンドルを単独所有し、非ブロッキングな死活問い合わせを提供する監視の器を実装する（呼び手スレッドを一切ブロックしない）
  - 一度「終了」を観測したら以後は再ポーリングせず同じ終了種別を返す sticky キャッシュ挙動を実装する
  - 冪等な強制終了（二重呼び出しでも成功扱い）と、破棄時の自動後始末（後始末失敗はログのみでパニックしない）を実装する
  - 単体テスト: ポーリングが1秒未満で完了すること（非ブロッキングの裏付け）、スタンドインプロセスの終了コード0／5がそれぞれ正常終了／異常終了として分類されること、終了検出後の再問い合わせが同じ結果を返す（sticky）こと、終了済みへの二重終了要求が成功として扱われること
  - Observable: 上記単体テストがすべて通り、非ブロッキング・sticky・冪等後始末の3性質が個別に確認できる
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 5.2, 7.7_
  - _Boundary: 死活監視の器（HelperLifecycle）_
  - _Depends: 1.2_

- [x] 2.3 死活・request 失敗の統一報告の生成とログ規律の実装
  - 死活監視の器に、request 失敗を受け取り現在の死活状態と突合して統一報告を返す機能を実装する（死活起因と判定された場合はログを発行する）
  - 失敗経路（強制終了の入出力エラー、破棄時後始末の入出力エラー、統一報告生成時の死活起因判定）にログ発行を配置し、いずれも戻り値の `Err`／報告データとして握り潰さず返す
  - Observable: 単体テストで、request 失敗と死活状態の組み合わせから統一報告が生成され、死活起因判定時にログが発行されることが確認できる
  - _Requirements: 2.5, 2.6, 2.7, 4.4, 7.6_
  - _Boundary: 死活監視の器（HelperLifecycle）_
  - _Depends: 2.1, 2.2_

- [x] 2.4 正規の正常終了要求（ホスト側）と shutdown 失敗語彙を実装
  - 死活監視の器に、正常終了要求を行うメソッドを実装する: 既に終了済みなら送出せず短絡成功、未終了なら凍結済み wire 語彙の UNLOAD 種別で正常終了要求を送出し、ack が厳密な期待バイト列であることを確認し、終了種別を bounded ポーリングで観測して返す
  - 送出失敗・ack 契約違反・終了未観測の3失敗経路をそれぞれ専用の shutdown 失敗語彙で表し、ログ発行と戻り値の `Err` の両方で surface する
  - 再起動を試みるロジックは一切持たない
  - Observable: 既終了スタンドインへの正常終了要求が短絡的に成功として返る単体テストが通る（実 helper なしで検証可能な短絡経路）
  - _Requirements: 5.1, 5.3, 5.4, 5.5_
  - _Depends: 2.2_

- [ ] 3. Core: 実 helper への正規正常終了経路の増設（helper 側・別クレート）
- [x] 3.1 (P) UNLOAD 受信の分類と終了要求フラグを追加
  - 受信メッセージ分類ロジックに、UNLOAD 種別（ペイロード有無を問わない）を専用アクションとして区別する分岐を追加する（現状の「既知だが無視」対象から UNLOAD を除く）
  - helper の共有状態に、終了要求フラグ（および観測用カウンタ）を追加する
  - 単体テスト: UNLOAD をペイロードあり／なし双方で専用アクションに分類すること、既存の HELLO／LOAD／REQUEST の分類が影響を受けないこと
  - Observable: i686 対象の単体テストで新しい分類とフラグの追加が既存分類に無影響であることが確認できる
  - _Requirements: 5.6_
  - _Boundary: helper 受信分類・共有状態（shiori-host32-helper）_

- [x] 3.2 UNLOAD 応答アーム（courtesy unload → ack → メッセージループ正常終了）を実装
  - UNLOAD 受信時に、SHIORI プロキシを取り出して即座に破棄する（借用を保持しないことで courtesy unload・ライブラリ解放を安全に実行する）
  - 終了要求フラグを立て、厳密1バイトの ack を返送する（既存の LOAD ack と同型の応答経路を使う・新しい応答契約を発明しない）
  - 自プロセスのメッセージループを起こす通知を送り、メインループのフィルタが終了要求フラグを検知したらループを正常終了させ、プロセスが終了コード0で終わるよう結線する
  - Observable: スタンドイン経由の統合テストで、UNLOAD 受信後に helper プロセスが終了コード0で終了し、ack 送出が unload 完了後・ループ終了前の順序で行われたことが確認できる
  - _Requirements: 5.1, 5.6_
  - _Depends: 3.1_

- [x] 3.3 helper 側 UNLOAD 経路の loopback 統合テストを追加
  - 既存の loopback テスト群に UNLOAD ケースを追加する: UNLOAD 送出後に SHIORI プロキシが未確立状態に戻ること、終了要求フラグが立つこと、親スタンドインが厳密1バイトの ack を受領することを確認する
  - Observable: `cargo test -p shiori-host32-helper --target i686-pc-windows-msvc`（PowerShell 実行）で新しい UNLOAD ケースを含む loopback テスト一式が通る
  - _Requirements: 5.1, 7.3, 7.4_
  - _Depends: 3.2_

- [x] 4. Integration: `lifecycle` モジュールの公開 re-export とワークスペース整合確認
  - `shiori-host32-host` の `lib.rs` から、死活監視の器・統一報告語彙・突合純関数・shutdown 失敗語彙を公開 re-export する
  - 凍結境界（`shiori-host32-ipc` の wire／framing／`MsgTag`／`ResponseSlot`／timeout）と `host32-request` の出口 API（`Shiori3Client`／`RequestError`）が一切変更されていないことを確認する
  - Observable: `cargo build` がワークスペース全体で成功し、新しい公開項目に未使用警告や可視性エラーが出ない
  - _Requirements: 1.6, 2.4, 7.1, 7.2_
  - _Depends: 2.3, 2.4, 3.3_

- [ ] 5. Validation: 常駐健全性の決定的end-to-end実証
- [ ] 5.1 周期運転（連打）と正規clean shutdownのend-to-endテストを実装
  - 実 i686 helper と fixture を起動し、ハンドシェイク後、イベント意味論を持たないダミーIDで固定応答の往復を200回連続（実時間 sleep なしの back-to-back）で行い、各往復の成功とfixture固定応答を確認する
  - 反復後もhelperが生存継続していることを確認したうえで、正常終了要求を発行し、正規の正常終了経路を通じて正常終了種別が観測されることを確認する
  - Observable: `cargo test -p shiori-host32-host --test lifecycle_cyclic_e2e`（PowerShell実行）が実 i686 helper 越しに通り、200回の連打成功と正規clean shutdownの両方を一つのテストで実証する
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 5.1, 5.3, 7.3, 7.4, 7.5_
  - _Depends: 4, 3.3_

- [ ]* 5.2 env-gate 実 SHIORI DLL への周期連打confidenceテストを追加
  - 実DLLを指すenvが未設定なら明示的にskipし、設定済みだがDLLが見つからない場合は明示的に失敗させ、設定済みで存在する場合は300回の通知連打（応答内容非依存・transport健全性のみ観測）を行った上で正規clean shutdownを確認する
  - 同一バイナリ内に窓を使うテストが2本（周期連打・本テスト）存在するため、env設定時は直列実行（`--test-threads=1`）が必要であることをテスト実行手順として明記・確認する
  - Observable: env未設定時は明示的なskipメッセージが観測でき、env設定＋`--test-threads=1`実行時は2窓制約の衝突なく完走する
  - _Requirements: 6.1, 6.2, 6.3_
  - _Depends: 5.1_

- [ ] 5.3 (P) 強制kill注入と統一報告のend-to-endテストを実装
  - 周期連打テストとは別のテストバイナリとして（1窓制約対処）、実 i686 helper を起動しベースラインの往復成功を確認した後、強制終了を注入する
  - bounded ポーリングで異常な終了種別（helperプロセス由来の非正常終了）が検出されること、その後のrequestが無限待ちにならず有限時間内に観測可能なエラーとして返ること、そのエラーから統一報告が死活起因として分類されることを確認する
  - 二重の強制終了要求が冪等に成功として扱われることも同一runで確認する
  - Observable: `cargo test -p shiori-host32-host --test lifecycle_kill_e2e`（PowerShell実行）が通り、異常検出・有限復帰・統一報告分類・冪等な二重終了要求のすべてが一つのテストで実証される
  - _Requirements: 4.1, 4.2, 4.3, 2.1, 2.5, 5.2_
  - _Boundary: lifecycle_kill_e2e.rs（周期連打テストとは別バイナリ）_
  - _Depends: 4, 3.3_
