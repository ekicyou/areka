# Requirements Document

## Project Description (Input)
困っているのは、開発者と、後から文書を足すセッションである。今は文書の置き場が `doc/` と `docs/` の 2 つあり（`docs/` は wintf のクリック透過の説明 `click_through.md` の 1 つだけ）、`doc/` の中ではファイル名の書き方が混ざっている（大文字と下線の `CONSTITUTION.md`・`COMPAT_ARCHITECTURE.md`・`PASTA_PROFILE.md` と、小文字のハイフンつなぎの `impl-watch.md`・`pasta-real-run.md` など）。案内 `doc/README.md` には「足す・動かすときは案内の表に 1 行足す」と書いてあるが、機械の検査が無いので、置いた後に表に載らないまま散らかりうる。2026-11-04 の再測定（main `932406f9`）では、`doc/` の直下は 13 のファイルと 7 つのフォルダ、`doc/` と `docs/` を合わせて 129 ファイルである。

変えたいこと: 置き場を `doc/` の 1 つにし、名前の書き方の決まりを案内に書いて名前をそろえ、案内の表と実物の食い違いを決定論の試験 1 本で見つけ、動かしたパスを名指す所をどれも新しいパスへ向ける。正本は `brief.md`（「2026-11-04 棚卸㉞の再測定」の節を含む）。

## Introduction

文書の置き場の案内 `doc/README.md` は、完了した `areka-P0-roadmap-overview-html` が文書の棚卸のときに作った。そこで「歴史は `doc/archive/` の下だけ」「足す・動かすときは表に 1 行足す」という決まりができたが、決まりは書いてあるだけで、守られているかを誰も確かめていない。`docs/` はそのときの片付けで残った。

本 spec は、置き場と名前だけを整える（文書の中身は書き直さない）。動かすのは次の 4 つに限る。

| 今のパス | 新しいパス | 理由 |
|---|---|---|
| `docs/click_through.md` | `doc/click-through.md` | 置き場を `doc/` にまとめる・名前の決まりに合わせる |
| `doc/CONSTITUTION.md` | `doc/constitution.md` | 名前の決まりに合わせる |
| `doc/PASTA_PROFILE.md` | `doc/pasta-profile.md` | 名前の決まりに合わせる |
| `doc/shiori/equivalence_evidence.toml` | `doc/shiori/equivalence-evidence.toml` | 名前の決まりに合わせる |

再測定では、この 4 つを名指すコードは 0 で、名指すのは steering・根の `README.md`・`doc/` の中の文書・完了済みの spec だけである。名指しの最も多い `doc/COMPAT_ARCHITECTURE.md`（クレートの 18 ファイル・未完了の spec 64 本・steering 6 ファイル）は、同じウェーブ（C17）で並んで走る見直しの spec との約束により、本 spec では名前を変えず、例外として案内に載せ、改名を後続の spec へ渡す。

案内の表と実物を突き合わせる試験（以下「照合テスト」）を 1 本足し、いつもの全体テストで回す。これで、表に無いファイル・表にあるのに無いファイル・名前の決まりの破れが、足した人の枝で赤になり、散らかりが戻らない。

## Boundary Context

- **In scope**:
  - `docs/` の始末（要件 1）。
  - 名前の書き方の決まりと、決まりに合わない名前の改名と例外の表（要件 2）。
  - 案内 `doc/README.md` の置き方の決まり・直下の表・古い名前と新しい名前の対応の表、`doc/` の直下とフォルダの棚卸（要件 3）。
  - 照合テスト（要件 4）。
  - 動かしたパスを名指す所の直し（コード・テスト・道具・スキル・steering・未着手と着手中の spec・`doc/` の中の文書・根の `README.md` と `CLAUDE.md`）（要件 5）。
  - 同じウェーブの spec との約束と、`doc/COMPAT_ARCHITECTURE.md` の改名を受け持つ後続の起票（要件 6）。
- **Out of scope**:
  - 文書の中身の書き直し（置き場と名前と、それで食い違うパスの書き方だけを変える）。
  - `.kiro/` の下の整理。
  - `doc/ukadoc-coverage/` の生成物の作り方。
  - 根の `README.md` の中身（名指しの行のパスだけを直す）。
  - `doc/archive/` の下の文書の中身と名前（歴史として残す）。
  - 外のリポジトリの文書（ぱすたのゴーストの開発のリポジトリなどが古いパスを名指していても直さない。要件 3.4 の対応の表で辿れるようにする）。
  - `doc/COMPAT_ARCHITECTURE.md` の改名と、`doc/ssp-mcp/` の下の名前と置き場の変更（要件 6.1）。
  - `doc/shiori/fragments/` の下の名前の変更（要件 2.6 の例外）。
  - `doc/` の直下のファイルをフォルダへまとめ直すこと（要件 3.5）。
  - フォルダの中のファイル 1 つずつを `doc/README.md` の表に載せること（フォルダの中は、表のフォルダの行の役目と、あればそのフォルダの `README.md` が受け持つ）。
  - 歴史の場所（`.kiro/specs/completed/` の下・`doc/archive/` の下・`.kiro/steering/roadmap-history.md`）の中の古いパスの書き換え（要件 5.3）。
  - 古いパスと古いファイル名が戻らないかを毎回の試験で見張ること（確かめるのは完了のときの 1 回だけ・要件 5.4。ファイルはもう無いので、戻っても照合テストが別の形で赤にする）。
- **Adjacent expectations**:
  - 同じウェーブの見直し 7 本（`audit-parsers-sakura`・`audit-emo-present`・`audit-wintf-graphics-world`・`audit-ghost-catalog`・`audit-wintf-runtime`・`audit-emo-compose`・`audit-seriko`）は、受け持つファイルで `doc/`・`docs/` を名指す行を足さず、変えない（roadmap のウェーブ C17 の約束）。本 spec はそれを当てにし、見直しの受け持つファイルに触らない。
  - 後から文書を足す spec と道具（俯瞰の頁を `doc/roadmap-overview-{YYYY-MM-DD}.html` へ書く `/kiro-next` の道具、`doc/release-notes/` へ書くリリースの手順、`doc/ideas/` へ書く `/kiro-discovery`、`doc/ukadoc-coverage/` を書き換える `ukadoc-catalog-refresh`・`coverage-roadmap-refresh`）は、要件 2 の名前の決まりと要件 3.1 の置き方の決まりに従う。今の名前の付け方はどれも決まりに合う。
  - `shiori4-api` は `doc/shiori/fragments/_shared.toml` を書き換える。本 spec は `doc/shiori/fragments/` の下の名前を変えないので、ぶつからない。
  - 本体のクレート分けの 6 本（`placement-crate-split` から始まる直列）は、`doc/COMPAT_ARCHITECTURE.md`・網羅台帳の根拠の場所・`structure.md` を書き換えるので、本 spec と同じウェーブには置かない（roadmap の約束）。

## Requirements

### Requirement 1: 文書の置き場を `doc/` の 1 つにまとめる
**Objective:** As a 後から文書を足すセッション, I want 文書の置き場がリポジトリに 1 つだけであること, so that どこに置くか・どこを探すかで迷わない

#### Acceptance Criteria
1. The 本 spec の作業 shall `docs/click_through.md` を `doc/click-through.md` へ移し、`docs/` のフォルダをリポジトリから無くす。
2. When `docs/click_through.md` を移すとき, the 本 spec の作業 shall 中身を変えない（変えてよいのは、移したことで食い違うパスの書き方だけ）。
3. The 本 spec の作業 shall `docs/click_through.md` を wintf のクレートの中へは移さない（中身が `crates/areka/src/main.rs` の結線も述べるリポジトリの開発者向けの説明であり、wintf の利用者向けの説明ではないため。また、同じウェーブで wintf を受け持つ見直しのファイルと重ねないため）。

### Requirement 2: ファイル名の書き方の決まり
**Objective:** As a 開発者, I want ファイルとフォルダの名前の書き方が 1 つの決まりでそろっていること, so that 名前の書き方で迷わず、新しく足す名前も同じ形になる

#### Acceptance Criteria
1. The 文書の案内 shall 名前の決まりを 1 つ書く: 名前は小文字の英字・数字・ハイフン・点だけで書き、語はハイフンでつなぐ（拡張子・版の番号・日付・並びの番号の点とハイフンは含めてよい）。各フォルダの案内の `README.md` だけは決まりの外とする。
2. The 名前の決まり shall `doc/` の下のファイルとフォルダのうち、`doc/archive/` の下を除く全部に効く（`doc/archive/` の下は歴史として名前も残す）。
3. The 本 spec の作業 shall 決まりに合わない次の 3 つの名前を変える: `doc/CONSTITUTION.md` → `doc/constitution.md`、`doc/PASTA_PROFILE.md` → `doc/pasta-profile.md`、`doc/shiori/equivalence_evidence.toml` → `doc/shiori/equivalence-evidence.toml`（`docs/click_through.md` は要件 1.1 で決まりに合う名前になる）。
4. When 名前を変えるとき, the 本 spec の作業 shall 中身を変えない（変えてよいのは、改名で食い違う自分の名前とパスの書き方だけ。`doc/CONSTITUTION.md` の見出しの語「CONSTITUTION」はファイル名ではなく中身の語なので変えない）。
5. The 文書の案内 shall 決まりに合わないまま残す名前を例外の表に 1 行ずつ載せ、各行に理由と行方（後で改名する／改名しない）を書く。
6. The 例外の表 shall 次の 2 行で始まる: `doc/COMPAT_ARCHITECTURE.md`（理由＝同じウェーブの約束で本 spec では変えない・行方＝後で改名する・要件 6.5）と、`doc/shiori/fragments/` の下の全部（理由＝名前と結合の順を目録 `_manifest.toml` が決める、道具が読むデータの目録であるため・行方＝改名しない）。
7. The 例外の表 shall フォルダ単位の行を、中の名前を読む側のデータや道具が決めるフォルダにだけ使い、ほかはファイル 1 つずつの行で載せる。

### Requirement 3: 案内に置き方の決まりを書き、表を実物に合わせる
**Objective:** As a 後から文書を足すセッション, I want 文書をどこにどの名前で置き、置いた後に何をするかが案内の 1 枚に書いてあること, so that 置いた文書が案内から辿れる

#### Acceptance Criteria
1. The 文書の案内 shall 置き方の決まりを書く: 文書は `doc/` の下に置き、`docs/` などほかの置き場を作らない／歴史は `doc/archive/` の下だけに置く／名前は要件 2 の決まりに従う／`doc/` の直下にファイルやフォルダを足す・動かす・消すときは表の行を足す・直す・消す。
2. The 文書の案内 shall 今ある「決まりは書くだけで、機械の検査は無い」を、照合テストが表と実物と名前の決まりを確かめることと、どの試験を回せば確かめられるかに書き替える。
3. The 文書の案内 shall `doc/` の直下のファイルとフォルダの 1 つずつに表の 1 行を持ち、各行に役目と「今も効く／歴史」を書く。日付つきの俯瞰の頁は、今のとおり `roadmap-overview-{YYYY-MM-DD}.html` の型の 1 行で載せる。
4. The 文書の案内 shall 本 spec が動かした 4 つについて、古いパスと新しいパスの対応を表にして残す（歴史の記録に残る古いパスから今のファイルを辿れるように）。
5. The 本 spec の作業 shall `doc/` の直下とフォルダを棚卸し、表の各行の役目の書き方が実物と食い違っていれば直す。置き場や名前を変えるのは要件 1 と要件 2 の 4 つだけとし、直下のファイルをフォルダへまとめ直すことはしない。
6. The steering の構成の文書（`.kiro/steering/structure.md`） shall 文書の置き場と名前の決まりの正本として `doc/README.md` を名指し、最上位の並びから `docs/`（単発の技術メモ）の項を消す。

### Requirement 4: 案内の表と実物を照合する試験
**Objective:** As a 開発者, I want 案内の表と実物の食い違いと名前の決まりの破れを、いつもの全体テストが見つけること, so that 散らかりが戻らない

#### Acceptance Criteria
1. The 照合テスト shall `doc/` の直下にあるファイルとフォルダの集まりと、案内の直下の表の行の集まりを突き合わせる。
2. If `doc/` の直下に表のどの行にも当たらないファイルかフォルダがあるとき, then the 照合テスト shall 赤になり、その名前と「表に行を足すか、置き場を直す」ことを出す。
3. If 表の行が名指すファイルかフォルダが無いとき, then the 照合テスト shall 赤になり、その行の名前と「行を消すか直す」ことを出す。
4. When 表の `{YYYY-MM-DD}` の型の行を読むとき, the 照合テスト shall `{YYYY-MM-DD}` を 4 桁・2 桁・2 桁の数字をハイフンでつないだ日付の形として読み、その形に当たる直下のファイルを表に載っているものとして数え、当たるファイルが 0 でも赤にしない（俯瞰の頁を置くかは任意のため）。
5. If リポジトリの根に `docs/` があるとき, then the 照合テスト shall 赤になり、`doc/` へまとめることを出す。
6. If `doc/` の下（`doc/archive/` の下と、例外の表のフォルダの行の下を除く）に、名前の決まりに合わず例外の表にも無いファイルかフォルダがあるとき, then the 照合テスト shall 赤になり、その名前を出す。
7. If 例外の表の行が名指すものが無いか、もう名前の決まりに合っているとき（フォルダ単位の行では、その下に名前の決まりに合わない名前が 1 つも無くなったとき）, then the 照合テスト shall 赤になり、その行を消すよう出す（例外は今そこにある事情だけを表し、減る向きにだけ動く）。
8. If 実物の数え上げ・直下の表の読み取り・例外の表の読み取りのどれかが 1 件も拾わないとき, then the 照合テスト shall 赤になる（壊れて何も見ていないのに緑になることを防ぐ）。
9. The 照合テスト shall 表から既知の 1 行を外すとその 1 件で赤になることと、決まりに合わない名前を 1 つ加えるとその 1 件で赤になることを、実物のデータで確かめる自己検査を持つ。
10. The 照合テスト shall リポジトリの中のファイルだけを読み、ネット・時刻・走る順に左右されず、何度回しても同じ結果を出す。
11. The 照合テスト shall いつもの全体テスト（`tools/test-all.ps1`）の中で走る。

### Requirement 5: 動かしたパスを名指す所を直す
**Objective:** As a 後のセッション, I want 動かした文書のパスを名指す所がどれも新しいパスを指すこと, so that 名指しを辿って文書が見つからないことが起きない

#### Acceptance Criteria
1. The 本 spec の作業 shall 要件 1 と要件 2 で動かした 4 つの古いパスと古いファイル名を、リポジトリ全域で探す（相対のリンクと、パスを付けないファイル名だけの書き方を含む）。
2. The 本 spec の作業 shall 見つかった名指しのうち、5.3 の場所の外にあるものを全部、新しいパスへ直す（コード・テスト・道具・`.github/workflows/`・スキル・steering・未着手と着手中の spec・`doc/` の中の文書・根の `README.md` と `CLAUDE.md`）。
3. The 本 spec の作業 shall 次の場所の古いパスを直さずに残す: 歴史の場所（`.kiro/specs/completed/` の下・`doc/archive/` の下・`.kiro/steering/roadmap-history.md`）と、古い名前を本 spec の仕事として述べる所（`.kiro/specs/doc-tree-tidy/` の文書・roadmap で本 spec の仕事を述べる行〔台帳・版の段・ウェーブの編成・棚卸の記録・チェックの行〕・案内の古いパスと新しいパスの対応の表）。
4. When 本 spec の作業を終えるとき, the 本 spec の作業 shall 5.3 の場所の外で古いパスと古いファイル名を探して 0 件であることを確かめ、その前に、同じ探し方で歴史の場所の中の古いパスが見つかることを確かめて、探し方が空振りしていないことを示す。
5. The 本 spec の作業 shall 文書を実際に開くコードとテスト（`doc/ukadoc-coverage/`・`doc/ssp-mcp/`・`doc/release-notes/` を読むもの）の振る舞いを変えず、全体テストが緑であることを確かめる。
6. When 本 spec を完了するとき, the 本 spec の作業 shall `doc/PASTA_PROFILE.md` を名指すほかのリポジトリを持つ「ぱすた開発」のセッションへ、新しいパス `doc/pasta-profile.md` と案内の対応の表の場所を知らせる（外のリポジトリの直しは向こうの仕事）。

### Requirement 6: 同じウェーブで並んで走る spec との約束
**Objective:** As a 同じウェーブで並んで走る見直しの spec, I want 本 spec が自分たちの受け持つファイルと文書の名指しに触らないこと, so that 互いの変更がぶつからず、それぞれ単独で着地できる

#### Acceptance Criteria
1. The 本 spec の作業 shall `doc/COMPAT_ARCHITECTURE.md` の名前と、`doc/ssp-mcp/` の下のファイルとフォルダの名前と置き場を変えない。
2. The 本 spec の作業 shall 同じウェーブの見直し 7 本（`audit-parsers-sakura`・`audit-emo-present`・`audit-wintf-graphics-world`・`audit-ghost-catalog`・`audit-wintf-runtime`・`audit-emo-compose`・`audit-seriko`）が受け持つファイルを変えない。
3. The 本 spec の作業 shall 依存を足さず（`Cargo.lock` を変えない）、文字の層のクレート（`crates/areka-emo-text/`）にファイルを足さない。
4. The 照合テスト shall 6.2 の見直しが受け持つクレートの外に置く。
5. When 本 spec を完了するとき, the 本 spec の作業 shall `doc/COMPAT_ARCHITECTURE.md` の改名を受け持つ後続の spec を `/kiro-discovery` で起票し、例外の表のその行の行方にその spec の名前を書く。後続の brief には、`doc/COMPAT_ARCHITECTURE.md` を名指すファイルを受け持つ見直しの spec と本体のクレート分けの spec とは同じウェーブに置かないことを書く。
