# ギャップ分析: audit-ghost-catalog

> 2026-11-05・枝 `claude/kiro-start-audit-ghost-catalog-c0351e`（HEAD `a7d3c668`・`main` の `88eb0a43` を取り込み済み）。
> 物差しは ponytail のプラグイン 5.3.2 の `ponytail-audit`（`~/.claude/plugins/cache/ponytail/ponytail/5.3.2/skills/ponytail-audit/SKILL.md`）。
> 作法のうち 2 つを外した（要件 1.3・1.4）: 「最大 20 件」は外して全件を載せる。「読まなかった所」は残さない（範囲の 102 ファイルを全部読んだ）。
> 読み方: 11 本の読み手（読むだけのサブエージェント）に範囲を分けて全行を読ませ、出た指摘をここでまとめ、重なりを 1 件に寄せ、番号を振り直した。載せた指摘はどれも、こちらでファイルと行を開いて中身を確かめた（確かめられなかったものは落とした）。

## 1. 範囲の測り直し（要件 1.2）

| | 起票のとき（main `e3c2cbe4`） | 今（HEAD `a7d3c668`） |
|---|---|---|
| ファイルの数 | 92 | **102**（Rust 99＋`Cargo.toml` 3。fixture のファイルは無い） |
| コードの行 | 8,847 | **9,524**（27 ファイル） |
| テストの行 | 22,248 | **24,351**（72 ファイル・`_tests.rs`／`tests/`／`_test_support.rs`） |
| `Cargo.toml` | — | 114 行 |

- クレートごと: `areka-ghost` 64 ファイル・`areka-nar` 28・`areka-user-data` 9（どれも `Cargo.toml` 込み）。
- 数え方の注: `areka-ghost/src/test_log_capture.rs`（165 行）は名前の決まりではコードに入るが、中身はテストの道具。
- 1,000 行の近く: `tests/ghost/inproc_e2e_test.rs` 1,160（番人の例外表に載る）・`runtime_tests.rs` 993・`plan_tests.rs` 977・`dispatcher_choice_tests.rs` 936・`lib_tests.rs` 922。

## 2. この範囲がしていること・想定した負荷

- **何をするか**: `areka-ghost` はゴーストの読み込み（`descript.txt` からの組み立て・ゴースト／シェル／バルーンの一覧・SHIORI との結線・起動から終了までのまとめ役・刻み）。`areka-nar` はネットから落とした `.nar`（zip）を読んでベースウェアの根へ入れる。`areka-user-data` は利用者ごとの書ける置き場を決めて、実行ファイルの隣の記憶をそこへ写す。
- **想定した負荷**: 利用者は 1 人・ゴーストは 1〜数体。ただし `.nar` とゴーストのフォルダは**信用できない外からの入力**（境目の検査は削らない）。`--install`・最初の起動・投げ込みは別のプロセスになりうるので、**areka のプロセスが同時に 2 つ以上動く**ことは起こりうる（多重起動を止める仕組みはツリーに無い）。書庫と台本は大きいことがある（「数 KB」で線を引かない）。

## 3. 指摘の一覧（全 91 件）

組の名は audit の 3 つ: **Must fix（必ず直す）**・**Should fix（直したほうがよい）**・**Nice to have（できれば）**。番号は 3 つの組を通して 1 から。

- 数: **Must fix 5 件・Should fix 47 件・Nice to have 39 件**。
- 「範囲外」の欄: 直しが受け持つ 3 つのクレートの外のファイルに及ぶとき、そのファイルと持ち主の spec。「—」は及ばない。
- 「残す候補」の欄: 直さずに残してよい 3 種類（⑴ 開発者の裁定・⑵ わざと置いた先送りの印・⑶ SSP／ukadoc の正典に関わる振る舞いの変更）に当たるかもしれないもの。決めるのは要件の討議（要件 2.2）。「—」は当たらない。

### 3.1 Must fix（5 件）

| # | 場所 | 何が壊れるか | いちばん小さい直し | 範囲外 | 残す候補 |
|---|---|---|---|---|---|
| 1 | `areka-nar/src/install.rs:229-269`（`prepare_shelf`） | 片付けが `.nar-work` の下の**ほかのプロセスの作業中のフォルダ**まで消す。2 つの areka が同じ根へ同時に入れると、片方が元のゴーストを退避している最中の木を消し、元のゴーストが消えるか、半分消えた木が成功として置かれる | `.nar-work/.lock` を共有なし（`share_mode(0)`・std の `OpenOptionsExt`・依存は増えない）で作成から確定まで持ち、持っている間だけ片付ける。2 本目は待つか `IoPhase::Stage` で失敗 | — | — |
| 2 | `areka-nar/src/lib.rs:289-311`（`peek_install_txt`・`read_archive_bytes`） | 投げ込みの判定（`crates/areka/src/input_events/file_drop.rs:57-61`・UI スレッド）が書庫を**丸ごと**メモリへ読む。6 GB の写真の zip を落とすと UI が長く止まり、メモリが足りなければ落ちる。`open` も読む前に大きさを見ない | peek は末尾の `EOCD`（最大 64 KiB＋22）と中央ディレクトリの範囲だけを読む。`read_archive_bytes` は先に `metadata().len()` を見て、zip64 なしで有り得ない大きさは `TooLarge` で拒む | —（署名は変えない） | — |
| 3 | `areka-nar/src/names.rs:40-43`（`RESERVED_STEMS`） | Windows の予約名のうち `COM¹ COM² COM³ LPT¹ LPT² LPT³`（上付きの数字）が抜けている。`ghost/master/COM¹.txt` を持つ書庫が通り、展開でデバイスを開く（COM1 が在れば中身が通信口へ流れ、インストールは成功と報告される） | 6 つの語を表に足し、`names_tests` に 1 件足す | — | —（`COM0`／`LPT0` を外す決め〔L37-39〕はこれを含まない） |
| 4 | `areka-ghost/src/shiori_inproc_adapter_tests.rs:255-268`・`shiori_inproc_tests.rs:28-48`・`tests/ghost/inproc_e2e_test.rs:1046-1080`・`tests/ghost/spine_e2e_test_s7_second_boot_record_present.rs:26-47` | 4 つのテストが一時フォルダを `std::env::temp_dir()`（`%TEMP%`）に作る＝「一時フォルダはワークツリーの `target\` の下だけ」（絶対の決まり）に反する。途中で落ちると `%TEMP%` に残り、S7 は消せなかった前回の残りの上で起動する（`let _ =` で失敗を捨てる） | 4 つとも `temp_path_kit::TempPath::new(..)` にし、手書きの一意な名前と `remove_dir_all` を消す | `crates/log-capture-kit/tests/temp_path_guard_test.rs`（例外表の 4 行・`ALLOWED_COUNT`・種別ごとの数。持ち主 `audit-test-kits`・C17 には居ない） | ⑴ 例外表は 4 件を「プロセス識別子で一意化済み・要件 12.2 の移行対象外」と記録（完了した spec の決め） |
| 5 | `areka-ghost/tests/ghost/inproc_fixture.rs:278-291・336-344`（`shared_test_ghost`） | 固定の `%TEMP%\areka_shiori4_test_ghost_shared` を**機械の全ワークツリー**が分け合う。ウェーブで 2 つのワークツリーが同時にテストすると、B が A の作った DLL を「新しい」と見て使い、B の I1／I2 は A のコードを試して緑になる。A が組み直す間に B が DLL を掴んでいると、消せずに `hard_link`／`copy` が失敗して panic | 固定の名前のまま、置き場をこのワークツリーの `target\test-roots\` の下へ移す（プロセスをまたぐ再利用は残る） | 同上の例外表（`inproc_fixture.rs` の行・較正で外す 1 件 `CALIBRATION_DROPPED_ENTRY` がこのファイルを名指す＝別の項目へ替える） | ⑴ 例外表は「固定であることが仕様（プロセスをまたぐ共有）」と記録。ワークツリーをまたぐ共有はその理由に含まれない |

### 3.2 Should fix（47 件）

#### areka-nar（本番）

| # | 場所 | 何が壊れるか | いちばん小さい直し | 範囲外 | 残す候補 |
|---|---|---|---|---|---|
| 6 | `names.rs:211-224`（`validate_entry_names`） | ファイル `x` と、`x/y.txt`（暗に `x` をフォルダにする）が同じ書庫に居ても拒まない。展開で `fs::write` が失敗し、`OnInstallFailure` に「大小の衝突」でなく入出力の失敗が届く。テストは明示のフォルダ `a/`＋`a` だけ | 各エントリの先祖の前置き（小文字）をフォルダの鍵として持ち、ファイルの鍵と重なれば `CaseCollision` | — | — |
| 7 | `manifest.rs:426-447`・`plan.rs:113-136` | 2 つの同梱（`balloon.directory,foo` と `balloon0.directory,FOO`）が同じ宛先を指しても検出しない。ゴーストなら 2 本目の `rename` が 2 秒待って失敗し、全体が巻き戻る。シェルで宛先が既に在れば、1 本目の中身が黙って 2 本目に置き換わり、結果には両方が「入った」と載る | `collect_companions` で宛先の小文字を集合に持ち、重なれば拒む（`InvalidDirectoryName`）か警告して飛ばす | — | ⑶ SSP が同じフォルダを指す 2 行をどう扱うかを ukadoc（install.txt の `*.directory`）で確かめてから拒む／飛ばすを決める |
| 8 | `install.rs:341-372`（`copy_existing`） | 既に在る宛先の中のジャンクション／シンボリックリンクを「ファイル」として `fs::copy` し、フォルダへのリンクで失敗する＝利用者がリンクを置いたゴーストは上書き・更新・サプリメントが毎回 Stage で失敗する。ファイルへのリンクは黙って実体の写しになる。`areka-user-data` の写し（`copy.rs:107-109`）はリンクを拒むので、2 つの写しの決まりがずれている | `file_type().is_symlink()` を先に見る。拒む（入出力の失敗とパスを返す）か、たどって写すかは決めが要る | — | ⑴ インストール済みのゴーストの中のリンクをどう扱うかは開発者の決めが要る |

#### areka-nar（テスト）

| # | 場所 | 何が壊れるか | いちばん小さい直し | 範囲外 | 残す候補 |
|---|---|---|---|---|---|
| 9 | `plan_tests.rs:879-899・955-963` | 「計画は根へ何も書かない」の写しが**長さ**しか比べない。`install.txt` を同じ長さの別の中身へ書き換えても緑。較正も 1 バイト→2 バイトの変化しか試さない | 長さでなくバイト列を比べる（下の #12 の共有の `tree` を使う）。較正は `b"1"`→`b"2"` | — | — |
| 10 | `container_tests.rs:515-526` | 「上限ちょうどは通る」と言うテストが上限−1 しか試さない。本番の `>` を `>=` に変えても緑 | `.declared_size(MAX_TOTAL_DECLARED_SIZE)` にする | — | — |
| 11 | `install.rs:253-256` の枝（テストは無い） | 棚（`.nar-work`）に残った**ファイル**を消す枝を試すテストが無い。壊れると残りが毎回「消せなかった」に載り続ける | 棚に `junk.txt` を置いて `WorkArea::create` を呼び、消えたことと残りが空なことを見るテストを 1 本 | — | — |
| 12 | `install_tests.rs:70-93` と `lib_tests.rs:60-83`（`tree`/`walk`）・`install_commit_tests.rs:41-49` と `lib_tests.rs:378-389`（`hold`）・`install_tests.rs:144-148` と `plan_tests.rs:73-77`（`root_with_ghost`）・`install_tests.rs:48-60` と `plan_tests.rs:25-34`（読む→検証→解く） | 同じ形のテストの道具が 2 か所ずつ。片方だけ変わると「変わっていない木」の意味がずれる（#9 は既にずれた例） | `lib_tests` の `tree`／`hold` を `pub(super)` にして写しを消す。`root_with_ghost` と読みの道具も 1 つに | — | — |
| 13 | `container_tests.rs:530-588`・`manifest_tests.rs:570-616`・`names_tests.rs:686-741` | 「この部品は入出力を綴らない」を調べる走査と較正が 3 つ写され、既にずれている（`Path` と `path::`） | `lib_tests.rs` に `assert_source_has_no_io(source, needles, harmless)` を 1 つ置き、3 つのファイルはそれを呼ぶ（約 −90 行） | — | — |
| 14 | `plan_tests.rs`（977 行） | 次にテストを 1 本足すと 1,000 行の番人が赤 | #9・#12・#60・#61 で約 30〜40 行減る。足りなければ「書き込みをしない」の塊（L876-971）を兄弟のファイルへ | — | — |

#### areka-user-data

| # | 場所 | 何が壊れるか | いちばん小さい直し | 範囲外 | 残す候補 |
|---|---|---|---|---|---|
| 15 | `copy.rs:55-77・100-113`（`copy_tree_atomic`・`build`） | 書ける根（＝作業の棚）が写す元の中にあると、写しが自分自身の中へ入り込み続ける。例 `AREKA_USER_ROOT=<exe>\profile\areka\x`。パスの長さの上限で止まるか、起動のたびにスタックが溢れて落ちる | 先頭で `to`／`work_root` が `from` の下（大小を無視して比べる）なら `InvalidInput` で拒む | — | — |
| 16 | `copy.rs:62-72` | 作業フォルダ `<pid>-<連番>` が、落ちたプロセスの後に**消されない**（片付けが無い）。Windows は pid を使い回すので、後の areka の最初の写しが `AlreadyExists` で失敗する（移しなら記憶の移しが失敗する） | `AlreadyExists` なら連番を進めて取り直す。棚の古い（例 1 日より前の）項目は写しの初めに消す | — | — |
| 17 | `establish.rs:255-291`（`migrate`・`migrate_source`） | 1 回目の移しが失敗すると（ウイルス対策が掴んでいた等）、空の `<根>\profile\areka` に新しい記憶が書かれ、次から `to` が在るので**隣の記憶は二度と移らない** | 失敗した回はその起動だけ隣の profile を使い、`to` を作らない（次の起動でやり直す） | `crates/areka/src/boot_config.rs`・`crates/areka-mcp-bridge/src/config.rs`（持ち主 `audit-areka-app`・`audit-mcp-update-crates`・C17 には居ない） | ⑴ 完了 `areka-P0-user-data-root` 要件 5.5「写せなくても置き場は書ける根の profile\areka のまま」 |
| 18 | `lib.rs:207-209`（`app_profile_dir`） | `AREKA_PROFILE_DIR` の空・相対をそのまま使う（`AREKA_USER_ROOT` は両方拒む＝同じ決まりが 2 か所で違う）。相対だと起動の仕方で別のフォルダを読み書きし、記憶が消えたように見える | `absolute()` を通し、駄目なら `error!` を出して変数を無視する | `crates/areka-mcp-bridge/src/config.rs:105・128`・`crates/areka/src/boot_config.rs:520` | ⑴ 同 spec 要件 5.1「今のまま」 |
| 19 | `establish.rs:56-61`（winget の控えの探し） | 一覧の 1 件ごとの読み失敗を `flatten()` が記録なしで捨てる。控えが「無い」と判断されると、winget の入れ方なのに記憶が実行ファイルの隣に書かれ、更新や削除で消える | 各 `Err` を既にある `user_data_marker_unreadable` の `warn!` で記録する | — | — |
| 20 | `copy.rs:82-93`（先を越された枝） | 2 つのプロセスが同時に写したとき、負けた側を `Raced` にする枝を試すテストが無い（今のテストは L56 の早い戻りで止まる） | L82-94 を小さな関数に出し、在る `to` に対して `Raced` と作業フォルダの消去を見るテストを 1 本 | — | — |
| 21 | `establish_tests.rs:94-133`（対象は `establish.rs:241-251・276-282`） | 失敗の道のテストが `error!` の記録を見ない。`app_memory_migrate_failed`・`user_root_unavailable`（`create_failed`）の `error!` を消しても緑。`user_data_copy_work_left` はどこでも見ていない | `log_capture_kit::capture` で包み、ERROR が 1 件出ることを見る（`establish_locate_tests.rs:195-200` と同じ形） | — | — |

#### areka-ghost（本番）

| # | 場所 | 何が壊れるか | いちばん小さい直し | 範囲外 | 残す候補 |
|---|---|---|---|---|---|
| 22 | `config.rs:90-101`（`resolve_shell_name`）と `catalog.rs:441-447`（`lowercased`） | `descript.txt` の読み方が 2 つで違う。シェルの `Name,Foo` はメニューに「Foo」と出るが、OnBoot のシェル名は鍵が見つからずフォルダ名 `master` になる。シェルの `name,`（空）は OnBoot に空の名を送る（一覧は「名無し」扱い）。ゴーストの `Name,Upper` も一覧は「Upper」だが、`areka-parsers` の読みでは名前が無く、ログやMCP に根のパスが出る | `resolve_shell_name` も空を落とし大小を無視して引く（`catalog::lowercased` を `pub(crate)` にして使う）。`Name,X`・`name,` のテストを 1 本ずつ | 揃えるなら `crates/areka-parsers/src/package/resolve_shell.rs`（持ち主 `audit-parsers-sakura`・**C17 で並走中**＝申し合わせが要る） | ⑶ descript の鍵の大小と、`name,` のときに SSP が OnBoot の Reference0 に何を送るかを ukadoc で確かめる（ukadoc の検索では鍵の大小の定めを見つけられなかった＝要調査） |
| 23 | `catalog.rs:350-365` と `420-436` | 「descript を読んで小文字の表にする」が 2 つ。無いことの見分け方が違い（`is_file` の先回りか `NotFound` か）、`ghost/master/descript.txt` がフォルダだと片方は黙り片方は `warn!`。先回りの `is_file` は一覧のたびに 1 回ずつ余計に見る | `read_descript` を `NotFound` の形にし、`master_descript_keys` は `read_descript(&master_descript(g))` で置き換える | — | — |
| 24 | `catalog.rs:461-464`（readme の最上位だけの検査）・`catalog_tests.rs:99-114` | 外から来た descript の `readme,..\x.txt`／`readme,sub\x.txt`／`readme,C:\…` を最上位のファイルだけに絞る検査を試すテストが無い。検査を簡単にすると、`Identity.readme` がゴーストの外を指しても赤にならない | `readme,..\outside.txt` と `readme,sub\x.txt` で `readme == None` を見る 1 件を足す | — | — |
| 25 | `config.rs:45-57`（`listed_ghost_name`）と `crates/areka/src/mcp/resolve.rs:36-50・80-90`（`listed_value`） | 「一覧に出すゴーストの名」の作り方がクレートをまたいで 2 つ（テストに 3 つ目）。片方が変わると `get_log ghost_name=…` の絞り込みが黙って当たらなくなる | `areka-ghost` に `pub fn listed_name(name, ghost_root) -> String` を 1 つ置き、両方が使う | `crates/areka/src/mcp/resolve.rs`（持ち主 `audit-areka-app`・C17 には居ない） | — |
| 26 | `shiori_inproc.rs:37-45` | モジュール全体の `#![allow(dead_code)]` の理由（「task 2.3 が使うまで」）は既に切れている（`runtime.rs:696-697` が使う）。660 行のどこかが使われなくなっても警告が出ない | 段落と allow を消す（`non_snake_case` の allow は残す） | — | —（⑵ の印だが、書いた限界が既に切れている） |
| 27 | `shiori_inproc.rs:384-402`（`map_get_outcome`）と `crates/shiori-host32-host/src/client.rs:413-437`（`map_get_result`／`map_get_reply`） | 応答の状態の分け方（400／500／ErrorLevel・204）の写し。doc は「同じ決まり」と言うが縛るものが無く、片方だけ変わると 32 bit と同じプロセス内の道で答えの扱いが分かれる | `map_get_reply` を公開して使う | `crates/shiori-host32-host/src/{client.rs,lib.rs}`（持ち主 `audit-shiori-host32`・C17 には居ない） | ⑴ 完了でない `shiori4-api` の brief（L103・L131）がこのモジュールの `InProcHost` を消し、注釈を掃除すると決めている＝作り直しで消える直し |
| 28 | `shiori_inproc.rs:90-108`（`load_library_quiet`） | `LoadLibraryW` は DLL 自身のフォルダを依存の探し先に入れない。ゴーストが SHIORI と同じフォルダに `lua54.dll` を持つと、読めないか、実行ファイルの隣や PATH の別の DLL を掴む（DLL の植え込み）。今は本番が同じプロセス内の道を選ばない（テストだけ）ので潜在 | `LoadLibraryExW(.., LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR \| LOAD_LIBRARY_SEARCH_DEFAULT_DIRS)`（同じ `windows` の機能の中）・パスは絶対に（#29） | — | ⑴ 同上（`shiori4-api` が DLL の読み込みの芯を `shiori4_host` へ移す） |
| 29 | `shiori_inproc.rs:586-597`（`inproc_connect`）と `shiori_wiring.rs:275-281`（`helper_load_dir`） | `load` に渡すフォルダの形を、helper の道は整える（絶対・`\`・末尾 1 つ）が、同じプロセス内の道は整えない＝同じ決まりが片方だけ。華和梨のようにフォルダ＋ファイル名をつなぐ SHIORI は `…\ghost/masterdic.txt` を開いて失敗する（「．さくら」で直した不具合と同じ） | `helper_load_dir` を `pub(crate)` にして両方に使う | — | ⑴ `shiori4-api` の brief L128「この spec に残るのは v4 の道の形」 |
| 30 | `sylphya_wiring.rs:164-172`（`make_username_resource_sink`） | sylphya の役者が死んで `barrier()` が失敗しても `warn!` だけ。プロパティの置き場が死んだことが ERROR の検索に出ない（閉包は `()` を返す形なので `Err` は返せない） | `warn!` を `error!` に（続行は変えない） | — | ⑴ doc（L153-154）が「design Risks: warn＋続行」を決めとして引く |

#### areka-ghost（`src` のテスト）

| # | 場所 | 何が壊れるか | いちばん小さい直し | 範囲外 | 残す候補 |
|---|---|---|---|---|---|
| 31 | `dispatcher_choice_tests.rs:127-133`（対象 `dispatcher.rs:394`） | 刻みのミリ秒→秒の換算を見るのは経過 0.0 の 1 件だけ。`/ 100.0` に壊しても緑。もう 1 本（`dispatcher_slot_tests.rs:361-377`）は直後の `try_recv` の競りで、重い机では素通りする | 同期の檻で `Tick(7_000)`→`Tick(7_500)` を送り、`Tick(0.0)`→`Tick(0.5)` を見る（約 6 行） | — | — |
| 32 | `catalog_roots.rs:209-214`（`head_skipped`）・`catalog_roots_temporary_tests.rs:78-97` | 一時の根を飛ばして先頭を選ぶ `find(\|r\| !r.is_temporary())` を `.first()` に壊しても全テストが緑（一時の根がいつも在るため）。一時の親フォルダが消えた時に、入れ先でない根を「飛ばした」と記録する | `[一時(t), 無い(gone), a]` で `head_skipped == Some(gone)` を見る 1 件 | — | — |
| 33 | `shiori_inproc_host_tests.rs:51-58`（対象 `shiori_inproc.rs:289-293`） | doc は「warn で見える化」と言うが記録を見ない。`warn!` を消すと `Raise` が黙って捨てられても緑 | `crate::test_log_capture::capture` で包み WARN を見る | — | — |
| 34 | `shiori_inproc_adapter_tests.rs:140-176` | テストの道具が `inproc_connect` を手で写している（`CreateInstance` に渡すフォルダ・持つ順）。本番の `inproc_connect` が変わっても、テストは自分の写しを試して緑。DLL の探しも 3 か所（`runtime_tests.rs:558-570`・`shiori_inproc_tests.rs:94-107`） | `TempPath` の下に DLL を写した fixture を作り、`resolve`→`inproc_connect(..)()` で組む（`runtime_tests.rs:577-603` と同じ形）。DLL の探しは 1 つの関数に | — | — |
| 35 | `catalog_test_support.rs:10-33`・`catalog_roots_ghost_balloon_tests.rs:18-100`・`catalog_roots_tests.rs:93-120`・`catalog_roots_temporary_tests.rs:13-18`・`catalog_tests.rs:228-234` | ゴースト／バルーンの fixture を書く道具が 5 組、二つの根の fixture が 2 つ、WARN の絞り（`TARGET` 定数つき）が 2 つ。ゴーストの形の決まりが変わると 5 か所を揃える要があり、1 つ漏れると「0 件を数える」assert が素通りする | `catalog::test_support` を `pub(crate)` にして `put`・`put_ghost_in`・`put_balloon`・`two_roots`・`catalog_warns` を 1 つずつ | — | — |
| 36 | `dispatcher_test_support.rs:12-63`・`runtime_tests.rs:12-18・120-125・221-232・514-540`・`ticker_tests.rs:8-19`・`relay.rs:51-61`・`runtime_origin_tests.rs:54-77・116-127`・`runtime_within_tests.rs:81-103・157-164`・`runtime_shell_tests.rs:48-59・73-82`・`runtime_names_tests.rs:67-76`・`shiori_wiring_empty_tests.rs:100-105・199-207`・`translate_wiring_tests.rs:194-209`・`translate_wiring_e2e_tests.rs:101-116・159-168` | `run_bounded`（4 つ）・`test_system_vars`（2）・`RecordingSink`（2）・`NoopSink`（4）・ゴーストの fixture（3＋似た 2）・有界の終了（5）・OnBoot を待つ輪（2）が写されている。「取り込めない」とある注釈は `pub(super)` のせいで、理由は無い | クレートの `#[cfg(test)] mod test_support;`（`pub(crate)`）へ 1 つずつ集め、写しを消す | — | — |
| 37 | `runtime_tests.rs`（993 行）`:143-154・246-257・288-299・407-418・467-478` | 1,000 行まで 7 行。`GhostBootOptions` の塊が 5 回貼られている。次の 1 本で番人が赤 | `fake_options(root)` を 1 つ（約 −50 行）。#36・#38・#39 と合わせて約 −200 行 | — | — |
| 38 | `runtime_tests.rs:84-115`（`FakeShioriBackend`） | 本番の `shiori_wiring::EmptyShiori`（`shiori_wiring.rs:288-317`）と全く同じ振る舞いの偽物 | 消して `ShioriWiring::Custom(Box::new(crate::shiori_wiring::empty_connect()))` | — | — |
| 39 | `runtime_tests.rs:127-173`（`boot_happy_path_…`） | `shutdown` を呼ばずに捨てるので、回すたびに役者のスレッド約 7 本が残り、生きた役者の下で `TempPath` が消える。見ていることは `into_parts_exposes_live_senders_…`（L282-326）が全部見て、きちんと畳む | このテストを消す（−47 行） | — | ⑴ doc（L132-136）が「意図的に drop・design.md の保持物の節」と言う |
| 40 | `runtime_tests.rs:684-697`（`SharedGateIo`）・`prop_sink.rs:156-164` | `Arc<FakePersistIo>` の包み（`impl PersistIo for Shared…`）がツリーに 22 個。テストが要るたびに写しが増える | `areka-sylphya` に `impl<T: PersistIo + ?Sized> PersistIo for Arc<T>` を 1 つ置き、包みを消す | `crates/areka-sylphya/src/persist/io.rs`（必須・持ち主 `audit-sylphya-actor`・C17 には居ない）。消すだけなら `crates/areka/src/` の 18 のテストのファイルと `areka-sylphya/src/actor_actor_integration_tests.rs` も（任意） | — |
| 41 | `runtime_origin_tests.rs:152-180`（OnGhostChanged の Reference7） | テストが Reference7 をシェルの**フォルダ名**（`master`）で縛る。ukadoc は「Reference7: 切り替わったゴーストのシェル名。[SSPのみ]」（`ukadoc:list/shiori-event/boot-close-change/OnGhostChanged`）。一方 areka の OnBoot の Reference0 は descript の `name`（`config.rs`）＝同じ「シェル名」の読みが 2 つのイベントで違う | 決めてから直す（今は直しの形を決めない） | 変えるなら `crates/areka-kanade/src/msg.rs:546`・`crates/areka-ghost/src/runtime.rs:582`（`audit-kanade`・C17 には居ない） | ⑴ 完了 `areka-P0-ghost-shell-balloon-switch` 要件 4.1「Ref7＝切替先で使うシェルのフォルダ名（例 master）」／⑶ SSP が実際に送る値は `ssp_shiori_log.txt` で測れる |

#### areka-ghost（`tests/` の結合テスト）

| # | 場所 | 何が壊れるか | いちばん小さい直し | 範囲外 | 残す候補 |
|---|---|---|---|---|---|
| 42 | `tests/ghost/inproc_e2e_test.rs:57-106`・`snapshot_capture_test.rs:127-176`・`tests/real_pasta_online.rs:81-130`（`BoxedBackend`・**要件 7**） | 3 つはバイト単位で同じ（doc だけ違う）。`ShioriBackend` に既定の本体つきの口が増えると（`*_with_headers` のように）、写し漏れが黙って頭や応答のレベルを落とす | 2 つのテストの実行ファイル（`tests/ghost.rs` と `tests/real_pasta_online.rs`）が既に `#[path = "ghost/recorder.rs"]` で分け合っている `recorder.rs` に `pub struct BoxedBackend` を 1 つ置き、3 つの写しと各ファイルの `use shiori_host32_host::{…}` を消す（`recorder.rs` は 435→約 487 行・新しいファイルも `mod` 行も要らない） | —（`areka-kanade` に `Box<T>` への包括の実装を置く案は本番に及ぶので取らない） | — |
| 43 | `real_pasta_test.rs:67-100`・`snapshot_capture_test.rs:181-213`・`real_pasta_online.rs:133-162`（`resolve_helper_exe`） | helper の探し方の写しが 3 つ（中身は同じ）。探し方を変えると 3 か所が食い違う | 両方の実行ファイルが読む `common.rs` に 1 つ置く | —（4 つ目 `crates/areka-kanade/tests/kanade/real_helper_test.rs:48` は残す） | — |
| 44 | `real_pasta_test.rs:230-244`・`snapshot_capture_test.rs:291-306`・`real_pasta_online.rs:318-328`・`inproc_e2e_test.rs:664` ほか（`run_bounded` は `ghost` の実行ファイルの中に 6 つ）・待ちの輪 `real_pasta_test.rs:206-222` と `snapshot_capture_test.rs:392-406` | 有界の待ちと終了の写し。既にずれた（snapshot の待ちは `assert!(observed)` を失った＝#47） | `common.rs` に `run_bounded`／`run_bounded_shutdown` を置き、全部がそれを使う | — | — |
| 45 | `inproc_fixture.rs:295-309`・`inproc_e2e_test.rs:699・847`（I1・I2） | I1 と I2 は同じ固定の根を並んで使う。I1 の初回の記録（`profile/areka/sylphya.toml`）が I2 の消去の後・起動の前に書かれると、I2 は 2 回目扱いで OnFirstBoot を飛ばし、60 秒待って赤（逆もある）。コードを読んでの推定で、走らせて再現はしていない | I1・I2 の頭で 1 つの `static Mutex<()>` を取り、終了まで持つ | — | — |
| 46 | `real_pasta_test.rs:118-146・176-178` | pasta の DLL だけを置いたゴーストを作る。pasta は `dic/`・`scripts/`・`pasta.toml` を読むので（`real_pasta_online.rs:15-16` の注）、答えられない。環境変数で動かすと起動か 60 秒の待ちで落ちるか、pasta でない cue で通る（snapshot は同じ理由でフォルダごと写す＝既にずれている） | `SampleRoot::acquire("emo2")` に DLL を写す形（`real_pasta_online.rs:216-222`）にし、`write_real_pasta_ghost_fixture` と `unique_temp_dir` を消す（約 −45 行） | — | — |
| 47 | `snapshot_capture_test.rs:232-245・261-263・392-406` | 凍結した写し（`AREKA_SNAPSHOT_OUT`）を作る道具が、写しの失敗を捨て（`Err(_) => return`・`let _ = copy`）、何も見えなくても assert しない。辞書が写らず OnBoot が 204 でも、204 の写しを書いて緑。写す元の `profile/areka/sylphya.toml` も一緒に写るので 2 回目扱いで OnFirstBoot を取り逃す | 捨てている失敗を `expect` に、`profile` を写さない（または #46 の `SampleRoot`）、待ちの後に `assert!(observed)` | — | — |
| 48 | `inproc_fixture.rs:67-85・188-194・203-206・469-489`（`TempGhost`・`assemble_test_ghost`） | ツリー全体で呼ぶのは自分のテストだけ（そのテストは `TempPath` の `Drop` を確かめ直すだけ）。これが消えると `ASSEMBLE_FS_LOCK` も要らない（`OnceLock` が直列にする） | 4 つを消す（約 −55 行）。名指す doc も直す | — | — |
| 49 | `spine_e2e_test_s5_close_deadline.rs:159・253-327` | S5 は SHIORI の呼び出しの列しか見ず、通常の Quit と同じ列になる。終了の台本がすぐ `TalkDone{Quit}` で終わる退行でも緑で、締切の道（`KanadeStopCause::DeadlineExceeded`）は試されない | S2 と同じく `boot_with_kanade_stop` で起こし、`Stopped{cause: DeadlineExceeded}` を見る | — | — |
| 50 | `spine_e2e_test_s1〜s7`・`sylphya_integration_test.rs`（`write_ghost_fixture` 7 つ・`join_bounded` 3・`run_bounded` 5・`expected_from_shiori_call` 3・`unique_temp_dir` 7・`BOUND`／`E2E_BOUND` 4） | 写しの理由（「兄弟のモジュールから私的な物は見えない」）は誤り＝s1〜s7 は `spine_e2e_test.rs` の**子**で、既に `super::spin_pumping_ticks` を使う。fixture の決まりが変わると 7 か所 | 親の `spine_e2e_test.rs` に 1 つずつ置き、子は `super::`、`sylphya_integration_test` は `pub(crate)` で使う。`write_ghost_fixture` はゴーストの名を引数に | — | — |
| 51 | `s1:166-192`・`s4:218-243`・`s5:276-301`（期待の起動の列）・`s3:202・207`（数の 6）・`s1:96-104` ほか 5 か所（起動の台本） | 起動の列が変わるたびに約 10 か所を手で直してきた（注釈は「4→5→6」のまま・S3 L189 は「ちょうど 5 件」とあるのに 6 を見る） | 親に `boot_script()` と `expected_boot_prefix(&config)` を置き、S3 はその長さで待つ | — | — |
| 52 | `s3:147-172`・`s7:155-180`・`sylphya_integration_test.rs:216-238`（刻みを押す輪）・`s1:223-229` ほか 3（待つだけの輪） | 刻みの押し方の直し（2026-07-30・親の L27-47）を通らない手書きの輪が 7 つ。押し方を変えると漏れる | 刻みは `super::spin_pumping_ticks`、待つだけは親に `wait_until(what, cond)` を 1 つ | — | — |

### 3.3 Nice to have（39 件）

| # | 場所 | 中身 | 直し | 範囲外 | 残す候補 |
|---|---|---|---|---|---|
| 53 | `areka-ghost/src/ticker.rs:203-234・305-308` | `target = "dispatcher"` は欄で、tracing の target ではない＝`RUST_LOG=dispatcher=info` で出ない | `target:`（コロン）か欄名を `lane` に | — | — |
| 54 | `ticker.rs:16-23・38-40・160-164` | 「`DispatcherMsg` はまだ無い」という古い注釈（既にある・`runtime.rs:775`） | 段落を消す（汎用の `D` は残す） | — | — |
| 55 | `catalog.rs:273-280`（`companion_balloon`） | 同じ install.txt を 2 回復号して解く | 1 回で解いた表から引く | — | — |
| 56 | `catalog.rs:101-109`（`BasewareRoot::ghost_dir`／`balloon_dir`） | 公開だが本番に呼び手が無い（テストだけ） | 消すならテストを `ghost_store().join(..)` に | `crates/areka/src/boot_resolve{,_option,_balloon,_mark_text}_tests.rs` | — |
| 57 | `catalog.rs:123-137・468-473`（`Identity` の `craftmanw`・`id`・`readme`・`has_thumbnail`） | 本番は `folder` と `name` しか読まないのに、一覧のたびに `is_file` を 2 回ずつ打つ | 決めが要る（消さない案が推し） | — | ⑴ 完了 `areka-P0-baseware-root-layout` 要件 2.4「素性 7 項目」 |
| 58 | `sink.rs:335-350` | `LogSink` は欄の無い構造体なので `assert_eq!(original, cloned)` は必ず真＝落ちないテスト | テストを消す | — | — |
| 59 | `sink.rs:72-89`（`LogSink::log`） | 呼び手 1 つの素通しの関数 | 本体を `emit` へ移して消す | — | — |
| 60 | `areka-nar/src/plan_tests.rs:38-44`（`plan_and_skipped_of`） | 3 つ目の値を落とすだけの包み（さらに `plan_of` が包む） | 消して `let (plan, skipped, _) = accepted(..)` | — | — |
| 61 | `plan_source_path_tests.rs:13-22` と `plan_roots_tests.rs:188-197` | 同じ install.txt の頭の写し | 親の `plan_tests.rs` へ 1 つ | — | — |
| 62 | `install.rs:433-436・466`・`install_commit_tests.rs:732-757` | doc とテストの文言が「巻き戻せないとき `committed` は確定したまま残る」と言うが、`unwind` は先に宛先を消すので残っていないことがある | doc を「宛先は無いことがある・`survivors` を見よ」に直し、テストに `assert!(!first_stuck.exists())` | — | — |
| 63 | `install_commit_tests.rs:362・459・499・552`・`install_nested_tests.rs:205・261`（`install.rs:650-652` の 2 秒） | 掴みっぱなしのテストが 1 本ごとに約 2 秒寝る（合計 12〜14 秒） | `ON_RENAME_REFUSED` の隣に `#[cfg(test)]` の待ちの差し替えを置き 0 に | — | ⑵ `install_commit_tests.rs:394`「待つ分だけ約 2 秒遅くなる」を受け入れた注 |
| 64 | `plan.rs:144-151・337-343`・`install.rs:347-349・382-390` と `names.rs:216` | 大小の比べ方が層で違う（名前の検証は Unicode、計画と展開は ASCII だけ）。全角の `ＢＡＬ` と `ｂａｌ/` が同梱として見つからず本体の中に入る | `same_name(a, b)`（`to_lowercase` で比べる）を 1 つにして両方で使う | — | — |
| 65 | `lib_tests.rs:86-90`・`lib_peek_tests.rs:45-49`・`lib_entry_tests.rs:39`・`lib_skip_tests.rs:103`・`lib_vocabulary_tests.rs:208・290・333` | `NarBuilder::write_to`（`sample-ghost-kit`）が既にあるのに手書き | `builder.write_to(&path)` に | — | — |
| 66 | `error_tests.rs:84-93・108-125・127-137` | `kind()` と `ALL_KINDS` は同じマクロから出るので落ちようのないテスト | 3 本を消す（さらに L12-73・L95-106 も重複） | — | — |
| 67 | `lib_tests.rs:363-373・629-630・671-679` | 「較正」が定数の文字列どうしを比べるだけ | その assert を消す | — | — |
| 68 | `lib_tests.rs:910-922` | 自前の 1,000 行の検査が全体の番人と食い違う（`< 1000` と `> 1000`）・3 ファイルだけ | テストを消す | — | ⑴ 完了 `areka-P0-nar-install` の要件 10.8 が置いた |
| 69 | `manifest_companion_tests.rs:35-50・172-189・193-212` | `manifest_companion_reading_tests.rs` と同じことを 3 組見ている | 片方を消す | — | — |
| 70 | `lib_vocabulary_tests.rs:34-36・210-219` | `Option<Option<&str>>` で使うのは 2 つの値だけ | `bool` に | — | — |
| 71 | `manifest.rs:135-136`（`charset_declared`）・`manifest_tests.rs` の 8 か所 | 本番に読み手が無い欄を 8 つの assert が縛る | 公開の語彙として残すか、欄ごと消すか | 消すなら `crates/areka/src/install/judge_tests.rs:28` | ⑵ `manifest_tests.rs:305-311`「既知の盲点」／⑶ `charset` の行の位置を ukadoc の install.txt で確かめる |
| 72 | `areka-user-data/src/establish.rs:116-120`（`establish_readonly_from` の `_profile_env`） | 使わない引数（関数を差し替える形の `establish_readonly` には要るが `_from` には要らない） | 引数を消す | `crates/areka/src/readonly_run_tests.rs:168` | — |
| 73 | `establish_locate_tests.rs:38-60・91-96`・`establish_readonly_tests.rs:19-61・78-83`・`establish_tests.rs:13-35` | `tree`・`profile` ほかのテストの道具の写し | `#[cfg(test)] mod test_support` に 1 つずつ | — | — |
| 74 | `copy.rs:110-112`（`build`） | 再帰に深さの上限が無い。1 万段の入れ子のゴーストを写すとスタックが溢れうる | 明示の `Vec` の積み上げで歩く | — | — |
| 75 | `areka-ghost/src/dispatcher_choice_tests.rs`（936 行） | `StartTalk { .. }` の手書きが 18 か所・`Close`＋join の後始末が 10 か所 | `StartTalk::new(..)`（`areka-talk`）と `close_and_join` に（約 −150 行） | — | — |
| 76 | `dispatcher_slot_tests.rs:360・373・385・599-603・624-628` | `recv_text(want)` が合ったものしか返さないので直後の `assert_eq!` は必ず真 | 5 つの assert を消す | — | — |
| 77 | `catalog_tests.rs:116-135` と `catalog_all_shells_tests.rs:11-54`・SJIS の手書き `catalog_tests.rs:73-77` と `catalog_test_support.rs:36-43` | 同じ「隠しシェルを除く」を 2 本（3 本目もある）・SJIS のバイトの写し | 1 本を消す・SJIS は道具を使う | — | — |
| 78 | `catalog_roots.rs:292-295`（`root_key` の `C:` の枝） | `same_root` の結果を変えない枝（等しさは枝の有無で同じ）・テストも無い | 4 行と注を消す | — | — |
| 79 | `dispatcher_choice_timeout_tests.rs:20-67` | `areka-sakura` の警告 `choice_timeout_unreadable` を試すのがここだけ＝`cargo test -p areka-sakura` で捕まらない | 警告のテストを `areka-sakura` の compile のテストへ移す | `crates/areka-sakura/src/` の compile のテスト（持ち主 `audit-parsers-sakura`・**C17 で並走中**） | ⑴ 完了 `areka-P0-choice-timeout-directive` のタスク 3.2 がここに置いた |
| 80 | `runtime_tests.rs:187-188・878-881` | `TempPath` が作ったフォルダを消す・既に在るフォルダを作る、の 2 つの空振りの行と誤った注 | 消す | — | — |
| 81 | `ticker_tests.rs:489-496・339-342` | `real_clock` を 2 回読んで減らないことだけ＝定数を返しても緑 | 消す | — | — |
| 82 | `ticker_tests.rs:279-331` | 名前は「切れたら送らない（sticky）」だが、それを見ていない | 名前を見ていることに合わせる | — | — |
| 83 | `ticker_tests.rs:419-424` | 200 ms 待って「2 回目が来ない」＝遅いと偶然通る・毎回 200 ms。同じ決まりは L115-141 が時間なしで見る | assert を消す | — | — |
| 84 | `translate_wiring_tests.rs:232-234` | 時間の上限なしの `shutdown`（ほかは 10 秒の上限）＝止まるとテストの実行ファイルごと止まる | #36 の有界の終了を使う | — | — |
| 85 | `tests/ghost/inproc_e2e_test.rs:739-761` と `894-916` | I1 と I2 の駆動の輪が 23 行の写し | 1 つの関数に（約 −20 行） | — | — |
| 86 | `inproc_e2e_test.rs:684-695` | I1 の「ずれ」の assert は金の列と I2 が既に縛る・`contains("200")` は値の中の "200" でも通る | 消す | — | — |
| 87 | `inproc_fixture.rs:356-369` | `locate` の 3 つの検査のうち 2 つは落ちようがない | (c) だけ残す | — | — |
| 88 | `inproc_e2e_test.rs:125`・`snapshot_capture_test.rs:224-229`・`common.rs:12`・`inproc_fixture.rs:18` | 古い注（「117 件」は 118 件・doc が別の関数に付いている）・理由の切れた `#![allow(dead_code)]` 2 つ | 直す・消す（`recorder.rs:16` の allow は今も要る） | — | — |
| 89 | `spine_e2e_test_s1_boot_success.rs:67-77` ほか全写し・`sylphya_integration_test.rs:180-208` | `run_bounded` は閉包の中の panic を「60 秒で終わらない（止まった）」と報告する＝原因を取り違える | 共有の 1 つで `Disconnected` を「panic した」と分ける | — | — |
| 90 | `spine_e2e_test_global_log_probe.rs:20-41`・`s7:211-218` | `CapturedEvent` を写すだけの `CapturedLine`。プロセス全体の記録の錠を持ったまま毎回文字列を複製する | `CapturedLine` を消して直接見る（`install()` は番人の表どおり残す） | — | — |
| 91 | `s1:4-7`・`spine_e2e_test.rs:8`・`tests/ghost.rs:7`・`s3:189` | doc が見ていないこと（TalkDone の転送）を言う・「S1〜S6」（S7 が在る）・「5 件」（6 を見る） | 文言を直す | — | — |

## 4. 指摘にしなかったもの（確かめた結果）

- 書庫の読み（`areka-nar`）の境目の検査は、`..`・絶対パス・ドライブ・UNC・ADS `:`・制御文字・末尾の点と空白・リンクのエントリ・CRC と大きさの照合・伸長の上限（宣言の大きさ）・総和 1 GiB・オフセットの桁あふれ・zip64／分割／暗号の拒否・ローカルの頭の名前を使わないこと、を確かめて穴は無かった。
- `catalog.rs` の「読めなければ `warn!`＋除く」（要件 2.7）・`translate_wiring.rs` の錠の毒の `error!`・`relay.rs` の使わない内側の通り道は、書いた決めどおりで指摘にしない。
- テストの一時フォルダは、上の #4・#5 のほかは全部 `TempPath`／`sample_ghost_kit::WorkDir`（どちらも `target\` の下）。`C:\…` の綴りはパスの値として渡すだけで、ディスクには触らない。
- 「実 sleep を許す」と書いた 3 か所（`real_pasta_online.rs:74`・`real_pasta_test.rs:47-51`・`snapshot_capture_test.rs:117-119`）は環境変数で動くテストだけで、書いた限界の内側。

### わざと置いた先送りの印（⑵ の候補・今も限界の内側）

- `catalog_roots.rs:260` の `ponytail:`（`fold_key` の大小の表が NTFS と食い違えば OS の比べ方へ）——食い違いの報告は無い。
- `copy.rs:84-85` の `ponytail:`（「在る」は `rename` の後に見る）——守られている（その枝のテストの抜けは #20）。
- `plan.rs:68` の `#[allow(dead_code)]`（`skip_top_level_install_txt`・読むのは兄弟のテストだけ）——守られている。
- `prop_sink.rs:19`（`areka.prop.inc` は名前だけの予約）・`prop_sink.rs:363` のテストの中の `#[allow(dead_code)]`・`sylphya_wiring.rs:101・117`（運行の sink は未配線）——守られている。
- `install.rs:369`・`copy.rs:119` の `#[allow(clippy::permissions_set_readonly_false)]`——理由つきで狭い。
- `shiori_inproc.rs:45` の `#![allow(dead_code)]` は限界が切れている＝#26 で指摘にした。

## 5. 要件と今のコードの対応（Requirement-to-Asset Map）

| 要件 | 今あるもの | ギャップ |
|---|---|---|
| 1（洗い出し） | 本書 §1・§3 | 済み（全 102 ファイル・91 件） |
| 2（討議で確定） | 本書の「残す候補」の欄・§7 | 討議で 3 種類の裁定を仰ぐ（**Constraint**） |
| 3（全部直す・呼び手まで） | 「範囲外」の欄 | 範囲の外へ及ぶ直しが 13 件（#4・5・17・18・22・25・27・40・41・56・71・72・79 のうち任意を含む）＝**Constraint** |
| 4（テストで縛る） | 既存の檻の形（`log_capture_kit::capture`・`TempPath`・`NarBuilder`・`WorkDir`） | Must fix と Bug の各件に落ちるテストが要る（#1〜#3・#6〜#8・#15〜#19・#22・#28・#29 など）＝**Missing** |
| 5（決まり） | 1,000 行の番人・一時フォルダの番人 | `inproc_e2e_test.rs` は例外表に載る＝1,000 行を割ると番人の「まだ超えている」が赤（§6）＝**Constraint** |
| 6（範囲の外） | — | 並走中の `audit-parsers-sakura` に及ぶのは #22（任意）・#79＝申し合わせ |
| 7（`BoxedBackend`） | `tests/ghost/recorder.rs` を 2 つの実行ファイルが共有済み | #42 の形で 1 か所に（新しいファイル不要） |
| 8（かけ直し 0 件） | — | 最後のタスクで同じ作法でかけ直す |

## 6. 1,000 行の番人と `inproc_e2e_test.rs`

- `crates/log-capture-kit/tests/file_length_guard_test.rs` の例外表に `crates/areka-ghost/tests/ghost/inproc_e2e_test.rs` が載る。番人には「例外の項目はまだ上限を超えている」こと（`every_over_limit_exception_is_still_over_the_limit`）を見るテストがあり、**1,000 行以下へ下がると赤**になる。ロードマップの決まりは「どの spec も例外表に触れない」。
- 今 1,160 行。#42（`BoxedBackend`）で −55−4+1 ＝**1,102 行**（`use` を 1 行にまとめず rustfmt が 3 行に折れば 1,103）。#4（I3 の一時フォルダ）で約 −12、#85 で約 −20、#86 で約 −12 ＝合わせて**約 1,058 行**。どの組み合わせでも 1,000 を超えたまま。
- もし設計で I1〜I3 を別のファイルへ分ける形を選ぶと 1,000 を割り、例外表を直す要が出る＝その形は取らないか、開発者の決めを仰ぐ。

## 7. 実装の進め方の案

- **案 A（今のファイルを直す・推す）**: 指摘ごとに今のファイルの中で直し、テストの道具はクレートごとに 1 か所（`areka-ghost` の `src` は `#[cfg(test)] mod test_support`、`tests/` は `common.rs`／`recorder.rs`、`spine` は親の `spine_e2e_test.rs`、`areka-nar` は `lib_tests.rs`、`areka-user-data` は新しい `test_support`）へ寄せる。新しい本番のファイルは作らない。
  - 良い: 今の層と決まりのまま・差分が小さい。悪い: テストのファイルの行が一時的に動くので、1,000 行の近いファイル（#14・#37・#75）は先に減らす順にする。
- **案 B（テストの道具を先に新しいファイルへ）**: 共有の道具を新しい `*_test_support.rs` に先に作り、指摘の直しはその後。
  - 良い: 1,000 行の心配が早く消える。悪い: 新しいファイルが増え、テストの行が数え直しになる。
- **案 C（組ごと）**: Must fix を先に 1 段で直して全体テスト、次に Should fix、最後に Nice to have。
  - 良い: 危ない所が早く直る。悪い: 同じファイルに 3 回触る。
- 規模の見立て: **L**（1〜2 週間）。理由: 91 件・3 クレート・範囲の外 13 件。危うさ: **中**（#1 の錠、#2 の部分読み、#16 の片付け、#45 の I1／I2 の錠は新しい形を引く。ほかは既にある形の中）。
- 減る行の見立て: 道具の写しと落ちないテストを消して、テストの側で約 −700〜−900 行。依存は 1 つも増えない（#1 の錠も #28 の `LoadLibraryExW` も今の依存の中）。

## 8. 討議に渡す議題の種（要件 2）

1. **直さずに残す候補（⑴ 開発者の裁定）**: #4・#5（一時フォルダの番人の例外表＝完了した spec の決め。直すと `audit-test-kits` の持ち物の表を直す）・#17（要件 5.5）・#18（要件 5.1）・#30（design Risks の warn）・#39（design の保持物の節）・#41（要件 4.1）・#57（要件 2.4）・#68（要件 10.8）・#79（タスク 3.2 の置き場所）・#8（リンクの扱いの決めが要る）。
2. **直さずに残す候補（⑵ 先送りの印）**: #63（2 秒の待ちを受け入れた注）・#71（「既知の盲点」）。#26 は印の限界が切れているので残せない。
3. **直さずに残す候補（⑶ SSP／ukadoc）**: #7（同じ宛先の 2 つの同梱）・#22（descript の鍵の大小と空の `name`）・#41（Reference7 は「シェル名」＝ukadoc。フォルダ名か descript の名か・SSP の実測は `ssp_shiori_log.txt`）・#71（`charset` の行の位置）。ukadoc で引いた結果: OnGhostChanged の Reference7 は「切り替わったゴーストのシェル名。[SSPのみ]」。descript の鍵の大小の定めは ukadoc の検索で見つからなかった（**Research Needed**）。
4. **作り直しで消える直し**: #27・#28・#29（と #26・#33・#34）は `shiori_inproc.rs` の中で、`shiori4-api`（v0.2.0・この見直しの着地を待つ）が `InProcHost` を消し、`load` の形を決めると brief に書いてある。今直すか（開発者の方針「全部直す」）、⑴ として残すかを決める。
5. **範囲の外へ及ぶ直し**: 並走中（C17）の `audit-parsers-sakura` に及ぶ＝#22（揃えるなら `resolve_shell.rs`）・#79（`areka-sakura` の compile のテスト）→ 申し合わせてから触る。C17 に居ない持ち主に及ぶ＝#4・#5（`audit-test-kits`）・#17・#18・#25（`audit-areka-app`・`audit-mcp-update-crates`）・#27（`audit-shiori-host32`）・#40（`audit-sylphya-actor`）・#41（`audit-kanade`）・#56・#71・#72（`crates/areka` のテスト）。
6. **1,000 行の番人**: `inproc_e2e_test.rs` は直しの後も約 1,058〜1,103 行で 1,000 を超えたまま（§6）。分けない。
7. **タスクの数と切り分け（要件 2.6）**: 直しをファイルのまとまりで束ねた見立て＝Must fix 5 件で 4〜5 タスク・Should fix 47 件で 15〜17 タスク・Nice to have 39 件で 7〜9 タスク・かけ直しと全体テストで 2 ＝**約 28〜33 タスク**で 20 を越える。切り分けの案:
   - 案 ①（組で切る・推す）: 1 本目＝Must fix＋Should fix＋かけ直し（約 20〜22）・2 本目（同じ範囲）＝Nice to have（約 8〜10）。1 本目が 20 を少しまたぐなら、Should fix のうちテストだけの写しの寄せ（#35〜#38・#42〜#44・#50〜#52）を 2 本目へ回す。
   - 案 ②（本番とテストで切る）: 1 本目＝本番のコードに触る指摘（Must fix の #1〜#3・Should fix の #6〜#8・#15〜#19・#22〜#30）＋要件 7（約 14〜16）・2 本目＝テストだけの指摘（約 15〜17）。要件 2.6 の「組ごと」からは外れる。
   - どちらでも要件 8（かけ直し 0 件）は 2 本目の終わりで見る形になる＝1 本目のかけ直しは「1 本目の受け持つ指摘が 0 件」で数える決めが要る。

## 9. 調べが要るもの（Research Needed）

- descript.txt の鍵が大小を区別するか（ukadoc・SSP）。`name,`（空の値）のとき SSP が OnBoot の Reference0 に何を送るか（#22）。
- SSP が OnGhostChanged の Reference7 に送る値（フォルダ名か descript の名か）を `ssp_shiori_log.txt` で測る（#41・測るには開発者の許しが要る）。
- 同じ宛先を指す 2 つの `*.directory` を SSP がどう扱うか（#7）。
- #45（I1／I2 の競り）と #1（2 つのプロセスの同時のインストール）は、コードを読んでの推定で、走らせての再現はしていない。直しのテストで再現の形（錠を外すと赤）を作る。
- #2 の部分読みで、中央ディレクトリのオフセットが書庫の大きさを越える壊れた書庫の拒み方（今の `read_central_directory` の語彙を変えずに済むか）。

## 10. 設計の段の調べと決め（2026-11-05・`/kiro-spec-design -y`）

### 要約

- **調べの種類**: 既存の仕組みの直し（軽い調べ・`design-discovery-light`）。新しいライブラリは無い。外の文書は引かず、コードを読んで決めた（ukadoc の確かめは要件の討議で済み＝要件 9.2・9.8・9.9）。
- **使った手引き**: `kiro-spec-design` の `design-principles`・`design-discovery-light`・`design-synthesis`・`design-review-gate`、ponytail の「いちばん小さい完全な変更」（利用者全体のプラグイン）。
- **大事な見つけ**:
  - #2 の判定は今 `read_central_directory` を通るので、目次の各件でローカルの頭も読む（全体のバイト列が要る）。目次の読みを「ローカルの頭を見るか」の引数つきに分ければ、`open` の断る順を変えずに判定だけ末尾と目次で済む。
  - #1 の錠は `std` の `share_mode(0)`＋`FILE_FLAG_DELETE_ON_CLOSE` で足りる（依存は増えない）。待たずに失敗にすれば、根が 2 つの書庫で 2 つのプロセスが待ち合って止まることが無い。
  - #45 の競りは、錠を取ってから消す口と `try_lock` の口を並べれば、1 つのスレッドの順で再現できる（時間待ちを使わない）。
  - 一時フォルダの番人は、#4・#5 の 5 行を消すと「固定名が仕様」の種別を使う項目が 0 になり、組まれない種別が警告になる＝種別ごと消す。較正で外す 1 件は C17 で走っている spec の持ち物でない `crates/areka/src/install/fetch_url.rs` へ替える。
  - `audit-parsers-sakura` の要件 7.10 は「`lowercased` を parsers の kv の隣へ置き package で使う」まで。乗り換えと `config.rs` は本 spec。

### 調べたこと

#### #1 棚の錠

- **きっかけ**: `prepare_shelf` は自分の番地以外の項目を、保持の期限の内側でなければ全部消す。
- **見たもの**: `crates/areka-nar/src/install.rs`（`WorkArea::create_at`・`next_address`・`prepare_shelf`）・`lib.rs`（`place` が根ごとに `WorkArea` を掘り、終わるまで持つ）・`.nar-work` を名指すテスト（`install_tests.rs`・`lib_tests.rs`・`crates/areka/src/install/destination_tests.rs` ほか）。
- **分かったこと**: `WorkArea` の寿命が「組み上げ・確定・後片付け」の全部を覆うので、錠をそこに持たせれば足りる。棚は展開の後も残るので、錠のファイルは閉じたら消える形にして、根の木を比べるテストの期待を変えない。
- **影響**: `prepare_shelf` は `.lock` を飛ばす。2 本目は `StageError`→`NarError::Io{Stage}`→`log_failure` の `error!`（新しい記録の名前は要らない）。

#### #2 部分読み

- **見たもの**: `container.rs`（`find_eocd`・`read_eocd`・`read_central_header`・`locate_entry_data`）・`lib.rs`（`peek_install_txt`・`read_archive_bytes`）・`crates/areka/src/input_events/file_drop.rs`（読めない書庫は `warn!` してインストール対象でないファイルとして扱う）・`lib_peek_tests.rs`。
- **分かったこと**: `find_eocd` は「署名の位置＋22＋コメント長＝バイト列の長さ」で探すので、ファイルの末尾の窓の上でもそのまま動く。範囲の検査だけファイル全体の長さが要る。ローカルの頭の壊れは判定では見えなくなるが、`install.txt` 在り＝インストール対象（入らなければ `OnInstallFailure`）の決めに沿う。
- **影響**: `open` の大きさの上限は、宣言の総和の上限 1 GiB の 2 倍（2 GiB）。越える書庫は中身を全部読んでも必ず断る。語彙は `Unsupported::TooLarge` を使い、増やさない。

#### #15・#16 写しの守り

- **見たもの**: `crates/areka-user-data/src/copy.rs`・呼び手 3 つ（`establish.rs` の `migrate`・`crates/areka/src/roots/user_root.rs`・`crates/real-run-kit/src/evidence.rs`）。
- **分かったこと**: 呼び手はどれも失敗を記録する（`error!` か標準エラー）。`real-run-kit` の写しの行き先は走りのフォルダで、写す元の外。
- **影響**: 番地の取り直しと片付けは `areka-nar` の `next_address`／保持の期限と同じく、連番と時刻を引数で受ける形にしてテストする。片付けに錠は使わない（今の「先を越されたら `Raced`」の決めを変えないため）。

#### #45 I1・I2

- **見たもの**: `inproc_fixture.rs`（`shared_test_ghost` は呼ぶたびに `profile\areka` を消す）・`inproc_e2e_test.rs`（I1・I2 の頭で呼ぶ）。
- **影響**: 消去を錠の内側へ移し、I1・I2 は終わるまで錠を持つ。

#### #28・#29

- **見たもの**: `shiori_inproc.rs`（`load_library_quiet`・`inproc_connect`）・`shiori_wiring.rs`（`helper_load_dir`）・`areka-ghost/Cargo.toml`（`windows` の `LibraryLoader` は既に使える）。
- **影響**: `LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR` は絶対パスを要るので、#29 の整えと組みで直す。縛るテストは「相対の DLL パスは読めない」（`LoadLibraryW` へ戻すと今の作業フォルダから読めて赤）。

### 寄せ方の見比べ

| 案 | 中身 | 良い | 悪い | 決め |
|---|---|---|---|---|
| 今のファイルを直す（`research.md` §7 の案 A） | 指摘ごとに今のファイルで直し、道具はクレートと組み立ての単位ごとに 1 か所 | 層と決まりのまま・差分が小さい | 1,000 行の近いファイルは先に減らす順が要る | 採る |
| 道具を先に新しいファイルへ（案 B） | 共有の道具を新しい `*_test_support.rs` に先に作る | 行の心配が早く消える | ファイルが増える | 採らない（新しいのは `src/test_support.rs` 2 つだけ） |
| 組ごと（案 C） | Must → Should → Nice の順に 3 段 | 危ない所が早い | 同じファイルに 3 回触る | 採らない（要件 9.13 がファイルのまとまりで束ねると決めた） |

### 決め

#### 決め: 棚の錠は待たずに失敗にする

- **見比べ**: ⑴ 待つ（取れるまで繰り返す） ⑵ すぐ失敗にする。
- **採る**: ⑵。根が 2 つの書庫を 2 つのプロセスが逆の順で掘ると、⑴ は互いに待って止まる。失敗は `OnInstallFailure` で利用者に届き、入れ直せば入る。
- **引き換え**: ほぼ同時の 2 回のインストールの片方が失敗する（今は片方が他方の作業を壊す）。

#### 決め: 判定の目次の読みはローカルの頭を見ない

- **見比べ**: ⑴ 目次の各件でローカルの頭だけを小さく読みに行く（答えは今と同じ） ⑵ 目次だけを読む。
- **採る**: ⑵。⑴ は件の数だけ読みに行き、読みの口を抽象にする手間がかかる。⑵ で食い違うのは「ローカルの頭だけが壊れた書庫」だけで、それは `open` が断り `OnInstallFailure` になる（「install.txt 在り＝インストール対象」の決めどおり）。

#### 決め: 名前の比べ方は `to_lowercase` 1 つ

- **見比べ**: ⑴ ASCII の大小無視へそろえる ⑵ 名前の検証と同じ `to_lowercase` へそろえる ⑶ OS の比べ方（`CompareStringOrdinal`）。
- **採る**: ⑵。名前の検証が既に Unicode の規則で衝突を止めているので、計画と展開を合わせるのが小さい。⑶ は `catalog_roots.rs` の `ponytail:` の注にある上げ先で、食い違いの報告はまだ無い。

#### 決め: テストの道具の置き場は組み立ての単位ごとに 1 つ

- **理由**: 単体テスト（`src`）と結合テスト（`tests/`）は別の組み立ての単位で、互いのコードが見えない。`src/test_support.rs` と `tests/ghost/common.rs` の 2 つは写しでなく、それぞれの単位の唯一の置き場。

#### 決め: #22 の乗り換えは相手の着地の後・本 spec の完了の前

- **理由**: 要件 9.10 は本 spec が乗り換えると決めた。相手の関数が無い間は自前の `lowercased` で直し（バグはその場で直る）、T18 の頭で乗り換える。自前の写しを残したまま完了しない。#79 の削除も同じ順。

### 寄せて小さくしたもの（統合の 3 つの目）

- **共通化**: 「棚の番地を取る・古い物を片付ける」は `areka-nar` と `areka-user-data` で同じ形だが、クレートをまたぐ path の依存は `Cargo.lock` を変えるので作らない。形（連番と時刻を引数で受ける）だけそろえる。
- **作るか借りるか**: zip の部分読みは今の自前の読み手を分けて使う（新しいライブラリは足さない・依存 0 の約束）。錠は `std` の共有の口。
- **削り**: 錠の待ち・目次の大きさの別の上限・新しい拒みの語彙・新しい記録の名前（`profile_dir_env_unusable` の 1 つだけを足す）は持たない。

### 危うさと手当て

- `FILE_FLAG_DELETE_ON_CLOSE` で錠のファイルが消えない作りだったとき — 縛るテスト `the_shelf_lock_is_released_when_the_work_area_is_dropped` が見つける。そのときは `WorkArea` の `Drop` で消す 1 手を置く（根の木を比べるテストの期待は変えない）。
- `inproc_e2e_test.rs` が 1,000 行を割る — 見立ては約 1,045 行。T15・T16 の終わりに測り、割る直しはしない。
- 相手（`audit-parsers-sakura`）の着地が遅れる — #22 のバグは T9 で自前の読みで直る。T18 の乗り換えと #79 の削除だけが待つ。
- #28 の縛るテストは「相対の DLL パスを拒む」で旗の効きを間接に見る（依存の DLL を持つテスト用の DLL は無く、作ると新しいクレート＝`Cargo.lock` が変わる）。
