# 検証報告: areka-P0-nar-install

本 spec の検証で「走らせて確かめた事実」を残す。後続のタスクは節を足す（既にある節は書き換えない）。

---

## 5.1 検体の畳み込みと往復の一致（2026-09-19）

対象要件: 8.2・8.3・8.4・8.5・8.6・8.7

### 何をしたか

展開済みで追跡していた検体 4 本を `vendors/sample_ghost/*.nar` へ畳み、畳んだ `.nar` を本番の展開器（`areka-nar`）で空の根へ入れ直して、元のツリーと**ファイル集合とバイト列**を突き合わせた。

畳む手順は使い捨ての実行体 `crates/sample-ghost-kit/examples/fold-samples.rs` に置いた。

```
cargo run -p sample-ghost-kit --example fold-samples            # 登記表の検体を全て畳んで往復まで確かめる
cargo run -p sample-ghost-kit --example fold-samples -- --from <展開形のフォルダ>  # そのフォルダ 1 本だけ
cargo run -p sample-ghost-kit --example fold-samples -- --check # 畳まずに往復だけ確かめる
```

本タスクで実際に踏んだのは引数無しの形（4 本まとめて）である。ただし**後続の検体を足すときに使えるのは `--from` の形だけ**なので、`vendors/sample_ghost/README.md`（タスク 6.4）に書くのはこちらにすること。

```
cargo run -p sample-ghost-kit --example fold-samples -- --from vendors/sample_ghost/StayseeBalloon
```

理由——引数無しの形は登記表 `SAMPLES` を回り、各検体の在処を `checked_in_parent` から組む。タスク 5.6 が旧置き場（`crates/pilot/examples/shiori-host-32/fixtures/` と `vendors/sample_ghost/R_POST_and_KOMAINU/`）を消した後は、`git ls-files` が空を返して最初の検体で「追跡ファイルが 0 件」となり落ちる（`git ls-files` は実在しないパスに対して空・終了コード 0 を返すので、失敗が静かに来る）。`--from` は登記表を見ず、渡されたフォルダだけを畳むので、旧置き場の有無に依らない。**登記表への 1 行は畳んだ後に足す**。

`--from` が使う 3 つの判定（追跡 0 件でないこと・写しの数が追跡の数と等しいこと・写像が 2 つの元を同じ宛先へ潰していないこと）と写像の作り方（`install.txt` の解釈結果だけから作る）は、引数無しの形とまったく同じものである。実測でも、4 本すべてを `--from` で畳み直して SHA-256 が変わらないことを確かめた。

外部の書庫道具は使っていない。畳むのはタスク 2.4 の `sample_ghost_kit::nar_writer::fold_tree`（無圧縮・方式 0）で、`.nar` を読むのは `areka_nar::NarArchive` である。

**入れるファイルの出どころは `git ls-files` だけ**にした（要件 8.3）。フォルダを走査すると追跡外の永続化フォルダまで入る——emo2 の木は畳んだ日、ディスク上 157 ファイル・追跡 110 ファイルで **47 件ずれていた**（差は全て `ghost/master/profile/` の配下）。手順は追跡ファイルだけを作業フォルダへバイト複製してから畳むので、改行も文字コードも変わらない（要件 8.7）。

### 検体ごとのファイル数とハッシュ

| 検体 | 種別 | 追跡ファイル数 | 展開後のファイル数 | 展開先の内訳 | `.nar` の大きさ | `.nar` の SHA-256 |
|------|------|---------------|------------------|-------------|----------------|-------------------|
| `emo2` | ゴースト（同梱バルーン 1） | 110 | 110 | `ghost/emo2` = 90 ／ `balloon/emo2-kakukaku` = 20 | 6,631,325 バイト | `77BA5D93E3347CC6FE2CC3CCA0EBA9EC079D6BE740A270945EE7A980A81FFDB8` |
| `R_POST_and_KOMAINU` | ゴースト | 43 | 43 | `ghost/R_POST_and_KOMAINU` = 43 | 1,625,603 バイト | `BEB956865E4F651DF0846FB28A8E8F209B56C1C963B63AE8255E5B7E2552DE71` |
| `emo2-kakukaku-offsetdpi` | バルーン | 20 | 20 | `balloon/emo2-kakukaku-offsetdpi` = 20 | 35,788 バイト | `699F19474F29B75CA8F14AAF53DAB9838D415CC1C678639FCD06EDE7117376BD` |
| `emo2-kakukaku-wplimit` | バルーン | 20 | 20 | `balloon/emo2-kakukaku-wplimit` = 20 | 33,906 バイト | `A62E08F78C6D4AAA36F68FDC8541A23E6DC757D9B5701DAC814BC69AD870FEAF` |

合計 193 ファイル・8,326,622 バイト。**4 本すべてで不一致 0 件**（ファイル集合・バイト列とも）。

要件が数えていた値と一致する——8.2 の「110（ゴースト本体 90＋同梱バルーン 20）」・8.3 の「43」「各 20」。

参考: git の索引に入った blob（SHA-1）は `emo2` = `35c7e5ed9734aed161986b7f364ae56f6bdb38bb`・`R_POST_and_KOMAINU` = `c3f20f883f262426492ae7070c97ce8608582e7f`・`emo2-kakukaku-offsetdpi` = `0d355a37c5d6a756d3657af26e88b502bb1f412c`・`emo2-kakukaku-wplimit` = `adff06a3daf27a6f766e19e01d919b15d0555ed9`。

### emo2 の「和」の突き合わせ（要件 8.4）

emo2 だけは、元のツリーの 1 本が展開後は**2 つのフォルダに分かれる**。写像は `install.txt` の解釈結果（`balloon.source.directory` と `balloon.directory`）だけから作り、登記表の綴りは使っていない。

- `<元>/emo2-kakukaku/…`（20）→ `<根>/balloon/emo2-kakukaku/…`
- それ以外（90）→ `<根>/ghost/emo2/…`

突き合わせは片側だけでなく**両方向**に行った——⑴ 展開後に置かれた全ファイルが写像の像に在ること（元に無いものが増えていない）⑵ 写像の像の全ファイルが置かれ、かつ元とバイトが同じこと。数が 0 のまま緑になる経路を塞ぐため、追跡ファイルが 0 件でないこと・写しの数が追跡の数と等しいこと・写像が 2 つの元を同じ宛先へ潰していないことを、手順が実行時に判定する。

### 同じ入力からは同じバイト列（畳み直しの較正）

続けて 2 度畳み、4 本すべての SHA-256 が変わらないことを確かめた。`fold_tree` が走査順を名前順に固定し DOS 日時を 1980-01-01 00:00 に固定しているので、畳み直しでも差分が出ない。

### 外部の読み手による裏取り

`.nar` を作るのには使っていないが、**確かめる側**には独立の読み手（Python の `zipfile`）を当てた。4 本とも `testzip()` が `None`（全エントリの CRC が合う）・圧縮方式は全て 0（無圧縮）・ファイルのエントリ数は 110／43／20／20・フォルダのエントリ数は 18／6／0／0。

### `.nar` をバイト保存で追跡していること（要件 8.6 前半）

規則の字面ではなく **git 自身の見え方**で確かめた。

```
$ git check-attr text eol -- vendors/sample_ghost/*.nar
vendors/sample_ghost/emo2.nar: text: unset          ← `-text` ＝ 変換しない
vendors/sample_ghost/emo2.nar: eol: unspecified
（4 本とも同じ）
```

規則は既にある `vendors/sample_ghost/.gitattributes` の `* -text` で、`.nar` にも効いている。較正として、属性の届かない場所の同じ拡張子を問うと `text: unspecified` になる（`crates/areka/x.nar`）＝問い方が場所を見ていることの対照。

索引を往復させたバイト列も突き合わせた。4 本を `git add` し、`git show :<パス>` で取り出したバイト列が作業ツリーのファイルと一致した（`cmp` で 4 本とも差分 0・`LF will be replaced by CRLF` の警告も出ない）。

さらに「属性が効いていること」自体の較正として、同じ内容を場所だけ変えて git に読ませた。CRLF のテキストは `vendors/sample_ghost/` 扱いで `c30dea8a…`・属性の無い場所扱いで `422c2b7a…` と**別のハッシュ**になる（変換機構が生きている）。`.nar` そのものは両扱いで同じハッシュ（`adff06a3…`）で、これは git の二値判定（NUL バイトを含む）が先に変換を止めているため。つまり `.nar` の保護は二重で、`-text` は git の推測が外れる将来（先頭 8000 バイトに NUL が無い書庫）への備えとして効いている。

### 無視規則が検体の一部を落としていないこと（要件 8.6 後半）

`.nar` の中身の**全ファイル名 193 件**を取り出し、`git check-ignore --no-index --stdin` に通した。`--no-index` を付けるのは、既定の `git check-ignore` が索引に在るパスを「無視されない」と即答してしまい、**追跡済みであることを理由にした恒真の 0** になるため。規則そのものに問うている。

2 つの置き場で数えた。

| 問うたパス | 無視されたもの |
|-----------|--------------|
| 元の置き場（`crates/pilot/examples/shiori-host-32/fixtures/<名>/…` と `vendors/sample_ghost/R_POST_and_KOMAINU/…`） | **0 件 / 193** |
| 保管先の配下（`vendors/sample_ghost/<名>/…`） | **0 件 / 193** |

較正はこの 3 件。

1. **`dic09_Test.txt` が「無視されない」と判定される**（要件 8.6 が名指しした較正）。`-v` を付けると理由の規則まで出て、当たっている最後の規則が否定側であることが見える。

   ```
   vendors/sample_ghost/.gitignore:5:!*_test.txt
       vendors/sample_ghost/R_POST_and_KOMAINU/ghost/master/dic09_Test.txt
   ```

2. **同じ名前を否定規則の無い場所に置くと無視される**——`crates/pilot/examples/shiori-host-32/fixtures/emo2/ghost/master/dic09_Test.txt` も `crates/areka/src/dic09_Test.txt` も、リポジトリ直下の `.gitignore:6:*_test.txt` に当たって無視される。判定が素通りしていないことの対照であり、同時に**里々の辞書がこの拡張子の規則 1 行で黙って消え得た**ことの実測でもある（`core.ignorecase=true` なので大小違いでも当たる）。
3. **追跡外の永続化フォルダは無視される**——`crates/pilot/examples/shiori-host-32/fixtures/emo2/ghost/master/profile/areka/sylphya.toml` は `crates/pilot/examples/shiori-host-32/.gitignore:3:fixtures/emo2/ghost/master/profile/` に当たる。畳む対象から外れた 47 件が「たまたま無かった」のではなく規則で落ちていることの確認。

### `StayseeBalloon` は在らなかった（要件 8.5）

畳んだ時点で `vendors/sample_ghost/StayseeBalloon/` は**存在しない**。したがって `StayseeBalloon.nar` は作らず、共有ヘルパの登記表 `SAMPLES` にも登記していない。

**畳むのは `areka-P0-default-balloon-bundle` が行う**（要件 10.10 の手順＝`vendors/sample_ghost/README.md` に従う）。同 spec の `brief.md` に 2026-09-19 の追記としてこの申し送りを書いた。

### 書き写した `install.txt` の実物との再照合（タスク 3.3 からの持ち越し）

`crates/areka-nar/src/manifest_tests.rs` は実物の `install.txt` 4 本を `include_bytes!` ではなく**バイト列の写し**として持つ。タスク 5.6 が元のツリーを消すため、実物がディスクに在るのはこの時点が最後になる。4 本すべてを実物と 1 バイト単位で照合した。

| 定数 | 実物 | 結果 |
|------|------|------|
| `EMO2_GHOST` | `crates/pilot/examples/shiori-host-32/fixtures/emo2/install.txt`（135 バイト） | 一致 |
| `R_POST` | `vendors/sample_ghost/R_POST_and_KOMAINU/install.txt`（83 バイト） | 一致 |
| `EMO2_BALLOON` | `crates/pilot/examples/shiori-host-32/fixtures/emo2/emo2-kakukaku/install.txt`（78 バイト） | 一致 |
| `HELLO_PASTA` | `vendors/pasta/crates/pasta_sample_ghost/ghosts/hello-pasta/install.txt`（62 バイト） | 一致 |

較正として、写しの 1 バイトを反転させた値を同じ照合に掛け、「違う」と報告されることを確かめた（照合が常に一致を返す作りになっていない）。

なお `HELLO_PASTA` の実物は `vendors/pasta`（別リポジトリ）の配下で、タスク 5.6 の削除対象ではない。

### 残した懸念

- `.nar` は**無圧縮**（方式 0）のまま置く。`fold_tree` が圧縮を掛けないため。「展開形が消えるから追跡量は釣り合う」という理屈は**成り立たない**——git の履歴は消えず、展開形の 193 blob（5,448,314 バイト）は永久に残る。無圧縮のままでよい本当の理由は、**git 自身の zlib が既に同じ仕事をしている**ことである。4 本を git オブジェクトにすると、無圧縮のまま **5,442,668 バイト**（`.git/objects` に実在する 4 つの loose object の実寸を足した値）・deflate を掛けてから入れると **5,273,192 バイト**（レビュー側の見積り。上の無圧縮の実寸を再現できることで較正済み）で、差は **169,476 バイト（3.2%）**しかない。既に 53.48 MiB あるパックに対して 3.2% のために `nar_writer`（タスク 2.4 の完了済みモジュール）を開け直す価値は無い。小さいバルーン 2 本に至っては、二重圧縮の費用が利得を上回って**無圧縮のほうが git 上で小さい**。
- 検体が方式 0 だけなので、**要件 9.7 の実機一周が踏む圧縮方式は 0 だけ**である。方式 8（deflate）を踏むのは `areka-nar` の決定論テスト 8 本（`container_tests.rs` 6 本・`install_tests.rs` 1 本・`lib_tests.rs` 1 本）に限られる。実機で一周したことをもって deflate の経路まで確かめたとは書かないこと。
- `vendors/sample_ghost/.gitignore` の否定 2 行は、タスク 5.6 が展開形を消した後は**当たる相手が居なくなる**（`.nar` の名前は `*_test.txt`／`*_dump.txt` に当たらない）。消すと将来また展開形を置いたときに同じ罠に落ちるので、消すなら理由を書き足すこと。
- 同じ期限切れが `vendors/sample_ghost/.gitattributes` にもある。`* -text` の理由書きは「里々テンプレートは辞書も `surfaces.txt` も `descript.txt` も全行 CRLF」と**消える展開形だけ**を根拠に書いてある。5.6 の後に読んだ人は規則を狭めたり消したりして当然なので、5.6 で `*.nar binary` を足すか理由書きを `.nar` の話へ書き改めること。なお今日の時点では規則は十分に効いており（`git check-attr text` が `unset`）、`.nar` は先頭から 5 バイト目に NUL が来るので git の二値判定でも独立に守られている。

---

## 5.6 展開済みツリーの削除と、その場しのぎの初期化の解消（2026-09-19）

対象要件: 8.1・8.8・8.9

### 何をしたか

追跡していた展開形を消し、追跡対象の検体を配布形（`.nar`）だけにした。消す前に、消える 2 つの木を綴っている場所をワークスペース全体で数え直し、1 件ずつ行き先を決めてから消した。

### 消す前に数えた参照（削除の前提）

区切り記号は `/` と `\` の両方で探し、`target/` は除いた。

| 置き場 | 件数 | 行き先 |
|--------|------|--------|
| `.rs` の**実行行** | **0** | 段 ① で全て窓口経由に寄せ済み。コンパイルではなく数えて確かめた（文字列なので消えても落ちない） |
| `.rs` の**コメント行**（16 ファイル） | 16 ファイル | 本タスクで文言を改めた（下記） |
| `.rs` の合成テスト値 | 1 | `crates/pilot/examples/shiori-host-32/shiori3.rs` の `build_onboot` 引数（下記） |
| `crates/pilot/examples/shiori-host-32/.gitignore` | 1 ファイル | 削除（要件 8.9） |
| `crates/pilot/examples/shiori-host-32/README.md:26` ほか | 2 か所 | **タスク 6.1 の担当**（`_Boundary:` が「pilot example README」と名指ししている）。本タスクでは触らない |
| `tools/perf/{invoke-followup-checks.ps1,perf-loop.measure.ps1,judge-perf.py}` | 3 ファイル | **タスク 6.1 の担当** |
| `doc/emo2-conformance-scope.md:25` | 1 | **タスク 6.1 の担当** |
| `doc/ukadoc-coverage/briefing-assets.md:362,377,439` | 3 | **タスク 6.3 の担当** |
| `.kiro/steering/roadmap-history.md` の 2 行 | 2 | 過去の記録なので**そのまま残す**（当時そこに在ったことが事実） |
| `vendors/sample_ghost/emo2-kakukaku-{offsetdpi,wplimit}.nar` の中の `readme.txt` | 2 本 | **触らない**。要件 8.7 が中身のバイト不変を求める。書庫の中の説明文が旧置き場を綴っているのは、畳んだ時点の記録として正しい |

旧置き場を綴る `.rs` は、削除後に 0 件であることを同じ検索語で数え直して確かめた（`vendors/sample_ghost/*.nar` を名指しする文書・スクリプトは要件 1.8 が禁じていない）。

### 消したもの

| 対象 | 追跡ファイル数 |
|------|----------------|
| `crates/pilot/examples/shiori-host-32/fixtures/` | 150 |
| `vendors/sample_ghost/R_POST_and_KOMAINU/` | 43 |
| `crates/pilot/examples/shiori-host-32/.gitignore` | 1 |

削除後、`git ls-files vendors/sample_ghost` が返すのは `.nar` 4 本と `.gitattributes`・`.gitignore` だけである（要件 8.1）。`git status` は追跡外の新顔を 1 件も出さない——消した `.gitignore` が隠していた `fixtures/emo2/ghost/master/profile/` も木ごと消えたためである。

消した木が `.nar` から取り出し直せることは窓口のコマンドで確かめた。`nar-sample-path -- R_POST_and_KOMAINU` が配った根は 43 ファイルで、大小違いの名前 `dic09_Test.txt` も在る。

### 起動記録を消す初期化の解消（要件 8.8）

`crates/areka/src/emo2_boot/spine.rs` の私家版 `emo2_root()` は、呼ぶたびに `<検体>/ghost/master/profile/areka/` を `remove_dir_all` していた。段 ③ では `sample_test_support` の `static LazyLock<SampleRoot>` 1 つを spine の 2 つの起動口が共有していたので、**この初期化を消すだけでは同じプロセスの 2 回目の起動が 1 回目の `[boot] count` を読み**、`OnFirstBoot` を出さなくなる。

実際に初期化だけを消して走らせたところ `spine_s5_close_handshake_consumes_onclose_and_joins_all_handles_bounded` が落ちた。記録された呼出列は `OnInitialize → username → OnBoot → basewareversion` で、**`OnFirstBoot` が抜けている**。

直し方は「消して祈る」でも「残す」でもなく、**取得の粒度を起動に合わせる**ことにした。窓口は取得のたびに起動記録の無い新品を配る（要件 7.4）ので、起動ごとに取得すれば初期化は要らない。

- `sample_test_support` に `acquire_emo2()`（1 つ取得する）を足し、共有の静的値はそれを呼ぶ形にした。読むだけのテスト（`assets_tests`・`frame_attach_tests`・`frame_visibility_integration_tests`・`mod.rs` の `wire_tests`）は今までどおり静的値を使うので、木の複製はテストバイナリあたり 1 つのままである。
- `spine.rs` の `boot_with` が起動ごとに `acquire_emo2()` を呼び、得た値を `SpineHarness` の**最後の欄**で保持する。後片付けでも最後に捨てる。ただしこの順は**用心であって、裏付けるテストは無い**——欄を先頭へ移しても、`drop(sample)` を `ghost.shutdown` より前へ移しても 33 本すべて緑になる（終了処理が書く永続化は失敗しても捨てられ、どのテストも読まない）。順を守る理由は、木が在るうちに畳むほうが後から観測を足したときに驚きが少ないことだけである。コード側のコメントも同じ言い方に直した。
- 初期化を持っていた私家版 `emo2_root()` は無くなった。

この直しが効いていることは摂動で確かめた。`ghost_root` だけを共有の静的値へ戻すと `spine_harness_boots_scripted_ghost_and_reaches_attach_ready`・`spine_s5_close_handshake_consumes_onclose_and_joins_all_handles_bounded`・`spine_close_request_runs_the_farewell_then_the_quit_phase_closes_the_windows`・`kanade_probe_raises_no_shiori_call_and_observes_the_close` の 4 本が赤になり、戻すと 33 本すべて緑に戻る。

### コメントの文言（16 ファイル）

旧置き場の綴りを、窓口が使う**検体名を起点にした言い方**へ改めた（例: `crates/pilot/examples/shiori-host-32/fixtures/emo2/shell/master/descript.txt` → `検体 emo2 の shell/master/descript.txt`、同梱バルーンは `検体 emo2 の同梱バルーン emo2-kakukaku の descript.txt`）。絶対パスが要る手順（`transition_judge_offset_signoff_tests.rs` の実機採取手順・`emo2_real_run.rs` の直接起動）は、`nar-sample-path` の `folder=` の行から得る形に書き替えた。

`crates/pilot/examples/shiori-host-32/shiori3.rs` の `build_onboot` テストは、旧置き場を**バックスラッシュ区切りの文字列リテラル**で持っていた（タスク 1.5 が本タスクへ送った 1 件）。`build_onboot` は ghostdir を捨てる（SHIORI/3.0 の要求に載らない）ので合成値でよく、`Path::new("ghost/master")` に置き換えた。バックスラッシュ綴りだったことが、常設検査（1.6）の走査語（`/` 区切り）に掛からなかった理由でもある。

### 無視規則と属性の始末

- `crates/pilot/examples/shiori-host-32/.gitignore` は**削除**した（要件 8.9）。指していた `fixtures/emo2/ghost/master/profile/` ごと消えたので残す理由が無い。
- `vendors/sample_ghost/.gitignore` の否定 2 行は**残した**。理由は**これから起きる作業**である——タスク 6.4 が案内する「検体を足す手順」は、畳む前の展開形をこのフォルダへ一時的に置くことを求める。その間、この 2 行が無ければ里々の辞書 `dic09_Test.txt` がリポジトリ直下の `.gitignore:6:*_test.txt` に当たって落ち、**辞書が 1 つ欠けた検体が黙って出来上がる**（`core.ignorecase=true` なので大小違いでも当たる）。理由書きをこの 1 点で書き直した。

  「要件 8.6 の判定（`git check-ignore --no-index`）がこの 2 行のおかげで正しく答えるから残す」という言い方は**採らない**。判定のために規則を残し、その規則について判定するのは循環であって何も証明しない。実測そのもの（判定の出どころが `vendors/sample_ghost/.gitignore` の `!*_test.txt` の行であり、同じ名前を否定の無い場所に置くとリポジトリ直下の規則に当たること）は正しいが、残す理由にはならない。展開形が消えた今、`.nar` の中身がディスクに現れるのは `target/` の下だけで、そこは丸ごと無視されている。
- 上の 2 ファイルは **LF** で書き直した。最初の書き替えで CRLF に転んでおり、`* -text` が変換を止めるこのフォルダでは**そのバイトがそのまま登録される**ため、5 行のコメント直しが全 16 行の入れ替えとして記録されるところだった（触っていない `* -text`・`!*_test.txt`・`!*_dump.txt` の行まで含めて）。バイトを揺らさないために在るファイルで揺らしては本末転倒である。`git show HEAD:… | tr -cd '\r' | wc -c` と作業コピーの双方で CR が 0 であること、`git diff` がコメント行だけを出すことを確かめた。
- `vendors/sample_ghost/.gitattributes` の `* -text` も**残した**。`*.nar binary` は足していない——`* -text` が既に `.nar` を覆っており（`git check-attr text` が `unset`）、足しても判定は変わらないためである。代わりに理由書きを「消える展開形の CRLF」から「書庫の中の生バイトが 1 バイトでも揺れると刻印による陳腐化の検出が成り立たない」へ書き改めた。

### 走らせた結果

| 何を | 結果 |
|------|------|
| `cargo test --workspace -j 4` | **7,932 passed / 0 failed / 40 ignored**。走った的は 110（`Running` 86 ＋ `Doc-tests` 24） |
| `cargo test -p log-capture-kit -j 4`（常設検査 1.6 を含む） | 8 的すべて緑（117 passed / 0 failed / 2 ignored） |
| `cargo build -p {areka,areka-emo-text,pilot,sample-ghost-kit} --examples` | 緑 |
| `cargo build -p pilot --example shiori-host-32-helper --target i686-pc-windows-msvc` | 緑 |
| `cargo fmt --all -- --check` | 緑 |
| `cargo clippy -p <触った crate> --all-targets` | `areka-emo-text` は**そもそも通らない**（下記） |

常設検査（1.6）の除外形 ⑴ は `Presence::Retired`＝「窓口の中にこの綴りが**無い**」を主張する形である。本タスクのコメント書き替えは窓口の外なので、この主張は動かない（緑のまま）。

### 走らせて**通らなかった**もの（いずれも本タスク以前からの既存事象）

- `cargo clippy -p areka-emo-text --all-targets` は**終了コード 101 で落ちる**。`deny` 水準の `absurd_extreme_comparisons` が 3 件（`tests/choice_fixture_test.rs:561`・`tests/emo2_fixture_e2e_test.rs:638`・`tests/line_pitch_readback_test.rs:705`）。3 件のうち 2 件は本タスクで 1 行も触っていないファイル、1 件は 8 行目のコメントだけを直したファイルなので、**本タスクの持ち込みではない**。他の 7 クレート（`areka`・`areka-parsers`・`areka-nar`・`shiori-host32-host`・`pilot`・`sample-ghost-kit`・`log-capture-kit`）はエラー 0・新規警告 0 である。
- `cargo test -p areka --bin areka spine -- --test-threads=1` は `STATUS_ACCESS_VIOLATION` で落ちる。spine の 3 ファイルを HEAD へ戻しても同じように落ちるので、**本タスクの持ち込みではない**。spine の判定は既定の並列走行（`-j 4`）で行っている。

### 速くなったか遅くなったか（起動ごとの取得の代償）

`boot_with` が起動ごとに木を複製するようになったので、spine の的は**ちょうど 2 倍**遅くなった。

| | 3 回走らせた中央値 |
|---|---|
| 共有の複製 1 つ（本タスク前） | **2.44 秒** |
| 起動ごとに複製（本タスク後） | **4.95 秒**（自分の実測 3.76／4.85／5.02 の中央値は 4.85 秒） |

複製の中身は 110 ファイル・6,614,383 バイト（6.6 MiB）で、spine の起動口は 23 か所ある。増えた 2.5 秒は、12 分前後かかるワークスペース全体の走行に対して測れるほどの重さではない。対価は「同じプロセスの 2 回目の起動で `OnFirstBoot` が黙って出なくなる」欠陥が構造的に起きえなくなることなので、この交換は引き受ける。速さが問題になったときに効く手は、複製を `emo2_boot` の 1 つに戻すことではなく（それが今回の欠陥そのものである）、起動を伴わない的が共有の複製を使い続けることである（既にそうしてある）。

### 5.1 の記述の期限切れ（追記・5.1 は書き換えない）

5.1 の「畳む手順」の囲みは 3 つの呼び方を並べているが、そのうち **2 つはもう通らない**。タスク 5.4 が `--from` を必須にしたためである。

| 5.1 の綴り | いま |
|------------|------|
| `cargo run -p sample-ghost-kit --example fold-samples` | **失敗する**。`--from <展開形のフォルダ> を書くこと（登記表は展開形の在処を持たない）` を返して終わる |
| `cargo run -p sample-ghost-kit --example fold-samples -- --check` | **失敗する**。同じ理由。`--check` は単独では使えず `--from` と併せる |
| `cargo run -p sample-ghost-kit --example fold-samples -- --from <フォルダ>` | 通る。**これが唯一の呼び方**である |

置き換わったのは `--from <フォルダ>` と `--from <フォルダ> --check`（畳み直さずに往復だけ確かめる）の 2 つで、実行体の doc がこの 2 行を正本として持っている。タスク 6.4 が `vendors/sample_ghost/README.md` に書くのもこの 2 行である。

### 残した懸念

- `crates/pilot/examples/shiori-host-32/README.md:26` は `fixtures/emo2/` を検証フィクスチャとして案内したままで、実体はもう無い。同ファイルの「fixture 取り込み済ゆえ nar 展開は不要」も同じく古い。**タスク 6.1 の担当**（`_Boundary:` が名指ししている）なので本タスクでは触らなかったが、6.1 が入るまでこの README は読み手を存在しない場所へ案内する。
- `design.md` の「見張り」の節は「`crates/sample-ghost-kit/src/` に ⑴〜⑷ の実体があること」と書いているが、常設検査の `EXCLUSION_FORMS` が実測で宣言しているのは ⑶ の 1 形だけである（⑴ は `Retired`、⑵⑷ は `ComposedAtRuntime`）。本タスクの後もこの食い違いは残る。

---

## 6.1 検体パスを綴るスクリプトと文書を窓口経由へ（2026-09-19）

対象要件: 10.5（得方は 1.9・`.nar` の名指しが許されることは 1.8）

### 何をしたか

| ファイル | 前 | 後 |
|----------|----|----|
| `tools/perf/invoke-followup-checks.ps1` | `$DEFAULT_GHOST_ROOT` が旧置き場を綴った固定パス・`-BalloonRoot` 省略時は `<GhostRoot>` の下へ名前を継ぎ足していた | `$SAMPLE_NAME` / `$SAMPLE_BALLOON_NAME` の 2 つの名前だけを持ち、根は窓口の `folder=` の行、バルーンは `balloon.<名前>=` の行から得る |
| `tools/perf/perf-loop.measure.ps1` | `$MEASURE_GHOST_ROOT_RELPATH` が旧置き場への相対パス・バルーンは同じく継ぎ足し | 同上（`$MEASURE_SAMPLE_NAME` / `$MEASURE_SAMPLE_BALLOON_NAME`） |
| `tools/perf/perf-loop.common.ps1` | — | 窓口を呼んで `key=value` を表にする `Get-NarSampleMap` と、鍵を 1 つ取り出す `Get-NarSamplePath` を新設 |
| `tools/perf/judge-perf.py` | コメントの出典が旧置き場の `surfaces.txt` | 検体 `vendors/sample_ghost/emo2.nar` の中の `shell/master/surfaces.txt`（展開した実物の道は窓口が教える） |
| `doc/emo2-conformance-scope.md` | 旧置き場の `menu.pasta` と辞書フォルダを 2 か所で名指し | 検体の中の道（`ghost/master/dic/…`）＋窓口の案内 |
| `crates/pilot/examples/shiori-host-32/README.md` | 「検証フィクスチャ: `fixtures/emo2/`」「fixture 取り込み済ゆえ nar 展開は不要」 | 検体は `vendors/sample_ghost/emo2.nar`・絶対パスは窓口のコマンドが `folder=` で教える・展開は窓口が行う |

`perf-loop.measure.ps1` は 991 行あって 1,000 行の上限まで 9 行しか残っていないので、共有する部品は
`perf-loop.common.ps1`（`perf-loop.ps1` が先に dot-source する）へ置いた。
`invoke-followup-checks.ps1` は common を読み込まない独立した入口（自前の `Stop-Run` を持ち、
署名が違う）なので、同じ 2 関数を自前に持たせ、互いを指す注記を両方に入れた——これは同ディレクトリの
`Get-PythonCommand` が既に取っている形と同じである。

### `folder=` を読むときの 3 つの罠

| 罠 | 本タスクへの当たり | どう扱ったか |
|----|--------------------|--------------|
| ⑴ 同梱バルーンを持たない検体は `balloon.` の行が**そもそも出ない**（空の値ではなく行が無い） | 当たる。既定の検体 `emo2` は出るが、別の検体を渡す将来の呼び手が黙って空のパスを掴む | `Get-NarSamplePath` が鍵の不在を見て `Stop-Run` する。実測で `R_POST_and_KOMAINU` に `balloon.emo2-kakukaku` を尋ね、両方の写しが理由付きで止まることを確かめた |
| ⑵ バルーンの `folder=` は `ghost` の下ではなく `<根>/balloon/<名前>` に在る | 当たる。旧来の「`<GhostRoot>` に名前を継ぎ足す」は新しい配置ではもう成り立たない | 継ぎ足しをやめ、`balloon.<名前>=` の行をそのまま使う形にした |
| ⑶ `=` は**最初の 1 つだけ**で割る | 当たらない（Windows の絶対パスに `=` は現れない）が、割り方を誤ると値が黙って切れる | `IndexOf('=')` ＋ `Substring` で最初の 1 つだけを区切りにした（`-split '='` は使っていない） |

### 窓口が失敗したときに何が起きるか（黙って進まないこと）

窓口は未登録の名前に対して標準エラーへ理由を書き、**終了コード 2** で終わり、標準出力には 1 行も出さない。
両方の写しは終了コードを見て止まる。実測（4 通り・いずれも子プロセスで起こして終了コードを確かめた）:

| 呼び | 結果 |
|------|------|
| `emo2` の `folder` | 絶対パスを得て、そのフォルダが実在することを確認（`EXISTS=True`） |
| `emo2` の `balloon.emo2-kakukaku` | 同上（`EXISTS=True`） |
| `R_POST_and_KOMAINU` の `balloon.emo2-kakukaku` | 「出力に `'balloon.emo2-kakukaku='` の行がありません（出た鍵: folder, root）」で停止。`perf-loop` 側は `code=4`、`invoke-followup-checks` 側は `code=1` と `FOLLOWUP RESULT overall=INCONCLUSIVE` を出す |
| 未登録の `no-such-sample` | 「終了コード 2」と、窓口が出した既知の名前の一覧を添えて停止 |

窓口は呼ぶたびに検体を展開し直すので、両方の写しは**検体 1 つにつき 1 回だけ**呼んで表を取っておく
（`Get-MeasureGhostRoot` / `Get-MeasureBalloonRoot` は 1 回の走行で 4 回呼ばれる）。

### 旧置き場の綴りの数え（較正つき）

数える場所は要件 10.5 のとおり `tools/` と、タスク 6.3 が持つ `doc/ukadoc-coverage/` を除いた `doc/`。
`/` と `\` の両方の綴りを拾うため、区切りは `.`（任意の 1 文字）で書いた。

| 検索語 | 変更前 tools/ | 変更前 doc/（6.3 を除く） | **変更後 tools/** | **変更後 doc/（6.3 を除く）** | 対照: doc/ukadoc-coverage/（6.3 の担当・触っていない） |
|--------|---------------|---------------------------|-------------------|-------------------------------|------------------------------------------------------|
| `shiori-host-32.fixtures` | 3 | 1 | **0** | **0** | 2 |
| `fixtures.emo2` | 3 | 1 | **0** | **0** | 3 |
| `sample_ghost.(検体名)[^.]`（展開形） | 0 | 0 | **0** | **0** | 0 |

境界の `crates/pilot/examples/shiori-host-32/README.md` も 3 語すべて **0**（変更前は `fixtures.emo2` が 1）。

**0 が本物であることの確かめ方**——数えた 0 が「検索語が壊れていたから 0」ではないことを 2 通りで見た。

1. 同じ検索語が**変更前に実物を見つけている**（上の表の左 2 列。合計 8 か所が挙がり、その全部が本タスクと 6.3 の担当ファイルだった）。
2. 同じ検索語を**いま当てても 6.3 の担当ファイルでは当たる**（右端の列が 2 と 3）。検索語が死んでいれば、ここも 0 になる。
3. 3 つ目の検索語（展開形）は変更前も 0 だったので、上の 2 つが効かない。そこで旧綴りを 2 通り（`/` 区切りと `\` 区切り）と、**許されている** `.nar` の綴り 1 通りを並べた合成の入力に当て、前の 2 行だけが挙がり `.nar` の行は挙がらないことを確かめた。`.nar` を名指しすることは要件 1.8 が明示的に許しているので、これを赤にしてはならない。

変更後に `tools/`・`doc/`・境界の README に残る `sample_ghost` の綴りは 3 か所で、いずれも
`vendors/sample_ghost/emo2.nar`（配布形のファイルそのもの）を指している。

### 走らせて確かめたこと

| 確かめ | 結果 |
|--------|------|
| 4 本の `.ps1` の構文（`Parser::ParseFile`） | `invoke-followup-checks.ps1`・`perf-loop.common.ps1`・`perf-loop.measure.ps1`・`perf-loop.ps1` すべて構文誤り 0 |
| `judge-perf.py` の構文（`python -m py_compile`） | 通る |
| 道具の自己較正 `perf-loop.ps1 selftest` | `judge-perf.py --selftest` 合格・`invoke-followup-checks.ps1 -SelfTest` は `ok=4 ng=0`。全体は `ok=8 ng=1` で、赤の 1 件は下の「残した懸念」に書いた `check-quiet.ps1`（本タスクで触っていないファイル） |
| 窓口の解決（上の 4 通り） | 上表のとおり |

本タスクで**確かめられなかったこと**: 性能計測の本走行（`measure-baseline` / `rank-run` / `followup`）は
1 本 25 分前後かかり、静寂な機械と 32bit ヘルパと管理者権限を要求するので回していない。確かめたのは
「既定の根の得方」の段までである。本走行の中で根が使われる先（`invoke-perf-run.ps1` への `-GhostRoot` /
`-BalloonRoot` の引き渡し）は、`Get-MeasureGhostRoot` / `Get-MeasureBalloonRoot` の戻り値をそのまま渡す
配線で、本タスクはその戻り値の作り方だけを変えている。

### 残した懸念（いずれも本タスクの `_Boundary:_` の外）

- **`tools/perf/check-quiet.ps1` の自己較正が 1 件赤い**（「在るだけで落とす名前 0 件が『-』になりません」）。
  このファイルは HEAD から 1 文字も変わっていない（`git diff --quiet HEAD` が 0）ので、本タスクより前から赤い。
  `perf-loop.ps1 selftest` は 1 件でも赤ければ `code=4` で止まるため、性能計測を回す前に誰かが直す必要がある。
- **`tools/perf/invoke-perf-run.ps1` の `-BalloonRoot` 省略時の既定が古い**。この走者は `<GhostRoot>` に
  `emo2-kakukaku` を継ぎ足す（`:540`）が、新しい配置ではバルーンはゴーストの下に無い。要件 10.5 が名指しする
  3 本に入っておらず、旧置き場の綴りも持たないので数えには出ないが、`tools/perf/README.md:123` もこの継ぎ足しを
  案内している。ただし**黙っては壊れない**——`:546` が「バルーンのルートのフォルダが存在しません」で止まる。
  なお `perf-loop.measure.ps1` はこの走者を呼ぶとき必ず両方を明示して渡すので、計測ループの中では通らない道である。

### 6.1 差し戻し後の是正: `invoke-perf-run.ps1` の既定を直さずに消した（追記・上の 6.1 は書き換えない）

差し戻しの指摘は 1 点だった——`_Boundary:_` の `tools/perf` は**ディレクトリ全体**であり、
design が名指しする 3 本だけではない。`tools/perf/invoke-perf-run.ps1` は
`-BalloonRoot` 省略時に `<GhostRoot>` へ `emo2-kakukaku` を継ぎ足す既定を持ったままで、
段 ③ の配置ではこれは**必ず存在しない場所**を指す。上の節は「`:546` が止めるので黙っては
壊れない」と書いて先送りしたが、要件 10 の目的は 「検体パスを綴っていた**文書やスクリプトが
壊れていないでほしい**」 であり、`tools/perf/README.md:123-124` の 「省略すると
`<GhostRoot>\emo2-kakukaku` を補います」 は**いま壊れている文書**である。この spec に
`tools/perf` を持つ後続タスクは無く、先送り先が存在しなかった。

**直し方は「既定を新しい場所へ差し替える」ではなく「既定を消す」**。`perf-loop.measure.ps1` は
この走者を呼ぶとき必ず `-BalloonRoot` を明示して渡す（`:254`）ので、既定に依存する呼び手は
1 つも無い。消せば窓口を呼ぶ 3 つ目の写しも要らない。

| 場所 | 前 | 後 |
|------|----|----|
| `invoke-perf-run.ps1:539-541` | 省略時に `<GhostRoot>` へ継ぎ足していた | 省略を `Stop-Run $EXIT_BAD_ARGS` にし、`-GhostRoot` の未指定検査（`:523`）と同じ形に揃えた。文言に窓口の得方（`balloon.emo2-kakukaku=` の行）を書いた |
| `:41`（較正値一覧）・`:175`（定義）・`:681`（実行条件の記録） | `DEFAULT_BALLOON_SUBDIR` を持ち `run-meta.txt` にも書き出していた | 3 か所とも削除。`tools/` 全域に綴りは 0 件 |
| `:546`（エラー文言） | 「省略時は `<GhostRoot>\…` を補います」 | 補う話を削除 |
| `:77`（使い方）・`:127`（引数の注記）・`:79`（`-DryRun` の例） | `[-BalloonRoot …]` と旧入れ子を見せていた | 省略できない引数として見せ、値は窓口から得る形に改めた |
| `tools/perf/README.md:123-124` | 「省略すると `<GhostRoot>\emo2-kakukaku` を補います」 | 「省略できません」＋窓口の `folder=` と `balloon.<名前>=` から得る案内に差し替え |
| `tools/perf/README.md:29-38`・`:805`（2 つの呼び出し例） | `-GhostRoot C:\絶対パス\emo2` だけで `-BalloonRoot` が無く、そのまま打つと止まる | 窓口を呼ぶ 1 行を先頭に足し、2 つの値を渡す形にした（早わかりの丸数字も繰り下げ） |

### 走らせて見つけた欠陥（報告だけで済ませなかったので出た）

最初に書いた停止の文言を実際に出させたところ、`balloon` が **`alloon`** と表示された。
PowerShell の二重引用符の中ではバッククォートが逃がし記号で、``` `b ``` が後退空白として食われていた
（``` `cargo ``` 側もバッククォートが消えていた）。**逐語で見せる文は単引用符で書く**形に直し、
その理由をその行の注記に残した。文言を目で読んだだけでは分からない種類の欠陥である。

### 確かめたこと

| 確かめ | 結果 |
|--------|------|
| `invoke-perf-run.ps1` の構文（`Parser::ParseFile`） | 誤り 0 |
| ⒜ `-BalloonRoot` を省略して起こす（`-GhostRoot` は窓口から得た実在の根・`-DryRun`） | `exit 3`（`EXIT_BAD_ARGS`）。文言は欠けずに全部出る |
| ⒝ 窓口から得た `folder=` と `balloon.emo2-kakukaku=` の 2 つを渡して起こす（`-DryRun`） | `exit 0`。ゴースト・バルーン・出力先の 3 行が窓口の値そのままで出て、前提の検証を通過した |
| ⒝ が書いた `run-meta.txt` | 63 行・`DEFAULT_BALLOON_SUBDIR` の行は **0 件**・`ghost_root` と `balloon_root` は窓口の値と一致。この鍵を読む道具は `tools/` に 1 つも無いことも確かめた |
| `perf-loop.ps1 selftest` の再走 | `ok=8 ng=1`。赤は**是正前と同じ `check-quiet.ps1` の 1 件だけ**（本タスクで触っていない・HEAD から不変） |
| 旧置き場の綴りの数え直し | 3 つの検索語すべて tools/ **0**・doc/（6.3 を除く）**0**・境界の README **0**。対照の `doc/ukadoc-coverage/` は 2・3・0 で変わらず（検索語が生きている証拠） |

### 1,000 行の番人は `.ps1` を見ていない（記録・行動はしない）

`perf-loop.measure.ps1` が 991 行だったため共有部品を `perf-loop.common.ps1` へ置いたが、
**この上限は `.ps1` には強制されていない**。番人の列挙は
`crates/log-capture-kit/tests/workspace_scan/mod.rs` の `collect_rs_files` が
`name.ends_with(".rs")` で絞っており、`.ps1` は最初から集められない。つまり 993 行の
`perf-loop.measure.ps1` が 1,000 行を越えても**赤にはならない**。置き場の判断は規律で
保たれているだけなので、ここに書き残す。

### 6.1 2 度目の差し戻し: 幅を決め打ちしたワイルドカードが作った 3 つ目の偽の 0（追記・上の 2 節は書き換えない）

指摘は 1 点、直しは 2 行だった——`tools/perf/perf-loop.ps1:146` と `:149` の引数の注記が、
既に消した既定を現役として説明したままだった。

| 行 | 前 | 後 |
|----|----|----|
| `:146` | 「ゴースト一式のルート（絶対パス。省略時は **emo2 fixture**）」 | 「…省略時は検体の窓口が教える emo2 の根」 |
| `:149` | 「バルーンのルート（絶対パス。省略時は **`<GhostRoot>\emo2-kakukaku`**）」 | 「…省略時は検体の窓口が教える emo2 同梱バルーンの場所」（`invoke-followup-checks.ps1:84` と同じ文言に揃えた） |

**振る舞いは元から正しかった**——`-BalloonRoot` を省くと `$script:BalloonRootArg`（`:240`）が
空のままになり、`Get-MeasureBalloonRoot`（`perf-loop.measure.ps1:133-135`）が窓口の
`balloon.emo2-kakukaku=` を読む。誤っていたのは注記の散文だけである。同じ文はもう 2 か所にあり、
1 つは 1 度目で直し（`invoke-followup-checks.ps1:84`）、1 つは 2 度目で消した
（`invoke-perf-run.ps1`）。**3 つのうち 2 つを直して 1 つを取り落とした。**

### なぜ取り落としたか——幅を決め打ちしたワイルドカードは偽の 0 を作る

2 度目の報告で「旧入れ子の綴りは 0 件」と結論した根拠は検索語 `GhostRoot.emo2-kakukaku` だったが、
実物の綴りは `<GhostRoot>\emo2-kakukaku` で、間に **2 文字**（`>` と `\`）ある。`.` は 1 文字ぶんしか
合わないので、**在るのに 0 と出た**。区切りを文字クラス `[^ ]*` で書けば当たる。

較正は**対で**採った。当てる先は合成の文字列ではなく **git が持つ修正前の実物**にした
（この節を書く途中、合成の入力を `printf` で作ろうとして `\e` が逃がし記号として ESC に化け、
**較正そのものが空振りした**。道具で作った検体は道具の逃がし規則に汚染される）。

| 検索語 | HEAD の `perf-loop.ps1`（修正前＝当たるべき） | 作業ツリー（修正後＝0 であるべき） |
|--------|---------------------------------------------|------------------------------------|
| `GhostRoot[^ ]*emo2-kakukaku`（文字クラス） | **1 件**（`:149`） | **0 件**（`tools/`・`doc/`・境界の README のいずれも） |
| `GhostRoot.emo2-kakukaku`（幅 1 の `.`） | **0 件** ← 偽の 0 の再現 | 0 件（意味を持たない） |
| `emo2 fixture` | **1 件**（`:146`） | **0 件**（`tools/` 全域） |

左の列が当たっているので、右の列の 0 は「検索語が死んでいるから 0」ではない。

**この種別を閉じる**ため、同じ言い回しを全部出して目視した。`grep -rn "省略時は" tools/perf/*.ps1`
は 8 行返し、うち 3 行が `perf-loop.ps1`（`:142` 実行体の所在・`:146`・`:149`）。残り 5 行は
記号キャッシュ・`target\debug`・出力先・ビルド種別・`invoke-followup-checks.ps1:84`（1 度目で直した行）で、
いずれも検体の在処と関係が無い。是正後の 8 行はすべて現状と一致する。

### この spec に持ち越す教訓

本タスク 1 つで**偽の 0 が 3 回**出た。⑴ ハーネスのシェルが検索語のバックスラッシュを落とした、
⑵ レビュー側の `git grep` がリポジトリ全域で 0 を返した、⑶ 幅を決め打ちした `.` が 2 文字の区切りを
跨げなかった。**数えて 0 を主張する前に、同じ検索語が既知の当たりを拾うことを必ず見せる。**
当たりの供給元は、合成した文字列より **git の履歴に在る実物**が安全である。

### 確かめたこと

| 確かめ | 結果 |
|--------|------|
| `.ps1` 5 本の構文（`Parser::ParseFile`） | 誤り 0（`invoke-followup-checks` / `perf-loop.common` / `perf-loop.measure` / `perf-loop` / `invoke-perf-run`） |
| `perf-loop.ps1 selftest` の再走 | `ok=8 ng=1`。赤は 3 度とも同じ `check-quiet.ps1` 1 件（本タスクで触っていない） |
| 数え直し | 上表のとおり。較正の対つき |
| ワークスペース走行 | **回していない**。本是正は注記 2 行のみで実行経路に触れていない（直前の走行は 111 スイート・7,935 passed・0 failed で緑） |

### 2 度目の報告の数の訂正

`invoke-perf-run.ps1` の差分を「削除 12・追加 8」と書いたが、正しくは **10 追加 / 10 削除**
（行数は 887 のまま増減なし）。結論には影響しない。

## 6.2 steering の 3 本と謝辞の追随（2026-09-19）

対象要件: 10.1（謝辞の再生成と差分）・10.4（構成の登記）・10.9（実機運転の定石）

### 編集の前に数えたこと（較正）

| 調べ | 結果 |
|------|------|
| `roadmap.md` の `zip` の綴り（文字クラス走査） | 7 か所。うち**依存を指すのは 3 か所**＝制約の節（外部依存の追加）・仮裁定 5・A0 のウェーブ行①。残る 4 か所（利用者の一周・完成の器・#17 の行・A5 の行）は**配布物の zip** で対象外 |
| 同じ綴りの 4 か所目 | `roadmap.md` の 2026-09-12 追記(96) が `zip 8.6` を記録している。日付つきの記録なので**書き換えず、決着の一文を足すだけ**にした |
| `log-capture-kit` の見張りの節 | 3 本（`with_default_guard_test.rs`・`file_length_guard_test.rs`・`temp_path_guard_test.rs`）と書かれていた。実ファイルは 4 本（`sample_path_guard_test.rs` がタスク 1.6 で増えている）＝**節が 1 本遅れていた** |
| 謝辞の事前の状態 | `miniz_oxide 0.8.9` が 2 か所、`adler2 2.0.1` が 1 か所**既に載っていた**。MIT 209 件 |

### 謝辞をどう再生成したか

正典の手順は `about.toml` の先頭と `kiro-complete` の DoD が綴る
`cargo about generate --workspace about.hbs -o THIRD-PARTY-NOTICES.md`（cargo-about 0.9.2）。
`about.toml` は**ターゲットを指定していない**ので、cargo-about は Windows 以外の条件つき依存も含めて歩く。

- 終了コード 0。**2 度走らせて md5 が一致**（`4ac1fa5971e64eda7e4acc94c2deb22e`）＝差分は並び替えの揺らぎではない。
- `Cargo.lock`（追跡外）の md5 は再生成の前後で不変（`55f711a5b7177c39fd8016433a3c9031`）＝`cargo update` は起きていない。

### 差分（増えた項目だけ・並び替えや書式の揺れは 0）

| 増えた項目 | 何か |
|------------|------|
| `miniz_oxide 0.9.1`（2 か所） | 本タスクの依存。MIT の本文が 2 種類あるため MIT の節が 2 つに分かれており、その両方に付く（`miniz_oxide 0.8.9` も同じ 2 か所に居る） |
| `areka-nar 0.0.1` | 本仕様が新設した**自分のクレート**（第三者ではない。ワークスペースの crate も MIT として載る慣行） |
| `sample-ghost-kit 0.0.1` | 同上 |

見出しの数は MIT が 209 → 213（＝上の 4 行）。他のライセンスの件数は不変。

**`adler2 2.0.1` は増えていない——既に載っていたからである。** 出どころは
`human-panic → backtrace → miniz_oxide 0.8.9 → adler2` で、この経路は
`cfg(not(all(windows, target_env = "msvc", …)))` の下にあり Windows では通らないが、
`about.toml` がターゲットを絞っていないので謝辞は以前からこれを含んでいた。
したがって**「差分は 2 項目」ではなく、第三者の増分は `miniz_oxide 0.9.1` の 1 件**（表れは 2 行）で、
残り 2 行は自分のクレートである。要件 10.1 が求める「増える項目が追加した依存とその推移的依存に限られる」は満たす。

### 本番依存の数え直し（ターゲットごと・`--target all` は使わない）

`cargo tree -e normal --target <t> --workspace --exclude sample-ghost-kit --exclude log-capture-kit --exclude temp-path-kit`

| ターゲット | 本番グラフの crate 数 | `miniz_oxide 0.9.1` / `adler2 2.0.1` |
|------------|----------------------|--------------------------------------|
| `x86_64-pc-windows-msvc` | 171 | 両方あり |
| `i686-pc-windows-msvc` | 171 | 両方あり |
| `aarch64-pc-windows-msvc` | 166 | 両方あり |

タスク 2.1 の確定値（x86_64 171／i686 171／aarch64 166）から動いていない。

### steering に入れた記述

| ファイル | 追加・訂正 |
|----------|------------|
| `tech.md` | Key Libraries に `miniz_oxide` (0.9) を `encoding_rs` と同じ書式で登記（意図的依存追加＝2026-09-18 承認済・伸長のみ・`with-alloc` だけ・圧縮側を綴らないことを `areka-nar/src/lib_tests.rs` が見張る・推移的依存は `adler2` 1 本） |
| `structure.md` | クレート一覧に `areka-nar`（本番）と `sample-ghost-kit`（テスト専用 leaf・窓口の置き場と bin `nar-sample-path`）の 2 節。見張りの節を 3 本 → **4 本**（`sample_path_guard_test.rs`） |
| `roadmap.md` | 実機運転の定石に**検体の絶対パスの得方**を 1 行（`cargo run -p sample-ghost-kit --bin nar-sample-path -- emo2`。`manual/<検体>/` を呼ぶたびに作り直すので**2 つの端末で同時に呼ぶと互いの木を消す**ことも書いた）。`zip` を承認待ちの依存と綴る 3 か所を `miniz_oxide`（承認済）へ。追記(96) には決着の一文を追加 |

3 本とも先頭の `updated_at` を 2026-09-19 にした。

### 確かめたこと

| 確かめ | 結果 |
|--------|------|
| `cargo deny check` | `advisories ok, bans ok, licenses ok, sources ok`（終了コード 0。ワイルドカードの警告は既存のまま） |
| `cargo test --workspace -j 4` | **111 スイート・7,935 passed・0 failed**（終了コード 0） |
| `Cargo.lock` の md5 | 再生成の前後で不変 |
| `roadmap.md` に残る `zip` | **7 行**。L20・L22・L103・L153 は**配布物の zip**、L132・L148 は本タスクが直した「`zip` を採らなかった」の意味、L195 は日付つきの記録。**`zip` を承認待ちの依存として綴る箇所は 0**（これが判定すべき主張である）。⚠ 行単位で `zip` と `承認待ち` を突き合わせると L132 が 1 件当たるが、その `承認待ち` は同じ行の `md-5` に掛かっており、`zip` の側は「採らず `miniz_oxide` で決着（承認済）」と読む |

### 直さなかったもの

`roadmap-history.md` の追記(96) 全文と W14／単独枠のウェーブ行は `zip 8.6` と
`vendors/sample_ghost/R_POST_and_KOMAINU/`（展開形）を綴ったままである。日付つきの記録の書庫なので
書き換えない。

現役の steering 3 本の展開形の綴りは、文字クラス `sample_ghost[/\\][A-Za-z0-9_.-]+[/\\]` で数えた。
**HEAD では 1 件当たる（`roadmap.md:148` の A0 ウェーブ行③）＝較正済み**で、この 1 件は
「保管は展開フォルダ（`vendors/sample_ghost/StayseeBalloon/`＝現行慣行）」と読め、**タスク 5.6 が廃した形を
A0-③ `default-balloon-bundle` に指示したままだった**。本タスクで配布形（`StayseeBalloon.nar`）へ直した。
是正後の現役 steering 3 本は **0 件**。

なお ③ の一文が指す `vendors/sample_ghost/` の README は**タスク 6.4 が作る**（要件 10.10）。
6.4 の着地後に真になる前方参照であり、6.4 までは指す先が存在しない。

### 1 度目の報告の訂正

⑴ 「現役の steering 3 本には展開形の検体パスの綴りは 0 件」と書いたが、**実際は 1 件あった**（上記。
走査語も較正も示さない裸の 0 で、本仕様で 5 度目の偽の 0 である）。
⑵ 「設計が圧縮側の見張りを `sample_path_guard_test.rs` と取り違えている」と書いたのは**誤りなので取り下げる**。
設計 `design.md:797` は `no_deflate_side_is_called` を「Unit Tests（`areka-nar/src/*_tests.rs`…）」の下に置き、
`:840` は「`areka-nar/src/*.rs` が `miniz_oxide::deflate` を綴らない」と書いている。実体の
`crates/areka-nar/src/lib_tests.rs:338` はその `areka-nar/src/*_tests.rs` に当たる＝**設計は正しい**。
⑶ `about.toml` が `targets` を持たないことは**受容された恒久の姿勢**であり、先送りではない。
本番依存グラフの数え（タスク 2.1）では過剰計上が事実誤りになるが、ライセンス表示は逆で、
**載せ過ぎは無害・載せ落としが法務上の危険**である。`about.toml` の冒頭と `deny.toml` が
同じ理由で全ターゲットを評価する旨を既に綴っている。
