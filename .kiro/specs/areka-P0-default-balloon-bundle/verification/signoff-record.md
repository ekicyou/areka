# 裁定と検証の記録（signoff-record）

| 項目 | 値 |
|---|---|
| 対象仕様 | `areka-P0-default-balloon-bundle` |
| 本節（§1）の対象要件 | 1.1・1.2・1.3・1.4 |
| 実施日 | 2026-09-18 |
| 実施した作業木 | `C:\home\maz\git\areka\.claude\worktrees\areka-p0-balloon-bundle-6fe55e`（ブランチ `claude/areka-p0-balloon-bundle-6fe55e`） |
| §1 記入時の HEAD | `cef8f60d99e919333cf70fa7c208b44b96b162ec` |

この記録は「既定バルーンを `StayseeBalloon` に決めた判断と、その理由（採った理由・採らなかった理由・見た目をそのまま受け入れる理由）が、追加の会話なしに後から読んで分かる」ことを目的とする。取得元・ハッシュ・保管の検査といった**出典の事実**は隣の `provenance.md` が正本なので、ここでは繰り返さず参照する。

本文の値はすべて本ブランチで採り直した実測であり、要件書や先行タスクの報告を写したものではない。各値には**実際に打った命令**を添えた。命令はすべて作業木の根から打っている。

## この記録の埋まり具合

この記録は 8 節から成る。本タスク（1.4）が埋めたのは **§1 だけ**である。§2〜§8 は見出しだけを置いてあり、それぞれの節に書いてある担当タスクが後で埋める。

---

## 0. 結論（先に）

| # | 主張 | 根拠の所在 | 判定 |
|---|---|---|---|
| 1 | 既定バルーンは `StayseeBalloon` に確定している（開発者裁定 2026-09-18） | §1.1 | 確定 |
| 2 | 再配布条件は `LICENSE` 本文と `readme.txt` の**両方**で確認できる | §1.2 ⑴ | 両方で確認 |
| 3 | 特定ゴースト専用の指定は無い | §1.2 ⑵ | 指定なし |
| 4 | 原作者の唯一の注意事項（半透明前提）は areka の固定の扱いと一致する | §1.2 ⑶ | 一致 |
| 5 | 他候補を採らない理由は書き残されている | §1.3 | 4 群すべて記載 |
| 6 | 文字は ukadoc 既定の書体で描かれ、それは正典どおりの挙動である | §1.4 | 正典どおり |
| 7 | 上の 6 を守るために areka 側の本番コードもバルーン側のファイルも変えない | §1.4 | 変更 0 |

---

## 1. 裁定と選定根拠（要件 1.1・1.2・1.3・1.4）

### 1.1 裁定（要件 1.2）

| 項目 | 内容 |
|---|---|
| 日付 | 2026-09-18 |
| 裁定者 | 開発者 |
| 結論 | 「ukadoc 準拠とし、StayseeBalloon をそのまま採用する」 |
| 対象資産 | `Balloon for Staysee Syncfield`（id `StayseeBalloon`・作者「ぽな（ばぐとら研究所/整備班）」・ライセンス CC0-1.0） |
| 根拠 | ⑴ 再配布条件が `LICENSE` 本文と `readme.txt` の両方で確認できる ⑵ 特定ゴースト専用の指定が無い ⑶ 原作者が明記した前提（半透明）が areka の固定の扱いと一致する |

この裁定により、既定バルーンの候補選びはここで閉じる。以降（要件 2 以降）は `StayseeBalloon` を前提に進め、実機での見た目の確認（要件 4）は**採否を覆すための関門ではなく品質確認**として行う。見た目に崩れが出たときの行き先は要件 3.9／4.3（バルーンを改変して合わせるのではなく areka 側の欠陥として扱う）であり、§4 に記録する。

裁定に至るまでの議論そのものは `brief.md` の「議論の記録（2026-09-18・開発者と Claudia）」に残っている。本節はその結論と、結論を支える事実の裏取りである。

関連する別の開発者裁定（同日・本仕様の前提として効いている）:

- 「**areka は常に `use_self_alpha,1`。pna 対応は不要**」——§1.2 ⑶ の一致判定の前提。
- 「**同梱バルーンは癖が無いものが必要。`emo2-kakukaku` では厳しい**」——§1.3 の不採用理由。

### 1.2 候補を選んだ根拠（要件 1.1）

3 点とも保管済みの実物（`vendors/sample_ghost/StayseeBalloon/`）と areka の実コードで裏を取った。

#### ⑴ 再配布条件が `LICENSE` 本文と `readme.txt` の両方で確認できる

| 出所 | 読んだもの | 命令 |
|---|---|---|
| `LICENSE`（ASCII・復号不要） | 1 行目 `Creative Commons Legal Code`・3 行目 `CC0 1.0 Universal` | `head -6 vendors/sample_ghost/StayseeBalloon/LICENSE` |
| `readme.txt`（Shift_JIS＝CP932） | 見出し「■転載・再配布・同梱・改変等について」、本文「煮るなり焼くなり好きにしてください。」、次行「License : CC0 https://creativecommons.org/publicdomain/zero/1.0/deed.ja」 | `iconv -f CP932 -t UTF-8 vendors/sample_ghost/StayseeBalloon/readme.txt` |

**両方で確認できることを根拠に据えた理由**: 法文（`LICENSE`）だけなら「作者が意図して置いたのか、雛形が紛れ込んだのか」が読み取れない。作者の言葉（`readme.txt`）だけなら「CC0」が正確な意味で使われているか分からない。二つが同じ結論を指しているので、再配布・同梱の可否は推測ではなく書かれた事実として扱える。同梱して配る側（areka）にとってはこの一致が最も重い。

なお CC0 は帰属表示を義務づけないが、出典と作者名は礼儀として記録し配布物にも残す（要件 5・`provenance.md` §1 と本記録 §6）。

#### ⑵ 特定ゴースト専用の指定が無い

`readme.txt`（復号後）に次の 2 行がある。

- 「特定のゴーストを意識して作ったバルーンですが、専用指定はしていません。」
- 「デザイン的には極端な特化はしていないので、なにか流用できるかもしれません。」

既定バルーンは「どのゴーストが入ってきても、まずこれで喋れる」ための資産なので、特定のゴースト向けという但し書きが付いていないことが要る。原作者自身が流用を想定していると書いているので、用途の面でも借り物にならない。

#### ⑶ 半透明前提が areka の固定の扱いと一致する

原作者が挙げた注意事項は 1 つだけである（`readme.txt` 復号後）。

- 「■！！！注意事項！！！」「半透明（アルファチャンネル）ONを前提に作っており、OFFではまともに見られません。」

areka はこの設定を**バルーンの宣言から読まない**。常に画像のアルファをそのまま尊重する側に固定してある。実在を次の 4 か所で確かめた。

| # | 場所（何の定義か） | 何が書いてあるか |
|---|---|---|
| a | `crates/areka-emo-present/src/balloon.rs` のモジュール doc（「正典整理」の「PNG α 尊重」の項） | 「`use_self_alpha,1` 相当＝`UseSelfAlpha::On` で bake する」「`.pna` 対応は `ElementDecoder::probe_pna` の既存 seam に委ね本 spec では追加しない」 |
| b | 同ファイルの `build_balloon_target_from_faces` の中で `SurfaceSet` を組む箇所 | `alpha_params` に `use_self_alpha: UseSelfAlpha::On` を**定数で**渡す（バルーンの宣言値を参照する引数は無い） |
| c | `crates/areka/src/emo2_boot/assets.rs` の `build_boot_assets`（起動時の焼き込み） | シェル側の `SurfaceSet` にも同じ `use_self_alpha: UseSelfAlpha::On` を定数で渡す。バルーン側はこの関数から `build_balloon_target_from_faces` へ委ねるので、経路のどちらを通っても固定値になる |
| d | `crates/areka-parsers/src/balloon/parse.rs`（バルーン定義の読み手） | `use_self_alpha`／`use_input_alpha`／`paint_transparent_region_black` の 3 つの鍵を**引かない** |

d の「引かない」は 0 件の主張なので、数え方と較正を添える。

```
# 判定: バルーン定義の読み手が 3 鍵に触れているか
grep -n "use_self_alpha\|use_input_alpha\|paint_transparent_region_black" \
     crates/areka-parsers/src/balloon/parse.rs | wc -l
# → 0

# 判定: crates/ 全体で 3 鍵を「設定ファイルの鍵の綴り」として引いている箇所
grep -rn "\"use_self_alpha\"\|\"use_input_alpha\"\|\"paint_transparent_region_black\"" \
     crates/ --include=*.rs | wc -l
# → 0

# 較正: 同じ数え方で、読み手が実際に引いている鍵を数える（0 でないことを確かめる）
grep -on "\"validrect\.\|\"wordwrappoint\|\"font\.\|\"vertical\"\|\"origin\." \
     crates/areka-parsers/src/balloon/parse.rs
# → origin. / wordwrappoint / validrect. / font. などが多数ヒット（0 ではない）
```

較正の意味: 同じファイルに同じやり方を当てて `origin.`・`wordwrappoint`・`validrect.`・`font.` は拾えている。したがって上の 0 件は「探し方が壊れていて何も拾えなかった」のではなく「本当に引いていない」である。

補足として、`crates/` を `use_self_alpha` の語だけで検索すると 60 行が当たる（`grep -rn "use_self_alpha" crates/ --include=*.rs | wc -l`）。その内訳は ⑴ Rust 側の構造体 `AlphaParams` の項目名（`use_self_alpha: UseSelfAlpha::On`）とその説明、⑵ テストが組み立てるシェル側の定義テキストに現れる `seriko.use_self_alpha,1` の行、の 2 種であり、**バルーンの `descript.txt` の鍵として引いている箇所は 1 つも無い**。それが上の 2 本目の命令の 0 件である。

**一致と言える理由**: areka は宣言に関わらず常にアルファを尊重する。つまり「半透明 ON でないと正しく見られない」バルーンは areka では**必ず**その前提で扱われる。逆に「半透明 OFF 前提」で作られたバルーンは areka では救えない。StayseeBalloon は前者なので、areka の固定の扱いとぶつからない側の資産である。この固定は areka の意図した裁量なので、要件 6 で `doc/COMPAT_ARCHITECTURE.md` §8 と網羅台帳に登記する。

なお StayseeBalloon 自身も `descript.txt` に `use_self_alpha,1` と書いている（`iconv -f CP932 -t UTF-8 vendors/sample_ghost/StayseeBalloon/descript.txt` の `use_self_alpha` 行）。areka はこの行を読まないが、読む・読まないに関わらず結果は同じになる。

### 1.3 他の候補を採らない理由（要件 1.4）

| 候補 | 採らない理由 | 理由の出所 |
|---|---|---|
| SSP 同梱の「SSPデフォルト+」「balloon for Emily/P4」 | **再配布条件が公開されていない**。SSP 公式サイト・SSP ヘルプの FAQ・ukadoc のいずれにも記載が無く、SSP 本体のソースも GitHub には無い（`github.com/ponapalt/ssp` は 404）。条件を知るには SSP の配布 zip に入っている readme を読むか作者に直接尋ねるほかなく、同梱して配る根拠としては薄い。 | `brief.md`「議論の記録」2 の第 1 項（2026-09-18 の調査） |
| `emo2-kakukaku` と派生 2 つ（`emo2-kakukaku-offsetdpi`・`emo2-kakukaku-wplimit`） | **開発者裁定「同梱バルーンは癖が無いものが必要。`emo2-kakukaku` では厳しい」**（2026-09-18）。これらは作者自作の検証用資産であり、特定の観測を固定するために形を尖らせてある。検証用としては今までどおり使い続けるが、第三者が最初に目にする既定にはしない。 | `brief.md`「議論の記録」1（開発者発言） |
| 整備班（`ms.shillest.net`）の他のバルーン 6 つ | 配布条件が配布ページに書かれていない。上の 1 件目と同じ理由で借用の根拠が立たない。 | `brief.md`「議論の記録」2 の第 3 項 |
| 自作の無地バルーン | 採らないのではなく**次善として残す**。CC0 の候補が見た目の理由で不採用になった場合の逃げ道だが、画像を描く手間と `descript.txt` の調整が要り、既製品より高くつく。§1.1 の裁定で CC0 候補が採用されたので、今回は使わない。 | `brief.md`「議論の記録」2 の第 4 項・要件書 Introduction「候補（2026-09-18 調査）」 |

不採用にした `emo2-kakukaku` 系 3 つが実在し、今も検証用として置かれたままであることを確かめた（本仕様はこれらの中身も置き場所も変えない＝要件 2.7）。

```
find . -type d -name "emo2-kakukaku*" -not -path "./.git/*" | sort
# → ./crates/pilot/examples/shiori-host-32/fixtures/emo2-kakukaku-offsetdpi
#   ./crates/pilot/examples/shiori-host-32/fixtures/emo2-kakukaku-wplimit
#   ./crates/pilot/examples/shiori-host-32/fixtures/emo2/emo2-kakukaku
```

### 1.4 見た目は ukadoc 既定の書体のまま受け入れる（要件 1.3）

#### 事実: StayseeBalloon は書体名を宣言していない

```
# 判定（行頭一致）
iconv -f CP932 -t UTF-8 vendors/sample_ghost/StayseeBalloon/descript.txt | grep -c "^font\.name"
# → 0（grep -c は 0 件のとき終了コード 1 を返すので、この 1 は失敗ではない）

# 判定（行頭に限らず、文字列としてどこかに現れるか）
iconv -f CP932 -t UTF-8 vendors/sample_ghost/StayseeBalloon/descript.txt | grep -c "font\.name"
# → 0

# 較正: 同じ数え方で font. で始まる行を数える
iconv -f CP932 -t UTF-8 vendors/sample_ghost/StayseeBalloon/descript.txt | grep -c "^font\."
# → 4（font.height,12 / font.color.r,0 / font.color.g,40 / font.color.b,100）

# 較正: 復号そのものが成立していること
iconv -f CP932 -t UTF-8 vendors/sample_ghost/StayseeBalloon/descript.txt | wc -l
# → 69 行
```

較正の意味: 同じ命令で `font.` の行は 4 本拾えており、復号も 69 行成立している。したがって `font.name` の 0 件は「復号に失敗して空を数えた」のでも「綴りを間違えて探した」のでもなく、**本当に宣言が無い**。

あわせて、同じやり方で次の 2 つも 0 件だった（いずれも要件 3.3 が「未指定として扱う」と定めているもの）。

| 鍵 | 件数 | 未指定のときの areka の扱い |
|---|---|---|
| `font.name` | 0 件 | 既定の書体へ縮退する |
| `vertical` | 0 件 | 横書きとして扱う |
| `wordwrappoint` | 0 件 | 折返し基準は `validrect` の遠辺へ縮退する |

これら 3 つの 0 件はいずれも欠陥ではない。ukadoc が既定を定めている鍵であり、areka にも既定へ落ちる道が既に通っている。

#### areka 側の既定書体が実在すること

| 場所（何の定義か） | 内容 |
|---|---|
| `crates/areka-emo-text/src/draw.rs` の `DEFAULT_FONT_NAME` の定義 | `pub const DEFAULT_FONT_NAME: &str = "ＭＳ ゴシック";` |
| 同ファイルの `ResolvedFont` の解決手順の説明（`font.name` の縮退を述べた行） | 「`font.name` 欠落 → `DEFAULT_FONT_NAME`（ukadoc 既定＝正常系・ログなし）」 |
| 同ファイルのモジュール doc（既定値を列挙した行） | 「`DEFAULT_FONT_NAME`＝ＭＳ ゴシック／`DEFAULT_FONT_HEIGHT`＝12／色＝黒」 |

「ログなし」と明記されているとおり、書体名の欠落は areka では**正常系**である。記録を出さないのは異常を握り潰しているからではなく、ukadoc が定めた既定へ落ちるだけだからである。

#### 受け入れの結論

StayseeBalloon の本文は ukadoc 既定の書体（ＭＳ ゴシック）と、バルーン自身が宣言する `font.height,12` で描かれる。これは**正典どおりの挙動**なので、直すべきものは無い。したがって本仕様は次のどちらもしない。

| しないこと | 理由 | 守られる要件 |
|---|---|---|
| areka 側の既定書体の定数（`crates/areka-emo-text/src/draw.rs` の `DEFAULT_FONT_NAME`）を変えない | 変える理由が無い。変えれば ukadoc の既定から外れるうえ、本番コードの変更が発生する | 要件 8.1（本番コードの変更 0 行） |
| バルーン側の `descript.txt` に `font.name` を書き足さない | 書き足せば原作のバイト列が変わり、上流との同一性の検証（`provenance.md` §3 のハッシュ一覧）が成り立たなくなる | 要件 2.2（1 バイトも変えない） |

この 2 つを同時に守れるのは、「見た目を合わせるために何かを変える」必要が無いからである。ukadoc 既定の書体で描かれることは受け入れる結論であって、妥協ではない。

見た目が実際に崩れないことは決定論テスト（要件 3・§2）と実機目視（要件 4・§5）で確かめる。そこで崩れが出た場合も、バルーンを改変して合わせるのではなく areka 側の欠陥として扱う（要件 3.9／4.3・行き先は §4 に記録する）。

---

## 2. 決定論テスト

この節はタスク 2.8 が埋める（未記入）。結果・較正で動いた期待値とその理由・`cargo test --workspace` の本数（着手前と着手後）を書く。

## 3. 既知の縮退（事実の記録）

この節はタスク 2.8 が埋める（未記入）。枠の原点色の白への縮退・折返し基準が未宣言であることの記録・相方側の縮退 2 件を書く。

## 4. 崩れの処理（要件 3.9／4.3）

この節はタスク 2.8 とタスク 4 が埋める（未記入）。要件 3.9／4.3 が発動したかどうかと、発動した場合の行き先（本仕様内で直したか、引受先の spec の実在を確かめて先送りしたか）を書く。

## 5. 実機目視の観察記録（要件 4.2）

この節はタスク 4 が埋める（未記入）。日付・表示に使ったゴースト・バルーンの絶対パス・表示スケール・観察項目・所見を書く。

## 6. 第三者向け README への申し送り文（要件 5.2）

この節はタスク 5.1 が埋める（未記入）。資産名・作者・CC0・出典 URL・既知の制限を、そのまま写せる形で書く。

## 7. 台帳の検査（要件 6.5）

この節はタスク 3.2 が埋める（未記入）。`cargo test -p ukadoc-survey` の結果と、報告 2 本を作り直したことを書く。

## 8. 下流への申し送りの実施記録（要件 7.3・7.4）

この節はタスク 5.2 が埋める（未記入）。申し送り先 3 つの brief が実在することを確かめたことと、追記した内容を書く。
