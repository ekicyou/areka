# doc の棚卸の表

要件 7（棚卸の表）の実装の時点の数え。数えた時点はブランチ `claude/areka-p0-roadmap-html-7eaf4d` のコミット `dd7c86a4`（2026-10-16）で、`doc/` の削除・移動（タスク 6.2）の前。扱いの列は requirements.md の「議題 3」の表の確定（2026-10-15 要件討議）をそのまま写した。

## 数え方

- 行は `doc/` の直下の各ファイルと各フォルダ。フォルダの中にコードかテストがファイルとして読むファイルが在れば、別の行に分けた（`doc/ssp-mcp/` の 2 ファイル）。`doc/ukadoc-coverage/` と `doc/release-notes/` はフォルダの全体が読まれるので 1 行。`doc/discussion-260702-` の 3 ファイル（md 1・svg 2）も 1 ファイル 1 行で、扱いの「3 つを一緒に」は動かし方の決めで、行の数の決めではない。
- ファイルの数は `git ls-files`、最後に変わった日は `git log -1 --format=%cs`。
- 参照の欄は「名指すファイルの数/名指す行の数」。場所の名前（`doc/…`）を `git grep -P` で探した。`doc/` の中だけは、相対のリンクも拾うためにファイル名（フォルダはフォルダ名＋`/`）でも探した。行の自分自身の中の名指しは数えない。`vendors/`（サブモジュール）と `target/`（リポジトリに無い）は探さない。
- 参照の分け方: コードとテスト＝下の 6 つ以外（`crates/`・`tools/`・`.github/` など）／steering＝`.kiro/steering/`（`roadmap-history.md` を除く）／スキル＝`.claude/`／まだ完了していない spec＝`.kiro/specs/`（`completed/` と本仕様を除く）／ほかの doc＝`doc/`／根の文書＝リポジトリの根のファイルと根の `docs/`。
- 歴史の記録からの参照（要件 7.3）＝`.kiro/specs/completed/` と `.kiro/steering/roadmap-history.md` の合計で、ほかの参照と分けた。
- 本仕様の文書（`.kiro/specs/areka-P0-roadmap-overview-html/`）は、この表そのものと移動の記録で全部の行を名指すので、どの欄にも入れず「本仕様」の欄に分けた。「本仕様」の欄はこのファイルをコミットする前の数で、コミットの後に打ち直すとこのファイルの分だけ増える（ほかの欄は変わらない）。
- 「コード・テストの中の名指し」は、ファイルを読む所とコメント・文言の中の名指しを両方含む。ファイルとして読むか・場所の名前を突き合わせるかは「読むか」の欄に書いた（要件 Introduction の引き直した事実と同じ）。

## 表

| 行 | ファイル数 | 最後に変わった日 | 読むか（コード・テスト） | コード・テストの中の名指し | steering | スキル | まだ完了していない spec | ほかの doc | 根の文書 | 歴史の記録からの参照 | 本仕様 | 中身の見立て | 扱い（確定） |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `doc/ROADMAP.md` | 1 | 2026-06-28 | 読まない | 0/0 | 2/2（`focus.md`・`workflow.md`） | 1/3（`kiro-complete/SKILL.md`） | 0/0 | 1/1（`doc/archive/ARCHITECTURE.md`） | 0/0 | 4/9 | 6/23 | 歴史・古い（steering の roadmap.md への 18 行のポインタ） | **消す**（steering 2 か所とスキルの 3 か所を直す。`doc/archive/` の中の名指しは歴史として残す） |
| `doc/discussion-260702-m1-roadmap.md` | 1 | 2026-07-03 | 読まない | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 1/1（`completed/areka-P0-surface-element-nesting/brief.md`） | 3/3 | 歴史・古い（M1 期の議論の写し。自分で「正本が勝つ」と書く） | **`doc/archive/` へ寄せる**（3 つを一緒に） |
| `doc/discussion-260702-concurrency-model.svg` | 1 | 2026-07-03 | 読まない | 0/0 | 0/0 | 0/0 | 0/0 | 1/2（`doc/discussion-260702-m1-roadmap.md`） | 0/0 | 0/0 | 1/1 | 歴史・古い（M1 期の議論の図。上の md が載せる） | **`doc/archive/` へ寄せる**（3 つを一緒に） |
| `doc/discussion-260702-roadmap-tree.svg` | 1 | 2026-07-03 | 読まない | 0/0 | 0/0 | 0/0 | 0/0 | 1/2（`doc/discussion-260702-m1-roadmap.md`） | 0/0 | 0/0 | 1/1 | 歴史・古い（M1 期の議論の図。上の md が載せる） | **`doc/archive/` へ寄せる**（3 つを一緒に） |
| `doc/archive/`（`spec_backup_20251101_082532/` を含む） | 36 | 2026-10-03 | 読まない | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 6/23 | 6/28 | 歴史・古い（既に歴史の置き場。旧 ARCHITECTURE・旧 UI の設計など） | **残す**（歴史の置き場） |
| `doc/shiori/` | 41 | 2026-06-28 | 読まない | 1/1（`ukadoc-survey/src/evidence/resolve.rs` の doc コメント） | 1/2（`roadmap.md`） | 0/0 | 1/4（`shiori4-api/brief.md`） | 3/10（`anchor-compat.md`・`choice-cascade-compat.md`・`ukadoc-coverage/ledger/shiori.toml` の注） | 0/0 | 33/111 | 5/11 | 今の正本（SHIORI の紙の契約のカタログ。まだ完了していない `shiori4-api` が正本と呼ぶ） | **残す**（中身が今の正本と食い違うかは `shiori4-api` の持ち物） |
| `doc/emo2-conformance-scope.md` | 1 | 2026-10-10 | 読まない | 0/0 | 0/0 | 0/0 | 0/0 | 5/15（`choice-cascade-compat.md`・`discussion-260702-m1-roadmap.md`・`ukadoc-coverage/` の briefing 2 つと `ledger/shiori.toml`） | 0/0 | 63/134 | 1/1 | 今の正本（M1 期の範囲の正本。台帳が今も引く） | **残す** |
| `doc/CONSTITUTION.md` | 1 | 2026-10-03 | 読まない | 0/0 | 0/0 | 0/0 | 0/0 | 1/1（`doc/archive/ARCHITECTURE.md`） | 1/1（`README.md`） | 8/12 | 2/2 | 今の正本 | **残す** |
| `doc/PASTA_PROFILE.md` | 1 | 2026-09-11 | 読まない | 0/0 | 1/1（`product.md`） | 0/0 | 0/0 | 0/0 | 1/1（`README.md`） | 8/16 | 0/0 | 今の正本 | **残す** |
| `doc/COMPAT_ARCHITECTURE.md` | 1 | 2026-10-10 | 読まない | 16/19（コメントの中の名指し） | 5/10（`focus.md`・`product.md`・`roadmap.md`・`structure.md`・`tech.md`） | 1/1（`kiro-next/SKILL.md`） | 58/145（brief） | 15/106 | 2/2（`README.md`・`docs/click_through.md`） | 321/775 | 1/1 | 今の正本 | **残す** |
| `doc/anchor-compat.md` | 1 | 2026-10-15 | 読まない | 0/0 | 1/4（`roadmap.md`） | 0/0 | 5/7（brief） | 3/5（`COMPAT_ARCHITECTURE.md`・`ledger/sakura-script.toml`・`ledger/shiori.toml`） | 0/0 | 5/12 | 2/2 | 今の正本 | **残す** |
| `doc/choice-cascade-compat.md` | 1 | 2026-10-07 | 読まない | 0/0 | 1/1（`roadmap.md`） | 0/0 | 1/1（`talk-anchor-interrupt-ticks/brief.md`） | 4/12（`COMPAT_ARCHITECTURE.md`・`anchor-compat.md`・`ledger/` 2 つ） | 0/0 | 17/44 | 0/0 | 今の正本 | **残す** |
| `doc/crates-io-publish.md` | 1 | 2026-10-06 | 読まない | 1/1（`.github/workflows/crates-io.yml` のコメント） | 3/3（`roadmap.md`・`structure.md`・`tech.md`） | 1/2（`release/procedure.md`） | 1/1（`dola-yaml-backend/brief.md`） | 0/0 | 0/0 | 15/39 | 0/0 | 使っている手順書 | **残す** |
| `doc/impl-watch.md` | 1 | 2026-10-10 | 場所の名前を突き合わせる（`areka-impl-watch` のエラー文とそのテスト） | 5/8（`areka-impl-watch` の `cli.rs`・`error.rs` とテスト 3 つ） | 2/3（`roadmap.md`・`structure.md`） | 1/4（`kiro-watch-clear/SKILL.md`） | 2/5（`impl-watch-residue`・`impl-watch-skills` の brief） | 0/0 | 1/1（`CLAUDE.md`） | 3/16 | 3/6 | 使っている手順書 | **残す** |
| `doc/ukadoc-coverage/` | 19 | 2026-10-10 | 読む（全体。`crates/ukadoc-survey/src/io/paths.rs` ほか） | 44/143（`ukadoc-survey` の 43 ファイルと `areka-sylphya/src/vocab/dotted.rs`） | 2/7（`roadmap.md`・`structure.md`） | 2/2（`kiro-complete`・`kiro-next`） | 48/117（brief） | 2/3（`COMPAT_ARCHITECTURE.md`・`anchor-compat.md`） | 1/1（`README.md`） | 203/765 | 4/19 | 今の正本（網羅台帳）・作り直す生成物（`report/`） | **残す** |
| `doc/ssp-mcp/`（下の 2 ファイルを除く 6 ファイル） | 8（フォルダ全体） | 2026-10-11 | 読まない（下の 2 ファイルだけを読む） | 4/4（`areka-mcp`・`areka` のコメントが `areka-tools.md`・`transport-diff-areka.md` を名指す） | 3/11（`roadmap.md`・`structure.md`・`tech.md`） | 0/0 | 17/42（brief） | 1/2（`anchor-compat.md`） | 0/0 | 58/136 | 3/5 | 今の正本（SSP の MCP の事実の控えと差の一覧） | **残す** |
| `doc/ssp-mcp/tools-list-ssp-2.9.05.json` | 1 | 2026-09-29 | 読む（`areka-mcp` の `tools_tests.rs`・`tools_socket_tests.rs`・`tools_own_socket_tests.rs`） | 13/13（読むテスト 3 つと、各ツールのコメント 10） | 0/0 | 0/0 | 0/0 | 4/4（`doc/ssp-mcp/` の 4 ファイル） | 0/0 | 14/21 | 3/4 | 今の正本（SSP 2.9.05 の答えの控え） | **残す** |
| `doc/ssp-mcp/log-convention.md` | 1 | 2026-10-06 | 読む（`crates/areka/src/log_history_convention_tests.rs`） | 1/1 | 2/2（`roadmap.md`・`structure.md`） | 0/0 | 2/4（`mcp-kanade-tools`・`mcp-strict-errors` の brief） | 0/0 | 0/0 | 6/15 | 3/4 | 今の正本（ログの決まり） | **残す** |
| `doc/release-notes/` | 2 | 2026-10-15 | 読む（`tools/release-notes.ps1` と `.github/workflows/release.yml`） | 2/6 | 3/6（`roadmap.md`・`structure.md`・`tech.md`） | 2/15（`release/SKILL.md`・`release/procedure.md`） | 0/0 | 0/0 | 0/0 | 12/84 | 3/9 | 今の正本（版ごとの見どころ） | **残す** |
| `doc/images/` | 1 | 2026-10-03 | 読まない（根の `README.md` が表示する） | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 1/1（`README.md`） | 0/0 | 2/3 | 今の正本（README の絵） | **残す** |
| 根の `docs/`（参考・`doc/` の外） | 1 | 2026-09-24 | 読まない | — | — | — | — | — | — | — | — | `doc/` の外 | **触らない** |

合計: `doc/` は 119 ファイル（直下 12 ファイルと 6 フォルダ）。

## requirements.md の確定の表との突き合わせ

- 扱い: 全行が確定の表と同じ（消す 1 行・寄せる 3 行・残す 16 行・触らない 1 行の 21 行）。確定の表の「discussion の md と svg 2 つ」の行・「`CONSTITUTION.md` ほか 7 ファイル」の行・「`ukadoc-coverage/` ほか 4 フォルダ」の行は、ここではファイルとフォルダごとの行に分けた（扱いはそれぞれ確定の表の行のまま）。
- ファイルの数と最後に変わった日: 確定の表と同じ（`doc/ROADMAP.md` 1・06-28、discussion 3（各 1）・07-03、`doc/archive/` 36・10-03、`doc/shiori/` 41・06-28、`doc/emo2-conformance-scope.md` 1・10-10、7 ファイル 09-11〜10-15、`doc/ukadoc-coverage/` 19・`doc/ssp-mcp/` 8・`doc/release-notes/` 2・`doc/images/` 1、根の `docs/` 1・09-24）。要件の時点からファイルの足し引きは無い。
- 参照: 確定の表の「ほかの参照」と食い違いは無い。確定の表に書かれていない分は次のとおりで、どれも扱いを変えない。
  - `doc/shiori/`: コード 1 件（`crates/ukadoc-survey/src/evidence/resolve.rs:41` の doc コメントが `doc/shiori/fragments/` を名指す）。読むのではない（要件 Introduction の引き直した事実と同じ）。
  - `doc/COMPAT_ARCHITECTURE.md`: コードの 16 ファイルのコメントが名指す。読むのではない。
  - `doc/crates-io-publish.md`: `.github/workflows/crates-io.yml` のコメントが名指す。読むのではない。
  - `doc/ssp-mcp/` の残りの 6 ファイル: `areka-mcp`・`areka` のコメント 4 件が `areka-tools.md`・`transport-diff-areka.md` を名指す。読むのではない。
- 確定の表で「本仕様の brief だけ」とした行のうち、`doc/archive/` と `doc/discussion-260702-m1-roadmap.md` は、歴史と本仕様の欄のほかは 0 で一致する。2 つの svg は、`doc/discussion-260702-m1-roadmap.md` が図として載せる名指し（各 2 行）が「ほかの doc」に在る。寄せる 3 つの中の名指しで、一緒に動かすので相対のリンクは効いたまま（design の DocCleanup）。確定の表の「本仕様の brief だけ」はこの 3 つの間の名指しを数えていない。
- 本仕様の欄は、ほかの欄と同じく `doc/` を付けた完全な名前（`doc/` の中は名前だけ）で探した数。本仕様の文書には、これと別に `doc/discussion-260702-*` や `discussion-260702-` の形で 3 つをまとめて名指す行（brief・design・research）と、`doc/` を付けない名前だけの行（brief の 24 行・design のファイルの木）が在り、それは数に入らない。
- タスク 7.1 で足す `doc/roadmap-overview-{YYYY-MM-DD}.html` と、タスク 6.4 で足す `doc/README.md` は、この数えの時点では無い。

## 探したコマンド

ワークツリーの根で Git Bash から打つ（`git grep -P` が使えること）。出力は行ごとに `行 | n=ファイル数 | 最後に変わった日 | code=… | steer=… | skill=… | spec=… | doc=… | root=… | hist=… | self=…`（各「ファイルの数/行の数」）で、その下に歴史と本仕様を除く欄の名指すファイルの一覧が付く。一時のファイルは `target/inv_hits` に書いて消す。

```sh
#!/bin/sh
# run at worktree root. row LABEL LOCATION FULLREGEX DOCREGEX
SELF=.kiro/specs/areka-P0-roadmap-overview-html/
H=target/inv_hits
row() {
  loc=$2
  n=$(git ls-files -- ":(glob)$loc*" "$loc" | sort -u | wc -l)
  d=$(git log -1 --format=%cs -- ":(glob)$loc*" "$loc")
  { git grep -P -c "$3" -- . ':!vendors' ':!doc' ; git grep -P -c "$4" -- doc ; } > $H 2>/dev/null
  awk -F: -v loc="$loc" -v self="$SELF" -v label="$1" -v n="$n" -v d="$d" '
  { f=$1; c=$2
    if (index(f, loc)==1) next
    if (index(f, self)==1) k="self"
    else if (f ~ /^\.kiro\/specs\/completed\// || f==".kiro/steering/roadmap-history.md") k="hist"
    else if (f ~ /^\.kiro\/steering\//) k="steer"
    else if (f ~ /^\.claude\//) k="skill"
    else if (f ~ /^\.kiro\/specs\//) k="spec"
    else if (f ~ /^doc\//) k="doc"
    else if (f !~ /\// || f ~ /^docs\//) k="root"
    else k="code"
    files[k]++; lines[k]+=c; list[k]=list[k] " " f
  }
  END { printf "%s | n=%s | %s", label, n, d
    split("code steer skill spec doc root hist self", ks, " ")
    for (i=1;i<=8;i++){k=ks[i]; printf " | %s=%d/%d", k, files[k], lines[k]}
    print ""
    for (i=1;i<=6;i++){k=ks[i]; if (files[k]) print "    " k ":" list[k]}
  }' $H
  rm -f $H
}
F() { b=$(printf '%s' "$1" | sed 's/[.]/[.]/g'); row "$1" "doc/$1" "doc/$b" "(?<![\w-])$b" ; }
D() { row "$1/" "doc/$1/" "doc/$1\b" "(?:doc/|(?<![\w/.-]))$1/" ; }
F ROADMAP.md
for f in discussion-260702-m1-roadmap.md discussion-260702-concurrency-model.svg discussion-260702-roadmap-tree.svg; do F $f; done
D archive
D shiori
F emo2-conformance-scope.md
for f in CONSTITUTION PASTA_PROFILE COMPAT_ARCHITECTURE anchor-compat choice-cascade-compat crates-io-publish impl-watch; do F $f.md; done
D ukadoc-coverage
row ssp-mcp/ doc/ssp-mcp/ 'doc/ssp-mcp(?![\w-])(?!/(tools-list-ssp-2\.9\.05\.json|log-convention\.md))' '(?:doc/|(?<![\w/.-]))ssp-mcp/(?!(tools-list-ssp-2\.9\.05\.json|log-convention\.md))'
row ssp-mcp/tools-list-ssp-2.9.05.json doc/ssp-mcp/tools-list-ssp-2.9.05.json 'doc/ssp-mcp/tools-list-ssp-2\.9\.05\.json' '(?<![\w-])tools-list-ssp-2\.9\.05\.json'
row ssp-mcp/log-convention.md doc/ssp-mcp/log-convention.md 'doc/ssp-mcp/log-convention\.md' '(?<![\w-])log-convention\.md'
D release-notes
D images
```

根の `docs/` の行は `git ls-files docs | wc -l` と `git log -1 --format=%cs -- docs`、`doc/` の合計は `git ls-files doc | wc -l` で数えた。

## 元の場所の名前が残っていないことの確かめ（要件 8.4）

2026-10-16、コミット `5f85863b` の上（作業ツリーに変更なし）で、Git Bash から `git grep`（git 2.55.0）で探した。`--untracked` で追跡していないファイルも含めた。動かした 4 ファイルは、消した `doc/ROADMAP.md` と、`doc/archive/` へ寄せた `doc/discussion-260702-m1-roadmap.md`・`doc/discussion-260702-concurrency-model.svg`・`doc/discussion-260702-roadmap-tree.svg`。3 つの discussion は名前の頭がそろうので `discussion-260702-` の 1 つの型でまとめて探した。

探した型は 5 つ。`doc/` を付けた完全な名前に加え、`doc/` の中から相対のリンクで書ける名前だけの形（`](ROADMAP.md)` など）も拾うため、名前だけの型も探した。

| 型 | 拾うもの |
|---|---|
| `doc/ROADMAP\.md` | 完全な名前 |
| `(?<![\w/.-])ROADMAP\.md` | 前にパスの付かない名前だけ（相対のリンク） |
| `ROADMAP\.md` | 前に何が付いても（ほかのフォルダの同名も含む・最も広い） |
| `doc/discussion-260702-` | 3 つの完全な名前 |
| `discussion-260702-` | 3 つの名前だけ（相対のリンク・最も広い） |

大文字と小文字は区別した（`.kiro/steering/roadmap.md` は別のファイル）。念のため `doc/roadmap\.md` を `-i` で探した結果も 0 件。

### 除く所を外して探したコマンドと結果

```sh
EX=(':!.kiro/specs/completed/' ':!.kiro/steering/roadmap-history.md' ':!doc/archive/' ':!.kiro/specs/areka-P0-roadmap-overview-html/')
for p in 'doc/ROADMAP\.md' '(?<![\w/.-])ROADMAP\.md' 'ROADMAP\.md' 'doc/discussion-260702-' 'discussion-260702-'; do
  git grep -P -n --untracked "$p" -- . "${EX[@]}"; echo "rc=$?"
done
git grep -n -i --untracked 'doc/roadmap\.md' -- . "${EX[@]}"; echo "rc=$?"
```

結果: 6 つとも出力なし・`rc=1`（一致なし）。**除く所の外に残った参照は 0 件。**

### 較正（除く所を外さずに同じ型で探す）

```sh
git grep -P -c --untracked "$p" -- . | awk -F: '{s+=$2; n++} END{print n" files / "s" lines"}'
for d in .kiro/specs/completed/ .kiro/steering/roadmap-history.md doc/archive/ .kiro/specs/areka-P0-roadmap-overview-html/; do
  git grep -P -c --untracked "$p" -- "$d" | awk -F: -v d="$d" '{s+=$2; n++} END{printf "%s: %d files / %d lines\n", d, n, s}'
done
```

除く所の 4 つはどれも在る（追跡しているファイルの数: `.kiro/specs/completed/` 2052・`roadmap-history.md` 1・`doc/archive/` 39・本仕様のフォルダ 8）。除く所を外さないと同じ型で一致が出て、その数は除く所ごとの数の和にちょうど等しい。だから除く指定は効いていて、上の 0 件は「型が何にも当たらない」のではなく「除く所の外に無い」ことを示す。

### 除いた所に残る件数（ファイル数 / 行数）

| 型 | 外さずに全体 | `.kiro/specs/completed/` | `roadmap-history.md` | `doc/archive/` | 本仕様のフォルダ |
|---|---|---|---|---|---|
| `doc/ROADMAP\.md` | 12 / 35 | 4 / 9 | 0 / 0 | 1 / 1 | 7 / 25 |
| `(?<![\w/.-])ROADMAP\.md` | 12 / 76 | 9 / 72 | 0 / 0 | 0 / 0 | 3 / 4 |
| `ROADMAP\.md` | 18 / 124 | 10 / 94 | 0 / 0 | 1 / 1 | 7 / 29 |
| `doc/discussion-260702-` | 6 / 13 | 1 / 1 | 0 / 0 | 0 / 0 | 5 / 12 |
| `discussion-260702-` | 9 / 27 | 2 / 2 | 0 / 0 | 1 / 3 | 6 / 22 |

- `doc/archive/` の `ROADMAP.md` の 1 行は `doc/archive/ARCHITECTURE.md` の名指しで、棚卸の表の決めどおり歴史として残す。`discussion-260702-` の 3 行は寄せた `discussion-260702-m1-roadmap.md` が 2 つの svg を載せる相対のリンクで、一緒に動かしたので効いたまま。
- 本仕様のフォルダの数は、この節を足す前に数えた。この節そのものが型の文字を含むので、足した後は増える。
- `.kiro/specs/completed/` の `ROADMAP.md` の行には、ほかの場所の同じ名前を指すものも含む（広い型なので）。どちらも当時の記録で書き換えない（要件 8.3）。
