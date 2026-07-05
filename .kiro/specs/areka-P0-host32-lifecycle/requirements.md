# Requirements Document

## Project Description (Input)

M1 `areka-P0-emo2-boot` の「① SHIORI 通信層エンジン host-32」トラック**最終ユニット**（逐次チェーン: pilot✅ → `host32-ipc`✅ → `host32-shiori-load`✅ → `host32-request`✅ → **本ユニット**）。

**問題**: host-32 は「load 貫通・request 往復」まで完成した（`host32-request` 完了）が、**常駐運転の健全性が未証明**である。ゴーストは分単位・時間単位で生き続け毎秒 request を受ける存在であり、① helper の長時間 msg loop 生存、② 周期 request 連打への耐性、③ helper 異常（crash／強制終了）の**検出と観測可能な報告**、④ clean shutdown の全経路、が実証されていない。ここが埋まらないと下流 `kanade`（毎秒 pump の運行）が砂上に立つ。

**現状**: request 完了時の実シンボルとして `Shiori3Client::get`／`notify`（同期往復・`RequestError{Handshake/Timeout/Ipc/Shiori}` 区別語彙）、`spawn(helper_exe, load_dir, shiori_name, parent_hwnd) -> HelperHandle`、そして `poll_exit(handle) -> Option<i32>`／`poll_exit_kind(handle) -> Option<ExitKind>`（非ブロッキング）＋ `ExitKind::{Clean, Abnormal(i32), Terminated}` が存在する。死活検出の seam は既にあるが、監視ループ化・異常時の上位報告経路・周期運転試験は未実装（確認済み）。`HelperHandle::terminate()`（冪等）も検証用に存在する。teardown は `ShioriByteProxy` Drop（courtesy unload ＋ FreeLibrary）として load 層で確立済み。

**あるべき姿**: 実 i686 helper が**長時間運転（周期 request 連打）で健全**であり、**異常終了が検出・区別・報告**され、**clean shutdown の全経路**（通常終了・異常後の後始末）が決定的に通る。

## Introduction

本仕様の到達目標は「**実 i686 helper が常駐運転（周期 request 連打）に耐えて健全であり、helper の異常終了が検出・区別・観測可能なエラーとして上位へ報告され、shutdown の全経路（通常終了・異常後の後始末）が決定的に通る**」ことである。これは host-32 トラックの常駐健全性の証明と、下流 `kanade` が消費する死活報告 API の確定を担う。

本ユニットは既存の死活検出 seam（`poll_exit`／`poll_exit_kind`／`ExitKind`）を**監視として常設化**し、request 経路の失敗語彙（`RequestError`）と helper の終了種別（`ExitKind`）を突合して、呼び手（将来 `kanade`）が「helper が死んだ／応答しないだけ／SHIORI がエラーを返した」を**単一の語彙で区別**できる報告へ整理する。周期 request は「耐性試験の負荷」であって**イベント意味論を持たない**（ID はダミーで可）。

本仕様は**再起動戦略を持たない**——検出と報告までを担い、自動再起動・縮退の判断は下流（`kanade`／`ghost-setup`）の M2 領分である。上流の凍結境界（`shiori-host32-ipc` の wire／framing／`MsgTag`／`ResponseSlot`／timeout）は一切改変しない。

## Boundary Context

- **In scope（本仕様が担う観測可能な振る舞い）**:
  - **[死活監視の常設化]** 既存の非ブロッキング死活 seam（`poll_exit`／`poll_exit_kind`／`ExitKind`）を常設監視として運用し、稼働中／終了種別（`Clean`／`Abnormal(i32)`／`Terminated`）を観測点として上位へ供給する。監視は呼び手スレッドをブロックしない。
  - **[異常語彙の統一報告]** helper 死亡に起因する request 失敗（`RequestError::Ipc`／`Timeout`）と、`ExitKind` の異常終了（`Abnormal`／`Terminated`）を突合し、呼び手が「helper 死亡」「応答不能／タイムアウト」「SHIORI エラー応答」を区別できる報告として surface する。単一の不透明失敗へ潰さない。
  - **[周期運転試験]** 実 i686 helper（testdll fixture）に対し、OnSecondChange 相当の頻度で request を連打する決定的ハーネスを走らせ、反復往復が全て成功し、リーク・ハンドル枯渇・`ResponseSlot` 巻き込みなく、往復後に helper が生存継続（`poll_exit_kind` → `None`）することを観測する。
  - **[強制 kill 注入試験]** 稼働中 helper を強制終了（`terminate` 相当）した後、監視が終了種別（`Abnormal`／`Terminated`）を検出し、以降の request が観測可能なエラーとして返る（ハング・無限待ちを起こさない）ことを観測する。
  - **[shutdown 全経路]** 通常終了（helper プロセス終了 → `ExitKind::Clean` 確認）と、異常後の後始末（`Abnormal`／`Terminated` 検出 → ハンドル後始末・二重 kill 安全＝冪等）の双方が決定的に通る。
  - **[env-gate 実 pasta 追験]** env 指定時に実 pasta.dll に対する長時間相当の周期運転を confidence 検証として実行する（CI 必須ゲートにはしない）。
- **Out of scope（本仕様が所有しないもの）**:
  - イベントカタログ・OnSecondChange の意味論・発火順序・Value 配送・boot/close 運行（→ 下流 `areka-P0-kanade`）。本仕様の周期 request はイベント意味論を持たない負荷であり、ID はダミーで可。
  - 自動再起動・縮退戦略・helper のプロセス処分方針（→ M2・判断は上位 `kanade`／`ghost-setup`）。本仕様は検出と報告までに留める。
  - IPC フレーム・wire・`MsgTag`・`ResponseSlot`・timeout 機構の変更（`shiori-host32-ipc` 凍結・不改変）。
  - SHIORI/3.0 request 組立・response 解析 codec の新設や `Shiori3Client` 出口 API の意味論変更（`host32-request` 完了・本仕様は消費するのみ）。
  - SAORI（emo2 未使用）。
  - 専用監視スレッドの導入や `areka-actor` への結線（本ユニットは actor 非依存＝先行可。監視方式が親窓 pump スレッド内 poll で足るか専用スレッドが要るかは design 判断であり、要件は「非ブロッキング・`Send` な報告データ」という観測可能な制約のみを固定する）。
- **Adjacent expectations（隣接仕様への期待）**:
  - **上流 `areka-P0-host32-request`（完了）**: `Shiori3Client::get`／`notify`、`RequestError{Handshake/Timeout/Ipc/Shiori}`、`spawn`／`HelperHandle`／`poll_exit`／`poll_exit_kind`／`ExitKind{Clean/Abnormal(i32)/Terminated}`／`HelperHandle::terminate()`（冪等）を提供済み。本仕様はこれらの上に監視・報告・試験を増分し、既存の意味論を変更しない。
  - **上流 `shiori-host32-ipc`（完了・凍結）**: wire／framing／`MsgTag`／`ResponseSlot`／`SMTO_ABORTIFHUNG` ＋ timeout を提供済み。本仕様はこれを不透明に利用し、有限復帰（ハングしない）はこの timeout 機構に乗る。
  - **下流 `areka-P0-kanade`（後続）**: 本仕様の死活報告 API と統一失敗語彙を、毎秒 pump（OnSecondChange 循環）の request 送出の健全性判断としてそのまま消費する。**報告型は本仕様が正本**であり、`kanade` は消費・再定義しない。報告データは `Send` な所有データで、将来の shiori アクター inbox 処理から非ブロッキングに呼べる形に切る。
  - **下流 `ghost-setup`（後続）**: shutdown 全経路（通常終了・異常後後始末）を終了系列の受け皿として共有する。
  - **ukadoc 正典**: 本ユニットはイベント意味論非依存ゆえ ukadoc 参照は最小。OnSecondChange の Reference 詳細・発火規律は `kanade` の領分。unload の作法は `host32-shiori-load` 完了時の確立済み契約（courtesy unload）を踏襲する。
  - **未確定で design が埋める項目**: 監視の駆動タイミング（request 前後 or 周期チェック・専用スレッド要否）／統一報告型の具体形（`RequestError` の拡張か包む型か）／周期運転試験の反復回数と決定性の担保方法。これらは design フェーズで確定する。

## Requirements

### Requirement 1: helper 死活の常設監視

**Objective:** As a host-32 ホスト層, I want 稼働中 helper の生死と終了種別を常設の非ブロッキング監視で把握する, so that 常駐運転中に helper が死んだ事実を即時かつ呼び手スレッドを止めずに検知できる

#### Acceptance Criteria

1. While helper が稼働中である, the host-32 ホスト層 shall 死活問い合わせに対して「稼働中（終了未検出）」を返し、呼び手スレッドをブロックしない。
2. When helper プロセスが終了する, the host-32 ホスト層 shall 終了を非ブロッキングに検出し、終了種別（正常終了・異常終了・強制終了）を区別して報告する。
3. When helper が終了コード 0 で終了する, the host-32 ホスト層 shall それを正常終了（`ExitKind::Clean`）として分類する。
4. When helper が非 0 の終了コードで終了する, the host-32 ホスト層 shall それを異常終了（`ExitKind::Abnormal`）として分類し、当該終了コードを保持する。
5. When helper が終了コードを持たない形で終了する（強制終了・シグナル相当）, the host-32 ホスト層 shall それを強制終了（`ExitKind::Terminated`）として分類する。
6. The host-32 ホスト層 shall 死活監視を既存の非ブロッキング死活 seam（`poll_exit`／`poll_exit_kind`）の上に常設化し、上流 `shiori-host32-ipc` の wire／timeout 機構を改変しない。

### Requirement 2: 死活・request 失敗の統一報告語彙

**Objective:** As a 下流の呼び手（kanade／ghost-setup）, I want helper 死活と request 失敗を単一の語彙で区別して受け取る, so that 「helper が死んだ・応答しないだけ・SHIORI がエラーを返した」を同じ報告面の上で判断できる

#### Acceptance Criteria

1. When helper 死亡に起因して request が失敗する, the host-32 ホスト層 shall その失敗を helper 死活（応答不能・強制終了・異常終了）として、SHIORI エラー応答やタイムアウトと区別できる形で報告する。
2. If request 往復が上限時間内に応答を返さない, then the host-32 ホスト層 shall それをタイムアウトとして helper 死活・SHIORI エラーと区別できる形で報告する。
3. If request が SHIORI エラー応答（400／500・`ErrorLevel`）を示す, then the host-32 ホスト層 shall それを SHIORI エラーとして helper 死活・タイムアウトと区別できる形で報告する。
4. The host-32 ホスト層 shall helper 死活・タイムアウト・SHIORI エラーを単一の不透明失敗へ潰さず、呼び手のリトライ／縮退／処分判断に足る区別を保った語彙で報告する。
5. When 監視が異常終了（`Abnormal`／`Terminated`）を検出した状態で request が試みられる, the host-32 ホスト層 shall それを helper 死活として観測可能なエラーで返し、無限待ち・ハングを起こさない。
6. The host-32 ホスト層 shall 死活報告に用いるデータをスレッド跨ぎで受け渡し可能な所有データ（`Send`）とし、将来の呼び手が非ブロッキングに参照できる形に切る。
7. The 本仕様 shall helper のプロセス処分・自動再起動・縮退の判断を報告語彙に含めず、検出と報告までに留める（処分判断は下流の領分）。

### Requirement 3: 周期運転（連打）耐性の決定的検証

**Objective:** As a host-32 トラックの開発者, I want 実 i686 helper へ周期 request を連打する決定的ハーネスで常駐運転の健全性を観測する, so that 毎秒 pump の運行がリーク・ハンドル枯渇・応答枠巻き込みなく成立することを CI 再現可能に裏付けられる

#### Acceptance Criteria

1. The 周期運転検証 shall 実 i686 helper プロセス越しに testdll fixture へ OnSecondChange 相当の頻度で request を反復連打する決定的ハーネスを備える。
2. When 周期 request が連打される, the 周期運転検証 shall 各往復が成功し、期待する応答（fixture 固定 response）を得ることを観測する。
3. While 周期運転検証が反復している間, the 周期運転検証 shall 連打に用いる request をイベント意味論を持たないダミー ID で送出し、特定イベント（OnSecondChange 等）の意味論に依存しない。
4. When 周期運転が完了する, the 周期運転検証 shall 往復後も helper が生存継続する（終了未検出）ことを観測する。
5. The 周期運転検証 shall リーク・ハンドル枯渇・`ResponseSlot`（応答枠）巻き込みが反復往復で発生しないことを、決定的に（実時間 sleep への依存を最小化して）検証する。
6. The 周期運転検証 shall 本物 pasta.dll を CI 必須ゲートとして要求せず、決定的検証を testdll fixture で成立させる。

### Requirement 4: helper 異常終了の検出・報告（強制 kill 注入）

**Objective:** As a host-32 トラックの開発者, I want 稼働中 helper を強制終了させた注入試験で監視の検出と上位報告を観測する, so that 実運転で helper が crash／強制終了した際に確実に検出され観測可能なエラーとして報告されることを保証できる

#### Acceptance Criteria

1. When 稼働中 helper が強制終了される, the host-32 ホスト層 shall 監視を通じて異常な終了種別（`Abnormal` または `Terminated`）を検出する。
2. When helper 強制終了後に request が試みられる, the host-32 ホスト層 shall それを helper 死活のエラーとして観測可能に返し、上限時間内に有限復帰する（無限待ち・ハングを起こさない）。
3. The 強制 kill 注入検証 shall 実 i686 helper プロセスに対して強制終了を注入し、監視の異常検出と request 経路の観測可能エラー報告を単一の決定的 run で観測する。
4. If helper 死活検出が失敗経路を通る, then the host-32 ホスト層 shall その失敗を握り潰さず、エラーログ（`error!`）＋戻り値の `Err` として surface する（silent failure を許さない）。

### Requirement 5: shutdown 全経路の決定性

**Objective:** As a host-32 ホスト層, I want 通常終了と異常後後始末の双方の shutdown 経路が決定的に通る, so that 常駐運転の終了系列が下流 `ghost-setup` に安全に引き渡せる

#### Acceptance Criteria

1. When 通常の shutdown が要求される, the host-32 ホスト層 shall helper プロセスを終了させ、終了種別が `ExitKind::Clean`（正常終了）であることを観測できる。
2. When helper が異常終了（`Abnormal`／`Terminated`）した後の後始末が行われる, the host-32 ホスト層 shall 稼働中 helper への参照（ハンドル）を安全に後始末し、既に終了しているプロセスへの終了要求でも失敗させない（冪等・二重 kill 安全）。
3. The host-32 ホスト層 shall 通常終了と異常後後始末の双方の shutdown 経路を決定的に検証する。
4. The 本仕様 shall shutdown 経路に自動再起動を含めず、終了と後始末までに留める（再起動判断は下流の領分）。
5. When shutdown 経路が失敗する, the host-32 ホスト層 shall その失敗を握り潰さず、エラーログ（`error!`）＋戻り値の `Err` として surface する。

### Requirement 6: env-gate 実 pasta 長時間追験

**Objective:** As a host-32 トラックの開発者, I want env 指定時に実 pasta.dll で周期運転の confidence 検証を追加実行する, so that 決定的 fixture 検証に加えて本物 SHIORI DLL でも常駐健全性を確認できる

#### Acceptance Criteria

1. Where env（例: `HOST32_PASTA_DLL`）で本物 pasta.dll が指定されている, the 周期運転検証 shall 実 pasta.dll に対して長時間相当の周期 request 連打を confidence 検証として実行する。
2. If 実 pasta.dll を指す env が設定されているのに指定 DLL が見つからない, then the テスト shall 明示的に失敗する（silent skip を認めない）。
3. Where 実 pasta.dll を指す env が未設定である, the 周期運転検証 shall 実 pasta 追験を skip し、決定的 fixture 検証を CI 必須ゲートとして成立させる。

### Requirement 7: 凍結境界・隔離規律・32bit ビルド規律・ログ規律の遵守（横断）

**Objective:** As a areka 開発者, I want 凍結境界・隔離規律・32bit ビルド規律・ログ規律が本仕様の全作業で守られる, so that 上流資産の安定性と検証の再現性が損なわれず、失敗が観測可能に保たれる

#### Acceptance Criteria

1. The 本仕様 shall `shiori-host32-ipc` の wire／framing／`MsgTag`／`ResponseSlot`／timeout の定義を改変しない（凍結境界）。
2. The 本仕様 shall `host32-request` の `Shiori3Client` 出口 API・`RequestError` 語彙・SHIORI/3.0 codec の意味論を変更せず、これらを消費するに留める。
3. The i686 成果物（helper・fixture）のビルドおよびテスト shall PowerShell 経由で実行し、`cargo test --target i686-pc-windows-msvc` を検証に含める。
4. The i686 側テスト shall helper／fixture 不在時にサイレントスキップせず、明示的に失敗する（先行ユニット踏襲）。
5. The 本仕様の検証 shall 実時間 sleep への依存を最小化した決定的テストとして構成し、有限復帰は凍結 transport の timeout（`SMTO_ABORTIFHUNG`）機構に乗る。
6. The 本仕様の失敗経路 shall 安易な panic を避け、失敗を `error!` ＋ `Err` 戻り値として surface する（panic は致命限定＋直前ログ・開発者指示 2026-07-04）。
7. The 本仕様 shall areka-actor へ結線せず（先行可）、死活報告データを `Send` な所有データ・非ブロッキングに切ることで将来の shiori アクター結線を阻害しない。
