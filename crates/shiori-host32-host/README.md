# host-32 IPC transport（`areka-P0-host32-ipc`）

x64/arm64 親プロセスから 32bit（i686）の SHIORI DLL を使うための、親と i686 helper プロセスの
組み。WM_COPYDATA の transport に加え、SHIORI DLL のロード（helper）・SHIORI/3.0 と 2.x の
組み立てと解析・文字コードの交渉（host）・helper の寿命（正常終了・強制終了・孤児化防止）を持つ。

> このユニットの中核は 3 クレートに分割されている。本 README は
> **host-32 トラック共通のビルド／テスト規律**（特に i686 helper のビルド手順）を固定する。

## 責務境界（owns / does NOT own）

| | 内容 |
|---|---|
| **owns** | transport（WM_COPYDATA framing・HWND u32 LE 符号化・HELLO ハンドシェイク・再入 RESPONSE 受領によるデッドロック回避・`SendMessageTimeout` による timeout/wedge 検出）／helper の spawn・非ブロッキング生存監視・正常終了（UNLOAD）・強制終了・Job Object による孤児化防止・標準エラーの記録への転送（host）／SHIORI DLL の `LoadLibraryExW`（`LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR \| LOAD_LIBRARY_SEARCH_DEFAULT_DIRS`）・`GetProcAddress`・`loadu`/`load`・`unload`・`request` の呼び出し（helper）／SHIORI/3.0 と 2.x の build・parse・charset の交渉と変換（host の `shiori3`・`shiori2`・`charset`・`client`） |
| **does NOT own** | `IShiori` ABI の実装本体と、ゴーストの起動への結線（`areka-ghost`）／常駐の進行（`OnSecondChange` などのイベントの送り方・処分の判断）は使い手（`areka-ghost`・`areka-kanade`・`areka`）の領分／`shiori-abi` へは依存しない |

この負の境界は要件 8.1〜8.4 の negative 基準であり、依存グラフ・grep で観測可能に保つ
（下記「責務境界の確認」）。

## 3 クレート構成（design discussion #1 で確定した Option B-2）

| クレート | ターゲット | 役割 |
|---|---|---|
| `shiori-host32-ipc` | x64 / arm64 / i686（全ターゲット可搬） | **proto**：`MsgTag`・framing・HWND u32 LE 符号化・`ResponseSlot`・`send_copydata`/`send_request`・`IpcError`。凍結する WM_COPYDATA ワイヤ規約の単一ソース。 |
| `shiori-host32-host` | x64 / arm64（ネイティブ） | **host**：`ProcessHost`（spawn/生存監視）・`ParentMessageWindow`（HELLO 記録・RESPONSE 再入受領・送信パス `send_request`）・`HelperLifecycle`（正常終了・強制終了）・`Shiori3Client`（SHIORI/3.0 と 2.x の往復・文字コードの交渉）。 |
| `shiori-host32-helper` | i686 のみ | **helper**：`HelperMessageWindow`（HELLO 送出・LOAD／REQUEST／UNLOAD 受領）・`ShioriByteProxy`（SHIORI DLL のロードと呼び出し）。 |

`-host` / `-helper` は `-ipc` を **一方向依存**する。host↔helper 間のコード依存は無く、
プロセス境界で WM_COPYDATA のみが跨ぐ。ターゲット分離は `cfg` でなく crate 境界で担保される。

## ビルド／テストは PowerShell 必須（Git Bash 不可）

i686 ターゲットのビルドは **必ず PowerShell** で行う。Git Bash では GNU coreutils の
`link.exe` が MSVC の `link.exe` を PATH で遮蔽し、最終リンク段で `'\377\376'` エラー
（UTF-16 BOM 由来）になる。x64 でも本トラックは PowerShell に統一する。

### 前提（一度だけ）

```powershell
# i686 ターゲットの導入（未導入なら）
rustup target add i686-pc-windows-msvc
```

加えて VS2022 の MSVC ツールが必要。arm64 上でクロスビルドする場合は
`Microsoft.VisualStudio.Component.VC.Tools.ARM64` も要る（無いと最終リンクのみ落ちる）。
i686 は x86 の MSVC ツールセットで足りる。

### 手順（コピペ可）

ワークスペース全体を回すなら `pwsh -NoProfile -File tools/test-all.ps1` が下の ① ③ と i686 の導入をまとめて行う（kiro-complete の Test Gate の正本）。以下は host-32 だけを個別に回す手順。

統合 E2E テスト（`shiori_load_e2e` / `shiori_request_e2e`）は **実 i686 helper exe を spawn** し
**実 i686 testdll（`shiori.dll`）を load** するため、helper と testdll を先にビルドしてから
親テストを走らせる 2 段階になる。

```powershell
# ① i686 helper と testdll を先にビルド（統合テストが spawn/load する成果物を用意）
cargo build -p shiori-host32-helper  --target i686-pc-windows-msvc
cargo build -p shiori-host32-testdll --target i686-pc-windows-msvc
cargo build -p shiori-host32-testdll-loadu --target i686-pc-windows-msvc

# ② host 側テスト（単体 ＋ 統合）。統合テストは ① の成果物を要する
cargo test -p shiori-host32-host

#    個別に走らせる場合:
cargo test -p shiori-host32-host --test shiori_load_e2e     # LOAD 往復（proxy 確立 ack）
cargo test -p shiori-host32-host --test shiori_request_e2e  # request 往復（GET Value / NOTIFY 204）
cargo test -p shiori-host32-host --test error_paths         # エラー経路（要件 2.5/3.4/5.x）

# ③ i686 でしか走らないテスト（ipc の 32bit 可搬性・要件 7.2/7.3 に加え、helper の proxy・
#    loopback・再入・固まった窓のテスト。合わせて約 42 秒）
cargo test -p shiori-host32-helper -p shiori-host32-ipc --target i686-pc-windows-msvc
```

統合テストは helper exe を
`target/i686-pc-windows-msvc/{debug,release}/shiori-host32-helper.exe` から自動探索する。
別の場所に置いた場合は env `HOST32_HELPER_EXE` で exe パスを明示できる。helper 未ビルドの
場合はテストが「先に i686 helper をビルドせよ」という明確な panic で fail する
（無言スキップで緑を偽装しない）。

## 責務境界の確認（要件 8.1〜8.4 の negative 基準）

依存グラフに **`shiori-abi`／pasta 系／pilot** が現れないことを確認する。

```powershell
cargo tree -p shiori-host32-ipc
cargo tree -p shiori-host32-host
cargo tree -p shiori-host32-helper --target i686-pc-windows-msvc
```

直接の依存は次のとおりで、`shiori-abi`・pasta・pilot は現れない。

- `shiori-host32-ipc`: `windows` / `thiserror`
- `shiori-host32-host`: `shiori-host32-ipc` / `encoding_rs` / `wintf-winmsg-executor` / `thiserror` / `tracing` / `windows`
- `shiori-host32-helper`: `shiori-host32-ipc` / `wintf-winmsg-executor` / `windows`

SHIORI DLL を読むのは helper（`LoadLibraryExW`）だけで、host は DLL を読まない。
