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
