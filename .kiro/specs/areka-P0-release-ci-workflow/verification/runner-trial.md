# 証跡: リリース workflow の確かめ

対象は `.github/workflows/release.yml`。手元の確かめ（タスク 1.1〜1.6）・静的な確かめ（タスク 3）・実行環境での 1 回の走り（タスク 4）の結果を、節を足して記録する。

## 1.1 取り出しの action の固定

`actions/checkout` を、その時点の最新の版が指すコミットの 40 桁の SHA で固定した。

| 項目 | 値 |
|---|---|
| 引いた日 | 2026-10-03 |
| 最新の版 | `v7.0.1` |
| 指すコミット | `3d3c42e5aac5ba805825da76410c181273ba90b1` |
| タグの種類 | 軽量タグ（`git/ref/tags/v7.0.1` の `object.type` が `commit`）＝タグの SHA とコミットの SHA が同じ |
| `release.yml` の行 | `uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1` |

引いた手順:

```
gh api repos/actions/checkout/releases/latest --jq .tag_name
gh api repos/actions/checkout/git/ref/tags/v7.0.1 --jq '.object.type+" "+.object.sha'
gh api repos/actions/checkout/commits/v7.0.1 --jq .sha
```

## 1.2 版の検査の段（S4）を手元で 5 通り回す

回した日: 2026-10-03。今の版（`Cargo.toml` の `[workspace.package] version`）は `0.0.1`。

本文の取り出し方: Python と PyYAML で `release.yml` を読み、`jobs.release.steps` から名前が `版の検査` の段をちょうど 1 件取り、その `run:` の本文をワークツリーの `target\` の下の一時ファイルへ書いた。GitHub の pwsh の包み方を写して、先頭に `$ErrorActionPreference = 'stop'`、末尾に `if ((Test-Path -LiteralPath variable:\LASTEXITCODE)) { exit $LASTEXITCODE }` を足し、ワークツリーの根から `pwsh -NoProfile -NonInteractive -Command ". '<一時ファイル>'"` で回した。環境変数は `GITHUB_REF_TYPE`・`GITHUB_REF_NAME` を差し替え、`GITHUB_OUTPUT` は場合ごとの空の一時ファイルを指した。判定は道具が機械でした（終了コード・印字に期待の字が在るか・`GITHUB_OUTPUT` の中身が `version=0.0.1` か空か）。取り出す前に、段の `id` が `version` であること・本文に `${{` が無いこと・段に `timeout-minutes` が無いことも確かめた。一時ファイルは確かめの後に消した。

| 始めた参照 | 期待 | 終了コード | 印字 | `GITHUB_OUTPUT` | 判定 |
|---|---|---|---|---|---|
| タグ `v0.0.1` | 成功・`version=0.0.1` | 0 | `タグの版と Cargo.toml の版が一致（'0.0.1'）` | `version=0.0.1` | PASS |
| タグ `v0.0.2` | 失敗・`0.0.2` と `0.0.1` の両方を印字 | 1 | `タグの版と Cargo.toml の版が違う（タグの版 '0.0.2'・Cargo.toml の版 '0.0.1'）` | 空 | PASS |
| タグ `v` | 失敗・空の版と `0.0.1` を印字 | 1 | `タグの版と Cargo.toml の版が違う（タグの版 ''・Cargo.toml の版 '0.0.1'）` | 空 | PASS |
| タグ `v0.0.1-x` | 失敗・両方を印字 | 1 | `タグの版と Cargo.toml の版が違う（タグの版 '0.0.1-x'・Cargo.toml の版 '0.0.1'）` | 空 | PASS |
| 枝 `main` | 成功・「タグが無い」と `0.0.1` を印字 | 0 | `タグが無いので比べを飛ばす（Cargo.toml の版 '0.0.1'）` | `version=0.0.1` | PASS |

段を置く前に同じ道具を回すと「`版の検査` の段が 0 件」で失敗した（赤）。道具が判定できることの較正として、不一致のときの `exit 1` を一時的に `exit 0` へ変えて回すと、タグ `v0.0.2`・`v`・`v0.0.1-x` の 3 通りが FAIL になった（`release.yml` は元へ戻した）。

## 1.3 既存の Release の検査の段（S5）を手元で回す

回した日: 2026-10-03。段の名前は `既存の Release の検査`・`id` は `guard`・`timeout-minutes: 5`。トークン（`GH_TOKEN`）と S4 の版（`VERSION`）は段の `env:` で渡し、本文に `${{` は無い。

本文の取り出し方: Python と PyYAML で `release.yml` を読み、`jobs.release.steps` から名前が `既存の Release の検査` の段をちょうど 1 件取り、`run:` の本文をワークツリーの `target\` の下の一時ファイルへ書いた。取り出すときに、`id` が `guard`・`timeout-minutes` が 5・`env` の `GH_TOKEN` と `VERSION` の 2 つ・この段とほかの全部の段の本文に `${{` が無いことを機械で判定した（6 項目すべて合格）。一つ前のタグを決める関数は、本文の `function Get-PrevTag` の行から、行頭の `}` だけの行までを別の一時ファイルへ切り出した。段の全体は、1.2 と同じく GitHub の pwsh の包み方を写して（先頭に `$ErrorActionPreference = 'stop'`、末尾に `if ((Test-Path -LiteralPath variable:\LASTEXITCODE)) { exit $LASTEXITCODE }`）、`pwsh -NoProfile -NonInteractive` で回した。`GH_TOKEN` は設定せず、手元の `gh` の認証で読むだけの呼び出しをした。`GITHUB_OUTPUT` は場合ごとの空の一時ファイルを指した。一時ファイルは確かめの後に消した。

段を置く前に取り出しの道具を回すと「`既存の Release の検査` の段が 0 件」で失敗した（赤）。

### 一つ前のタグを決める関数の 6 通り

道具が `Get-PrevTag` の返す字と期待を文字どおり比べて判定した。

| タグの並び | 今の版 | 期待 | 返った値 | 判定 |
|---|---|---|---|---|
| `v0.0.1` | `0.0.2` | `v0.0.1` | `v0.0.1` | PASS |
| `v0.0.1`・`v0.0.2` | `0.0.2` | `v0.0.1` | `v0.0.1` | PASS |
| `v0.0.2` | `0.0.2` | 空 | 空 | PASS |
| `v`・`vfoo`・`v0.0.1` | `0.0.2` | `v0.0.1` | `v0.0.1` | PASS |
| `v0.0.1`・`v0.0.2`・`v0.0.3-rc.1` | `0.0.3` | `v0.0.3-rc.1` | `v0.0.3-rc.1` | PASS |
| `v0.0.1`・`v0.0.2` | `0.0.3-rc.1` | `v0.0.2` | `v0.0.2` | PASS |

較正: 切り出した関数の写しで比べの `-lt` を `-le` に変えると、2 行目（`v0.0.2` を返した）と 3 行目（`v0.0.2` を返した）の 2 通りが FAIL になった。`v` を除かずに読む写し（`TryParse($name, ...)`）では 5 通りが FAIL になった。どちらも写しだけを変え、`release.yml` は変えていない。

### 段の全体を手元の `gh` で回す

先に `gh api repos/ekicyou/areka/releases --jq length` で Release が 0 件、タグの一覧が `v0.0.1` の 1 本であることを確かめた。

| 場合 | リポジトリ | 始めた参照 | `VERSION` | `PUBLISH` | 印字 | 終了コード | `GITHUB_OUTPUT` |
|---|---|---|---|---|---|---|---|
| 本番の初回を写す | `ekicyou/areka` | タグ `v0.0.2` | `0.0.2` | `true` | `タグ 'v0.0.2' の Release は無い（下書きを含めて 0 件）`・`一つ前のタグ: 'v0.0.1'` | 0 | `absent=true`・`prev_tag=v0.0.1` |
| 枝で始めた乾いた走り | `ekicyou/areka` | 枝 `main` | `0.0.1` | `false` | `タグ 'v0.0.1' の Release は無い（下書きを含めて 0 件）`・`一つ前のタグ: ''` | 0 | `absent=true`・`prev_tag=`（空） |

1 行目が完了の形の「無い・`v0.0.1`」。

### 在る経路と取れない経路（ほかの公開リポジトリを読むだけで通した）

Release を作らずに「在る」経路を通すため、Release を持つ公開リポジトリ `cli/cli` を読むだけで回した。`cli/cli` は Release が 205 件・タグが 205 本で、どちらも 3 ページにまたがる（ページ送りが要る）。

| 場合 | リポジトリ | 始めた参照 | `PUBLISH` | 印字 | 終了コード | `GITHUB_OUTPUT` |
|---|---|---|---|---|---|---|
| 本番で在る | `cli/cli` | タグ `v2.101.0` | `true` | `タグ 'v2.101.0' の Release が既に在る（URL …/releases/tag/v2.101.0・下書き false）` | 1 | 空（`absent` を出さない） |
| 乾いた走りで在る | `cli/cli` | タグ `v2.101.0` | `false` | 同じ印字・`乾いた走りなので止めずに続ける`・`一つ前のタグ: 'v2.100.0'` | 0 | `prev_tag=v2.100.0`（`absent` は無い） |
| 本番で在る（3 ページ目にだけ在る最古の Release） | `cli/cli` | タグ `v0.4.0` | `true` | `タグ 'v0.4.0' の Release が既に在る（URL …/releases/tag/v0.4.0・下書き false）` | 1 | 空 |
| 一覧を取れない | 実在しないリポジトリ | タグ `v0.0.2` | `false` | `gh: Not Found (HTTP 404)`・`Release の一覧を取れない（gh api が終了コード 1）` | 1 | 空 |

`v2.100.0` が `v2.101.0` の一つ前であることは、タグの一覧（全ページ）を別に引いて確かめた。乾いた走りの「取れない」も失敗で止まる（設計の S5 の事後条件どおり）。

### 残り

- 下書きの Release が一覧に入ることは、Release を作らないので手元では確かめていない（設計の Risks のとおり `release-cycle` へ申し送る）。
- Release を探す行（`$found = @(gh api --paginate ... | Where-Object { ... -ceq $tag })`）は後始末の段（タスク 1.6）でそのまま使う 1 行にしてある。2 か所が同じことの判定はタスク 3 で行う。
