---
name: update
description: 'areka の依存ライブラリ（Rust のクレート）を調べ、互換の更新・下限の引き上げ・互換の無い更新（メジャー）の 3 つに分けて、推奨つきで一括で提案する。承認された分を、区分ごと・依存ごとのコミットにして検証し、PR を作って squash で統合する。Use when: /update, 依存の更新, cargo update, 依存ライブラリの最新化, 古い依存の洗い出し。DO NOT USE FOR: リリース（/release を使用。版上げの PR は版の行しか変えない）, 新しい依存を足す（その spec で行い、tech.md へ「意図的依存追加」を登記する）。'
disable-model-invocation: true
argument-hint: '[--check]（調べて提案するところで終える）'
---

# update — 依存ライブラリの更新と提案

pasta の `/update`（pasta の PR #185）を areka へ移したもの。areka には npm のフォルダが無いので、扱うのは Rust のクレートだけである。依存を、次の 3 つの区分に分けて扱う。

| 区分 | 変えるもの | 推奨の既定 |
|------|-----------|-----------|
| A. 互換の更新 | `Cargo.lock` と `THIRD-PARTY-NOTICES.md` だけ（`Cargo.toml` の要求は変えない） | 入れる |
| B. 下限の引き上げ | `Cargo.toml` の要求の版を、互換の範囲の最新へ上げる | 見送る（下の「推奨の決め方」） |
| C. 互換の無い更新 | メジャー（`0.x` は MINOR、`0.0.x` は PATCH）を上げる。`=` で固定した版を動かすのもここ。`Cargo.toml` と、必要ならコードを直す | 依存ごとに見積もって決める |

調べる対象に入れないもの: ワークスペースの自分のクレート（`dola`・`wintf` など `path` の依存）、`vendors/` のサブモジュール（ビルドの依存ではない）、GitHub Actions の版。

## 行う時期

- いちばんよいのは、リリースの後、次の版の作業を始めるときである。上げた依存が、その版の開発のあいだずっと試される。
- リリースの前に行うなら、このスキルの PR を統合し終えてから `/release` を始める。`/release` は段 1 の 1.2 で根の `Cargo.toml`・`Cargo.lock`・`THIRD-PARTY-NOTICES.md` を触る開いた PR があると止まり、段 3 の 3.2 で `Cargo.lock` の差分が版の行だけであることを確かめる。`/release` の実行中は行わない。
- `Cargo.lock`・`THIRD-PARTY-NOTICES.md`・`tech.md` の依存の行は、1 ウェーブに 1 本だけが持つ席である（`kiro-next` の Single-seat resources・`roadmap.md` の「1 ウェーブに 1 本だけの席」）。段 1 で席の持ち主を調べ、居れば提案の頭に書き、開発者の判断を待つ。

## 実行の約束

1. 作業ブランチで行う。main で実行されたら止まり、ハーネスのワークツリーで実行し直すよう求める。main へ直接 push しない。
2. 段 2 と段 3 では何も変えない。変えるのは、段 3 で承認された分だけである。
3. コミットは、A で 1 つ、B で 1 つ、C は依存ごとに 1 つずつにする。どのコミットも、それだけを戻せば元に戻る形にする（`Cargo.lock` の差分と、それに対応する `THIRD-PARTY-NOTICES.md` の差分を同じコミットに入れる）。`git add` にはファイルのパスを並べる。
4. 各コミットの前に、段 4 の「コミットごとの検証」を通す。全体テスト（`tools/test-all.ps1`）は、全部のコミットの後に 1 回だけ回す（何度も回さない）。
5. `cargo publish` を実行しない。版の表記（`[workspace.package]` の `version`・`[workspace.dependencies]` の `dola` の `version`）を変えない。

## 段 1. 始める前の確認

1. 現在のブランチが main でないこと、`git status --porcelain` が空であることを確かめる。どちらかが違えば止まる。
2. `git fetch origin` の後、`git merge origin/main` で main を取り込む。衝突したら `git merge --abort` して止まる。
3. 席の持ち主を調べる。
   - `gh pr list --state open --limit 200 --json number,title,files` で、`files` に根の `Cargo.toml`・`Cargo.lock`・`THIRD-PARTY-NOTICES.md` を持つ開いた PR を数える（`files` がちょうど 100 件の PR は `gh pr diff <番号> --name-only` で読み直す）。
   - `.kiro/steering/roadmap.md` の「1 ウェーブに 1 本だけの席」の行で、いまのウェーブに依存の席を使う spec が書いてあるかを読む。

## 段 2. 調べる（何も変えない）

次のコマンドを実行し、出力を区分に振り分ける。

| コマンド | 分かること |
|----------|-----------|
| `cargo update --dry-run --verbose` | A（`Updating`・`Adding` の行）。最後の `Unchanged … (available: …)` の行は、直接の依存の要求に止められている間接の依存（C の候補を上げれば動くことがある） |
| `cargo upgrade --dry-run --incompatible --pinned` | 直接の依存ごとの `old req`・`compatible`・`latest`。`latest` が `compatible` と同じ行は B の候補、違う行は C の候補。`=` の行（`rmcp`・`rectangle-pack`・`wintf-winmsg-executor`）は、`latest` が今と違えば C の候補 |
| `cargo deny check` | 勧告・ライセンス・取得元。勧告に当たった依存は、どの区分でも「入れる」を推奨にする |
| `cargo search image-webp --limit 1` | 公開版が 0.2.5 以上なら、根の `Cargo.toml` の `[patch.crates-io]` を外せる（`tech.md` の `image-webp` の項の「取り外し条件」）。C の候補として出す |

`cargo upgrade` と `cargo deny` が無ければ、`cargo install cargo-edit cargo-deny --locked` を案内して止まる。

`Adding` の行（新しく入る間接の依存）は、実際に `cargo update` するまで `cargo tree -i` で引けない。どの直接の依存が引き込んだかは、段 4 の A の後に `cargo tree -i <名前>` で調べて報告に書く。

### C の候補の見積もり

C の候補ごとに、次の 4 つを調べる。

1. **変更履歴**: 今の版から最新までの破壊的変更。crates.io の `repository` の GitHub の Releases か `CHANGELOG.md` を読む（crates.io の API は `curl -A` で User-Agent をつける）。
2. **areka での使い方**: その依存を使っているクレートとファイルの数と場所（`Grep` で `use <名前>::` や `<名前>::`）。破壊的変更が、その使い方に当たるかどうか。
3. **据え置きの理由と決まり**: `.kiro/steering/tech.md` の依存の一覧と、`Cargo.toml` のコメントに、今の版に留めている理由や、上げるときの手順が書いてあれば、それを示す。areka で特に当たるもの:
   - `windows`・`windows-core`・`windows-numerics` は同じ版の系列で揃える。片方だけ上げない。
   - `bevy_*`（`bevy_app`・`bevy_ecs`・`bevy_tasks`・`bevy_utils`）は 4 つ揃えて上げる。
   - `rmcp` は 3 つのクレート（`areka-mcp`・`areka-mcp-bridge`・`real-run-kit`）で同じ `=` に揃える。上げるときの検証と書き直す文書は `tech.md` の `rmcp` の項に書いてある。
   - `unicode-segmentation` を上げると Unicode の版が変わり、絵文字の切り方が動きうる。
4. **新しい版の `rust-version`**: 手元の `rustc --version` より新しいものを要求していないか。

## 段 3. 一括で提案する

すべての候補を、1 回で、推奨つきでチャットに示す。1 つずつ尋ねない。`AskUserQuestion` を使わない。

```
依存の提案（base: <origin/main の短い SHA>）

A. 互換の更新 — 推奨: 入れる
  <件数> 件（<目立つものを 5 つまで: 名前 今→新>）
  新しく入る依存: <名前> / なし（引き込んだ依存は入れた後に調べる）

B. 下限の引き上げ — 推奨: <入れる / 見送る>
  <名前>  <old req> → <new req>  <推奨の理由>

C. 互換の無い更新
  1. <名前>  <今> → <最新>  推奨: <入れる / 起票 / 見送る>
     破壊的変更: <areka に当たるものを 1〜3 行。当たらなければ「当たらない」>
     影響: <触るクレートとファイルの数と場所>
     例: <直す箇所が 1 つでもあれば、変更前と変更後のコードを 1 か所>
  2. ...

勧告: <名前と勧告の ID。無ければ「なし」>
席: <依存の席を持つ開いた PR・走っている spec。無ければ「なし」>

承認: 「推奨どおり」か、番号で指定（例: A と C1 だけ）
```

`--check` で実行されたときは、ここで終える。

### 推奨の決め方

- **A**: 入れる。新しく入る依存が `cargo deny check` に通らないときは、その依存を引き込んだ更新だけを `cargo update -p <名前> --precise <今の版>` で据え置く案にする。
- **B**: 見送る。`wintf` と `dola` は crates.io に出しており、根の `[workspace.dependencies]` の下限を上げると、使う側の選べる版が狭まる。上げるのは次のときだけである: 勧告の修正版を必須にしたい／areka が新しい版で入った API を使っている（`Cargo.lock` の版でしか試していないため、使っていれば下限が嘘になる）。
- **C**: 次の順に当て、最初に当てはまったものにする。
  1. 勧告の修正が新しいメジャーにしか無い → 入れる。
  2. `image-webp` の公開版が取り外し条件を満たした → 入れる（手順は `tech.md` の `image-webp` の項）。
  3. 破壊的変更が areka の使い方に当たらないか、直しが数ファイルで済む → 入れる（この PR で、依存ごとのコミット）。
  4. 据え置きの理由が書いてあり、その理由がまだ当てはまる → 見送る。理由を書く。
  5. 直しが公開 API・描画や ECS の土台（`windows`・`bevy_*` の系列など）に及ぶ → `/kiro-discovery` での起票を推奨にする。何を調べればよいかを 1 行添える。

## 段 4. 入れる（承認の後）

A → B → C の順に行う。区分（C は依存）ごとに、変える → コミットごとの検証 → コミットする。全部のコミットの後に、全体テストを 1 回回す。

### 変える

| 区分 | コマンド |
|------|---------|
| A | `cargo update`（据え置く依存があれば、続けて `cargo update -p <名前> --precise <今の版>`） |
| B | `cargo upgrade -p <名前>` |
| C | `cargo upgrade -p <名前> --incompatible`（`=` の依存は `--pinned` も）の後、コードを直す。`image-webp` の取り外しは `tech.md` の手順どおり |

C で版を上げたら、`.kiro/steering/tech.md` の依存の一覧の版の表記も合わせる。

### コミットごとの検証

どれも排他テストの机は要らない。

1. `cargo deny check`
2. `cargo about generate --workspace about.hbs -o THIRD-PARTY-NOTICES.md`（差分はこのコミットに入れる）
3. `cargo build --workspace --all-targets --locked`
4. C だけ: その依存を使うクレートの `cargo test -p <クレート>`（`--ignored` はつけない）。`rmcp` は `tech.md` の `rmcp` の項に挙げたテストも

### 全体テスト（全部のコミットの後に 1 回）

`pwsh -NoProfile -File tools/test-all.ps1 -License` をバックグラウンドで回す（約 19 分。x64 と i686 のテスト・整形の確かめ・`cargo deny`・`cargo about` を含む）。ほかのセッションの全体テストと重ならないよう、走行の間だけ `areka-impl-watch.exe` の排他テストの机を持つ。机の決まりと正本は `doc/impl-watch.md`、呼び方は `.claude/skills/release/procedure.md` の「机の段」と同じ形で、次だけを読み替える: `--name update-<YYYY-MM-DD>`、`--purpose "update"`、「段 7 に書く」は段 6 の報告に書く。机が使えないときは机なしで回し、訳を報告に書く。

走り終えたら、成否によらず `return` と `leave` を回してから判定する。作業木に差分が残っていたら（`THIRD-PARTY-NOTICES.md` など）、どのコミットの検証が抜けていたかを調べ、そのコミットに入れ直してから先へ進む。

### 失敗したとき

- **コミットごとの検証で A が落ちた**: どの依存の更新で壊れたかを、`cargo update -p <名前> --precise <今の版>` で 1 つずつ戻して突き止める。その依存だけを据え置いて A を入れ、据え置いた依存と原因を報告に書く。原因が areka 側の不具合なら、ここで直す。
- **コミットごとの検証で C が落ちた／直しが見積もりより大きくなった**: その依存の変更を `git restore` で捨て（コミット前なので消えるのはその依存の分だけ）、起票を推奨にして報告に回す。ほかの依存は続ける。
- **全体テストが赤**: 赤の段とテストの名前から、どのコミットが原因かを当て、そのコミットを `git revert` するか直す。回し直しは、赤が出たテストだけに絞る（`cargo test -p <クレート> <テストの名前>`、i686 の段なら `--target i686-pc-windows-msvc`）。触っていないクレートだけが赤なら、そこで止めて報告する。

### コミット

件名は次の形にする。

- A: `chore(deps): 互換の範囲で依存を更新する`
- B: `chore(deps): <名前> の下限を <版> に上げる`
- C: `chore(deps): <名前> を <版> に上げる`

本文に、上げた依存の一覧（A は件数と目立つもの）を書く。

## 段 5. PR を作って統合する

外へ出るコマンドは、1 回のツール呼び出しに単独の 1 行で打つ（`cd … &&` や `; echo` とつながない）。

1. `git push -u origin HEAD` の後、`gh pr create --base main` で PR を作る。題名は `chore(deps): 依存の更新（<YYYY-MM-DD>）`、本文は段 3 の提案のうち入れた分・見送った分・起票を勧める分と、全体テストの結果。
2. CI を待たずに `gh pr merge --squash --delete-branch` で統合する。成否はこのコマンドの結果だけで判定する（`main already used by worktree` の警告は失敗ではない）。
3. 自動モードの見張りが push や PR を止めたら、開発者の設定（`autoMode.allow`）に許可のルールを足してもらう。手順の側から設定は触らない。
4. `Cargo.lock` の変更が中継 `areka-mcp-bridge` の依存の木に及んだら（`cargo tree -p areka-mcp-bridge -e normal --prefix none` に出るクレートの版が動いたら）、`pwsh -NoProfile -File <作業木の絶対パス>\tools\mcp-bridge-install.ps1` で中継の exe を置き直す（`tech.md` の `rmcp` の項）。

## 段 6. 報告する

```
依存の更新: <統合した / 提案だけ（--check）>（PR: <URL>）
- 入れた: A <件数> 件・B <名前…>・C <名前 今→新…>
- 新しく入った依存: <名前（引き込んだ依存）>。無ければ「なし」
- 据え置いた: <名前と理由。無ければ「なし」>
- 起票を勧める: <名前と、調べること。無ければ「なし」>
- 全体テスト: <緑 / 赤の段と扱い>・机: <持った / 飛ばした訳>
- 中継の置き直し: <した / 要らなかった>
```
