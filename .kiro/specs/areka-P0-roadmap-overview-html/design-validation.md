# 設計レビュー: areka-P0-roadmap-overview-html

> 2026-10-15・worktree の HEAD `43c4c580`（spec の文書だけを積んだ枝）で `design.md` を読み、設計が名前と行番号で指した物を今の木に対して 1 つずつ確かめた。非対話で書いた（開発者への質問は無し）。`design.md`・`requirements.md`・`research.md`・`spec.json` は変えていない。

## 要約

設計は前例 `tools/release-notes.ps1` の型（較正値・`Read-Lf`・BOM なしの書き出し・`-SelfTest` の `ok`／`FAIL` の 1 行ずつ・文言の外出し）をそのままなぞり、「読み手（純粋な関数）が中身をすべて決め、ひな形の JS は描くだけ」という分け方で、要件 1〜9 のすべてに部品と検査の道筋が付いている。roadmap.md の 5 か所の節の見出し・最初の表・ウェーブ C6 の 8 本・完了サマリの名前の拾い方は今の roadmap.md で成り立つことを確かめ、`doc/` の整理で直す参照の一覧にも漏れは無かった。残る指摘は、要件 1.8 の「どのコミットから作ったか」が squash マージの後に辿れなくなる点と、見本の突き合わせが改行の変換で崩れうる書き方の 2 つで、どちらも骨格を変えずに直せる。

## 引用の実在の確認（設計が名前で指したもの）

| 設計の引用 | 実在 | 備考 |
|---|---|---|
| `tools/release-notes.ps1` の `$root = Split-Path $PSScriptRoot -Parent`・`Read-Lf`（CRLF を LF に）・`WriteAllText`（`UTF8Encoding($false)`）・`-SelfTest` の `ok selftest`／`FAIL selftest`・パラメーターセット | ○ | 127・133・216・219〜222・97 行 |
| `tools/test-all.ps1` の段 `release notes check` と、その後の `-License` の段・頭の説明の番号（8 が release notes・9 が `-License`） | ○ | 52〜55 行・14 行。「9 を足し `-License` を 10 へ」は正しい |
| `tools/encoding-check.ps1` の規則 A は `tools/*.ps1`（`tools/perf/` を除く）だけが対象 | ○ | `.html`・`.json` は対象外＝ひな形の ASCII だけは `-SelfTest` の 2 で見る、という設計の分担は正しい |
| `tools/utf8-child.ps1` の `Invoke-Utf8Child` | ○ | `release-notes.ps1` 291 行が `git -C $root` で使う前例 |
| `ConvertTo-Json -Depth 10 -Compress -EscapeHandling EscapeHtml` | ○ | 手元の pwsh で、データの形（root→data→bands→groups→cards→prereqs→要素）は深さ 9 で収まり 10 で切れないこと、`</script>` が `</script>` になることを確かめた（深さ 8 では `prereqs` の要素が文字列に潰れる＝余裕は 1） |
| roadmap.md の節の見出し `## 版の段`（26 行）・`## 完了サマリ`（79）・`## ウェーブ編成`（134）・`## spec 台帳`（195）・`## テーマ別の決めごと`（400）と前置きの `updated_at: 2026-10-15` | ○ | どれも `## {目印}` で始まる。CRLF・1,095 行・376 KB |
| 各節の最初の表＝読む表 | ○ | 版の段は `| 版 | 目標 | spec…`、完了サマリは `| Wave | 完了日 | ユニット（PR） | 一行要約 |`（後ろに別の表）、ウェーブ編成は `| Wave | ユニット…`。最初の行が **C6** |
| ウェーブ C6 の行の 2 欄目のバッククォートの語 | ○ | ちょうど 8 本（`range-choice-tag`〜`dot-sakura-specimen`）で、ほかの名前は混ざらない |
| 完了サマリのユニットの欄の名前（深さを問わず拾う） | ○ | spec 名に見える語は 113 個。台帳の名前と 1 つも重ならない＝未完了の spec を誤って完了にしない |
| `#### Specs (dependency order)` の行の形 | ○ | 37 節・129 行（`- [ ]` 84・`- [x]` 34・`- [→]` 11）。`, ` で並ぶ行は今は無い。`Dependencies:` の無い行（`package-check-temp-cleanup` ほか 1 行）は「残り全部を一言の元」で読める。`- [x]` の残り 5 行は `#### Existing Spec Updates` の下で、設計どおり数えない |
| `- [→]`（吸収済み）の 11 本の名前 | ○ | 台帳の前提の欄にも版の段の表にも出ない＝「roadmap に無い」の誤った印は出ない |
| 直す参照（`kiro-complete` の `SKILL.md` 324・339・350・412 行／`focus.md` 63 行／`workflow.md` 89 行／`tech.md` 94 行の `release notes check` の項／`structure.md` 18 行の `tools/` の一覧／`doc/archive/ARCHITECTURE.md` 171 行） | ○ | 行番号と中身は設計の記述どおり |
| 0 件の確かめのコマンド（要件 8.4） | ○ | 今の木で回すと、ヒットは設計の Modified Files の 6 か所と、動かす当のファイル（`doc/ROADMAP.md` と `doc/discussion-260702-m1-roadmap.md` の中の自分の名指し）だけ。ほかの未完了の spec の文書・根の文書からの参照は無い＝直す一覧に漏れは無い |
| `.gitattributes` | 無し | `core.autocrlf=true`。前例の `tools/release-notes-fixture/*` と `release-notes.strings.json` は `i/lf w/crlf`（作業の木では CRLF）＝指摘 2 の根拠 |

## 重要な指摘（2 件）

### 🔴 指摘 1: 入れる 1 枚の「作ったときのコミット」が、squash マージの後に main から辿れない

- **懸念**: Source の `commit` は `git -C $root rev-parse HEAD` の先頭 12 字で、Snapshot は「roadmap.md の直しをコミットした後に道具を回す」＝ワークツリーの枝のコミットを指す。areka の main への道は squash マージだけ（`workflow.md`）なので、枝が消えるとこのコミットは main の歴史に無く、HTML の頭に出る値から roadmap.md のその版を開けない。開発者が見たい日に作るスナップショットも、ふつうはワークツリーの枝の上で作るので同じことが起きる。
- **影響**: 要件 1.8 の「どの roadmap.md から作ったか」は、ファイル名の日付（`updated_at`）だけが頼りになる。`updated_at` は同じ日に何度も roadmap.md が動いても変わらないので、同じ日付の版を見分けられない。
- **提案**: Source に roadmap.md の中身の名前（`git -C $root hash-object -- .kiro/steering/roadmap.md` の先頭 12 字＝ブロブの名前）を足し、頭に出す。中身が同じなら squash の後も main に同じブロブが在り、`git log --find-object={名前}` で辿れる。読むのは roadmap.md だけなので要件 4.1 にも、入力からだけ決まるので要件 5.4 にも反しない。足さないなら、`doc/README.md` に「コミットは枝の上の値で、マージの後は辿れないことがある」と書く。
- **トレース**: 要件 1.8・5.3・5.4
- **根拠**: design.md「Cli（本体の入出力）」の **Source** の項・「全体テストと文書」の **Snapshot** の項

### 🔴 指摘 2: 見本の「1 字違わず」の突き合わせが、改行の変換で新しい写しだけ赤になりうる

- **懸念**: `.gitattributes` が無く `core.autocrlf=true` なので、入れた `expected-data.json`・ひな形・文言のファイルは、新しく取り出した写し（別のワークツリー・main の手元）では CRLF になる。設計は見本の `roadmap.md` についてだけ「LF でも CRLF でもよい」と書き、`-SelfTest` の 3 は「LF の JSON が `expected-data.json` と 1 字違わず同じ」と書く。期待のファイルを生のまま読む実装にすると、作った枝では LF のまま緑で、マージの後に取り出した写しで全体テストの段 `roadmap overview check` が赤になる。ひな形を生のまま読むと、HTML の改行も写しごとに変わり「LF で書く」が崩れる。
- **影響**: 全体テスト（`/kiro-complete` の門）が、道具の中身でなく写しの取り出し方で赤になる。作った枝の上では気付けない。
- **提案**: 道具が読むファイル（文言・ひな形・見本の `roadmap.md`・`expected-data.json`）はすべて `Read-Lf` を通す、と Cli と SelfTest の項に 1 行書く（前例の `release-notes.ps1` は期待のファイルも `Read-Lf` で読み、`w/crlf` の写しで緑になっている）。
- **トレース**: 要件 5.4・6.5・6.6
- **根拠**: design.md「SelfTest」の 3・「File Structure Plan」の `roadmap-overview-fixture/` の注・「Assembler」の「改行は LF」

## 設計の良いところ

- **判断をすべて PowerShell の純粋な関数に寄せた分け方**: 区切り・完了の判定・前提の居場所・一言の切り方・警告をすべて Reader に置き、ひな形の JS は `textContent` で描くだけにしたので、`-SelfTest` が見本だけで読み方の全分岐・警告 5 種・欠け 7 種・HTML の決定論と `</script>` の逃がしまで判定できる。壊した見本が見本と同じなら空振りとして落とす、という較正まで入っている。
- **今の roadmap.md の難所を先回りして規則に落としている**: 節ごとの最初の表だけを読む（同じ見出し行の表が別の節に在る）・「＝**N 本**」の**最後**の一致・括弧の深さ 0 の区切り・前提の括弧の中の名前を数えない・`Existing Spec Updates` の `- [x]` を数えない、といった規則は、今の roadmap.md で実際に起きている形（上の表で確かめた）に合っている。`doc/` の整理も、0 件の確かめのコマンドを今の木で回すと設計の直す一覧と過不足なく一致する。

## 判定

**GO**

- **理由**: 骨格（読み手・組み立て・入出力・ひな形の分け方）と要件の対応に食い違いは無く、引用した前例・行番号・roadmap.md の形はすべて実在した。2 つの指摘はどちらも 1〜2 行の書き足しで直り、部品や流れを変えない。
- **次の段**: 設計討議（`/kiro-design-discussion areka-P0-roadmap-overview-html`）で指摘 1・2 を決め、`design.md` に反映してから `/kiro-spec-tasks areka-P0-roadmap-overview-html` へ進む。
- **付記（指摘にしない小さな点）**: Template の項の「矢印などの記号は JS や CSS の逃がし `→`・`\2192` で書く」の 1 つ目は、ASCII の逃がし `→` のつもりと読める（そのまま書くと `-SelfTest` の 2 が落とすので、実装で取り違えても気付ける）。要件の事実の節の「`#### Specs (dependency order)` の下の `- [x]` は 39 行」は、今の数えでは Specs の下が 34 行・`Existing Spec Updates` の下が 5 行（合わせて 39）で、設計の規則（Specs の下だけを完了と数える）に影響は無い。
