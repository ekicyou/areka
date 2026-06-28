# Implementation Plan

- [x] 1. 基盤: モジュール配線とビルド疎通
- [x] 1.1 リファレンス脳とデモドライバのモジュールを宣言しビルドを通す
  - `reference_brain` と `shiori_demo` の最小スタブモジュールを作成し、バイナリへ `mod` 宣言する
  - 既存 `shiori_host`／`shiori_session` の `#![allow(dead_code)]` 依存は本配線で順次解消する前提を確認する（本タスクでは宣言まで）
  - 観測: 両モジュール宣言を含めて `cargo build -p areka` がエラーなく成功する（挙動はスタブ）
  - _Requirements: 1.2_

- [ ] 2. リファレンス脳と生成入口
- [x] 2.1 ライフサイクルとロード状態・未ロード拒否
  - `Load` で受け取った host を AddRef 保持し Loaded へ遷移、`Unload` で Release し Unloaded へ遷移する
  - ロード状態を保持し、未ロード状態の `Request` を有効処理として受理せず判別可能な失敗（`SHIORI_E_NOT_LOADED`）として報告する
  - ロード／アンロードの失敗を呼び出し側へ判別可能に報告する
  - 観測: 未ロード時の `Request` が `SHIORI_E_NOT_LOADED` を返すユニットテストが通る
  - _Requirements: 1.1, 1.3, 2.1, 2.2, 2.3, 2.4_
  - _Boundary: ReferenceBrain_

- [x] 2.2 即時応答（固定／エコー・content 不透明）
  - `Request` を同期メソッド呼び出しとして受け、`S_OK`＋応答文字列を出力引数へ move-out する
  - 応答文字列を固定文字列または受信 content のエコーとして生成し、content を解析・スキーマ検証・意味づけしない（不透明 UTF-16 のまま取り回す）
  - 観測: 即時応答の HSTRING が往復し内容が不解釈のまま一致するユニットテストが通る
  - _Requirements: 1.4, 3.1, 3.2, 3.3, 3.4, 8.1_
  - _Boundary: ReferenceBrain_

- [x] 2.3 遅延応答（pending＋トークン）と Complete 発火
  - 遅延扱い時に即時応答文字列を伴わず `SHIORI_S_PENDING`＋採番した相関トークンを返し、トークンを完了まで突合可能に保持する
  - 保持 host へ vtable 直呼びで `Complete(token, response)` を発火し、対応トークンと応答文字列を渡す
  - 観測: 遅延 `Request` が `SHIORI_S_PENDING`＋トークンを返し、保持 host へ `Complete` を発火するユニットテストが通る
  - _Requirements: 4.1, 4.2, 4.4_
  - _Boundary: ReferenceBrain_

- [ ] 2.4 能動通知 Raise
  - 保持 host へ vtable 直呼びで `Raise(script)` を発火し、通知内容を固定または既知の不透明文字列として渡す（内容を解釈しない）
  - 観測: 保持 host へ `Raise` を固定文字列で発火するユニットテストが通る
  - _Requirements: 5.1, 5.2_
  - _Boundary: ReferenceBrain_

- [ ] 2.5 純粋C コンストラクタ shiori_create
  - `IShiori` 実体生成の唯一の純粋C コンストラクタ `shiori_create` を、COM 標準呼出規約（`extern "system"`）＋C リンケージ（`#[unsafe(no_mangle)]`、edition 2024 形）で公開する
  - 成功時は参照カウント 1 の `IShiori` を出力引数へ move-out し成功 HRESULT を返す。失敗時は出力を書き込まず判別可能な失敗 HRESULT を返す
  - 対象を COM（x64／ARM64・in-proc）生成入口に限定する（過去互換 flat-C・32bit DLL ホスティングは対象外）
  - 観測: 成功時 refcount 1 の `IShiori`＋`S_OK`、失敗時 out 未書込・失敗 HRESULT を確認するユニットテストが通る
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.6, 9.7_
  - _Boundary: ReferenceBrain_
  - _Depends: 2.1_

- [ ] 2.6 リファレンス見本ドキュメント（module-level doc）
  - 脳モジュールの module-level doc に、各経路（ロード／アンロード・即時・遅延・Raise）の正解見本説明、content 不透明・固定／エコー方針、下流（host-32／reference-ghost）の参照位置づけを集約する
  - 正準 content プロトコルは完了仕様 `areka-P0-shiori-protocol`（`doc/shiori/fragments/`）の責務であり参照・複製しない旨を明示する
  - 観測: 脳モジュールの doc が各経路・content 不透明方針・下流位置づけ・protocol 委譲を記載している
  - _Requirements: 7.1, 7.2, 7.3, 8.2_
  - _Depends: 2.1, 2.5_

- [ ] 3. デモドライバ
- [ ] 3.1 セッション駆動と各経路の観測
  - `shiori_create` で `IShiori` を取得・所有し、`ShioriSession::activate` で in-proc アクティベーションする
  - OnBoot 形の不透明リクエストを例に、即時応答・遅延応答・能動通知（Raise）を含む数往復をドライブし、既存セッション規律（単一 in-flight・相関トークン突合・タイムアウト）に従い `poll_completions` を同一ループで drain して遅延完了を待ち合わせる
  - 即時応答・遅延応答（最低 1 回ずつ）・Raise（最低 1 回）を実演し、完了後 `unload` で後始末して `IShiori` を Release する
  - 各経路の疎通結果を構造化 `tracing::info!`（`logging.md` 準拠）で観測可能にする（視覚 UX・会話描画に依存しない）
  - 観測: デモ駆動で即時・遅延+Complete・Raise・unload の各経路が `tracing` の info ログとして出力される
  - _Requirements: 4.3, 5.3, 6.1, 6.2, 6.3, 6.4, 6.5, 9.5_
  - _Boundary: ShioriDemoDriver_
  - _Depends: 2.1, 2.2, 2.3, 2.4, 2.5_

- [ ] 3.2 エラー処理とフラグゲート
  - 駆動失敗（生成失敗・セッション規律違反・遅延タイムアウト）を判別可能に保持し `tracing::error!` で報告したうえで、いずれの失敗でも `unload` 後始末を試みる
  - デモはフラグまたは環境変数で明示有効化されたときのみ起動し、既定（通常起動）では駆動しないゲートを設ける
  - 観測: フラグ無効時は `run_demo` が起動せず、失敗注入時も後始末が試みられ `tracing::error!` で報告される
  - _Requirements: 6.6, 6.8_
  - _Boundary: ShioriDemoDriver_
  - _Depends: 3.1_

- [ ] 4. main 統合（フラグゲートフック）
- [ ] 4.1 メッセージループ前のデモフック配線
  - `WinThreadMgr` 構築後・`mgr.run()` 呼び出し前に、フラグ／環境変数が有効なときのみ `run_demo()` を main スレッドで同期呼び出しする
  - デモ配線により不要となった `shiori_host`／`shiori_session` の `#![allow(dead_code)]` を整理する
  - 観測: フラグ有効時に通常起動経路でデモが一度駆動され、既定では駆動されない（`mgr.run()` の UI 立ち上げを阻害しない）
  - _Requirements: 6.1, 6.8_
  - _Boundary: main.rs_
  - _Depends: 3.2_

- [ ] 5. テストと検証
- [ ] 5.1 (P) 統合テスト（ShioriSession 越しデモ経路）
  - 即時→遅延+Complete→Raise→unload の数往復を `ShioriSession` 経由で駆動し、`poll_completions` が完了／通知を drain し保留が解除されることを検証する
  - 単一 in-flight（遅延保留中の次 `request` が `RequestInFlight` で拒否）、`expire_if_elapsed` の決定的タイムアウト、タイムアウト後の stale `Complete` が `SHIORI_E_UNKNOWN_TOKEN` で弾かれること、失敗注入時の `unload` 後始末を検証する
  - 観測: 上記経路の統合テストが決定的に（実時間 sleep に依存せず）通る
  - _Requirements: 6.4, 6.5, 6.6_
  - _Boundary: 統合テスト_
  - _Depends: 3.1, 3.2_

- [ ] 5.2 手動検証とマルチターゲットビルド
  - フラグ／環境変数有効時のみ `run_demo()` が起動し既定では駆動しないことを手動確認する
  - debug 実行（または `RUST_LOG=info`）で各経路の info ログが観測でき、視覚 UX・会話描画に依存しないこと、会話描画・さくらスクリプト解釈・balloon 反映を行わないことを確認する
  - x64 と ARM64 の両ターゲットでビルドが通ることを確認する（ソース分岐なし）
  - 観測: フラグ有効 debug 実行で各経路の info ログが出力され、x64／ARM64 双方のビルドが成功する
  - _Requirements: 6.7, 6.8, 8.3_
  - _Depends: 4.1_
