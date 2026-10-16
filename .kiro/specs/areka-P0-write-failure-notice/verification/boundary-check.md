# 境界の確かめの記録（タスク 7.1）

対象仕様: `areka-P0-write-failure-notice`
対象要件: 2.8 / 2.10 / 3.5 / 6.4 / 6.5 / 8.1 / 8.2 / 9.1 / 9.3 / 9.4
実施日: 2026-10-17
実施した版: `ce01252b`（タスク 6.3 まで着地済み）＋作業ツリーの roadmap の 1 項目（下の 6 節）
比較の基点: `caa2003e`（`git merge-base HEAD origin/main`＝今の `origin/main` の先頭）

書いてある数は、すべてこの日に実際に走らせて得たもの。

---

## 0. 結論（先に）

- 約束のファイル・フォルダに触った所は **0**（6 か所すべて）。
- 新しい SHIORI イベントは **0**、新しい Reference は **0**。
- `crates/areka/src/main.rs` は **994 行**（1,000 行以下）。
- 印の代わりに落ちたことに気付く補いの仕組みは **作っていない**。
- ネットワーク更新の失敗の語と、更新の作業フォルダの扱いは **変えていない**。
- emo2 の辞書は **書き換えていない**。申し送りは roadmap に 1 項目で書いた。
- **タスク 5.4（起動の時点の書ける確かめ）はまだ着地していない**。この確かめは `ce01252b` までの差分だけを見ている。5.4 は `areka` から `areka-user-data` への依存と `main.rs` の数行を足すので、**5.4 の着地の後に、同じ手順でもう一度確かめる**（特に `main.rs` の行数と、約束のファイルに触らないこと）。

---

## 1. 比べ方（main の取り込みを除く）

このブランチには `e9a3f3e2` で `origin/main` を取り込んである（親は `16c2e552` と `caa2003e`）。取り込んだ側の変更を数えないよう、**分かれ目（merge-base）から HEAD までの差分**だけを見た。

| コマンド（逐語） | 結果 |
|---|---|
| `git fetch origin` | 最新にした |
| `git merge-base HEAD origin/main` | `caa2003e`（`origin/main` の先頭と同じ） |
| `git diff --name-only $(git merge-base HEAD origin/main)..HEAD \| wc -l` | 56 |
| `git diff --name-only origin/main...HEAD \| wc -l`（3 点の形で照合） | 56（同じ） |
| `git log --oneline $(git merge-base HEAD origin/main)..HEAD \| wc -l` | 28 コミット |

分かれ目が `origin/main` の先頭と同じなので、この差分はこのブランチが自分で入れた変更だけになる。

## 2. 約束のファイル・フォルダ（要件 6.4・2.10・6.5）

「差分 0」を書く前に、そのパスが本当に追跡されていることを、両方の版で数えて確かめた（パスを打ち間違えると、触っていても 0 に見えるため）。

| パス | HEAD で追跡中のファイル数 | 基点で追跡中のファイル数 | 差分のファイル数 |
|---|---|---|---|
| `crates/areka/src/boot_config.rs` | 1 | 1 | **0** |
| `crates/areka/src/roots/mod.rs` | 1 | 1 | **0** |
| `crates/areka/src/roots/resolve.rs` | 1 | 1 | **0** |
| `crates/areka-update/` | 30 | 30 | **0** |
| `crates/areka-mcp-bridge/` | 23 | 23 | **0** |
| `vendors/` | 12 | 12 | **0** |

コマンド: `git ls-files -- <パス> | wc -l`・`git ls-tree -r --name-only <基点> -- <パス> | wc -l`・`git diff --name-only <基点>..HEAD -- <パス> | wc -l`。

較正（同じ手順が「触った」を拾えること）: 隣の `crates/areka/src/roots/` で同じ差分を取ると、触ったファイル 2 つ（`roots/edit.rs`・`roots/edit_write_failure_tests.rs`）が出る。`roots/mod.rs`・`roots/resolve.rs` は出ない。

あわせて、差分の中の `Cargo.toml`・`Cargo.lock` は 0（`git diff --name-only <基点>..HEAD | grep -c 'Cargo\.\(toml\|lock\)'`）。

ネットワーク更新（要件 2.10・6.5）: `crates/` と `doc/` の差分の足した行・消した行に `update-work`・`update_work`・`network`・`areka_update`・`OnUpdateFailure` は 0 行。`doc/COMPAT_ARCHITECTURE.md` の差分は「インストールの失敗理由の語の写し」と「SHIORI の失敗でプロセスが終わるときの終了コード」の 2 行だけ。

## 3. 新しい SHIORI イベントと Reference（要件 2.8・9.1）

| 確かめ | コマンドの要点 | 結果 |
|---|---|---|
| 足した行に出るイベント名 | `git diff <基点>..HEAD -- crates` の `+` 行から `\bOn[A-Z][A-Za-z]+` を拾って重複を除く | `OnInstallFailure` の 1 つだけ |
| その名前は前から在るか | `git grep -c '\bOnInstallFailure\b' <基点> -- crates doc` | 基点で 21 ファイルに在る＝新しいイベントではない |
| 引用符つきの名前の増減 | `+` 行と `-` 行の `"On..."` を数える | `"OnInstallFailure"` が +1／−1（書き換えで同じ数） |
| Reference を足す行 | コメント以外の `+` 行で `Reference`・`refs`・`Ref[0-9]`・`raise_event` などを探す | 0 行（`Ref6/7`・`Ref7` は記録の文の中の語だけで、送る Reference は増えていない） |

`OnInstallFailure` の Reference0 は語 1 つのままで、その語が書けないとき `fileio` になっただけ（要件 5.4 のまま）。

## 4. `main.rs` の行数（要件 6.4）

| 版 | 行数 |
|---|---|
| 基点 `caa2003e` | 971 |
| HEAD `ce01252b` | **994** |

コマンド: `wc -l crates/areka/src/main.rs`・`git show <基点>:crates/areka/src/main.rs | wc -l`。上限 1,000 まで残り 6 行。5.4 で足す行を数え直すこと。

## 5. 印の代わりの補いの仕組み（要件 3.5・9.3）

- 新しく足したファイル 10 本のうち、テストでないものは `crates/areka/src/write_failure.rs` の 1 本だけ。中身は書けない失敗を集めて致命へ渡す机（`install`・`sender`・`fatal`・`stop_boot`・`take_receiver`・`log_only`・`tell_at_close`）で、印の代わりに何かを残す物ではない。
- テストでない変更ファイル 17 本の足した行に、ファイル・フォルダ・レジストリへ書く呼び出し（`fs::write`・`fs::create*`・`fs::rename`・`fs::copy`・`File::create`・`OpenOptions`・`create_dir`・`RegSetValue`）は **0 行**。
- `main.rs` の差分は、印を消せなかった失敗を返して告知を 1 つ選ぶ形に変えただけで、印の外に落ちたことを覚える場所は足していない。

## 6. emo2 の辞書と申し送り（要件 8.1・8.2・9.4）

- `vendors/` の差分は 0（2 節）＝配布形の `vendors/sample_ghost/emo2.nar` を含め、辞書は書き換えていない。
- `.kiro/steering/roadmap.md` の「winget-manifest-submission の手元の確かめで見つかった件」の節に、「emo2 の辞書の持ち主（pasta の側）への申し送り」の項目を 1 つ足した（`OnInstallFailure` に `fileio` が届くようになったこと・理由ごとに台詞を分けてほしいこと）。
- 「次のリリースへの申し送り」の節には書いていない。あの節は `/release` が読み、版上げのファイルの外の仕事が残っていると止まるため。

## 7. 残り

- タスク 5.4 の着地の後に、1〜5 節を同じコマンドでもう一度走らせ、この記録に結果を足す（5.4 は `areka` → `areka-user-data` の依存と `main.rs` の数行を足す）。
