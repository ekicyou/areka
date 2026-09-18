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
