# Requirements Document

## Project Description (Input)
リリースのたびに、areka のクレート群を同じ版で crates.io へ出したい（開発者「可能なら crates.io へのリリースも組み込んでほしい」・2026-10-02 `/kiro-discovery`）。今は `areka`・`dola`・`wintf` の 3 つが 0.0.1 で名前を押さえてあるだけで、`areka` が使う部品のクレートは `publish = false` のまま。crates.io は手元のパスだけの依存を受け付けないので、`areka` を新しい版で出すには部品もすべて出ている必要がある。部品のクレートを公開できる形に整え、手元の公開前の確認（`--dry-run`）、初回の手元からの公開と Trusted Publishing の設定の手順、以後 GitHub Release の公開の後に自動で crates.io へ出す自分の workflow（`crates-io.yml`）、そして「crates.io は部品と本体の公開のため・利用者の入れ方ではない」という説明を揃える。詳細は brief.md。

## Introduction

areka は 1 つのワークスペースの中で、本体（`areka`）と多くの部品のクレートに分けて作られている。crates.io は「手元のパスだけ」の依存を受け付けないので、`areka` を新しい版で crates.io へ出すには、`areka` が使う部品もすべて同じ版で crates.io に出ている必要がある。

本 spec は、次の 4 つを揃えるところまでを受け持つ。

1. 出すクレートと出さないクレートを、各クレートの設定にはっきり書き分ける。
2. 何も上げずに「出せるか」を確かめる手元の確認を用意し、緑にする。
3. 初回だけ開発者が手元から出す手順と、その後に各クレートへ Trusted Publishing（GitHub Actions から、長く使える鍵を置かずに出す crates.io の仕組み）を設定する手順を書く。
4. 以後のリリースでは、GitHub Release が公開された後に、自分の workflow が同じ版で crates.io へ出す。

実際の初回の公開（`v0.0.2`）は、本 spec ではなく `release-cycle` の初回の中で開発者が行う。本 spec の完了は「手元の確認が緑・workflow と手順が揃った」時点である。

crates.io に一度出した版は消すことも差し替えることもできない（取り下げ＝yank だけ）。そのため本書の要件は「上げる前に止まる」ことを重く扱う。

### 本書で使う言葉

- **公開する一覧**: `areka` と、`areka` が通常の依存（テスト専用の依存を除く）として直接または間接に使うワークスペース内のクレートの集まり。起票時点（2026-10-03・`Cargo.toml` の members＝`crates/*` の 30 クレート）で数えると次の 21 クレート。
  - 既に crates.io に名前がある 3 つ: `areka`・`dola`・`wintf`
  - 初めて出す 18 の部品: `areka-actor`・`areka-emo-atlas`・`areka-emo-compose`・`areka-emo-present`・`areka-emo-text`・`areka-ghost`・`areka-kanade`・`areka-mcp`・`areka-nar`・`areka-parsers`・`areka-sakura`・`areka-seriko`・`areka-sylphya`・`areka-talk`・`areka-update`・`shiori-abi`・`shiori-host32-host`・`shiori-host32-ipc`
  - `areka-mcp`（完了 `mcp-server-core` で加わった）は `areka` の通常の依存なので、公開する一覧に入る。
- **出さない一覧**: 上の一覧に入らない残りの 9 クレート。
  - 試験用の DLL: `shiori-host32-testdll`・`shiori-host32-testdll-loadu`・`shiori4-testdll`
  - 試し掘りの場: `pilot`
  - テストだけが使う道具: `sample-ghost-kit`・`log-capture-kit`・`temp-path-kit`（どれも他のクレートからはテスト専用の依存としてだけ使われる。テスト専用の依存は crates.io へ出すときに外れるので、出す必要が無い）
  - 調査の道具: `ukadoc-survey`
  - 32 ビットの補助 exe: `shiori-host32-helper`（`areka` の依存ではない。出すかどうかは要件討議の議題＝下の「要件討議で決める事項」1。本書は既定として出さない側に置く）
- **公開前の確認**: 何も crates.io へ上げずに、公開する一覧のすべてを「その版で出したとしたら」包んで組み立ててみる手元の確認。
- **公開の段**: GitHub Release の公開の後に、同じ版を crates.io へ出す自分の GitHub Actions の workflow（`.github/workflows/crates-io.yml`）。

## Boundary Context

- **In scope**:
  - 各クレートの設定の欄（出す・出さないの印、説明などの欄）の整備。出さないクレートには理由を 1 行添える。
  - 公開前の確認の用意と、本 spec の完了時点での緑。
  - 初回の手元からの公開の手順と、Trusted Publishing の設定の手順（どちらも実行は `release-cycle` の初回で開発者が行う）。
  - 公開の段（`.github/workflows/crates-io.yml`・新規）。
  - 説明の文書: 根の `README.md`・配布物の `dist/README.txt`・開発者向けの決めごと（`.kiro/steering/tech.md`）。
- **Out of scope**:
  - 版の決め方と上げ方（ワークスペースで 1 つ・`release-cycle` が受け持つ）。
  - ビルド・zip・GitHub Release の作成（`release.yml`＝`release-ci-workflow` が受け持つ。本 spec は `release.yml` に触らない）。
  - crates.io 以外の置き場。
  - `cargo install areka` で動く形にすること（32 ビットの補助 exe を同梱できない）。
  - docs.rs での見た目の整備（出た後に必要なら別に）。
  - クレート間の依存の組み合わせの変更・各クレートのソースコードの変更。
  - テストの実行（テストの門は手元の全体テストのまま。公開の段はテストを回さない）。
- **Adjacent expectations**:
  - `release-ci-workflow` は、タグ `v{版}` のときに GitHub Release を**公開**の状態（下書きでない）で作る。公開の段はその公開を受けて動く。`release.yml` はリポジトリ既定の権限で Release を作る予定なので、公開の段はそうして作られた Release でも確実に動き出す必要がある（要件 4.1）。
  - `release-cycle` は版を +0.0.1 し、その中で公開前の確認を通す。初回（`v0.0.2`）は、タグを打つ前に本 spec の手順で手元から公開し、Trusted Publishing を設定する。
  - `winget-manifest-submission` は、`README.md`・`dist/README.txt` の本 spec が書く節に、winget での入れ方の行を後から足す。
  - 同じウェーブ C2 で `Cargo.toml` を触る spec は置かない（並走の約束）。

## Requirements

### Requirement 1: 出すクレートと出さないクレートの書き分け

**Objective:** As a 開発者, I want 各クレートの設定を見るだけで crates.io へ出すかどうかと、その理由が分かる, so that 出すつもりのないクレートを誤って出さず、`areka` に要る部品を出し漏らさない

#### Acceptance Criteria

1. The ワークスペースの設定 shall 公開する一覧の 21 クレートのそれぞれに、crates.io へ出す印を持たせる。
2. The ワークスペースの設定 shall 出さない一覧の 9 クレートのそれぞれに、出さない印を明示し、出さない理由をそのクレートの設定ファイルに 1 行のコメントで添える。
3. The ワークスペースの設定 shall 公開する一覧の各クレートに、空でない説明・ライセンス・リポジトリの欄を持たせる。
4. Where 公開する一覧のクレートがクレートの中に説明書（README）を持つ, the ワークスペースの設定 shall その説明書が crates.io のクレートの頁に載るようにする。
5. The ワークスペースの設定 shall クレート間の依存の組み合わせ（どのクレートがどのクレートに依存するか）と、外部のクレートの版を変えない（要件 1.7 の版の指定の書き足しは、組み合わせを変えないのでこれに当たらない）。
6. If 本 spec の変更で依存の版の固定を記録したファイル（`Cargo.lock`）の中身が変わる, then 開発者 shall その変更を取り込まずに作業を止め、原因を報告する。
7. The ワークスペースの設定 shall 公開する一覧のクレートがワークスペース内のクレートへ持つ通常の依存のすべてに、ワークスペースの版と一致する版の指定を持たせる（crates.io は版の指定の無いパスだけの依存を受け付けない）。

### Requirement 2: 公開前の確認（手元）

**Objective:** As a 開発者, I want 何も上げずに「この版で全部出せるか」を手元の 1 つの操作で確かめたい, so that 一度出したら消せない crates.io に、出せない組み合わせを途中まで上げてしまうことがない

#### Acceptance Criteria

1. When 開発者が公開前の確認を実行する, the 公開前の確認 shall 公開する一覧の 21 クレートを、依存の順に、すべてその版で包んで組み立て、crates.io へは何も上げない。
2. The 公開前の確認 shall まだ crates.io に無い部品に依存するクレートも、その部品が同じ回で出されるものとして確かめる。
3. The 公開前の確認 shall 出さない一覧のクレートを対象に含めない。
4. If 公開の対象になるクレートの集まりが公開する一覧と食い違う（一覧の外のクレートが対象に入る、または一覧のクレートが対象から外れる）, then the 公開前の確認 shall 失敗として終わり、食い違うクレートの名前を示す。
5. If 包んだクレートのどれかが crates.io の 1 クレートあたりの大きさの上限（圧縮後 10 MB）を超える, then the 公開前の確認 shall 失敗として終わり、そのクレートの名前と大きさを示す。
6. If どれかのクレートが包めない、または組み立てに失敗する, then the 公開前の確認 shall 失敗として終わり、失敗したクレートの名前を示す。
7. The 公開前の確認 shall 成功か失敗かを終了コードで返す。
8. The 公開前の確認 shall リポジトリの中の追跡されたファイルを書き換えず、作業の残り物をワークスペースの `target\` の外に置かない。
9. When 本 spec を完了とする, the 公開前の確認 shall 完了時点のコミットで緑である。
10. If その版が crates.io に既に在るクレートがある（本 spec の完了時点の 0.0.1 の `areka`・`dola`・`wintf` など）, then the 公開前の確認 shall それを失敗とせず、既に在ることを名前つきで示して残りの確認を続ける（要件 4.8 と同じ扱い）。

### Requirement 3: 初回の手元からの公開と Trusted Publishing の設定の手順

**Objective:** As a 開発者, I want 初回（`release-cycle` の初回・`v0.0.2`）に手元から全部を同じ版で出し、その後に自動の公開へつなぐ手順を、迷わず辿れる形で持ちたい, so that 新しいクレートは crates.io に在らないと Trusted Publishing を設定できないという制約を越えて、以後のリリースを機械に任せられる

#### Acceptance Criteria

1. The 公開の手順書 shall リポジトリの中に置かれ、その場所が開発者向けの決めごと（要件 5.4）に書かれる。
2. The 公開の手順書 shall 手元から出す前の必達として、版を上げた後のコミットで全体テストが緑であることと、公開前の確認が緑であることを並べる。
3. The 公開の手順書 shall crates.io の鍵を開発者の手元の cargo の設定にだけ置き、リポジトリにも GitHub の秘密の置き場にも置かない手順にする。
4. The 公開の手順書 shall 公開する一覧の 21 クレートを同じ版で出す操作を示し、出す順番は手で並べず cargo に任せる。
5. The 公開の手順書 shall crates.io が新しいクレートの公開の速さに上限を設けている（起票時点の公表値で、続けて 5 つまで、その後は 10 分に 1 つ）ことと、初回は新しいクレートが 18 あるため途中で止められる見込みが高いことを書く。
6. If 初回の公開が途中で止まる（速さの上限・通信の失敗など）, then the 公開の手順書 shall 版を変えずに、まだ出ていないクレートだけを出し直す操作を示す。
7. The 公開の手順書 shall 初回の公開の後に、21 クレートすべてがその版で crates.io に在ることを確かめる操作を示す。
8. The 公開の手順書 shall 初回の公開の後に、21 クレートのそれぞれへ、リポジトリ `ekicyou/areka`・workflow のファイル名 `crates-io.yml` で Trusted Publishing を設定する手順を、クレートの名前の確認の一覧つきで示す。
9. The 公開の手順書 shall 初回に使った crates.io の鍵を、設定を終えた後に開発者が取り消す（または期限を切る）手順を含める。
10. The 公開の手順書 shall 認証の情報やリポジトリの接続先の URL を画面やログへ印字する操作を含めない。

### Requirement 4: 以後の自動の公開（公開の段）

**Objective:** As a 開発者, I want GitHub Release が公開された後に、同じ版の 21 クレートが自動で crates.io へ出る, so that リリースのたびに手元で公開の操作をしなくてよい

#### Acceptance Criteria

1. When ある版の GitHub Release が公開される（`release-ci-workflow` の workflow がリポジトリ既定の権限で公開した場合を含む）, the 公開の段 shall その版で crates.io への公開を始める。
2. If GitHub Release の公開でない出来事（普通の push・PR・下書きの Release の作成など）が起きる, then the 公開の段 shall 動き出さず、crates.io へ何も出さない。
3. If Release のタグが示す版と、そのコミットのワークスペースの版が一致しない, then the 公開の段 shall 何も上げずに止まり、二つの版を示す。
4. The 公開の段 shall 上げ始める前に要件 2 と同じ公開前の確認を走らせ、それが失敗したら何も上げずに止まる。
5. If 公開する一覧のどれかのクレートが crates.io にまだ 1 つの版も無い, then the 公開の段 shall 何も上げずに止まり、そのクレートの名前と「要件 3 の手元からの初回の手順で出す」ことを示す。
6. The 公開の段 shall crates.io への認証に Trusted Publishing だけを使い、長く使える crates.io の鍵をリポジトリにも GitHub の秘密の置き場にも置かない。
7. The 公開の段 shall 公開する一覧の 21 クレートを同じ版で、依存の順に crates.io へ出す。
8. If その版が crates.io に既に在るクレートがある（初回の手元からの公開の後・途中で止まった回のやり直しなど）, then the 公開の段 shall そのクレートを飛ばして失敗とせず、まだ無いクレートだけを出す（同じ版での段のやり直しも、この振る舞いで残りだけを出して終える）。
9. If 公開の段が途中で失敗する, then the 公開の段 shall 失敗として終わり、出せたクレートと出せなかったクレートの名前を実行の記録に残す。
10. The 公開の段 shall GitHub Release とその添付物、および winget への提出の段に手を加えず、自分の失敗でそれらを取り消したり止めたりしない。
11. The 公開の段 shall 認証の情報やリポジトリの接続先の URL を実行の記録へ印字しない。

### Requirement 5: 説明の文書

**Objective:** As a 利用者と開発者, I want crates.io に areka が在る理由と、利用者がどこから入れるべきかが文書で分かる, so that 利用者が `cargo install areka` で入れて「補助 exe が無くて動かない」状態に陥らない

#### Acceptance Criteria

1. The 根の `README.md` shall crates.io の areka は部品と本体の公開・名前の確保のためであり、`cargo install areka` では 32 ビットの補助 exe が付かないので利用者の入れ方ではないこと、利用者は配布の zip で入れることを書く。
2. The 配布物の `dist/README.txt` shall 要件 5.1 と同じ趣旨を、配布物を受け取った人に向けた平易な言葉で書く。
3. The 説明の文書 shall winget での入れ方の行を、後から `winget-manifest-submission` が同じ節に足せる形にする（本 spec では winget の行を書かない）。
4. The 開発者向けの決めごと（`.kiro/steering/tech.md`） shall 公開する一覧の決め方（`areka` が通常の依存として使うクレート）、出さないクレートの扱い、公開前の確認の実行の仕方、公開の段の動き方、および初回の手順書の場所を書く。
5. The 開発者向けの決めごと shall 新しいクレートをワークスペースに足すときに、出すか出さないかの印と理由を必ず書くことを決めごととして書く。

## 要件討議で決める事項

本書は次の点を既定の扱いで書いている。答えによって作業が変わるので、要件討議で確かめる。

1. **`shiori-host32-helper` を出すか**: `areka` の依存ではない bin だけのクレート（32 ビットの補助 exe）。既定＝出さない（出さない一覧に置き、理由を 1 行添える）。出す場合は公開する一覧が 22 になり、要件 2.4 の食い違いの判定の基準も「`areka` の依存＋このクレート」に変わる。
2. **全体テスト（`tools/test-all.ps1`）に公開前の確認を足すか**: 組み立てまで含めると、ギャップ分析の実測で空の `target\` から約 12 分（brief の「1〜2 分」より長い）。包むだけ（組み立てを省く）ならすぐ終わる。既定＝足さない（公開前の確認は手順書・`release-cycle`・公開の段で走らせる）。
3. **`log-capture-kit`・`temp-path-kit` を出さない一覧に置いたこと**: brief は名前の空きを確かめた中にこの 2 つを含めていたが、出さない一覧（brief の Desired Outcome 2）にも、`areka` の依存（同 1）にも挙げていない。どちらもテスト専用の依存としてだけ使われ、crates.io へ出すときに外れるので、「`areka` が依存する部品を出す」という決めごとに従って出さない側に置いた。
4. **公開する部品の説明の欄の言葉づかい**: 今の説明には spec の名前や内部の呼び名だけで書かれたものがある（例: `areka-emo-compose`・`areka-ghost`）。crates.io で誰でも読める欄なので、外の人が読んで役割が分かる言葉へ書き直すかどうか。既定＝書き直さない（要件 1.3 は空でないことだけを求める）。
