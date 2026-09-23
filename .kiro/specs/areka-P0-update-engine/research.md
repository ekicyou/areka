# Gap Analysis: areka-P0-update-engine

> 2026-09-23 `/kiro-validate-gap`。本文の file:line・件数は**全て本ブランチ（main `92f5f448` 起点・worktree `areka-p0-update-engine-9a173d`）で数え直した値**である。brief（09-20）の数字は写していない。
> 入力: `requirements.md`（確定・触らない）・`brief.md`・親 brief `.kiro/specs/areka-P0-network-update/brief.md`・steering（`tech.md`・`structure.md`・`logging.md`・`roadmap.md` 台帳 #51/#52 と A1 干渉台帳）。
> 本文書は**情報と選択肢**を出す。決めるのは要件ディスカッション（末尾「設計判断の項目」）。

## 1. 分析の要約

- **ネットワーク更新に当たる実装は 0 のまま**（本ブランチで再測定）。`WinHttp|WinInet` 0 ファイル・`updates2|updates\.txt` 0・`OnUpdate` 0・`delete\.txt` は `crates/areka-parsers/src/package/validation_tests.rs` の 1 ファイルだけ・`GetACP` 0・パーセント符号化の実装 0・`TcpListener` 0。`homeurl` は `areka-sylphya` の語彙 2 ファイルと、`areka-nar` のテストが「知らないキー」として通す 1 か所にしかない。
- **新しいクレートは既存ソースに触らずに建つ。** `Cargo.toml` の `members = ["crates/*"]`、`windows` の機能はクレート自身の `Cargo.toml` で足す前例が **3 つ**（`dola`・`areka-emo-text`・`pilot`）、必要な機能名 `Win32_Networking_WinHttp`・`Win32_Security_Cryptography` は `windows 0.62.2` に実在し、要る関数・定数（WinHTTP 11 本・BCrypt 8 本）も全て束縛済み。ライセンスの関門（`cargo-deny 0.20.2`・`cargo-about 0.9.2`）は手元に入っている。
- **`areka-nar` の確定の部品は再利用に向かない**（`pub(crate)` であること以上に、単位が「フォルダ丸ごとの `rename`」で、`names.rs` は `\` を拒否するが本仕様は `\` を区切りとして受け入れる）。同じ**思想**（作業場所で組む → 退避 → 置く → 失敗なら逆順に戻す）と**語彙**（`rolled_back`・`work`・`leftovers`）を写し、ファイル単位の確定を新クレートに持つのが最短で、並走 #52 との共有も 0 になる。
- **再利用できる既存の部品は「試験の道具」と「規律の型」である。** `log-capture-kit::count_levels`（レベル別件数）・`sample-ghost-kit::{SampleRoot, WorkDir}`（検体の複製・空の作業フォルダ）・`areka-nar` の `refuse_reasons!` マクロ（閉じた語彙と `ALL_KINDS` の全数対応テスト）・`install_commit_tests.rs` の `hold()`（読み共有で開いたままにして確定を失敗させる注入）と `tree()`（バイト単位の木の写し）。文字コードは `areka_parsers::charset::decode` を**そのまま**は使えない（`updates2.dau` の `charset=` を見ない・解決不能を `debug!` で流す）ので、`encoding_rs::Encoding::for_label`＋`SHIFT_JIS` を直接使う（`areka-nar/src/names.rs` と同じ）。
- **設計へ持ち越す研究項目**は WinHTTP の細部（転送の既定方針・プロキシ種別・時間切れの値・本文の受け方）と、パーセント符号化を**復号した後**のバイト列を何の文字コードで読むか（正典が沈黙）の 2 群。規模は **M・リスク Medium**（unsafe の FFI が新しいが小さい／全か無かの確定は前例の設計を写せる）。

## 2. 現状の実測（Current State Investigation）

### 2.1 ワークスペースと登記の型

| 項目 | 実測 | 意味 |
|---|---|---|
| メンバー | `Cargo.toml` `[workspace] members = ["crates/*"]`・`crates/` 直下 27 クレート | クレートを足しても根の `Cargo.toml` に差分 0（要件 10.4） |
| 根の `windows` 機能 | `[workspace.dependencies.windows]` に 30 機能。`Win32_Networking_*`・`Win32_Security_*` は無い | 新クレートが自分で足す |
| クレート側で機能を足す前例 | `crates/dola/Cargo.toml` `[target.'cfg(windows)'.dependencies] windows = { workspace = true, features = ["Win32_System_Performance"] }`／`crates/areka-emo-text/Cargo.toml` `features = ["Win32_Graphics_Dxgi"]`／`crates/pilot/Cargo.toml` 3 機能 | `workspace = true` に `features` を上乗せする形で足せる |
| `windows 0.62.2` の機能名 | レジストリ `windows-0.62.2/Cargo.toml`: `Win32_Networking_WinHttp = ["Win32_Networking"]`・`Win32_Security_Cryptography = ["Win32_Security"]` | 機能名は確定。`Win32_Security` は `shiori-host32-host` がクレート内で足している（根には無い） |
| `Cargo.lock` | `.gitignore` 2 行目で追跡外。main の実物は 261 パッケージ。`md-5`／`md5`／`digest`／`percent-encoding`／`url` は **無い**（`cpufeatures` だけ在る） | `md-5` を採れば新規パッケージが増える。パーセント符号化は自前 |
| レジストリ（オフライン解決の可否） | `c:/rust/cargo/registry/src/…/md-5-0.11.0`（依存 `cfg-if 1`・`digest 0.11`・MIT OR Apache-2.0）・`digest-0.11.3`（MIT OR Apache-2.0）が手元に在る | 採る場合もオフラインで解決できる見込み（推移的依存の全数は `cargo tree` で実測が要る＝研究項目） |
| ライセンスの関門 | `deny.toml` allow 9 種・`[graph] all-features = true`・`about.toml` `ignore-dev-dependencies = true`。手元に `cargo-deny 0.20.2`・`cargo-about 0.9.2`・`rustc 1.98.1` | `md-5` は MIT OR Apache-2.0 で通る |
| `THIRD-PARTY-NOTICES.md` | **ワークスペースのクレートも載る**（`areka-nar 0.0.1` L2010・`sample-ghost-kit 0.0.1` L2019・MIT の項に 211 crate） | 新クレートで必ず 1 行増える＝依存の増減が無くても再生成（要件 10.5）。生成コマンドは `cargo about generate --workspace about.hbs -o THIRD-PARTY-NOTICES.md`（`.claude/skills/kiro-complete/SKILL.md` L143） |
| `tech.md` の登記の型 | 「意図的依存追加」の項＝`encoding_rs`（2026-07-02 承認済）・`miniz_oxide`（2026-09-18 承認済・既定機能を切る理由・見張りのテストの所在まで書く） | 要件 10.2／10.3 の書式はこれに倣う |
| `structure.md` の登記の型 | 「NAR Container Crate（areka-nar）」の項＝Location／Purpose／Modules／Dependencies／規律 の 5 行 | 要件 10.6 の書式 |
| 1,000 行の番人 | `crates/log-capture-kit/tests/file_length_guard_test.rs`（`LINE_LIMIT` 1000・例外表 10 件）。走査は `crates/**/*.rs` を列挙（`workspace_scan/mod.rs` L80〜89） | 新クレートは**自動で**対象になる。`areka-nar/src/lib_tests.rs` L383〜390 は新設分だけを自分でも数えている |
| 一時パスの番人 | `temp_path_guard_test.rs`＝`std::env::temp_dir` の呼出が `temp-path-kit` の定義と例外表の外に 1 件も無いことを見張る | **本番コードの作業場所を OS の一時フォルダに取れない**。`areka-nar` は `<根>/.nar-work/<pid>-<連番>/` を根の直下に掘っている（`install.rs` L21・`WorkArea::create`）＝同じボリュームで `rename` が効く |
| 検体の番人 | `sample_path_guard_test.rs`＝`sample-ghost-kit` が `[dependencies]`・`[build-dependencies]`・`[target.'cfg(windows)'.dependencies]` に現れたら赤（L692〜697） | 検体は `[dev-dependencies]` からのみ |
| 記録の番人 | `with_default_guard_test.rs`＝自前の subscriber 差し込みを禁じる | テストの記録の捕捉は `log-capture-kit` 経由のみ（要件 10.8） |

### 2.2 `areka-nar`（確定の前例）の形

`crates/areka-nar/src/install.rs`（468 行）・`lib.rs`（252 行）を読んだ結果。

| 部品 | 可視性 | 単位・振る舞い | 本仕様への当てはまり |
|---|---|---|---|
| `WorkArea::create(root)` | `pub(crate)` | `<根>/.nar-work/<pid>-<連番>/` を掘る。棚の残骸を先に片付け、消せなかったものを `residue` に持つ | 思想はそのまま使える（作業場所は対象フォルダの直下・同じボリューム） |
| `stage_placement(stage, placement, contents)` | `pub(crate)` | **配置（フォルダ）1 つ**の完成形を作業フォルダに組む。既存の宛先の木を丸ごと複写してから書庫の中身を上書き | 更新は「在る木の一部のファイル」なので複写が要らない。単位が違う |
| `commit_all` → `commit_one` | `pub(crate)` | 宛先フォルダを `old-<k>` へ `rename` → 作業フォルダを宛先へ `rename`。`Undo::{Remove, Restore}` を積み、失敗で逆順に解く（`unwind`） | **ファイル単位に写せる**（`old/<相対パス>` へ退避 → 置く → 失敗で逆順）。`rename` はフォルダ相手だと既存を上書きしない（`install.rs` の `Undo::Restore` の注記）が、**ファイル相手の `std::fs::rename` は Windows では既存を置き換える**（`MOVEFILE_REPLACE_EXISTING`）＝退避を先にしないと元の内容が消える |
| `CommitError { phase, path, source, committed, rolled_back }`・`InstallOutcome { leftovers, .. }` | 型は `pub`（`error.rs`・`install.rs`） | 要件 5.5／7.3 の「戻せたか」「残骸」はこの語彙 | 語彙を写す（型を共有する必要は無い） |
| `log_failure`（`lib.rs`） | private | `Err` を返す直前に `tracing::error!` を **1 回**、欄は `archive`・`reason`・`committed`・`rolled_back`・`work` | 要件 8.1 の形そのもの。スコープ接頭辞 `[areka_nar]` |
| `names.rs::validate_one` | `pub(crate)` | NUL → **`\` を拒否** → 絶対（`/` 始まり・ドライブ文字）→ `..` → 空要素 → Windows 予約名 → シンボリックリンク → 大小の衝突（`to_lowercase` で全パス比較） | 順序と語彙は写せるが、**`\` の扱いが逆**（要件 1.13 は `\` を区切りとして受け入れる）。関数のまま流用は不可 |
| `crc32.rs`（51 行） | `pub` | `const` の表引き。較正値 `b"123456789" → 0xCBF43926` を doctest に持つ | 自前 MD5（裁定候補 ⑴-⑶）の書き方の前例。ハッシュの crate を入れない方針の逐語（L9） |

**再利用の結論（裁定候補 ⑶ の材料）**: 公開するなら `pub(crate) → pub` の変更が `install.rs`（4 か所）・`lib.rs`（`mod install` の公開）に及び、`crates/areka-nar/src/` を #52（`nar-install-hardening`・A1 並走）と共有する（`roadmap.md` A1 干渉台帳「条件付きの重なり 1 件」＝③を先に着地させる条件付き）。しかも使えるのは `WorkArea`（棚の管理・約 80 行）だけで、`stage_placement`／`commit_one` は単位が違うので写しても改造になる。**新クレート内にファイル単位の確定を持つ ⒝ が、差分 0・共有 0・改造 0 で最短。**

### 2.3 文字コードと行の分解（既存層の当てはまり）

| 部品 | 公開面 | 使えるか |
|---|---|---|
| `areka_parsers::charset::decode(bytes, DefaultEncoding) -> String`（`decode.rs` L24） | `pub` | **部分的**。⑴ 先読み `prescan_charset` は「最初のカンマで分けて key が `charset`」の行しか見ない（`prescan.rs`）＝`updates.txt` の `charset,<名>` 行には当たるが、**`updates2.dau` の先頭エントリ末尾の `charset=<名>`（`\x01` 区切り・`=`）は見ない**。⑵ 解決できないラベルは `tracing::debug!` で既定へ後退（要件 1.10 は **warn**）。⑶ 復号の損失も `debug!`。⑷ `DefaultEncoding::Ansi → SHIFT_JIS` の固定写像 `to_encoding` は `pub(crate)` |
| `encoding_rs`（workspace 依存・意図的依存追加 2026-07-02 承認済） | — | **そのまま使う**。`Encoding::for_label(name)`＋`SHIFT_JIS` の 2 行で要件 1.8〜1.10 が書ける。`areka-nar/src/names.rs` L86 が `encoding_rs::SHIFT_JIS.decode_without_bom_handling` を直に呼ぶ前例。新クレートの依存に足しても登記は不要（既承認） |
| `areka_parsers::kv::parse_kv(text) -> BTreeMap<String,String>` | `pub` | **不可**。同一キー後勝ちの写像なので `file,` 行が複数ある `updates.txt` を潰す。順序（要件 2.6）も失う |
| `DefaultEncoding` の思想（OS ロケール不読・固定写像） | `model.rs` L11〜14 | 裁定候補 ⑵-⒜ の根拠。ワークスペースに `GetACP` の呼出は 0（`pilot` の `WideCharToMultiByte(CP_ACP)` は host-32 の先進坑だけ） |

**結論**: 新クレートは `areka-parsers` に依存しなくてよい（依存しても新規登記は不要だが、使える関数が無い）。`\x01` 分割・`file,`／`charset,` の行種別・`key=value` 拡張フィールドは自前（純関数・50〜100 行）。

### 2.4 試験の道具（そのまま使える）

| 道具 | 公開面（実測） | 本仕様の要件 |
|---|---|---|
| `log_capture_kit::count_levels(f) -> (R, LevelCounts { error, warn, info, debug, trace })`（`event.rs` L184〜215） | dev 依存 | 要件 8.4「失敗 1 回に error 1 件・無効エントリ 1 件に warn 1 件」を**件数で判定** |
| `log_capture_kit::capture(f) -> (R, Vec<CapturedEvent>)`＋`CapturedEvent::field(name)` | dev 依存 | 要件 8.1 の欄（URL・対象・段・理由・ファイル名）を値で判定 |
| `sample_ghost_kit::SampleRoot::acquire("emo2")`／`.folder()`（複製は `target/` 配下の名前空間・破棄で消える）・登記表 `SAMPLES` は 5 検体（`emo2`・`R_POST_and_KOMAINU`・`emo2-kakukaku-offsetdpi`・`emo2-kakukaku-wplimit`・`konnoyayame`） | dev 依存 | 要件 9.7「検体ゴーストの複製を対象フォルダに」・決定論テストの「本物の木」 |
| `sample_ghost_kit::WorkDir::new()`（空の作業フォルダ・`target/` 配下・札付き） | dev 依存 | `areka-nar` が「空の根」に使っている。固定入力の小さな木はこれに `make_tree` する |
| `temp_path_kit::TempPath::new(label)`（OS の一時フォルダ・pid＋連番） | dev 依存 | `WorkDir` で足りるなら不要（どちらも可） |
| `install_commit_tests.rs::hold(path)`（L39〜45・`share_mode(FILE_SHARE_READ)` で開いたまま持つ→`rename` が os error 5） | 私有ヘルパ（写す） | 要件 9.3「確定の途中失敗（注入）」・要件 5.7「開かれている SHIORI」 |
| `install_commit_tests.rs::tree(root) -> BTreeMap<String, Vec<u8>>`（相対パス→バイト列） | 私有ヘルパ（写す） | 要件 9.4「バイト単位で同一」 |
| `lib_vocabulary_tests.rs::every_refusal_kind_has_a_fixture_and_is_recorded_once`（L211）＝各変種の固定入力を 1 本で組み、得た `kind()` の集合を `ALL_KINDS` と**完全一致**で突合・失敗ごとの error 件数も同時に数える | 私有（写す） | 要件 7.4「閉じた語彙の各項目に固定入力 1 つ以上」・8.4 |
| `error.rs` の `refuse_reasons!` マクロ（`kind()`＋`ALL_KINDS` を 1 宣言から生成） | 私有（写す） | 要件 7.4 の語彙を二重管理せずに閉じる |
| `lib_tests.rs::only_the_public_surface_writes_records`（L307）＝本番ソースで `tracing::` を綴るファイルが 1 つ・`tracing::error!` が 1 か所であることを字面で判定 | 私有（写す） | 要件 8.1「二重記録しない」の静的な側 |
| `crates/pilot`（`examples/<spec>/main.rs`＋README 3 幕・`cargo run -p pilot --example <spec>`・dev 依存に `sample-ghost-kit` 済み・`windows` は既に依存） | 先進坑 | 要件 9.7 の実機の一周の置き場。**ローカル HTTP は無い**（`TcpListener` 0 件）→ `std::net::TcpListener` で静的配信 60 行前後を example に置くか、外部のサーバ（`python -m http.server` 等）を手順にする |

### 2.5 OS の機能の束縛（`windows 0.62.2`・レジストリ実測）

- **WinHTTP**（`src/Windows/Win32/Networking/WinHttp/mod.rs`）: `WinHttpOpen`・`WinHttpCrackUrl`・`WinHttpConnect`・`WinHttpOpenRequest`・`WinHttpSetTimeouts`・`WinHttpSetOption`・`WinHttpSendRequest`・`WinHttpReceiveResponse`・`WinHttpQueryHeaders`（`WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER` で状態コードを数値で取れる）・`WinHttpReadData`・`WinHttpCloseHandle`。定数 `WINHTTP_FLAG_SECURE`・`WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY`／`DEFAULT_PROXY`・`WINHTTP_OPTION_REDIRECT_POLICY`（`_ALWAYS`／`_DISALLOW_HTTPS_TO_HTTP`）・`ERROR_WINHTTP_TIMEOUT`／`NAME_NOT_RESOLVED`／`CANNOT_CONNECT`／`SECURE_FAILURE`。要件 3.4〜3.6 の語彙（転送追随・状態コード・名前解決／接続／時間切れ）は全てここから引ける。
- **CNG**（`src/Windows/Win32/Security/Cryptography/mod.rs`）: `BCryptHash`（一発でハッシュ）＋擬似ハンドル `BCRYPT_MD5_ALG_HANDLE`（Open／Close 不要）で MD5 が **1 呼出**。段階的な `BCryptOpenAlgorithmProvider`／`CreateHash`／`HashData`／`FinishHash` も在る。
- 既存ソースで `BCrypt`／`WinHttp` を綴るファイルは 0＝**unsafe の FFI をこの 2 系統で初めて書く**（規模は各 50〜120 行）。

## 3. 要件 → 資産の対応表（Requirement-to-Asset Map）

凡例: **Missing**＝新規に書く／**Reuse**＝既存をそのまま使う／**Pattern**＝既存の形を写す／**Constraint**＝既存の規律が縛る／**Unknown**＝設計で研究。

| 要件 | 必要な部品 | 既存資産 | 判定 |
|---|---|---|---|
| 1.1〜1.4 定義ファイルの取得順・後退の条件・末尾 `/` | 取得の境界＋「見つからない」の区別 | 無し。WinHTTP の状態コードは数値で取れる（2.5） | Missing（境界の型は 3 と共通） |
| 1.5〜1.7・1.11〜1.14・1.16 行の形・拡張フィールド・無効エントリ・重複 | 純関数の読み手 | `areka-nar/names.rs` の検査順序（写す・`\` の扱いだけ逆）・`parse_kv` は不可（2.3） | Missing（Pattern） |
| 1.8〜1.10 文字コード | `for_label`＋既定 | `encoding_rs`（Reuse）・`charset::decode` は不可（2.3） | Reuse＋Missing（`charset=` の先読み） |
| 1.15 パーセント符号化の判定・復号・符号化 | 判定と変換 | 無し（`percent-encoding` は lock に無い） | Missing（自前 30 行前後）。**復号後のバイト列の文字コードは Unknown**（正典が沈黙・§6） |
| 2.1〜2.6 差分（MD5） | MD5＋木の走査 | MD5 は裁定候補 ⑴（CNG 1 呼出／`md-5`／自前 100 行） | Missing |
| 3.1〜3.2 境界と偽実装 | trait＋固定表＋失敗注入 | 無し（型は自明） | Missing |
| 3.3〜3.6 本物の実装（WinHTTP） | unsafe FFI | 束縛は全て在る（2.5）・ワークスペースに前例 0 | Missing（Unknown＝既定の転送方針・プロキシ・時間切れ値） |
| 3.7 同期・スレッドを起こさない | — | `areka-nar` も同期 | Constraint（設計で明記） |
| 4.1〜4.8 作業場所への取得と照合・片付け | 作業場所の管理 | `WorkArea` の思想（Pattern）。**OS の一時フォルダは番人が禁じる**（2.1）→ 対象フォルダ直下（`.update-work/<pid>-<連番>/` 等） | Pattern＋Constraint |
| 5.1〜5.8 全か無かの確定・戻し | ファイル単位の退避→置く→逆順の戻し | `commit_one`／`Undo`／`unwind` の思想（Pattern・単位を変える）。実パスの配下確認は `every_file_resolves_under`（`install_commit_tests.rs` L74）の形 | Pattern |
| 6.1〜6.7 `delete.txt` | 行の読み手＋安全な削除 | 拒否 3 種は `names.rs` の順序を写す。フォルダの中身ごと消すのは `remove_tree`（`install.rs`） | Missing（Pattern） |
| 7.1 進捗の観測者 | 型 | 無し | Missing（形は §6 の設計項目） |
| 7.2〜7.4 結果と閉じた語彙 | enum＋`ALL_KINDS` | `refuse_reasons!`（Pattern）・`CommitError` の欄（語彙を写す） | Pattern |
| 7.5〜7.6 SHIORI に触らない・0 始まり | — | 依存に kanade・sylphya を置かなければ構造的に満たす | Constraint |
| 8.1〜8.4 記録 | `error!` 1 か所・warn・info・件数判定 | `log_failure`（Pattern）・`count_levels`（Reuse）・字面の見張り（Pattern） | Reuse＋Pattern |
| 9.1〜9.6 決定論テスト | 固定入力・偽の取得口・木の写し・注入 | `WorkDir`・`SampleRoot`・`hold`・`tree`（Reuse／Pattern） | Reuse |
| 9.7 実機の一周 | ローカル HTTP＋検体の複製＋`opt-level='z'` | `pilot` の型（Reuse）・HTTP サーバは無い | Missing（60 行前後 or 外部手順） |
| 10.1〜10.8 登記・関門 | `tech.md`／`structure.md`／NOTICES 再生成／`cargo deny` | 型と道具は揃っている（2.1） | Constraint（手順） |

## 4. 実装アプローチの選択肢

### Option A: `areka-nar` を拡張する（同クレート内に `update` モジュール）

- **中身**: `crates/areka-nar/src/update/…` を足し、`WorkArea`・`remove_tree`・`names.rs` の検査を `pub(crate)` のまま呼ぶ。
- **利点**: 作業フォルダの棚（`.nar-work`）の片付けと語彙を 1 か所で持てる。公開範囲を広げなくてよい。
- **欠点**: ⑴ `crates/areka-nar/src/` を #52 と共有（並走の前提が崩れる）。⑵ クレートの目的（`.nar` を読む）から外れ、`areka-nar` は既に 19 ファイル 8,273 行。⑶ `windows` 依存（WinHTTP・CNG）が `areka-nar` に入り、`sample-ghost-kit` 経由で全テストの依存グラフに乗る。⑷ brief と roadmap #51 の「新規クレート」に反する。
- **判定**: 採らない理由が 4 つ。**非推奨**。

### Option B: 新クレート 1 つに閉じる（推奨・要件生成者の見立て ⒝ と同じ）

- **中身**: `crates/areka-update/`（名前は brief の仮称・`crates/` に衝突無し）。依存は `thiserror`・`tracing`・`encoding_rs`・`windows`（`cfg(windows)`・機能 2 つをクレート側で）。dev 依存は `log-capture-kit`・`sample-ghost-kit`（＋必要なら `temp-path-kit`）。ワークスペース内の本番クレートには依存しない。
- **モジュールの当たり**（1 ファイル 1,000 行の内側に収める分け方の一例）: `manifest.rs`（定義ファイルの読み手・純関数）／`charset.rs`（`for_label`＋既定）／`urlpath.rs`（パーセント符号化の判定・復号・符号化）／`md5.rs`（裁定候補 ⑴ の 1 つ）／`diff.rs`（差分）／`fetch.rs`（境界 trait＋偽実装は test 側）／`winhttp.rs`（本物）／`work.rs`（作業場所）／`commit.rs`（ファイル単位の確定と戻し）／`delete.rs`／`error.rs`（閉じた語彙）／`progress.rs`（観測者と結果）／`lib.rs`（公開面と記録の唯一の出口）。
- **利点**: 既存ソースへの差分 0・#52 との共有 0・依存グラフが最小・試験の道具はそのまま。
- **欠点**: `WorkArea` の棚の管理（80 行前後）と `remove_tree`（10 行）が `areka-nar` と**二重に**存在する。思想の共有で許容するか、後日 #52 の後に共通の小クレートへ寄せるかは設計の外（先送りは追跡 spec が要る）。
- **判定**: **推奨**。

### Option C: 新クレート＋`areka-nar` の部品を公開して共用（裁定候補 ⑶-⒜）

- **中身**: `areka-nar` の `WorkArea`（と `remove_tree`）を `pub` にして新クレートが引く。`stage_placement`／`commit_all` はフォルダ単位なので使わず、ファイル単位は結局新クレートに書く。
- **利点**: 棚の管理の二重化を避けられる（80 行）。
- **欠点**: ⑴ `crates/areka-nar/src/` を触る＝#52 の着地を待つ（A1 干渉台帳の条件）。⑵ 新クレートが `areka-nar` に依存し、`miniz_oxide`・`areka-parsers` が更新エンジンの依存に乗る。⑶ 作業フォルダの名前（`.nar-work`）を更新でも使うか、名前を引数にする改造が要る。
- **判定**: 80 行の重複を避けるために並走の独立性と依存の最小性を手放す。**非推奨**。

## 5. 規模とリスク

- **規模: M**（brief の 12〜14 タスクと整合）。純関数の読み手・差分・`delete.txt`・語彙・記録は既存の型を写すだけで S 相当。M に押し上げるのは、⑴ ファイル単位の全か無かの確定と戻しの網羅テスト（注入 4 経路）、⑵ WinHTTP・CNG の unsafe FFI（前例 0）、⑶ 実機の一周の器（ローカル HTTP）。
- **リスク: Medium**。理由: WinHTTP の細部（転送の既定・プロキシ・時間切れ）は設計で研究が要るが、常時テストからは偽実装で切り離せる。確定の設計は `areka-nar` の実証済みの形を単位だけ変えて写せる。第三者のフォルダを書き換える経路の安全性（要件 1.12・5.6・6.3）は `names.rs` と同じ検査順序で閉じられる。

## 6. 設計判断の項目（要件ディスカッションへ）

番号は要件の「裁定候補」を引き継ぎ、本分析で新たに見つけた項目を続ける。

1. **⑴ MD5 の出どころ**（要件 10.2／10.3 が分岐）。実測で加わった材料: ⒜ CNG は `BCryptHash`＋`BCRYPT_MD5_ALG_HANDLE`（擬似ハンドル）で **1 呼出・unsafe 20 行前後**、依存 0、`Win32_Security_Cryptography` の機能 1 行。⒝ `md-5 0.11.0` はレジストリに在り（`digest 0.11`・`cfg-if`）、`deny.toml` を通るが、推移的依存の全数は `cargo tree` の実測待ち・`tech.md` 登記と NOTICES の差分確認が要る。⒞ 自前は `crc32.rs` と同じ書き方で 100 行前後＋RFC 1321 の較正 7 本。**どの案でも決定論テストは「既知の入力 → 既知の 32 桁」の較正を 1 つ持つ**（要件の逐語）。
2. **⑵ 既定の文字コード**。実測: ワークスペースの `GetACP` 呼出 0・`DefaultEncoding::Ansi → SHIFT_JIS` の固定写像が全層の前例。⒝ OS 既定を採る場合だけ `Win32_Globalization` の機能とロケールを読む 1 か所が増える。
3. **⑶ `areka-nar` の再利用**。実測（§2.2）: 公開しても使えるのは `WorkArea` 80 行前後だけ・`names.rs` は `\` の扱いが逆・#52 と `crates/areka-nar/src/` を共有する。**Option B（⒝）を推す**。二重化する 80 行を将来まとめるなら追跡先を決める（先送りには実在の spec が要る）。
4. **作業場所の置き方**（要件 4.1／4.8／5.5 の実体）。番人が OS の一時フォルダを禁じるので**対象フォルダの直下**（例 `<対象>/.update-work/<pid>-<連番>/`）が既定の候補。決めること: 名前・退避の下位フォルダ（`old/<相対パス>`）・他の走行の残骸の扱い（`prepare_shelf` と同じく「消せなければ結果に列挙・止めない」）・`delete.txt` の行がこのフォルダを指したときの扱い（確定後に自分で消すので拒否は不要だが、警告を出すか）。
5. **ファイル単位の確定の手順**（要件 5.2／5.4）。候補: ⒜ 既存あり＝`rename(dest → old/rel)` → `rename(work/rel → dest)` の 2 手・既存なし＝1 手＋親フォルダの作成（作った親フォルダも戻しで消すか）。⒝ `ReplaceFileW`（OS の原子的置換・バックアップ名付き）。⒜ は `commit_one` の写しで前例が実証済み。**`std::fs::rename` はファイル相手だと既存を置き換える**ので、退避を先に置く順序が要る（§2.2）。
6. **取得した内容の持ち方**（要件 4.1／4.7）。`areka-nar` は伸長済みの全内容をメモリに持つ。更新はシェルの絵が数十 MB になり得るので、**1 件ごとに作業場所へ書き、MD5 はバイト列から取ってから書く**（メモリに全件を溜めない）のが候補。上限（本文の最大サイズ）を持つかは設計。
7. **取得の境界の型**（要件 3.1・3.5・3.6）。候補: `trait Fetch { fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> }`・`FetchError` は閉じた語彙（`NotFound`・`Status(u16)`・`NameResolution`・`Connect`・`Timeout`・`Tls`・`Other(code)`）で WinHTTP のエラー番号（`ERROR_WINHTTP_*`）から写像。偽実装は「URL → バイト列の固定表＋失敗の注入」（要件 3.2）で test 側に置く（本番に `#[cfg(test)]` 以外の偽実装を置かない）。
8. **WinHTTP の細部（研究項目・設計で決める）**: ⒜ 転送の方針＝WinHTTP の既定は `DISALLOW_HTTPS_TO_HTTP`（https → http の降格を拒む）。要件 3.4 の「3xx に追随」に降格を含めるか。⒝ プロキシ種別＝`AUTOMATIC_PROXY`（Windows 8.1 以降）か `DEFAULT_PROXY`。⒞ 時間切れの値（`WinHttpSetTimeouts` の 4 つ）。⒟ 本文の受け方（`WinHttpReadData` の反復・Content-Length を信じない）。⒠ `User-Agent` 文字列。⒡ セッション（`WinHttpOpen`）を一周で 1 つ持ち回るか。
9. **パーセント符号化の復号後の文字コード**（要件 1.15・正典が沈黙）。全エントリが符号化済みのとき、復号した**バイト列**をローカルパスにするには文字コードが要る。候補: ⒜ UTF-8 として読み、失敗なら定義ファイルの文字コードへ後退／⒝ 定義ファイルの文字コードで読む／⒞ UTF-8 固定。URL 側の符号化を UTF-8 と決めた（要件の逐語）ことと対称にするなら ⒜ か ⒞。
10. **`delete.txt` の文字コードの取り方**（要件 6.2「定義ファイルと同じ規則」の読み）。⒜ 定義ファイルで**解決した**文字コードを引き継ぐ／⒝ `delete.txt` 単独で再判定（正典の `delete.txt` に `charset` 行は無いので事実上「既定」）。どちらかを設計で明記する。
11. **進捗の観測者の形**（要件 7.1）。候補: ⒜ `enum Progress { … }`＋`&mut dyn FnMut(&Progress)`（最小）／⒝ `trait UpdateObserver` の 6 メソッド。`network-update` がイベントへ写すだけなので ⒜ で足りる見込み。
12. **失敗の閉じた語彙の全数**（要件 7.4）。要件の 7 項目＋実測から加わる候補: 「更新先 URL が不正（`WinHttpCrackUrl` が拒む）」「対象フォルダが実在しない」「作業場所を作れない」「片付け失敗」。`refuse_reasons!` の型で 1 宣言に閉じ、`ALL_KINDS` の完全一致テストを写す。
13. **定義ファイルを対象フォルダへ置く際の名前**（要件 5.3）。`updates.txt` へ後退した周では古い `updates2.dau` が対象に残り得る。残す（触らない＝要件 2.3 の精神）で足りるか。
14. **実機の一周の器**（要件 9.7）。⒜ `crates/pilot/examples/update-engine/` に `std::net::TcpListener` の静的配信（60 行前後）＋`SampleRoot::acquire` の複製を対象にする／⒝ 外部のサーバ（`python -m http.server`）を手順に書く。⒜ は cargo だけで再現でき、README 3 幕に記録を残せる。
15. **クレート名と登記**（要件 10.6）。`areka-update`（brief）で衝突無し。`structure.md` の 5 行の型・`tech.md`（⑴ が ⒝ のときだけ「意図的依存追加」、⒜⒞ のとき「`md-5` は採らなかった」）。

## 7. 設計フェーズへの申し送り（Research Needed）

- WinHTTP: 転送方針の既定値・`AUTOMATIC_PROXY` の可用性・時間切れの推奨値・`WinHttpReadData` の反復と本文の上限（§6-8）。
- `md-5 0.11` の推移的依存の全数（`cargo tree -p md-5` の実測）と NOTICES の差分（⑴ が ⒝ に決まったときだけ）。
- `BCryptHash`＋擬似ハンドルの最小対応 OS（Windows 10 以降で可の見込み・製品の下限と照合）。
- パーセント復号後の文字コード（§6-9）と、正典 `spec_update_file` の「URL エンコード」節の逐語の再確認（ukadoc MCP の id は親 brief の `ukadoc:spec_update_file:*` 系）。
- `std::fs::rename` のファイル相手の置換の挙動（Windows・`MOVEFILE_REPLACE_EXISTING`）を設計の試験方針に逐語で書く。

## 8. 分析の方法（Document Status）

`kiro-validate-gap/rules/gap-analysis.md` の枠組みに沿い、Grep／Glob／Read で本ブランチのソース・`Cargo.toml`・`deny.toml`・`about.toml`・`THIRD-PARTY-NOTICES.md`・steering・`log-capture-kit/tests` の番人・`c:/rust/cargo/registry` の `windows-0.62.2`／`md-5-0.11.0`／`digest-0.11.3` を実測した。`Cargo.lock` は worktree に無い（追跡外）ため main の実物を読み取り専用で参照した。ネットへは出ていない。

## 9. 次の段

`/kiro-requirements-discussion areka-P0-update-engine` で §6 の 15 項目を裁き、`/kiro-design areka-P0-update-engine` へ進む。
