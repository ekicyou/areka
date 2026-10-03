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
