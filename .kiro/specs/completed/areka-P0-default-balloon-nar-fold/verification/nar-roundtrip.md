# `StayseeBalloon.nar` の往復照合の記録（要件 1.4・2.4）

- 日付: 2026-09-23
- 場所: 本 spec のブランチ（作業開始時の HEAD `a9be1797`）。タスク 1 の作業中、展開フォルダ `vendors/sample_ghost/StayseeBalloon/` を消す**前**に採った。
- 性格: **一度きり**の照合（2026-09-23 裁定＝29 本の sha256 を常設のテストでは固定しない）。以後の中身の保証は結合テスト `staysee_balloon_fixture_test` の 29 ファイル名＋画像の縦横が持つ。

## 0. 結論（先に）

- 畳む道具の往復照合: **不一致 0 件**・終了コード 0（§1）。
- `nar-sample-path -- StayseeBalloon`: `root=`／`folder=` の 2 行を印字し**終了コード 0**（§2＝要件 2.4 の直接の証跡）。
- `folder=` の 29 本と完了 spec `areka-P0-default-balloon-bundle` の provenance §3.1: **29 本すべて一致・不一致 0 件**（ファイル名・バイト数・sha256。§3）。
- `vendors/sample_ghost/StayseeBalloon.nar` の改行変換: `text: unset`（§4）。

## 1. 畳む（`fold-samples`）

命令（展開フォルダがまだ在る状態で 1 回）:

```
cargo run -q -p sample-ghost-kit --example fold-samples -- --from vendors/sample_ghost/StayseeBalloon
```

印字（逐語）:

```
StayseeBalloon: 追跡 29 ファイル / 展開 29 ファイル / balloon/StayseeBalloon=29 / .nar 73021 バイト / 不一致 0 件
全ての検体で一致
```

終了コード: `0`。生成物 `vendors/sample_ghost/StayseeBalloon.nar` は 73,021 バイト（`ls -l` で同値を確認）。

## 2. 検体のパスを印字する bin（要件 2.4）

命令（`SAMPLES` に `StayseeBalloon` の 1 行を足した後）:

```
cargo run -q -p sample-ghost-kit --bin nar-sample-path -- StayseeBalloon
```

標準出力（逐語）:

```
root=C:\home\maz\git\areka\.claude\worktrees\areka-p0-default-balloon-nar-9bdcb1\target\nar-samples\manual\StayseeBalloon
folder=C:\home\maz\git\areka\.claude\worktrees\areka-p0-default-balloon-nar-9bdcb1\target\nar-samples\manual\StayseeBalloon\balloon\StayseeBalloon
```

標準エラー: 0 行（ビルドの印字を除く）。終了コード: `0`。

この bin は使い捨ての複製ではなく、プロセスが終わっても残る `target/nar-samples/manual/<名>/` へ配る（bin の module doc「配る先」）。だから bin が終わった後に `folder=` の 29 本を読めた。

## 3. `folder=` の 29 本と provenance §3.1 の突き合わせ（要件 1.4）

命令（Git Bash。`F` は §2 の `folder=` をリポジトリ根からの相対で書いたもの）:

```
F=target/nar-samples/manual/StayseeBalloon/balloon/StayseeBalloon

# 期待値: 完了 spec の provenance §3.1 の表から「ファイル名 バイト数 sha256」を抜く
grep -E '^\| [0-9]+ \| `' .kiro/specs/completed/areka-P0-default-balloon-bundle/verification/provenance.md \
  | awk -F'|' '{gsub(/[` ,]/,"",$3);gsub(/[` ,]/,"",$4);gsub(/[` ]/,"",$5);print $3, $4, $5}' > expected.txt

# 実測: 展開結果の全ファイルについて「ファイル名 バイト数 sha256」
(cd $F && find . -type f | sed 's|^\./||' | LC_ALL=C sort | while read f; do
  printf '%s %s %s\n' "$f" "$(stat -c %s "$f")" "$(sha256sum "$f" | cut -d' ' -f1)"; done) > actual.txt

diff <(LC_ALL=C sort expected.txt) <(LC_ALL=C sort actual.txt) && echo "diff: 0 lines"
find $F -mindepth 1 -type d | wc -l
```

結果: 期待値 29 行・実測 29 行・`diff` の出力 0 行（`diff: 0 lines`）・`folder=` の下のサブフォルダ 0 個。

| # | ファイル | バイト数（両者一致） | 展開結果の sha256 | provenance §3.1 の sha256 | 判定 |
|---|---|---|---|---|---|
| 1 | `LICENSE` | 7048 | `a2010f343487d3f7618affe54f789f5487602331c0a8d03f49e9a7c547cf0499` | `a2010f343487d3f7618affe54f789f5487602331c0a8d03f49e9a7c547cf0499` | 一致 |
| 2 | `arrow0.png` | 284 | `ee13c9104c00fcc787c7908aa83bfe08330b5bde416f85ab78fd0b12cc1e58be` | `ee13c9104c00fcc787c7908aa83bfe08330b5bde416f85ab78fd0b12cc1e58be` | 一致 |
| 3 | `arrow1.png` | 297 | `12c1ec8f61ea09f659879b3a4ca1efdc44db6abdcaa07740e55db97b6011bd45` | `12c1ec8f61ea09f659879b3a4ca1efdc44db6abdcaa07740e55db97b6011bd45` | 一致 |
| 4 | `balloonc0.png` | 3616 | `751f85ee947fb3958709bca3f76c4d3074bd005ba1753e8da11b9d7c0e24dbc4` | `751f85ee947fb3958709bca3f76c4d3074bd005ba1753e8da11b9d7c0e24dbc4` | 一致 |
| 5 | `balloonc1.png` | 4102 | `90cde84d2867875787cfb2bbed7b6842d7a2fb1c0db37eb96239db4008fe6a87` | `90cde84d2867875787cfb2bbed7b6842d7a2fb1c0db37eb96239db4008fe6a87` | 一致 |
| 6 | `balloonc2.png` | 3552 | `30f57a9e2a43ef83e12ec48574b537dda60b310db8f85d8855c10d070d6670a5` | `30f57a9e2a43ef83e12ec48574b537dda60b310db8f85d8855c10d070d6670a5` | 一致 |
| 7 | `balloonc3.png` | 3476 | `0a286f8a7ed1de42d82670c00169bb54cbd1f29f712e40059d78b0c4934221ef` | `0a286f8a7ed1de42d82670c00169bb54cbd1f29f712e40059d78b0c4934221ef` | 一致 |
| 8 | `balloonc4.png` | 3793 | `af1d3ad9e34addd00963dd4455da1c9110b306a44a45d3bd3b5ff6f8a3ca5cb2` | `af1d3ad9e34addd00963dd4455da1c9110b306a44a45d3bd3b5ff6f8a3ca5cb2` | 一致 |
| 9 | `balloonk0.png` | 4780 | `3b1874e2d35811ac96bd878f6acb9464a9c848dbeffede645f379086f47bc42b` | `3b1874e2d35811ac96bd878f6acb9464a9c848dbeffede645f379086f47bc42b` | 一致 |
| 10 | `balloonk1.png` | 4773 | `70eb76af5878effb21e3911586f1283b09f81b4dfa72e0dad7a74782f1ab4869` | `70eb76af5878effb21e3911586f1283b09f81b4dfa72e0dad7a74782f1ab4869` | 一致 |
| 11 | `balloons0.png` | 5117 | `0fc496e0620de4cf1f6277f854653ce1ffde54d05cf5ea2e400a987ed80fab61` | `0fc496e0620de4cf1f6277f854653ce1ffde54d05cf5ea2e400a987ed80fab61` | 一致 |
| 12 | `balloons1.png` | 5128 | `d819cd4116d17a4c553e326f480d90ce4155c417b79c768998fa688ba8f3a198` | `d819cd4116d17a4c553e326f480d90ce4155c417b79c768998fa688ba8f3a198` | 一致 |
| 13 | `balloons2.png` | 5891 | `8b6c84abbadbb7b29c1aa6c291636763eb9cf24db0367107b9e1825f64214844` | `8b6c84abbadbb7b29c1aa6c291636763eb9cf24db0367107b9e1825f64214844` | 一致 |
| 14 | `balloons3.png` | 5917 | `3293c92931550f79be8f9da9357751aefd9aaa956496c4edf4cc45b6fe199b51` | `3293c92931550f79be8f9da9357751aefd9aaa956496c4edf4cc45b6fe199b51` | 一致 |
| 15 | `descript.txt` | 1323 | `6696b4eb6c127b1c530a668c8853f14b6e0356b0488cf8589d6e7d14a3126333` | `6696b4eb6c127b1c530a668c8853f14b6e0356b0488cf8589d6e7d14a3126333` | 一致 |
| 16 | `install.txt` | 95 | `aad39c0b4a6962ad1c1865d97fd6fd9553871f5b22ba4050ca5081b52ee253ec` | `aad39c0b4a6962ad1c1865d97fd6fd9553871f5b22ba4050ca5081b52ee253ec` | 一致 |
| 17 | `marker.png` | 368 | `2403c0a5f3bfa526fe4a2809b681706f614e858a2f348e36740c41802fa6a272` | `2403c0a5f3bfa526fe4a2809b681706f614e858a2f348e36740c41802fa6a272` | 一致 |
| 18 | `online0.png` | 479 | `c4b6ca314a71b611d35abd3092ee6429785dd4c5d616bf57436346fcc5cdb540` | `c4b6ca314a71b611d35abd3092ee6429785dd4c5d616bf57436346fcc5cdb540` | 一致 |
| 19 | `online1.png` | 567 | `3d7a26f1166641f1080d619a7c37c5c74be1ee253598865649545a1d05adc59d` | `3d7a26f1166641f1080d619a7c37c5c74be1ee253598865649545a1d05adc59d` | 一致 |
| 20 | `online2.png` | 749 | `e3431100072c42816e6689db23074cd6818134975fe930f6b5e23d2854298d64` | `e3431100072c42816e6689db23074cd6818134975fe930f6b5e23d2854298d64` | 一致 |
| 21 | `online3.png` | 792 | `83c5bd8fbe258d99f96cd9c041777fac14c5866bf37a12bf34b885657b426f69` | `83c5bd8fbe258d99f96cd9c041777fac14c5866bf37a12bf34b885657b426f69` | 一致 |
| 22 | `online4.png` | 859 | `391f4cfed5d5039f4ba4a3e2e82f033109b23606e1fbf0b978da2ee3a9ac52e3` | `391f4cfed5d5039f4ba4a3e2e82f033109b23606e1fbf0b978da2ee3a9ac52e3` | 一致 |
| 23 | `online5.png` | 964 | `242af30a28eef57c1ecfb85be0a0c7db16bc0086dbf1757a1473f271b11714cf` | `242af30a28eef57c1ecfb85be0a0c7db16bc0086dbf1757a1473f271b11714cf` | 一致 |
| 24 | `online6.png` | 945 | `471401e32df21f330cdfa82ae349c4756b71eed13cede9db831f9d714b5e585e` | `471401e32df21f330cdfa82ae349c4756b71eed13cede9db831f9d714b5e585e` | 一致 |
| 25 | `online7.png` | 1092 | `dff4ef1f4747c3087fa00546608bd86cf969a1b841fb8d52d2e5df6590c54db6` | `dff4ef1f4747c3087fa00546608bd86cf969a1b841fb8d52d2e5df6590c54db6` | 一致 |
| 26 | `online8.png` | 1097 | `dd9a3ce63a2a77d3b8f498a2c96aae6dca2177ef85d4fd1aa28ea993d35fb3a0` | `dd9a3ce63a2a77d3b8f498a2c96aae6dca2177ef85d4fd1aa28ea993d35fb3a0` | 一致 |
| 27 | `readme.txt` | 937 | `ff58764261050187d13cf780fa8171ac1f9e7fd2e004079120db8e3f9f016144` | `ff58764261050187d13cf780fa8171ac1f9e7fd2e004079120db8e3f9f016144` | 一致 |
| 28 | `sstp.png` | 189 | `d2aec88d4c36368459efaa3715670257ab86ef83538940384e4d21deed7c38b2` | `d2aec88d4c36368459efaa3715670257ab86ef83538940384e4d21deed7c38b2` | 一致 |
| 29 | `thumbnail.pnr` | 1899 | `56122ce24e900819b1079dac51ad5fbcd3063ff280eea2c975c1d9a6efed9621` | `56122ce24e900819b1079dac51ad5fbcd3063ff280eea2c975c1d9a6efed9621` | 一致 |

一致 **29 本**・不一致 **0 件**・片側にしか無いファイル **0 本**（`diff` が 0 行＝名前の集合も一致）。`install.txt` も展開先に残っている（`type,balloon` では最上位の `install.txt` が置かれる）。

### 3.1 この突き合わせが恒真でないことの較正

期待値の 15 行目（`descript.txt`）の sha256 の 8 文字目を `b`→`c` に 1 文字だけ変えた写しで同じ `diff` を走らせると、`15c15` の差分 1 組を出して終了コード `1` になった。

```
sed '15s/6696b4eb/6696b4ec/' expected.txt > expected_mut.txt
diff <(LC_ALL=C sort expected_mut.txt) <(LC_ALL=C sort actual.txt); echo "diff exit=$?"
```

```
15c15
< descript.txt 1323 6696b4ec6c127b1c530a668c8853f14b6e0356b0488cf8589d6e7d14a3126333
---
> descript.txt 1323 6696b4eb6c127b1c530a668c8853f14b6e0356b0488cf8589d6e7d14a3126333
diff exit=1
```

＝1 本 1 文字の違いでも `diff` は 0 行にならない。上の「0 行」は比べた結果であって、比べ損ねた結果ではない。

## 4. 改行変換が掛からないこと

```
git check-attr text -- vendors/sample_ghost/StayseeBalloon.nar
```

```
vendors/sample_ghost/StayseeBalloon.nar: text: unset
```

`vendors/sample_ghost/.gitattributes` の `* -text` が効いている。
