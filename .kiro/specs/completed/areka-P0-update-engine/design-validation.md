# Design Validation: areka-P0-update-engine

> 2026-09-24 `/kiro-validate-design`（非対話・レビュー報告を本ファイルに保存）。入力は `design.md`（2026-09-23）・`requirements.md`（10 要件・78 受入基準・裁定 3 件込み）・`research.md`・`brief.md`・steering（`product.md`・`tech.md`・`structure.md`・`logging.md`・`roadmap.md`）。既存コードへの言及は本ブランチで Grep／Read で引き直し、正典は ukadoc MCP（`spec_update_file`・`manual_update`・`dev_update`・`descript_install`）で照合した。要件ディスカッションで決着した 4 点（MD5＝OS の CNG・既定の文字コード＝Shift_JIS 固定・`areka-nar` の部品は再利用しない・作業場所＝対象フォルダ直下／差分 0 の周は何も書かない）は再検討しない。

## レビューの要約

設計は 78 の受入基準すべてに部品・関数・テストを対応付けており（抜き取り 20 件で欠落 0）、既存コードとレジストリ（`windows 0.62.2`・std の `rename`・`areka-nar` の前例・見張りのテスト・試験の道具）への言及はすべて実物と一致した。正典（行の形・行種別・拡張フィールド・文字コード・セキュリティチェック・URL エンコード・`delete.txt` の置き場）とも食い違いは無い。残る懸念は 2 つで、どちらも「全か無か」の約束の周辺にある小さな手順の穴であり、構造を変えずに数行で塞げる。

## 検証の記録

### (a) 要件 → 設計の対応（抜き取り 20 件・全 10 要件から）

| 基準 | 設計での受け皿 | 判定 |
|---|---|---|
| 1.1・1.2・1.3 | `run` 手順 2（`NotFound` のときだけ後退・それ以外は `ManifestFetch`・両方無しは `ManifestMissing`） | ○ |
| 1.8〜1.10 | `manifest::parse` 手順 1（先読み → `for_label` → 解決不能は `UnknownCharset` 警告＋`DEFAULT_CHARSET`） | ○ |
| 1.11・1.12 | `InvalidWhy` 9 種（正典 3＋追加 6）・検査順序が明記 | ○ |
| 1.15 | `urlpath::{is_encoded, decode, encode}`＋復号後は UTF-8 → 定義ファイルの文字コードの順（Design Decisions に理由） | ○ |
| 1.17 | `Stage::Entry`・`TargetMissing`／`InvalidHomeurl`・取得口を呼ばない・`run_tests` に「呼出 0 回」 | ○ |
| 2.3・2.5 | `diff::plan` は定義の側だけ歩く・差分 0 は作業場所を作らず `delete.txt` も読まない | ○ |
| 3.2・3.6 | `FakeFetch`（固定表＋失敗注入）・`TIMEOUTS_MS` 4 値＋`MAX_BODY_BYTES` | ○ |
| 4.5・4.6 | `Md5Compared` を通知してから `Md5Mismatch`（`Stage::Verify`）・`FileFetch`（`Stage::Download { i, total }`） | ○ |
| 4.8・5.5 | `WorkArea::cleanup`／`keep`・`UpdateError::{leftovers, work}`・`RollbackFailed { restored, stuck }` | ○（ただし下記の重大 1） |
| 5.4・5.6・5.7 | `Undo` 3 種の逆順の解き・`resolves_under` を差分と確定の 2 回・`hold` 注入 | ○（ただし下記の重大 2） |
| 6.1〜6.3 | `delete_files`（`delete.txt` → 数字の昇順）・`apply(charset)`・`DeleteWhy` 4 種 | ○ |
| 7.1・7.2・7.4 | `Progress` 6 変種・`UpdateOutcome::{Unchanged, Updated}`・`fail_reasons!` 11 語＋`ALL_KINDS` 完全一致テスト | ○ |
| 8.1・8.4 | `log_failure` 1 か所・`count_levels` で件数判定・字面の見張り | ○ |
| 9.1〜9.5 | `manifest_tests`／`diff_tests`／`run_tests`／`commit_tests`／`delete_tests` の固定入力が要件の列挙を全て含む | ○ |
| 9.7 | `winhttp_real_tests.rs`（`#[ignore]`・`TcpListener`・`/r/` で 302・`--release`） | ○ |
| 10.2・10.4・10.8 | `md5.rs`（`BCryptHash`）・機能フラグはクレート側・`log-capture-kit` は dev 依存のみ | ○ |

### (b) 既存コードへの言及（実物と照合）

| 設計の主張 | 実物 | 判定 |
|---|---|---|
| `std::fs::rename` はファイル相手だと既存を置き換える | `library/std/src/fs.rs` 2747 行「replacing the original file if `to` already exists」・`sys/fs/windows.rs` 1322 行 `MoveFileExW(…, MOVEFILE_REPLACE_EXISTING)` | ○ |
| `windows 0.62.2` に `Win32_Networking_WinHttp`・`Win32_Security_Cryptography` の機能 | レジストリ `windows-0.62.2/Cargo.toml` 505 行・518 行 | ○ |
| `BCryptHash(BCRYPT_ALG_HANDLE, Option<&[u8]>, &[u8], &mut [u8]) -> NTSTATUS`・`BCRYPT_MD5_ALG_HANDLE = 33` | `Cryptography/mod.rs` 247 行・3664 行 | ○ |
| WinHTTP のエラー番号 12007／12029／12030／12002／12175／12188／12156 | `WinHttp/mod.rs` 357・334・340・378・373・374・362 行 | ○ |
| `windows_core::Error::from_thread()` | `windows-result-0.4.1/src/error.rs` 118 行 | ○ |
| 機能をクレート側で足す前例（`pilot` は `[dependencies]`・`dola` は `cfg(windows)`） | `crates/pilot/Cargo.toml` 29 行・`crates/dola/Cargo.toml` 32〜33 行 | ○ |
| 根の `Cargo.toml` の `windows` 機能一覧に上記 2 機能が無い | `[workspace.dependencies.windows]` 30 機能・該当無し | ○ |
| 1,000 行の見張りは `crates/**/*.rs` を自動で数える | `log-capture-kit/tests/workspace_scan/mod.rs` 45 行（`LINE_LIMIT = 1000`）・80 行 | ○ |
| `temp_path_guard_test` が `std::env::temp_dir` を禁じる | `temp_path_guard_test.rs` 5 行・362 行 | ○ |
| `areka-nar` の `WorkArea`／`commit_one`／`Undo`／`unwind` は `pub(crate)` 以下・退避先は作業フォルダの中 | `install.rs` 68・81・302・376・436 行・`retired` は `dir.join("old-k")` | ○ |
| `refuse_reasons!`（`kind()`＋`ALL_KINDS`）・`log_failure` の `error!` 1 回・字面の見張り・語彙の完全一致テスト | `error.rs` 16・34・41 行・`lib.rs` 228・239 行・`lib_tests.rs` 307 行・`lib_vocabulary_tests.rs` 211 行 | ○ |
| `hold`（`share_mode(FILE_SHARE_READ)`）・`names.rs` は `SHIFT_JIS` を直に呼ぶ | `install_commit_tests.rs` 39〜42 行・`names.rs` 91 行 | ○ |
| `count_levels`・`capture`・`CapturedEvent::field`・`SampleRoot::acquire`／`folder`・`WorkDir` | `event.rs` 200 行・`capture.rs` 99 行・`event.rs` 91 行・`sample-ghost-kit/src/lib.rs` 151・228 行・`devroot.rs` 154 行 | ○ |
| `[profile.release]` は `opt-level = 'z'`・`lto = true` | 根 `Cargo.toml` 92〜96 行 | ○ |
| `crates/areka-update` は未使用の名前・`about.hbs`／`about.toml`／`deny.toml` が在る・`structure.md` の 5 行の型 | `crates/` に衝突無し・3 ファイル実在・`structure.md` 330〜335 行 | ○ |

### (c) 正典（ukadoc）との整合

- 行フォーマット「`ファイルパス\x01MD5ハッシュ\x01拡張フィールド…`・`\x01` はバイト値 1・改行は CRLF」＝設計手順 3・4 のとおり（LF も受け入れるのは要件 1.5 の決定）。
- 行種別「`file,` でも `charset,` でもない行は無視」「`charset,` は以降の行の文字コード」＝設計手順 1・3 のとおり。
- 拡張フィールド「`charset=` は `updates2.dau` の先頭エントリの末尾にのみ・文字コード判定のみに使われ読み込み時は無視」＝設計手順 1・4 のとおり。
- 文字コード「デフォルトは OS デフォルトの charset」→ 裁定 ⑵ で Shift_JIS 固定（再検討しない）。
- セキュリティチェック「MD5 がない／末尾が `/` または `\`／`..\` や `../` を含む」＝`InvalidWhy::{NoMd5, FolderEntry, DotDot}`。
- URL エンコード「全エントリがパーセントエンコード済みならデコードして使用・そうでなければ URL 側のみエンコード」＝`urlpath` の判定。復号後の文字コードは正典が沈黙しており、設計の決定（UTF-8 → 定義ファイルの文字コード）は妥当。
- `manual_update`「`delete.txt` はサーバ上で `updates2.dau` と同じ位置」「`delete1.txt`（SSP のみ・`delete[数字].txt` を認識可）」・`dev_update`「上書きであって同期ではない」＝設計 6.1・2.3 のとおり。

### (d) 設計内部の整合

- 失敗の語彙は `fail_reasons!` の 11 変種＝「要件 7.4 の 10＋`EscapesTarget`」で、本文・表・Open Questions の数が一致する。
- 無効エントリは `InvalidWhy` 9 変種＝「正典 3＋追加 6」で、Traceability（「追加 5 種＋作業場所」）・Testing・Security の数え方が一致する。
- 段 `Stage` は 6 変種で、削除の段を持たない理由（6.6 で失敗にしない）が Open Questions と一致する。
- 確定の手順「退避 → 置く」の順序と `Undo` の逆順は正しい。置き換えのあるファイルは `Restore` を積んでから `Remove` を積み、解くときは `Remove`（新しい内容を消す）→ `Restore`（元を戻す）の順になる。途中で止まった走行の後は、次の走行の差分計算が「無い」を要取得にして自然に直る。読み取り専用のファイルは `rename` では動かせ `remove_file` では失敗するので、「戻せなかった」の固定入力の作り方は成り立つ。
- 例外は下記の重大 1・2。

### (e) リポジトリの制約

- 外部クレートの追加 0（`encoding_rs`・`thiserror`・`tracing`・`windows` はすべて既存の workspace 依存）。
- 1 ファイル 1,000 行は見張りが自動で数える。`work.rs`＋`commit.rs` で 350 行前後・`winhttp.rs` 100〜150 行の見立ては妥当。
- 常時テストはネットへ出ない（本物の取得口は `#[ignore]` のみ・字面の見張りで `WinHttp` の綴りを `winhttp.rs` に閉じる）。
- 記録は `error!` を `log_failure` 1 か所に閉じ、他は警告をデータで返す（`logging.md`・areka-nar の前例と同じ）。

## 重大な懸念（最大 3 件）

### 🔴 重大 1: 「戻せなかった」ときに残した作業場所を、次の走行が黙って消す

**懸念**: `WorkArea::create` は「棚 `.update-work/` に残る他の走行の残骸を先に消す」（`work` の Concurrency strategy・`areka-nar` の `prepare_shelf` と同じ）。一方 `RollbackFailed` のときは `WorkArea::keep` で作業場所を残し、`UpdateError::work` に「元の内容が残る場所」として返す（5.5）。利用者が失敗の案内を見て**もう一度更新を試す**（最も自然な行動）と、2 回目の `WorkArea::create` が 1 回目の `old/` ごと `remove_dir_all` する。要件 5.5 が約束した「元の内容が残っている」は、次の一周の最初の数ミリ秒で嘘になる。
**影響**: 戻せなかった半端な状態から利用者が手で復旧する唯一の手がかりが消える。同じ弱点は `areka-nar` にも在るが、本仕様は 5.5 で「残す」と明言している分、約束の破り方が明確になる。
**提案**: 棚の片付けで「`old/` に中身が残るフォルダ」は消さず残骸（`leftovers`）に列挙する、または `keep` のときにフォルダ名を `kept-<pid>-<連番>` に改名して片付けの対象から外す、のどちらか（5〜10 行）。`work_tests` に「戻せなかった走行の後にもう 1 周回しても `old/` が残る」の固定入力を 1 本足す。
**Traceability**: 要件 4.8・5.5・7.3
**Evidence**: design.md「`work`（作業場所）」Concurrency strategy／「`commit`」Idempotency & recovery／「確定と戻し」の `RF` 枝

### 🔴 重大 2: 確定で「親フォルダを作る」が「実パスが配下か」の検査より先に走る

**懸念**: 「確定と戻し」の流れ図（`P → MK → CK`）と `commit` の Batch contract（⑴ `create_dir_all` → ⑵ `resolves_under`）は、検査の前にフォルダを作る順序になっている。差分の段でも一度検査しているが、差分と確定の間には取得（ネット・数秒〜数分）が挟まる。その間に対象フォルダの中のフォルダが外を指すジャンクションに置き換わっていると、確定はまず外側にフォルダを作り、その後で `Escapes` を返す。要件 5.6「外へ解決されるパスを決して作らない」に反する（戻しの `RemoveDir` で空なら消えるが、「作らない」ではない）。
**影響**: 第三者のフォルダを書き換える口として、検査を先に置くのが原則。`resolves_under` は「最も深い実在する祖先」を見る設計なので、作る前に呼べる。
**提案**: 順序を ⑴ `resolves_under` → ⑵ `create_dir_all` → ⑶ 退避 → ⑷ 置く、に入れ替え、流れ図と Batch contract の両方を直す（コードは行の入れ替えのみ）。`paths_tests` のジャンクションの固定入力を `commit_tests` でも 1 本通す（「外を指す親の下には何も作られない」）。
**Traceability**: 要件 5.6
**Evidence**: design.md「System Flows › 確定と戻し」流れ図・「`commit` › Batch / Job Contract › Input / validation」

## 設計の強み

1. **前例の写し方が的確**: `areka-nar` から「思想と語彙」（作業場所で組む → 退避 → 置く → 逆順に戻す・`rolled_back`／`work`／`leftovers`・`refuse_reasons!`・`log_failure` 1 か所・字面の見張り・`ALL_KINDS` の完全一致テスト）だけを写し、部品は共有しない。並走 spec との共有 0・外部依存 0 を、既存コードの実測に基づいて成立させている。
2. **決定論テストの網が要件の列挙を漏らさない**: 9.1〜9.5 の固定入力の列挙が要件の語句と 1 対 1 で対応し、「戻せなかった」経路まで偽の取得口の `on_get` で決定論的に組む手順が具体的（`areka-nar` が検査できなかった経路を最初から持つ）。実機の一周も `#[ignore]` テスト＋`TcpListener` で cargo だけで再現できる。

## 最終判定

**GO**（条件付き）

**根拠**: 要件との対応・既存コードの主張・正典との整合・依存と記録の規律はすべて実物で裏が取れ、構造上の誤りは無い。重大 1・2 は「全か無か」の約束の縁にある手順の穴だが、どちらも数行の入れ替え・追加で塞がり、部品の分け方や公開面を変えない。設計ディスカッションで 2 件を反映してからタスク生成へ進む。

**次の段**: `/kiro-design-discussion areka-P0-update-engine` で重大 1・2 と下記の軽微な観察を裁き、`design.md` を直してから `/kiro-spec-tasks areka-P0-update-engine`。

## 軽微な観察（ディスカッションの候補）

1. 「読めない `delete.txt` → 警告して続行」（`delete_tests`）に対応する `UpdateWarning` の変種が無い（8 変種のどれにも当たらない）。`DeleteFileUnreadable { file, source }` を足すか、`Undeletable` に寄せるかを決める。
2. `run_tests` の成功の順序「`DownloadBegin`×n → `Md5Compared`×n」は、Event Contract の「(`DownloadBegin` → `Md5Compared`)×n」（1 件ごとに交互）と読みが違う。表記を揃える。
3. `charset=` の先読みを「位置 2 以降」で探すのは正典「先頭エントリの末尾にのみ」より緩い。害は無いが、要件 1.7 の逐語（末尾）に合わせるなら「最後の欄」に限定する。
4. `urlpath::encode` の「残す文字」を未予約文字＋`/` に限ると、URL のパスで使える `!`・`(`・`)`・`,`・`=`・`@` 等も符号化する。サーバは復号するので実害は無いが、要件 1.15「URL のパスに使えない文字」より広い。方針として書き留めるだけでよい。
5. 確定の完了から `delete.txt` の適用までの間で落ちると、次の走行は差分 0 になり `delete.txt` を読まない（6.1 の決定）ので、作者が定義ファイルを次に変えるまで削除が適用されない。窓は小さく、要件の決定どおりなので設計は正しい。知っておく事項として記す。
6. `delete.txt` の「フォルダの行」がファイルを指すときの振る舞いが書かれていない（`remove_dir_all` が失敗し `Undeletable` の警告になる＝安全側）。1 行明記しておくと迷わない。
7. 失敗の結果（`UpdateError`）は要取得の一覧を持たない（`Stage::Download { index, total }` の件数だけ）。要件 2.6「結果と進捗に含める」は成功側と `Progress::DiffDecided` で満たすと読めるが、`network-update` が失敗時の Ref に一覧を要するなら足す。
