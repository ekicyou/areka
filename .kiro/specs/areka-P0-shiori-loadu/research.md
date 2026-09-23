# Gap Analysis: areka-P0-shiori-loadu

> 2026-09-23・本ブランチ（`claude/kiro-start-areka-p0-shiori-d1a08b`・HEAD `fe6ead48`）で実測。requirements.md は確定済み（本文書は変更しない）。コードは「何の定義か」（関数名・型名＋ファイルパス）で指す。
> 目的は実装方針の材料を揃えることで、最終決定はしない。裁定が要る点は末尾「設計判断の議題」に番号で並べた。

## 1. 分析の要約

- **足す場所は 1 ファイル 2 関数に閉じる。** 入口の解決は `crates/shiori-host32-helper/src/shiori_proxy.rs` の `ShioriByteProxy::load` の中の `resolve` クロージャだけ、パスの符号化は同ファイルの `fn ansi_encode` だけ。呼ぶ側 `main.rs` の `TriggerLoad` の枝は `ShioriByteProxy::load(&dll_path, &s.load_dir)` を 1 回呼ぶだけで、親との受け渡し（`MsgTag`・ack 1 バイト）には触れずに済む。
- **既存の作法がそのまま使える。** 純関数を x64 で常時、DLL を読むテストは `#[cfg_attr(not(target_arch = "x86"), ignore = "…")]` で i686 限定、fixture は `target/i686-pc-windows-msvc/{debug,release}/<名>.dll` を探して無ければ明確に panic、env の注入は `HOST32_TESTDLL_` 接頭辞、テスト間の env 競合は `TESTDLL_SERIAL` の Mutex で直列化——全部 `shiori_proxy.rs` の `mod tests` と `main_loopback_tests.rs` に実物がある。
- **検体の実測が brief と 1 件食い違う。** `dumpbin /exports` で取り直したところ、emo2 の `pasta.dll` は **`loadu` を公開している**（brief と requirements の表は「なし」）。実装後は emo2 も `loadu` の枝を踏み、`load` の枝を実機で踏む検体は里々だけになる。要件 7.4 が「一致しなければ 7.1〜7.2 の期待値を改める」と先に書いているので手順どおりに改めればよいが、**pasta 側の `loadu` の実装は手元のソース履歴に無い**（下記 §4.2）ため、emo2 の実機確認が「動くかどうか分からない枝」を初めて踏む場になる。
- **文書の要件 8.3 は現物と合わない。** `doc/ukadoc-coverage/briefing-shiori.md` は `ukadoc-survey` の生成物ではなく **人が書く正本**で、台帳冒頭の `#` 注釈がその写し（完了 spec `areka-P0-ukadoc-survey-shiori` の tasks.md 1.4 の記録）。生成器が作り直すのは `report/shiori.md`・`report/summary.md` だけで、`spec_dll` の `status` は `degraded` のままなので数字は変わらない。
- **表せない字の検出には 1 つ罠がある。** `WideCharToMultiByte` の `lpUsedDefaultChar` は、既定コードページが UTF-8（Windows の「ベータ: 世界言語対応で Unicode UTF-8 を使用」を有効にした機械では `GetACP()` が 65001）だと **関数自体が失敗**する。素直に引数を 1 つ足すと、その機械では `load` の枝が全部 `EncodingFailed` になる。往復比較（`MultiByteToWideChar` で戻して元と比べる）なら、どのコードページでも同じ手順で判定できる。

規模 **S**（brief の XS〜S を維持）・リスク **中の下**（新技術 0・依存 0。不確かなのは pasta の `loadu` の実装と UTF-8 既定機の 2 点だけ）。

## 2. 現状の調査

### 2.1 入口の解決と呼び出し（変更対象）

| 何 | どこ | いま |
|---|---|---|
| 3 入口の解決 | `shiori_proxy.rs`・`ShioriByteProxy::load` の `resolve` クロージャ | `GetProcAddress(module, s!("load"))` → `"unload"` → `"request"` の順。最初に欠けた名前で `ProxyError::EntryNotFound(&'static str)` を返す |
| 型 | 同ファイル `type LoadFn`／`type UnloadFn`／`type RequestFn` | `LoadFn = unsafe extern "C" fn(HGLOBAL, usize) -> bool`・`UnloadFn = unsafe extern "C" fn() -> bool`。`bool` は Rust の 1 バイト |
| 符号化＋確保＋呼出 | 同ファイル `ShioriByteProxy::encode_alloc_and_load(load: LoadFn, load_dir: &Path)` | `ansi_encode` → `global_alloc_copy` → `load(hdir, len)`。`if ok { Ok(()) } else { Err(LoadReturnedFalse) }` |
| 既定コードページ符号化 | 同ファイル `fn ansi_encode(path: &Path)` | `OsStr::encode_wide` → `WideCharToMultiByte(CP_ACP, 0, &wide, None, PCSTR::null(), None)` を長さ問い合わせと変換の 2 回。**最後の引数（`lpUsedDefaultChar`）は両方 `None`**。表せない字は既定文字に置き換えられ成功で返る |
| 失敗の記録 | `main.rs`・`handle_message` の `InboundAction::TriggerLoad` の枝 | 失敗時のみ `eprintln!("[helper] LOAD 失敗（観測・ack[0]）: {e:?}")`。成功時は 0 行 |
| パスの出どころ | `main.rs`・`fn load_dir_arg_env`（argv 第 2 引数 → env `HOST32_LOAD_DIR`）→ `PathBuf` → `HelperShared::load_dir` | 親 `crates/shiori-host32-host/src/process_host.rs` の `pub fn spawn(helper_exe, load_dir, shiori_name, parent_hwnd)` が `Command::arg(load_dir)` で渡す（UTF-16 の `CreateProcessW`）。helper の中では常に正しい Unicode |
| `shiori_proxy.rs` にログは 0 行 | — | `eprintln!("[helper] …")` は全て `main.rs`。proxy は `Result` で返すだけ |
| 説明文 | `shiori_proxy.rs` 冒頭のモジュール doc「確立シーケンス」手順 2〜5 | 「3 エクスポートすべて」「戻り `false`→`LoadReturnedFalse`」と書いてある（要件 8.4 の書き直し対象） |

`windows` 0.62.2 の `Win32_Globalization` は helper の `Cargo.toml` に既に有効で、`GetACP`・`MultiByteToWideChar`・`WC_NO_BEST_FIT_CHARS`・`CP_UTF8` が追加なしで使える（`c:\rust\cargo\registry\src\…\windows-0.62.2\src\Windows\Win32\Globalization\mod.rs` で確認）。`WideCharToMultiByte` の最後の引数は `Option<*mut BOOL>`。

### 2.2 テスト資産と作法

- **偽 DLL**: `crates/shiori-host32-testdll`（`[lib] name = "shiori"` → `shiori.dll`・`crate-type = ["cdylib"]`・依存は `windows` の `Win32_Foundation`＋`Win32_System_Memory` だけ）。`load` は受け取った HGLOBAL を `GlobalFree` して `HOST32_TESTDLL_LOAD_FAIL=1` なら偽、`unload` は `HOST32_TESTDLL_UNLOAD_MARKER` のパスへ `b"unloaded"` を書く。**無改変で残す**（要件 6.2）。
- **i686 限定の作法**: `#[cfg_attr(not(target_arch = "x86"), ignore = "i686 専用: …")]`（`shiori_proxy.rs` の `testdll_drop_invokes_courtesy_unload`／`testdll_request_roundtrip_get_and_notify`、`main_loopback_tests.rs` の `loopback_hello_request_proxy_driven_and_bounded_loop`）。
- **fixture の所在解決**: `fn resolve_testdll() -> PathBuf`——env `HOST32_TESTDLL_DLL` → `CARGO_MANIFEST_DIR/../../target/i686-pc-windows-msvc/{debug,release}/shiori.dll` → 無ければ「まず PowerShell で `cargo build -p shiori-host32-testdll --target i686-pc-windows-msvc`」と panic。**同じ関数が 6 か所に写されている**（helper: `shiori_proxy.rs` の `mod tests`・`main_loopback_tests.rs`／host: `tests/shiori_load_e2e.rs`・`shiori_request_e2e.rs`・`lifecycle_cyclic_e2e.rs`・`lifecycle_kill_e2e.rs`）。ファイル名 `shiori.dll` が固定で埋め込まれている。
- **テストは DLL を一時フォルダへコピーして読む**: `std::env::temp_dir().join(unique)` を `load_dir` にし、そこへ `shiori.dll` をコピーして `ShioriByteProxy::load(&dll_path, &load_dir)`。読む側は名前を自由に選べる（helper には argv で `shiori_name` を渡す）。
- **env 競合の直列化**: `static TESTDLL_SERIAL: Mutex<()>`（`shiori_proxy.rs` の `mod tests`）。fixture が読む env はプロセス全体で 1 つなので、fixture を読むテストは全部この鍵を取る。
- **純関数テストの置き場**: `main.rs` は `#[cfg(test)] #[path = "main_load_ack_tests.rs"] mod load_ack_tests;` の形で兄弟ファイルへ分けている（`main_classify_tests.rs`・`main_resolve_param_tests.rs` も同型）。1 ファイル 1,000 行の目安は `crates/log-capture-kit/tests/file_length_guard_test.rs` が機械で見張る（`LINE_LIMIT` = 1000・本番とテストの双方）。
- **行数の余裕**: `shiori_proxy.rs` **584 行**（本番 323 行＋テスト 261 行）、`main.rs` **586 行**。本番の追加は 60〜100 行の見込み、テストの追加が 150 行を超えるなら `shiori_proxy_loadu_tests.rs` のような兄弟ファイルへ（`#[path]` は `main.rs` の前例どおり）。
- **i686 先ビルド手順を書いている場所**（要件 6.9「全ての場所」の実測）: `crates/shiori-host32-host/README.md` の「手順（コピペ可）」①、host の e2e 4 本の冒頭 doc コメント（`//! cargo build -p shiori-host32-testdll --target i686-pc-windows-msvc`）と `resolve_testdll` の panic 文言、helper の `resolve_testdll` 2 か所の panic 文言、`.kiro/steering/structure.md`「Test DLL Fixture Crates」の節（「`load`/`unload`/`request` 3 エクスポート」と書いている）。`tools/perf/*.ps1` は helper exe だけを扱い testdll に触れない。開発者の記憶ファイル（`workspace-test-needs-i686-host32-artifacts`）はリポジトリ外。
- **workspace のメンバは `crates/*` の glob** なので、新クレートは `Cargo.toml` の編集なしで加わる。第三者ライセンスの謝辞 `THIRD-PARTY-NOTICES.md` は `cargo about generate --workspace about.hbs -o THIRD-PARTY-NOTICES.md` の生成物（`/kiro-complete` の License Gate が撮り直す）。roadmap.md の干渉台帳が「A1 の他 spec と共有するのはこの生成物と COMPAT §8 の末尾だけ」と登記している。

### 2.3 文書と台帳

- `doc/COMPAT_ARCHITECTURE.md` §8「沈黙ルール対応表」は 4 列（項目・裁量・根拠・出典 spec）の 1 表。`loadu`・`spec_dll` を含む行は **0 件**（要件 8.1 の 3 行を末尾に足す）。
- `doc/ukadoc-coverage/ledger/shiori.toml` の `[entry."ukadoc:spec_dll"]`（`status = "degraded"`・`priority = "A18"`）の `note` に「loadu は引かない」が **2 段落**（「壊れ方:」と「内容:」）にある。台帳冒頭の `#` 注釈「群 14c — DLL 共通仕様」にも同文。
- `doc/ukadoc-coverage/briefing-shiori.md` の群 14c（564 行付近）と「足りない物」（1109 行付近）にも同文。**この文書は人が書く**——`crates/ukadoc-survey/src/io/paths.rs` の `briefing_path()` が返すのは `briefing.md`（統合ブリーフィング）だけで、`briefing-shiori.md` を書く副手続きは無い（`cli/generate.rs` の冒頭 doc が生成側の副手続き 5 つを列挙: `catalog`・`ledger-init`・`report`・`report-summary`・`priority-apply`）。完了 spec `areka-P0-ukadoc-survey-shiori` の tasks.md 1.4 が「群の索引の正本は `briefing-shiori.md` の冒頭、写しが台帳冒頭の `#` コメント。`check` は `#` を読まない。更新は正本 → 写しの順」と記録している。
- `doc/ukadoc-coverage/README.md`: 「台帳を触ったら `cargo test -p ukadoc-survey` を走らせること」。

### 2.4 並走 spec との重なり

`.kiro/specs/` 直下の active spec（`completed/` を除く）で `shiori_proxy.rs` または `shiori-host32-helper` を挙げるのは 4 本の brief だけで、**編集を予定しているものは無い**:
- `areka-P0-makoto-dll-host/brief.md`: 「切れ端 ⓐ は本 spec へ移管・`shiori_proxy.rs` は編集集合から外れる」（明記）。
- `areka-P0-property-ipc-transport/brief.md`: 編集集合の見込みに `crates/shiori-host32-helper/src/` が入るが、対象は property の受け渡し（`main.rs` の分類・wire）で、入口解決には触れない見込み。**`main.rs` の `TriggerLoad` の枝に 1 行足す本仕様と同じファイルを触る可能性**は残る（着手時に rebase して確認・要件の「並走」の記述どおり）。
- `areka-P0-property-query-channels`: `areka-ghost` 側。`shiori_proxy.rs` には触れない。
- `areka-P0-alpha-release-signoff/brief.md`: 配布 zip に helper exe を入れる話のみ。

## 3. 要件 → 資産の対応表

| 要件 | 既存資産 | 隙間 | 種別 |
|---|---|---|---|
| 1.1〜1.5 入口の選択（4 通り・合わせて最大 1 回） | `resolve` クロージャ（3 名の解決） | `loadu` の任意解決＋「有無 × 有無 → 呼ぶ入口」の純関数が無い | Missing |
| 1.4 両方無い → `EntryNotFound` 相当 | `ProxyError::EntryNotFound(&'static str)` | 既存テスト `kernel32_yields_entry_not_found` が **`EntryNotFound("load")`** を固定している。解決順を変えても「両方無い」の名札を `"load"` に保てば要件 6.8（既存テスト無改変）と両立する | Constraint |
| 1.6 `loadu` が 0 でも `load` へ落ちない | `LoadReturnedFalse` | 判断が 1 か所なら自然に満たす（落ちる経路を作らなければよい） | — |
| 1.7 `unload`・`request` は必須のまま | `resolve` | 不変 | — |
| 2.1〜2.5 `loadu` へ UTF-8 | `Path` は helper 内で Unicode | `path.to_str()`／`as_os_str().to_string_lossy()` の選択（`OsStr` が不正 UTF-16 なら？→ argv 由来なので常に有効。`to_str()` の `None` を `EncodingFailed` にするのが最短） | Missing（小） |
| 2.3 メモリ規約同一 | `global_alloc_copy`（callee 解放） | 再利用 | — |
| 3.1〜3.6 表せない字の検出と警告 | `ansi_encode`（検出無し） | 検出の方法が 2 案（§5 の D1）。**UTF-8 既定機の罠** | Missing／Unknown |
| 3.4 CP932 を前提にしない | `CP_ACP` を渡している | 判定をコードページの値に依存させない設計（往復比較）か、`GetACP()` を読んで分岐 | Constraint |
| 4.1〜4.3 入口名を 1 行 | `[helper]` 行は `main.rs` のみ | どの入口を使ったかを知るのは proxy の中。proxy から出すか、戻り値で `main.rs` へ運ぶか（D3） | Missing |
| 4.4 失敗種別 1 行 | `eprintln!("[helper] LOAD 失敗…{e:?}")` | 既存で足りる（新 variant を増やさなければ `{e:?}` がそのまま出る） | — |
| 4.5 親へ伝えない | `load_result_to_ack` | 不変 | — |
| 5.1〜5.5 戻り値 `u8` | `LoadFn`／`UnloadFn` の `-> bool` | 型定義 2 行＋判定 1 行の差し替え。`transmute` の対象型が変わるだけ | Missing（小） |
| 6.1 2 本目の偽 DLL | `shiori-host32-testdll` が雛形 | 新クレート。**出力名は `shiori.dll` と別**（同じ target フォルダに同名 cdylib が 2 つあると cargo が「output filename collision」で後勝ちになり、既存 6 か所の解決器が拾う `shiori.dll` を壊す） | Missing |
| 6.3〜6.5 純関数テスト（x64 常時） | `ansi_encode_*` テスト 3 本が前例 | 入口の選択の判断表・UTF-8 固定バイト列・検出の決定論（コードページを引数で渡せる形にすれば `20127`（US-ASCII・常在）で「有り」、`65001` で「無し」を機械に依存せず固定できる） | Missing |
| 6.6〜6.7 i686 で 2 本目を実際に読む | `testdll_drop_invokes_courtesy_unload` が雛形 | 記録ファイルの env・偽返却の env・`TESTDLL_SERIAL` の共用 | Missing |
| 6.8 既存テスト無改変で緑 | — | 1.4 の名札の件（上）以外に衝突無し | Constraint |
| 6.9 先ビルド手順の全ての場所 | §2.2 の一覧 | README・e2e 4 本の doc コメント・steering structure.md。e2e の panic 文言は **e2e が 2 本目を読まない**ので足す必要が無い（足すと嘘になる） | Missing |
| 7.1〜7.5 実機 | `AREKA_APP_SMOKE_EXIT_MS`（有界 auto-exit）・`sample-ghost-kit`（検体の展開・`SampleRoot::acquire`） | 期待値の改訂（emo2 → `loadu`）。§6 参照 | Unknown |
| 8.1 COMPAT §8 に 3 行 | 表あり・該当行 0 | 追記のみ | Missing |
| 8.2 台帳の 2 段落＋冒頭注釈 | — | 手で直す（正本 → 写しの順） | Missing |
| 8.3 派生文書を生成器で | `report`／`report-summary` | **`briefing-shiori.md` は生成物ではない**（§2.3）。要件の文言が現物と合わない | Constraint（要件と食い違い） |
| 8.4 冒頭の説明 | モジュール doc | 書き直し | Missing |
| 9.1〜9.6 制約 | 依存 0・env は fixture のみ・unsafe 集約・1,000 行・i686 限定・ログ無し失敗 0 | 全て既存の作法で満たせる | — |

## 4. 実測

### 4.1 検体 3 体の入口（`dumpbin /exports`・VS2022 Professional 14.44.35207・`Hostx64/x86`）

`.nar` を `C:\Users\maz-o\AppData\Local\Temp\claude\loadu-gap\<名>\` へ展開（リポジトリ外）。全て `14C machine (x86)`。

| 検体 | DLL（大きさ） | `loadu` | `load` | `unload` | `request` | 他の公開名 |
|---|---|---|---|---|---|---|
| `konnoyayame.nar`（YAYA） | `ghost/master/yaya.dll`（909,312 B） | **あり** | あり | あり | あり | `multi_load`／`multi_loadu`／`multi_unload`／`multi_request`／`CI_check_failed`／`Set_loghandler`／`logsend` ほか |
| `R_POST_and_KOMAINU.nar`（里々） | `ghost/master/satori.dll`（720,896 B） | **なし** | あり | あり | あり | `getversionlist` |
| `emo2.nar`（pasta） | `ghost/master/pasta.dll`（3,832,832 B） | **あり**（brief・requirements は「なし」） | あり | あり | あり | `DllMain` と C ランタイムの printf 系 180 件弱 |
| （参考）同 里々の SAORI | `ghost/master/saori/fill_desktop.dll`（24,064 B） | なし | あり | あり | あり | — |

brief の表と食い違うのは **pasta の 1 件だけ**。YAYA・里々は一致。

### 4.2 pasta の `loadu` の出自（Research Needed）

- `vendors/pasta` の作業木（`048d646c`・`v0.1.6-1`）の `crates/pasta_shiori/src/windows.rs` に `loadu` は無い（`pub extern "C" fn load(hdir: HGLOBAL, len: usize) -> bool` のみ）。
- ローカルに在る全 ref（タグ `v0.3.4` まで）を `git grep loadu` しても **ソースに 1 件も無い**。`git log --all -S loadu` が当てるのは `release/hello-pasta/ghost/master/pasta.dll` などの **バイナリ**だけ（`v0.2.3`・`v0.3.2` のリリースビルド）。
- emo2.nar の `pasta.dll`（3,832,832 B）は各タグの `release/…/pasta.dll`（`v0.3.4` で 3,821,568 B）のどれとも一致しない＝**手元のどのタグより新しい（または別の）ビルド**。
- ~~結論: 手元で読めない~~ → **要件ディスカッション（2026-09-23）で解決**: 本ブランチの `vendors/pasta` の作業木が、本流の記録する版（`48c42fc3`＝`release: v0.3.5 (#34)`）より古い `048d646c` に取り残されていただけだった。`git -C vendors/pasta fetch` で記録された版を取得して読むと:
  - emo2.nar の `pasta.dll` は `48c42fc3` の `release/hello-pasta/ghost/master/pasta.dll` と **blob が一致**（`0ecfafdead1972225ee5658fbe2666fd5f12031d`・3,832,832 B）。
  - `crates/pasta_shiori/src/windows.rs` の `pub extern "C" fn loadu(hdir: HGLOBAL, len: usize) -> bool` は `load_entry("loadu", …, DirEncoding::Utf8)`。戻りは `load` と同じ Rust の `bool`（1 バイト）。`loadu` で初期化済みのとき後続の `load` は HGLOBAL を解放して何もせず真を返す（正典の「望ましい」作法）。上流に `tests/ffi_loadu_test.rs` と `windows_tests.rs` の検証がある。
  - ＝emo2 の `loadu` の枝は上流で検証済みの実装を踏む。実機確認（要件 7.1）は念押しとして残る。

### 4.3 i686 限定テストと fixture の所在（§2.2 の裏取り）

- ゲート: `#[cfg_attr(not(target_arch = "x86"), ignore = "i686 専用: …")]`。x64 の `cargo test --workspace` では ignored として数えられ、`cargo test -p shiori-host32-helper --target i686-pc-windows-msvc`（PowerShell 必須）で実行される。
- 所在: `target/i686-pc-windows-msvc/{debug,release}/shiori.dll`。2 本目は同じフォルダに **別名**（例: `shiori_loadu.dll`）で置き、helper の解決器を「ファイル名を引数に取る 1 関数」に寄せれば 6 か所の複製は増えない（host の e2e 4 本は 2 本目を読まないので触らない）。
- 先ビルドの命令は 1 行増える: `cargo build -p <新クレート名> --target i686-pc-windows-msvc`（`-p` を 2 つ並べた 1 行にまとめてもよい。完了 spec の tasks に `cargo build -p shiori-host32-helper -p shiori-host32-testdll --target i686-pc-windows-msvc` の前例がある）。

### 4.4 行数（1,000 行の目安）

| ファイル | 現在 | 見込み |
|---|---|---|
| `crates/shiori-host32-helper/src/shiori_proxy.rs` | 584 行 | 本番 +60〜100・テスト +100〜200 → 750〜880 行。テストが膨らむなら兄弟ファイル `shiori_proxy_loadu_tests.rs` へ（`#[path]`） |
| `crates/shiori-host32-helper/src/main.rs` | 586 行 | +0〜5 行（記録 1 行を proxy 側で出すなら 0） |
| 新 fixture クレート `src/lib.rs` | — | 既存 testdll（340 行・うちテスト 130 行）を雛形に 150〜250 行 |

## 5. 実装案

### 案 A: 既存 2 関数を拡張する（brief の推奨・本文書も推奨）

- `resolve` に `GetProcAddress(module, s!("loadu"))` を **任意**で足し、`load` も任意にして、`unload`・`request` は必須のまま。
- 純関数 `fn choose_init_entry(loadu: Option<LoaduFn>, load: Option<LoadFn>) -> Result<InitEntry, ProxyError>`（`enum InitEntry { Loadu(LoaduFn), Load(LoadFn) }`）を 1 つ置く。判断はここだけ（要件 1.8）。両方 `None` → `EntryNotFound("load")`（既存テストの名札を保つ）。
- `encode_alloc_and_load(entry: InitEntry, load_dir)`: `Loadu` なら `load_dir.to_str()` の UTF-8 バイト列、`Load` なら `ansi_encode`（検出付き）。確保と呼出は共通。戻り `u8 != 0`。
- `ansi_encode` に検出を足し、検出したら `eprintln!("[helper] …")` を 1 行（または検出結果を返して呼び手が出す）。
- 入口名の 1 行は呼ぶ直前に proxy から `eprintln!`（D3）。
- 長所: 差分が最小・凍結面（wire・親・ack）無改変・既存テスト無改変。短所: `shiori_proxy.rs` にログの前例ができる（今は 0 行）。

### 案 B: 入口の選択と符号化を新モジュールへ切り出す

- `shiori_entry.rs`（入口の解決＋選択の純関数）と `shiori_encode.rs`（UTF-8／既定コードページ・検出）を新設し、`shiori_proxy.rs` は呼ぶだけに痩せる。
- 長所: `shiori_proxy.rs` の行数が減り、純関数テストが自然に別ファイルへ。短所: unsafe（`GetProcAddress`・`WideCharToMultiByte`）の置き場が 2〜3 ファイルに広がり、要件 9.3「unsafe を `shiori_proxy.rs` に集約」と衝突する。ファイルも増える。

### 案 C: 案 A ＋ テストだけ兄弟ファイルへ

- 本番は案 A のまま `shiori_proxy.rs` に閉じ、新しいテスト（判断表・UTF-8 固定列・検出の決定論・2 本目 fixture の i686 実読）は `shiori_proxy_loadu_tests.rs` に置く（`main.rs` の `#[path]` の作法）。共有の `resolve_testdll` は `shiori_proxy_test_support.rs` のような 1 か所へ寄せ、ファイル名を引数に取る。
- 長所: 9.3 と 9.4 を同時に満たし、行数の余裕を保つ。短所: `mod tests` の既存 4 本と新テストが 2 ファイルに分かれる（探す先が 2 つ）。

**推奨: 案 A、テストが 150 行を超えたら案 C。** 案 B は 9.3 と衝突するので採らない。

## 6. 実機確認に要るもの（本フェーズでは実行しない）

1. **成果物**: PowerShell で `cargo build -p shiori-host32-helper --target i686-pc-windows-msvc`。workspace のビルドが `target\debug\shiori-host32-helper.exe` を x64 で上書きする罠があるので、i686 の exe を `areka.exe` の隣へ複製する（完了 spec `balloon-break` tasks の記録・`tools/perf/README.md` の `Sync-MeasureShioriHelper` と同じ理由）。
2. **起動**: `areka.exe` に検体のゴースト根を渡し（`crates/areka/src/main.rs` の `resolve_config_inputs(&args)` が決める）、`AREKA_APP_SMOKE_EXIT_MS` で有界に終える。検体は `sample-ghost-kit` の `SampleRoot::acquire("konnoyayame" | "R_POST_and_KOMAINU" | "emo2")` で展開したものを使う（`.nar` の直接展開は見張りテスト `sample_path_guard_test.rs` の対象外＝手作業なら可）。
3. **観測**: helper の `eprintln!("[helper] …")` は親の stderr にそのまま混ざる。要件 4.1 の固定語句と要件 3.1 の警告を stderr から grep する。`RUST_LOG` は判定の分岐の水準まで開ける（記憶: 実機サインオフは判定の分岐の log level まで開ける）。
4. **既定コードページに無い字を含むフォルダ**: 展開した検体を `…\ゴースト😀\` のような名前のフォルダへ複製して同じ手順で起動。**実装前に**今日の壊れ方（里々: 黙って辞書を見失う・YAYA／pasta: `load` に化けたパスが渡る）を赤として記録する（要件 7.3）。`emo2` 実走は絶対パス・短いパスが要る（記憶 `areka-emo2-signoff-needs-absolute-paths`）ので、絵文字入りフォルダは浅い場所に置く。
5. **期待値**（§4.1 の実測に合わせて改める）: `konnoyayame` → `loadu`、**`emo2` → `loadu`**（pasta の `loadu` が動くかはここで初めて分かる）、`R_POST_and_KOMAINU` → `load`・警告 0 行、絵文字フォルダの里々 → `load`・警告 1 行。

## 7. 設計フェーズへの推奨と調査項目

- **推奨**: 案 A（テスト肥大なら案 C）。判断の純関数を 1 つ、符号化の分岐を 1 つ、ログ 2 行（入口名・警告）を proxy に。戻り値は `u8`。
- **Research Needed**:
  1. 表せない字の検出法の確定（D1）。`GetACP() == 65001` の機械で `lpUsedDefaultChar` が `ERROR_INVALID_PARAMETER` になる点は Win32 の仕様（`CP_UTF8`／`CP_UTF7` では `lpDefaultChar`・`lpUsedDefaultChar` は NULL 必須）。往復比較なら回避できる。設計で 1 度だけ実機（既定を UTF-8 に切り替えた環境は無くてもよい＝`65001` を明示して呼べば同じ経路を踏める）で確認する。
  2. pasta の `loadu` の実装（§4.2）。`vendors/pasta` を最新へ進めて読めるなら読む。読めなくても実機確認で決着する。
  3. `to_str()` が `None` を返す経路（argv が不正 UTF-16）を `EncodingFailed` に落とす扱いでよいか（実際には起きないが、ログ無し失敗経路 0 本の規律から見て 1 行残す）。

## 8. 設計判断の議題（要件ディスカッションへ）

> **設計フェーズ（2026-09-23・`/kiro-spec-design`）で 1〜7・11〜13 は全て決めた**——決定は design.md 末尾「設計判断の対応」の表が正本、経緯は本文書 §10。
> **要件ディスカッション（2026-09-23）の仕分け**: 1〜7・11・12 は how の判断＝**設計フェーズ（`/kiro-spec-design`）で決める**。8 は解決済み（pasta の `loadu` は上流で検証済み＝§4.2・要件 7.1〜7.2 を改訂）。9 は要件 8.3 の改訂で決着。10 の前半（steering `structure.md`）は要件 8.6 として要件に入れ、後半（`THIRD-PARTY-NOTICES.md`）は `/kiro-complete` の仕事のまま。開発者に問う議題は 0 件。

1. **表せない字の検出法**: (a) `WideCharToMultiByte` に `WC_NO_BEST_FIT_CHARS` と `lpUsedDefaultChar` を渡す＋`GetACP()` が 65001 なら検出を飛ばす（UTF-8 に表せない字は無い）／(b) 変換後に `MultiByteToWideChar` で戻して元の UTF-16 と比べる（コードページを問わず同じ手順・「最も近い字へ寄せる」置換も検出できる・OS 呼び出しが 1 回増える）。どちらも新しい依存無し。**(b) を推奨**（分岐が 1 つ減り、要件 3.4・6.5 の「コードページに依存しない判定」がそのまま満たせる）。
2. **符号化の関数の形**: 本番は `CP_ACP` 固定のまま、テストのためにコードページを引数で受ける内側の関数（例: `fn encode_with_codepage(cp: u32, path) -> Result<(Vec<u8>, bool /* 置換あり */), _>`）を置くか。置けば要件 6.5 を `20127`（US-ASCII・全機に常在）と `65001` で機械に依存せず固定できる。置かないと CP932 の有無に縛られる。**置くのを推奨**。
3. **入口名の 1 行をどこで出すか**: (a) proxy が呼ぶ直前に `eprintln!("[helper] …")`（proxy 初のログ行。成功・失敗を問わず 1 回で要件 4.2 を自然に満たす）／(b) `ShioriByteProxy::load` の戻りに使った入口を載せて `main.rs` が出す（失敗時は `ProxyError` にも入口を載せる必要があり variant が増える）。**(a) を推奨**。
4. **「両方無い」の名札**: 既存テスト `kernel32_yields_entry_not_found` は `EntryNotFound("load")` を固定している。`loadu` を先に引いても、両方無いときの名札を `"load"`（または `"loadu/load"` にしてテストを改める）のどちらにするか。要件 6.8（既存テスト無改変）を守るなら `"load"`。**`"load"` を推奨**し、variant の doc コメントで「初期化の入口（`loadu`／`load`）が両方無いときは `"load"` と記す」と書く。
5. **2 本目の偽 DLL の名前と場所**: クレート名（例 `shiori-host32-testdll-loadu`）・`[lib] name`（例 `shiori_loadu` → `shiori_loadu.dll`）。`shiori.dll` の再利用は不可（同じ target フォルダでの出力名衝突＋既存解決器が同名を拾う）。
6. **偽 DLL の記録の形**: env `HOST32_TESTDLL_LOADU_RECORD=<ファイル>` に「呼ばれた入口名＋受け取ったバイト列」を書く。`load` も同じファイルへ **追記**する（「`load` が呼ばれなかった」を「記録に `load` の行が無い」で示すため）。偽返却は `HOST32_TESTDLL_LOADU_FAIL=1`。`unload`・`request` は既存 testdll と同じ固定応答でよいか（`request` は今回の対象外なので 400 固定でも足りる）。
7. **既存 `resolve_testdll` 6 か所の複製**: helper 側 2 か所をファイル名引数の 1 関数（`shiori_proxy_test_support.rs`）へ寄せるか、2 本目用に 1 つ足すだけにするか。host の e2e 4 本は触らない。
8. **要件 7.1〜7.2 の期待値の改訂**（要件 7.4 の手順）: emo2 は `loadu` の枝へ。`load` の枝を実機で踏む検体は里々だけになる。pasta の `loadu` が動かなかった場合の扱い（本仕様の欠陥ではない・上流へ報告・emo2 の起動が壊れるので放置はできない）を先に決めておく。
9. **要件 8.3 の読み替え**: `briefing-shiori.md` の群 14c を **手で**直し（正本）、台帳冒頭の注釈（写し）と `[entry."ukadoc:spec_dll"]` の `note` 2 段落を揃え、`cargo run -p ukadoc-survey -- report` と `report-summary` で `report/*.md` を撮り直す（数字は変わらない見込み）。`cargo test -p ukadoc-survey` を通す。
10. **steering の追随**: `.kiro/steering/structure.md`「Test DLL Fixture Crates」の節に 2 本目を 1 行足す（要件 6.9 の「全ての場所」に含めるか）。`THIRD-PARTY-NOTICES.md` は `/kiro-complete` の License Gate が撮り直す（本仕様のタスクには入れない）。
11. **`unload` の型変更の範囲**: `UnloadFn -> u8` に変えても Drop は結果を捨てるだけ（要件 5.4）。既存 testdll の `unload() -> bool` と ABI が 1 バイトで一致するので無改変で通る。確認だけ。
12. **`loadu` へ渡すパスが UTF-8 にできないとき**（§7 の調査項目 3）: `Path::to_str()` が `None`（argv が不正な UTF-16）の経路。実際には起きないが、ログ無し失敗経路 0 本（要件 9.6）から見て「符号化失敗」として 1 行残すか、`to_string_lossy` で置き換えて渡すか。要件 3.6 の「符号化失敗」と同じ扱いに寄せるのが素直。
13. **表せない字の検出（議題 1）は要件 3.1 の改訂で制約が 1 つ増えた**: 似た字への置き換え（best-fit）も「表せない」に数え、かつ `load` へ渡すバイト列は今日と同一（変換フラグを変えない）。(a) の `WC_NO_BEST_FIT_CHARS` を本番の変換に掛けると渡すバイト列が変わるので、(a) を採るなら判定専用の 2 回目の変換になる。(b) の往復比較は本番の変換をそのまま使えるので、この制約とも素直に合う。

## 9. requirements.md と食い違う点

> **要件ディスカッション（2026-09-23）で 3 件とも requirements.md へ反映済み**（1: 要件 1.3・7.1・7.2・7.4 と Introduction の検体表／2: 要件 8.3 を「正本を手で → 写し → `report/*.md` は生成器」に改訂／3: 要件 9.4 を 584 行に）。

1. **要件 7.2・Introduction の検体表・brief の表**: emo2 の `pasta.dll` は `loadu` を**公開している**（§4.1・`dumpbin /exports`）。要件 7.2「`emo2`（pasta）… `load` を記録」は実物と合わない。要件 7.4 が改訂の手順を先に書いているので、ディスカッションで 7.1〜7.2 を「`konnoyayame`・`emo2` → `loadu`／`R_POST_and_KOMAINU` → `load`」に改める。同時に要件 1.3 の括弧「既存の検体＝里々・pasta と既存のテストは挙動不変」の「pasta」は外れる（pasta は挙動が変わる）。
2. **要件 8.3**: 「派生文書（`briefing-shiori.md` ほか）を `ukadoc-survey` の生成器で撮り直し、手で直さない」は現物と合わない。`briefing-shiori.md` は人が書く正本で、生成器に書く副手続きが無い（§2.3）。生成器で撮り直せるのは `report/shiori.md`・`report/summary.md` だけ。文言を「`briefing-shiori.md`（正本）と台帳（写し・項目）を手で揃え、`report/*.md` を生成器で撮り直す」に改める必要がある。
3. **要件 9.4 の行数**: 「現在 585 行」は実測 **584 行**（`wc -l`）。結論に影響しない。

（要件 1.4 と 6.8 の名札の件は食い違いではなく設計で吸収できる制約＝議題 4。）

## 10. 設計フェーズの記録（2026-09-23・`/kiro-spec-design`）

> ディスカバリの種別: **拡張（light）**。§1〜§7 の調査で対象の 2 関数・既存の作法・fixture の雛形が揃っていたので、追加の調査は `windows` 0.62.2 の API 形（`MultiByteToWideChar(codepage, flags, &[u8], Option<&mut [u16]>) -> i32`・`WideCharToMultiByte` の最後の引数が `Option<*mut BOOL>`・`GetACP`・`CP_UTF8`・`WC_NO_BEST_FIT_CHARS` が `Globalization/mod.rs` に在ること）の確認と、helper の `Cargo.toml` に `tracing` が無い（観測は全て `eprintln!`）ことの確認だけ。外部調査（WebSearch）は行っていない（新技術 0・正典は要件に逐語で載っている）。

### 10.1 統合（synthesis）の結果

- **一般化**: `loadu` と `load` は「同じ署名・同じ所有権規約・違うのはパスの文字コードだけ」なので、fn ポインタの型は `LoadFn` 1 つに統一し、違いは `InitEntry` の変種（`Loadu`／`Load`）に載せる。符号化は「入口 → バイト列」の 1 関数 `init_bytes` に畳む。`encode_with_codepage` はコードページを引数に取る形にしたが、実装は `CP_ACP` 1 つしか使わない（一般化は境界だけ・実装は今の要件の範囲）。
- **作る／借りる**: 表せない字の検出は Win32 の既存 API（`MultiByteToWideChar`）で往復比較するだけ。crate は足さない。`lpUsedDefaultChar` は UTF-8 既定機で関数自体が失敗するので借りない（§1・§8-1）。
- **簡素化**: (i) 入口名の行は proxy から直接 `eprintln!`（`ProxyError` に入口を載せて `main.rs` へ運ぶ案は variant が増え、`main.rs` と 2 ファイルの改変になる）。(ii) `struct ShioriByteProxy` の `load: LoadFn` 欄は読む者が無いので外す。(iii) 2 本目の fixture の `request` は 400 固定（既存の `parse_request` を写さない）。(iv) `resolve_testdll` の統合は要件 6.8 が既存テストの改変を禁じるので行わず、新テストファイルに 1 つだけ置く。

### 10.2 決定（§8 の議題 1〜7・11〜13）

| # | 決定 | 捨てた案と理由 |
|---|---|---|
| 1 | **(b) 往復比較**。本番の `WideCharToMultiByte(cp, 0, …)` は今日のまま、`MultiByteToWideChar(cp, 0, &bytes, …)` で戻して元の UTF-16 と比べ、違えば `lossy = true`。戻せない（0 以下）ときも `lossy = true`（警告して渡す） | (a) `lpUsedDefaultChar`＋`GetACP()==65001` の分岐: 65001 で関数が失敗する罠・分岐が 1 つ増える・best-fit を捕まえるには判定専用の 2 回目の変換が要る（§8-13） |
| 2 | **置く**。`fn encode_with_codepage(cp: u32, path: &Path) -> Result<CodepageEncoded { bytes, lossy }, ProxyError>`。`ansi_encode` は `.map(\|e\| e.bytes)` の薄い包みで署名不変 | 置かない: 検出のテストが機械の CP932 の有無に縛られる（要件 6.5 を満たせない） |
| 3 | **(a) proxy が呼ぶ直前に `eprintln!`**。固定語句 `[helper] SHIORI 初期化の入口: {loadu\|load}`。`main.rs` は 0 行変更 | (b) 戻り値で運ぶ: `ProxyError` の全 variant に入口を載せる必要があり、`main.rs` の `{e:?}` の出力も変わる |
| 4 | **`"load"`**。`choose_init_entry` の `(None, None)` は `EntryNotFound("load")`。variant の doc に明記 | `"loadu/load"`: 既存テスト `kernel32_yields_entry_not_found` の改変が要る（要件 6.8 違反） |
| 5 | `crates/shiori-host32-testdll-loadu`・`[lib] name = "shiori_loadu"` → `shiori_loadu.dll`。**戻りは `i32`（Win32 `BOOL` 4 バイト）**＝既存 fixture の Rust `bool` 1 バイトと対にして、helper の `u8` 受けが両方で正しいことを偽 DLL で踏む | `shiori.dll` の再利用: 出力名の衝突（同じ target フォルダで後勝ち）＋既存 6 か所の解決器が拾う |
| 6 | env `HOST32_TESTDLL_LOADU_RECORD=<ファイル>` に `<入口名>\t<小文字 16 進>\n` を追記（`loadu`・`load` とも同じファイル）。`HOST32_TESTDLL_LOADU_FAIL=1` で `loadu` が 0。`unload` は 1 固定・`request` は 400 固定 | 生バイトをそのまま書く: 改行や任意バイトで行の境界が壊れる。`load` を別ファイルに: 「呼ばれなかった」の判定が「ファイルが無い」になり、書き忘れと区別できない |
| 7 | **統合しない**。新テストファイルに `resolve_loadu_testdll()` を 1 つ置く（7 か所目）。steering `structure.md` の `<stem>_test_support.rs` への集約は、要件 6.8 の凍結（既存テスト無改変）が解けた次の機会に既存 6 か所と一緒に行う | 今統合する: 既存 `mod tests`・`main_loopback_tests.rs` の改変になる（6.8 違反）。1 消費者のための support ファイルは指向の無い間接になる |
| 11 | `UnloadFn -> u8`・`Drop` は `let _ = (self.unload)()` のまま。既存 fixture の `unload() -> bool` と 1 バイトで ABI 一致（確認のみ・改変 0） | — |
| 12 | `Path::to_str()` が `None` → `EncodingFailed`（`main.rs` の既存の 1 行が出る） | `to_string_lossy` で置き換えて渡す: 「置き換えない」（要件 2.4）と矛盾 |
| 13 | 1 の往復比較で同時に満たす（本番の変換に `WC_NO_BEST_FIT_CHARS` を掛けない・渡すバイト列は今日と同一） | — |

### 10.3 境界の決定と根拠

- **`main.rs` を触らない**（決定 3 の帰結）。並走 `areka-P0-property-ipc-transport` が `main.rs` を触る見込み（§2.4）なので、同じファイルを触らないことで合流の衝突を 0 にする。
- **既存 `mod tests` と `main_loopback_tests.rs` を触らない**（要件 6.8）。新テストは兄弟ファイル `shiori_proxy_loadu_tests.rs`（`shiori_proxy.rs` 末尾の `#[cfg(test)] #[path] mod loadu_tests;`）。env が既存 fixture と重ならないので直列化の鍵も自前（`LOADU_SERIAL`）。
- **`ProxyError` の variant を増やさない**。増やすと `main.rs` の `{e:?}` の語彙が変わり、失敗種別の grep（要件 4.4）と下流 `makoto-dll-host` の期待が動く。
- **観測の 2 行の位置**: 警告は符号化の直後（`Load` の枝だけ）、入口名は呼ぶ直前。符号化・確保で失敗したときは入口を呼ばないので入口名の行は出ない（要件 4.1 の主語は「呼んだ」入口）。失敗の種別は `main.rs` の既存の 1 行が出す。

### 10.4 リスクと手当て

- **20127（US-ASCII）が無効な機械**: 往路の `WideCharToMultiByte` が 0 以下を返し `Err` になるので、群 C のテストは黙って緑にならず赤で気付く。そのときは 1252 に差し替える（設計 `encode_with_codepage` の Implementation Notes）。
- **fixture の戻りを `i32` にしたことで既存 fixture と型が揃わない**: 意図的（5.1 の両側を踏む）。`lib.rs` の冒頭 doc に理由を書く。
- **pasta の `loadu` の実機**: 上流で検証済み（§4.2）。実機確認 7.1 で念押し。
- **UTF-8 既定機（`GetACP()==65001`）**: 本番の変換は今日のままなので `load` の枝は壊れず、往復は一致するので警告 0 行。設計で 1 度だけ `encode_with_codepage(65001, …)` を群 C で踏む（機械を切り替えずに同じ経路を通る）。
