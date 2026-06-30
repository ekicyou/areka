# Implementation Plan

> 種別: **先進坑（pilot・使い捨て）**。成果物はコードでなく知見（go／違う／直す ＋ 学び）。一次記録は `crates/pilot/examples/shiori-host-32/README.md`（3 幕）。
> 実装は design.md のコンポーネント/契約・research.md の確定決定に従う。**葉ノード隔離（命綱）厳守**・使い捨て品質可。ビルドは PowerShell（i686 リンカトラップ回避）。

- [ ] 1. Foundation: 先進坑スキャフォールド＋2 段ビルド基盤＋IPC 契約
- [x] 1.1 example フォルダ生成・helper 独立ターゲット宣言・親/helper 最小スケルトン
  - `_template` を `examples/shiori-host-32/` へコピーし、親 `main.rs`（x64）と helper `helper.rs`（i686）の最小スケルトンを置く
  - `pilot/Cargo.toml` に helper 用 `[[example]]`（name=`shiori-host-32-helper`, path=`examples/shiori-host-32/helper.rs`）を追加
  - 探索コードは `examples/shiori-host-32/` のみに置き、production クレートへの inbound 依存を一切作らない（葉ノード隔離・空 lib＋examples-only を崩さない）
  - 観測: `cargo run -p pilot --example shiori-host-32`（x64）と `cargo build -p pilot --example shiori-host-32-helper --target i686-pc-windows-msvc`（i686・PowerShell）が**両方ビルド成功**する
  - _Requirements: 1.5, 7.1, 7.2, 7.3, 7.4, 7.5_
- [x] 1.2 WM_COPYDATA IPC 契約モジュール（親/helper 共有）
  - メッセージ種別タグ（HELLO/LOAD/REQUEST/RESPONSE/UNLOAD）を `dwData` の**低 32bit** に載せる規約、ペイロード＝生バイト列・長さは `cbData`、HWND は **u32 LE** 表現で受け渡し（跨ビットネス安全・ポインタ/HANDLE/struct を載せない）
  - `SendMessageTimeout`（タイムアウト＋`SMTO_ABORTIFHUNG`）を IPC 送出の基本規約として定義
  - 観測: 親/helper 双方から import できる共有プロトコルが定義され、タグが低 32bit に収まる単体テストが通る
  - _Requirements: 2.1, 2.3_
  - _Boundary: IpcChannel_

- [ ] 2. Core: x64 親側コンポーネント
- [x] 2.1 (P) SHIORI/3.0 OnBoot 組立と Value 抽出（x64）
  - `OnBoot` を `GET SHIORI/3.0`＋必須ヘッダ（`ID: OnBoot`／`Charset: UTF-8`／`Sender`）CRLF＋空行終端で **UTF-8** 生成（GET ＝応答を返す経路・NOTIFY でない）
  - 応答バイト列から `Value:`（さくらスクリプト本体）を UTF-8 で抽出、不在で None
  - 観測: 生成バイト列が SHIORI/3.0 GET 形式になり、`Value:` 入り応答から本体を抽出できる（単体テストで確認）
  - _Requirements: 4.1, 4.2, 4.4_
  - _Boundary: Shiori3Codec_
- [x] 2.2 (P) helper プロセスの起動と生存監視（x64）
  - `HELPER_EXE`（無ければ第 1 引数）で helper exe パスを受け、ghostdir と親 HWND を引数/環境で渡して起動
  - 子プロセスハンドルで終了コード/生死を IPC レイヤと直交に監視し、clean / 異常終了を観測可能な形で記録
  - 観測: 親が helper exe を起動し、その終了コードを取得できる（helper スケルトンの起動→終了で確認）
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.4_
  - _Boundary: ProcessHost_
  - _Depends: 1.2_

- [ ] 3. Core: i686 helper 側コンポーネント
- [ ] 3.1 (P) pasta.dll 動的ロードとバイト proxy（i686）
  - `LoadLibraryW`＋`GetProcAddress` で `load`/`unload`/`request`（cdecl flat-C・返り値 `bool`(1byte)・pasta 実ソース確定）を解決し関数ポインタ化
  - `load(ghostdir)` は ghostdir を **ANSI(Shift_JIS)** で HGLOBAL 化して呼びクラッシュせず完了。`request` は受信バイト列を HGLOBAL 化して渡し応答 HGLOBAL からバイト取得（**入力 HGLOBAL は DLL 解放／応答 HGLOBAL はホスト解放**・HGLOBAL は IPC を跨がない）
  - ロード/解決失敗は観測可能な形で返す。SHIORI3 ロジックは持たない（バイト proxy に徹する）
  - 観測: i686 helper 内で pasta.dll をロードし 3 エントリを解決、`load(ghostdir=fixtures/emo2/ghost/master/)` がクラッシュせず完了する
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 4.2_
  - _Boundary: ShioriByteProxy_
  - _Depends: 1.1_
- [ ] 3.2 helper メッセージ窓とループ（i686）
  - `wintf-winmsg-executor` で message-only 窓を生成しメッセージループを回す。起動時に親へ HELLO（自 HWND を u32 LE）を 1st WM_COPYDATA で送る
  - WndProc で REQUEST を受領しバイト proxy を駆動、応答を 2nd WM_COPYDATA で親へ返す。UNLOAD でループ停止 → clean unload
  - 観測: helper 起動で窓生成＋親へ HELLO 送出、N 秒間ループが破綻せず回り続ける
  - _Requirements: 5.1, 5.2, 5.3_
  - _Boundary: HelperMessageWindow_
  - _Depends: 1.2, 3.1_

- [ ] 4. Integration: 親⇄helper 1 往復結線（go 基準 1）
- [ ] 4.1 全体駆動・HWND ハンドシェイク・受け皿セル再入受領・OnBoot 往復
  - 親を駆動: helper 起動 → 親メッセージ窓生成 → HELLO で HWND ハンドシェイク → OnBoot 組立 → REQUEST 送出（`SendMessageTimeout`）
  - **受け皿セル方式**で RESPONSE を再入受領（応答 WndProc は非ブロッキング・両方向 Timeout・single-in-flight ＝**循環待ちなしのデッドロック回避**）→ `Value:` を parse → 標準出力
  - 観測: 親が emo2 の OnBoot 応答 `Value:`（起動挨拶さくらスクリプト）を受領し標準出力に表示する＝**go 基準(1) 充足**
  - _Requirements: 2.1, 2.2, 2.3, 4.3, 4.5_
  - _Boundary: ParentDriver, IpcChannel_
  - _Depends: 2.1, 2.2, 3.1, 3.2_

- [ ] 5. Integration/Validation: go 基準 2＋異常系
- [ ] 5.1 メッセージループ N 秒生存 → clean unload の結線・観測
  - ループを N 秒運転後、親が UNLOAD を送り helper が `unload`→`FreeLibrary`→終了コード 0 で正常終了
  - 観測: helper が N 秒生存後 clean unload し、**終了コード 0 を親が観測**する＝**go 基準(2) 充足**
  - _Requirements: 1.3, 5.2, 5.4_
  - _Depends: 4.1_
- [ ] 5.2 異常系: IPC タイムアウトと helper 異常終了検出
  - 無応答 helper に対し `SendMessageTimeout` が所定時間で Timeout を返しハングしない。helper 強制終了を親が終了コードで検出
  - 観測: 無応答時に Timeout、強制終了時に異常検出が観測でき、**いずれもハングしない**
  - _Requirements: 1.4, 2.3, 2.4_
  - _Depends: 5.1_

- [ ] 6. Validation: 実走 go 検証と README 一次記録
- [ ] 6.1 実 pasta.dll での go 基準実走検証
  - emo2 fixture に対し full pilot を実走し、go 基準(1) Value 受領と go 基準(2) ループ生存 → clean unload を観測。実行時挙動（`request` の block-on-reply・`load` 起点 `spawn_actor` スレッド）も観測・記録
  - 観測: 実 pasta.dll で go 基準(1)(2) の充足/不充足が観測され、結果（数値・ログ）が記録される
  - _Requirements: 4.5, 5.4, 6.4_
  - _Depends: 4.1, 5.1_
- [ ] 6.2 README 3 幕 一次記録
  - `_template/README.md` をコピーし、動機（本坑 `areka-P0-host32-*` 群を名指し）→ 概要・**実行法（必須 3 項目: i686 ビルドコマンド／生成 exe パス／`HELPER_EXE` での渡し方）**→ 検証結果（go/違う/直す ＋ 学び ＋ 日付）を記述。go 判定は人間判断に委ね、判断材料の提供に徹する
  - 観測: README に 3 幕が揃い、実行法 3 項目と go 基準充足状況（判定材料）が記載される
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_
  - _Depends: 6.1_
- [ ]* 6.3 補助単体テスト（任意・MVP 後でも可）
  - `Shiori3Codec` の OnBoot 組立／`Value:` 抽出、IPC タグの低 32bit 境界の単体テスト
  - 観測: build_onboot/parse_value・タグ境界の単体テストが通る（要件 4.1/4.2/2.1 の受入確認）
  - _Requirements: 2.1, 4.1, 4.2_

## Implementation Notes

- 1.2: `windows` 0.62.2 では `COPYDATASTRUCT` は `Win32_System_DataExchange` feature 配下（`Win32_UI_WindowsAndMessaging` でない）。pilot crate の `windows` 依存に crate-scoped で feature 追加が必要。後続で WM 系の windows 型を使うタスクは feature 不足の E0432 に注意。
- 共有モジュールは各バイナリで `#[path = "ipc.rs"] mod ipc;`（main.rs/helper.rs 双方）。同方式を shiori3.rs 等の他共有モジュールにも適用予定。
