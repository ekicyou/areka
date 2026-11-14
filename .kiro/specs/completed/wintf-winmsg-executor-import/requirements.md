# Requirements Document

## Project Description (Input)
`wintf` の土台のクレート `wintf-winmsg-executor`（メッセージループ・窓の生成・UI スレッドの async）は、開発者のフォークで、areka とは別のリポジトリ `ekicyou/wintf-winmsg-executor` に在り、crates.io の 0.0.5 を `=` で固定して使っている。困っているのは開発者と、`windows` まわりの依存を動かすセッションである。公開面に `HWND` を出して `windows = "0.62"` に頼るので、`wintf` と同じ変更で動かさないと型が合わなくなるが、別リポジトリのままだと「向こうで版を出す → areka が受ける」の 2 段になり、1 つの PR で確かめられない。2026-11-14 の `/kiro-discovery` で windows-rs 0.101 系への移行の行き先を「`windows` クレートを使わず、`windows-bindgen` で作った束縛で API を呼ぶ」と決めた（開発者の裁定）。このクレートは C の関数しか呼ばず小さいので移行の最初の的になり、どう転んでもその前に areka のリポジトリへ移しておく必要がある。

変えたいこと: 手元の `main`（`4f20275`）と同じ中身のクレートを `crates/wintf-winmsg-executor/` に置いてワークスペースの一員にし（動きは変えない）、areka の全クレートが crates.io の 0.0.5 でなくこの写しを使う。次のリリースで公開の段が `dola`・`wintf` と一緒に `wintf-winmsg-executor` をリリースの版で crates.io へ出せる形にし、元のリポジトリは移転したと分かる状態にし、`tech.md`・`structure.md`・`/release` の手順が新しい形を説明する。詳細は `brief.md`。

## Introduction
この spec は、別リポジトリに在る開発者の持ち物のクレート `wintf-winmsg-executor` を、動きを変えずに areka のリポジトリへ写し、crates.io への公開を areka の公開の段へ引き継ぐ。読み手は開発者と、後に続く windows-rs の移行の spec（先進坑 `pilot-windows-rs-next` と移行の本体）を回すセッションである。この spec が着地すると、`wintf` とその土台を 1 つの PR で一緒に動かして確かめられるようになる。

この spec は写すことと引き継ぐことだけを受け持つ。`windows` 0.62 から `windows-bindgen` の束縛への移し（2026-11-14 の開発者の裁定＝移行の行き先は `windows` クレートを使わない形）は、下流の spec が受け持つ。

## Boundary Context
- **In scope**:
  - 手元の `main`（`4f20275`）のクレートの中身（`src`・`examples`・`README.md`・`LICENSE-MIT`・`LICENSE-APACHE`・`Cargo.toml`）を `crates/wintf-winmsg-executor/` へ写し、その `Cargo.toml` を areka 向けに直すこと（版・`repository`・公開の扱い・`license`）。
  - areka の全クレートの依存を、crates.io の 0.0.5 からこの写しへ替えること（根の `Cargo.toml`・`crates/pilot/Cargo.toml`・`Cargo.lock`）。
  - crates.io への公開の引き継ぎ（`tools/crates-io.ps1` の公開の一覧と自分の検査・`.github/workflows/crates-io.yml` の注記・`doc/crates-io-publish.md`・スキル `/release` の手順のうち版上げで動かす行）。
  - steering の登記（`.kiro/steering/tech.md`・`.kiro/steering/structure.md`）。
  - 元のリポジトリ `ekicyou/wintf-winmsg-executor` の後始末（README に移転先を書く・archive）。開発者の許しを得てから行う。
  - 要件に名指しの無い、同じ事実の古い記述の書き換え（2026-11-14 要件の討議の議題 2）: スキル `/update` の `SKILL.md`（`=` で固定した行の一覧・自前のクレートの例・版の表記）・`structure.md` の「その他の最上位」の `crates-io.yml` の説明・`tech.md` の「UI スレッド基盤の外部クレ化」の文・`crates/pilot/Cargo.toml` の 0.0.5 を固定する旨の注記。2026-11-14 の設計の討議（議題 3）で、スキル `/update` の `SKILL.md` の crates.io に出しているクレートの文・`README.md` の crates.io に版を出しているクレートの文・`doc/README.md` の `crates-io-publish.md` の説明の 3 か所を足した。
- **Out of scope**:
  - クレートの動きと公開面の変更。
  - `windows` 0.62 からの移行（`windows-bindgen` の束縛へ移すのは先進坑 `pilot-windows-rs-next` と、その後に起票する移行の本体）。
  - edition 2021 から 2024 への引き上げ（写すだけにする＝上げない）。
  - フォーク元 `timokroeger/winmsg-executor` への追随。
  - `wintf` の `runtime/`（このクレートを使う側）の作り。
  - crates.io の Trusted Publishing の設定そのもの（開発者のログインが要る操作。手順書に書き、開発者に頼む）。
  - `wintf`・`dola` の公開の仕組みの作り替え。
  - crates.io へ実際に版を出すこと（出すのは次のリリースの公開の段）。
- **Adjacent expectations**:
  - 同じウェーブ C20 の兄弟の spec は依存を足さず、`Cargo.lock` の席はこの spec だけが使う（`roadmap.md` の「ウェーブ編成」C20 の約束 ⑸）。`audit-tools-scripts` はこのウェーブに入らず、この spec の `tools/crates-io.ps1` の書き換えの後に取る（同 ⑹・C21 の見込み）。
  - 下流の `pilot-windows-rs-next` は、この spec が置いた `crates/wintf-winmsg-executor/` の中身を独立の小さなワークスペースへ写して束縛へ移す手間を測る。`crates/pilot/Cargo.toml` と根の `Cargo.toml` には触らない（C21 の見込みの約束）。この spec はその前提＝ワークスペースの中に写しが在り、本線の全クレートがそれを使う状態を用意する。
  - `tech.md`・`structure.md`・根の `Cargo.toml`・`Cargo.lock` に触るほかの spec（本体のクレート分けの直列・見直しの spec・`/update`）とは触る行が少ないが、同じウェーブに置くなら後の方が取り込み直す。

## Requirements

### Requirement 1: クレートの中身の写し
**Objective:** As a 開発者, I want 元のリポジトリの `main` と同じ中身のクレートが areka の木に在ること, so that 動きを変えずに置き場所だけを移せる

#### Acceptance Criteria
1. The 取り込み shall 元のリポジトリの手元の写し `C:\home\maz\git\wintf-winmsg-executor` の `main`（`4f20275`）で追跡されているファイルのうち、`src` の下の全ファイル・`examples` の下の全ファイル・`README.md`・`LICENSE-MIT`・`LICENSE-APACHE`・`Cargo.toml` を `crates/wintf-winmsg-executor/` の下に同じ相対パスで置く。
2. The 取り込み shall 元のリポジトリの `.claude`・`.kiro`・`CLAUDE.md`・`.gitignore`・`Cargo.lock` を areka の木へ写さない。
3. The 写したクレート shall `src`・`examples`・`README.md`・`LICENSE-MIT`・`LICENSE-APACHE` の中身を、全体テストの整形の段（`cargo fmt --all`）が動かす分のほかは `4f20275` と 1 字違わず同じに保つ。
4. The 取り込みのコミット shall その本文に、写した元のリポジトリとコミット番号 `4f20275` を書く。
5. If 取り込みの時点で手元の写しの作業木が汚れているか、`main` の先頭が `4f20275` でないか、`main` が `origin/main`（判定の直前に GitHub から取りに行った〔`git fetch` した〕もの。作業木と `main` は動かさない）と食い違う, then the 取り込み shall ファイルを写さずに止まり、食い違いの中身を開発者へ報告する。
6. The 写したクレート shall 公開面（公開の型・関数・欄・モジュールの並び）と動きを、crates.io の 0.0.5 と同じに保つ。
7. The 写したクレート shall edition 2021 と、`windows` の 0.62 の系列への依存（機能 `Win32_Foundation`・`Win32_Graphics_Gdi`・`Win32_System_Threading`・`Win32_UI_WindowsAndMessaging`）と、`async-task` 4.7.1 への依存（既定の機能なし）を保つ。

### Requirement 2: areka 向けのマニフェスト
**Objective:** As a 開発者, I want 写したクレートのマニフェストが areka のワークスペースの決まりに合うこと, so that リリースの版で公開でき、ライセンスの表示も崩れない

#### Acceptance Criteria
1. The 写したクレート shall 版としてワークスペースの版（根の `Cargo.toml` の `[workspace.package]` の `version`）を使う。
2. The 写したクレート shall `license` を自分で `MIT OR Apache-2.0` と持ち、ワークスペースの `license`（`MIT`）を受け継がない。
3. The 写したクレート shall `repository` として areka のリポジトリ `https://github.com/ekicyou/areka` を示す。
4. The 写したクレート shall `description`・`readme`・`keywords`・`categories`・`include` と docs.rs の設定を `4f20275` のまま保ち、包んだ物に `LICENSE-MIT`・`LICENSE-APACHE`・`README.md` が入る。
5. The 写したクレート shall crates.io へ出せるクレートとして扱われ、公開前の確認の道具（`tools/crates-io.ps1`）の判定「一覧」「欄」「版」「大きさ」をすべて通る。
6. The 写したクレート shall 新しいクレートを足すときのワークスペースの決まり（`tech.md` の「crates.io への公開」）に従い、全体テストのライセンスの段（`cargo deny check`・`cargo about generate`）を落とさない。

### Requirement 3: ワークスペースへの組み込みと依存の置き換え
**Objective:** As a `windows` まわりの依存を動かすセッション, I want areka の全クレートが crates.io の 0.0.5 でなく木の中の写しを使うこと, so that `wintf` とその土台を 1 つの PR で一緒に動かして確かめられる

#### Acceptance Criteria
1. The areka のワークスペース shall 写したクレートをワークスペースの一員として組む。
2. The areka のワークスペース shall `wintf`・`areka-actor`・`areka-emo-text`・`shiori-host32-host`・`shiori-host32-helper`・`pilot` のすべてに、木の中の写しを使わせる。
3. The 根の `Cargo.toml` shall `[workspace.dependencies]` の `wintf-winmsg-executor` の行を、`dola` と同じ「版＋木の中の置き場」の形で持つ。
4. The `crates/pilot/Cargo.toml` shall 版の直書き（`"0.0.5"`）をやめ、ワークスペースの依存を使う。
5. The `Cargo.lock` shall crates.io 由来の `wintf-winmsg-executor` の項を持たない。
6. The `Cargo.lock` shall この spec の前と比べて、`wintf-winmsg-executor` の出どころと版のほかに、外部のクレートの項を増やさず版も変えない（`windows`・`windows-core`・`async-task` はそれぞれ今の 1 つの版のまま）。
7. When i686 の補助 exe（`shiori-host32-helper`）を組むとき, the areka のワークスペース shall 写したクレートを使って組み上げる。
8. When 開発者が全体テスト（`tools/test-all.ps1 -Format -License`）を回すとき, the areka のワークスペース shall 緑で終わる。写したクレートの単体テストと doctest（本物の MessageBox を出すテストを含む）は全体テストから外さず、UI テストとして排他テストの机で回す（2026-11-14 要件の討議の議題 1。並走で揺れが出たら、原因の題名の取り違えを起票する）。
9. When 全体テストのライセンスの段が第三者ライセンスの謝辞（`THIRD-PARTY-NOTICES.md`）を作り直すとき, the 謝辞 shall `wintf-winmsg-executor` の項に `MIT OR Apache-2.0` の表示と、フォーク元の著作権の表示（Timo Kröger と winmsg-executor の寄稿者）・フォークの著作権の表示を残す。

### Requirement 4: crates.io への公開の引き継ぎ
**Objective:** As a 開発者, I want 次のリリースで公開の段が `wintf-winmsg-executor` を `dola`・`wintf` と一緒にリリースの版で出せること, so that 別リポジトリで手元から版を出す手間が無くなる

#### Acceptance Criteria
1. The 公開前の確認の道具 shall 公開する一覧を `dola`・`wintf`・`wintf-winmsg-executor` の 3 つにする。
2. When 公開前の確認の道具が版を渡されて回るとき, the 公開前の確認の道具 shall `wintf-winmsg-executor` の版もリリースの版と同じことを確かめ、違えば止まる。
3. When 公開前の確認の道具が `-Pending` で回るとき, the 公開前の確認の道具 shall ワークスペースの版が crates.io にまだ無い `wintf-winmsg-executor` を、まだ出ていないクレートとして名前を出す。
4. The 公開前の確認の道具 shall 自分の検査（較正）を、公開する一覧が 3 つになった後も緑で通し、一覧から外れたクレート・出せないクレート・版の違い・大きさの超過をそれぞれ赤にできる力を保つ。
5. The 公開前の確認の道具 shall 公開の順（`wintf-winmsg-executor` を `wintf` より前に出すこと）を手で並べず、cargo に決めさせる今の形を保つ。
6. The 公開の段（`.github/workflows/crates-io.yml`）shall その注記で出すクレートを 3 つと示し、認証を Trusted Publishing だけにする今の形を保つ（長く使える鍵を置かない）。
7. The 取り込み shall この spec の間に crates.io へ版を出さない（組み立てまでの確かめに留める）。
8. The 取り込み shall crates.io に在る `wintf-winmsg-executor` の 0.0.1〜0.0.5 を取り下げない（yank しない）。
9. When 取り込みが本線に入った後のリリースのタグ `v{版}` が push され、crates.io の `wintf-winmsg-executor` の Trusted Publishing が設定済みのとき, the areka の CI（`release.yml` の緑を待つ公開の段 `crates-io.yml`）shall 開発者が手元から版を出さなくても、`wintf-winmsg-executor` を `dola`・`wintf` と一緒にリリースの版で crates.io へ出す（2026-11-14 開発者「こっちのCIでリリースが作れるようにしておきたい」）。この spec の間の確かめは、公開前の確認の道具の `-Verify`（組み立てまで・何も上げない）と `-Pending`（`wintf-winmsg-executor` を残りの名前として出す）に留める。

### Requirement 5: 手順書と steering と `/release` の手順
**Objective:** As a 開発者, I want 手順書・steering・`/release` の手順が新しい形を説明すること, so that 次のリリースで迷わず、版上げで動かす行を取りこぼさない

#### Acceptance Criteria
1. The 手順書 `doc/crates-io-publish.md` shall 出すものが `wintf`・`dola`・`wintf-winmsg-executor` の 3 つであること、出す順（`dola` と `wintf-winmsg-executor` の後に `wintf`・順は cargo が決める）、手元から出す予備の手順の対象が 3 つであることを示す。
2. The 手順書 shall `wintf-winmsg-executor` にも Trusted Publishing を設定する手順（crates.io の `wintf-winmsg-executor` の Settings に、持ち主 `ekicyou`・リポジトリ `areka`・ワークフロー `crates-io.yml`・environment は空を足す）と、取り込みが本線に入った後の最初のリリースのタグより前に済ませる必要を示す。
3. The 手順書 shall `wintf-winmsg-executor` の Trusted Publishing が済んでいないまま公開の段が回ったときに起きること（公開が途中で止まり、一部のクレートだけが上がった状態になりうること）と、設定を済ませた後のやり直し（手での起動・既に出たクレートは飛ばす）を示す。
4. The 手順書 shall crates.io の `wintf-winmsg-executor` の版が、次のリリースで 0.0.5 からリリースの版へ飛ぶことを示す。
5. The `tech.md` shall Key Libraries の `wintf-winmsg-executor` の項を、`=0.0.5` の固定でなく木の中の `crates/wintf-winmsg-executor/` を使う形に書き直し、「crates.io への公開」の節で出すクレートを 3 つにし、版を上げるときに動かす根の `Cargo.toml` の行に `wintf-winmsg-executor` の `version` を足す。
6. The `structure.md` shall `crates/wintf-winmsg-executor/` の置き場と役割（`wintf` の土台・crates.io へ出す・ライセンスは `MIT OR Apache-2.0`）を登記する。
7. The スキル `/release` の手順 shall 版上げで動かす根の `Cargo.toml` の行に `wintf-winmsg-executor` の `version` の行を足し、版を読む段・版を上げる段（代わりのやり方を含む）・上がった版の判定（足した行と消した行の数とその中身）・タグの前の確かめ・「次が変わったら見直す」の覚え書きを、その行の数に合わせる。
8. When `/release` の版を上げる段が回った後, the `/release` の手順 shall 根の `Cargo.toml` の `[workspace.package]` の `version`・`dola` の `version`・`wintf-winmsg-executor` の `version` の 3 つがどれもリリースの版でなければ止まる。
9. The この spec が書く文書 shall 日本語で書き、project の語をそのまま使う。
10. The この spec shall In scope の「要件に名指しの無い、同じ事実の古い記述」の 7 か所（要件の討議の 4 か所と設計の討議の 3 か所）を、新しい形（木の中の写し・公開するクレートは 3 つ）に書き換える。

### Requirement 6: 元のリポジトリの後始末
**Objective:** As a 開発者, I want 元のリポジトリが移転したと分かる状態になること, so that 元のリポジトリを見た人が新しい置き場へ辿れ、古い置き場へ変更が入らない

#### Acceptance Criteria
1. The 取り込み shall 元のリポジトリ `ekicyou/wintf-winmsg-executor` へ何かを書く前（README の変更の PR・archive）に、すること（README に書く文・archive の操作）を示して開発者の許しをチャットで得る。
2. If 開発者の許しが得られていない, then the 取り込み shall 元のリポジトリに何も書かず、後始末を残りとして完了の報告に載せる。
3. When 開発者が許したとき, the 元のリポジトリ shall README の先頭に、移転先（areka のリポジトリの `crates/wintf-winmsg-executor/`）と、以後の版は areka から出ることを示す。
4. When README の移転の案内が元のリポジトリの既定の枝に入った後, the 元のリポジトリ shall archive（読み取り専用）の状態になる。
5. The 取り込み shall 元のリポジトリの履歴とタグを消さない（履歴の置き場は元のリポジトリのまま）。

### Requirement 7: 完了の報告での申し送り
**Objective:** As a 開発者, I want 自分の手でしかできない操作が完了の報告で確実に伝わること, so that 次のリリースの公開の段が途中で止まらない

#### Acceptance Criteria
1. When この spec の完了を報告するとき, the 完了の報告 shall crates.io の `wintf-winmsg-executor` の Trusted Publishing の設定が開発者の操作として残っていること、その設定の値（持ち主 `ekicyou`・リポジトリ `areka`・ワークフロー `crates-io.yml`・environment は空）、取り込みが本線に入った後の最初のリリースのタグより前に済ませる必要を示す。
2. When この spec の完了を報告するとき, the 完了の報告 shall 元のリポジトリの後始末（README の移転の案内・archive）が済んだか残っているかを示す。
