# 出典と保管の記録（provenance・タスク 1.3）

| 項目 | 値 |
|---|---|
| 対象仕様 | `areka-P0-default-balloon-bundle` |
| 対象要件 | 2.2・2.3・2.4・2.5・2.6・5.1・7.1 |
| 実施日 | 2026-09-18 |
| 実施した作業木 | `C:\home\maz\git\areka\.claude\worktrees\areka-p0-balloon-bundle-6fe55e`（ブランチ `claude/areka-p0-balloon-bundle-6fe55e`） |
| 実施時の HEAD | `ef0890471680487cc10b6264642753f24493b8c8` |
| 保管を行ったコミット | `0abc746ea08b014fc8d4e19d451ada22c689a718`（`vendor StayseeBalloon 29 files byte-identical (task 1.2)`） |
| 保管先 | `vendors/sample_ghost/StayseeBalloon/` |

この記録は「取得元・保管内容・改行と除外の検査結果・既定バルーン id を、第三者が同じ命令を打って再検証できる」ことを目的とする。本文の値はすべてタスク 1.3 で採り直した実測値であり、先行タスクの報告を写したものではない。各値には**実際に打った命令**を添えた。

---

## 0. 結論（先に）

| # | 主張 | 期待値 | 実測値 | 判定 |
|---|---|---|---|---|
| 1 | 保管フォルダの本数 | 29 本・サブフォルダ 0 | 29 本・サブフォルダ 0 | 一致 |
| 2 | 保管した中身が上流と同一 | 29 本の sha256 がすべて上流と一致（食い違い 0 本） | 食い違い **0 本** | 一致 |
| 3 | 改行変換（属性照会） | `git check-attr text` が 29 本すべて `unset` | `unset` 29 本・`unset` 以外 **0 本** | 一致 |
| 4 | 改行変換（索引の改行表示） | 索引に配布時の改行がそのまま入っている（`.txt` 3 本は `crlf`・`LICENSE` は `lf`・2 進 25 本は `-text`） | そのとおり（§4.2 の表） | 一致 |
| 5 | 除外規則との衝突 | リポジトリ直下の `.gitignore`（`*_test.txt`／`*_dump.txt`）に大小無視で当たる名前 **0 件** | **0 件**（`git check-ignore` も 0 件） | 一致 |
| 6 | 要件 2.6 の突合 | 一覧・descript の要点・LICENSE の種別がすべて要件本文と一致＝条件節が発動しない | 3 観点とも一致・**発動なし**（`requirements.md` の是正 0 行） | 一致 |
| 7 | 既定バルーン id（要件 7.1） | descript の `id` ＝ `install.txt` の `directory` ＝ 実フォルダ名 | 3 つとも `StayseeBalloon`（大小まで同綴り） | 一致 |

上の 7 件はいずれも「印字した数」ではなく突合の判定である。判定に使った検査が**恒真でない**（欠陥を混ぜれば赤くなる）ことは §3.3・§4.4 に較正として載せた。

---

## 1. 資産の素性（要件 5.1）

| 項目 | 値 | 典拠 |
|---|---|---|
| 資産名 | `Balloon for Staysee Syncfield` | `descript.txt` の `name` 行／`install.txt` の `name` 行／`readme.txt` の表題 |
| id | `StayseeBalloon` | `descript.txt` の `id` 行 |
| 作者（日本語表記） | ぽな（ばぐとら研究所/整備班） | `readme.txt` の「作った人：」行 |
| 作者（descript の登記） | `craftman,SSP BUGTRAQ`／`craftmanw,ばぐとら研究所/整備班` | `descript.txt` の `craftman` 行・`craftmanw` 行 |
| ライセンス | CC0-1.0（パブリックドメイン提供） | `LICENSE` の 1〜3 行目「Creative Commons Legal Code」「CC0 1.0 Universal」／`readme.txt` の「License : CC0 https://creativecommons.org/publicdomain/zero/1.0/deed.ja」 |
| 再配布条件の原文 | 「■転載・再配布・同梱・改変等について／煮るなり焼くなり好きにしてください。」 | `readme.txt` |
| 出典 URL（上流リポジトリ） | `https://github.com/ponapalt/StayseeBalloon` | 取得に用いた URL（§2） |
| 出典 URL（配布サイト） | `http://ms.shillest.net/balloon/StayseeBalloon/` | `descript.txt` の `homeurl` 行 |
| 作者サイト | `http://ms.shillest.net/` | `descript.txt` の `craftmanurl` 行／`readme.txt` の「■連絡先・配布サイトとか」 |
| 動作確認（原作者の記載） | SSP 2.4 | `readme.txt` の「■動作確認」 |
| 注意事項（原作者の記載） | 「半透明（アルファチャンネル）ONを前提に作っており、OFFではまともに見られません。」 | `readme.txt` の「■！！！注意事項！！！」 |

`descript.txt`・`install.txt`・`readme.txt` は Shift_JIS（CP932）で書かれている。上表の日本語は次の命令で復号して読んだ。

```
iconv -f CP932 -t UTF-8 vendors/sample_ghost/StayseeBalloon/descript.txt
iconv -f CP932 -t UTF-8 vendors/sample_ghost/StayseeBalloon/install.txt
iconv -f CP932 -t UTF-8 vendors/sample_ghost/StayseeBalloon/readme.txt
```

`LICENSE` は ASCII なので復号せずそのまま読める（`head -6 vendors/sample_ghost/StayseeBalloon/LICENSE`）。

---

## 2. 取得（要件 2.3）

| 項目 | 値 |
|---|---|
| 上流リポジトリ | `https://github.com/ponapalt/StayseeBalloon` |
| 取得したコミット | `fe1b02f30d5e263cf30df800c32b2b525a52e3ad` |
| そのコミットの件名 | `use_input_alpha` |
| コミット日時（原文のタイムゾーン） | `2021-11-06 05:04:31 +0900`（author・committer とも同値） |
| コミット日時（UTC） | `2021-11-05 20:04:31 +0000` |
| 取得日 | 2026-09-18 |
| 取得時点の上流の枝 | 既定枝の先端。`fe1b02f3` が取得時の HEAD |
| readme が名乗る版 | v1.00A（2020/6/27） |
| readme の更新履歴 | 2020/6/26 v1.00 First Release ／ 2020/6/27 v1.00A「readmeにゴースト縛りがないことを明記 / 半透明の件」 |
| 上流の作業木の本数 | 29 本（サブフォルダ 0・`.pna` 0 本） |

日時は次の 2 命令で採った（前者が原文の +0900、後者が UTC）。

```
git -C <上流の複製> log -1 --format='author:%ad committer:%cd' --date=iso HEAD
TZ=UTC git -C <上流の複製> log -1 --format='UTC author:%ad committer:%cd' --date=iso-local HEAD
```

### 2.1 取り直しの手順（この機械で再現するときの注意）

この機械の git は **`core.autocrlf` が `true`**（`git config --system --get core.autocrlf` が `true`）。素の `git clone` で上流を取り直すと、上流で LF だけの `LICENSE` が作業木で CRLF に変換され、保管先とハッシュが食い違う（§3.3 の較正 B で実測）。取り直しは次のどちらかで行うこと。

```
# 方法 1（本記録で採用）: 作業木を作らず、上流のオブジェクト DB から blob の生バイトを読む
git -c core.autocrlf=false -c core.eol=lf clone --bare https://github.com/ponapalt/StayseeBalloon.git <一時領域>
git -C <一時領域> cat-file blob HEAD:<ファイル名> | sha256sum

# 方法 2: 変換を切って作業木ごと取り直す
git -c core.autocrlf=false -c core.eol=lf clone https://github.com/ponapalt/StayseeBalloon.git <一時領域>
```

本記録は方法 1 を使った。`git cat-file blob` は改行変換もスマッジフィルタも通らないので、比較の基準として最も素性が確かである。一時領域はリポジトリの外（このセッションの作業用フォルダ）に置き、比較後に残していない。

---

## 3. ハッシュ一覧（要件 2.3）

### 3.1 保管フォルダから採り直した 29 本の sha256

採り方:

```
cd <リポジトリ根>
find vendors/sample_ghost/StayseeBalloon -type f | sort | xargs sha256sum
```

| # | ファイル | バイト数 | sha256 |
|---|---|---|---|
| 1 | `LICENSE` | 7,048 | `a2010f343487d3f7618affe54f789f5487602331c0a8d03f49e9a7c547cf0499` |
| 2 | `arrow0.png` | 284 | `ee13c9104c00fcc787c7908aa83bfe08330b5bde416f85ab78fd0b12cc1e58be` |
| 3 | `arrow1.png` | 297 | `12c1ec8f61ea09f659879b3a4ca1efdc44db6abdcaa07740e55db97b6011bd45` |
| 4 | `balloonc0.png` | 3,616 | `751f85ee947fb3958709bca3f76c4d3074bd005ba1753e8da11b9d7c0e24dbc4` |
| 5 | `balloonc1.png` | 4,102 | `90cde84d2867875787cfb2bbed7b6842d7a2fb1c0db37eb96239db4008fe6a87` |
| 6 | `balloonc2.png` | 3,552 | `30f57a9e2a43ef83e12ec48574b537dda60b310db8f85d8855c10d070d6670a5` |
| 7 | `balloonc3.png` | 3,476 | `0a286f8a7ed1de42d82670c00169bb54cbd1f29f712e40059d78b0c4934221ef` |
| 8 | `balloonc4.png` | 3,793 | `af1d3ad9e34addd00963dd4455da1c9110b306a44a45d3bd3b5ff6f8a3ca5cb2` |
| 9 | `balloonk0.png` | 4,780 | `3b1874e2d35811ac96bd878f6acb9464a9c848dbeffede645f379086f47bc42b` |
| 10 | `balloonk1.png` | 4,773 | `70eb76af5878effb21e3911586f1283b09f81b4dfa72e0dad7a74782f1ab4869` |
| 11 | `balloons0.png` | 5,117 | `0fc496e0620de4cf1f6277f854653ce1ffde54d05cf5ea2e400a987ed80fab61` |
| 12 | `balloons1.png` | 5,128 | `d819cd4116d17a4c553e326f480d90ce4155c417b79c768998fa688ba8f3a198` |
| 13 | `balloons2.png` | 5,891 | `8b6c84abbadbb7b29c1aa6c291636763eb9cf24db0367107b9e1825f64214844` |
| 14 | `balloons3.png` | 5,917 | `3293c92931550f79be8f9da9357751aefd9aaa956496c4edf4cc45b6fe199b51` |
| 15 | `descript.txt` | 1,323 | `6696b4eb6c127b1c530a668c8853f14b6e0356b0488cf8589d6e7d14a3126333` |
| 16 | `install.txt` | 95 | `aad39c0b4a6962ad1c1865d97fd6fd9553871f5b22ba4050ca5081b52ee253ec` |
| 17 | `marker.png` | 368 | `2403c0a5f3bfa526fe4a2809b681706f614e858a2f348e36740c41802fa6a272` |
| 18 | `online0.png` | 479 | `c4b6ca314a71b611d35abd3092ee6429785dd4c5d616bf57436346fcc5cdb540` |
| 19 | `online1.png` | 567 | `3d7a26f1166641f1080d619a7c37c5c74be1ee253598865649545a1d05adc59d` |
| 20 | `online2.png` | 749 | `e3431100072c42816e6689db23074cd6818134975fe930f6b5e23d2854298d64` |
| 21 | `online3.png` | 792 | `83c5bd8fbe258d99f96cd9c041777fac14c5866bf37a12bf34b885657b426f69` |
| 22 | `online4.png` | 859 | `391f4cfed5d5039f4ba4a3e2e82f033109b23606e1fbf0b978da2ee3a9ac52e3` |
| 23 | `online5.png` | 964 | `242af30a28eef57c1ecfb85be0a0c7db16bc0086dbf1757a1473f271b11714cf` |
| 24 | `online6.png` | 945 | `471401e32df21f330cdfa82ae349c4756b71eed13cede9db831f9d714b5e585e` |
| 25 | `online7.png` | 1,092 | `dff4ef1f4747c3087fa00546608bd86cf969a1b841fb8d52d2e5df6590c54db6` |
| 26 | `online8.png` | 1,097 | `dd9a3ce63a2a77d3b8f498a2c96aae6dca2177ef85d4fd1aa28ea993d35fb3a0` |
| 27 | `readme.txt` | 937 | `ff58764261050187d13cf780fa8171ac1f9e7fd2e004079120db8e3f9f016144` |
| 28 | `sstp.png` | 189 | `d2aec88d4c36368459efaa3715670257ab86ef83538940384e4d21deed7c38b2` |
| 29 | `thumbnail.pnr` | 1,899 | `56122ce24e900819b1079dac51ad5fbcd3063ff280eea2c975c1d9a6efed9621` |

合計 70,129 バイト。バイト数は `find vendors/sample_ghost/StayseeBalloon -maxdepth 1 -type f -printf '%f %s\n'` で採った。

サブフォルダは **0 個**（`find vendors/sample_ghost/StayseeBalloon -mindepth 1 -type d | wc -l` が `0`）。git が追跡している本数も **29 本**（`git ls-files vendors/sample_ghost/StayseeBalloon/ | wc -l`）で、作業木の本数と一致する＝追跡外の余分なファイルも、追跡だけされて実体が無いファイルも無い。

### 3.2 上流との 1 対 1 突合

上流の blob（§2.1 の方法 1）から同じ 29 本の sha256 を採り、保管フォルダの一覧と行単位で突き合わせた。

```
for f in $(git -C <上流の複製> ls-tree -r --name-only HEAD | sort); do
  git -C <上流の複製> cat-file blob "HEAD:$f" | sha256sum
done > upstream.sha256
diff upstream.sha256 stored.sha256
```

| 項目 | 期待値 | 実測値 | 判定 |
|---|---|---|---|
| 上流の本数 | 29 | 29 | 一致 |
| 保管の本数 | 29 | 29 | 一致 |
| `diff` の差分行数 | 0 | **0**（`diff` は exit 0） | 一致 |

**食い違いは 0 本**＝保管した 29 本は上流 `fe1b02f3` のバイト列そのものである（要件 2.2 の「1 バイトも変えず」の機械的な裏取り）。

### 3.3 この突合が恒真でないことの較正

「差分 0」は、比較そのものが壊れていても出る。壊れた側を混ぜたときに**赤が出る**ことを 2 通りで確かめた。

**較正 A（人工の欠陥）**: `LICENSE` を CRLF へ変換した写しを一時領域に作り（`sed 's/$/\r/'`）、そのハッシュで保管側の一覧の 1 行を差し替えて同じ `diff` に掛けた。

```
sha256sum LICENSE.orig  -> a2010f343487d3f7618affe54f789f5487602331c0a8d03f49e9a7c547cf0499
sha256sum LICENSE.crlf  -> f4e7f373b9b996950337e8d41a4a2939c2d90b7725e9baf3d5084a22717ad328
```

結果: `diff` は 1 行の食い違い（`a2010f34…` 対 `f4e7f373…`）を出して非 0 終了した。＝この突合は改行だけの違いを検出できる。

**較正 B（実際に起こりうる欠陥）**: `core.autocrlf=true` のまま素の `git clone` で上流を取り直し、その作業木の 29 本を同じ `diff` に掛けた。

```
git clone https://github.com/ponapalt/StayseeBalloon.git <一時領域>
diff upstream.sha256 naive.sha256
```

結果: `LICENSE` の 1 行だけが食い違い、値は較正 A と同じ `f4e7f373…` だった。＝この機械で素の clone を使うと `LICENSE` が確実に壊れること、およびこの突合がその壊れ方を捕まえることの両方が実測で示された。残り 28 本が食い違わないのは、`.txt` 3 本はもともと CRLF で、PNG／PNR 25 本は git が 2 進と判定して変換しないため。

---

## 4. 保管の検査（要件 2.4・2.5）

### 4.1 改行変換が掛かっていないこと ⑴ ── 属性照会

```
git ls-files -z vendors/sample_ghost/StayseeBalloon/ | xargs -0 git check-attr text --
```

| 項目 | 期待値 | 実測値 | 判定 |
|---|---|---|---|
| 出力の行数（＝照会した本数） | 29 | 29 | 一致 |
| `text: unset` の行数 | 29 | 29 | 一致 |
| `unset` 以外の行数 | 0 | **0** | 一致 |

`unset` 以外の行数は `awk '$NF!="unset"' <出力> | wc -l` で数えた（`grep -c` は 0 件のとき非 0 終了して判定を汚すので使っていない）。`unset` は `vendors/sample_ghost/.gitattributes` の `* -text` が効いている状態であり、この属性が付いている限り git は追加時にも取り出し時にも改行を書き換えない。

### 4.2 改行変換が掛かっていないこと ⑵ ── 索引の改行表示

```
git ls-files --eol -- vendors/sample_ghost/StayseeBalloon/
```

この照会は**索引（git の index）に入っているファイルにしか答えない**。29 本は先行タスクのコミット `0abc746e` で索引に入っているのでそのまま採れる。

| 群 | 本数 | `i/`（索引側の改行） | `w/`（作業木側） | `attr/` | ファイル |
|---|---|---|---|---|---|
| 2 進 | 25 | `-text` | `-text` | `-text` | `arrow0/1.png`・`balloonc0〜4.png`・`balloonk0/1.png`・`balloons0〜3.png`・`marker.png`・`online0〜8.png`・`sstp.png`・`thumbnail.pnr` |
| CRLF のテキスト | 3 | `crlf` | `crlf` | `-text` | `descript.txt`・`install.txt`・`readme.txt` |
| LF のテキスト | 1 | `lf` | `lf` | `-text` | `LICENSE` |
| 合計 | 29 | — | — | 全 29 本が `-text` | — |

**この表の読み方**: `attr/-text` は 29 本すべてに付いており、§4.1 の属性照会と一致する。`i/` の欄は索引に実際に入っているバイト列の改行であって、`-text` が効いていなければこうはならない。`core.autocrlf=true` のこの機械で `-text` が無ければ、`descript.txt`・`install.txt`・`readme.txt` は索引では LF に正規化されて `i/lf` と表示されるはずである。実測は `i/crlf` なので、**配布時の CRLF がそのまま索引に入っている**＝変換は 1 本も起きていない。`LICENSE` が `i/lf` なのも同じ理屈で、上流の LF がそのまま入っていることを示す（§3.2 で上流 blob とハッシュが一致することは既に確かめた）。

配布時の改行を直接数えた結果も添える（`od -An -v -tu1 <file> | tr ' ' '\n'` で値 13＝CR の個数を数え、`tr -cd` で同じ値を採り直した。Git Bash の `grep` は入力の CR を自分で落とすことがあるので、この数えには使っていない）。

| ファイル | バイト数 | CR の個数 | LF の個数 | 改行の形 |
|---|---|---|---|---|
| `LICENSE` | 7,048 | **0** | 121 | LF のみ |
| `descript.txt` | 1,323 | 69 | 69 | CRLF |
| `install.txt` | 95 | 4 | 4 | CRLF |
| `readme.txt` | 937 | 32 | 32 | CRLF |

（`LICENSE` の CR が 0 であることは `od -c` でも確かめた。先頭は `Creative Commons Legal Code\n\nCC0 1.0 Universal\n\n` で、CR を挟まない。）

### 4.3 除外規則との照合（要件 2.5）

リポジトリ直下の `.gitignore` の該当規則は 2 行である。

```
.gitignore:6:*_test.txt
.gitignore:7:*_dump.txt
```

**照合 A（名前の突合・大小無視）**: 29 本の名前を小文字化して 2 つの規則に当てた。

```
git ls-files vendors/sample_ghost/StayseeBalloon/ | xargs -n1 basename | tr 'A-Z' 'a-z' \
  | awk '/_test\.txt$/ || /_dump\.txt$/ {print}'
```

| 項目 | 期待値 | 実測値 | 判定 |
|---|---|---|---|
| 照合した名前の数（母数） | 29 | 29 | 一致 |
| 当たった名前 | 0 件 | **0 件**（出力は空） | 一致 |

母数を先に示したのは、「名前が 0 個の一覧に当たりが 0 件」という恒真の緑を避けるため。小文字化してから当てているので、`README_TEST.TXT` のような大文字綴りも取りこぼさない。

**照合 B（git 自身の判定）**:

```
git ls-files vendors/sample_ghost/StayseeBalloon/ | git check-ignore --stdin
```

| 項目 | 期待値 | 実測値 | 判定 |
|---|---|---|---|
| 除外対象と判定された本数 | 0 | **0**（出力 0 行・exit 1） | 一致 |

`git check-ignore` は 1 件も当たらないと exit 1 を返す。ここでは exit 1 が「当たり 0 件」の正しい姿である。

**この 0 が意味を持つ理由**: 当たる名前があった場合、リポジトリ直下の規則が資産を追跡対象から落とし、保管が 1 本欠けたまま緑に見えてしまう（`vendors/sample_ghost/.gitignore` の打ち消し `!*_test.txt`／`!*_dump.txt` が拾い直す仕掛けは在るが、それが効いているかどうかは別に確かめる必要がある）。当たり 0 件なら、この検体に関しては打ち消しに頼らずに 29 本が素直に追跡されている、と言い切れる。

### 4.4 除外の検査が恒真でないことの較正

| 較正 | 打った命令 | 期待 | 実測 | 意味 |
|---|---|---|---|---|
| A'（名前の突合） | 29 本の一覧に `balloon_test.txt` と `x_dump.txt` を足して同じ `awk` に掛ける | 2 件当たる | **2 件**（両方出力） | 照合 A は当たりを検出できる＝0 件は恒真でない |
| B1（git の判定・規則が効く側） | `printf 'foo_test.txt\nfoo_dump.txt\n' \| git check-ignore --stdin -v` | 2 件当たる | **2 件**（`.gitignore:6:*_test.txt`・`.gitignore:7:*_dump.txt` を報告・exit 0） | 規則そのものは生きている |
| B2（打ち消しが効く側） | `printf 'vendors/sample_ghost/StayseeBalloon/foo_test.txt\n' \| git check-ignore --stdin` | 当たらない | **0 行・exit 1**（`-v` を付けると `vendors/sample_ghost/.gitignore:5:!*_test.txt` を報告） | 保管フォルダの中では打ち消しが効く＝万一当たる名前が来ても落ちない |

B2 は `-v` の有無で見え方が変わる（`-v` は打ち消し規則に当たったことも報告して exit 0 を返す）。判定に使うのは `-v` **なし**の形である。

### 4.5 上流資料の記述との差異（申し送り）

`research.md` §8.2 は `LICENSE` について括弧書きで CRLF と記しているが、§4.2 の実測は **CR 0・LF 121 の LF のみ**である。§3.2 で上流の blob と保管のハッシュが一致していることから、上流の `LICENSE` が LF であることも確定している。本タスクは `research.md` を変更対象に含まないので是正していない。この記録の値（LF のみ）が実測であり、以降の判断はこちらを採ること。

---

## 5. 要件 2.6 の判定（食い違い突合）

要件 2.6 は「取得した配布物の中身が本文の候補の記述（ファイル一覧・descript の要点・LICENSE の種別）と食い違うなら、本文を是正してから保管し、食い違いを記録に書く」という条件節である。3 観点それぞれを機械で突き合わせた。

### 5.1 観点⑴ ファイル一覧

`requirements.md` の Requirement 2.2 が列挙する一覧（`descript.txt`・`install.txt`・`readme.txt`・`LICENSE`・`thumbnail.pnr`・`balloons0〜3.png`・`balloonk0〜1.png`・`balloonc0〜4.png`・`arrow0/1.png`・`online0〜8.png`・`marker.png`・`sstp.png`）を 1 本ずつ展開すると 29 本になる。これを保管の実測と `diff` した。

| 項目 | 期待値 | 実測値 | 判定 |
|---|---|---|---|
| 要件の一覧を展開した本数 | 29 | 29 | 一致 |
| 保管の本数 | 29 | 29 | 一致 |
| `diff` の差分行数 | 0 | **0** | 一致（欠落 0・余分 0） |

較正: 期待の一覧から `marker.png` を 1 本落として同じ `diff` に掛けると `> marker.png` の 1 行が出て非 0 終了した＝この突合は 1 本の差を検出できる。

### 5.2 観点⑵ descript の要点

`requirements.md` の Introduction「候補（2026-09-18 調査）」が挙げる要点を、復号した `descript.txt` に対し 1 行完全一致で数えた（`awk -v k=… '$0==k{c++}END{print c+0}'`）。

| 鍵と値 | 期待 | 実測 | 判定 |
|---|---|---|---|
| `charset,Shift_JIS` | 1 | 1 | 一致 |
| `type,balloon` | 1 | 1 | 一致 |
| `name,Balloon for Staysee Syncfield` | 1 | 1 | 一致 |
| `id,StayseeBalloon` | 1 | 1 | 一致 |
| `craftman,SSP BUGTRAQ` | 1 | 1 | 一致 |
| `use_self_alpha,1` | 1 | 1 | 一致 |
| `use_input_alpha,1` | 1 | 1 | 一致 |
| `paint_transparent_region_black,0` | 1 | 1 | 一致 |
| `validrect.left,22` | 1 | 1 | 一致 |
| `validrect.top,20` | 1 | 1 | 一致 |
| `validrect.right,-26` | 1 | 1 | 一致 |
| `validrect.bottom,-47` | 1 | 1 | 一致 |
| `font.height,12` | 1 | 1 | 一致 |

要件本文が「宣言が無い」と書いている 3 つも数えた（行頭一致で数え、**0 件であること**を確かめた。0 を書かずに黙っていると「調べていない」と区別がつかないので明示する）。

| 鍵 | 期待 | 実測 | 判定 |
|---|---|---|---|
| `vertical`（行頭） | 0 | **0** | 一致（横書き扱いへ縮退する前提が成り立つ） |
| `wordwrappoint`（行頭） | 0 | **0** | 一致（折返し基準は `validrect` の遠辺へ縮退する前提が成り立つ） |
| `font.name`（行頭） | 0 | **0** | 一致（既定書体へ縮退する前提が成り立つ） |

較正: 同じ数え方（行頭一致）で `font.height` を数えると 1 件出る＝「0 件」は数え方が壊れて全部 0 になったせいではない。

**参考（要件が名指ししていない鍵）**: `descript.txt` の鍵は重複を除いて **57 種**あり、うち要件本文が名指ししているのは上表の 13 種と `homeurl`（要件 5.1）の計 14 種。残る 43 種は次のとおりで、いずれも要件の「要点」の枠外にあるため 2.6 の食い違いには当たらない。

`anchor.font.color.r/g/b`・`arrow0.x/y`・`arrow1.x/y`・`communicatebox.x/y/width/height`・`craftmanurl`・`craftmanw`・`cursor.blendmethod`・`cursor.style`・`cursor.brush.color.r/g/b`・`cursor.pen.color.r/g/b`・`cursor.font.color.r/g/b`・`font.color.r/g/b`・`number.font.height`・`number.font.color.r/g/b`・`number.xr`・`number.y`・`onlinemarker.x/y`・`sstpmarker.x/y`・`sstpmessage.x/y`・`sstpmessage.font.height`・`sstpmessage.font.color.r/g/b`

### 5.3 観点⑶ LICENSE の種別

| 項目 | 期待値 | 実測値 | 判定 |
|---|---|---|---|
| `LICENSE` の種別 | CC0-1.0 | 1〜3 行目が `Creative Commons Legal Code` / 空行 / `CC0 1.0 Universal`＝CC0 1.0 の法典本文 | 一致 |
| `readme.txt` の記載 | CC0 | 「License : CC0 https://creativecommons.org/publicdomain/zero/1.0/deed.ja」 | 一致 |

### 5.4 判定

**3 観点すべて一致。要件 2.6 の条件節は発動しない。** したがって `requirements.md` の本文は 1 文字も是正していない（このタスクでの同ファイルの変更は 0 行）。

---

## 6. 既定バルーン id（要件 7.1）

| 綴りの出どころ | 値 |
|---|---|
| `descript.txt` の `id` 行 | `StayseeBalloon` |
| `install.txt` の `directory` 行（インストール時の配置先） | `StayseeBalloon` |
| リポジトリに置いた実フォルダ名 | `StayseeBalloon` |
| git の索引に登録されている綴り | `vendors/sample_ghost/StayseeBalloon/…`（`git ls-files` の出力そのまま） |

**判定: 3 つとも同綴り（大小を含めてバイト一致）。既定バルーン id ＝ `StayseeBalloon`。**

採り方（2 つの値を復号して取り出し、実フォルダ名と `=` で比べた）:

```
ID=$(iconv -f CP932 -t UTF-8 descript.txt | tr -d '\r' | awk -F, '/^id,/{print $2}')
DIR=$(iconv -f CP932 -t UTF-8 install.txt | tr -d '\r' | awk -F, '/^directory,/{print $2}')
FOLDER=$(basename "$PWD")
[ "$ID" = "$DIR" ] && [ "$DIR" = "$FOLDER" ]
```

較正: 同じ比較に小文字綴り `stayseeballoon` を与えると不一致になる＝この判定は大小を区別している（`StayseeBalloon` と `stayseeballoon` を同じものとして緑にしてしまう比較ではない）。Windows のファイルシステムは大小を区別しないが、git の索引は綴りをそのまま保持しており、`.nar` へ畳む側・配布 zip を作る側はこの綴りをそのまま使えばよい。

---

## 付録: このタスクで触ったファイル

新規作成はこの `verification/provenance.md` 1 本のみ。`vendors/sample_ghost/StayseeBalloon/` は読むだけで変更していない（検査の前後で `git status --porcelain` が空・`git diff --stat -- vendors/sample_ghost/StayseeBalloon/` が空）。上流の複製と較正用の写しはリポジトリ外の一時領域に作り、比較後に残していない。
