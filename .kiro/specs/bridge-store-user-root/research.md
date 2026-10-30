# ギャップ分析: bridge-store-user-root

> 2026-10-29・`/kiro-validate-gap`。入力は `requirements.md`（要件 1〜6）・`brief.md`・steering（`product.md`・`tech.md`・`structure.md`・`logging.md`）・今のコード（ワークツリーの先頭 `7cd36fe9`）。行の番号はこの日のもの。

## 1. 要約

- **直す所はほぼ `crates/areka-mcp-bridge/src/config.rs` の 1 か所**。今は `from_env` が `areka_user_data::establish` を呼び、利用者ごとの置き場を作り（`create_dir_all`）、exe の隣の記憶を写す（`migrate`）。要件 2.2・2.3 の直しは、この呼び出しを「決めるだけ」に替えることに尽きる。
- **見落としやすい穴が 1 つある**: 控えを書く `cache.rs` の `replace` は、書く前に控えのフォルダを `create_dir_all` で親ごと作る。決めた控えの置き場が「まだ無い利用者ごとの置き場」や「まだ無い `profile\areka`」の下を指したままだと、`establish` を外しても、最初の控えの書きで置き場が作られてしまう。要件 2.4・2.5 の場面では、控えの置き場そのものを「無し」（`None`）にしないと要件 2.2・2.5 を守れない。
- **`readonly-mode` が足した口（`establish_readonly`・`readonly_app_profile_dir`）は、そのままでは使えない**。置き場がまだ無いときは exe の隣の形を返し、写すはずの場面では exe の隣の `profile\areka` を返す。どちらも控えが exe の隣に書かれる（要件 2.7 に反する）。記録の文も「read-only run」で、中継の記録としては誤解を招く。判断（`probe`・`decide`）は公開されていて使える。写しの条件（`migrate_source`）はクレートの外から呼べない。
- **箱の外へ届く手は、要件 4.2 の条件をすべて満たすものが見つかっていない**。文書で約束されている止め方は Claude アプリのパッケージの宣言でしか選べない。例外として、子のプロセスを「デスクトップ アプリの実行環境」の外で起こす文書化された属性（`PROC_THREAD_ATTRIBUTE_DESKTOP_APP_POLICY`）がある。ただし、受け継いだ方針の下で子孫が外へ出られるかは文書から読み切れず、作りも重い。
- **箱の中で動いているかの見分けは、パッケージの名乗りだけでは足りないおそれが強い**。この机で測ると、Claude アプリが起こした子のプロセスは、パッケージの名乗りを持たない（`GetCurrentPackageFullName` が「パッケージ無し」を返す）のに、`%LOCALAPPDATA%` の読み書きは箱へ回されていた。一方、箱へ回されたフォルダの実際の場所は、`GetFinalPathNameByHandle`（Rust の `std::fs::canonicalize` と同じ OS の呼び出し）で読めた。依存を足さずに書ける。ただし、箱の中にまだ何も無い初めての起動では見分けられない。

## 2. 今のコードの姿

### 2.1 中継の控えの置き場の決め方（`crates/areka-mcp-bridge/src/config.rs`・130 行）

- `Config::from_env`（58〜86 行）が、環境と exe の場所を読む唯一の口。
  - 隣に `areka.exe` があれば、`areka_user_data::establish(Some(dir), profile_env)`（67 行）で形を決め、同時に**作る・写す**。そのあと `app_profile_dir`（68 行）で記憶の置き場を出す。
  - 控えの置き場が無いときは、`warn!` を 1 件残す（80〜84 行・ASCII）。
- `Config::from_values`（94〜112 行）は値だけで決める純粋な関数。段は `AREKA_PROFILE_DIR` → `beside_profile` → `LOCALAPPDATA\areka` → 無し、で、どれもその下の `mcp-bridge`。
- 置き場を決めた行は `main.rs` の 71〜73 行（`[bridge] cache dir`・info）が残す。

### 2.2 控えの書き（`crates/areka-mcp-bridge/src/cache.rs`・101 行）

- `replace`（88〜97 行）は、`create_dir_all(dir)` で控えのフォルダを親ごと作ってから、一時の名前に書いて `rename` で置き換える（要件 3.11 of `mcp-stdio-bridge`＝控えはいつも答えの全体）。
- 書けないときは `warn!`（場所と理由・69 行）を残して続ける。要件 3.4 は今のままで満たしている。
- **この spec にとっての穴**: 控えの置き場を「まだ無い置き場の下」に決めると、ここで置き場が作られる。決めるだけにする直しは、`config.rs` で `cache_dir = None` を返すところまで要る。

### 2.3 `areka-user-data` の公開の口（`lib.rs` 205 行・`establish.rs` 310 行）

| 口 | 公開 | 作る | 写す | 記録 | この spec での使い道 |
|---|---|---|---|---|---|
| `probe(exe_dir)` | pub | しない（書けるかは物を作らずに尋ねる） | しない | winget の控えを読めないときの warn | そのまま使える。ただし環境変数を直に読むので、テストでは `Signals` を組んで渡す口が要る |
| `decide(&Signals)` | pub | しない | しない | しない | そのまま使える。決められないとき（`Fallback`）の記録は呼び手が持つ |
| `establish` | pub | する | する | `user_data_layout` ほか | **中継から外す対象** |
| `establish_readonly(_from)` | pub | しない | しない | `decide_logged` の記録・`user_root_withheld`（"read-only run" の文）・`user_data_layout` | 置き場が無いときは `user_root: None`（＝exe の隣の形）を返す。理由（`Reason`）が `Neither` でないのに `user_root` が無い、という形でしか「控えた」ことが分からず、決められない場合（`Fallback`）と見分けがつかない |
| `readonly_app_profile_dir` | pub | しない | しない | `app_memory_read_beside`（"read-only run" の文） | 写すはずの場面では **exe の隣の `profile\areka` を返す**＝要件 2.5・2.7 と合わない |
| `app_profile_dir` | pub | しない | しない | しない | そのまま使える |
| `decide_logged` | 非公開 | — | — | 書けるか分からない warn・決められない error | 記録を分け合いたいなら公開が要る |
| `migrate_source(root, exe_dir)` | `pub(crate)` | — | — | — | 要件 2.5 の判定（`root\profile\areka` が無く、exe の隣の `profile\areka` がフォルダ）と**まったく同じ条件**。注記に「写しの条件の持ち主は 1 つ」とある。中継が同じ条件を書き写すと持ち主が 2 つになる |

areka 本体（`crates/areka/src/boot_config.rs` 528〜552 行）は、ふつうの走りで `establish`＋`app_profile_dir`、読み取りだけの走りで `establish_readonly`＋`readonly_app_profile_dir` を使う。この spec で `areka-user-data` に口を足したり `migrate_source` を公開したりしても、areka 本体の振る舞いは変わらない（要件 6.1 に反しない）。

### 2.4 要件 2 の各場面で、今の口が返す控えの置き場

`W` は winget の形（利用者ごとの置き場を使う形）、`R` は利用者ごとの置き場（`%LOCALAPPDATA%\areka`）、`X` は exe のフォルダ。

| 場面 | 要件が求める置き場 | 今（`establish`） | `establish_readonly`＋`readonly_app_profile_dir` |
|---|---|---|---|
| 2.4 W・`R` が無い | 無し（作らない） | `R` を作る→`R\profile\areka\mcp-bridge`（`X\profile\areka` があれば写す） | `X\profile\areka\mcp-bridge`（要件 2.7 に反する） |
| 2.5 W・`R` は在る・`R\profile\areka` が無い・`X\profile\areka` が在る | 無し（`R\profile\areka` を作らない） | 写してから `R\profile\areka\mcp-bridge` | `X\profile\areka\mcp-bridge`（要件 2.7 に反する） |
| 2.6 W・ほか | `R\profile\areka\mcp-bridge` | 同じ（ただし `R\profile\areka` が無ければ控えの書きで作る） | 同じ |
| 2.8 zip の形 | `X\profile\areka\mcp-bridge` | 同じ | 同じ |
| 2.9 `AREKA_PROFILE_DIR` | その下の `mcp-bridge` | 同じ（ただし `establish` は `R` を作る） | 同じ |
| 2.10 隣に `areka.exe` が無い | `LOCALAPPDATA\areka\mcp-bridge` | 同じ | 同じ |

2.6 の「`R` は在るが `R\profile\areka` も `X\profile\areka` も無い」場面（areka を一度起動して置き場だけ作り、記憶がまだ無い）は、`replace` の `create_dir_all` が `R\profile\areka` を作る。写しの条件は「写し先が無いこと」なので、この後に areka が写しを飛ばしても、写す元が無いので失うものは無い。要件 2.6 のとおりでよい見込み（設計で確かめる）。

**`Fallback`（利用者ごとの置き場を使う形に当たったが、場所を決められない）**は、areka 本体が exe の隣の形で続ける場面。今の中継は `X\profile\areka\mcp-bridge` に控えを置く。これが要件 2.7（利用者ごとの置き場を使う形なら exe の隣に書かない）に当たるのか、要件 2.1（どの置き場に当たるかの答えを今から変えない）と要件 3.3（決められなければ控えなしで warn）のどちらに倒すのかは、要件の文からは決まらない（討議 4）。

### 2.5 テストの今の形

- `config_tests.rs`（166 行）は `from_values` の値だけを見る。I/O は無い。
- `tests/exe_test.rs` は建てた exe を起こす。`AREKA_PROFILE_DIR` を `target` の下へ向け、`AREKA_USER_ROOT` を外している（52〜54 行の注記: `target\debug` の隣に `areka.exe` があり、中継は areka と同じ準備を通る）。直した後は、この準備が何も作らなくなる。
- 中継のクレートは `log-capture-kit` を開発用の依存に持っていない。要件 6.6（記録の行を決定論テストで固定）には、ワークスペースの中のクレートを開発用の依存に足すことが要る（外部依存ではない）。`areka-user-data` の `establish_readonly_tests.rs` が、`Signals` を組んで渡し、`target` の下の根で「作られていない・中身が変わっていない」を確かめる型（`tree` の比べ）を持っている。要件 6.5 はこの型を写せばよい。
- 要件 6.7（環境の値と OS の場所を注入する）には、`probe` が環境変数を直に読むので、中継にも「`Signals` と exe のフォルダと `AREKA_PROFILE_DIR` の値を受け取る内側の口」が要る（`establish_readonly_from` と同じ分け方）。

### 2.6 説明書と道具

- `dist/README.txt`（460 行）で中継の控えの場所を書いているのは 3 か所: 289 行（zip の形の一覧の 1 行）・297 行（利用者ごとの置き場の形の一覧の 1 行）・308 行（Store 版の箱へ回る話の段落）。brief は「289 行目」と書いているが、Store 版の段落は 308 行目。要件 5.5 の「段落の外の行を変えない」は、どの行を段落と数えるかで変えてよい範囲が変わる（討議 7）。
- `tools/mcp-bridge-install.ps1`（156 行）は中継を `AREKA_MCP_HOME` に置く道具。隣に `areka.exe` が無い形なので、開発者の机の中継では要件 2.4〜2.8 の場面は起きない（brief の見立てどおり）。要件 1 の測りには、`areka.exe`＋中継＋winget の控えのファイル（`Areka.Areka.Portable_*.db`）を並べた検体のフォルダを `target\` の下に組む手順が別に要る。この道具は変えなくてよい見込み。

## 3. 箱（MSIX の書き込みの振り替え）の調べ

### 3.1 文書で約束されていること

- Windows 10 1903 以降、仮想化される（virtualized）パッケージのアプリが `AppData` の下に**新しく作る**ファイルとフォルダは、利用者ごと・アプリごとの専用の場所へ書かれ、実行時に本当の `AppData` に在るように重ねて見せられる。対象は `Local`・`Local\Microsoft`・`Roaming` などの下。ファイルを開くときは専用の場所を先に見て、無ければ本当の `AppData` から開く。本当の場所から開いたファイルは振り替えない（既にあるファイルの書き換えは本当の場所に届く）。
  - https://learn.microsoft.com/en-us/windows/msix/desktop/desktop-to-uwp-behind-the-scenes （"AppData operations on Windows 10, version 1903 and later" と "Common file system operations" の表）
  - https://learn.microsoft.com/en-us/windows/msix/desktop/flexible-virtualization （"Default MSIX behavior" の表の AppData の行: 読みは専用の場所を先に試し、無ければ本当の `AppData` へ戻る）
- 振り替えを止める手は、パッケージの宣言（`AppxManifest.xml`）に書く 3 つだけ: 制限付きの能力 `unvirtualizedResources` と、`desktop6:FileSystemWriteVirtualization`（`disabled`）、Windows 11 の `virtualization:ExcludedDirectories`（止めるフォルダを名指し）。どれも**パッケージの作り手（ここでは Claude アプリ）が宣言するもの**で、起こされた子のプロセスが自分で選ぶ口は文書に無い。
  - https://learn.microsoft.com/en-us/windows/msix/desktop/flexible-virtualization
- パッケージのデスクトップ アプリの子のプロセスを、「デスクトップ アプリの実行環境」の中で起こすか外で起こすかは、プロセスを作る側が `PROC_THREAD_ATTRIBUTE_DESKTOP_APP_POLICY` で選ぶ。方針を何も決めていないプロセスの既定は「子を外で起こす」。「子を中で起こす」（`…_DISABLE_PROCESS_TREE`）は子孫へ受け継がれ、`…_ENABLE_PROCESS_TREE` で作り直されるまで続く。
  - https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute （Remarks の `PROC_THREAD_ATTRIBUTE_DESKTOP_APP_POLICY` の値）
- パッケージの名乗りを持つかは `GetCurrentPackageFullName`（kernel32）で調べ、名乗りが無ければ `APPMODEL_ERROR_NO_PACKAGE`（15700）が返る。
  - https://learn.microsoft.com/en-us/windows/msix/detect-package-identity
  - https://devblogs.microsoft.com/insidemsix/how-to-tell-if-a-process-is-packaged/
- 文書に**書かれていない**こと: (a) 名乗りを持たないのに実行環境の中で動く子のプロセスがどう扱われるか、(b) 振り替え先のフォルダの名前（文書は「専用の場所」とだけ言い、`Packages\<ファミリー名>\LocalCache\Local\` という形は約束していない）、(c) `GetFinalPathNameByHandle` が振り替え先の場所を返すこと。

### 3.2 この机での実測（2026-10-29・読むだけ・何も作っていない）

Claude アプリ（Store 版・4.29.0）の中のこのセッションで、2 つの道具から同じ場所を読んだ。

| 測ったこと | PowerShell の道具から | Bash の道具から（とその子の `powershell.exe`・`cmd.exe`） |
|---|---|---|
| `%LOCALAPPDATA%\areka` が在るか | 無い | **在る**（中に `mcp-bridge`） |
| 箱（`%LOCALAPPDATA%\Packages\Claude_pzs8sxrjxfjjc\LocalCache\Local\areka\`） | 在る（中に `mcp-bridge`） | — |
| `GetCurrentPackageFullName`・`GetCurrentPackageFamilyName` | — | **どちらも 15700（パッケージ無し）** |
| `%LOCALAPPDATA%\areka` を開いて `GetFinalPathNameByHandleW` | — | `\\?\C:\Users\<利用者>\AppData\Local\Packages\Claude_pzs8sxrjxfjjc\LocalCache\Local\areka` |
| `%LOCALAPPDATA%` そのものを同じく | — | `\\?\C:\Users\<利用者>\AppData\Local`（本当の場所） |

読み取れること:

1. 本当の `%LOCALAPPDATA%\areka` は無い。箱の中には、前に中継（`AREKA_MCP_HOME` の形）が作った `areka\mcp-bridge` がある（完了 `mcp-stdio-bridge` の実測の残り）。
2. 振り替えを受けるプロセスからは、箱の中の物が本当の場所に在るように**見える**（読みも重ねて見せられる）。文書の「重ねて見せる」のとおり。要件 1.1 (d)「箱の中から本当の置き場が在ると見えるか」は、文書の上では「見える（読みは本当の場所へ戻る）」。実測でまだ確かめていないのは「本当の場所だけに在る物」が見えるか。この机では本当の `%LOCALAPPDATA%\areka` が無いので測れなかった。→ 11.3 の S2 で測った: 見える。
3. **振り替えを受けるのに、パッケージの名乗りを持たないプロセスがある**。Bash の道具の子は名乗りが無いのに振り替えられていた。中継も Claude アプリが起こす子なので、同じく名乗りを持たない見込みがある。この場合、名乗りで「箱の中か」を見分ける手は見落とす（要件 3.5 の行が 0 件になる）。中継そのものでの確かめは要件 1 の測りで行う。
4. 振り替えられたフォルダを開いて最終の場所を尋ねると、箱の中の本当の場所が返る。名乗りが無くても返った。Rust の `std::fs::canonicalize` は Windows では `CreateFile`＋`GetFinalPathNameByHandle` を使うので（標準ライブラリの説明に書かれている）、依存を足さずに同じことができる見込み。中継の exe での確かめは要件 1 の測りで行う。
5. PowerShell の道具は振り替えを受けない。Claude アプリがこの道具を実行環境の外で起こしていると読める。完了 `user-data-root` の実測（PowerShell からは逸らされない）と合う。

### 3.3 箱の外の本当の置き場へ届く手（要件 4.1 の調べ）

| 手 | 依存を足さない | Claude の側を変えない | 文書の約束の内 | 結論 |
|---|---|---|---|---|
| R1. パッケージの宣言で振り替えを止める（`unvirtualizedResources`＋`FileSystemWriteVirtualization`／`ExcludedDirectories`） | ○ | **×**（Claude アプリの宣言） | ○ | 要件 4.2 を満たさない |
| R2. 本当の場所に既にあるファイルを、その場で書き換える（本当の場所から開いたファイルは振り替えない） | ○ | ○ | ○（既にあるファイルの書き換え） | ファイルが先に本当の場所に在ることが要る（areka 本体か、箱の外で動いた中継が作る）。一時の名前に書いて `rename` する今の置き換えは「新しいファイルを作る」ので箱へ行く。その場で書くと、控えがいつも答えの全体であること（`mcp-stdio-bridge` 要件 3.11）を崩す。要件 6.1・6.2 に当たる |
| R3. 利用者ごとの置き場を `AppData` の外に置く（`AREKA_USER_ROOT` を `D:\areka` などへ） | ○ | ○（利用者の設定の `env`） | ○（`AppData` の外の書きは振り替えない） | 既定の形では使えない。areka と中継の両方に同じ値を渡す必要がある。説明書で選べる手として伝えるかは討議 2 |
| R4. 控えを areka 本体に書いてもらう（MCP 越し） | ○ | ○ | ○ | 中継の MCP の働きと areka 本体を変える。範囲外（要件 6.1・6.2） |
| R5. 別名のパス（`\\?\GLOBALROOT`・`\\localhost\C$`・短い名前・ジャンクション）で振り替えをくぐる | ○ | ○ | **×** | 文書に無い振る舞いに頼る。採らない |
| R6. 中継が助けのプロセスを `PROC_THREAD_ATTRIBUTE_DESKTOP_APP_POLICY`（`…_ENABLE_PROCESS_TREE`）付きで起こし、その子（実行環境の外）に控えを書かせる | ○（`windows` の `Win32_System_Threading` はワークスペースで既に有効） | ○ | △ | API は文書にある。ただし、受け継いだ「中で起こす」方針の下に居るプロセスが、孫を外へ出せるかは文書から読み切れない。プロセスが 2 段増え、標準入出力の受け渡しか書くたびの起こし直しが要る。規模が S から外れる |

まとめ: 要件 4.2 の 3 条件を**はっきり満たす手は無い**。R6 だけが条件の文面に触れずに済むが、確かさが「文書から読み切れない」で、作りが重い。要件の既定（箱の中に置くことを受け入れる・要件 4.3）に倒すのが今の材料では自然。R6 を測りの対象に入れるかは討議 1。

測りで足したこと（11.3・M-1 の S2）: 本当の `%LOCALAPPDATA%\areka\profile\areka\` が先に在ると、Store 版の Claude Desktop から起こした中継は、`mcp-bridge\` のフォルダも、一時の名前から `rename` で置く `server-info.json`・`tools.json` も、**新しく作る物まで本当の場所へ**書いた（箱の中には何もできない）。R2 の「新しいファイルは箱へ行く」は、親のフォルダが本当の場所に在る場面では当たらない。つまり、areka 本体が箱の外で先に根を作っていれば、中継は今の作りのまま箱の外へ届く。届かないのは根がまだ無い場面（S1）だけで、そこで中継が根を作ると、その根は箱の中にできる。これは「中継は作らず決めるだけ」（要件 2）の向きを裏づける。R1〜R6 の結論は変えない。

### 3.4 箱の中で動いていると中継が知る手（要件 3.5・3.6）

| 手 | 依存 | 見落とし | 箱の場所が分かるか | 備考 |
|---|---|---|---|---|
| D1. `GetCurrentPackageFamilyName`（kernel32） | `windows` の機能 `Win32_Storage_Packaging_Appx` を中継のクレートで足す（`windows` 0.62.2 は `Cargo.lock` に既にある＝新しいクレートは 0。中継は今 `windows` に依存していない）。または `extern "system"` の宣言 1 つ | **名乗りの無い子では見落とす**（3.2 の 3） | ファミリー名から `%LOCALAPPDATA%\Packages\<ファミリー名>\LocalCache\Local` を組める。ただしこの形は文書の約束の外 | 名乗りを持つ起こされ方なら確か |
| D2. 既にある物を `std::fs::canonicalize` し、`%LOCALAPPDATA%` の下の綴りと違う場所（`…\Packages\…\LocalCache\Local\…`）が返るか見る | 無し | 箱の中にまだ何も無い（初めての起動・控えがまだ無い）と見分けられない。本当の場所に在る物（areka が作った `R`）を尋ねても本当の場所が返るだけ | 分かる（返った場所そのもの） | OS の約束は `GetFinalPathNameByHandle` の働きまでで、振り替えの下での答えは文書に無い |
| D3. D1 と D2 の重ね（名乗りがあれば D1、無ければ D2） | D1 と同じ | D2 と同じ場面 | 分かる | 見落としを減らす |
| D4. 試しに 1 つ書いて D2 で見る | 無し | 無い | 分かる | 「作る数 0」（要件 2.2）の趣旨に反する。箱の外では本当の `%LOCALAPPDATA%` に物を作る。採らない |
| D5. 親のプロセスの exe が `WindowsApps` の下か | `Win32_System_Diagnostics_ToolHelp` など | 起こし方しだい | 分からない | 文書の約束の外の見立て。採らない |

D2 の見落としの場面は、要件 3.5（箱の中なら 1 件）・要件 3.7（箱の外なら 0 件）のどちらにも当たらない第 3 の場面＝「分からない」になる。記録で「分からない」をどう扱うか（行を出さない・別の文で出す）は討議 3。

### 3.5 `windows` クレートの今の機能

- 根の `Cargo.toml` の `[workspace.dependencies.windows]`（58〜89 行・0.62）に、`Win32_Storage_Packaging_Appx`（`GetCurrentPackageFullName`・`GetCurrentPackageFamilyName`）と `Win32_Storage_FileSystem`（`GetFinalPathNameByHandleW`）は**入っていない**。`Win32_System_Threading`（`PROC_THREAD_ATTRIBUTE_DESKTOP_APP_POLICY`）は入っている。`PROCESS_CREATION_DESKTOP_APP_BREAKAWAY_*` の定数は `Win32_System_WindowsProgramming` にあり、これも入っていない。
- ほかのクレートは、使う機能だけを自分の `Cargo.toml` で足している（例: `crates/areka-update/Cargo.toml` 20〜24 行）。中継は `windows` に依存していない。
- 前例: 完了 `mcp-stdio-bridge` は `Cargo.lock` に既にあった `httparse`・`sse-stream` を中継の直接の依存にし、「新しい外部のクレートは 0 個」と数えた（`tech.md` の rmcp の項）。同じ数え方なら、D1 で `windows` を中継の依存に足しても要件 6.3 の「0」に入る。ただしそう数えてよいかは討議 3 で確かめる。
- D2 は標準ライブラリだけで書ける。

## 4. 要件と今の資産の対応

| 要件 | 使える資産 | 足りない物・制約 | 印 |
|---|---|---|---|
| 1.1〜1.3 実機の測り | 完了 `mcp-stdio-bridge`／`user-data-root` の実測の手順・3.2 の読むだけの調べ | winget の形の検体（`areka.exe`＋中継＋`Areka.Areka.Portable_*.db`）を `target\` の下に組む手順。Claude Desktop の設定を検体の中継へ向け直すのは開発者 | Missing |
| 1.4 測りの根は `target\` の下・`AREKA_USER_ROOT` を `target\` へ向けた形を先に | — | **`target\` は `AppData` の外なので、そこへ向けると振り替えが起きない**。箱の働き（1.1 の a・b・c）を測るには、既定の `%LOCALAPPDATA%\areka` を使う形が要る | Constraint |
| 1.5 本物の `%LOCALAPPDATA%\areka`・箱の中の控えの控え | — | この机の箱には `LocalCache\Local\areka\mcp-bridge` が既にある。箱の中の残りは、直した中継の判断（`R` が「見える」か）を変える。測る前に退けて、後で戻す手順が要る | Constraint |
| 1.6 開発者に頼む | — | 人の待ち（Claude Desktop の設定の向け直しと起動し直し） | Constraint |
| 2.1 決め方の規則を変えない | `probe`・`decide`・`app_profile_dir` | — | — |
| 2.2・2.3 作らない・写さない | `establish_readonly` は作らず写さない | `config.rs` 67 行の `establish` を外す。**`cache.rs` の `create_dir_all` が残るので、2.4・2.5 では `cache_dir = None` が要る** | Missing |
| 2.4 `R` が無い→控え無し | `establish_readonly` の判定（`root.dir.is_dir()`） | `Layout` からは「控えた」と「決められなかった」が見分けられない | Missing |
| 2.5 写しがまだ→控え無し | `migrate_source`（`pub(crate)`） | クレートの外から呼べない。中継で同じ条件を書き写すと持ち主が 2 つ | Missing |
| 2.6〜2.10 | `from_values` の段 | 2.7（exe の隣に書かない）は 2.4・2.5 を `None` にすれば満たす | — |
| 2.11 起動のときに 1 回 | 今の作り（`from_env` は 1 回） | — | — |
| 2.12 控えが無くても答え方を変えない | 完了 `mcp-stdio-bridge` の組み込みの答え | `cache_dir = None` の道は今もある（要件 6.3 of `user-data-root`） | — |
| 3.1 置き場の行 | `main.rs` 71〜73 行 | — | — |
| 3.2 控えを持たない理由の行（info） | — | 新しい行。今は `None` のとき `warn!` 1 件（80〜84 行）＝重さと文を分ける | Missing |
| 3.3 決められない（warn） | `from_env` の `warn!` | `Fallback` の記録は今 `decide_logged` の error（`user_root_unavailable`）。`decide` を直に呼ぶと、この記録が落ちる | Constraint |
| 3.4 書けない（warn） | `cache.rs` 69 行 | — | — |
| 3.5〜3.7 箱の行 | — | 見分ける手が無い（3.4 の D1〜D3）。名乗りでは見落としうる | Missing / Unknown |
| 3.8 ASCII | 今の記録の文 | `areka-user-data` の記録の文に "read-only run" が入る（`establish_readonly` を使う場合） | Constraint |
| 4.1〜4.4 | 3.3 の調べ | R6 の確かさ | Unknown |
| 5.1〜5.5 説明書 | 308 行の段落 | 289・297 行をどう扱うか。v0.0.3 で配った中継（`establish` を呼ぶ）が箱の中に作った写しの残りの扱い | Constraint |
| 6.4〜6.7 テスト | `establish_readonly_tests.rs` の型（`Signals` を組む・`tree` で比べる）・`temp-path-kit` | 中継に内側の口（`Signals` を受ける）と `log-capture-kit` の開発用の依存 | Missing |

## 5. 実装の案

### 案 A: 中継の `config.rs` だけで直す（`areka-user-data` は読むだけ）

- `from_env` で `probe` → `decide` を直に呼び、`UserRoot` に当たったら中継で 3 つの在る・無いを見る（`R` がフォルダか・`R\profile\areka` が在るか・`X\profile\areka` がフォルダか）。2.4・2.5 なら `cache_dir = None` と info の行。
- 良い点: 触るのは中継のクレートだけ。brief の「`areka-user-data` は読むだけ」に合う。
- 悪い点: 写しの条件を中継で書き写す（`migrate_source` の「持ち主は 1 つ」に反する）。`decide_logged` の記録（書けるか分からない warn・決められない error）と `user_data_layout` の行を中継で作り直すか、落とすことになる。落とすと「ログの無い失敗の道」が増える。

### 案 B: `areka-user-data` に「決めるだけ」の口を 1 つ足し、中継はそれを呼ぶ

- 例: `decide_only(_from)(signals, exe_dir, profile_env)` が `decide_logged` を分け合い、「`R` が無い」「写しがまだ」「使える」「隣の形」「決められない」を区別した答えを返す。`migrate_source` を使う判定はクレートの中に留まる。
- 良い点: 写しの条件と判断の記録の持ち主が 1 つのまま。中継のテストは答えの種類ごとに書ける。areka 本体の振る舞いは変わらない（新しい口は areka 本体から呼ばれない）。
- 悪い点: `areka-user-data` のファイル（`establish.rs`＝310 行・`lib.rs`）とテストに手が入る。brief の触るファイルの見立てより 1〜2 本増える。並走の重なり（brief では 0）を着手時に照らし直す必要がある。

### 案 C: 重ね（`migrate_source` を公開するだけ・判断は `establish_readonly_from` を使う）

- `migrate_source` を `pub` にし、中継は `establish_readonly_from` で形を決めてから、`Reason` と `R` の有無と `migrate_source` で 2.4・2.5 を見分ける。
- 良い点: `areka-user-data` の変化は公開の 1 語。
- 悪い点: `establish_readonly` は "read-only run" の記録を出す（中継の記録として誤解を招く）。`Layout` からは「控えた」と「決められなかった」を区別できず、中継で `R.is_dir()` を見直すことになる（判定が 2 か所）。

### 箱の行（要件 3.5〜3.7）の足し方

どの案でも、中継の `config.rs`（または新しい小さなファイル）に「箱の中かを見る口」を 1 つ足し、`from_env` から呼んで結果を記録する形になる。手は 3.4 の D1・D2・D3 から選ぶ。テストでは「見た結果」を値で注入し（要件 6.6・6.7）、本物の OS を呼ぶ部分は薄く保つ。

## 6. 規模とリスク

- **規模: S（2〜3 日）**。直しのコードは中継の `config.rs` と `main.rs` の記録の行、案 B なら `areka-user-data` に口 1 つ。説明書の段落 1 つ。手間の大半は実機の測り（直す前・直した後）と、その人の待ち。R6 を採ると M 以上。
- **リスク: 中**。コードの直しは今の型に沿うので低い。不確かさが 2 つ残る: (1) 中継が Claude アプリから起こされたときにパッケージの名乗りを持つか（箱の見分けの手の選びが変わる）、(2) 測りで本物の `%LOCALAPPDATA%` と箱の中の残りに触れる手順（箱の働きは `AppData` の下でしか起きないため）。

## 7. 設計へ持ち越す調べ（Research Needed）

1. Claude アプリ（Store 版）が起こした中継の exe が、パッケージの名乗りを持つか（`GetCurrentPackageFamilyName` の答え）。3.2 の Bash の子は持たなかった。要件 1 の最初の測りで、中継の起動の記録に一時的に出して確かめる。
2. 中継の exe で `std::fs::canonicalize` が箱の中の場所を返すか（3.2 は PowerShell から `GetFinalPathNameByHandleW` を直に呼んで確かめた）。
3. 箱の中から、本当の場所にだけ在る `R`（areka が箱の外で作った物）が見えるか（要件 1.1 の d）。この机には本当の `R` が無いので、測りで作る必要がある（要件 1.5 の控えと戻し）。→ 11.3 の S2 で測った: 見える。
4. 箱の中の残り（v0.0.3 の中継や前の測りが作った `LocalCache\Local\areka\…`）が、直した中継の判断を変える場面の洗い出し。重ねて見せられるので、本当の `R` が無くても箱の中の `areka` があれば「`R` が在る」と見える。
5. R6（実行環境の外で子を起こす）を確かめるか。確かめるなら、受け継いだ方針の下の子孫が `…_ENABLE_PROCESS_TREE` で外へ出られるかを、`target\` の下の検体で測る。
6. winget の本物の置き場（`%LOCALAPPDATA%\Microsoft\WinGet\Packages\…`）は `AppData\Local\Microsoft` の下で、振り替えの対象に入る。箱の中の中継から exe の隣への書き込みも箱へ行く。要件 2.7 で exe の隣には書かないので直接は効かないが、説明書の例と測りの検体の置き場（`target\` は `AppData` の外）の違いとして記録に残す。

## 8. 要件の討議に出す決め事

1. **箱の外へ届く手（要件 4）**: 3.3 の R1〜R6 のうち、要件 4.2 をはっきり満たすものは無い。既定どおり「箱の中に置くことを受け入れる」で決めてよいか。R6（実行環境の外で子を起こす）を測りの対象に入れるか（入れると規模が上がる）。
2. **`AppData` の外に置く手を説明書で伝えるか（R3）**: `AREKA_USER_ROOT` を `AppData` の外へ向ければ、Store 版でも控えは箱に入らない。説明書に「選べる手」として 1 行足すか（要件 5 の範囲に入るか）。
3. **箱の見分け方（要件 3.5〜3.7）**: D1（名乗り）・D2（最終の場所を尋ねる）・D3（重ね）のどれにするか。名乗りの無い子で D1 が見落とすおそれ（3.2 の 3）と、D2 が初めての起動で「分からない」になること。「分からない」ときの記録をどうするか（行を出さない・別の文で 1 件出す）。D1 で `windows` を中継の依存に足すのを、要件 6.3 の「新しい外部依存 0」に数えてよいか（`Cargo.lock` には既にある）。
4. **`Fallback`（利用者ごとの置き場を使う形だが場所を決められない）のときの控え**: 今は exe の隣の `profile\areka\mcp-bridge`（areka 本体と同じ所）。要件 2.1（答えを変えない）に従って今のままにするか、要件 2.7 に寄せて控え無し（要件 3.3 の warn）にするか。
5. **`areka-user-data` に手を入れるか（案 A／B／C）**: brief は「読むだけ」と見立てたが、`migrate_source` の持ち主を 1 つに保つには、公開するか口を足すことになる。中継で条件を書き写す案 A を許すか。
6. **記録の数え方**: 今の `None` のときの `warn!`（`from_env` 80〜84 行）と、要件 3.2 の info の行・要件 3.1 の info の行の関係。2.4・2.5 の場面で warn を出さず info 1 件にするか。`areka-user-data` が出す `user_data_layout` などの行を、中継の記録に今どおり残すか（案 A では消えうる）。
7. **説明書の「段落」の範囲（要件 5.5）**: 中継の控えの場所を書く行は 289・297・308 行の 3 か所。変えてよいのを 308 行の段落だけにするか、297 行（利用者ごとの置き場の形の一覧の 1 行）も含めるか。
8. **v0.0.3 の中継が箱の中に作ったかもしれない写しの扱い**: v0.0.3（10-18 公開）の中継は `establish` を呼ぶので、Store 版の Claude Desktop と winget の areka を使う人の箱の中には、利用者ごとの置き場と記憶の写しが既にあるかもしれない。直した中継は消さない（要件 2.3 の「消す数 0」）。説明書で場所と「消してよい」ことを伝えるか。
9. **測りの形（要件 1.4）**: `AREKA_USER_ROOT` を `target\` の下へ向けると振り替えが起きず、箱の働き（要件 1.1 の a〜c）は測れない。箱の働きは既定の `%LOCALAPPDATA%\areka` で測り、その前後で本物の置き場と箱の中の残りを控え・戻す（要件 1.5）ことにしてよいか。`AREKA_USER_ROOT` の形は「作らない・写さない」の確かめにだけ使う。
10. **要件 1.1 の「Store 版の Claude Desktop から起こす」の代わり**: 3.2 のとおり、このセッションの Bash の道具の子も振り替えを受ける。設定の向け直しと起動し直し（人の待ち）の前に、Bash から検体の中継を起こして下見をしてよいか。本番の測りは要件どおり Claude Desktop から起こす。

## 8b. 設計フェーズへ回す事項（要件の討議で分けたもの）

1. `areka-user-data` に足す「決めるだけ」の口の形（名前・答えの型で「置き場がまだ無い」「写しがまだ」「使える」「隣の形」「決められない」を分ける・`decide_logged` と `migrate_source` をクレートの中で分け合う）。
2. 要件 2.4・2.5 の場面で `cache_dir = None` にする所（`cache.rs` の `replace` の `create_dir_all` が置き場を作らないように）。
3. 箱の見分けの手（3.4 の D1〜D3）。`windows` クレートを中継に足さずに書ける形（`GetCurrentPackageFamilyName` の `extern "system"` の宣言・`std::fs::canonicalize`）を先に考える。調べの 1・2 の測りの結果で決める。
4. 記録の行の並べ方（今の `None` のときの `warn!` と、要件 3.1・3.2 の info の行の分け・`areka-user-data` が出す `user_data_layout` などの行を中継の記録に残すか）。
5. テストの組み方（`Signals` と exe のフォルダと `AREKA_PROFILE_DIR` を受ける内側の口・`establish_readonly_tests.rs` の `tree` の比べ・`log-capture-kit` を開発用の依存へ）。
6. 7 の調べの 1〜4 と 6（中継の exe の名乗り・`canonicalize` の答え・重ね見せ・箱の残りが判断を変える場面・winget の置き場も振り替えの対象）。
7. 測りの検体（`areka.exe`＋中継＋winget の控えの `Areka.Areka.Portable_*.db`）を `target\` の下に組む手順。

## 9. 次の段

- 要件の討議（`/kiro-requirements-discussion bridge-store-user-root`）で 8 の決め事を決める。
- 設計（`/kiro-design bridge-store-user-root`）では、案 A／B／C のうち討議 5 で選んだ形と、箱の見分けの手（討議 3）を、7 の調べの 1・2 の測りの結果で確かめてから固める。

## 10. 設計の段の調べと決めたこと（2026-10-29・`/kiro-spec-design -y`）

> 入力は要件（討議で決めた 7 点を含む）・この文書の 1〜8b・steering（`product.md`・`tech.md`・`structure.md`・`logging.md`）・ワークツリーの先頭 `29f6ff2c` のコード。使った手引きは `kiro-spec-design` の `design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`・`design-principles.md`、steering の `logging.md`（`[bridge]` の頭・構造化の欄・テストの捕捉は `log-capture-kit`）と `structure.md`（テストは兄弟のファイル・1 ファイル 1,000 行・共有のヘルパは `<stem>_test_support.rs`）。

### 10.1 調べの種類

- **軽い調べ（拡張）**。直す所は既存の 2 つのクレート（`areka-mcp-bridge`・`areka-user-data`）と説明書の 2 か所。新しい外部依存は無い。外の文書は 3 章で読み終えている（足して読んだものは無い）。

### 10.2 コードで確かめたこと

- `areka-user-data` の口を呼ぶのは、areka 本体では `crates/areka/src/roots/user_root.rs`（`establish`／`establish_readonly` の選び分け）と `crates/areka/src/boot_config.rs`（`app_profile_dir`／`readonly_app_profile_dir`）だけ。中継は `crates/areka-mcp-bridge/src/config.rs` の `from_env` だけ。新しい口を足しても、この 3 か所は 1 行も変わらない（要件 6.1）。
- `decide_logged`（`establish.rs`・非公開）は、`Decision::Beside` と `Decision::Fallback` のどちらでも `(None, reason)` を返す。`decide` は `Beside` の理由をいつも `Reason::Neither` にし、`Fallback` の理由は決して `Neither` にしない（`lib.rs` の `decide` の本文と `Decision::Beside` の注記）。新しい口はこの決まりで 2 つを見分けられ、`decide_logged` を変えずに済む。
- `Config::from_values` の段は `profile_dir.or(beside_profile).or(local)`。隣に `areka.exe` が在って控えを持たない場面で `beside_profile` を `None` にすると、段が `LOCALAPPDATA\areka\mcp-bridge` へ落ち、最初の控えの書き（`cache.rs` の `replace` の `create_dir_all`）が `%LOCALAPPDATA%\areka` そのものを作る。要件 2.2・2.4 を守るには、「隣に areka が居ない」と「隣に areka が居て控えを持たない」を `from_values` の入力の形で分ける必要がある（2 章 2.2 の穴の続き）。
- `from_values` を呼ぶのは `config.rs` の `from_env` とテスト（`config_tests.rs`）だけ。形を変えても外へは響かない。
- `tests/exe_test.rs` は `AREKA_PROFILE_DIR` を `target` の下へ向けて建った exe を起こす。直した後もその形のまま動く（控えは `AREKA_PROFILE_DIR` の下）。52〜53 行の注記（「そこを作るので外す」）は古くなるので書き直す。
- 中継の `Cargo.toml` の開発用の依存は `temp-path-kit` と `tokio` だけ。記録の行のテスト（要件 6.6）には `log-capture-kit`（ワークスペースの中のクレート）を足す。外部依存は 0 のまま（要件 6.3）。
- areka 本体は `mcp-bridge` のフォルダを作らない（`crates/areka/src` に `mcp-bridge` を作る道は無い）。控えのフォルダを作るのは中継だけなので、控えのフォルダの最終の場所を尋ねれば「中継の書きがどこへ行ったか」が分かる。

### 10.3 決めたこと

| # | 決めたこと | 理由 | 捨てた案 |
|---|---|---|---|
| D-1 | `areka-user-data` に口を 1 つだけ足す: `locate_app_profile(signals, exe_dir, profile_env) -> AppProfile`（`establish.rs`）と答えの型 `AppProfile`（`lib.rs`）。答えは `Ready(記憶の置き場)`・`RootMissing { root }`・`MigrationPending { root }`・`Unlocatable` の 4 つ | 討議 5。写しの条件（`migrate_source`）と判断の記録（`decide_logged`・`log_layout`）を `establish.rs` の中に置いたまま使える。`_from` の対を作らず、呼び手が公開の `probe` で入力を作って渡す（口は 1 つ） | 案 A（中継で条件を書き写す）・案 C（`establish_readonly_from` の重ね・"read-only run" の記録が出る） |
| D-2 | `Ready` の値は `app_profile_dir` で出す（`AREKA_PROFILE_DIR` が在ればそれ・在る書ける根の `profile\areka`・隣の形なら exe の隣の `profile\areka`） | 「`profile\areka`」の綴りの持ち主を `app_profile_dir` 1 つに保つ。答えが今と同じ（要件 2.1） | 中継で `join` する |
| D-3 | `AREKA_PROFILE_DIR` が在れば、根が無くても写しがまだでも `Ready(その値)` | 要件 2.9（4〜8 に関わらず）。`establish` も `AREKA_PROFILE_DIR` が在れば写さない | — |
| D-4 | `Config::from_values` の 3 つ目の引数を `Option<PathBuf>` から `Option<AppProfile>` に替える。`None`＝隣に `areka.exe` が無い（今どおり `LOCALAPPDATA` の段へ）・`Some(Ready)`＝その下・`Some(ほか)`＝控え無し（`LOCALAPPDATA` の段へ落ちない） | 10.2 の 3 つ目の穴。型で分けるので、落ちる道が書けない | `bool` を 1 つ足す・中継に別の列挙を作る |
| D-5 | `Config::from_env` の内側に `from_inputs`（値だけを受け、`locate_app_profile`→`from_values`→控えを持たない理由の記録）を置く。`from_env` は本物の環境・exe の場所・`probe` を読むだけ | 要件 6.7（注入）。`establish_readonly_from` と同じ分け方 | — |
| D-6 | 箱の見分けは、控えのフォルダの最終の場所を `std::fs::canonicalize` で尋ねるだけ（3.4 の D2）。パッケージの名乗り（D1・`extern "system"` の宣言）は使わない | 3.2 の 3（名乗りの無い子でも箱へ回される）で D1 は「箱の外」の証にならず、「箱の中」の証も初めての起動の 1 回分しか増やさない。`unsafe` の宣言を足さずに済む。D2 は標準ライブラリだけ | D1・D3（D1＋D2）・D4・D5 |
| D-7 | 見立ては 3 つ: 控えのフォルダが `LOCALAPPDATA` の外の場所に解けた＝箱の中（解けた場所から `LOCALAPPDATA` の下の相対の部分を除いた所を箱のフォルダとして出す）・本当の場所に解けた＝箱の外（行 0 件）・控えが無い／まだ無い／`LOCALAPPDATA` の下でない／解けない＝見分けられない（info 1 件） | 要件 3.5〜3.7・3.9。箱のフォルダの形（`Packages\…\LocalCache\Local`）を決め打ちしない（文書の約束の外） | `Packages\<ファミリー名>\LocalCache\Local` を組む |
| D-8 | 控えを持たない理由の行は info 1 件（`reason` の欄に `user_root_missing`／`app_memory_not_migrated`・`user_root` の欄に場所）。場所を決められないときは warn 1 件（文を新しく）。どれにも当たらない今の「置き場が無い」は今の warn のまま | 討議 2・3。今の `warn!` 1 件を、場面ごとに重さと文を分ける | 2.4・2.5 も warn |
| D-9 | `areka-user-data` が出す記録（`user_data_layout`・書けるか分からない warn・決められない error）は、中継の記録にも今どおり残す | 今の中継の記録（`establish` 経由）と同じ行が残る。測りで決めた形が読める | 中継の記録から外す |
| D-10 | 箱の見立ては `from_env` の最後に 1 回（新しい兄弟のファイル `redirect.rs` の `observe`→`report`） | 要件 2.11（起動のときに 1 回）。`config.rs` を太らせない。OS に触る所は `canonicalize` の 2 回だけ | `main.rs` で呼ぶ |
| D-11 | 実機の測りの記録は、この文書の新しい節（11 章）に書く | 要件 1.1〜1.3「spec の記録」。ファイルを増やさない | 別のファイル |

### 10.4 合成（generalization・build vs adopt・simplification）

- **まとめられること**: 要件 2.4・2.5・2.7（`Unlocatable`）は、どれも「隣に areka が居るが控えを持たない」の言い換え。`AppProfile` の `Ready` 以外の 3 つとして 1 つの型にまとめ、`from_values` はそれを「控え無し」の 1 つの腕で受ける。記録の文だけが 3 つに分かれる。
- **作るか借りるか**: 箱の最終の場所は OS の `GetFinalPathNameByHandle` を `std::fs::canonicalize` で借りる（Windows では `CreateFile`＋`GetFinalPathNameByHandle`）。`windows` クレートも `extern` の宣言も足さない。記録の捕捉は既存の `log-capture-kit`、一時の根は既存の `temp-path-kit`。
- **削ったもの**: `locate_app_profile_from`（`_from` の対）・パッケージの名乗りの読み・箱のフォルダの組み立て・中継の新しい列挙（`BesideAreka` など）・測りの道具のスクリプトの登記（測りの手順は `target\` の下で使い捨て）。

### 10.5 残る危うさ

- **箱の外の取り違え（一度だけ）**: 箱の外で動いた中継（PowerShell から起こしたなど）が本当の場所に控えのフォルダを作った後、箱の中の中継が初めて起こされると、本当の場所が見える（重ねて見せられる）ので「箱の外」と見立てる。その起動の書きで箱の中に写しができるので、次の起動からは「箱の中」になる。取り違えは 1 回の起動に限られる。コードに `shortcut:` の注記で残す。
- **ジャンクションの取り違え**: 利用者が `%LOCALAPPDATA%\areka` をジャンクションで別の場所へ向けていると、「箱の中」と見立てる。行には尋ねた場所と解けた場所の両方を出すので、読めば分かる。
- **中継の exe で `canonicalize` が箱の場所を返すか**: 3.2 は PowerShell から OS を直に呼んで確かめただけ。直した後の測り（要件 1.3）で、箱の行に箱のフォルダが出ることを確かめる。出なければ D-6 を見直す（設計に戻る）。
- **箱の中から本当の `R` が見えるか（要件 1.1 の d）**: 文書では見える。11.3 の S2 で、測っても見えた。見えなければ、Store 版の利用者は毎回 `RootMissing`（控え無し）になる。要件のとおりの振る舞いだが、説明書の文を測りに合わせて直す（要件 5.1）。

## 11. 実機の測り（2026-10-29〜）

### 11.1 M-1 の準備（直す前の検体・測る前の形・控え）

**検体**（要件 1.4）: ワークツリーの先頭 `58734aa37`（直しのコードの前）で `cargo build -p areka -p areka-mcp-bridge`（dev の建て方）を通し、`target\bridge-store-measure\before\` に並べた。

```
\areka.exe                         35232768  SHA256 0EF34B62D4470C7BAEFBE583FD43C859C8294A6D605E8CEDDDAF3130337F6A8F
\areka-mcp-bridge.exe               8949248  SHA256 C6675CE2C93C97F6E9FC1B409D53E27980C239723FA538139AC5EB80B55EB16B
\Areka.Areka.Portable_measure.db          0  (winget の控え・空)
\profile\areka\sylphya.toml              76  (exe の隣の記憶・中身は下)
```

exe の隣の記憶の中身（改行は LF）と SHA256:

```
# bridge-store-user-root measure specimen (M-1)
[measure]
marker = "before"
```

`SHA256 EB18050A6C0F99D8C2F68EAF9C04ED3C1B8F5DDF7C458F8D32FAED0F8DACC900`

**測る前の形**（PowerShell の道具＝箱の外の見え方で写した）:

- 箱のフォルダの名前は `Claude_pzs8sxrjxfjjc`（`%LOCALAPPDATA%\Packages\` の下に `Claude_` で始まるのはこれ 1 つ）。
- 箱の中 `C:\Users\<利用者>\AppData\Local\Packages\Claude_pzs8sxrjxfjjc\LocalCache\Local\areka\` の木（相対の名前・大きさ・最後に書いた時刻）:

```
\mcp-bridge                    <DIR>  2026-10-23 23:37:49
\mcp-bridge\server-info.json     625  2026-10-23 23:37:49
\mcp-bridge\tools.json          6807  2026-10-23 23:37:49
```

  中身の SHA256: `server-info.json` = `4F3F1CEC484AF2C800FD7CBD091BDA8B1D97015FFD830977D99EFD26CA24EEBA`、`tools.json` = `CFF68DF958D4CA3FAF00DEE1064A8A4F28AC07E630D0E6B54349FC43674E7989`。`profile\` は無い（前の版の中継が作る写しは、今の箱には無かった）。
- 本当の `C:\Users\<利用者>\AppData\Local\areka\` は無い（`Test-Path` が False）。
- 写した時点で動いていた中継は、開発者の机の本物（`C:\home\maz\store\.areka-mcp\areka-mcp-bridge.exe`、pid 12928 と 18580・Claude Desktop が起こしたもの）。止めていない。

**控え**（要件 1.5）: 箱の中の `LocalCache\Local\areka\` を PowerShell の道具から `target\bridge-store-measure\backup\box-LocalCache-Local-areka\` へ移した（木と SHA256 は上と同じことを移した後に確かめた）。本当の `%LOCALAPPDATA%\areka` は無いので控える物は無い。移した後、箱の中に `areka\` は無い。箱の `LocalCache\Local\` には `areka-diag` など areka の測りと関わらない物が在るが、触っていない。

**裏の書きの注意**: 箱の中の `areka\` を空けた後も、開発者の机の本物の中継（Claude Desktop が起こしたもの）は動き続けている。この中継が areka へつなぎ直すなどして控えを書くと、箱の中に `areka\mcp-bridge\` がまた現れうる。この後の測りでは、起こす直前と直後に箱の木を取り、検体の中継が作った物と分けて、本物の中継が書いた物は「裏の書き」として別に数える。戻す（手順 8）ときは、`backup\` の控えを元の場所へ戻す前に、裏の書きでできた物が在れば記録してから片付ける。

### 11.2 M-1 の下見と PowerShell から起こした比べ（2026-10-29 22:51）

**共通の形**: 検体は 11.1 の `before\` のもの（直す前）。流したのは次の 2 行で、流した後に標準入力を閉じた（`target\bridge-store-measure\m1-12\input.jsonl`）。

```
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"measure","version":"0"}}}
{"jsonrpc":"2.0","id":2,"method":"tools/list"}
```

- 起こす直前と直後に、PowerShell の道具（箱の外の見え方）から 4 か所の木を写して比べた: 箱の中の `areka\`、本当の `%LOCALAPPDATA%\areka\`、`before\user-root\`、検体のフォルダ `before\`（写しは `m1-12\preview-pre.txt`・`preview-post.txt`・`compare-pre.txt`・`compare-post.txt`、中身の SHA256 つき）。数えたのは直後に増えた物だけ。
- `RUST_LOG`・`AREKA_USER_ROOT`・`AREKA_PROFILE_DIR`・`AREKA_MCP_PORT` は、Bash の道具でも PowerShell の道具でも空だった（記録の重さは中継の既定のまま）。
- 測った間、areka は動いていなかった（`areka.exe` のプロセスは無い）。9801 と 9821 では SSP の MCP（`ssp-mcp-server`）が待ち受けていた。
- 中継は 2 回とも、標準入力を閉じた後に自分で終わった（終了コード 0・`[bridge] finished quit=Closed`）。標準出力には `id` 1 と 2 の答えが 1 行ずつ出た（組み込みの答え）。止めたプロセスは無い。
- 開発者の机の本物の中継（`C:\home\maz\store\.areka-mcp\areka-mcp-bridge.exe`・pid 12928 と 18580）は、どちらの測りの間も動き続けていた。止めていない。

#### 下見（Bash の道具から起こした・本番の代わりにはしない・要件 1.7）

起こし手は Bash の道具（箱の中へ回される・3.2）。22:51:11 に起こし、22:51:13 に終わった。

増えた物（箱の中 `C:\Users\<利用者>\AppData\Local\Packages\Claude_pzs8sxrjxfjjc\LocalCache\Local\areka\` の下）:

```
\                                (areka\ そのもの)
\.copy-work                <DIR> (空)
\profile                   <DIR>
\profile\areka             <DIR>
\profile\areka\sylphya.toml   76  SHA256 EB18050A6C0F99D8C2F68EAF9C04ED3C1B8F5DDF7C458F8D32FAED0F8DACC900
```

中継の記録には `user root created ... dir=C:\Users\<利用者>\AppData\Local\areka` と「作った」と出たが、本当の `%LOCALAPPDATA%\areka` は箱の外から見ると無いまま（`Test-Path` が False）。作った物はすべて箱の中へ回された。検体の隣の記憶は変わっていない（SHA256 も時刻も同じ）。`mcp-bridge\` はできていない（areka につながらなかったため・下の比べの (c) と同じ理由）。裏の書きは無かった（起こす直前の箱の中に `areka\` は無く、直後に増えた物に `mcp-bridge\` は無い）。

標準エラー（ASCII のまま・行の先頭の時刻 `2026-10-29T13:51:12.875051Z` などは省き、利用者名だけ置き換えた・元は `m1-12\preview-stderr.txt`）:

```
INFO areka_user_data::establish: user root created event="user_root_created" dir=C:\Users\<利用者>\AppData\Local\areka
INFO areka_user_data::establish: app memory migrated to the user root event="app_memory_migrated" from=C:\home\maz\git\areka\.claude\worktrees\kiro-start-bridge-store-user-root-d1760a\target\bridge-store-measure\before\profile\areka to=C:\Users\<利用者>\AppData\Local\areka\profile\areka files=1
INFO areka_user_data::establish: user data layout decided event="user_data_layout" mode="user_root" reason="winget_install" user_root=C:\Users\<利用者>\AppData\Local\areka origin="default" marker="Areka.Areka.Portable_measure.db"
INFO areka_mcp_bridge: [bridge] candidates on 127.0.0.1 ports=[9801, 9821, 9802, 9822, 9803, 9823, 9804, 9824, 9805, 9825, 9806, 9826, 9807, 9827, 9808, 9828, 9809, 9829, 9810, 9830]
INFO areka_mcp_bridge: [bridge] cache dir dir=C:\Users\<利用者>\AppData\Local\areka\profile\areka\mcp-bridge
INFO areka_mcp_bridge: [bridge] timeouts connect=300ms verify=3s reply=30s
INFO areka_mcp_bridge::scan: [bridge] skipped a candidate: it answered as another server port=9801 name="ssp-mcp-server"
INFO areka_mcp_bridge::scan: [bridge] skipped a candidate: it answered as another server port=9821 name="ssp-mcp-server"
INFO areka_mcp_bridge::service: [bridge] answered the first inquiry method="initialize" requested=2025-06-18 answered=2025-06-18
INFO areka_mcp_bridge::service: [bridge] areka not found; answered from the built-in definitions method="initialize"
INFO areka_mcp_bridge::scan: [bridge] skipped a candidate: it answered as another server port=9801 name="ssp-mcp-server"
INFO areka_mcp_bridge::scan: [bridge] skipped a candidate: it answered as another server port=9821 name="ssp-mcp-server"
INFO areka_mcp_bridge::service: [bridge] areka not found; answered from the built-in definitions method="tools/list"
INFO areka_mcp_bridge: [bridge] finished quit=Closed
```

**下見の後の戻し**: 箱の中にできた `areka\` を、PowerShell の道具から `target\bridge-store-measure\m1-12\preview-box-areka\` へ移した（証拠として残す・消していない）。移した後、箱の中に `areka\` は無く、11.1 の控えの後の形に戻った。本物の中継が書いた物は無かったので、分けて残す物も無い。

下見で分かったこと: 直す前の中継は、Bash の道具（箱の中）から起こすと、winget の印（`Areka.Areka.Portable_measure.db`）を見て利用者ごとの置き場の形と決め（`reason="winget_install"`）、箱の中に `areka\`・`profile\areka\` を作り、検体の隣の記憶を 1 つ写す。記録の上の場所は本当の `%LOCALAPPDATA%\areka` だが、実物は箱の中にある。0.0.3 の中継が箱の中に作った写しと同じ形が、検体でも起きた。

#### 比べ（PowerShell の道具から起こした・要件 1.2）

起こし手は PowerShell の道具（箱の外）。`AREKA_USER_ROOT` は、起こす子のプロセスにだけ `C:\home\maz\git\areka\.claude\worktrees\kiro-start-bridge-store-user-root-d1760a\target\bridge-store-measure\before\user-root\` を渡した（PowerShell 自身の環境には書いていない）。pid 37244、22:51:48 に起こし、22:51:52 に終わった。この pid に `GetPackageFamilyName` を尋ねると、答えは 15700（`APPMODEL_ERROR_NO_PACKAGE`＝パッケージの名乗りを持たない）だった。

4 点の表（下見の欄は比べのための参考）:

| 点 | 下見（起こし手: Bash の道具・箱の中） | 比べ（起こし手: PowerShell の道具・箱の外・`AREKA_USER_ROOT` あり） |
|---|---|---|
| (a) できたフォルダ | 箱の中: `areka\`・`areka\profile\`・`areka\profile\areka\`・`areka\.copy-work\` ができた。本当の `%LOCALAPPDATA%\areka`: 0。`user-root\`: 0 | 箱の中: 0。本当の `%LOCALAPPDATA%\areka`: 0。`before\user-root\` に `user-root\` そのもの・`profile\`・`profile\areka\`・`.copy-work\`（空）ができた |
| (b) 記憶の写し | `sylphya.toml` の写しは 1（箱の中の `areka\profile\areka\`・SHA256 は元と同じ）。検体の隣の記憶の SHA256 は `EB18050A…C900` のまま | `sylphya.toml` の写しは 1（`before\user-root\profile\areka\`・SHA256 は元と同じ）。箱の中と本当の置き場は 0。検体の隣の記憶の SHA256 は `EB18050A…C900` のまま |
| (c) `server-info.json`・`tools.json` | 書かれない（箱・本当の置き場・`user-root\`・検体の隣のどこにも無い）。理由: areka が動いておらず、9801・9821 は SSP が答え、ほかの番号は答えが無く、中継は組み込みの答えで答えた。控えは areka につながったときだけ書く。決めた控えの置き場は `C:\Users\<利用者>\AppData\Local\areka\profile\areka\mcp-bridge`（実物は箱の中へ回る先） | 書かれない（同じ 4 か所のどこにも無い）。理由は下見と同じ（areka が居ない）。決めた控えの置き場は `before\user-root\profile\areka\mcp-bridge` |
| (d) 中継の標準エラーの行 | `user_root_created`（dir=`C:\Users\<利用者>\AppData\Local\areka`）・`app_memory_migrated`（files=1）・`user_data_layout`（`mode="user_root" reason="winget_install" origin="default"`） | `user_root_created`（dir=`...\before\user-root\`）・`app_memory_migrated`（files=1）・`user_data_layout`（`mode="user_root" reason="env_var" origin="env_var" marker=""`） |
| 裏の書き | 無し | 無し（箱の中は起こす直前も直後も `areka\` が無い） |

比べの標準エラー（ASCII のまま・時刻を省いた・元は `m1-12\compare-stderr.txt`）:

```
INFO areka_user_data::establish: user root created event="user_root_created" dir=C:\home\maz\git\areka\.claude\worktrees\kiro-start-bridge-store-user-root-d1760a\target\bridge-store-measure\before\user-root\
INFO areka_user_data::establish: app memory migrated to the user root event="app_memory_migrated" from=C:\home\maz\git\areka\.claude\worktrees\kiro-start-bridge-store-user-root-d1760a\target\bridge-store-measure\before\profile\areka to=C:\home\maz\git\areka\.claude\worktrees\kiro-start-bridge-store-user-root-d1760a\target\bridge-store-measure\before\user-root\profile\areka files=1
INFO areka_user_data::establish: user data layout decided event="user_data_layout" mode="user_root" reason="env_var" user_root=C:\home\maz\git\areka\.claude\worktrees\kiro-start-bridge-store-user-root-d1760a\target\bridge-store-measure\before\user-root\ origin="env_var" marker=""
INFO areka_mcp_bridge: [bridge] candidates on 127.0.0.1 ports=[9801, 9821, 9802, 9822, 9803, 9823, 9804, 9824, 9805, 9825, 9806, 9826, 9807, 9827, 9808, 9828, 9809, 9829, 9810, 9830]
INFO areka_mcp_bridge: [bridge] cache dir dir=C:\home\maz\git\areka\.claude\worktrees\kiro-start-bridge-store-user-root-d1760a\target\bridge-store-measure\before\user-root\profile\areka\mcp-bridge
INFO areka_mcp_bridge: [bridge] timeouts connect=300ms verify=3s reply=30s
INFO areka_mcp_bridge::scan: [bridge] skipped a candidate: it answered as another server port=9801 name="ssp-mcp-server"
INFO areka_mcp_bridge::scan: [bridge] skipped a candidate: it answered as another server port=9821 name="ssp-mcp-server"
INFO areka_mcp_bridge::service: [bridge] answered the first inquiry method="initialize" requested=2025-06-18 answered=2025-06-18
INFO areka_mcp_bridge::service: [bridge] areka not found; answered from the built-in definitions method="initialize"
INFO areka_mcp_bridge::scan: [bridge] skipped a candidate: it answered as another server port=9801 name="ssp-mcp-server"
INFO areka_mcp_bridge::scan: [bridge] skipped a candidate: it answered as another server port=9821 name="ssp-mcp-server"
INFO areka_mcp_bridge::service: [bridge] areka not found; answered from the built-in definitions method="tools/list"
INFO areka_mcp_bridge: [bridge] finished quit=Closed
```

**`before\user-root\` の中身**（測ったまま残す・`target\` の下）: `.copy-work\`（空）・`profile\areka\sylphya.toml`（76 バイト・SHA256 `EB18050A6C0F99D8C2F68EAF9C04ED3C1B8F5DDF7C458F8D32FAED0F8DACC900`＝検体の隣の記憶と同じ中身）。`mcp-bridge\` は無い。

**本当の置き場**: 下見の後も比べの後も、本当の `C:\Users\<利用者>\AppData\Local\areka` は無い（`Test-Path` が False）。どちらの測りも作っていない。箱の中の `areka\` も、比べの後は無い（11.1 の控えの後の形のまま）。

比べで分かったこと: 直す前の中継は、箱の外から `AREKA_USER_ROOT` を渡して起こしても、根が無ければ作り（`user_root_created`）、検体の隣の記憶を 1 つ写す（`app_memory_migrated files=1`）。作る・写すは起こし手（箱の中か外か）に関わらず起き、違うのは行き先だけ（箱の中か、渡した根か）。PowerShell から起こした中継の子のプロセスは、パッケージの名乗りを持たない。Claude Desktop から起こしたときの名乗りの有無と、Store 版から起こしたときの 4 点は、手順 5 の本番（開発者に起動し直しを頼む測り）で取る。

### 11.3 M-1 の本番（Store 版の Claude Desktop から起こした・2026-10-30 07:45〜07:58）

**共通の形**:

- 検体は 11.1 の `before\areka-mcp-bridge.exe`。開発者が `claude_desktop_config.json`（本当の `C:\Users\<利用者>\AppData\Roaming\Claude\` の 1 つだけ・箱の `LocalCache\Roaming` に写しは無い）の `areka` の `command` を検体へ向け直し、Claude Desktop を起動し直した（S1・S2 で 1 回ずつ・合わせて 2 回）。
- 控えが書かれる（c）には areka につながる必要がある。そこで、排他テストの机を取り、ワークツリーの `target\debug\areka.exe` を `target\bridge-store-measure\areka-host\` の根と記憶で起こした（`AREKA_ROOT`・`AREKA_PROFILE_DIR` はそこ、`AREKA_USER_ROOT` は渡していない）。areka は `http://127.0.0.1:9801/api/mcp/v1` で待ち受けた（SSP が 9801 に居ても 127.0.0.1 の口は取れた）。この areka は測りの後に測る側が止め、机も返した。
- 中継は起こされた時点では areka を探さない。最初の MCP の要求のときに探して、つながれば `server-info.json` を書く（`service.rs` の `ensure_link`）。どちらの場面でも、測る側（この Claude Code のセッション）から `get_status` を 1 回呼んでつながせた。
- 中継の標準エラーは Claude Desktop の記録 `C:\Users\<利用者>\AppData\Local\Claude\Logs\mcp-server-areka.log` にあった（本当の `%LOCALAPPDATA%` の下・箱の中ではない）。測った時間帯の行の写しは `m1-13\desktop-mcp-server-areka-excerpt.log`。
- 箱の中には、開発者の本物の中継が書いた `areka\mcp-bridge\`（2026-10-29 23:04・23:09）が在った。これは裏の書きとして 2026-10-29 23:22 に `m1-13\ura-before-s1-box-areka\` へ移し、箱の中を空にした（`s1-pre.txt`）。S1 の起動し直し（07:45）の直前には箱を見直していない。ただし起動し直しの後に最初に見た箱の中の物は、07:45:17 にできたフォルダと、元の時刻を持ったままの写し `sylphya.toml` だけで、`mcp-bridge\` は無かった。その間の裏の書きは無かったと見る。

**S1**（本当の `%LOCALAPPDATA%\areka` が無い・直前の形は `m1-13\s1-pre.txt`）: Claude Desktop の起動し直しで中継が 2 つ起きた（07:45:17・07:45:19、親はどちらも Claude Desktop の `claude` のプロセス）。07:47:46 に areka へつないだ。

**S2**（本当の `%LOCALAPPDATA%\areka\profile\areka\` を、測りの間だけ PowerShell の道具から空で作った・07:48:24・直前の形は `m1-13\s2-pre.txt`）: S1 の後に箱の中の `areka\` を消してから作った。起動し直しで中継が 2 つ起き（07:57:25・07:57:27）、07:57:27 に areka へつないだ。

| 点 | S1（本当の置き場が無い） | S2（本当の `profile\areka\` が在る） | 比べ（11.2・PowerShell から・`AREKA_USER_ROOT` あり） |
|---|---|---|---|
| (a) できたフォルダ | 箱の中: `areka\`・`areka\.copy-work\`（空）・`areka\profile\`・`areka\profile\areka\`・`areka\profile\areka\mcp-bridge\`。本当の置き場: 0 | 箱の中: 0（`areka\` そのものができない）。本当の置き場: `profile\areka\mcp-bridge\` が 1 つ（`profile\` と `profile\areka\` は測る側が作った物）。`.copy-work\` はできない | 箱の中 0・本当の置き場 0・`user-root\` の下に 4 つ |
| (b) 記憶の写し | `sylphya.toml` の写しが 1（箱の中の `areka\profile\areka\`・SHA256 `EB18050A…C900`＝検体の隣と同じ） | 0（根が在るので作らず写さない） | 1（`user-root\` の下） |
| (c) `server-info.json`・`tools.json` | `server-info.json`（625 バイト）が箱の中の `areka\profile\areka\mcp-bridge\` に書かれた。`tools.json` は書かれなかった（ツールの一覧を取り直す要求が来なかった） | 両方とも**本当の** `%LOCALAPPDATA%\areka\profile\areka\mcp-bridge\` に書かれた（`server-info.json` 625 バイト・`tools.json` 6807 バイト）。箱の中には無い | 書かれない（areka が居なかった） |
| (d) 中継の標準エラーの行 | 1 つ目の中継に `user_root_created`・`app_memory_migrated files=1`・`user_data_layout reason="winget_install"`、2 つ目は `user_data_layout` だけ（もう根が在る）。どれも場所は本当の `C:\Users\<利用者>\AppData\Local\areka` と書く。`connected to areka port=9801` | 2 つとも `user_data_layout reason="winget_install"` だけ（`user_root_created`・`app_memory_migrated` は無い）。`connected to areka port=9801` | `user_root_created`・`app_memory_migrated files=1`・`user_data_layout reason="env_var"` |
| パッケージの名乗り | 2 つとも無し（`GetPackageFamilyName` が 15700）。それでも書いた物は箱の中へ回された | 2 つとも無し（15700） | 無し（15700） |
| 裏の書き | 測る前に在った物を移した（上の共通の形）。測りの間は無し | 無し | 無し |

S1 の 1 つ目の中継の行（ASCII のまま・時刻を省き、利用者名だけ置き換えた）:

```
INFO areka_user_data::establish: user root created event="user_root_created" dir=C:\Users\<利用者>\AppData\Local\areka
INFO areka_user_data::establish: app memory migrated to the user root event="app_memory_migrated" from=C:\home\maz\git\areka\.claude\worktrees\kiro-start-bridge-store-user-root-d1760a\target\bridge-store-measure\before\profile\areka to=C:\Users\<利用者>\AppData\Local\areka\profile\areka files=1
INFO areka_user_data::establish: user data layout decided event="user_data_layout" mode="user_root" reason="winget_install" user_root=C:\Users\<利用者>\AppData\Local\areka origin="default" marker="Areka.Areka.Portable_measure.db"
INFO areka_mcp_bridge: [bridge] cache dir dir=C:\Users\<利用者>\AppData\Local\areka\profile\areka\mcp-bridge
INFO areka_mcp_bridge::service: [bridge] connected to areka port=9801
```

S2 の中継の行（2 つとも同じ形）:

```
INFO areka_user_data::establish: user data layout decided event="user_data_layout" mode="user_root" reason="winget_install" user_root=C:\Users\<利用者>\AppData\Local\areka origin="default" marker="Areka.Areka.Portable_measure.db"
INFO areka_mcp_bridge: [bridge] cache dir dir=C:\Users\<利用者>\AppData\Local\areka\profile\areka\mcp-bridge
INFO areka_mcp_bridge::service: [bridge] connected to areka port=9801
```

測った物の写し（`target\bridge-store-measure\m1-13\` の下）: `s1-after-box-areka\`（S1 の後の箱の中の `areka\`）・`s2-after-real-areka\`（S2 の後の本当の `areka\`）・`s1-identity.txt`・`s2-identity.txt`（名乗りの答え）。

**M-1 で分かったこと**:

1. Store 版の Claude Desktop が起こした中継は、パッケージの名乗りを持たない。それでも `%LOCALAPPDATA%` への書きは箱の中へ回される。名乗りで「箱の中か」を知る手（3.4 の D1）は、この起こされ方を見落とす。
2. 本当の置き場が無いと（S1）、直す前の中継は根を作り記憶を 1 つ写す。その根は箱の中にでき、記録の上の場所（本当の `%LOCALAPPDATA%\areka`）と食い違う。直したい欠陥がそのまま出た。
3. 要件 1.1 (d): 箱の中から、本当の置き場は在ると見えた。S2 で中継は `user_root_created` を出さず（根が在ると見た）、控えを本当の場所へ書いた。本当の `profile\areka\` が先に在ると（S2）、中継は根を作らず写さず、控えは新しいファイルも含めて本当の場所へ書かれる。箱の中には何もできない。areka 本体が箱の外で根を作っていれば、中継が作らず決めるだけでも、控えは本当の置き場へ届く（3.3 の終わりに足した）。
4. Claude Desktop の中継の記録は、本当の `%LOCALAPPDATA%\Claude\Logs\` に在る。

**戻し**（07:58〜08:47・PowerShell の道具から）:

1. S2 で作った本当の `%LOCALAPPDATA%\areka` を、写しを `s2-after-real-areka\` に取った上で消した。消した後は `Test-Path` が False。
2. 箱の中の `areka\`（S1 の物は写しを取った上で 07:48 に消した）が無いことを確かめ、`backup\box-LocalCache-Local-areka\` を元の場所へ写して戻した。
3. 測る側が起こした areka（pid 17372）を止め、排他テストの机を返した。
4. 開発者が設定の `command` を元の `C:\\home\\maz\\store\\.areka-mcp\\areka-mcp-bridge.exe` へ戻し、Claude Desktop を起動し直した（08:46・本物の中継が 2 つ動いている）。
5. 08:47 に見比べた。箱の中の木は 11.1 の測る前の写しと同じ: `\mcp-bridge\server-info.json`（625・2026-10-23 23:37:49・SHA256 `4F3F1CEC…EEBA`）、`\mcp-bridge\tools.json`（6807・同じ時刻・SHA256 `CFF68DF9…7989`）。`profile\` は無い。違うのは `\mcp-bridge` のフォルダの時刻（戻した 07:58）だけ。本当の `%LOCALAPPDATA%\areka` は無い。

### 11.4 M-2 の Bash の道具と PowerShell から（直した後・2026-10-30 10:00〜10:02）

**検体**: HEAD `6406dfc06`（直しと説明書の後）で `cargo build -p areka -p areka-mcp-bridge` を通し、`target\bridge-store-measure\after\` に 11.1 と同じ形で並べた。S4 用に `after-alone\` に中継の exe だけを置いた。

```
\after\areka.exe                     35232768  SHA256 245B0EB229B0A3733AC2A0DB9DD6C44B3B8C1A752989195A04026BECADF075B2
\after\areka-mcp-bridge.exe           8936960  SHA256 99906F75887D71A21908E08400DF0AD2B199E41FCC31CBA9E767641F262E95B0
\after\Areka.Areka.Portable_measure.db      0
\after\profile\areka\sylphya.toml          76  SHA256 EB18050A6C0F99D8C2F68EAF9C04ED3C1B8F5DDF7C458F8D32FAED0F8DACC900
\after-alone\areka-mcp-bridge.exe     8936960  (上と同じ SHA256)
```

**測る前の形と控え**（`m2\pre-measure.txt`）: 箱の中の `areka\` は `\mcp-bridge\server-info.json`・`tools.json`（11.1 と同じ SHA256・時刻は 2026-10-30 09:02:51）だけ。開発者の机の本物の中継が、別のセッションの areka（9802）へつないで書き直したもの。これを `after\backup\box-LocalCache-Local-areka\` へ移した。本当の `%LOCALAPPDATA%\areka` は無い。

**共通の形**: 流したのは 11.2 と同じ 2 行（`m1-12\input.jsonl`）。Bash の道具では `AREKA_MCP_HOME` を外し、`AREKA_*` と `RUST_LOG` は空だった。測った間、9801・9821 では SSP が答え、9802 では別のセッション（window-work-area-clamp）の areka が待ち受けていた（S5 と比べでは待ち受けが終わっていた）。どの起動も、標準入力を閉じた後に自分で終わった（終了コード 0）。起こす直前と直後に `m2\snap.ps1` で箱・本当の置き場・検体の木を写し、増えた物を数えた（`m2\<場面>\pre.txt`・`post.txt`）。

| 点 | S1（本当の置き場が無い・Bash） | S3（本当の `areka\` だけ在る・Bash） | S5（箱の中に 0.0.3 の写しの形・Bash） | 比べ（PowerShell・`AREKA_USER_ROOT`＝`after\user-root\`） |
|---|---|---|---|---|
| 場面の作り方 | 何も作らない | PowerShell から本当の `%LOCALAPPDATA%\areka\` を空で作った | PowerShell から箱の中に `areka\profile\areka\sylphya.toml`（M-1 の S1 の写しと同じ物）を作った | 何も作らない（`after\user-root\` は無いまま） |
| (a) できたフォルダ | 0 | 0 | 0 | 0（`user-root\` もできない） |
| (b) 記憶の写し | 0。exe の隣の記憶の SHA256 は `EB18050A…C900` のまま | 0。同じ | 0。同じ | 0。同じ |
| (c) 控え | 無し（areka につながったが、置き場が無いので書かない） | 無し（同じ） | 置き場は箱の中の `profile\areka\mcp-bridge` に決まったが、フォルダが無く areka も居なかったので書かれない | 無し |
| (d) 判断の記録 | `user_data_layout reason="winget_install"` | 同じ | 同じ | `user_data_layout reason="env_var"` |
| 控えを持たない理由 | W1 `reason="user_root_missing"` | W1 `reason="app_memory_not_migrated"` | 無し（`Ready`） | W1 `reason="user_root_missing"` |
| 箱の行 | B2 `why="no_cache_dir"` | B2 `why="no_cache_dir"` | B2 `why="not_there_yet"`・`asked` つき | B2 `why="no_cache_dir"` |
| 裏の書き | 無し | 無し | 無し | 無し |

S1 の行（ASCII のまま・時刻を省き、利用者名だけ置き換えた・候補・待つ時間・SSP を飛ばした行・`answered the first inquiry`・`areka not found`・`finished quit=Closed` の行は省いた。下の S5 も同じ）:

```
INFO areka_user_data::establish: user data layout decided event="user_data_layout" mode="user_root" reason="winget_install" user_root=C:\Users\<利用者>\AppData\Local\areka origin="default" marker="Areka.Areka.Portable_measure.db"
INFO areka_mcp_bridge::config: [bridge] running without the cache until areka prepares its user root; it is used from the next start reason="user_root_missing" user_root=C:\Users\<利用者>\AppData\Local\areka
INFO areka_mcp_bridge::redirect: [bridge] cannot tell whether writes under LOCALAPPDATA are redirected why="no_cache_dir"
INFO areka_mcp_bridge::service: [bridge] connected to areka port=9802
```

S3 は 2 行目が `reason="app_memory_not_migrated"` で、ほかは S1 と同じ。S5 の行:

```
INFO areka_user_data::establish: user data layout decided event="user_data_layout" mode="user_root" reason="winget_install" user_root=C:\Users\<利用者>\AppData\Local\areka origin="default" marker="Areka.Areka.Portable_measure.db"
INFO areka_mcp_bridge::redirect: [bridge] cannot tell whether writes under LOCALAPPDATA are redirected why="not_there_yet" asked=C:\Users\<利用者>\AppData\Local\areka\profile\areka\mcp-bridge
INFO areka_mcp_bridge: [bridge] cache dir dir=C:\Users\<利用者>\AppData\Local\areka\profile\areka\mcp-bridge
```

**S5 の続きの下見（S5b・本番の代わりにしない）**: S5 の形に、PowerShell から箱の中の `areka\profile\areka\mcp-bridge\` を空で足して、Bash の道具から同じく起こした（10:02）。中継は 9802 の areka につなぎ、控えの 2 つを箱の中の `profile\areka\mcp-bridge\` に書いた（`server-info.json` 625・`tools.json` 6807）。箱の行は B1 で、`canonicalize` は箱の場所を返した:

```
INFO areka_mcp_bridge::redirect: [bridge] writes under LOCALAPPDATA are redirected into the app's private folder (Microsoft Store app) asked=C:\Users\<利用者>\AppData\Local\areka\profile\areka\mcp-bridge resolved=\\?\C:\Users\<利用者>\AppData\Local\Packages\Claude_pzs8sxrjxfjjc\LocalCache\Local\areka\profile\areka\mcp-bridge box_dir=\\?\C:\Users\<利用者>\AppData\Local\Packages\Claude_pzs8sxrjxfjjc\LocalCache\Local
```

`resolved` と `box_dir` には `canonicalize` の `\\?\` の頭が付いたまま出る（設計の文は頭の有無を決めていない）。Claude Desktop から起こしたときも同じかは 6.3 の S4 で見る。

**M-2 の Bash と PowerShell で分かったこと**:

1. 直した中継は、どの場面でもフォルダを作らず、記憶を写さない（作る数 0・写す数 0・exe の隣の記憶の SHA256 不変・要件 2.2・2.3）。
2. 根がまだ無い（S1）・写しがまだ（S3）では、理由の行 W1 を 1 件出して控え無しで動く（要件 2.4・2.5・3.2）。
3. 0.0.3 の写しが箱の中に残っていると（S5）、箱の中の中継からは根が在ると見え（`Ready`）、控えの置き場は箱の中の `profile\areka\mcp-bridge` になる。控えが書かれた後の起動では B1 で箱の中と分かる（S5b）。説明書のとおり、その写しを消せば S1 と同じ形に戻る。
4. Bash の道具から起こした中継でも、`canonicalize` は箱の中の場所を返した（設計の一番危うい見込みの下見）。

**戻し**: S3 で作った本当の `%LOCALAPPDATA%\areka`（空）を消した。S5・S5b で作った箱の中の `areka\`（控えの 2 つを含む）を消した。`after\user-root\` はできていない。10:02 の時点で、箱の中に `areka\` は無く、本当の `%LOCALAPPDATA%\areka` も無い。控えは `after\backup\` に置いたまま（戻すのは 6.3 の終わり）。

## 12. 設計の討議で決めたこと（2026-10-29）

- 議題 1（承認）: 実機の測りで Claude Desktop の起動し直しを頼むのは 4 回（M-1 の S1・S2、M-2 の S2 を 2 回）。M-2 の S1・S3 は Bash の道具（その子も同じ箱へ回される）から検体の中継を起こして測る。本物の Claude Desktop でしか確かめられない「箱の行と箱のフォルダの場所」は M-2 の S2 の 2 回目に入る。

## 13. 実装の途中で改めたこと（2026-10-30）

- M-2 の段取り（開発者の裁定・案 1）: M-1 の S2（11.3）で、本当の `profile\areka\` が在ると控えは新しいフォルダも含めて本当の場所へ書かれると分かった。12 の議題 1 の見込み（M-2 の S2 の 1 回目で控えのフォルダが箱にでき、2 回目で箱の行）は成り立たないので、次のように改めた。
  - M-2 の S2 は Claude Desktop から 1 回で、控えが本当の `profile\areka\mcp-bridge\` にでき、箱の行が出ないことを見る。
  - 箱の行と箱のフォルダの場所は、新しい場面 S4（中継を `areka.exe` の隣に置かず、本当の `%LOCALAPPDATA%\areka` が無い・控えは `%LOCALAPPDATA%\areka\mcp-bridge` に決まり箱の中にできる見込み）で Claude Desktop から 2 回起こして見る。
  - 新しい場面 S5（0.0.3 の中継が箱の中に残した `areka\profile\areka\` が在り、本当の置き場は無い）を Bash の道具から測る。タスク 5 の見直しで、箱の中から根が「在る」と見えて控えを箱の中に置き続けるかもしれないと指摘されたため。
  - Claude Desktop の起動し直しは合わせて 5 回（M-1 が 2 回・M-2 が 3 回）。design.md の実機の測りと tasks.md の 6.2・6.3 をこれに合わせた。
