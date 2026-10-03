# Requirements Document

## Project Description (Input)
リリースのたびに、areka のクレート群を同じ版で crates.io へ出したい（開発者「可能なら crates.io へのリリースも組み込んでほしい」・2026-10-02 `/kiro-discovery`）。今は `areka`・`dola`・`wintf` の 3 つが 0.0.1 で名前を押さえてあるだけで、`areka` が使う部品のクレートは `publish = false` のまま。crates.io は手元のパスだけの依存を受け付けないので、`areka` を新しい版で出すには部品もすべて出ている必要がある。部品のクレートを公開できる形に整え、手元の公開前の確認（`--dry-run`）、初回の手元からの公開と Trusted Publishing の設定の手順、以後 GitHub Release の公開の後に自動で crates.io へ出す自分の workflow（`crates-io.yml`）、そして「crates.io は部品と本体の公開のため・利用者の入れ方ではない」という説明を揃える。詳細は brief.md。

> **2026-10-03 要件討議の議題 3 で範囲を改めた**（開発者「wintf と dola 以外の areka 系をリリースまでする意義があまりない」）。crates.io へ版を出し続けるのは、areka の外でも使える汎用のライブラリ `wintf`・`dola` の 2 つだけにする。`areka` は 0.0.1 で名前を確保したまま据え置き、本体と部品（`areka-*`・`shiori-*` ほか）は出さない。部品の名前の確保もしない（名前の確保は `areka` だけで足りる）。本書は改めた後の範囲で書いている。

## Introduction

areka は 1 つのワークスペースの中で、本体（`areka`）と多くの部品のクレートに分けて作られている。このうち `wintf`（Windows の窓・描画・縦書きの土台）と `dola`（宣言的なアニメーションの記述と再生）は、areka の外でも使える汎用のライブラリである。`wintf` が依存するワークスペース内のクレートは `dola` だけで、`dola` はワークスペース内に依存を持たない。この 2 つは部品を 1 本も出さずに crates.io へ出せる。

areka の利用者向けの入れ方は配布の zip（後に winget）であり、`cargo install areka` ではない（32 ビットの補助 exe が付かない）。そのため本体と部品を crates.io へ出しても利用者の役に立たない。本体と部品は出さず、`areka` の名前は 0.0.1 のまま確保だけしておく。

本 spec は、次の 4 つを揃えるところまでを受け持つ。

1. 出すクレート（`wintf`・`dola`）と出さないクレートを、各クレートの設定にはっきり書き分ける。
2. 何も上げずに「出せるか」を確かめる手元の確認を用意し、緑にする。全体テストでも軽い形で毎回確かめる。
3. `wintf`・`dola` へ Trusted Publishing（GitHub Actions から、長く使える鍵を置かずに出す crates.io の仕組み）を設定する手順を書く。どちらも crates.io に既に在るので、手元から初回を出す必要は無い。
4. 以後のリリースでは、GitHub Release が公開された後に、自分の workflow が同じ版で `wintf`・`dola` を crates.io へ出す。

本 spec の完了は「手元の確認が緑・workflow と手順が揃った」時点である。実際の初めての自動の公開（`v0.0.2`）は `release-cycle` の初回の中で起きる。

crates.io に一度出した版は消すことも差し替えることもできない（取り下げ＝yank だけ）。そのため本書の要件は「上げる前に止まる」ことを重く扱う。

### 本書で使う言葉

- **公開する一覧**: crates.io へ版を出し続けるクレート。`wintf`・`dola` の 2 つ（どちらも 0.0.1 が crates.io に在る）。
- **出さない一覧**: ワークスペースの残りの 28 クレート（起票時点・`Cargo.toml` の members＝`crates/*` の 30 クレートから 2 つを除いたもの）。
  - 本体: `areka`（crates.io の 0.0.1 は名前の確保だけ。以後の版は出さない）
  - 本体の部品: `areka-*` の 15 クレート・`shiori-abi`・`shiori-host32-host`・`shiori-host32-ipc`（areka の中でしか意味を持たない）
  - 32 ビットの補助 exe: `shiori-host32-helper`
  - 試験用の DLL: `shiori-host32-testdll`・`shiori-host32-testdll-loadu`・`shiori4-testdll`
  - 試し掘りの場: `pilot`
  - テストだけが使う道具: `sample-ghost-kit`・`log-capture-kit`・`temp-path-kit`
  - 調査の道具: `ukadoc-survey`
- **公開前の確認**: 何も crates.io へ上げずに、公開する一覧のすべてを「その版で出したとしたら」包んで確かめる手元の確認。包むだけの形と、組み立てまでの形がある（要件 2.11）。
- **公開の段**: GitHub Release の公開の後に、同じ版を crates.io へ出す自分の GitHub Actions の workflow（`.github/workflows/crates-io.yml`）。

## Boundary Context

- **In scope**:
  - 各クレートの設定の欄（出す・出さないの印と理由）の整備と、`wintf` から `dola` への依存の版の指定。
  - 公開前の確認の用意と、本 spec の完了時点での緑。
  - 全体テスト（`tools/test-all.ps1`）への、包むだけの公開前の確認の段の追加（要件討議の議題 2 で決定）。
  - Trusted Publishing の設定の手順と、手で公開の段をやり直す手順（設定の実行は開発者が crates.io の画面で行う）。
  - 公開の段（`.github/workflows/crates-io.yml`・新規）。
  - 説明の文書: 根の `README.md`・配布物の `dist/README.txt`・`wintf`・`dola` の `README.md`・開発者向けの決めごと（`.kiro/steering/tech.md`）。
- **Out of scope**:
  - `areka` 本体と部品の crates.io への公開、部品の名前の確保（議題 3 で決定）。
  - 版の決め方と上げ方（ワークスペースで 1 つ・`release-cycle` が受け持つ）。
  - ビルド・zip・GitHub Release の作成（`release.yml`＝`release-ci-workflow` が受け持つ。本 spec は `release.yml` に触らない）。
  - crates.io 以外の置き場。
  - docs.rs での見た目の整備（出た後に必要なら別に）。
  - クレート間の依存の組み合わせの変更・各クレートのソースコードの変更。
  - テストの実行（テストの門は手元の全体テストのまま。公開の段はテストを回さない）。
- **Adjacent expectations**:
  - `release-ci-workflow` は、タグ `v{版}` のときに GitHub Release を**公開**の状態（下書きでない）で作る。そのうえで、Release の公開に成功した後に、公開の段を手で起動する口（`workflow_dispatch`）から版を渡して呼ぶ（要件討議の議題 1 で決定・`release-ci-workflow` の brief へ申し送り済み）。リポジトリ既定の権限（`GITHUB_TOKEN`）で作った Release は別の workflow の `release: published` を起こさず、`workflow_run` は crates.io の Trusted Publishing が受け付けないため、この呼び出しが公開の段の唯一の自動のきっかけになる。
  - `release-cycle` は版を +0.0.1 し、その中で公開前の確認（組み立てまでの形）を通す。Trusted Publishing の設定は初回のタグより前に済ませる（`wintf`・`dola` は crates.io に既に在るので、手元からの初回の公開は要らない）。
  - `winget-manifest-submission` は、`README.md`・`dist/README.txt` の本 spec が書く節に、winget での入れ方の行を後から足す。
  - 同じウェーブ C2 で `Cargo.toml` を触る spec は置かない（並走の約束）。

## Requirements

### Requirement 1: 出すクレートと出さないクレートの書き分け

**Objective:** As a 開発者, I want 各クレートの設定を見るだけで crates.io へ出すかどうかと、その理由が分かる, so that 出すつもりのないクレートを誤って出さず、出すクレートを出し漏らさない

#### Acceptance Criteria

1. The ワークスペースの設定 shall 公開する一覧の 2 クレート（`wintf`・`dola`）のそれぞれに、crates.io へ出す印を持たせる。
2. The ワークスペースの設定 shall 出さない一覧の 28 クレートのそれぞれに、出さない印を明示し、出さない理由をそのクレートの設定ファイルに 1 行のコメントで添える（`areka` は今の「出す」印を「出さない」へ改め、理由に「crates.io の 0.0.1 は名前の確保だけ・利用者は配布の zip で入れる」旨を書く）。
3. The ワークスペースの設定 shall 公開する一覧の各クレートに、空でない説明・ライセンス・リポジトリの欄を持たせる。
4. Where 公開する一覧のクレートがクレートの中に説明書（README）を持つ, the ワークスペースの設定 shall その説明書が crates.io のクレートの頁に載るようにする。
5. The ワークスペースの設定 shall クレート間の依存の組み合わせ（どのクレートがどのクレートに依存するか）と、外部のクレートの版を変えない（要件 1.7 の版の指定の書き足しは、組み合わせを変えないのでこれに当たらない）。
6. If 本 spec の変更で依存の版の固定を記録したファイル（`Cargo.lock`）の中身が変わる, then 開発者 shall その変更を取り込まずに作業を止め、原因を報告する。
7. The ワークスペースの設定 shall `wintf` から `dola` への通常の依存に、ワークスペースの版と一致する版の指定を持たせる（crates.io は版の指定の無いパスだけの依存を受け付けない）。

### Requirement 2: 公開前の確認（手元）

**Objective:** As a 開発者, I want 何も上げずに「この版で出せるか」を手元の 1 つの操作で確かめたい, so that 一度出したら消せない crates.io に、出せない組み合わせを途中まで上げてしまうことがない

#### Acceptance Criteria

1. When 開発者が公開前の確認を実行する, the 公開前の確認 shall 公開する一覧の 2 クレートを、依存の順（`dola` → `wintf`）に、その版で包んで確かめ、crates.io へは何も上げない。
2. The 公開前の確認 shall 同じ回で出される `dola` の新しい版に依存する `wintf` も、その `dola` が出されるものとして確かめる（その版の `dola` がまだ crates.io に無くても確かめられる）。
3. The 公開前の確認 shall 出さない一覧のクレートを対象に含めない。
4. If 公開の対象になるクレートの集まりが公開する一覧と食い違う（一覧の外のクレートが対象に入る、または一覧のクレートが対象から外れる）, then the 公開前の確認 shall 失敗として終わり、食い違うクレートの名前を示す。
5. If 包んだクレートのどれかが crates.io の 1 クレートあたりの大きさの上限（圧縮後 10 MB）を超える, then the 公開前の確認 shall 失敗として終わり、そのクレートの名前と大きさを示す。
6. If どれかのクレートが包めない、または組み立てに失敗する, then the 公開前の確認 shall 失敗として終わり、失敗したクレートの名前を示す。
7. The 公開前の確認 shall 成功か失敗かを終了コードで返す。
8. The 公開前の確認 shall リポジトリの中の追跡されたファイルを書き換えず、作業の残り物をワークスペースの `target\` の外に置かない。
9. When 本 spec を完了とする, the 公開前の確認 shall 完了時点のコミットで、包むだけの形と組み立てまでの形の両方で緑である。
10. If その版が crates.io に既に在るクレートがある（本 spec の完了時点の 0.0.1 の `wintf`・`dola`）, then the 公開前の確認 shall それを失敗とせず、既に在ることを名前つきで示して残りの確認を続ける（要件 4.8 と同じ扱い）。
11. The 公開前の確認 shall 包むだけの形（組み立てを省き、要件 2.3〜2.5・2.7・2.10 と要件 1.1〜1.3・1.7 の欄の漏れを確かめる）と、組み立てまでの形（要件 2.1〜2.10 のすべて）の 2 つを、引数で選べる 1 つの操作として持つ。
12. When 開発者が全体テスト（`tools/test-all.ps1`）を実行する, the 全体テスト shall 公開前の確認の包むだけの形を 1 つの段として毎回走らせ、それが失敗したら全体テストを失敗として終える（未コミットの変更がある作業中の作業木でも走る）。
13. The 公開の手順書と公開の段 shall 組み立てまでの形の公開前の確認を走らせる（要件 3.5・4.4）。

### Requirement 3: Trusted Publishing の設定と手での公開の手順

**Objective:** As a 開発者, I want `wintf`・`dola` を自動の公開へつなぐ設定と、止まったときのやり直しの手順を、迷わず辿れる形で持ちたい, so that 以後のリリースを機械に任せ、失敗しても慌てずに出し直せる

#### Acceptance Criteria

1. The 公開の手順書 shall リポジトリの中に置かれ、その場所が開発者向けの決めごと（要件 5.4）に書かれる。
2. The 公開の手順書 shall `wintf`・`dola` のそれぞれへ、リポジトリ `ekicyou/areka`・workflow のファイル名 `crates-io.yml` で Trusted Publishing を設定する手順を示し、それを最初の自動の公開（`release-cycle` の初回のタグ）より前に済ませることを書く。
3. The 公開の手順書 shall 公開の段が途中で止まったときに、同じ版を渡して公開の段を手で起動し直す操作を示す（要件 4.8 により、既に出たクレートは飛ばされる）。
4. The 公開の手順書 shall 公開の段の後に、公開する一覧の 2 クレートがその版で crates.io に在ることを確かめる操作を示す。
5. Where 公開の段が使えない（Trusted Publishing の不具合など）, the 公開の手順書 shall 手元から出す予備の手順を示し、その必達として、全体テストが緑であることと、組み立てまでの形の公開前の確認が緑であることを並べ、出す順番は手で並べず cargo に任せる。
6. The 公開の手順書 shall 予備の手順で使う crates.io の鍵を開発者の手元の cargo の設定にだけ置き、リポジトリにも GitHub の秘密の置き場にも置かず、使い終えたら取り消す（または期限を切る）手順にする。
7. The 公開の手順書 shall 認証の情報やリポジトリの接続先の URL を画面やログへ印字する操作を含めない。

### Requirement 4: 以後の自動の公開（公開の段）

**Objective:** As a 開発者, I want GitHub Release が公開された後に、同じ版の `wintf`・`dola` が自動で crates.io へ出る, so that リリースのたびに手元で公開の操作をしなくてよい

#### Acceptance Criteria

1. When `release-ci-workflow` の段が、ある版の GitHub Release を公開した後に公開の段をその版を渡して呼ぶ、または開発者が公開の段を版を指定して手で起動する, the 公開の段 shall その版で crates.io への公開を始める。
2. If 公開の段を呼ぶ・手で起動する以外の出来事（普通の push・タグの push・PR・Release の作成など）が起きる, then the 公開の段 shall 動き出さず、crates.io へ何も出さない。
3. If 渡された版と、そのタグのコミットのワークスペースの版が一致しない, then the 公開の段 shall 何も上げずに止まり、二つの版を示す。
4. The 公開の段 shall 上げ始める前に要件 2 の組み立てまでの形の公開前の確認を走らせ、それが失敗したら何も上げずに止まる。
5. If 公開する一覧のどれかのクレートが crates.io にまだ 1 つの版も無い, then the 公開の段 shall 何も上げずに止まり、そのクレートの名前と「要件 3.5 の予備の手順で出し、Trusted Publishing を設定する」ことを示す。
6. The 公開の段 shall crates.io への認証に Trusted Publishing だけを使い、長く使える crates.io の鍵をリポジトリにも GitHub の秘密の置き場にも置かない。
7. The 公開の段 shall 公開する一覧の 2 クレートを同じ版で、依存の順に crates.io へ出す。
8. If その版が crates.io に既に在るクレートがある（予備の手順で手元から出した後・途中で止まった回のやり直しなど）, then the 公開の段 shall そのクレートを飛ばして失敗とせず、まだ無いクレートだけを出す（同じ版での段のやり直しも、この振る舞いで残りだけを出して終える）。
9. If 公開の段が途中で失敗する, then the 公開の段 shall 失敗として終わり、出せたクレートと出せなかったクレートの名前を実行の記録に残す。
10. The 公開の段 shall GitHub Release とその添付物、および winget への提出の段に手を加えず、自分の失敗でそれらを取り消したり止めたりしない。
11. The 公開の段 shall 認証の情報やリポジトリの接続先の URL を実行の記録へ印字しない。
12. If 渡された版の GitHub Release が公開の状態で存在しない（下書き・未作成）, then the 公開の段 shall 何も上げずに止まり、その版を示す（Release より先に crates.io へ出さない）。

### Requirement 5: 説明の文書

**Objective:** As a 利用者と開発者, I want crates.io に何が在って何が無いのかと、利用者がどこから入れるべきかが文書で分かる, so that 利用者が `cargo install areka` を試して戸惑ったり、0.0.1 の古い名前の確保の版を本物と取り違えたりしない

#### Acceptance Criteria

1. The 根の `README.md` shall crates.io に版を出しているのは汎用のライブラリ `wintf`・`dola` だけであること、crates.io の `areka` は名前の確保（0.0.1）だけで本体と部品は出していないこと、`cargo install areka` は利用者の入れ方ではないこと（32 ビットの補助 exe が付かない）、利用者は配布の zip で入れることを書く。
2. The 配布物の `dist/README.txt` shall 要件 5.1 のうち利用者に関わる趣旨（入れ方は配布の zip・`cargo install areka` では使えない）を、配布物を受け取った人に向けた平易な言葉で書く。
3. The 説明の文書 shall winget での入れ方の行を、後から `winget-manifest-submission` が同じ節に足せる形にする（本 spec では winget の行を書かない）。
4. The 開発者向けの決めごと（`.kiro/steering/tech.md`） shall 公開する一覧の決め方（areka の外でも使える汎用のライブラリだけを出す・今は `wintf`・`dola`）、出さないクレートの扱い、公開前の確認の実行の仕方、公開の段の動き方、および手順書の場所を書く。
5. The 開発者向けの決めごと shall 新しいクレートをワークスペースに足すときに、出すか出さないかの印と理由を必ず書くことを決めごととして書く。
6. The `wintf`・`dola` の `README.md` shall 「名前の確保のための公開」とする今の文を改め、使える早期の版である（API はまだ安定していない）ことを書く。

## 要件討議の決定

1. **公開の段のきっかけ（議題 1）** → `release.yml` が Release の公開の後に `gh workflow run` で呼ぶ `workflow_dispatch`（要件 4.1・4.2・4.12・Adjacent expectations）。`release-ci-workflow`・`winget-manifest-submission` の brief へ申し送り済み。
2. **全体テストに公開前の確認を足すか（議題 2）** → 包むだけの形を全体テストに毎回、組み立てまでの形を手順書と公開の段に（要件 2.11〜2.13）。ギャップ分析の実測では、21 クレートを組み立てまで確かめると空の `target\` から約 12 分かかった（2 クレートになった今は短くなる見込み）。
3. **何を crates.io へ出すか（議題 3）** → `wintf`・`dola` だけ。`areka` は 0.0.1 の名前の確保のまま据え置き、本体と部品は出さない。部品の名前の確保もしない（`areka-` で始まる名前をまとめて持つ仕組みは crates.io に無く、名前の確保だけの公開を何本も出すのは好まれない。紛らわしさを防ぐ要は `areka` の名前で、それは確保済み）。もとの議題「`shiori-host32-helper`・`log-capture-kit`・`temp-path-kit` を出すか」と「部品の説明の欄の言葉づかい」は、部品を出さないので消えた。`wintf`・`dola` の README の「名前の確保のため」の文は、出す以上は直すのが自明なので要件 5.6 にした。
