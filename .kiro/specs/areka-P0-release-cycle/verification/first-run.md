# 初回のリリース（v0.0.2）の記録

初回（`v0.0.1` → `v0.0.2`）だけの確かめの結果。2 回目からは roadmap の「リリース」の表の 1 行だけを書く。

## 版上げの道具

- やり方: いつものやり方（`cargo set-version --bump patch --workspace`。cargo-edit 0.13.13）。代わりのやり方は使っていない。
- 根の `Cargo.toml` の 2 行は両方動いた（版上げのコミット `01f5dd16` の差分）。

  ```diff
  -version = "0.0.1"
  +version = "0.0.2"
  -dola = { version = "0.0.1", path = "crates/dola" }
  +dola = { version = "0.0.2", path = "crates/dola" }
  ```

- 段 3 の差分の範囲の判定は、表の全部の行が当たった（`Cargo.toml` 2・2、`Cargo.lock` 30・30、`THIRD-PARTY-NOTICES.md` 30・30、`dist/README.txt` 1・1）。

## タグの前

- Trusted Publishing: 2026-10-06 に開発者が `wintf`・`dola` の両方の設定の頁を貼り、どちらにも発行元 GitHub・`ekicyou/areka`・`crates-io.yml`・environment 空の 1 行が在ることを確かめた。`wintf` は、最初の追加が `areka` のクレートへ入っていたため、その場で入れ直した（`areka` のクレートの 1 行は今回の公開には使わない）。
- main の乾いた走り: 緑。
  - 1 回目（main `44fc0a61`）: success https://github.com/ekicyou/areka/actions/runs/37317923736 。ただし段の先頭で端末の文字コードを書き替えて通っていた。
  - 2 回目（main `19f68777`。`tools-utf8-child-output` が端末の書き替えを撤去した後）: success https://github.com/ekicyou/areka/actions/runs/37458177482
- 段 3 の `tools/package.ps1 -Check` が、`cargo metadata` の日本語の説明を Shift_JIS で読んで JSON が壊れ赤（終了コード 3）になった。版上げの PR に道具の直しは載せられないので、`tools-utf8-child-output` として起票し、そちらが main に入った（PR#256）後に main を取り込んで段 3 を頭から回し直した。
- 段 3 の全体テストの 1 回目は `areka-emo-present` の 2 本が赤（検体 `claudia` の原本の `rename` が os error 5）。既知の揺らぎの族で `ghost-session-test-load-flake` が引き取っている。単独で緑を確かめ、丸ごと回し直して 9 段すべて緑。

## 申し送りの 6 項目

| 項目 | 見た事実 | 結果 | 起票した spec |
|---|---|---|---|
| 1 Release の公開の段 | Release は公開の状態（下書きでない）で、添わった物がちょうど 4 つ（x64・arm64 の zip とそれぞれの `.sha256`）。本文の比べる範囲は `v0.0.1...v0.0.2` | 緑 | — |
| 2 後始末の段の消す経路 | 赤も取り消しも起きず、段「後始末」は `skipped` | 起きなかった | — |
| 3 下書きの Release の見え方 | 段「既存の Release の検査」は「下書きを含めて 0 件」と記録。下書きは残っていない | 起きなかった | — |
| 4 赤の走りの「Re-run」 | Re-run は行っていない | 起きなかった | — |
| 5 マージ後の乾いた走り | 前半: main の乾いた走りは緑（上の「タグの前」）。後半: `gh workflow run release.yml --ref v0.0.2` は道具の安全の見張りに止められ、開発者が許可を出さずに合格と判断した | 前半 緑・後半 省いた | — |
| 6 「タグで始めた」枝 | タグの push の走りの段「版の検査」「既存の Release の検査」がどちらも `success` | 緑 | — |

## 走り

- タグ: `v0.0.2` → `e32934570c0ef640106577e6999e5fc9d6c0c411`（PR#257 の squash）。2026-10-06 に押した。
- Release の走り: success https://github.com/ekicyou/areka/actions/runs/37464630569
- GitHub Release: https://github.com/ekicyou/areka/releases/tag/v0.0.2
- 公開の走り: success https://github.com/ekicyou/areka/actions/runs/37464630734 。`tools/crates-io.ps1 -Pending -Version 0.0.2` は終了コード 0・標準出力 0 行（`dola 0.0.2`・`wintf 0.0.2` が在る）。
- winget: タグのコミットに `winget.yml` が無いので見守りの相手に入れていない。
