# host-32 IPC transport（`areka-P0-host32-ipc`）

x64/arm64 親プロセスと i686 helper プロセスの間で **生バイト列を往復させる transport 層
（bytes-over-wire）** を提供するユニット。SHIORI/3.0 セマンティクス・`pasta.dll` ロード・
常駐 lifecycle は所有せず、下流ユニットの領分とする。

> このユニットの中核は 3 クレートに分割されている。本 README は
> **host-32 トラック共通のビルド／テスト規律**（特に i686 helper のビルド手順）を固定する。

## 責務境界（owns / does NOT own）

| | 内容 |
|---|---|
| **owns** | bytes-over-wire transport（WM_COPYDATA framing・HWND u32 LE 符号化・HELLO ハンドシェイク・再入 RESPONSE 受領によるデッドロック回避・`SendMessageTimeout(SMTO_ABORTIFHUNG)` による timeout/wedge 検出・helper spawn と非ブロッキング生存監視・pasta 非依存の echo 往復・i686 ビルドと 32bit 可搬性） |
| **does NOT own** | `pasta.dll` の `LoadLibraryW`/`GetProcAddress`/load・unload・request 解決（下流 `areka-P0-host32-shiori-load`）／SHIORI/3.0 の build・parse・charset 変換（下流 `-request`）／常駐メッセージループ・`OnSecondChange` ポーリング・unload・crash 監視 lifecycle（下流 `-lifecycle`）／`IShiori` ABI 実装本体（`shiori-abi` へは依存しない・下流で結線） |

この負の境界は要件 8.1〜8.4 の negative 基準であり、依存グラフ・grep で観測可能に保つ
（下記「責務境界の確認」）。

## 3 クレート構成（design discussion #1 で確定した Option B-2）

| クレート | ターゲット | 役割 |
|---|---|---|
| `shiori-host32-ipc` | x64 / arm64 / i686（全ターゲット可搬） | **proto**：`MsgTag`・framing・HWND u32 LE 符号化・`ResponseSlot`・`send_copydata`/`send_request`・`IpcError`。凍結する WM_COPYDATA ワイヤ規約の単一ソース。 |
| `shiori-host32-host` | x64 / arm64（ネイティブ） | **host**：`ProcessHost`（spawn/生存監視）・`ParentMessageWindow`（HELLO 記録・RESPONSE 再入受領・送信パス `send_request`）。 |
| `shiori-host32-helper` | i686 のみ | **helper**：`HelperMessageWindow`（HELLO 送出・REQUEST 受領・`respond` echo）。下流 `-shiori-load` が `respond` を pasta 駆動へ差し替える点。 |

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

往復 echo 統合テストは **実 i686 helper exe を spawn** するため、helper を先にビルドしてから
親テストを走らせる 2 段階になる。

```powershell
# ① i686 helper を先にビルド（往復 echo 統合テストが spawn する exe を用意）
cargo build -p shiori-host32-helper --target i686-pc-windows-msvc

# ② host 側テスト（単体 ＋ 統合）。統合テストは ① の helper exe を要する
cargo test -p shiori-host32-host

#    個別に走らせる場合:
cargo test -p shiori-host32-host --test echo_roundtrip   # 往復 echo（要件 6.x のゲート指標）
cargo test -p shiori-host32-host --test error_paths      # エラー経路（要件 1.4/2.5/3.4/5.x）

# ③ proto の 32bit 可搬性テスト（i686 でも単体テストが通ることの確認・要件 7.2/7.3）
cargo test -p shiori-host32-ipc --target i686-pc-windows-msvc
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

いずれも依存は `shiori-host32-ipc` / `wintf-winmsg-executor` / `event-listener`（host のみ）/
`windows` / `windows-core` / `thiserror` に限られ、`shiori-abi`・pasta・pilot は現れない。
また production コード（各クレートの `src/`）には `LoadLibraryW`/`GetProcAddress`・SHIORI3 の
build/parse/charset 変換・常駐 lifecycle（`OnSecondChange` ポーリング/unload/crash 監視）を
一切含まない（echo 往復に必要な最小のメッセージループのみ）。
