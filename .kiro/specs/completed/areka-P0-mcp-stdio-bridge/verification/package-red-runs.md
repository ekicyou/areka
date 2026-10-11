# 配布スクリプトに中継を足した確かめの記録（タスク 6.3）

- 日付: 2026-10-10
- 机: 開発者の机（x64）。HEAD は `dbac922e`、作業ツリーの変更は `tools/package.ps1` の 1 本だけ（スクリプトの印字は `uncommitted changes 1`）。
- スクリプトの版: `tools/package.ps1` 2.1.0（zip の `BUILD-INFO.txt` に `script=tools/package.ps1 2.1.0`）。
- 直したスクリプトの SHA256: `C7A22A1AB0E7040B89061D7B3480E9BD514A6385425790AE0142649E855C05B9`
- 回した順: 緑（`-Arch all`）→ 赤 1 → 赤 2 → 緑（`-Check`）。重い実行はこの 4 回だけ。

## 赤の確かめ（2 回）

どちらも、直したスクリプトの写しを `target\package-6.3\package.ps1.orig` に取り、1 か所だけ一時的に変えて `pwsh -NoProfile -File tools/package.ps1`（x64）を回し、写しから戻した。戻した後の SHA256 は 2 回とも上の値と同じ。

### 赤 1: 組み立ての写しの行を外す

- 変えたこと: 段 `x64 assemble` の、`areka-mcp-bridge.exe` を写す `Copy-Item` の 1 行を外した（ほかは同じ）。
- 終了コード: 1（段 `x64 content check` で止まった）。
- 印字された判定の行:

  ```text
  FAIL 1 required entry missing: areka-mcp-bridge.exe
  FAIL 4 executable missing: areka-mcp-bridge.exe
  FAIL x64 content check (0 s, content check: 2 failure(s))
  ```

- 後始末: `target\package\` に x64 の zip・`.sha256`・仮の名前の物は残らなかった。

### 赤 2: 写す元を 32 ビットの補助 exe に替える

- 変えたこと: 同じ `Copy-Item` の写す元を補助 exe（`target/package/i686-pc-windows-msvc/release/shiori-host32-helper.exe`）に替え、写す先の名前は `areka-mcp-bridge.exe` のままにした。
- 終了コード: 1（段 `x64 content check` で止まった）。
- 印字された判定の行:

  ```text
  areka-mcp-bridge.exe: machine 0x014c, imports user32.dll, kernel32.dll, api-ms-win-core-synch-l1-2-0.dll, ntdll.dll, oleaut32.dll, KERNEL32.dll
  FAIL 5 wrong machine: areka-mcp-bridge.exe is 0x014c (expected 0x8664)
  FAIL x64 content check (0 s, content check: 1 failure(s))
  ```

- 後始末: 赤 1 と同じく、x64 の zip・`.sha256`・仮の名前の物は残らなかった。

## 緑の確かめ

### `-Arch all`（x64 と arm64 の zip）

終了コード 0。全段が緑（`all steps green`）。段 `check static linking` の中継の行:

```text
target/package/x86_64-pc-windows-msvc/release/areka-mcp-bridge.exe: machine 0x8664 (expected 0x8664), imports kernel32.dll, ntdll.dll, api-ms-win-core-synch-l1-2-0.dll, bcryptprimitives.dll, ws2_32.dll, KERNEL32.dll
target/package/aarch64-pc-windows-msvc/release/areka-mcp-bridge.exe: machine 0xaa64 (expected 0xaa64), imports kernel32.dll, ntdll.dll, api-ms-win-core-synch-l1-2-0.dll, bcryptprimitives.dll, ws2_32.dll, KERNEL32.dll
```

中継が取り込む DLL は OS の物だけで、拒否表（`DLL_DENY_PREFIXES`＝VC++ のランタイム）に当たる名前は無い（要件 7.3）。

zip の最上位の実行ファイル（名前・バイト数）。中継は `areka.exe` と同じ階層にある。

| zip | `areka.exe` | `areka-mcp-bridge.exe` | `shiori-host32-helper.exe` |
|---|---|---|---|
| `areka-0.0.2-x64.zip`（10,100,394 バイト・項目 150） | 9,577,984 | 3,252,224 | 219,648 |
| `areka-0.0.2-arm64.zip`（9,900,924 バイト・項目 150） | 8,517,632 | 2,893,312 | 219,648 |

段 `content check` はどちらの zip も `content checks 1-8 all passed`。中継の行は x64 が `machine 0x8664`、arm64 が `machine 0xaa64`。

### `-Check`（x64 の zip を展開して起動）

`pwsh -NoProfile -File tools/package.ps1 -Check` は終了コード 0。展開先は `target\package\check-205638`（判定の後に消えた）、記録は `target\package\check-205638-logs`。

```text
content checks 1-8 all passed
PASS not killed by watchdog (exited by itself)
PASS ran bounded (LOG_MARKER_SMOKE_GATE: 1 line(s))
PASS exit code 0 (exit code 0)
PASS ghost window opened (LOG_MARKER_WINDOWS: 1 line(s))
PASS no SHIORI connection failure (fault markers: 0 lines)
PASS talk started (LOG_MARKER_GREETING: 1 line(s) before auto exit, 1 in total)
PASS first balloon is bundled (route=Companion dir=...\target\package\check-205638\balloon\emo2-kakukaku)
OK   git status unchanged (0 s)
all steps green
```

`-Check` は中継を起動しない（設計のとおり・中継が起動できることは実機確認で見る）。展開した zip は上の中身の判定を通った物なので、起動した `areka.exe` の隣に `areka-mcp-bridge.exe` が在る。

## arm64

この机には arm64 のターゲット（`aarch64-pc-windows-msvc`）と Visual Studio の ARM64 の道具が有るので、`-Arch all` で arm64 の zip も作って中身の判定まで通した（上の表）。arm64 の機械での起動はこの机では確かめられない。CI（`release.yml` の「zip を作る」）の結果は完了のときに記録する。

## CI の確かめ

`.github/workflows/release.yml` は `tools/package.ps1 -Arch all` を呼び、その後の「4 つの確かめ」は zip 2 本と `.sha256` 2 本の有無と SHA256 だけを見る。zip の中身や実行ファイルの名前の一覧は持っていないので、ワークフローは直していない。
