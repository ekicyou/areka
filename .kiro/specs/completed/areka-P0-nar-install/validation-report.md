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

## 6.3 網羅台帳の担当欄の登記と関連文書の追随（2026-09-19）

対象要件: 10.5（旧置き場の綴りの 0 件）・10.6（`owner` 11 件と対象外 4 件の理由）・10.7（`roadmap-draft.md` の同時編集・報告の作り直し）

### 先に赤を出した（登記と表の追随を分けられないことの実証）

台帳へ `owner` を書いた時点で、表を直す前に `cargo test -p ukadoc-survey` を回した。**2 本赤くなった。**

```
判定 ⑸ が赤い（1 件）:
doc/ukadoc-coverage/ledger/assets.toml: ukadoc:descript_install:_2a.directory_…:1 の宛先
areka-P0-nar-install が doc/ukadoc-coverage/roadmap-draft.md の [[spec]] にも
doc/ukadoc-coverage/briefing.md の [[owner_completed]] にも無い（要件 7.4 ⑶）
```

赤くなったのは `spec_checks::the_spec_table_agrees_with_the_directories_the_ledgers_and_the_bundles` と
`spec_checks::rewriting_one_owner_turns_the_count_and_destination_arms_red` の 2 本である。
要件 10.7 が「`owner` 登記と分けない」と書いている根拠が、そのまま目に見える形で出た。
基準の緑（着手前）は 601 / 0 / 6 / 113 / 5 ＝ **725 passed・0 failed**。

### 台帳 `ledger/assets.toml`

`descript_install` は **15 項目**あり、内訳は `install.txt` のキー **11 項目**と、`install.txt` 以外の
**4 見出し**である。設計の「11 項目」「4 見出し」は実データと一致した。

| 登記 | 項目 |
|------|------|
| `owner = "areka-P0-nar-install"`（**11 件**） | `type,種別`・`name,オブジェクト名`・`directory,ディレクトリ名`・`charset,文字コード`・`accept,本体側名`・`refresh,数値`・`refreshundeletemask,ファイル名1:…`・`*.directory,…`・`*.source.directory,…`・`*.refresh,数値`・`*.refreshundeletemask,…` |
| `owner = ""` のまま（**4 件**） | `bootghost,ディレクトリ名`・`相対パス`・`相対パス,オプション1,オプション2,...`・`相対パス,ignore` |

空のまま残した 4 件に書いた理由は次のとおりである（全文は台帳の `note`）。

| 見出し | 理由 | 引受先 |
|--------|------|--------|
| `bootghost,ディレクトリ名` | `type,package`（複数の配布物を 1 つに束ねた形）専用の欄である。本仕様は種別を `ghost`・`shell`・`balloon`・`supplement` の 4 つに限り、`package` を受け付けない種別として明示的に拒否する | 束ねた形を解く仕様。**まだ起票されていない** |
| `相対パス` | `install.txt` ではなく `delete.txt`（更新のときに消す道の指示）の行の書式。本仕様は書庫を開いて入れるところまでしか受け持たない | `linkage.md` の段階 B「更新」の束（候補名 `areka-P0-network-update`）。まだ起票されていない |
| `相対パス,オプション1,オプション2,...` | `install.txt` ではなく `developer_options.txt`（配布物を**作る側**が置く開発者向けの指示）の行の書式。読む相手はベースウェアではない | areka が作る側の道具を持つときの仕様 |
| `相対パス,ignore` | 同上 | 同上 |

### ⚠ 状態（`status`）は `absent` のまま動かしていない

要件 10.6 は「着地後に状態を実測へ更新する」とも書いているが、本タスクでは**動かせなかった**。理由を 2 つ並べる。

1. **実装済みを名乗るには証拠が要る。** 検査 `ImplementedWithoutEvidence` は、`implemented` の項目に
   定義箇所の 1 行コメント（`// ukadoc: <URL>`）が 1 件も無いと所見を出し、常設の
   `checks::real_repo_data_produces_no_findings` が所見 0 件を主張しているので、その場で赤になる。
   `crates/areka-nar/src/` に `ukadoc:` の綴りは**現在 0 行**である（数え方: `grep -rn "ukadoc:" crates/areka-nar/src/`）。
2. **その 1 行を置く先が本タスクの担当範囲の外にある。** 6.3 の `_Boundary:_` は `doc/ukadoc-coverage` で、
   `crates/` は含まない。

事実としては、本仕様の `areka-nar` が書庫最上位の `install.txt` を読んで 11 項目すべてを解釈している
（`crates/areka-nar/src/manifest.rs` の許可キーの表とマニフェストの組み立て）。ただし `areka-nar` を
`[dependencies]` に持つのは `sample-ghost-kit` の 1 本だけで、製品の起動経路からは呼ばれない。
**この食い違いを黙らせないため、11 件の `note` に「状態は `absent` のまま」「実装済みを名乗るには
定義箇所の正典 URL が要る」「その置き先は担当範囲の外」を書いた。** 残作業は
「`crates/areka-nar/src/manifest.rs` に 11 行の正典 URL を置き、同じコミットで台帳の 11 件を
`implemented` へ動かし、報告 2 本を作り直す」の 1 組である。

### `roadmap-draft.md`

| 場所 | 前 | 後 |
|------|----|----|
| `[[spec]]`（新設） | — | `name = "areka-P0-nar-install"` / `stage = "B"` / `bundle = "インストール"` / `owner_count = 11` / `wave = "A0"` |
| `[briefs].count` | 27 | **28** |
| 「数え方」の直後（新設の段落） | — | `[briefs].count` が数えるのは置き場のディレクトリ数ではなく**表の行数**であること、2026-09-13 はたまたま両方 27 だったが 09-19 に 28 で分かれたこと |
| 「いま置き場にあるが表に無いもの **2 本**」 | `areka-P0-nar-install`・`areka-P0-shell-implicit-surface` | **1 本**（`areka-P0-shell-implicit-surface` のみ）＋行が増えた経緯への案内 |
| 「行数が 27 のまま合っているのは…」 | 出入りの数が同じという説明 | 削除し、代わりに「2026-09-19 の追加」の段落を置いた（足さざるを得なかった理由＝要件 7.4 ⑶ の見張り） |
| 「意図しない重なり 1 行」 | 「（依存する既存 spec **0 本**）」 | **0 本 → `areka-P0-nar-install`（A0・11 件）**。ただし 11 件は束の構成 **40 件**の過半に満たないので規則 ⑴ にも ⑵ にも当たらず、**重なりは解消していない**ことを明記した |
| 段階 B 順位 1「インストール」の行 | 依存する既存 spec `**0 本**` | `areka-P0-nar-install`（A0・11 件）。候補名の欄にも「裁定待ち」を添えた |
| 「読み方」の「0 本の束は **36** である」 | 36 | **35**（表の行のうち `**0 本**` を持つものを数え直した。変更前 36・変更後 35） |

「意図しないものは 1 行」という冒頭の数は**そのまま 1 行**にした。束の構成 40 件のうち本仕様が持つのは
11 件で過半に届かず、規則 ⑶（新しい名前）のままだからである。ここを「0 行」に書き換えかけて、
`linkage.md` の `[bundle."インストール"]` を数え直して取り消した（`members` **40 件**・うち
`descript_install` **15 件**）。

### `briefing-assets.md` の 3 か所

| 行（変更前） | 何を直したか |
|--------------|--------------|
| `:362` `OnUpdate` の行 | 「`crates/` 全体まで広げると 5 行」は展開形が消えた結果**いま 0 行**になった（数え直した）。辞書は `vendors/sample_ghost/emo2.nar` の中の `ghost/master/dic/update.pasta` に在り、絶対パスは窓口が教える、と書き換えた |
| `:377` 7 本の表の前書きと表の「置き場」の列 | 「`fixtures/` から下の道」を「検体と、その中の道」に変え、7 本の内訳を `emo2.nar` 5 本・`emo2-kakukaku-offsetdpi.nar` 1 本・`emo2-kakukaku-wplimit.nar` 1 本と実測して書いた（`zipfile` で全名を列挙）。**7 本という数は変わらない** |
| `:439` ④ 起動の実例 | 旧置き場を綴っていた 1 か所を「試験用ゴースト `emo2`（配布形は `vendors/sample_ghost/emo2.nar`・絶対パスは窓口が教える）」に置き換えた |

あわせて、5 本目の検体 `R_POST_and_KOMAINU.nar` にも `install.txt` が 1 本あるが、これは
試験用ゴーストの外なので 7 本には数えていないことを明記した（変更前の文書はこの検体に触れていない）。

**この節の数はもう現状ではない**旨の日付つきの断りを「測り直した areka の現状」の冒頭に足した。
本仕様の着地で「`.nar` の綴り 0 行」「`install.txt` の綴り 3 行」「書庫を開く依存 0 件」と、
6 段のうち ② 展開・③ 配置の記述が着地前の写真になっている。**測り直しは本タスクの範囲外**なので、
引受先（網羅台帳の統合担当 `ukadoc-coverage-roadmap`）を名指しして断りに書いた。

### 報告の作り直し

`cargo run -p ukadoc-survey -- report` と `-- report-summary` を走らせた。5 本とも書き出されたが、
**中身は 1 バイトも変わらなかった**（`git diff` が空。作業ツリー上で改行が CRLF から LF になっただけで、
git の正規化を通すと差分 0）。報告は状態・世代・束から組み立てられ、`owner` の欄を載せないためである。
生成物を CRLF に戻したうえで `DomainReportStale` を見る常設の検査を回し直し、緑を確かめた。

### 旧置き場の綴りの数え（較正つき）

較正の当たりは**合成した文字列ではなく `git show HEAD:<パス>` が返す実物**から取り、区切りは
文字クラス `[/\]` で書いた（幅 1 の `.` は使っていない）。

| 検索語 | 較正: HEAD の `briefing-assets.md`（当たるべき） | いま `doc/` 全域 | いま `tools/` 全域 |
|--------|--------------------------------------------------|------------------|--------------------|
| `shiori-host-32[/\]fixtures` | **2 件** | **0 件** | **0 件** |
| `fixtures[/\]emo2` | **3 件** | **0 件** | **0 件** |
| `sample_ghost[/\][A-Za-z0-9_.-]*[/\]`（展開形） | 0 件（変更前も 0） | **0 件** | **0 件** |

左の列が実物を拾っているので、右 2 列の 0 は「検索語が死んでいるから 0」ではない。3 つ目の検索語だけは
較正の当たりが取れない（変更前から 0）ので、**タスク 6.1 が同じ検索語を合成入力で較正済み**である
（旧綴り 2 通りが挙がり、許されている `.nar` の綴りは挙がらないことを確認済み）。

**⚠ 対照の列が消えた。** タスク 6.1 と 6.2 は「`doc/ukadoc-coverage/` では当たる（2 件・3 件）」を
検索語が生きている証拠＝対照列として使っていた。本タスクがその 3 行を直したので、**その対照列は
もう使えない**。以後この 3 語で 0 を主張する人は、`git show HEAD~<n>:…` で履歴から当たりを取ること。

**自分で作りかけた偽の 0 が 1 件ある。** 最初に書いた断り書きが、旧置き場の綴りをそのまま引用して
いたため `shiori-host-32[/\]fixtures` が **1 件**残った。数え直して見つけ、引用をやめて
「32bit 実証例の展開済みフォルダ」という言い方に直した。数える前に書いた文が数えられる側に回る、
という取り落とし方である。

### 走らせて確かめたこと

| 確かめ | 結果 |
|--------|------|
| `cargo test -p ukadoc-survey`（着手前・基準） | 601 / 0 / 6 / 113 / 5 ＝ **725 passed・0 failed** |
| 同（台帳だけ直した時点・意図した赤） | **113 中 2 failed**（上の判定 ⑸） |
| 同（`roadmap-draft.md` 追随後） | **725 passed・0 failed** |
| 同（報告を作り直して CRLF に戻した後） | consistency **113 passed・0 failed** |
| 旧置き場の綴りの数え | 上表のとおり（較正つき） |
| `cargo test --workspace -j 4` | **111 スイート・7,935 passed・0 failed**（`test result` 行を 111 本数えて完走を確かめた。`Select-Object -First N` や `| head` は使っていない） |

### 数の突合（設計が言った数 vs 実測）

| 設計の記述 | 実測 | 一致 |
|------------|------|------|
| `install.txt` のキー 11 項目 | 11 | ○ |
| 対象外 4 見出し | 4 | ○ |
| `descript_install` は 15 項目・全て `absent`・`owner = ""` | 15・全て `absent`・全て `owner = ""` | ○ |
| `owner_count = 11` | 11 | ○ |
| `[briefs].count` 27→28 | 27→28 | ○ |
| 散文 3 か所 | **4 か所**（「表に無い 2 本」・「意図しない重なり」・「数え方／行数が 27 のまま」の段落・「0 本の束は 36」）。4 つ目は設計が挙げていないが、`**0 本**` を持つ表の行を数え直すと 36→35 になるため直した | △（1 か所多い） |
| `wave` 欄は自由文字列なので `"A0"` は通る | 通った | ○ |

---

## 6.4 検体を畳む手順の README（2026-09-19）

対象要件: 10.10

### 何をしたか

`vendors/sample_ghost/README.md` を新設した（113 行・LF・CR は 0 個）。次の検体を足す人が、登記表に 1 行も持たない状態から畳み終えるまでを追える形にしてある。

書いたのは要件 10.10 が名指しする 5 点（追跡ファイルだけを含める・永続化フォルダを含めない・改行も文字コードも変換しない・配布形の構造・畳んだ後に一致を確かめる）と、5.1 で使った畳み込みの呼び方、加えて畳んだ後の取り出し方（`SampleRoot::acquire` と `nar-sample-path`）である。

### 5.1 との食い違いを 1 つだけ意図的に作った

5.1 の囲みは 3 つの呼び方を並べているが、そのうち引数無しの形と `--check` 単独の形は**タスク 5.4 以降もう通らない**（5.6 の「5.1 の記述の期限切れ」で既に記録済み）。README に書いたのは `--from <フォルダ>` と `--from <フォルダ> --check` の 2 つだけである。

実測で確かめた。

| 呼び方 | 終了コード | 標準エラー |
|--------|-----------|-----------|
| `cargo run -p sample-ghost-kit --example fold-samples` | **1** | `--from <展開形のフォルダ> を書くこと（登記表は展開形の在処を持たない）` |
| `… -- --check` | **1** | 同上 |
| `… -- --from vendors/sample_ghost/<フォルダ>` | 0 | — |

5.1 の「本タスクで実際に踏んだのは引数無しの形」という記述と README が食い違うのはこの 1 点だけで、5.1 自身が「後続の検体を足すときに使えるのは `--from` の形だけなので README に書くのはこちらにすること」と申し送っているとおりに書いた。他の 4 点（出どころは `git ls-files`・写しはバイト複製・往復で両方向を突き合わせる・写像は `install.txt` の解釈だけから作る）は 5.1 の記述と同じである。

### 手順を頭から通した（README の予行）

README が書いたとおりの順で、既存の検体を 1 本使って手順 1〜4 を実際に踏んだ。`emo2-kakukaku-wplimit.nar` を `vendors/sample_ghost/readme-rehearsal/` へ展開し（20 ファイル）、`git add` し、`--from` で畳み、`--check` で往復を確かめ、`git rm -r --cached` と実体削除で片付けた。

```
readme-rehearsal: 追跡 20 ファイル / 展開 20 ファイル / balloon/emo2-kakukaku-wplimit=20 / .nar 33906 バイト / 不一致 0 件
全ての検体で一致
```

畳み直した `readme-rehearsal.nar` の SHA-256 は元の `emo2-kakukaku-wplimit.nar` と**同じ**（`a62e08f78c6d4aaa36f68fdc8541a23e6dc757d9b5701dac814bc69ad870feaf`）で、5.1 が記録した値とも一致する。README に書いた印字の例はこの走行の実物である。

予行の後始末は済んでいて、`git status` は README 1 件（追跡外の新顔）だけを出す。`vendors/sample_ghost/` の中身は `.gitattributes`・`.gitignore`・`.nar` 4 本に戻っている。

### 5 点の裏取り

| 点 | 何で確かめたか |
|---|---|
| ⑴ 追跡ファイルだけ | 出どころは `fold-samples.rs` の `tracked_files()`＝`git ls-files -z` 1 つだけ。走査（`walk`）は突き合わせ側にしか使っていない |
| ⑴ の静かな 0 | `git ls-files vendors/sample_ghost/R_POST_and_KOMAINU/`（5.6 で消えた木）が **0 行・終了コード 0** を返すことを実測。手順側は追跡 0 件で落ちる判定を持つ |
| ⑵ 永続化フォルダ | 5.1 の実測（`emo2` はディスク 157・追跡 110＝差 47・**差は全て `profile/` 配下で追跡外**）を引いた。5.1 が言っているのは「追跡外」までで、除外の理由は言っていない。初稿はこれを「`.gitignore` の規則で落ちていた」と書いたが**誤り**で、下の実測で覆した |
| ⑵ `*_test.txt` の罠 | `git check-ignore --no-index` を 2 か所で実測。`vendors/sample_ghost/readme-rehearsal/dic09_Test.txt` は**終了コード 1**（無視されない。`-v` を付けると当たった規則が `vendors/sample_ghost/.gitignore` の `!*_test.txt` の行と出る）、`crates/areka/dic09_Test.txt` は**終了コード 0**（無視される。当たった規則はリポジトリ直下の `.gitignore:6:*_test.txt`）。片側だけなら判定が素通りしている可能性が残るので両方を採った |
| ⑶ 改行・文字コード | `git check-attr text` が `vendors/sample_ghost/*.nar` で `text: unset`・属性の届かない `crates/areka/x.nar` で `unspecified`。道具側は `std::fs::read` / `fs::copy` の生バイトで、テキストとして読む経路が無い |
| ⑷ 配布形の構造 | 独立の読み手（Python の `zipfile`）で 4 本を一覧。`install.txt` は**書庫の最上位**に在り（`emo2.nar` の最上位は `delete.txt` / `emo2-kakukaku/` / `ghost/` / `install.txt` / `readme.txt` / `shell/` / `updates.txt`）、**全エントリが圧縮方式 0**、`testzip()` は 4 本とも `None`、ファイルのエントリ数は 110／43／20／20。読み手側の要求は `areka_nar::manifest::locate_install_txt`＝最上位のエントリ名が丸ごと `install.txt` に等しいこと（包みフォルダ 1 段は黙って剥がさず拒否） |
| ⑸ 畳んだ後の一致 | 5.1 と同じ方法をそのまま書いた＝本番の展開器で空の根へ入れ直し、**両方向**（増えていない／欠けていない・バイトが同じ）を突き合わせる。恒真を塞ぐ 3 つの判定（追跡 0 件・写しの数＝追跡の数・写像が潰れていない）も書いた |

### ⑵ の因果を実測で引き直した（初稿の誤りの是正）

初稿の README は 47 件を「`.gitignore` の規則で落ちていた（たまたま無かったのではない）」と書いていた。**`profile/` に当たる無視規則は 1 つも存在しない。** 無視の出どころを全部当たって測った。

| 出どころ | 中身 | `profile/` に当たるか |
|---|---|---|
| リポジトリ直下の `.gitignore` | `target` / `Cargo.lock` / `tmpclaude*` / `.vs/` / `*_test.txt` / `*_dump.txt` / `__pycache__/` / `*.py[cod]` の 8 パターン | 当たらない |
| `.git/info/exclude`（ワークツリー側） | 空 | 当たらない |
| 共通 git ディレクトリの `info/exclude` | `.claude/**` 系のみ | 当たらない |
| `core.excludesFile` | 未設定（`git config --get` が終了コード 1） | — |
| `vendors/sample_ghost/.gitignore` | `!*_test.txt` / `!*_dump.txt` の打ち消し 2 行 | 当たらない（そもそも打ち消し側） |

```
$ git check-ignore -v --no-index vendors/sample_ghost/emo2/ghost/master/profile/ghost.dat     ; echo exit=$?
exit=1
$ git check-ignore -v --no-index vendors/sample_ghost/emo2/ghost/master/profile/sakura.dat    ; echo exit=$?
exit=1
$ git check-ignore -v --no-index vendors/sample_ghost/emo2/ghost/master/profile/updates2.dat  ; echo exit=$?
exit=1
```

出力は 1 行も無く、終了コードは 3 本とも 1（＝無視されない）。当たった規則が在れば `-v` がその出どころと行を印字するので、無言の 1 は「どの出どころにも当たっていない」ことを意味する。裏付けを 2 つ足した。

- `git log --all` で追加されたパスを全部数えても `profile/` を含むものは **0 件**＝この木は一度も追跡されたことが無い。
- `vendors/` の下に在った `.gitignore` は履歴上 `vendors/sample_ghost/.gitignore` の **1 本だけ**＝検体の木に入れ子の `.gitignore` が同梱されていたことも無い。

したがって 47 件が入らなかった理由は「規則で除外された」ではなく「**誰も `git add` しなかった**」であり、守っていたのは ⑴ の「出どころは追跡ファイルだけ」の側である。5.1 の表（`emo2` はディスク 157・追跡 110＝差 47）の「追跡外」という書き方が正しく、初稿はそれを「規則により除外」へ格上げしていた。

⚠ ただし出どころは初稿の創作ではない。**5.1 の箇条書き 3 点目（本報告 `## 5.1` の 3 番）が既に「47 件が『たまたま無かった』のではなく規則で落ちている」と書いている**。その行が引いている規則は `crates/pilot/examples/shiori-host-32/.gitignore:3:fixtures/emo2/ghost/master/profile/` で、**タスク 5.6 で当の `.gitignore` ごと消えた別の木**のものである。検体の保管場所 `vendors/sample_ghost/` には当時も今も `profile/` に当たる規則は無い。5.1 の節は追記のみで書き換えない約束なので当該行はそのまま残すが、**あの 1 行は検体の木の話ではない**——読むときはここを併せて読むこと。

### 危険が実在することと、足した守りが効くことを両方向で測った

規則が無い以上、`profile/` を持つ木に `git add` すれば入る。README の手順 2 に足した問いがそれを捕まえるかを、仮の木で両方向に踏んだ。

```
# 赤（危険が実在する）: profile/ を持つ木を git add すると追跡される
$ mkdir -p vendors/sample_ghost/guard-rehearsal/ghost/master/profile
$ printf 'directory,guard-rehearsal\n' > vendors/sample_ghost/guard-rehearsal/install.txt
$ printf 'x\n' > .../profile/ghost.dat ; printf 'y\n' > .../profile/sakura.dat
$ git add vendors/sample_ghost/guard-rehearsal
$ git ls-files vendors/sample_ghost/guard-rehearsal | grep -E '/profile/' | wc -l
2

# 緑（守りを踏むと消える）: profile/ を消して add し直すと 0
$ rm -rf vendors/sample_ghost/guard-rehearsal/ghost/master/profile
$ git add -A vendors/sample_ghost/guard-rehearsal
$ git ls-files vendors/sample_ghost/guard-rehearsal | grep -E '/profile/' | wc -l
0
$ git ls-files vendors/sample_ghost/guard-rehearsal
vendors/sample_ghost/guard-rehearsal/install.txt
```

赤の 2 は「`git add` が `profile/` を実際に追跡した」ことの実物で、無視規則が守っていないことの直接の証拠でもある。0 の側は同じ問いが黙ることを示しており、**判定が常に 0 を返す作りではない**ことを赤が較正している。

仮の木は後始末した（`git rm -r --cached` の後に実体削除）。`git status --porcelain` は README 1 件と本レポート 1 件の 2 行に戻り、`git diff --cached --name-only` は 0 行（索引は空）。`vendors/sample_ghost/` の中身は `.gitattributes`・`.gitignore`・`README.md`・`.nar` 4 本である。

### 先に書かれていた 2 つの約束に答えているか

| 約束している場所 | 何を約束しているか | 答え |
|---|---|---|
| `vendors/sample_ghost/.gitignore` のコメント | 「新しい検体を足す手順は、畳む前の展開形をここへ一時的に置くことを求める」 | README の手順 1 がまさにそれを指示し、⑵ が否定 2 行の要る理由（`dic09_Test.txt` が黙って落ちる）を説明している |
| `.kiro/steering/roadmap.md` の A0-③ | 次の spec（`default-balloon-bundle`）に「畳む手順は `vendors/sample_ghost/` の README」「取得は登記表 `SAMPLES` に 1 行足すだけ」と案内 | README は登記表に行を持たない状態から始められる（手順 3 の `--from` は登記表を見ない）。手順 5 が `SAMPLES` の 1 行の書き方（`name` は `.nar` のファイル名＝`install.txt` の `directory`・`balloons` は同時に入るバルーンの `directory`）と、**登記は畳んだ後**であることを書いている |

### 走らせた結果

| 何を | 結果 |
|------|------|
| `cargo test -p sample-ghost-kit -j 4` | 5 的すべて緑（48 / 0 / 3 / 5 / 3 passed・failed 0） |
| `cargo test -p log-capture-kit -j 4`（常設検査 1.6・1.8 と 1,000 行の番人を含む） | 8 的すべて緑（117 passed / 0 failed / 2 ignored） |
| `git status --porcelain` | `?? vendors/sample_ghost/README.md` と本レポートの 2 行のみ＝本タスクで触ったのはこの 2 ファイルだけ |

`vendors/sample_ghost` の外は 1 ファイルも触っていない（`_Boundary:` のとおり）。

### 残した懸念

- README は `.nar` の中のファイル数と大きさを表に持つ。これは 5.1 の実測と現物の大きさに一致しているが、**表示するだけの数で判定は付いていない**ので、検体が増減すれば古びる。判定を足さなかったのは、この表が手順の正しさではなく置き場の案内だからで、代わりに件数の正本は登記表（`SAMPLES`）と `.nar` そのものにある。**この申し送りは README の手順 5 の中へ移した**——次に検体を足す人（A0-③ の `default-balloon-bundle`）が開くのは README であって本レポートではないので、「登記表に 1 行足すのと同じコミットで先頭の表にも 1 行足すこと」は README 自身に書いてある。
- README の「圧縮方式 0 でも 8 でも読める」は `areka-nar` の決定論テスト（方式 8 を踏む 8 本）が根拠で、**実機で方式 8 の `.nar` を入れた記録は無い**（5.1 の残した懸念と同じ話）。外から貰った圧縮済みの `.nar` を初めて置く人は、そこが実機初踏であることを承知しておくこと。

## 6.5 実機で一周し、作業木が汚れないことを確かめる（2026-09-19）

**結論: 合格。** 2 周とも `OnFirstBoot` がログに出て、2 周とも `areka.exe` が終了コード 0 で終わり、ワークスペース全体のテストと実機 2 周の後に追跡外のファイルは 1 つも増えていない（`git status --porcelain` は 0 行）。要件 7.11・9.7 を満たす。

走らせた機械: Windows 11 Pro 10.0.26200・実表示あり・主モニタの実効 DPI 192（表示倍率 200%）・実 pasta.dll を 32bit の補助実行体が駆動。作業木 `…\.claude\worktrees\areka-p0-sylphya-ledger-487721`・`HEAD` は `e788b776`（6.4 の commit）。

### 走らせる前の状態

```
$ git status --porcelain
（0 行）
```

### ⑴ 前提の用意——32bit の補助実行体を所定の場所へ置く

ワークスペース全体のビルドは同じ名前の x64 版を `target/debug/` へ置いてしまうので、**置くのはビルドの後・走らせるの前**でなければならない（既存の定石。`crates/areka/tests/emo2_real_run.rs` 冒頭の doc と `crates/areka/src/placement/transition_judge_offset_signoff_tests.rs` の手順が正本）。順序は「i686 をビルド → ワークスペース全体のテスト → `areka` の bin をビルド → 補助実行体を複写 → 2 周」とした。

```powershell
> cargo build -p shiori-host32-helper -p shiori-host32-testdll --target i686-pc-windows-msvc
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.77s
EXIT=0

> Copy-Item "…\target\i686-pc-windows-msvc\debug\shiori-host32-helper.exe" `
            "…\target\debug\shiori-host32-helper.exe" -Force
EXIT=0
```

置いた実行体が本当に 32bit であることは、PE ヘッダの機種欄を読んで確かめた（**0x14C＝i386**。x64 なら 0x8664 になる）。この確認は 2 周の**それぞれの直前にも**行い、間に挟まる `cargo run`（次項の窓口）が x64 版で上書きしていないことを毎回確かめている。

```
PE machine = 0x14C   （補助実行体を置いた直後）
PE machine = 0x14C   （周 1 の窓口呼び出しの後）
PE machine = 0x14C   （周 2 の窓口呼び出しの後）
```

### ⑵ ワークスペース全体のテスト

```powershell
> cargo test --workspace -j 4
EXIT=0
```

`-j 4` は必須である（全並列だと「invalid metadata」（実体はページングファイル不足・os error 1455）で落ちることがある）。**完走の判定は末尾を眺めて行わず、結果行を数えて行った**（上流を止めてしまう `Select-Object -First N` は使っていない。出力は丸ごとファイルへ落として数えた）。

| 数えたもの | 数え方 | 結果 |
|---|---|---|
| 結果行 | `grep -E "^test result:" … \| wc -l` | **111** |
| 通った本数 | 同じ行から `passed` の数を合計 | **7,935** |
| 落ちた本数 | 同じ行から `failed` の数を合計 | **0** |
| 除外 | 同じ行から `ignored` の数を合計 | **40** |
| 赤の結果行 | `grep -cE "^test result: FAILED"` | **0** |

直前の commit で記録されている基準（結果行 111・7,935 本・失敗 0・除外 40）と**完全に一致**する。結果行が減っていれば走らなかった的があることになるが、減っていない。

### ⑶ 周 1

窓口（要件 1.9 のコマンド）に絶対パスを刷らせる。**印字された値をそのまま使い、自分でパスを継ぎ足していない**（相対パスや手組みのパスだと pasta.dll の LOAD が 0x8007007E で落ちる）。

```powershell
> cargo run -q -p sample-ghost-kit --bin nar-sample-path -- emo2
root=C:\home\maz\git\areka\.claude\worktrees\areka-p0-sylphya-ledger-487721\target\nar-samples\manual\emo2
folder=C:\home\maz\git\areka\.claude\worktrees\areka-p0-sylphya-ledger-487721\target\nar-samples\manual\emo2\ghost\emo2
balloon.emo2-kakukaku=C:\home\maz\git\areka\.claude\worktrees\areka-p0-sylphya-ledger-487721\target\nar-samples\manual\emo2\balloon\emo2-kakukaku
EXIT=0
```

配られた根に起動記録が無いことを、走らせる前に数えた——`profile` という名のフォルダは **0 個**である。

起動は有界の自動終了を付ける。

```powershell
> $env:RUST_LOG = "info,kanade=trace"
> $env:AREKA_APP_SMOKE_EXIT_MS = "30000"
> & "…\target\debug\areka.exe" <上の folder=> <上の balloon.emo2-kakukaku=>
LAP1 areka.exe EXIT=0  elapsed=32s  loglines=212
```

アプリが実際にその根を使ったことは、ログの 2 行目が印字した値をそのまま復唱していることで分かる。

```
2026-09-18T23:35:14.953528Z  INFO areka: resolved config inputs ghost_root=C:\home\maz\git\areka\.claude\worktrees\areka-p0-sylphya-ledger-487721\target\nar-samples\manual\emo2\ghost\emo2 balloon_root=C:\home\maz\git\areka\.claude\worktrees\areka-p0-sylphya-ledger-487721\target\nar-samples\manual\emo2\balloon\emo2-kakukaku
```

**初回起動のイベント（周 1）**:

```
2026-09-18T23:35:16.208338Z TRACE actor{actor=kanade}: kanade: SHIORI 送出 event="shiori_request" method=GET id=OnFirstBoot references=["0"] status=None
```

続けて挨拶が返り、起動系列が閉じている（＝32bit の補助実行体が実 pasta.dll を本当に駆動している）。

```
2026-09-18T23:35:16.212059Z  INFO actor{actor=kanade}: kanade: 起動グリーティングを再生起動 event="boot_talk" talk_id=1
2026-09-18T23:35:16.212301Z  INFO actor{actor=kanade}: kanade: basewareversion 完了——boot 系列完了・定常運転へ event="boot_complete"
2026-09-18T23:35:45.741842Z  INFO actor{actor=shiori}: shiori-actor: 正規 clean shutdown 完了（unload → helper 正常終了 exit(0)） event="unload_clean"
```

### ⑷ 周 1 が本当に記録を残したこと——周 2 の意味の担保

周 2 の `OnFirstBoot` が意味を持つのは、**周 1 が「次は初回ではない」と判定させる記録を確かに書いた**ときだけである。書いたことを実物で示す。

| 何を | 結果 |
|---|---|
| 根の下の `profile/` 配下のファイル数（走行前は 0） | **47** |
| そのうち areka 自身の起動記録 | `ghost\emo2\ghost\master\profile\areka\sylphya.toml`（39 バイト） |
| 中身 | `format-version = 1` / `[boot]` / `count = "1"` |
| ゴースト側（pasta）の永続化 | `profile\pasta\` 配下 46 ファイル（Lua のキャッシュ・ログ・`save\save.json`） |
| 実行体の隣（App スコープ）の `profile/` | 0 ファイル |

`areka.boot.count` は起動記録ゲートの唯一の鍵で、**存在すれば** `first_boot=false` になる（`crates/areka-ghost/src/runtime.rs` の `apply_boot_record_gate` step 1。値の数値解釈はせず存在だけを見る）。周 1 の終了時には `ghost-shutdown: persist flush confirmed` が出ており、書き込みは確定している。つまり**同じ根をそのまま使い回せば、周 2 は必ず `OnFirstBoot` を飛ばす**。

### ⑸ 周 2

もう一度窓口を呼ぶ。

```powershell
> cargo run -q -p sample-ghost-kit --bin nar-sample-path -- emo2
root=C:\home\maz\git\areka\.claude\worktrees\areka-p0-sylphya-ledger-487721\target\nar-samples\manual\emo2
folder=C:\home\maz\git\areka\.claude\worktrees\areka-p0-sylphya-ledger-487721\target\nar-samples\manual\emo2\ghost\emo2
balloon.emo2-kakukaku=C:\home\maz\git\areka\.claude\worktrees\areka-p0-sylphya-ledger-487721\target\nar-samples\manual\emo2\balloon\emo2-kakukaku
EXIT=0
```

**根のパスは周 1 と 1 文字も違わない**（3 行を突き合わせて一致）。手動用の根は `manual/<検体>/` という決まった場所で、新しい場所を作るのではなく**その場で作り直す**設計だからである（design「取得（manual）」）。したがって「新品であること」はパスの違いでは示せず、中身で示すほかない。

| 周 2 の窓口呼び出しの直後 | 結果 |
|---|---|
| 根の下の `profile/` 配下のファイル数（周 1 は 47 を残した） | **0** |
| `…\profile\areka\sylphya.toml` の実在 | **False** |

47 個あった永続化ファイルが 1 つ残らず消えている。つまり作り直しは本物である。

```powershell
> & "…\target\debug\areka.exe" <folder=> <balloon.emo2-kakukaku=>   （env は周 1 と同じ）
LAP2 areka.exe EXIT=0  elapsed=31s  loglines=184
```

**初回起動のイベント（周 2）**:

```
2026-09-18T23:36:47.951514Z TRACE actor{actor=kanade}: kanade: SHIORI 送出 event="shiori_request" method=GET id=OnFirstBoot references=["0"] status=None
```

配置の復元も「保存値なし」から始まっている（`saved_win_x=None saved_win_y=None saved_off_x=None saved_off_y=None`）。周 2 も挨拶が返り（`boot_talk talk_id=1`）、`unload_clean` で閉じ、終了コードは 0 である。

**2 周とも `OnFirstBoot` が出た。要件 7.4・9.7 の求める姿である。**

### ⑹ 「出ていない」を主張する側の較正

「`OnFirstBoot` が出た」だけでは、探し方が何でも拾う作りである可能性を排除できない。そこで**出てはならない綴りが 0 になること**と、**その綴りが実在する場所ではちゃんと拾えること**の両方を、同じ探し方で示す。

2 回目以降の起動では `OnFirstBoot` を発行せず、代わりに `boot_gate skip_first_boot` という記録が残る（`crates/areka-kanade/src/schedule/boot.rs` の `apply_boot_record_gate` 後の分岐、`first_boot` が偽の側）。

| 探した綴り | 探した先 | 期待 | 実測 |
|---|---|---|---|
| `OnFirstBoot` | 周 1 のログ | 1 件以上 | **1** |
| `OnFirstBoot` | 周 2 のログ | 1 件以上 | **1** |
| `skip_first_boot` | 周 1 のログ | 0 | **0** |
| `skip_first_boot` | 周 2 のログ | 0 | **0** |
| `boot_start` | 周 1・周 2 のログ | 1 件以上（探し方が何も拾えないわけではないことの当たり） | **各 1** |
| `skip_first_boot`（**綴りそのものの当たり**） | `crates/areka-kanade/src/schedule/boot.rs` | 1 件以上 | **1** |

最後の行が要である——同じ正規表現を、その綴りが本当にある場所へ当てると **1 件**返る。だから周 1・周 2 の **0** は「探し方が壊れていて何も拾えない」ではなく「本当に無い」である。この綴りの経路が到達可能であること自体は、決定論テスト `crates/areka-ghost/tests/ghost/spine_e2e_test_s7_second_boot_record_present.rs`（起動記録が在る根では `OnFirstBoot` を飛ばす）が押さえており、これは⑵のワークスペース走行に含まれて緑である。

送出したイベント ID の全数（2 周の合計）は次のとおりで、`OnFirstBoot` はちょうど 2 件——周ごとに 1 件ずつである。

```
$ grep -hoE 'id=On[A-Za-z]+' lap1.log lap2.log | sort | uniq -c
      2 id=OnClose
      2 id=OnFirstBoot
      2 id=OnInitialize
     60 id=OnSecondChange
```

### ⑺ 作業木が汚れていないこと（要件 7.11）

ワークスペース全体のテストと実機 2 周を終えた後の状態。

```
$ git status --porcelain
（0 行）

$ git status --porcelain | wc -l
0

$ git ls-files --others --exclude-standard | wc -l
0
```

**この 0 も較正する。** 追跡外のファイルを 1 つ置けば同じ問いが 1 を返し、消せば 0 に戻る。

```
$ touch .kiro/specs/areka-P0-nar-install/__calib_probe.tmp
$ git ls-files --others --exclude-standard
.kiro/specs/areka-P0-nar-install/__calib_probe.tmp
$ git ls-files --others --exclude-standard | wc -l
1
$ rm .kiro/specs/areka-P0-nar-install/__calib_probe.tmp
$ git ls-files --others --exclude-standard | wc -l
0
```

**シェル側の永続化ファイルも含めて**追跡領域に現れていないことを、名前で探して確かめた。

| 探したもの | 探した範囲 | 結果 |
|---|---|---|
| `profile` という名のフォルダ | `target/` と `.git/` を除く全域 | 1 件（下記のとおり本走行とは無関係） |
| `*.dat`（シェル側の永続化の典型的な綴り） | `target/` と `.git/` を除く全域 | **0** |
| `profile` という名のフォルダ | `target/nar-samples/` 配下（**当たり**） | `target/nar-samples/manual/emo2/ghost/emo2/ghost/master/profile` |

最後の行が当たりである——同じ探し方で、走行が本当に作った永続化フォルダは見つかる。見つかったうえで `git status` に現れないのは、手動用の根が `target/` の下（無視規則の 1 行目 `target`）にあるからで、`git check-ignore -v` が理由まで答える。

```
$ git check-ignore -v target/nar-samples/manual/emo2/ghost/emo2/ghost/master/profile/ghost.dat
.gitignore:1:target	target/nar-samples/manual/emo2/ghost/emo2/ghost/master/profile/ghost.dat
```

`target/` の外に 1 件見つかった `vendors/pasta/crates/pasta_lua/profile` は**本走行の産物ではない**——中にファイルは **1 個**ある（`find vendors/pasta/crates/pasta_lua/profile -type f` が 1 行返す＝`pasta/save/save.json`・2 バイト・2026-09-17 20:12）。無関係だと言える根拠は「空だから」ではなく**日時と追跡状態**である——本走行（2026-09-19 08:35／08:36）の 2 日前の日時であり、かつ `vendors/pasta` はサブモジュールで `git submodule status` も `git -C vendors/pasta status --porcelain` も汚れを報告しない（後者は 0 行）。つまり本走行の前から在って、本走行で動いてもいない。

### ⑻ この一周が確かめていない範囲

- **圧縮方式は 0（無圧縮）しか踏んでいない。** 検体の `.nar` 4 本は全エントリが方式 0 である（`emo2` 128／128・`R_POST_and_KOMAINU` 49／49・`emo2-kakukaku-offsetdpi` 20／20・`emo2-kakukaku-wplimit` 20／20——`zipfile` で全エントリの `compress_type` を数えた）。**方式 8（deflate）の伸長を実機で踏んだ記録はここには無い**。方式 8 は `areka-nar` の決定論テスト 8 本が押さえており（5.1・6.4 の README の残した懸念と同じ話）、外から貰った圧縮済みの `.nar` を初めて置く人は、そこが実機初踏であることを承知しておくこと。
- **検体は emo2 1 つだけ。** 要件 9.7 が名指しするのが emo2 の一周だからで、他の 3 検体を実機で起動してはいない（決定論テストの登記往復は 4 本すべてを踏んでいる）。
- **目視のサインオフはしていない。** 本タスクの完了条件はログと `git status` の記録であり、立ち絵の表示位置・typewriter の進行・ドラッグ追従といった人間の目で見る項目は `crates/areka/tests/emo2_real_run.rs` 冒頭の手順に従って別途行うものである。
- **有界の自動終了は強制終了（ForceQuit）の経路である。** 「この経路は強制終了で、別れの挨拶を経ない」と過去に記録されている但し書きは**正しく、本走行はそれを覆していない**。2 周とも同じ 1 行が出ている——`lap1.log:207` と `lap2.log:179` の
  `WARN actor{actor=kanade}: kanade: 強制終了指示——終了系列（Forced）へ直行 event="force_quit" reason="user"`。
  コードも同じことを言う——`crates/areka-ghost/src/runtime.rs:239` の `shutdown` が送るのは `KanadeMsg::ForceQuit` だけで、`crates/areka/src/emo2_boot/spine.rs:671` は「`GhostRuntime::shutdown` は**常に** ForceQuit 経路」と明記している。`OnClose` は NOTIFY であって GET ではないので、**返答としての別れの挨拶トークは再生されない**（2 周のログに `ClosePending`／`CloseTalkWait` の類は 1 行も無い）。
- **本走行がここに足したのは別の事実である。** 強制終了の経路であっても、`OnClose` NOTIFY → `unload_clean` → `persist flush confirmed` までは到達している（`lap1.log:208`〜`211`／`lap2.log:180`〜`183`）。だから周 1 は登記を書き終えてから終わったと言える——⑷の担保はこれで立つ。本タスクが主張しているのは起動側（`OnFirstBoot`）なので、挨拶トークが無いことは判定に影響しない。
- **窓口を 2 つ同時に呼んではいない。** 窓口は呼ぶたびに `manual/<検体>/` を消して作り直すので、走行中に別の端末で呼ぶと走っている木が消える。2 周は厳密に逐次で行った（周 1 の終了コードを受け取ってから周 2 の窓口を呼んだ）。6.1 が 6.5 へ送った未検証 ⒞ はこれで消化した。
- **`tools/perf/` の計測経路には触れていない。** 6.1 が 6.5 へ送った未検証のうち ⒜⒝⒟（実測 3 種での引き渡し・`invoke-followup-checks.ps1` の単独起動・`check-quiet` 経路）は本タスクの `_Boundary: 実機サインオフ` の外である。なお `tools/perf/check-quiet.ps1` の自己較正が HEAD で赤く `perf-loop.ps1 selftest` が終了コード 4 で止まることは既知で、本タスク以前からある事象であり、本走行はその経路を通っていない。
- **⒜⒝⒟ は「引受先不在の先送り」である。** 6.5 は本 spec の最後のタスクなので、境界の外と判定したこの 3 件を吸収する後続タスクが無い。よって次のとおり名指しして最終検証と開発者へ上げる——⒜ `tools/perf/invoke-perf-run.ps1` へ `-GhostRoot`／`-BalloonRoot` を実測 3 種で渡す経路の実走確認（6.1 で既定値を削除して必須引数にしたため、渡し忘れは起動時に止まる側へ倒してある）、⒝ `tools/perf/invoke-followup-checks.ps1` を perf-loop を介さず**単独起動**したときの窓口経路（perf-loop 経由では measure が必ず両方渡すので通らない）、⒟ `tools/perf/check-quiet.ps1` の経路（32bit ヘルパ・管理者権限・静寂な機械）——⒟は上記の既知の赤（`perf-loop.ps1 selftest` が終了コード 4）を先に解く必要がある。

## 最終検証（2026-09-19）

4 つの観点を並行で確かめ、いずれも中止の根拠は出なかった。

- **テスト**: `cargo test --workspace -j 4` が `test result:` 行 111 本・7,935 passed・0 failed・40 ignored。i686 helper を `target/debug/` へ置いた後に走らせ、途中で出力を切らずに `test result:` 行を数えた（切ると上流が止まって一部のスイートが走らないまま緑に見える）。
- **受け入れ基準**: 要件 10 節・95 項目のすべてに着地の根拠がある（機械で数え直して 95）。
- **構造と依存の向き**: 層の逆流は無い。`sample-ghost-kit` は本番の依存グラフに入らず、`areka-nar` は `areka-parsers`・`encoding_rs`・`miniz_oxide`・`thiserror`・`tracing` だけを引く。
- **古い綴りの残り**: `crates`／`tools`／`doc`／`vendors` に、消した 2 つの木を指す綴りは 0 件。較正は畳む前の実バイトから採った——`git grep -E "(shiori-host-32[/\\]+fixtures|sample_ghost[/\\]+R_POST_and_KOMAINU[/\\]+)" 834739a7^` は `crates/areka-nar/src/manifest_tests.rs` に 3 件など複数の当たりを返す。区切りは `[/\\]+` の文字クラスで書くこと（Rust の生文字列リテラルは `\` を 2 バイトで持つので、1 文字幅のクラスでは黙って取り落とす）。

### 改行コードの片道切符（これまで誰も書き残していない）

畳み込みは **作業ツリーのバイト列**を取り込んだ。旧 emo2 と派生バルーンの 2 系統には `.gitattributes` が無く、このリポジトリは `core.autocrlf=true` なので、作業ツリーの側が CRLF になっていた。結果、`.nar` の中身は git のオブジェクトデータベースの blob と次のようにずれている（実測）。

| 検体 | ファイル数 | バイト一致 | 改行だけ違う |
|---|---|---|---|
| `emo2.nar` | 110 | 78 | 32 |
| `emo2-kakukaku-offsetdpi.nar` | 20 | 15 | 5 |
| `emo2-kakukaku-wplimit.nar` | 20 | 15 | 5 |
| `R_POST_and_KOMAINU.nar` | 43 | 43 | 0 |
| 合計 | 193 | 151 | 42 |

`R_POST_and_KOMAINU` だけが 43/43 でバイト一致なのは、この検体だけが `* -text` の効く場所に置かれていたためである。改行以外で中身が違うファイルは 1 件も無い。

**挙動は保たれている。** 畳む前のテストと実機が読んでいたのも同じ作業ツリーのバイト列であり、そこは変わっていない。今後については `vendors/sample_ghost/.gitattributes` の `* -text` が `.nar` のバイト列を凍結する。

**ただしこれは片道切符である。** 書庫はある 1 台の作業ツリーの状態を封じ込めており、畳む前の木はもう無い。だから `core.autocrlf` の設定が違う環境から、この突き合わせをやり直すことはできない。作り直すなら、畳む前の木を git の履歴（`834739a7^`）から取り出すところから始めることになる。

### 最終検証で直したもの（挙動の変更は無し・文書とコメントのみ）

- **design.md の追随**（タスク 5.4 が予告して未着手だったもの）: ⑴ 見張りの較正の記述を、実際に作られた「4 形それぞれの実体の有無を 4 状態で宣言して判定する」形へ書き直した。⑵ 回収・掃除の失敗の宛先を `warn!`／`debug!` から標準エラー 1 行へ訂正。⑶ 窓口の `debug!`（cache hit／miss・掃除の件数・計時）3 か所は**そもそも到達不可能だった**——Allowed Dependencies が窓口に `tracing` を許していない。3 件とも誰も記録していなかった新規の指摘で、Allowed Dependencies の登記が拘束力を持つ旨を本文に書き添えた。
- **古い引用**: `install_commit_tests.rs` の「設計は `share_mode(0)` と書いている」を現行の `share_mode(1)` へ（0 が使えない理由は残した）。`error.rs` の要件番号 4 件が 2 組で入れ替わっていたのを正した（`IntegrityMismatch` 2.5→2.6・`UnsupportedEntry` 2.6→2.5・`SymlinkEntry` 4.7→2.7・`CaseCollision` 2.7→4.7）。`sample_support.rs` の「依存は `thiserror` のみ」を実測の依存一覧へ（純 Rust であるという肝心の主張は正しいので残した）。
- **同時に呼ぶと壊れる話を、読み手が開く 4 か所へ**: 窓口は呼ぶたびに `manual/<検体>/` を消して作り直すので、2 つ目のプロセスが走っている木を消す。この警告は `roadmap.md` の 1 か所にしかなかった。`nar-sample-path.rs` の説明・`tools/perf/README.md`・`crates/pilot/examples/shiori-host-32/README.md`・`vendors/sample_ghost/README.md` に 1〜2 文ずつ足した。
- **生きている 6 本の brief の古い綴り**: 要件 10.5 の掃除範囲が `tools/`／`doc/` だったため `.kiro/specs/` は掃かれておらず、6 本のうち 1 本しか直っていなかった。残りを日付つきの最小の書き換えで直した。`shell-implicit-surface` の「本 spec が先」という順序の推奨は、2026-09-18 の反転と本 spec の着地で**もう選べない**ので、その旨を書いた。
- **次の spec への地雷**: `crates/sample-ghost-kit/src/lib_tests.rs` の 2 本のテストが `SAMPLES.len()` を 4 と直書きしている。`StayseeBalloon` を登記すると両方赤になるので、`default-balloon-bundle` の brief の追記節に「登記の行を足す同じコミットで 2 か所の数も直すこと」を 1 行足した。

### 引受先の無い先送り（担当を決めていただきたい）

1. **台帳 11 行の `status` を `absent` から起こすこと**（6.3 の記録）。本体 `areka` の依存グラフへ `areka-nar` を繋ぐのは `areka-P0-ghost-install` だが、同 spec の brief には台帳のことが 1 文字も書かれていない。配線が着地したときに誰が台帳を起こすのかが決まっていない。
2. **`areka-nar` にパス長・要素長の上限が無いこと**（実装メモ 3.2 が自ら「設計の穴」と書いている）。30 万文字の要素名が受理される。利用者が渡す `.nar` を受け取る `ghost-install` が自然な引受先である。
3. **`tools/perf` の未検証 ⒜⒝⒟**（6.5 の記録）。⒟ は `perf-loop.ps1 selftest` が終了コード 4 で止まる**本 spec 以前からある赤**を先に解く必要がある（`tools/perf/check-quiet.ps1` は本 spec で 1 度も変更していない＝この範囲のコミット 0 件）。
4. **`CommitError` に作業フォルダのパスを載せるかどうか**（実装メモ 4.3 が 4.4 へ問い、答えが記録されていない）。載せないと、巻き戻せなかったとき利用者は元の木がどこに生き残っているかを知る手段が無い。
5. **失敗の記録の `work` 欄が未検査であること**（実装メモ 4.4）。13 の固定入力はすべて `WorkArea::create` より手前で拒否されるので、この欄については「空であること」しか主張できていない。確定の段の失敗を `NarArchive::install` 経由で踏むテストを作るときに、実在するフォルダを指していることを判定すること。
