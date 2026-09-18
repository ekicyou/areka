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
