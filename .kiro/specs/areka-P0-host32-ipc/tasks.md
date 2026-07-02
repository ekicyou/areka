# Implementation Plan

- [x] 1. 基盤: 3クレート・ワークスペース雛形とビルド構成
  - `shiori-host32-ipc`（proto）／`shiori-host32-host`（x64+arm64 lib）／`shiori-host32-helper`（i686 bin）の3クレートを新設し、`crates/*` glob でワークスペースメンバー化する
  - 各 `Cargo.toml` に依存を配線（`windows` 0.62.2 ＋ feature `Win32_System_DataExchange` / `Win32_UI_WindowsAndMessaging` / `Win32_Foundation`、`wintf-winmsg-executor` 0.0.5、`event-listener` 5、`thiserror` 2）。`-host` / `-helper` は `-ipc` を依存する
  - 観測可能な完了: `cargo build`（host・x64）と PowerShell での `cargo build -p shiori-host32-helper --target i686-pc-windows-msvc` が空雛形で成功し、ワークスペースが3メンバーを認識する
  - _Requirements: 7.1_

- [x] 2. 共有プロトコル（shiori-host32-ipc / proto）
- [x] 2.1 WM_COPYDATA framing と HWND 符号化・不正フレーム検出
  - `MsgTag`（`dwData` 低32bit・Hello/Load/Request/Response/Unload）、生バイト payload 規約（`cbData`=長さ・固定ヘッダ長0）、HWND の u32 LE 符号化／復元を実装する
  - 未知タグと `cbData`／実長 不整合を破損として検出する（framing 関数レベル）。shift 評価は必ず u64 cast（i686 の `usize`=32bit overflow 回避）
  - 観測可能な完了: 単体テストが x64 と i686 の両ターゲットで green（`MsgTag` u32 往復＋低32bit占有 `u64>>32==0`・未知タグ→Err・HWND u32LE 往復・`cbData` 不整合の拒否）
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 7.2, 7.3_
- [x] 2.2 エラー型・ResponseSlot・送信プリミティブ
  - `IpcError`（thiserror・`Timeout` / `SendFailed`。distinct な PeerGone は設けない）、`ResponseSlot`（single-in-flight・`clear→store→take` の1回消費）を実装する
  - `send_copydata`（`SendMessageTimeoutW` ＋ `SMTO_ABORTIFHUNG` ＋上限時間）と `send_request`（再入受領・1往復）を実装する
  - 観測可能な完了: `ResponseSlot` の clear/store/take 単体テストが green、`IpcError` バリアントが定義され、送信関数が上限時間付きで x64/i686 双方コンパイルできる
  - _Requirements: 2.1, 4.1, 4.3, 5.2, 5.3_
  - _Depends: 2.1_

- [x] 3. helper プロセス（shiori-host32-helper / i686）
  - `wintf-winmsg-executor` の message-only 窓＋`MessageLoop::run` を i686 で回し、起動時に親へ HELLO（自 HWND を u32 LE）を送出する
  - WndProc で REQUEST を受領 → `respond`（echo：受信 payload をそのまま返す）→ RESPONSE を1通返送 → 即 return（それ以上の跨プロセス SendMessage を発行しない）。`respond` は plain fn の echo（下流 `shiori-host32-shiori-load` の差し替え点・trait 抽象は設けない）
  - 観測可能な完了: PowerShell で i686 helper バイナリがビルドでき、in-process／loopback セルフテストで HELLO 送出＋REQUEST→echo RESPONSE＋bounded ループ生存（無クラッシュ）を観測できる
  - _Requirements: 3.1, 4.2, 6.1, 7.1_
  - _Boundary: shiori-host32-helper (HelperMessageWindow, respond)_
  - _Depends: 2.2_

- [x] 4. ホスト側（shiori-host32-host / x64+arm64）
- [x] 4.1 (P) ProcessHost（spawn・非ブロッキング生存監視）
  - `std::process::Command` で helper を起動（親 HWND は u32 ワイヤ値で arg/env に渡す・`windows` 非依存の std-only）、`try_wait` ベースの `poll_exit` / `poll_exit_kind` を実装する
  - `ExitKind` 分類（0=Clean／非0=Abnormal(i32)／コードなし=Terminated）、spawn 失敗は `SpawnError`（稼働中 helper 不在を維持）
  - 観測可能な完了: stand-in プロセス（`cmd.exe /c exit N`）で spawn＋非ブロッキング poll＋ExitKind 分類＋spawn 失敗→`SpawnError` の単体テストが x64 で green
  - task 3（helper・別クレート）と並行実行可
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5_
  - _Boundary: ProcessHost_
  - _Depends: 1_
- [x] 4.2 (P) ParentMessageWindow（HELLO 記録・ハンドシェイク）
  - message-only 親窓を生成し、WndProc で HELLO payload を復号して helper HWND を共有状態へ記録＝ハンドシェイク完了を観測する
  - `pump_until_hello_or(timeout)` を bounded に回し（heartbeat で無入力でも起床）、期限内未受領は `HandshakeError::Timeout` とする
  - 観測可能な完了: 親窓生成後、HELLO 受領で helper HWND が確定し pump が `Some(hwnd)` を返す／未受領で `None`（Timeout）を返すことを観測できる
  - _Requirements: 3.2, 3.4_
  - _Boundary: ParentMessageWindow_
  - _Depends: 2.2_
- [x] 4.3 送信パス（再入受領・ハンドシェイクゲート・timeout）
  - `send_request`：`slot.clear → SendMessageTimeout(REQUEST) → slot.take`。RESPONSE は親 WndProc へ再入配送され store 後即 return（跨プロセス SendMessage なし＝デッドロック回避の核）
  - ハンドシェイク未完の送信を拒否（`HandshakeError::Incomplete`）、上限時間内未応答は `IpcError::Timeout`（`SMTO_ABORTIFHUNG`）。heartbeat は pump フェーズ専用（in-flight 中は `SendMessageTimeout` がブロックし WM_NULL は配送されず `clear→store→take` 不変条件を壊さない）
  - 観測可能な完了: 単一往復が無デッドロックで成立し、未ハンドシェイク送信が `Incomplete` で弾かれ、無応答時に `Timeout` で復帰することを観測できる
  - _Requirements: 3.3, 4.1, 4.2, 4.3, 4.4, 5.1, 5.2, 5.3_
  - _Depends: 4.2, 2.2_

- [x] 5. 統合・検証
- [x] 5.1 往復 echo 統合テスト（M1 ゲート指標）
  - 親窓生成 → i686 helper spawn → HELLO 受領（helper HWND 確定）→ 任意 request bytes を `send_request` → 同一 bytes を response として受領し照合 → 両プロセス生存を確認する
  - 観測可能な完了: PowerShell で（事前ビルドした i686 helper を用いて）`cargo test` の echo 往復が無クラッシュ・無デッドロックで green になる
  - _Requirements: 6.1, 6.2, 6.3, 4.4_
  - _Depends: 3, 4.1, 4.3_
- [x] 5.2 エラー経路の統合テスト
  - ハンドシェイク timeout（HELLO を送らせず pump→`None`）、応答 timeout／wedge（無応答 helper に `send_request`→`Timeout`・親がハングしない）、helper 異常終了検出（強制終了→`poll_exit_kind`→`Abnormal`/`Terminated`）、不正フレーム隔離（実 WM_COPYDATA で未知タグ／`cbData` 不整合を送り観測カウンタ増加・上位へ渡らない）を検証する
  - 観測可能な完了: 4経路それぞれの統合テストが green（各失敗が観測可能な形で報告され、親が無限待機しない）
  - _Requirements: 1.4, 2.5, 3.4, 5.2, 5.3_
  - _Depends: 5.1_
- [x] 5.3 責務境界の確認と i686 ビルド規律の固定
  - `shiori-abi`／pasta／pilot への依存が無いこと、SHIORI3 build/parse・`LoadLibraryW`・常駐 lifecycle を持たないこと、pilot コードの非コピペを確認する
  - i686 target ビルド＋往復 echo テストの PowerShell 手順（helper ビルド → 親テスト）を README／steering に固定する
  - 観測可能な完了: 依存グラフに `shiori-abi`/pasta が無いことを確認でき、文書化した PowerShell 手順どおりに i686 helper がビルド・テストできる（R8 の negative 基準を充足）
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 7.1_
  - _Depends: 5.1_

## Implementation Notes

- 2.2: `IpcError` に第3バリアント `CorruptFrame(FramingError)`＋`From<FramingError>` を追加（task 本文の Timeout/SendFailed 列挙に加えて）。2.1 の framing エラーを `?` で一様化する ergonomic seam で、design File Structure Plan（§145-149 の FramingError doc）が想定済み。純加算・要件違反なし。併せて `FramingError` に `Display`+`core::error::Error` を追加（thiserror が `#[error]` 内包型に Display を要求するため。既存バリアント/`copydata_payload` シグネチャは不変）。
- 環境: `vendors/pasta` submodule はワークツリーで未展開だと `[patch.crates-io] pasta_core` の path 解決に失敗し全 cargo が転ぶ。実装開始前に `git submodule update --init vendors/pasta` 済（本フィーチャの新クレートは pasta 非依存だが、ワークスペース解決に必要）。
- ビルド規律: i686 ターゲット（`--target i686-pc-windows-msvc`）の cargo build/test は**必ず PowerShell** で実行する（Git Bash の GNU coreutils `link.exe` が MSVC link を遮蔽し `'\377\376'` エラーになる）。
- 3: helper は親HWNDを **arg1 優先・fallback env `HOST32_PARENT_HWND`** の **10進 u32**（`parse::<u32>()`）で取得する。**task 4.1 `ProcessHost::spawn` はこの規約（同一 env キー名・10進表現・引数順）で親HWNDを渡す**こと。統合 task 5.1 で実往復として整合を担保する（cross-task 契約）。
- 3: wintf-winmsg-executor 0.0.5 の message-only 窓は **同一 i686 プロセス内で2組独立生成すると2組目が `WindowCreationError`** になる実行時制約あり。helper のセルフテストは窓を1組に集約した単一 loopback テストにし、不正フレーム分類は窓なし純関数（`classify_inbound`）へ切り出して独立検証する。
- 環境: subagent の出力トランスクリプト（tasks/*.output）は0バイトで永続化されない＝消失した未コミット作業は復元不能。**未コミット実装をレビューへ回す前に scratchpad へバックアップ**し、レビュアーには **`git checkout`/`restore`/`stash`/`reset`/`clean` 禁止**（ミューテーション検証は Edit で戻す）を課すこと（task 3 は初回レビュアーの git 復元＋APIクラッシュで実装消失→再実装した）。
- 4.2: `ParentMessageWindow::create` は `WindowCreationError`（thiserror・parent_window.rs）を返す。design §413 は `HandshakeError` と記すが「窓生成失敗はハンドシェイク意味論以前」ゆえ型分離した（設計逸脱・実装が正・妥当）。heartbeat は別スレッド `PostMessageW(WM_NULL)` 25ms 間隔＋deadline 再評価で `pump_until_hello_or` の pump フェーズ専用。
- 5.2: **`copydata_payload` の `LengthMismatch`（cbData≠実長）分岐は実 WM_COPYDATA 受信経路では原理的に到達不能**。`read_copydata` が `cbData` バイトちょうどを `from_raw_parts` で slice して `classify_inbound` へ渡すため、呼び出し点では常に `declared_len == data.len()`。長さ詐称を実配送で作ると境界外読み取り（UB）を招く。よって長さ不整合検出は**単体**（proto `framing_rejects_length_mismatch`・host `length_mismatch_is_ignored_as_bad`）で被覆し、統合テストの WndProc 隔離検証は**到達可能な未知タグ**で行う（ハンドシェイク成立前に未知タグ注入→`pump_until_hello_or`→依然 `None` で非盲目に隔離を観測）。初回実装は cbData 詐称注入で REJECTED→是正した。
- 4.3: `send_request` は `SendError { Handshake(HandshakeError), Ipc(IpcError) }`（両 `#[from]`・lib.rs re-export）を返す。design §417 は `Result<Vec<u8>, IpcError>` と記すが、ゲート拒否（handshake 層・要件3.3）と transport 失敗（要件5.x）の型不整合を層分離で解決したもの（設計逸脱・実装が正）。**design.md §417 の型表記は実装 `SendError` に後追い更新する余地あり**（非ブロッキング）。RESPONSE 再入 store→即 return・heartbeat 不干渉（in-flight は SendMessageTimeout ブロックで WM_NULL 非配送）で `clear→store→take` 不変を保つ。
