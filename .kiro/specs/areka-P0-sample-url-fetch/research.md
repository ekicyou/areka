# ギャップ分析: areka-P0-sample-url-fetch

> 実測は 2026-10-18・枝 `claude/areka-p0-sample-url-fetch-835511`（main `e2370df8` の上に spec の初期化 1 つ）。コードは「何の定義か」（関数名・型名＋ファイルパス）で指す。行数は参考。
> 入力: `requirements.md`（確定）・`brief.md`（「2026-10-18 棚卸㉗の再測定」の節を優先）・`roadmap.md` のウェーブ C9 ⑤ の約束（`tools/test-all.ps1` に触らない・依存を足さない）・steering `tech.md`／`structure.md`／`product.md`。
> 書庫（`pasta-in-windows.nar`）は取り寄せていない。書庫の中身に関わる点は「調べが要る」として残した。

## 1. 要約

- **土台はそろっている**。登記表 `SAMPLES`（`crates/sample-ghost-kit/src/lib.rs`）は行の型 `Sample` を lib.rs の中だけで組んでいる（外のクレートは名前と `balloons` を読むだけ）ので、行に「取り寄せる URL・SHA-256・大きさ」を足しても外は壊れない。書庫の在処を決めるのは `devroot::nar_dir()` と、それを使う `fresh_root`・`manual_root`・`cached_root`（テスト専用）の 3 か所だけで、展開・刻印・複製・後始末はそのまま使える。
- **足りないもの**は 4 つ。① 行の区別（今の行／URL の行）② 取り寄せた書庫の置き場と、窓口がそこを読んで確かめる道 ③ `tools/` の取り寄せの段（新しいスクリプト）④ SHA-256 を確かめる手段。**ワークスペースに SHA-256 の部品が無い**（`Cargo.lock` に `sha2` 等が無い）。窓口が SHA-256 を確かめる（要件 3.4）には、手書きするか、OS の機能（CNG）を使うかを決める必要がある。後者は `sample-ghost-kit` に `windows` の依存を足すので `Cargo.lock` が動き、C9 の「`Cargo.lock` の席は ⑥」「依存を足さない」の約束に当たる。
- **議題 1（取り寄せの段を回していない全体テストが赤）を確かめた**。赤になるのは `sample-ghost-kit` の 3 本だけ（登記表の全部の行を引く 2 本＋全部の行で `nar-sample-path` を起こす 1 本）。ほかのクレートのテスト・`tools/perf/*.ps1`・`tools/package.ps1`・検体の見張り（`sample_path_guard_test.rs`）は、決まった名前（`emo2`・`StayseeBalloon` など）か名前の一覧しか使わないので当たらない。ただし `/kiro-complete` は全体テストの前に main を取り込むので、**⑤ の着地の後に完了する C9 の兄弟は、取り寄せの段を回していなければ全員この 3 本で赤になる**（「触っていないクレートだけが赤＝止めて報告」に掛かる）。
- **ウェーブの約束の文言は、窓口から取り寄せの段を呼ぶ形（議題 1 の案 い）を許している**（roadmap C9 ⑤「取り寄せは検体を引く所か別の台本から呼び、全体テストの入口へつなぐのは ① の着地の後」）。
- 規模は M（6〜9 タスク）・危うさは中（決めごとが多い・兄弟の全体テストに効く・ぱすたの書庫を areka の展開器が受けるかは取り寄せるまで分からない）。

## 2. 今ある物（調べた範囲）

### 2.1 検体の窓口 `crates/sample-ghost-kit/`

| ファイル | 行数 | 役目 | この spec との関わり |
|---|---|---|---|
| `src/lib.rs` | 548 | 登記表 `SAMPLES`・行の型 `Sample`（`name`・`kind`・`balloons`）・`SampleRoot::acquire`・`manual_paths`・`registered`・`check_registry`・`SampleError`（`#[non_exhaustive]`） | 行に URL・ハッシュ・大きさを持たせる。失敗の種類（書庫が無い・合わない）を足す |
| `src/devroot.rs` | 696 | `nar_dir()`（`vendors/sample_ghost` を `CARGO_MANIFEST_DIR` から綴る唯一の所）・`namespace_dir()`（`CARGO_TARGET_DIR` か実行ファイルの祖先の `target` の下の `nar-samples`）・`fresh_root`／`manual_root`／`cached_root` が `nar_dir().join("<名>.nar")` を組む・`cached_root_in` が書庫を丸ごと読んで刻印（長さ＋CRC-32）を作る | URL の行のときだけ書庫のパスを「取り寄せた置き場」に替え、確かめてから `cached_root_in` へ渡す。`*_in` 系は書庫のパスを引数で受けるので、ここから先は変えずに済む |
| `src/bin/nar-sample-path.rs` | 76 | `manual_paths` を呼び、成功 0・**失敗はすべて 2**（未登録・展開の失敗・入出力の失敗を区別しない） | 要件 3.7 の「2 とは別の 0 でない終了コード」を足す |
| `src/lib_tests.rs` | 648 | 登記の数 8 を 2 か所に直書き・未登録の名前の一覧 8 つを直書き・全部の行を引く判定 2 本 | 数と一覧を 9 へ。全部の行の判定の 1 本は `devroot::nar_dir()` を直に読む（下の 2.3） |
| `tests/nar_sample_path_test.rs` | — | 全部の行で実行体を起こす判定 1 本・未登録の名前で終了コード 2 の判定 | 全部の行の判定が URL の行を含む |
| `Cargo.toml` | — | 依存は `thiserror`・`areka-nar`・`miniz_oxide` の 3 つ | SHA-256 の手段次第で `windows` が要る（下の 3.4） |

- `Sample` を組む式（`Sample { … }`）は lib.rs の `SAMPLES` の中にしか無い（全ワークスペースを調べた）。外のクレートが読むのは `name`・`balloons`（`sample_path_guard_test.rs`）だけ。欄を足しても外のコンパイルは壊れない。
- 書庫の名前は「`.nar` のファイル名＝`install.txt` の `directory`＝行の `name`」が前提（`Sample::name` の doc）。URL の行でも、置き場に置くファイル名を `<name>.nar` にそろえれば前提はそのまま。
- 失敗の文: `SampleError` の表示は英語の ASCII（`Io` の `what` だけ日本語）。要件 3.7 の「標準エラーへ ASCII」は、新しく足す失敗の表示を ASCII にすれば満たせる。

### 2.2 取り寄せ・ハッシュの前例

- **SHA-256 の部品はワークスペースに無い**。`Cargo.lock` に `sha2`・`ring` 等は無く、`windows` の `Win32_Security_Cryptography` を使うのは `crates/areka-update/` だけ（`src/md5.rs` が `BCryptHash(BCRYPT_MD5_ALG_HANDLE, …)` 1 呼び出しで MD5 を取る。機能はクレートの `Cargo.toml` で上乗せし、根の `[workspace.dependencies.windows]` は変えない＝tech.md「ネットワーク更新の HTTP と MD5 は OS の機能で賄う」）。
- PowerShell の前例: `tools/package.ps1` が `Get-FileHash -Algorithm SHA256`（大文字が返るので小文字へ直す）・`tools/crates-io.ps1` が `Invoke-WebRequest`・`tools/release-notes.ps1` が `-SelfTest`（自分の判定を 1 つずつ印字）。
- 実測（この機械・pwsh 7.6.7）: `Invoke-WebRequest` は `file://` を受けない（`The 'file' scheme is not supported.`）。`curl.exe` は `C:\WINDOWS\system32\curl.exe` に在る（Windows 10 以降の標準）。
- `tools/encoding-check.ps1` が `tools/*.ps1`（直下だけ）を検査する: ⑴ 字句は ASCII だけ（規則 A）⑵ 端末の文字コードを書き換えない（規則 C）⑶ `cargo` を `metadata` か `nar-sample-path` つきで呼ぶなら `Invoke-Utf8Child` を通す（規則 D）。新しい取り寄せのスクリプトもこの検査に入る。

### 2.3 登記表の全部の行を使うもの（議題 1 の確かめ）

| どこ | 何をする | URL の行の書庫が無いとき |
|---|---|---|
| `lib_tests.rs` の `every_registered_sample_lands_where_its_registry_row_says` | 全部の行で `SampleRoot::acquire` | **赤**（要件 3.5 の文で失敗） |
| `lib_tests.rs` の `every_sample_nar_installs_exactly_the_elements_its_registry_row_declares` | 全部の行で `devroot::nar_dir().join("<名>.nar")` を直に開き `assert!(nar.is_file())` | **赤**。しかも**取り寄せた後でも赤**（`vendors/` を見に行くため）。書庫のパスを窓口と同じ解決に通す書き替えが要る |
| `tests/nar_sample_path_test.rs` の `the_command_prints_one_absolute_path_per_line_for_every_registered_sample` | 全部の行で `nar-sample-path` を起こし終了コード 0 を求める | **赤** |
| `lib_tests.rs` の `unknown_sample_fails_with_all_eight_known_names`・`nar_sample_path_test.rs` の未登録の判定 | 名前の一覧だけ | 当たらない（一覧の直書き 8 → 9 の直しは要る） |
| `crates/log-capture-kit/tests/sample_path_guard_test.rs` | `SAMPLES` の名前・同梱バルーン名から見張る語を組む。走査は `crates/**/*.rs` だけ（`workspace_scan::walk_workspace_sources`） | 当たらない。ぱすたは同梱バルーン 0（`pasta-in-windows-verification` の brief）なので、バルーン名から組む語も増えない。見張る語「`nar-samples`」は `tools/` の PowerShell を走査しない |
| ほかのクレートのテスト（`areka`・`areka-nar`・`areka-update`・`areka-emo-*`・`areka-seriko` ほか） | 決まった名前で `acquire` | 当たらない（`SAMPLES` を回すものは窓口の外に無い） |
| `tools/perf/perf-loop.common.ps1`・`invoke-followup-checks.ps1`・`invoke-perf-run.ps1` | `nar-sample-path -- emo2` ほか | 当たらない。終了コードは「0 以外なら止める」で読むので、新しい終了コードも受ける |
| `tools/package.ps1`・`.github/workflows/release.yml` | `emo2`・`StayseeBalloon` を `nar-sample-path` で引く・`vendors/sample_ghost/emo2.nar` を直に読む所が 1 つ | 当たらない（どちらも今の行のまま） |
| `tools/encoding-check.ps1` | `nar-sample-path` の呼び方の規則 | 新しいスクリプトが `nar-sample-path` を呼ぶなら規則 D・E の対象（E は `package.ps1` だけを名指し） |
| CI（`.github/workflows/*.yml`） | テストを回さない | 当たらない |

**C9 の兄弟への効き方**: `/kiro-complete` は全体テスト（`tools/test-all.ps1` の `cargo test --workspace`）の前に main を取り込む（スキルの前置き 6「番が来たら…既定の枝を取り込む」・ステップ 3）。だから ⑤ が先に着地すると、後から完了する兄弟のワークツリーは、取り寄せの段を回していない限り上の 3 本で赤になる。`/kiro-validate-impl` や実装の途中の全体テストも、main を取り込んだ後なら同じ。赤は「触っていないクレートだけが赤」の決まり（1 回だけ回し直し、変化が無ければ申し送り）に掛かり、兄弟のセッションが止まって開発者へ報告が上がる。

### 2.4 置き場・追跡の決まり

- 根の `.gitignore` の `target` はどの深さの `target` も外すので、ワークツリーの `target\` の下に置けば要件 1.6（追跡の候補が出ない）は自動で満たせる。
- 窓口の置き場の見つけ方は `CARGO_TARGET_DIR` に従う（`devroot::find_target_dir`）。`devroot_cache_tests.rs` の `four_processes_racing_the_first_install_all_get_a_complete_tree` は子プロセスの `CARGO_TARGET_DIR` を私有の場所へ振る。PowerShell の取り寄せの段が置き場を自分で綴ると、`CARGO_TARGET_DIR` を設定した走行で窓口と食い違いうる（下の 4 の決めごと 4）。
- 別の窓口 `under_target`（`crates/areka/` ほか）は `CARGO_TARGET_DIR` を見ず、いつもワークスペースの根の `target` を使う（`test-roots-under-target` の brief）。どちらに寄せるかで、スクリプトと窓口が置き場を合わせる手間が変わる。

### 2.5 文書

- `vendors/sample_ghost/README.md`（146 行）: `claudia.nar` の節に「URL とハッシュだけ記録して取得時にダウンロードする形にしない」（要件 7.3 の書き直しの対象）。「検体を 1 本足す手順」「守る 5 点」は書庫を置く前提。
- steering: `tech.md` の「検体は配布形 `.nar` のまま保管し、テストは窓口越しに引く」・`structure.md` の Sample Ghost Kit の節（`Pattern` に「2 手で終わる」）。`product.md` の「いずれも `vendors/sample_ghost/*.nar`（配布形のまま保管）」は 3 系統のテンプレートの話で、ぱすたを足さない限り嘘にならない（要件の範囲外）。
- 根の `README.md` の「動作を確かめているゴーストは…どれも `vendors/sample_ghost/` に配布形のまま置いています」: ぱすたをこの表に足すなら直しが要る（要件は名指ししていない）。
- `dist/README.txt` に検体の保管の形に触れる所は無い（要件 7.7 は触らずに済む）。

## 3. 要件と今ある物の対応

| 要件 | 使える物 | 足りない物 | 札 |
|---|---|---|---|
| 1.1・1.2 行の区別と欄 | `Sample`・`SAMPLES`（lib.rs の中だけで組む） | 行の出どころの区別（例: 「置く／取り寄せる」の列挙）と URL・SHA-256・大きさの欄 | 足りない |
| 1.3 URL の行の書庫を追跡しない | — | 「URL の行の名前の `.nar` が `vendors/sample_ghost/` に無い」の決定論テスト | 足りない |
| 1.4 今の 8 本は今のまま | `nar_dir()`・`fresh_root` ほか | 変えないことの確かめ（今のテストがそのまま通ること） | 制約 |
| 1.5 欄の欠け・形の誤りで赤 | — | URL の形（`https://`）・SHA-256 の形（小文字 16 進 64 桁）・大きさ 0 より大の決定論テスト | 足りない |
| 1.6 置き場を追跡しない | 根の `.gitignore` の `target` | 置き場を `target\` の下に決めること | 制約 |
| 2.1〜2.10 取り寄せの段 | `Invoke-WebRequest`・`curl.exe`・`Get-FileHash`・`Invoke-Utf8Child`・`-SelfTest` の前例 | スクリプトそのもの（一時の名前で取る → 確かめる → 名前を替えて置く・数の印字・終了コード）。登記を 1 か所から読む道（2.9） | 足りない・調べが要る |
| 3.1・3.4・3.6 窓口が置き場から読み、確かめてから展開 | `cached_root_in(namespace, name, nar)` が書庫のパスを引数で受ける・書庫を丸ごと読む所が既に在る | 置き場のパスの解決・大きさと SHA-256 の照合 | 足りない（SHA-256 の手段が決めごと） |
| 3.2・3.9 呼び方・出力の形・決定論 | `SampleRoot`・`manual_paths` | 変えないこと | 制約 |
| 3.3 窓口は通信しない | 今は通信の依存が無い | 議題 1 の案 い を採ると、窓口がスクリプトを起こす（通信はスクリプトの中） | 決めごと |
| 3.5 無いときの失敗の文 | `SampleError`（`#[non_exhaustive]`＝足しても外は壊れない） | 新しい失敗の種類（名前・探した場所・直し方のコマンド） | 足りない |
| 3.7 `nar-sample-path` の終了コード | `run()` が失敗をすべて 2 にする | 失敗の種類で終了コードを分ける | 足りない・決めごと |
| 3.8 今の行は取り寄せなしで引ける | 今の道 | — | 制約 |
| 4.1〜4.3 手で置く口 | — | 「置き場の同じファイル名に置けば同じに扱う」を窓口とスクリプトの両方で通すこと・README | 足りない |
| 5.1〜5.6 ぱすたの登記 | — | 行 1 つ・数 8 → 9・README の出どころの節 | 足りない・調べが要る（`install.txt` の `directory`・展開器が受けるか） |
| 6.1〜6.3 取り寄せていない全体テスト | — | 2.3 の 3 本が要件 3.5 の文で赤になること・README の手順 | 決めごと（議題 1） |
| 7.1〜7.7 文書と steering | README・`tech.md`・`structure.md` | 書き直し | 足りない |

## 4. 作り方の案

### 案 A: 窓口を広げる（今のファイルに足す）

- `lib.rs`: `Sample` に出どころの欄を足す（例: `source: Source`＝`Vendored`／`Fetched { url, sha256, size }`）。`SampleError` に「書庫が無い」「書庫が合わない」を足す。
- `devroot.rs`: `nar_dir().join(...)` を組む 3 か所を「行から書庫のパスを決める 1 つの関数」に替え、URL の行なら置き場のパスを確かめて返す。
- `nar-sample-path.rs`: 失敗の種類で終了コードを分ける。
- 長所: 触るファイルが少ない。`*_in` 系は変えずに済む。
- 短所: `devroot.rs` は 696 行。SHA-256 を手書きで足すと 1,000 行の上限に近づく。窓口の本体の「刻印・複製・後始末」と「取り寄せた書庫の確かめ」が 1 つのファイルに混ざる。

### 案 B: 新しい部品に分ける

- 新しいファイル（例: `src/fetched.rs`）に「置き場のパス・大きさと SHA-256 の照合・失敗の文」を置き、SHA-256 を手書きするならさらに別のファイルへ分ける。テストは兄弟のファイル（`*_tests.rs`）。
- `tools/` に取り寄せのスクリプト（新）。
- 長所: `devroot.rs` は 3 か所の書き替えだけ。照合は私有の置き場と `nar_writer` で組んだ書庫で決定論テストにしやすい（今の `*_in` と同じ型）。
- 短所: ファイルが増える。

### 案 C: 混ぜる（推しやすい形）

- 行の欄と失敗の種類は `lib.rs`（案 A）、置き場と照合は新しいファイル（案 B）、`devroot.rs` は書庫のパスを決める所だけ直す。
- 取り寄せの段は「登記を Rust から聞く → PowerShell が取る・確かめる・置く」の形にすると、登記（2.9）も置き場の位置（`CARGO_TARGET_DIR` との食い違い）も Rust 側の 1 か所で決まる（下の決めごと 3・4）。
- 長所: 既存の型に沿い、1,000 行の上限にも収まる。短所: 取り寄せの段の前に `cargo build` が要る。

## 5. 決めごと（要件の討議・設計へ渡す）

1. **取り寄せの段を回していない全体テストの赤（要件の議題 1）**。確かめた事実は 2.3 のとおり（赤は `sample-ghost-kit` の 3 本・兄弟は main を取り込んでから全体テストを回す）。ウェーブの約束の文言は「取り寄せは検体を引く所か別の台本から呼び」で、案 い（窓口が書庫の無い URL の行に当たったら同じスクリプトを 1 回呼ぶ）も約束の内に在る。
   - 案 あ（要件の本文）: 赤のまま・文が直し方を示す。代償は兄弟のセッションが止まって報告が上がること（⑤ が先に着地した場合、最大 7 本）。
   - 案 い: 窓口から取り寄せの段を呼ぶ。全体テストは緑のまま。代償はテストの走行の中で初回だけ通信が起きること（「テストは通信しない」の方針に反する）、並走するテストが同時に呼ぶので置き場への書き込みの取り合いを片付ける必要があること、`pwsh` か `curl.exe` を子として起こすこと。
   - 案 う: ① の着地の後に ⑤ を着地させ、同じ spec で `tools/test-all.ps1` にもつなぐ（約束の組み直し）。
   - 案 え（コードを変えない）: ⑤ を C9 の最後に着地させる（兄弟が先に完了すれば、兄弟の全体テストには URL の行がまだ無い）。次のウェーブからは、ワークツリーごとに 1 回回す手順か、① の着地の後のつなぎが効く。
   - 参考: 「URL の行だけを全体テストから外す」（テストを別の的に分けて機能の旗で切るなど）は要件 6.1（飛ばし・緑にしない）に反する。
2. **窓口の SHA-256 の手段（新しい決めごと）**。要件 3.4 は窓口が大きさとハッシュを確かめることを求めるが、ワークスペースに SHA-256 の部品が無い。
   - 案 ア: 標準ライブラリだけで SHA-256 を手書きする（数十行＋FIPS 180-4 の試験値の決定論テスト）。依存も `Cargo.lock` も動かない。デバッグ版（最適化なし）で 3 MB を何秒で読むかは測る（刻印〔長さ＋CRC-32〕が同じ原本が `cache/` に在るときは確かめを省く、などの手当てを設計で決める）。
   - 案 イ: `windows` の `BCryptHash`（`BCRYPT_SHA256_ALG_HANDLE`）を使う。前例は `areka-update/src/md5.rs`。速いが、`sample-ghost-kit` に `windows` の依存を足すので `Cargo.lock` の `sample-ghost-kit` の行が動く＝C9 の「`Cargo.lock` の席は ⑥」「⑤ は依存を足さない」に当たる。
   - 案 ウ: 窓口から `Get-FileHash`（`pwsh`）か `certutil` を子として起こす。依存は増えないが、遅く、子プロセスの失敗の扱いが増える。
   - 案 エ: 窓口は大きさと CRC-32（`areka_nar::crc32` は在る）だけを見て、SHA-256 は取り寄せの段だけが見る。登記に CRC-32 の欄も要る（同じ書庫の印を 2 つ書く）か、要件 3.4 を改めることになる。
3. **登記を 1 か所から読む道（要件 2.9）**。
   - 案 ① Rust の登記表を正本にし、Rust の実行体が「名前・URL・SHA-256・大きさ・置き場のパス」を印字して、スクリプトがそれを読む（`nar-sample-path` に別の呼び方を足すか、新しい実行体）。置き場の位置も Rust が決めるので、決めごと 4 の食い違いが起きない。代償は取り寄せの前に `cargo build` が要ること。`nar-sample-path` を呼ぶなら `encoding-check.ps1` の規則 D により `Invoke-Utf8Child` を通す。
   - 案 ② 登記を文字のファイル（例: `vendors/sample_ghost/` の下の 1 行 1 検体の表）へ出し、Rust（`include_str!` と読み分け）と PowerShell の両方が読む。`SAMPLES` は今 `const` の表なので、URL の行だけ実行時に読む形になり、登記が 2 つの形に割れる。JSON や TOML にすると Rust 側に読み手の依存が要る（`sample-ghost-kit` には無い）。
   - 案 ③ スクリプトが `lib.rs` を正規表現で読む。もろい（書式の変更で黙って壊れる）。
4. **置き場の位置（要件の議題 2 を含む）**。
   - ⒜ 窓口の名前空間の下（`<target>/nar-samples/…`）: `CARGO_TARGET_DIR` に従う。スクリプトが自分で綴ると `CARGO_TARGET_DIR` を設定した走行で食い違う（決めごと 3 の案 ① なら起きない）。
   - ⒝ ワークスペースの根の `target` に固定（`under_target` と同じ引き方）: スクリプトと窓口の両方が `CARGO_MANIFEST_DIR`／スクリプトの場所から同じ所を綴れる。代わりに `CARGO_TARGET_DIR` を振ったテストからも同じ書庫が見える（読むだけなので害は小さい）。
   - ⒞ ワークツリーをまたぐ置き場（要件の議題 2 の代わりの案）: 「検体・一時のフォルダはワークツリーの `target\` の下だけ」の決まりの例外になる＝開発者の決めごと。
5. **`nar-sample-path` の新しい終了コード（要件 3.7）**。今は失敗がすべて 2（未登録・展開の失敗・入出力の失敗）。「書庫が無い・合わない」にだけ新しい値（例: 3）を与えるか、ほかの失敗も分けるか。読み手（`tools/perf/` の 2 本・`package.ps1`）は「0 以外なら止める」なので、どちらでも壊れない。`nar-sample-path.rs` の doc（「0 が成功・2 が失敗」）と判定 `an_unregistered_name_prints_the_reason_to_standard_error_and_exits_with_two` の言い回しの直しが要る。
6. **ぱすたの行をどちらの spec が足すか（要件の議題 3）**。食い違いを確かめた: `roadmap.md` の「検体の登記表の順を改めた」は「`pasta-in-windows-verification`（URL の行を 1 つ足す）」、同 brief の Scope の In も「検体 `pasta-in-windows` の登記と…の自動テスト」を持つ。この spec の brief（棚卸㉗）と要件 5 はこの spec が足すとする。どちらを採っても、もう片方の文書（brief か roadmap）の直しが要る。この spec で足さない場合、取り寄せの段と窓口の確かめの道は、決定論テストの偽の書庫だけで確かめることになる（実物の URL を 1 度も通さずに着地する）。
7. **取り寄せの段の失敗の道の決定論テスト（要件 2.4・2.5）**。`Invoke-WebRequest` は `file://` を受けない（実測）。案: 取る部分を差し替えられる形にして `-SelfTest`（`release-notes.ps1` の前例）で通す・`curl.exe`（`file://` を受ける）を使う・手元に小さな HTTP の受け口を立てる。`tools/test-all.ps1` に触れないので、自分の検査を誰がいつ回すか（完了の手順・README）も決める。
8. **ghost_dev の履歴を書き換えない約束（要件の議題 4）**。コードには効かない。README の取り寄せ先の条件（要件 7.2）の書き方に効く。
9. **根の `README.md` の検体の表**。ぱすたを表に足すなら「どれも `vendors/sample_ghost/` に配布形のまま置いています」を直す。要件は名指ししていない（足さないなら触らない）。

## 6. 調べが要るもの（設計の段へ）

- **ぱすたの書庫の中身**（取り寄せてから確かめる）: `install.txt` の `type`・`directory`（行の `name` と置き場のファイル名をこれに合わせる＝要件 5.2）、`areka-nar` が受理して展開できるか（68 ファイル・ゴーストのフォルダの中に縦書きのバルーン）、同梱のバルーンが 0 で `check_registry` の照合が合うか。受理されなければこの spec の着地を止める障害になる（直しは展開器＝範囲の外）。最初のタスクで取り寄せて登記の往復を通すのが安全。
- **SHA-256 の速さ**（決めごと 2 の案 ア を採るなら）: デバッグ版で 3 MB の計算時間。全部の行を引く判定と `pasta-in-windows-verification` のテストが何回引くか。
- **取り寄せの道具**: `Invoke-WebRequest` と `curl.exe` のどちらを使うか（`raw.githubusercontent.com` の転送・途中で切れたときの終了の形・進み具合の表示を消す方法・端末へ出す文を ASCII だけにできるか）。PowerShell 7 が前提か（今の `tools/*.ps1` は `#Requires -Version 7`）。
- **同時に回したとき**: 取り寄せの段を 2 つ同時に回す・案 い で並走するテストが同時に呼ぶときの、一時のファイルの名前と、置き場への名前の付け替えの取り合い。
- **`-SelfTest` の当たり**: 偽の取り寄せ先で「合わないハッシュは置かずに捨てる」「途中で切れたら何も残さない」「2 回目は 0 件」を通す組み方。

## 7. 規模と危うさ

- **規模: M（6〜9 タスク）**。行の欄と失敗の種類・置き場と照合（＋SHA-256）・`nar-sample-path` の終了コード・取り寄せのスクリプトとその自分の検査・ぱすたの行と数のテスト・README と steering。既存の型（`*_in` に書庫のパスを渡す・`nar_writer` で偽の書庫を組む）に沿えるので、新しい作りは小さい。
- **危うさ: 中**。決めごと 1（兄弟の全体テスト）と決めごと 2（SHA-256 と依存の約束）は答えで作業が変わる。ぱすたの書庫を展開器が受けるかは取り寄せるまで分からない。通信そのものは前例のある道具（`Invoke-WebRequest`・`curl.exe`）で済む。

## 8. 設計の段への申し送り

- 推しやすい形は案 C（行と失敗は `lib.rs`・置き場と照合は新しいファイル・`devroot.rs` は書庫のパスを決める所だけ）。決めごと 3 の案 ① を採ると、登記と置き場の位置が Rust の 1 か所に寄る。
- 先に決めるもの: 決めごと 1（赤の扱い）・2（SHA-256 の手段）・6（ぱすたの行の持ち主）。この 3 つで、`Cargo.toml` を触るか・取り寄せの段を窓口から呼ぶか・最初のタスクで実物を取り寄せるかが決まる。
- 書き替えが要るテスト: `lib_tests.rs` の数 8 の 2 か所・未登録の一覧・`every_sample_nar_installs_exactly_the_elements_its_registry_row_declares`（`devroot::nar_dir()` を直に読む所を窓口と同じ解決に通す）・`nar_sample_path_test.rs` の終了コードの判定の言い回し。`lib_tests.rs` は 648 行なので、新しい判定は兄弟のファイルへ。
