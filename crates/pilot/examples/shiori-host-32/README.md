# 先進坑: shiori-host-32（host-32 実現可能性・一点突破検証）

> この README は本先進坑の**一次記録（正本）**である。本坑 spec の design はここの検証結果を参照し、同じ結果を二重化しない。
> 種別: **先進坑（pilot・使い捨て）**。成果物はコードでなく知見（go／違う／直す ＋ 学び）。葉ノード隔離（命綱）のみ厳守。

## 動機（なぜ掘るか）

- 対応する本坑 spec: **`areka-P0-host32-*` 群**（この先進坑が gate する被依存先）
  - `areka-P0-host32-ipc`（x64↔32bit helper ＋ handshake/lifecycle）
  - `areka-P0-host32-shiori-load`（`LoadLibrary` ＋ `load`/`unload`/`request` 解決）
  - `areka-P0-host32-request`（SHIORI/3.0 build ＋ marshal ＋ Value ＋ charset）
  - `areka-P0-host32-lifecycle`（msg loop ＋ unload ＋ crash 監視）
- 確認したい方向 / 実現可能性:
  - これは **M1 の唯一の耐力壁**である。「**x64 areka が emo2 の 32bit `pasta.dll`（PE Machine 0x014C）を駆動できるか**」が確証されない限り、本坑 host-32 トラックへの着手は「見てから違う」の最悪パターン（コード肥大後に方向誤りが発覚 → やり直し地獄）に落ちる。
  - x64 プロセスへ 32bit DLL を in-proc ロードするのは不可能。ゆえに SHIORI を 32bit 別プロセス（helper）でホストし、自前 IPC で x64 親と橋渡しする機構（host-32）の**実現可能性のみ**を、使い捨ての最小探索コードで一点突破検証する。

## 概要（何を作ったか）

2 プロセス・single-in-flight 同期 request/response の host-32 ブリッジ最小実装。トランスポートは **Window Message（WM_COPYDATA）一本化**。

- **x64 親（`main.rs`）**: ParentDriver。helper 起動 → 親 message-only 窓生成 → HELLO で HWND ハンドシェイク → OnBoot 組立 → REQUEST 送出 → **受け皿セル再入受領** → `Value:` parse → 標準出力 → N 秒生存監視 → UNLOAD → clean unload 観測。
- **i686 helper（`helper.rs`）**: HelperMessageWindow ＋ ShioriByteProxy。`wintf-winmsg-executor` の message-only 窓＋ループ。起動時に親へ HELLO、WndProc で REQUEST 受領 → バイト proxy 駆動 → 2nd WM_COPYDATA で応答 → UNLOAD で clean unload。
- **共有規約（`ipc.rs`）**: `MsgTag`（`dwData` 低 32bit）・ペイロード＝生バイト列（`cbData`=長さ）・HWND は u32 LE・`SendMessageTimeout`（`SMTO_ABORTIFHUNG`）。
- **バイト proxy（`shiori_proxy.rs`, i686）**: `LoadLibraryW`＋`GetProcAddress` で `pasta.dll` の flat-C `load`/`unload`/`request` を解決し、HGLOBAL 所有権規約・charset 非対称を守ってバイト列を運ぶ（SHIORI3 ロジックは持たない）。
- **SHIORI3 codec（`shiori3.rs`, x64）**: `build_onboot` / `parse_value`（UTF-8）。x64 過去互換 `IShiori` アダプタのミニチュア。
- 検証フィクスチャ: `fixtures/emo2/`（リポジトリ取り込み済）。ghostdir = `fixtures/emo2/ghost/master/`。

### 実行法（再現手順・必須 3 項目）

ビルドは **PowerShell** で行うこと（Git Bash の GNU `link.exe` が MSVC link を遮蔽する既知トラップ）。fixture 取り込み済ゆえ nar 展開は不要。

1. **helper の i686 ビルド**:
   ```powershell
   cargo build -p pilot --example shiori-host-32-helper --target i686-pc-windows-msvc
   ```
2. **生成 exe パス**:
   ```
   target\i686-pc-windows-msvc\debug\examples\shiori-host-32-helper.exe
   ```
3. **親起動（`HELPER_EXE` で helper exe を渡す）**:
   ```powershell
   $env:HELPER_EXE = "target\i686-pc-windows-msvc\debug\examples\shiori-host-32-helper.exe"
   cargo run -p pilot --example shiori-host-32
   ```
   異常系（IPC タイムアウト・helper 強制終了検出）の観測:
   ```powershell
   cargo run -p pilot --example shiori-host-32 -- --selftest-errors
   ```

> 注（環境 flake）: `cargo run`/`cargo test` が稀に rustup shim エラー `rustc.exe … not applicable to stable` を出すことがある（pilot でなく環境）。回避＝`cargo +stable …` か生成 exe 直実行（`$env:HELPER_EXE` を設定のうえ `target\debug\examples\shiori-host-32.exe`）。

## 検証結果

- **判定材料: go 基準(1)(2) とも充足**（最終 go 判定は開発者の人間判断に委ねる・要件 6.5）。検証日: **2026-07-01**。

### go 基準(1): load → OnBoot → Value 受領 → unload の 1 往復（要件 4.5）
**充足。** x64 親が i686 helper 越しに実 emo2 `pasta.dll` の OnBoot 応答 `Value:`（起動挨拶さくらスクリプト）を受領・標準出力に表示。
- Value 受領 293–543 バイト・RESPONSE 376–626 バイト（**実行毎に内容が変化＝live pasta 駆動の証拠**）。
- REQUEST→RESPONSE 同期往復 **3–6 ms**（block-on-reply）。

### go 基準(2): メッセージループ N 秒生存 → clean unload（要件 5.2/5.4）
**充足。** helper の `wintf-winmsg-executor` ループが N=4.0s 破綻なく生存（親が 250ms 毎に liveness poll、16 回すべて alive）、生存後の追加 REQUEST にも応答、UNLOAD → `unload` → `FreeLibrary` → **終了コード 0（kind=Clean）を親が観測**。

### 異常系（要件 1.4/2.3/2.4）
**充足。** いずれもハングしない: 無応答 helper に対し `SendMessageTimeout` が所定 500ms で Timeout を返す（wedge 3s に対し境界内）。helper 強制終了を親が終了コード（`Abnormal(1)`）で ~10ms で検出。

### 学び（本坑をクリーンに掘り直すための材料・コピペ donor にはしない）

1. **跨ビットネス再入 WM_COPYDATA 配送 GO（設計最大の賭け成立・design §210）**: x64 親が `SendMessageTimeout` でブロック中、i686 helper の RESPONSE(2nd WM_COPYDATA) が親 WndProc へ**再入配送**され受け皿セル（`ResponseSlot`）へ格納 → 復帰で取得できた。**named pipe 後退（Revalidation Trigger）は不要**。デッドロック回避＝応答 WndProc は payload 格納後**即 return**（それ以上跨プロセス SendMessage しない）・両方向 `SMTO_ABORTIFHUNG`＋timeout・single-in-flight ＝循環待ちなし。
2. **`wintf-winmsg-executor` 0.0.5 の i686 実行時 GO（design §443 Risk 撤回）**: message-only 窓生成・`MessageLoop::run`・WndProc dispatch・WM_COPYDATA 配送すべて i686 で機能。**raw Win32 ループ後退は不要**＝本坑 helper もこの版を流用可。
3. **i686 ビルドは PowerShell 必須**（Git Bash link.exe トラップ・[[arm64-windows-build]] と同根）。i686 では `usize`=32bit ゆえ `(x as usize) >> 32` が overflow lint でコンパイルエラー → dwData/ULONG_PTR 演算は `u64` で評価。共有モジュールは i686 でも `cargo test --target i686-pc-windows-msvc` を回すこと。
4. **pasta flat-C ABI は実ソースでバイト正確確認**（`vendors/pasta/crates/pasta_shiori`）: `load(HGLOBAL,usize)->bool` / `unload()->bool` / `request(HGLOBAL,*mut usize)->HGLOBAL`。返り値は Rust `bool`(1byte)（Win32 BOOL でない）。**HGLOBAL 所有権**＝入力は DLL 解放・request 返り値はホスト解放。`request` の `len` は **in/out**（入力長を先に書く）。**charset 非対称**＝`load` の dir は ANSI(CP_ACP/Shift_JIS)・`request` は UTF-8。HGLOBAL=`GlobalAlloc(GMEM_FIXED)` 生ポインタ（GlobalLock 不要）で IPC を跨がない。Shift_JIS は windows crate の CP_ACP（`WideCharToMultiByte`）で足り、`encoding_rs` 不要。
5. **確認できた実行時挙動は `request` の block-on-reply（3–6ms 同期往復）のみ**。design §495 が仮説化した `load`→`spawn_actor` 内部スレッドは**利用可能ソースで未確認**（`vendors/pasta` に該当シンボル皆無・`pasta_shiori/src/shiori.rs` は single-threaded と明記／かつ実ロードは prebuilt emo2 `pasta.dll` でその内部は vendored source から検証不能）。actor スレッドは**挙動証拠**（go(1)＋go(2)生存後の 2 応答 → clean unload ＝リクエスト処理ループが生存窓を跨いで稼働）に格下げ。本坑では実バイナリの内部前提を置かず、観測可能な契約（block-on-reply・clean unload）だけに依存すること。
6. **跨ぐのは生バイト列のみ**。HGLOBAL は 32bit ローカル・HSTRING は x64 ローカル（どちらも IPC を跨がない）。HWND は USER ハンドルゆえ 32bit 有意（u32 LE で受け渡し）。`COPYDATASTRUCT` は windows 0.62.2 では `Win32_System_DataExchange` feature 配下。
7. **命綱（葉ノード隔離）維持**: 探索コードは `examples/shiori-host-32/` のみ・production クレートへの inbound 依存ゼロ・helper は明示 `[[example]]` で独立 i686 ターゲット化。本坑は go 判定後に、この知見を見て一から綺麗に掘り直す（コピペ donor 流用禁止）。
