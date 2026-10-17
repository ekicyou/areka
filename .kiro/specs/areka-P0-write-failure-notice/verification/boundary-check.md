# 境界の確かめの記録（タスク 7.1）

対象仕様: `areka-P0-write-failure-notice`
対象要件: 2.8 / 2.10 / 3.5 / 6.4 / 6.5 / 8.1 / 8.2 / 9.1 / 9.3 / 9.4
実施日: 2026-10-17
実施した版: `c338248a`（タスク 5.4・6.4 まで着地済み。`5474dfc0` で `origin/main`〔`311e8863`＝PR#322 `user-data-root`〕を取り込み、`31d3338e` で取り込みに合わせたテストの直しを入れた後）
比較の基点: `311e8863`（`git merge-base HEAD origin/main`＝今の `origin/main` の先頭）

書いてある数は、すべてこの日に実際に走らせて得たもの。前回（`ce01252b`・基点 `caa2003e`）の記録を、5.4 の着地の後に同じ手順で取り直して置き換えた。

---

## 0. 結論（先に）

- 約束のファイル・フォルダに触った所は **0**（6 か所すべて）。
- 新しい SHIORI イベントは **0**、新しい Reference は **0**。
- `crates/areka/src/main.rs` は **1,000 行**（1,000 行以下。残りは 0 行）。
- 印の代わりに落ちたことに気付く補いの仕組みは **作っていない**。
- ネットワーク更新の失敗の語と、更新の作業フォルダの扱いは **変えていない**。
- emo2 の辞書は **書き換えていない**。申し送りは roadmap に 1 項目で書いた。
- **タスク 5.4 は着地した**（`7ecf6103`）。`areka` → `areka-user-data` の依存は、このブランチではなく main の PR#322 が先に入れていた（基点 `311e8863` の `crates/areka/Cargo.toml` 39 行目に在る）。そのため、このブランチの差分の `crates/areka/Cargo.toml`・`Cargo.lock` は **0**。5.4 が `main.rs` に足したのは呼び出しの **3 行**（コメント 1 行＋`check_boot_dirs` の呼び出し 2 行）だけ。
- 約束の一覧の外の 1 件: main から来た `crates/areka/src/boot_config_copy_tests.rs` を、取り込みの後始末として `31d3338e` で **1 行** 直した（下の 2 節）。約束の `boot_config.rs` そのものではない。

---

## 1. 比べ方（main の取り込みを除く）

このブランチには `5474dfc0` で `origin/main` を取り込んである（親は `fd1c2a8c` と `311e8863`）。取り込んだ側の変更を数えないよう、**分かれ目（merge-base）から HEAD までの差分**だけを見た。

| コマンド（逐語） | 結果 |
|---|---|
| `git fetch origin` | 最新にした |
| `git merge-base HEAD origin/main` | `311e8863`（`git rev-parse --short origin/main` も `311e8863`） |
| `git diff --name-only $(git merge-base HEAD origin/main)..HEAD \| wc -l` | 62 |
| `git diff --name-only origin/main...HEAD \| wc -l`（3 点の形で照合） | 62（同じ） |
| `git log --oneline $(git merge-base HEAD origin/main)..HEAD \| wc -l` | 34 コミット |

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

約束の一覧の外で、隣のファイルに触った 1 件（隠さずに書く）:

| パス | HEAD | 基点 | 差分のファイル数 | 中身 |
|---|---|---|---|---|
| `crates/areka/src/boot_config_copy_tests.rs` | 1 | 1 | 1 | 1 行の置き換え（`git diff --stat` で `1 insertion(+), 1 deletion(-)`） |

- このファイルは main（PR#322）から来たテストで、約束の `boot_config.rs` そのものではない。
- 直したのは `31d3338e` の 1 行だけ（`git log --oneline <基点>..HEAD -- crates/areka/src/boot_config_copy_tests.rs` はこの 1 コミット）。`write_session_mark(...)` が書けない失敗を返すようになった（このブランチの変更）ので、テストの呼び出しに `.expect("印を書けること")` を付けた。テストの確かめる中身は変えていない。

あわせて、差分の中の `Cargo.toml`・`Cargo.lock` は 0（`git diff --name-only <基点>..HEAD | grep -c 'Cargo\.\(toml\|lock\)'`）。`crates/areka/Cargo.toml`・`Cargo.lock` をそれぞれ `git diff --name-only <基点>..HEAD -- <パス> | wc -l` で数えても 0 と 0。`areka-user-data` への依存は基点の時点で在る（`git grep -n 'areka-user-data' <基点> -- crates/areka/Cargo.toml` → 39 行目、`git grep -n 'name = "areka-user-data"' <基点> -- Cargo.lock` → 383 行目）。

ネットワーク更新（要件 2.10・6.5）: `crates/` と `doc/` の差分の足した行・消した行に `update-work`・`update_work`・`network`・`areka_update`・`OnUpdateFailure` は 0 行。`doc/COMPAT_ARCHITECTURE.md` の差分は「インストールの失敗理由の語の写し」と「SHIORI の失敗でプロセスが終わるときの終了コード」の 2 行だけ（`git diff --numstat` で 2／2）。

## 3. 新しい SHIORI イベントと Reference（要件 2.8・9.1）

| 確かめ | コマンドの要点 | 結果 |
|---|---|---|
| 足した行に出るイベント名 | `git diff <基点>..HEAD -- crates` の `+` 行から `\bOn[A-Z][A-Za-z]+` を拾って重複を除く | `OnInstallFailure` の 1 つだけ |
| その名前は前から在るか | `git grep -l '\bOnInstallFailure\b' <基点> -- crates doc \| wc -l` | 基点で 21 ファイルに在る＝新しいイベントではない |
| 引用符つきの名前の増減 | `+` 行と `-` 行の `"On..."` を数える | `"OnInstallFailure"` が +1／−1（書き換えで同じ数） |
| Reference を足す行 | コメント以外の `+` 行で `Reference`・`refs`・`Ref[0-9]`・`raise_event` などを探す | 5 行当たるが、すべて記録の文・テストの確かめの文の中の語（`Ref7 は{remaining}`・`Ref6/7 付き`）。送る Reference を足す行は 0 |

`OnInstallFailure` の Reference0 は語 1 つのままで、その語が書けないとき `fileio` になっただけ（要件 5.4 のまま）。

## 4. `main.rs` の行数（要件 6.4）

| 版 | 行数 |
|---|---|
| 基点 `311e8863`（PR#322 の 3 行を含む） | 974 |
| 前回の版 `ce01252b` | 994 |
| HEAD `c338248a` | **1,000** |

コマンド: `wc -l < crates/areka/src/main.rs`・`git show <基点>:crates/areka/src/main.rs | wc -l`・`git show ce01252b:crates/areka/src/main.rs | wc -l`。上限 1,000 ちょうどで、残りは 0 行。この先 `main.rs` に 1 行でも足す仕事は、先に外へ出す必要がある。

基点からの差分は `git diff --numstat <基点>..HEAD -- crates/areka/src/main.rs` で +63／−37。そのうち 5.4（`7ecf6103`）の分は `git diff --numstat 7ecf6103~1 7ecf6103 -- crates/areka/src/main.rs` で +3／−0。足した 3 行は、コメント 1 行と `write_failure::check_boot_dirs(&cfg, &default_app_profile_dir()).map_err(write_failure::stop_boot)?;` の呼び出し（2 行に折り返し）だけ。確かめの中身は `write_failure.rs` に置いた。

## 5. 印の代わりの補いの仕組み（要件 3.5・9.3）

- 新しく足したコードのファイル 11 本（`git diff --name-only --diff-filter=A <基点>..HEAD`）のうち、テストでないものは `crates/areka/src/write_failure.rs` の 1 本だけ。中身は書けない失敗を集めて致命へ渡す机（`install`・`sender`・`fatal`・`stop_boot`・`take_receiver`・`log_only`・`tell_at_close`）と、5.4 で足した起動の前の書けるかの確かめ（`probe_dir`・`check_boot_dirs`）。確かめは在るいちばん深いフォルダを選んで `areka_user_data::open_dir_for_add` に渡すだけで、ファイルもフォルダも作らない。印の代わりに何かを残す物ではない。
- テストでない変更ファイル 19 本と `write_failure.rs` の計 20 本の足した行に、ファイル・フォルダ・レジストリへ書く呼び出し（`fs::write`・`fs::create*`・`fs::rename`・`fs::copy`・`File::create`・`OpenOptions`・`create_dir`・`RegSetValue`・`remove_file`）は **0 行**。
- `main.rs` の差分は、印を消せなかった失敗を返して告知を 1 つ選ぶ形に変えたことと、5.4 の確かめの呼び出しだけで、印の外に落ちたことを覚える場所は足していない。

## 6. emo2 の辞書と申し送り（要件 8.1・8.2・9.4）

- `vendors/` の差分は 0（2 節）＝配布形の `vendors/sample_ghost/emo2.nar` を含め、辞書は書き換えていない。
- `.kiro/steering/roadmap.md` の差分は足した行 1 行だけで、「winget-manifest-submission の手元の確かめで見つかった件」の節（861 行目から）の 874 行目に、「emo2 の辞書の持ち主（pasta の側）への申し送り」の項目を 1 つ足した（`OnInstallFailure` に `fileio` が届くようになったこと・理由ごとに台詞を分けてほしいこと）。HEAD でこの見出しの語に当たるのは 1 か所。
- 「次のリリースへの申し送り」の節には書いていない。あの節は `/release` が読み、版上げのファイルの外の仕事が残っていると止まるため。

## 7. 残り

- なし。5.4 の着地の後の取り直しはこの記録で済んだ。
- 申し送り: `main.rs` は 1,000 行ちょうど。次に `main.rs` へ足す仕事は、足す前に外へ出す。
