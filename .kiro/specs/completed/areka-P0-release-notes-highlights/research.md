# Research & Design Decisions

## Summary
- **Feature**: `areka-P0-release-notes-highlights`
- **Discovery Scope**: Extension（既存のリリース CI とリリースの手順への追加・軽い調査）
- **Key Findings**:
  - `tools/*.ps1` と workflow の `run:` の本文は、コメントの外に ASCII の外の字を 1 字も置けない（全体テストの段 `encoding check` の規則 A）。日本語の見出しや定型文は、スクリプトの外のファイルに置いて読む必要がある。
  - リリース CI の取り出しは浅い（`fetch-depth` の指定が無い）。`git log` で一覧を作るには、履歴の全部とタグを取る指定が要る。
  - `v0.0.1` のタグが在る。`v0.0.1` から `v0.0.2` までの本線のコミットは 614 で、PR の番号が題名の末尾に付くのは 194 だけである（古いコミットは PR を通していない）。`v0.0.2` より後の 37 は、すべて PR の番号が付く。

## Research Log

### 一覧のもと（`git log` か、GitHub の API か）
- **Context**: 詳しいリリースノートの行を何から作るか（brief の議題 9）。
- **Sources Consulted**: `git log --first-parent`（2026-10-11・main `392b9bce`）、`.github/workflows/release.yml` の段「Release を公開」。
- **Findings**:
  - `v0.0.2..HEAD` の本線は 37 コミットで、全部が「種類(スコープ): 題名 (#番号)」の形。
  - `v0.0.1..v0.0.2` の本線は 614 コミット。末尾が `(#番号)` は 194、`(PR#番号)` が 2、番号なしが 418。本線の上のマージコミットが 8。
  - スコープは `areka-P0-*` が 148、ほかは `spec`・`specs`・`discovery`・`roadmap`・`wintf`・`dola` など。
  - GitHub の API（`releases/generate-notes`）は PR の一覧を返す。通信が要り、返る文の形に頼ることになり、`gh` の出力を UTF-8 で読む手当ても要る。
- **Implications**: `git log --first-parent --format=%s` を使う。通信なしで、同じ入力から同じ出力になり、決まった題名の並びで検査できる。`v0.0.3` からは API の一覧と同じ中身になる。`v0.0.2` の 1 回だけ、PR を通していない古いコミットも並ぶ（要件 4.3 を「番号の無い変更は番号なしで載せる」に直した）。

### 文字コードの検査（`tools/encoding-check.ps1`）
- **Context**: 新しいスクリプトが日本語の題名を読み、日本語の見出しを書く。
- **Sources Consulted**: `tools/encoding-check.ps1` の規則 A〜E と較正の見本、`tools/utf8-child.ps1` の `Invoke-Utf8Child`。
- **Findings**:
  - 規則 A: コメントの外のトークンに ASCII の外の字が在れば赤。ファイルへ書くだけの文字列リテラルも赤。対象は `tools/*.ps1`（下のフォルダは見ない）と workflow の `run:` の本文。
  - 規則 C: 端末の文字コードの書き替えは赤。
  - `Invoke-Utf8Child git @(...)` は、子の標準出力を BOM なしの UTF-8 として読み、`Out` に 1 本の文字列で返す。
- **Implications**: 日本語の定型文は `tools/release-notes.strings.json` に置く。検査の見本（日本語の題名の並びと期待する出力）は `tools/release-notes-fixture/` のファイルに置く。`git log` は `Invoke-Utf8Child` で読む。

### 全体テストへの入れ方（`tools/test-all.ps1`）
- **Findings**: 段は `Step '名前' { コマンド }` の 1 行で足す。判定は終了コードだけ。段の一覧を写している所は、`tools/test-all.ps1` の冒頭のコメント・`tech.md` の全体テストの段の説明・`structure.md` の `tools/` の行・`kiro-complete` の段の一覧である。
- **Implications**: `Step 'release notes check' { pwsh -NoProfile -File tools/release-notes.ps1 -SelfTest }` を `encoding check` の後ろに足し、写しの 4 か所を合わせる。

### リリースの手順の数え上げ（brief の議題 8）
- **Findings**: `areka-P0-release-cycle` の文書で、版上げの数え上げと「4 つ」に従っている所は、要件に 6 か所、設計に約 25 か所、タスクに 10 か所在る（設計の「リリースの手順の文書の直し」の表にまとめた）。`git status --porcelain` の `??` の行を 0 行とする判定が在り、新しいファイルである見どころのファイルは、ここに 1 行で現れる。
- **Implications**: 直しは数の置き換えだけでは足りず、「新しいファイルが 1 つ在ってよい」を判定に足す。

### 改行
- **Findings**: リリース CI は取り出しの前に `core.autocrlf true` を置くので、取り出した見どころのファイルと検査の見本は CRLF になる。
- **Implications**: 道具は読んだ文の改行を LF にそろえてから扱い、書くときは LF・BOM なしの UTF-8 で書く。

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| 道具 1 本を `tools/` に置く | 検査・一覧・本文の組み立てを 1 本のスクリプトに入れ、手元と CI が同じ物を呼ぶ | 手元で同じ結果を出せる。全体テストで檻に入る | `release-ci-workflow` の「段の本文は workflow の中」から外れる | 採った |
| workflow の中に書く | 段の本文に整形の処理を書く | ファイルが増えない | 手元で回せない。検査を全体テストに入れられない | 要件 2.5・4.11・8.3 を満たせない |
| GitHub の API の一覧を整形する | `releases/generate-notes` の返す文を作り替える | 今の一覧と同じ件数 | 通信が要る。返る文の形に頼る | 採らない |

## Design Decisions

### Decision: 一覧は `git log --first-parent` の題名から作る
- **Alternatives Considered**: 上の表のとおり。
- **Selected Approach**: `{一つ前のタグ}..{先}` の本線の題名を読み、1 コミット 1 行にする。
- **Trade-offs**: `v0.0.2` の詳しいリリースノートは 614 行になり、GitHub の一覧（263 行）より多い。添付なので、本文の読みやすさには効かない。
- **Follow-up**: リリース CI の取り出しに `fetch-depth: 0` を足す。乾いた走りで、履歴とタグが取れていることを見る。

### Decision: 日本語の定型文はスクリプトの外の JSON に置く
- **Context**: 規則 A。
- **Alternatives Considered**: `` `u{...} `` の逃がしで書く（読めない）／規則 A に例外を足す（検査を弱める）。
- **Selected Approach**: `tools/release-notes.strings.json` を UTF-8 で読む。
- **Trade-offs**: ファイルが 1 つ増える。文言の直しがスクリプトの直しと分かれる。

### Decision: 内部の作業は「スコープが spec 名でない」で見分ける
- **Alternatives Considered**: 種類（`docs`・`chore`）で見分ける（`feat(skills)` などが新しい機能に混ざる）／内部のスコープの一覧を持つ（一覧が古びる）。
- **Selected Approach**: スコープが `areka-P0-` で始まる変更だけを製品の変更とし、種類で 4 つに分ける。ほかは末尾の見出しにまとめる。
- **Trade-offs**: 開発の道具の spec は製品の側に並ぶ。古いコミット（スコープが `wintf` など）は末尾に並ぶ。末尾の見出しの名前を「開発の内部の作業など」にして、言い過ぎないようにする。

### Decision: `v0.0.2` の頁は、承認の後・マージの前に直す
- **Context**: brief は「先に main へ入れてから差し替える」としていた。
- **Selected Approach**: 開発者の明示の承認の後に差し替え、最後のタスクで「頁の本文＝枝の見どころから作った本文」を確かめる。
- **Rationale**: 完了の関門はすべてのタスクの完了を求めるので、マージの後にしか行えない作業をタスクに置けない。差し替えは何度行っても同じ結果になる。
- **Follow-up**: 2026-10-11 に開発者が「全て推奨で」と答え、この進め方で決まった。

### Decision: 乾いた走りの一覧の先は「その版のタグが在ればタグ、無ければ先頭」
- **Context**: 乾いた走りは main の先頭で回る。版が公開済みのときに先頭までを並べると、その版に入っていない変更が混ざる。
- **Selected Approach**: `refs/tags/v{版}` が在ればそれを先にし、無ければ `HEAD` を先にする。タグの走りではタグ＝先頭なので、同じ決まりで足りる。

### Synthesis（一般化・作るか借りるか・削る）
- **一般化**: 見どころの検査・一覧・本文の組み立ては、手元・リリース CI・`v0.0.2` の直しの 3 か所で同じ物が要る。1 本の道具の 3 つの形にまとめた。
- **借りる**: 子の出力の読み方は `Invoke-Utf8Child`、全体テストへの入れ方は `Step`、Release の作り方は `gh release create --notes-file` を使う。変更履歴の専用の道具は足さない。
- **削る**: 一つ前のタグを道具の中で決め直さない（呼ぶ側が渡す）。見どころの版の行をファイルの中に持たせない（ファイルの名前が版）。添付を zip に入れない。

## Risks & Mitigations
- 新しい公開の段（5 つの添付・`--notes-file`）は、`v0.0.3` まで本番で動かない — 枝で乾いた走りを 1 回行い、公開の段の手前までを見る。公開の段はリリースの手順の確かめ（5 つ・本文）で見る。
- `v0.0.2` の頁の直しは外へ出る操作 — 直す前の本文と添付の一覧を保存し、開発者の明示の承認を得てから行う。
- 取り出しを深くすると、リリース CI の取り出しの時間が延びる — 乾いた走りで時間を見る（今の履歴は約 760 コミット）。

## References
- `.github/workflows/release.yml` — 段「版の検査」「既存の Release の検査」「4 つの確かめ」「Release を公開」
- `tools/encoding-check.ps1`・`tools/utf8-child.ps1`・`tools/test-all.ps1`
- `.kiro/specs/areka-P0-release-cycle/`（要件 3.3・3.5・4.2・6.1 と、それに従う設計・タスク）
- `vendors/sample_ghost/README.md` の「`claudia.nar` の出どころとライセンス」
