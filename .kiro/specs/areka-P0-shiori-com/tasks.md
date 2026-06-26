# 実装計画: areka-P0-shiori-com

> 設計の正本は `design.md`、要件は `requirements.md`。本計画は機能作業の分解であり、ファイル/型/シグネチャの詳細は design.md を参照する。
> 依存は番号順が基本。番号順で表せない横断依存のみ `_Depends:_` で明示する。`(P)` は直前の同位タスクと並行実行可能を表す。

- [ ] 1. Foundation: クレート作成と基本型
- [x] 1.1 shiori-abi クレートの新規作成と最小依存配線
  - `crates/shiori-abi` を新規作成し、`windows-core` 0.62.2 / `windows`（`Win32_System_Com`）/ `thiserror` 2 への最小依存のみを宣言する
  - `wintf`/`dola`/`bevy_ecs` に依存させない（下流 32bit ターゲットでもビルド可能な最小構成を保つ・x64/CPU ネイティブ前提）
  - 観測: `cargo build -p shiori-abi` が空の `lib` で成功し、依存ツリーに UI 基盤クレートが含まれないこと
  - _Requirements: 5.3_

- [x] 1.2 (P) エラー型と HRESULT 規約の定義
  - `ShioriError`（`thiserror`）と、カスタム HRESULT 定数 `SHIORI_S_PENDING`（成功・遅延）/ `SHIORI_E_NOT_LOADED` / `SHIORI_E_UNKNOWN_TOKEN`、および HRESULT⇄`ShioriError` 変換を定義する
  - 命名は既存 COM 規約（`com-resource-naming-unification`）と整合させる
  - 観測: `S_OK`/`SHIORI_S_PENDING`/各 error コード ⇄ `ShioriError` のマッピング単体テストが緑
  - _Requirements: 2.3, 2.4, 3.5, 3.6, 7.1, 7.2, 7.3_
  - _Boundary: shiori-abi/error.rs_
  - _Depends: 1.1_

- [x] 1.3 (P) 結果型と相関トークンの定義
  - `RequestOutcome`（`Immediate(HSTRING)` / `Deferred(CorrelationToken)`）と `CorrelationToken`（`u64`・単調増加採番、ABI 非公開の Rust 内部表現）を定義する
  - 観測: `S_OK`+response→`Immediate`、`SHIORI_S_PENDING`+token→`Deferred` の構築と、トークン単調増加・完了後再利用ポリシーの単体テストが緑
  - _Requirements: 3.2, 3.3, 3.5_
  - _Boundary: shiori-abi/outcome.rs_
  - _Depends: 1.1_

- [ ] 2. Core: raw COM ABI 層（カスタム `#[interface]` 定義）
- [x] 2.1 `IShiori` raw インターフェイスの定義
  - `#[interface(IID)]` で `IShiori`（`Load`/`Unload`/`Request`）を `unsafe fn -> HRESULT` 形で定義し、`Request` は `input`(`*const HSTRING`)・`out_response`(`*mut HSTRING`)・`out_token`(`*mut u64`) を取る
  - content は不透明 HSTRING として扱い（正準プロトコル json-rpc 採用は設計判断・本層ではパースしない）、in-proc 直 vtable によりマーシャリング非介在とする
  - HSTRING 所有権規約を doc 化: `[out]`=callee 確保/caller 解放（move-out/Drop）、`[in]`=借用
  - 観測: `IShiori` が IID 付きでコンパイルし、vtable のメソッド面・引数順が design.md と一致すること
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.6, 2.1, 2.2, 3.1, 3.4, 4.1, 5.4, 7.1_
  - _Boundary: shiori-abi/interface.rs_
  - _Depends: 1.1_

- [x] 2.2 `IShioriHost` raw インターフェイスの定義
  - `#[interface(IID)]` で `IShioriHost`（`Raise(script)` / `Complete(token, response)`）を定義する。単一 sink が能動通知と遅延完了の双方を受ける
  - `script`/`response` は `[in]` 借用（呼び出し中のみ有効）の HSTRING 規約を doc 化する
  - 観測: `IShioriHost` が IID 付きでコンパイルし、`Raise`/`Complete` の vtable 形が design.md と一致すること
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_
  - _Boundary: shiori-abi/interface.rs_
  - _Depends: 2.1_

- [ ] 3. ShioriExt エルゴノミック変換層
  - `&IShiori` に対する拡張トレイト `ShioriExt`（`load(&self, host: &IShioriHost)` / `unload` / `request`）を実装し、raw `unsafe` 呼び出しを `Result<RequestOutcome, ShioriError>` へ変換する（呼び出し側に raw/HRESULT を露出させない）
  - HRESULT マッピング: `S_OK`→`Immediate`、`SHIORI_S_PENDING`→`Deferred`、`SHIORI_E_NOT_LOADED`→`NotLoaded`、その他失敗→`ShioriError`。HSTRING 所有権規約（`[out]` move/Drop・`[in]` 借用）を正しく実装する
  - 観測: モック `IShiori` 経由で `request` が `Immediate`/`Deferred`/`Err` を返し分け、未ロード時に `NotLoaded` を返す単体テストが緑
  - _Requirements: 1.2, 2.1, 2.2, 2.3, 2.4, 3.1, 3.2, 3.3, 3.5, 3.6, 4.1, 4.2, 4.3, 7.1, 7.2_
  - _Boundary: shiori-abi/ergonomic.rs_
  - _Depends: 1.2, 1.3, 2.1, 2.2_

- [ ] 4. Integration: areka 側 host 実装と in-proc アクティベーション（最小受け皿）
- [ ] 4.1 areka 側 `IShioriHost` 実装（sink・突合枠・mailbox 投函）
  - `areka` クレートに `shiori-abi` の path 依存を追加し、`#[implement(IShioriHost)]` で `Raise`/`Complete` を実装する
  - 突合枠 `Option<CorrelationToken>` を所有し、`Complete` を thread-safe に areka のメールボックスへ投函して即返す。未知/stale トークンは `SHIORI_E_UNKNOWN_TOKEN` を返す。`[in]` HSTRING は保持時に clone する
  - 非循環所有: host 実装は脳へ強参照を持たない
  - 観測: `Complete` が token を突合枠と照合してメールボックスへ投函し、未知トークンでは `SHIORI_E_UNKNOWN_TOKEN` を返す結合テストが緑
  - _Requirements: 3.3, 6.1, 6.2, 6.3, 6.4, 6.5_
  - _Boundary: areka / IShioriHost 実装_
  - _Depends: 2.2, 1.2_

- [ ] 4.2 in-proc アクティベーション経路と利用規律（単一 in-flight・タイムアウト）
  - 同一プロセス内で `IShiori` 実装へ到達し、`Load` で areka 実装の sink を受け渡す最小経路を実装する
  - 単一 in-flight 規律（`Deferred` 中は対応する `Complete` まで次 `Request` を発行しない）、設定可能な遅延完了タイムアウトで保留枠を放棄し次 request を許可、`Unload` で保留取消。`Request`/`Load`/`Unload` は非ブロッキング前提
  - host 寿命: 脳が `Load`〜`Unload` 間 AddRef 保持し `Unload` で Release、areka は脳解放前に必ず `Unload` を呼ぶ
  - 観測: アクティベーションが native `IShiori` 実装へ in-proc 到達し `Load` で sink を渡すこと、保留 request がタイムアウトで枠解放され次 request が可能になることを結合テストで確認
  - _Requirements: 1.5, 2.4, 5.1, 5.2_
  - _Boundary: areka / in-proc activation wiring_
  - _Depends: 4.1, 3_

- [ ] 5. Validation: in-proc モック脳による結合テスト
- [ ] 5.1 モック脳と即時往復・所有権/非マーシャリングの実証
  - `#[implement(IShiori)]` のモック脳と in-proc 結合テストハーネスを用意する
  - 観測: `ShioriExt::request` の即時応答 HSTRING が内容一致で往復し、HSTRING の Drop 回数観測により二重解放・リークが発生しないこと（所有権規約＋in-proc 非マーシャリング R4.3 の実証）を結合テストで確認
  - _Requirements: 1.2, 3.1, 3.2, 4.1, 4.3, 5.2, 5.4_
  - _Depends: 3, 2.1_

- [ ] 5.2 遅延応答と push 経路の結合テスト
  - モック脳が `SHIORI_S_PENDING`＋token を返し、後で `IShioriHost::Complete(token, response)` を呼ぶ。areka sink が token を突き合わせて応答を配送する。`Raise` の能動通知配送も検証
  - 観測: 遅延完了が token 突合で配送され、`Raise` が届いて内容一致し、stale/未知トークンの `Complete` が `SHIORI_E_UNKNOWN_TOKEN` で拒否される結合テストが緑
  - _Requirements: 3.3, 6.1, 6.3, 6.4, 6.5_
  - _Depends: 4.1, 4.2_

- [ ] 5.3 ライフサイクルと単一 in-flight 規律の結合テスト
  - `Load`→`Request`→`Unload` の遷移、未ロード時 `Request` 拒否（`NotLoaded`）、`Unload` での保留取消、`Deferred` 中に次 `Request` を出さない規律を検証する
  - 観測: ライフサイクル遷移が成立し、未ロード時 request が拒否され、保留中の `Unload` で保留が取り消される結合テストが緑
  - _Requirements: 2.1, 2.2, 2.4_
  - _Depends: 4.2_

## Implementation Notes
- レビュー時に RED 再現目的で `git checkout`/破壊的 git を共有ワークツリーで実行しないこと（task 2.1 の成果を一時消失させかけた）。RED 再現はファイルのコピー退避で行う。
- windows-core 0.62 のカスタム COM は `#[interface("v4-GUID")] unsafe trait X: IUnknown { unsafe fn .. -> HRESULT }`＋`#[implement(X)]`＋`X_Impl` で確立。`#![allow(non_snake_case)]` をモジュール先頭に置く（`#[interface]` は trait への非 doc 属性を拒否）。IID は dev 値で固定しリリース時凍結（D7）。
