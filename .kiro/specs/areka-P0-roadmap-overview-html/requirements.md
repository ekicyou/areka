# Requirements Document

## Project Description (Input)

**誰の何が困っているか**: areka の開発者。ロードマップの正本 `.kiro/steering/roadmap.md` は 1,000 行を超え、1 行が数千字の表もあるので、版の段（v0.0.3〜v0.4.0・いつか・windows-reactor 待ち・版に縛らない）・テーマ・依存の順・区分を一目で見渡せない。`doc/` には古い記載が残り、どれが今も効いているのか分かりにくい（M1 期の議論・旧 ROADMAP のポインタ・旧 ARCHITECTURE など）。

**今の状態**: 俯瞰に要る材料は roadmap.md の 4 か所（「版の段」の表・「spec 台帳」の表・「テーマ別の決めごと」の各節の `#### Specs (dependency order)`・「ウェーブ編成」の表）に散っている。HTML を作る道具は無い（`tools/` は PowerShell の道具が並ぶ）。`doc/` は全部で 119 ファイルで、コードかテストが読むために消すと壊れるもの（`doc/ukadoc-coverage/`・`doc/ssp-mcp/` の 2 ファイル・`doc/release-notes/`・`doc/impl-watch.md`）と、steering・スキル・brief から名前が出るだけのものが混ざっている（brief の「`doc/shiori/equivalence_evidence.toml` をコードが読む」は誤り＝下の事実）。

**何を変えるか**: ① roadmap.md から、版の段ごとに spec を並べた 1 枚の自己完結の HTML を作り直す道具を作る（手書きの HTML は置かない）。② `doc/` の古い記載を棚卸し、開発者が選んだものを消す・`doc/archive/` へ寄せる。消した・寄せたファイルを参照していた所は同じ変更で直す。

> 起票: 2026-10-15 `/kiro-discovery`（「版の段」の続き・開発者「html でロードマップを俯瞰できる資料を作ってもらえないか？ doc フォルダの整理もかねて」「俯瞰図 spec を最初に実施する」）。要件生成: 2026-10-15（main `c670b612`）。要件討議で main `ce08088c`（Fable の列の付け直し）を取り込み、下の事実が変わらないことを確かめた。詳細は brief.md。

## Introduction

本仕様は、開発者がロードマップを一目で見渡せる 1 枚の HTML を、roadmap.md から作り直せる道具と、`doc/` の古い記載の整理の 2 つを受け持つ。どちらも「今の姿だけを見せる」ための手入れで、roadmap.md の中身（版の段・台帳）は決め直さない。HTML は手で書かず、いつでも今の roadmap.md から作り直せることを要とする（手書きの 1 枚は roadmap が動くたびに古くなり、「古い記載の doc を消す」と逆向きになるため）。

要件生成時に引き直した事実（設計はこれを再検証する）:

- roadmap.md（main `c670b612`・`updated_at: 2026-10-15`）の「版の段」の表は 8 行（v0.0.3・v0.1.0・v0.2.0・v0.3.0・v0.4.0・いつか・windows-reactor 待ち・版に縛らない）。spec の列は `／` で組に分かれ、組には「動く＝」「配れる＝」「後続: シェル内バルーン＝」のような小見出しが付くものがある。`→` が依存の順、`・` が並び、名前の後ろの括弧書き（中に `〔〕` が入れ子になる）が注、行の末尾の「＝**N 本**」が手書きの数（今は 5＋40＋11＋5＋1＋28＋4＋20 ＝ 114）。
- 「spec 台帳」の表の列は「spec・段・何をするか・規模・並び・前提・Fable」の 7 つで、114 行。段の列は「優先・区分 A」のように段と区分を 1 つのセルに持つ（今の実数え＝優先 A 33・優先 C 10・その他 A 11・その他 C 37・バグ B 10・据え置き D 12・保留 D 1）。
- 「テーマ別の決めごと」の各 `###` 節の `#### Specs (dependency order)` には `- [ ] areka-P0-{名前} -- {一言}。Dependencies: {前提}` の形の行が並ぶ（チェック済み `- [x]` は 39 行）。同じ節の `#### Existing Spec Updates` にも `- [ ]`・`- [x]`・`- [→]` の行（29 行）が在るが、これはほかの spec への申し送りで、その節に属する印ではない。台帳の 114 本のうち 30 本は、どの節の `#### Specs (dependency order)` にも載らない（`balloon-canon-residue`・`shiori4-api`・`tick-gate-adoption`・`property-name-case-fold` など。ギャップ分析の数え直し）。節に載るが台帳に無い未完了の spec は 0 本。
- 「ウェーブ編成」の表の最初の行が、いま走っているウェーブ C6（8 本）。2 行目以降は「次のウェーブの候補」「ウェーブの外」「保留」で、着手中ではない。
- `tools/release-notes.ps1` が、Markdown から文書を作る PowerShell の道具の前例（文言を `release-notes.strings.json` に外出し・端末へは ASCII だけ・`-SelfTest` を `tools/test-all.ps1` の 1 段として回す・同じ入力からは同じバイト）。リポジトリに入れた生成物を、作り直した物と突き合わせるテストの前例は `doc/ukadoc-coverage/report/`（`crates/ukadoc-survey/tests/consistency/` が古さを赤にする）。
- `doc/` は 119 ファイル（直下 12 ファイルと 6 フォルダ）。コードかテストがファイルとして読む・テストが場所の名前を突き合わせるのは、`doc/ukadoc-coverage/` の全体（`crates/ukadoc-survey/src/io/paths.rs` ほか）・`doc/ssp-mcp/tools-list-ssp-2.9.05.json` と `doc/ssp-mcp/log-convention.md`（areka-mcp と areka のテスト）・`doc/release-notes/`（`tools/release-notes.ps1` と `.github/workflows/release.yml`）・`doc/impl-watch.md`（areka-impl-watch のエラー文とそのテストが場所の名前を持つ）の 4 か所。根の `README.md` が `doc/images/emo2-boot.png` を表示する。
- **brief と roadmap.md の「ロードマップの俯瞰図と doc の整理」節の誤り**: 「`doc/shiori/equivalence_evidence.toml` は `crates/ukadoc-survey/src/evidence/resolve.rs` と網羅台帳 `doc/ukadoc-coverage/ledger/shiori.toml` が読む」は事実でない。`git grep equivalence_evidence` のヒットは `doc/shiori/README.md`・そのファイル自身・この 2 か所の記載・完了した spec の文書だけ。`resolve.rs:41` は `doc/shiori/fragments/` を doc コメントで名指すだけで、`ledger/shiori.toml` の 4 か所も自由文の注。`doc/shiori/` を読むコードとテストは 0 件。ただし、まだ完了していない `shiori4-api` の brief が `doc/shiori/fragments/` を「紙の契約の正本」と呼ぶ。
- `doc/ROADMAP.md`（18 行のポインタ）を名指すのは、`focus.md:63`・`workflow.md:89`・スキル `kiro-complete` の `SKILL.md`（339・350・412 行＝並行レーンで書いてよいファイルの一覧）と、歴史の記録。
- リポジトリの根に `doc/` とは別の `docs/`（`click_through.md` の 1 ファイル・2026-09-24）が在る。

## Boundary Context

- **In scope**: ① roadmap.md から俯瞰の HTML を作る道具と、その自分の検査（全体テストの 1 段）／② HTML の置き場と作り直しの手順（`doc/` の案内・steering の 1 行）／③ `doc/` の棚卸の表と、開発者が選んだファイルの削除・`doc/archive/` への移動と、参照している側の直し／④ `doc/` の直下の案内（残ったものが今も効くか歴史か）。
- **Out of scope**:
  - roadmap.md の中身（版の段・台帳・ウェーブ）の見直し（棚卸と `/kiro-discovery` の持ち物）。本仕様は roadmap.md を読むだけで、HTML のために書き方を変えない。本仕様の節の誤った記載（上の事実）の直しと、消した `doc/` の場所の名前の直しだけは行う。
  - 網羅台帳 `doc/ukadoc-coverage/` の数の直し（`coverage-roadmap-refresh`）・リリースノート（`/release`）・Web への公開（GitHub Pages など）。
  - `doc/` の正本の文書（`COMPAT_ARCHITECTURE.md` など）の中身の書き直し。
  - 完了した spec の文書（`.kiro/specs/completed/`）と `roadmap-history.md` の書き換え（当時の記録で、中の古い場所の名前はそのまま残す）。
  - 根の `docs/`（議題 3 で確定＝触らない）。
- **Adjacent expectations**:
  - 上流: roadmap.md の「版の段」節（2026-10-15）が俯瞰の背骨。4 か所の書き方は、棚卸と `/kiro-discovery` が書くままを道具の側で吸収する。読めない形に変わったら、道具は黙らずに知らせる（要件 4）。
  - 下流: 以後の棚卸と `/kiro-discovery`。HTML をリポジトリに入れないと決まれば（議題 1 の仮置き）、両者の手順に足す段は無い。入れると決まれば、roadmap を直したら作り直す段が両者に加わる（議題 2）。
  - `shiori4-api`: brief が `doc/shiori/` を指す。`doc/shiori/` を動かすなら、その brief の場所の名前を同じ変更で直す（要件 8.1）。
  - スキル `kiro-complete`: `doc/ROADMAP.md` を書いてよいファイルとして名指す。消すなら同じ変更で直す（要件 8.1）。
  - `coverage-roadmap-refresh`: `doc/ukadoc-coverage/` は本仕様では動かさない（コードとテストが読む）。

## 要件生成で決めた点と、要件討議の議題（仮置き）

brief が要件の段へ回した議題 3 つは、下の仮置きで要件を書いた（答えで変わる条項を併記）。どれも範囲（何を In とするか）の食い違いではなく、範囲の中の値と線引きの決めである。

- **議題 1 作った HTML をリポジトリに入れるか** → 仮置き＝**入れない（作るたびにワークツリーの `target\` の下へ書き、開く）**。理由: roadmap.md はほとんどの PR（`/kiro-complete`・`/kiro-discovery`・棚卸）が触るので、入れると並走する PR のたびに HTML の差分と衝突が出る（生成物の衝突は手で直さず作り直すことになり、毎回の取り込みに作り直しが挟まる）。HTML は台帳の長い文を抱えて大きく、PR の差分を読む手間（トークン）も増える。見るたびに作れば、作り直し忘れが起きない。入れると決まったら変わる条項: 要件 5.3・5.6・6.6（今の roadmap から作った物と 1 バイト違わないことを全体テストで見る形へ）と、Boundary Context の下流。
- **議題 2 作り直し忘れの気付き方** → 仮置き＝**議題 1 の仮置きの下では、手順を足さず、HTML に「どの時点の roadmap.md から作ったか」を出す**（要件 1.8・5.6）。議題 1 で「入れる」と決まったときの仮置き＝**全体テストで「入れた HTML が今の roadmap から作った物と 1 バイト違わない」を見る**（`doc/ukadoc-coverage/report/` の前例と同じ形。「棚卸の手順に書くだけ」は忘れたときに気付けないので、Desired Outcome の 2 を満たさない）。
- **議題 3 `doc/` の整理の線引き** → **確定（2026-10-15 要件討議・推奨案で確定）＝下の表のとおり**（要件 7 の棚卸の表）。「消す」「寄せる」の行はどれもコードとテストが読まないものに限った。根の `docs/` は触らない。steering から俯瞰図の作り方へ辿る 1 行は、`focus.md` の参照先の一覧の `doc/ROADMAP.md` の行と差し替えて置く（常に読まれるファイルの行を増やさない・要件 5.5）。
- **テーマの名前の見せ方**（要件討議で追加・推奨案で確定）→ `###` 見出しの最初の「（」より前だけを出す（例「SSP MCP の移植（α 後・2026-09-29 `/kiro-discovery` で起票）」→「SSP MCP の移植」。凡例が短くなり、開発者が使わないと決めた「α」の語が画面に出ない・要件 3.1）。
- **全体テストで実物の roadmap.md を通すか**（要件討議で追加・推奨案で確定）→ 通す。赤にするのは 4 か所が見つからない・読めないときだけで、数の食い違いなどの警告は赤にしない（棚卸の PR が書き方を崩したとき、HTML を開くまで気付けないのを防ぐ・要件 6.8）。

| 項目 | ファイル数・最後に変わった日 | コード・テストが読むか | ほかの参照（歴史の記録を除く） | 中身の見立て | 扱い（確定） |
|---|---|---|---|---|---|
| `doc/ROADMAP.md` | 1・06-28 | 読まない | `focus.md`・`workflow.md`・スキル `kiro-complete`・`doc/archive/ARCHITECTURE.md` | steering の roadmap.md への 18 行のポインタ（古い） | **消す**（steering 2 か所とスキルの 3 か所を直す。`doc/archive/` の中の名指しは歴史として残す） |
| `doc/discussion-260702-m1-roadmap.md` と `.svg` 2 つ | 3・07-03 | 読まない | 本仕様の brief だけ | M1 期の議論の写し（自分で「正本が勝つ」と書く） | **`doc/archive/` へ寄せる**（3 つを一緒に） |
| `doc/archive/`（`spec_backup_20251101_082532/` を含む） | 36・10-03 | 読まない | 本仕様の brief だけ | 既に歴史の置き場（旧 ARCHITECTURE・旧 UI の設計など） | **残す**（歴史の置き場。丸ごと消しても機械的に壊れる物は無い） |
| `doc/shiori/` | 41・06-28 | 読まない（上の事実） | `shiori4-api` の brief・roadmap.md・`anchor-compat.md`・`choice-cascade-compat.md`・網羅台帳 `ledger/shiori.toml` の注 | SHIORI の紙の契約のカタログ。まだ完了していない `shiori4-api` が正本と呼ぶ | **残す**（中身が今の正本と食い違うかは `shiori4-api` の持ち物） |
| `doc/emo2-conformance-scope.md` | 1・10-10 | 読まない | 網羅台帳の briefing と `ledger/shiori.toml`・`choice-cascade-compat.md` ほか | M1 期の範囲の正本。steering からは名指されないが、台帳が今も引く | **残す** |
| `doc/CONSTITUTION.md`・`PASTA_PROFILE.md`・`COMPAT_ARCHITECTURE.md`・`anchor-compat.md`・`choice-cascade-compat.md`・`crates-io-publish.md`・`impl-watch.md` | 7・09-11〜10-15 | `impl-watch.md` はテストが場所の名前を持つ。ほかは読まない | 根の README・steering・スキル・brief | 今の正本・使っている手順書 | **残す** |
| `doc/ukadoc-coverage/`・`doc/ssp-mcp/`・`doc/release-notes/`・`doc/images/` | 19・8・2・1 | 読む（images は根の README が表示） | 多い | 今の台帳・事実の控え・手順・README の絵 | **残す** |
| 根の `docs/`（参考） | 1・09-24 | 読まない | — | `doc/` の外 | **触らない**（brief の範囲は `doc/`） |

生成時に決めた点（勝者が明白なので議題にしない・討議で覆してよい）:

- コードかテストがファイルとして読む・場所の名前を突き合わせる `doc/` のファイルは、消さず動かさない（動かすとコードを直すことになり、doc の整理の範囲を越える）。
- 完了した spec の文書と `roadmap-history.md` は書き換えない（roadmap.md が「history が全文正本・非改変」と定める）。brief の Desired Outcome の 3 の「steering は 1 つも壊れない」は、`roadmap-history.md` を除く steering と読む。
- 歴史として置くものは `doc/archive/` の下だけに置き、`doc/archive/` の外に残るものは今も効くものとする。どちらかは `doc/` の直下の 1 枚の案内（新しく作る）で書き分ける。
- HTML の表示は、版の段の帯を左から右へ並べる（brief の推す形）。完了した spec は帯に入らないので、帯とは別の「完了」の場所にテーマごとにまとめる。
- テーマは「テーマ別の決めごと」の `###` 見出しとし、どの節の `#### Specs (dependency order)` にも載らない 30 本は「テーマなし」にまとめる（roadmap.md を書き足さない）。


## Requirements

### Requirement 1: 版の段ごとの帯と spec のカード

**Objective:** 開発者として、ブラウザで 1 枚の HTML を開くだけで版の段ごとに spec が並んだ姿を見たい。そうすれば、1,000 行を超える roadmap.md を読まずに、どの版に何が入るかを一目で見渡せる。

#### Acceptance Criteria

1. The 俯瞰の HTML shall roadmap.md の「版の段」の表の行ごとに 1 本の帯を持ち、帯を表の行の順（今は v0.0.3 → v0.1.0 → v0.2.0 → v0.3.0 → v0.4.0 → いつか → windows-reactor 待ち → 版に縛らない）に並べる（版の名前と順を道具の中に書き込まず、表から読む）。
2. The 俯瞰の HTML shall 各帯に、版の名前・目標の列の文・その帯の spec の数を出す。
3. The 俯瞰の HTML shall 「版の段」の表の spec の列に名前が出る spec を、1 本につき 1 枚のカードとして、その行の帯の中に置く。
4. Where 「版の段」の表の spec の列が小見出しで分かれている（「動く＝」「配れる＝」「後続: シェル内バルーン＝」など、`／` で区切った組）, the 俯瞰の HTML shall 帯の中でその組を小見出し付きで分けて見せ、組の順を表のとおりに保つ。
5. The 俯瞰の HTML shall 帯の中（小見出しの組の中）のカードを、表の spec の列に名前が出る順に並べ、`→` で結ばれた spec どうしは前から後ろへの依存の順と分かる形で見せる。
6. The カード shall spec の名前・一言・段（優先／その他／バグ／据え置き／保留）・区分（A〜D）・規模・前提を出す。一言は「テーマ別の決めごと」の各節の `#### Specs (dependency order)` の行の説明から取り、どの節にも載らない spec は「spec 台帳」の「何をするか」の書き出しを短く切って使う。
7. When 開発者がカードを開く操作をする, the 俯瞰の HTML shall その spec の「spec 台帳」の行の全文（何をするか・並び・前提）と、表の spec の列で名前に付いた括弧書きの注を見せる。
8. The 俯瞰の HTML shall どの roadmap.md から作ったかを、roadmap.md の更新日（先頭の `updated_at`）と、作ったときのコミット（roadmap.md に未コミットの変更が在ればその旨）として、開いてすぐ見える所に出す。

### Requirement 2: 着手中・完了・依存の見分け

**Objective:** 開発者として、どの spec がいま走っているか、どれが終わったか、何を待っているかを、帯の上で見分けたい。そうすれば、次に何を取るかを roadmap.md の複数の節を突き合わせずに判断できる。

#### Acceptance Criteria

1. The 俯瞰の HTML shall 「ウェーブ編成」の表の、いま走っているウェーブの行（表の最初の行）に名前が出る spec のカードに、着手中の印とウェーブの名前（今は C6）を付ける。
2. Where 帯の中の spec が「テーマ別の決めごと」の節の `#### Specs (dependency order)` の下でチェック済み（`- [x]`）になっている（`#### Existing Spec Updates` の下の印は申し送りの印なので数えない）, the 俯瞰の HTML shall そのカードに完了の印を付け、帯からは外さない。
3. The 俯瞰の HTML shall どの帯にも載らないチェック済み（`#### Specs (dependency order)` の下の `- [x]`）の spec を、完了の印を付けたカードとして、帯とは別の「完了」の場所にテーマごとにまとめて見せる。
4. The カード shall 前提（「spec 台帳」の「前提」の列と、テーマの節の `Dependencies:`）に挙がった spec の名前を出し、その spec がどの帯に居るか（または完了か）を名前の横に出す。

### Requirement 3: テーマの色分けと絞り込み

**Objective:** 開発者として、テーマ（シェル内バルーン・MCP・タスクトレイなど）ごとに色で見分け、1 つのテーマだけに絞って見たい。そうすれば、版をまたいで散らばった 1 つのテーマの進み具合を追える。

#### Acceptance Criteria

1. The 俯瞰の HTML shall 各 spec のテーマを「テーマ別の決めごと」の `###` 見出しのうち、その spec が `#### Specs (dependency order)` に載る節の見出しとし、どの節にも載らない spec のテーマを「テーマなし」とする。テーマの名前として見せるのは、見出しの最初の「（」より前だけとする（括弧書きの起票の注は出さない）。
2. The 俯瞰の HTML shall カードをテーマごとに見分けられる色で塗り、色とテーマの名前の対応を凡例として出す。
3. When 開発者が凡例または絞り込みの操作で 1 つ以上のテーマを選ぶ, the 俯瞰の HTML shall 選んだテーマの spec のカードだけを見せ、各帯の数を絞った後の数に替える。
4. When 開発者が絞り込みを解く, the 俯瞰の HTML shall すべてのカードと元の数を見せる。
5. The 俯瞰の HTML shall 色だけに頼らず、カードの上にテーマの名前を文字でも出す。

### Requirement 4: roadmap.md の読み取りと食い違いの知らせ

**Objective:** 開発者として、棚卸と `/kiro-discovery` が書くままの roadmap.md から HTML を作り、roadmap.md の中の食い違いがあれば知らされたい。そうすれば、HTML のために roadmap.md の書き方を変えずに済み、手書きの数のずれにも気付ける。

#### Acceptance Criteria

1. The 道具 shall roadmap.md を読むだけで書き換えず、HTML のために roadmap.md の書き方を変えることを求めない（「版の段」の表・「spec 台帳」の表・「テーマ別の決めごと」の節・「ウェーブ編成」の表の今の書き方を、そのまま読む）。
2. If roadmap.md が読めない、または 4 か所（「版の段」の表・「spec 台帳」の表・「テーマ別の決めごと」・「ウェーブ編成」の表）のどれかが見つからない, then the 道具 shall HTML を書かず（前に書いた HTML も消さず）、見つからない場所の名前を端末へ出して 0 でない終了コードで終わる。
3. If 「版の段」の表の行に書かれた spec の数（「＝**N 本**」）と、その行から読んだ spec の名前の数が違う, then the 道具 shall 端末へ警告を出し、HTML のその帯に両方の数を出したうえで、HTML は書き終える。
4. If 「版の段」の表に名前が出る spec が「spec 台帳」に無い、または「spec 台帳」の spec がどの帯にも載らない, then the 道具 shall 端末へ警告を出し、HTML でその spec に「台帳に無い」または「帯に無い」の印を付けて見せる（黙って落とさない）。
5. If 同じ spec の名前が 2 つ以上の帯に出る, then the 道具 shall 端末へ警告を出し、HTML では表で先に出る帯に置いて、もう一方の帯の名前をカードに出す。
6. When 道具が HTML を書き終える, the 道具 shall 書いたファイルの場所・帯の数・spec の数・警告の数を端末へ出し、0 の終了コードで終わる（警告は終了コードを変えない）。
7. If HTML のファイルを書けない, then the 道具 shall 書けなかった場所を端末へ出して 0 でない終了コードで終わる。

### Requirement 5: 作り直しと置き場

**Objective:** 開発者として、roadmap.md が動いた後でも、手で直さずに 1 つのコマンドで今の roadmap.md の HTML を作って開きたい。そうすれば、古い俯瞰図を見て判断を誤らずに済む。

#### Acceptance Criteria

1. The 道具 shall 1 つのコマンドで、今のリポジトリの roadmap.md から HTML を作る。
2. Where 開発者が開く指定を付ける, the 道具 shall 作った HTML を OS の既定のブラウザで開く。
3. The 道具 shall 作った HTML をリポジトリの追跡の外（ワークツリーの `target\` の下）に書き、リポジトリへは入れない（議題 1 の仮置き）。
4. The 道具 shall 同じ roadmap.md と同じコミットからは、1 バイトも違わない HTML を作る（作った時刻のような、入力によらない値を HTML に入れない）。
5. The 作り直しの手順 shall `doc/` の案内（要件 9）に、どのディレクトリから打っても動く絶対パスのコマンドとして書かれ、`focus.md` の参照先の一覧の 1 行（消す `doc/ROADMAP.md` の行と差し替える）から辿れる。
6. While HTML がリポジトリに入っていない（議題 1 の仮置き）, the 作り直しの手順 shall 棚卸と `/kiro-discovery` の手順に「roadmap を直したら HTML を作り直す」段を足さない（見るたびに作るので、作り直し忘れが起きない。どの時点の roadmap.md から作ったかは要件 1.8 で見える＝議題 2 の仮置き）。

### Requirement 6: 道具と HTML の性質

**Objective:** 開発者として、新しい依存を足さず、どの端末でも同じに動き、全体テストで自分の働きを確かめられる道具にしたい。そうすれば、道具が壊れたときに気付け、保守の手間が増えない。

#### Acceptance Criteria

1. The 道具 shall リポジトリが既に使っている PowerShell だけで動き、新しいモジュール・パッケージ・外の実行ファイルを足さない。
2. The 俯瞰の HTML shall 1 つのファイルで完結し、ネットにつながっていないブラウザでファイルを直に開いても、表示・カードを開く操作・絞り込みがすべて働く（外の CDN・フォント・画像を読まない）。
3. The 道具 shall 端末へ出す文を ASCII だけにし、端末の文字コードを読みも書きもしない（HTML の中身の日本語は、文字コードを明示したファイルとして書く）。
4. The 道具 shall 呼ばれた今のフォルダによらず、道具の在るリポジトリの roadmap.md を読み、今のフォルダを変えない。
5. The 道具 shall ネットを使わない自分の検査を持ち、見本の roadmap の断片から読み取った帯・spec・テーマ・印と、警告の出方（要件 4.2〜4.5）を、期待の値と突き合わせて判定する。
6. When 全体テスト（`tools/test-all.ps1`）を 1 回回す, the 全体テスト shall 道具の自分の検査を含めて走らせ、落ちた判定が在れば赤にする。
7. The 道具 shall HTML の中の決まった文言（見出し・凡例・印の名前・警告の文）を、道具の本体と分けて 1 か所で直せる形に持つ。
8. When 全体テストを 1 回回す, the 全体テスト shall 道具の見本だけでなく、リポジトリの今の roadmap.md も道具に通し、4 か所（要件 4.2）が見つからないか読めないときだけ赤にする（数の食い違い・台帳との食い違い・重なりの警告は赤にしない）。

### Requirement 7: doc の棚卸の表

**Objective:** 開発者として、`doc/` の各ファイル・フォルダについて「参照されているか」「中身が今の正本と食い違うか」を 1 枚の表で見て、消す・`doc/archive/` へ寄せる・残すを選びたい。そうすれば、どれを消しても何が壊れるかを事前に知ったうえで整理できる。

#### Acceptance Criteria

1. The 棚卸の表 shall `doc/` の直下の各ファイルと各フォルダを 1 行ずつ持ち、フォルダの中にコードかテストが読む（または場所の名前を突き合わせる）ファイルが在れば、そのファイルを別の行に分ける。
2. The 棚卸の表 shall 各行に、ファイルの数・最後に変わった日（git の記録）・参照している所（コードとテスト・steering・スキル・まだ完了していない spec の文書・ほかの `doc/` のファイル・リポジトリの根の文書に分けて）・中身の見立て（今の正本／使っている手順書／作り直す生成物／歴史・古い）・開発者が選んだ扱い（消す／`doc/archive/` へ寄せる／残す）を持つ。
3. The 棚卸の表 shall 完了した spec の文書（`.kiro/specs/completed/`）と `roadmap-history.md` からの参照を、ほかの参照と分けて「歴史の記録からの参照」として数える。
4. The 棚卸の表 shall 本仕様の文書の中に残り、各行の扱いが要件討議で開発者が選んだものと一致する。

### Requirement 8: doc の整理と参照の直し

**Objective:** 開発者として、選んだファイルを消す・寄せるときに、参照していた所が 1 つも壊れないようにしたい。そうすれば、整理の後に steering・スキル・コード・台帳のどれかが古い場所を指したまま残らない。

#### Acceptance Criteria

1. When 棚卸の表で「消す」または「`doc/archive/` へ寄せる」と選ばれたファイルを動かす, the 本仕様 shall そのファイルを参照しているコード・テスト・steering（`roadmap-history.md` を除く）・スキル・まだ完了していない spec の文書・残る `doc/` のファイル・リポジトリの根の文書を、同じ変更の中で新しい場所を指すか参照を外す形に直す。
2. The 本仕様 shall コードかテストがファイルとして読む、または場所の名前を突き合わせる `doc/` のファイル（`doc/ukadoc-coverage/` の全体・`doc/ssp-mcp/tools-list-ssp-2.9.05.json`・`doc/ssp-mcp/log-convention.md`・`doc/release-notes/`・`doc/impl-watch.md`）を消さず、場所も変えない。
3. The 本仕様 shall 完了した spec の文書（`.kiro/specs/completed/`）と `roadmap-history.md` を書き換えない（どちらも当時の記録で、中の古い場所の名前はそのまま残す）。
4. When 整理を終える, the 本仕様 shall 動かした各ファイルの元の場所の名前でリポジトリ全体（完了した spec の文書と `roadmap-history.md` を除く）を探し、残った参照が 0 件であることを記録に残す。
5. When 整理を終える, the 本仕様 shall 全体テストを回し、コードとテストが `doc/` を読む所が壊れていないことを確かめる。
6. The 本仕様 shall `doc/` の正本の文書（`COMPAT_ARCHITECTURE.md` など、棚卸の表で「残す」とした今の正本）の中身を書き直さない（参照の場所の直しだけは要件 8.1 に従って行う）。
7. The 本仕様 shall 本仕様の brief と roadmap.md の「ロードマップの俯瞰図と doc の整理」節にある「`doc/shiori/equivalence_evidence.toml` をコードと網羅台帳が読む」という誤った記載を、引き直した事実に合わせて直す。

### Requirement 9: doc の案内

**Objective:** 開発者として、整理の後の `doc/` に残った各ファイル・フォルダが「今も効いている」のか「歴史として置く」のかを、1 か所で知りたい。そうすれば、古い記載を正本と取り違えない。

#### Acceptance Criteria

1. The `doc/` shall 直下に 1 枚の案内を持ち、残った各ファイル・各フォルダの役目を 1 行で書き、今も効いているか歴史として置くかを書き分ける。
2. The `doc/` shall 歴史として置くものを `doc/archive/` の下だけに置き、`doc/archive/` の外に残ったものは、どれも今も効いているものとする。
3. The `doc/` の案内 shall 俯瞰の HTML の作り方と開き方（要件 5.5）と、`doc/` へ新しいファイルを足すときは案内に 1 行足すという決まりを持つ（決まりは書くだけで、機械の検査は作らない）。
