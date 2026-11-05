# ギャップ分析: doc-tree-tidy

- 測った日: 2026-11-05（枝 `claude/kiro-start-doc-tree-tidy-4790bf`・`8851b345`＝main `88eb0a43` の上に spec の初期化 1 コミット）
- 測り方: `git grep`（追跡しているファイルだけ・大文字と小文字を区別）と `find`。数はこの日の実測。
- 入力: `requirements.md`（確定）・`brief.md`・steering（`product.md`・`tech.md`・`structure.md`・`roadmap.md` のウェーブ C17 の行）

## 1. まとめ

- **動かす 4 つを実際に開くコードは 0**。名指すのはコメントでもなく、steering・根の `README.md`・`doc/` の中の文書だけ。動かしても試験の振る舞いは変わらない（要件 5.5 は確かめるだけで済む）。
- **直す所（歴史の外）は 9 ファイル・約 15 行**。クレート・道具・スキル・`.github/`・`CLAUDE.md`・未完了の spec（本 spec を除く）には 1 件も無い。同じウェーブの見直し 7 本の受け持つファイルにも 0 件で、ぶつからない。
- **名前の決まりの破れは要件が挙げた 4 つ＋例外の 2 行（`COMPAT_ARCHITECTURE.md`・`shiori/fragments/` の 16 ファイル）だけ**。ほかに破れは無い。
- **照合テストの置き場は、今ある「ワークスペース全体の見張りの置き場」`crates/log-capture-kit/tests/` が最有力**。新しいクレートは `members = ["crates/*"]` のため `Cargo.lock` が変わり、要件 6.3 に反するので取れない。
- 気をつける所: Windows の大文字と小文字だけの改名（`core.ignorecase=true`）、検索語の選び方（`click_through` は Rust の識別子にも出る）、例外の表のフォルダの行の「もう決まりに合っている」の意味。

## 2. 今の姿

### 2.1 `doc/` と `docs/`

- `docs/` は `click_through.md` の 1 つだけ。中のパスは全部リポジトリの根からの書き方（`crates/wintf/src/win_style.rs` など）で、マークダウンの相対リンクは無い。`doc/` へ移しても切れるリンクは無い。
- `doc/` の直下は 13 ファイル・7 フォルダ。`doc/` と `docs/` を合わせて 129 ファイル（全部追跡されている・追跡外のファイルは無い）。
- `doc/CONSTITUTION.md` の中の相対リンク（`COMPAT_ARCHITECTURE.md`・`../.kiro/...`）は、同じフォルダの中の改名なので切れない。
- `doc/archive/ARCHITECTURE.md:165` の `(CONSTITUTION.md)` は今すでに切れている（`doc/archive/` から見た相対）。歴史なので触らない（要件 5.3）。

### 2.2 案内 `doc/README.md` の形

- 見出し `## 直下のファイルとフォルダ` の下に 3 列の表（`名前`・`役目`・`今か歴史か`）。1 列目はどれもバッククォートで囲んだ名前 1 つで、フォルダは末尾に `/`。**このままで機械に読める**（見出しで表を見分け、1 列目のバッククォートの中を取る）。
- 型の行は `` `roadmap-overview-{YYYY-MM-DD}.html` `` の 1 行。`{YYYY-MM-DD}` は文字どおりの置き字で、ほかに置き字の行は無い。当たる実物は今 `roadmap-overview-2026-10-16.html` の 1 つ。
- 「今か歴史か」の列の値は 3 種（`今も効く`・`歴史`・`その日の姿のスナップショット（上の例外）`）。
- 表に載らない直下の実物は 0、表にあって無いものも 0（型の行を除く）。今の表は実物と合っている。
- 決まりの節の 3 つ目に「決まりは書くだけで、機械の検査は無い」がある（要件 3.2 で書き替える行）。
- 名前の決まり・例外の表・古いパスと新しいパスの対応の表は、まだ無い。足すと表が 3 つ増えるので、照合テストは**見出しで表を見分ける**必要がある（対応の表の 1 列目にもバッククォートのパスが並ぶため、見出しを見ずに読むと直下の行と取り違える）。

### 2.3 名前の決まりの破れ（要件 2.1 の決まりで全部を測った）

`doc/archive/` の下を除き、各フォルダの `README.md` を除いて、`[a-z0-9.-]` の外の字を含む名前:

| 名前 | 扱い |
|---|---|
| `doc/COMPAT_ARCHITECTURE.md` | 例外の表（要件 2.6） |
| `doc/CONSTITUTION.md` | 改名（要件 2.3） |
| `doc/PASTA_PROFILE.md` | 改名（要件 2.3） |
| `doc/shiori/equivalence_evidence.toml` | 改名（要件 2.3） |
| `doc/shiori/fragments/` の下の 16 ファイル（`_manifest.toml`・`_shared.toml`・`events/04.network_update.toml` など） | 例外の表のフォルダの行（要件 2.6） |
| `docs/click_through.md` | 移して改名（要件 1.1） |

**要件の挙げたもののほかに破れは無い**（フォルダ名もすべて合う。`tools-list-ssp-2.9.05.json`・`v0.1.0-gate-real-run.md`・`emo2-boot.png` も合う）。

補足:
- `doc/shiori/fragments/` の中を名前で読むのは、今のリポジトリでは目録 `_manifest.toml` 自身だけ（コードからの読みは 0）。目録は名前を `"path" = "_shared.toml"` のように持ち、結合の順の正本である。行方を「改名しない」とする理由（要件 2.6）は実物と合う。
- `doc/shiori/fragments/` というフォルダの名前そのものは決まりに合う（`fragments`）。

### 2.4 4 つを名指す所の全部（大文字と小文字を区別・追跡しているファイル）

**歴史の外（要件 5.2 で直す候補）**

| ファイル | 行 | 名指し |
|---|---|---|
| `README.md`（根） | 22 | `[doc/PASTA_PROFILE.md](doc/PASTA_PROFILE.md)` |
| `README.md`（根） | 126 | `[doc/CONSTITUTION.md](doc/CONSTITUTION.md)` |
| `.kiro/steering/product.md` | 84 | `` `doc/PASTA_PROFILE.md` `` |
| `.kiro/steering/structure.md` | 18 | `` `docs/`＝単発の技術メモ `` （最上位の一覧。`docs/` を無くすので消す・要件 3.6 の名指しもここか近くに足す） |
| `.kiro/steering/roadmap.md` | 1721・1734 | `` `doc/PASTA_PROFILE.md` ``（「利用統計とノベルゲーム基盤」の節の「その場で直した」「申し送り」） |
| `doc/README.md` | 16・18 | 直下の表の `CONSTITUTION.md`・`PASTA_PROFILE.md` の行 |
| `doc/ideas/novel-platform.md` | 48・54 | `` `doc/PASTA_PROFILE.md` `` |
| `doc/shiori/README.md` | 53・84 | `equivalence_evidence.toml`（パスを付けない書き方） |
| `doc/shiori/equivalence_evidence.toml` | 1 | 自分の名前（`# equivalence_evidence.toml — ...`・要件 2.4 で直してよい） |
| `doc/CONSTITUTION.md` | 1 | 見出し `# areka プロジェクト憲法 — CONSTITUTION`（ファイル名ではなく語。直すかは決める・議題 6） |

**本 spec の仕事として述べる所（要件 5.3 で残す候補）**

- `.kiro/specs/doc-tree-tidy/` の 3 文書（`brief.md`・`requirements.md`・この `research.md`）。
- `.kiro/steering/roadmap.md` の本 spec を述べる行: 39（版の段の表）・218（ウェーブ C17 の行）・564（台帳の行・`click_through.md` を名指す）・634（棚卸㉞の記録）・1736（チェックの行）。どれも `docs/` を本 spec の仕事として書く。要件 5.3 が名指すのは「台帳の行」だけなので、ほかの 4 行の扱いは決める（議題 4）。

**歴史の場所（要件 5.3 で残す・5.4 の空振りしない確かめに使える）**

| 検索語 | `.kiro/specs/completed/` | `doc/archive/` | `roadmap-history.md` |
|---|---|---|---|
| `click_through.md` | 13 ファイル | 0 | 1 行（380） |
| `CONSTITUTION.md` | 15 ファイル | 1（`ARCHITECTURE.md:165`） | 3 行（1450・1753・2487） |
| `PASTA_PROFILE.md` | 13 ファイル | 0 | 0（761 行目は `doc/{emo2-conformance-scope,PASTA_PROFILE}.md` の波括弧の書き方） |
| `equivalence_evidence.toml` | 3 ファイル | 0 | 0 |

4 つとも歴史の場所に当たりがあるので、要件 5.4 の「空振りしていない」確かめはどれでもできる。

**0 件だった所**: `crates/`・`tools/`・`.claude/`（スキル）・`.github/`・`CLAUDE.md`・`build.rs`・`*.toml`・`*.ps1`・未完了の spec（本 spec を除く）・`.kiro/steering/focus.md`・`tech.md`。

**検索語の落とし穴**:
- `click_through` だけで探すと Rust の識別子（`register_ghost_windows_click_through` など・クレートの 20 余りのファイル・未完了の `elevated-drop-filter/brief.md`）に当たる。パスの名指しではない。`click_through.md` と `docs/` で探す。
- `PASTA_PROFILE.md` だけで探すと、波括弧の書き方（`roadmap-history.md:761`）を取りこぼす。語幹 `PASTA_PROFILE` で探すほうが漏れない。
- `CONSTITUTION` だけで探すと、`doc/CONSTITUTION.md` の見出しの語に当たる。
- 大文字と小文字を区別しない検索は、新しい名前（`constitution.md`）にも当たる。区別して探す。
- `docs/` は URL（`https://fal.ai/docs/...`・`code.claude.com/docs/...`）にも当たる。URL を除く。

### 2.5 文書を実際に開くコード（要件 5.5）

| 読むもの | 場所 |
|---|---|
| `doc/ukadoc-coverage/` | `crates/ukadoc-survey/`（`catalog/read.rs`・`check/*.rs`・`documents/parse.rs` ほか） |
| `doc/ssp-mcp/tools-list-ssp-2.9.05.json` | `crates/areka-mcp/src/tools/tools_tests.rs`・`tools_socket_tests.rs`・`tools_own_socket_tests.rs` |
| `doc/ssp-mcp/areka-tools.md` | `crates/areka/src/emo2_boot/strict_judges_tests.rs:90`（`include_str!`） |
| `doc/ssp-mcp/log-convention.md` | `crates/areka/src/log_history_convention_tests.rs:240` |
| `doc/release-notes/` | `tools/release-notes.ps1`・`.github/workflows/release.yml`・`.claude/skills/release/procedure.md` |
| `doc/roadmap-overview-{日付}.html`（書く） | `.claude/skills/kiro-next/overview/roadmap-overview.ps1:786` |
| `doc/impl-watch.md`（文で名指すだけ） | `crates/areka-impl-watch/src/error.rs` と試験 |

**動かす 4 つを開く・`include_str!` で取り込む・`build.rs` で読むものは 0**。`Cargo.toml` の `readme`/`include`・`tools/package.ps1`・`tools/crates-io.ps1`・`.gitattributes` にも `doc/`・`docs/` の名指しは無い。

### 2.6 表の役目の書き方と実物の食い違い（要件 3.5）

- `ssp-mcp/` の行は「`tools-list-ssp-2.9.05.json`・`log-convention.md` はテストが読む」だが、`areka-tools.md` も試験が読む（`include_str!`・上の表）。役目の書き方を足す候補。名前と置き場は変えない（要件 6.1）。
- `pasta-real-run.md` の行の「v0.1.0 の版の門の最後の確かめで回す」は、v0.1.0 が出た今は過去の話。書き方を直すかは棚卸のときに決める。
- ほかの行（`images/`＝根の `README.md` の `emo2-boot.png`・`emo2-conformance-scope.md`＝網羅台帳の 3 ファイルが引く・`impl-watch.md`＝道具のエラー文・`archive/` の中身）は実物と合う。

### 2.7 照合テストを置ける所

- **今ある型**: `crates/log-capture-kit/tests/` に、ワークスペース全体を走査する見張りが 4 本ある（`file_length_guard_test.rs`・`temp_path_guard_test.rs`・`sample_path_guard_test.rs`・`with_default_guard_test.rs`）。`temp_path_guard_test.rs` の頭に「この `tests/` はワークスペース全体の見張りの置き場（設計 C6）」とある。共有の道具箱 `tests/workspace_scan/mod.rs` が `workspace_root()`（`CARGO_MANIFEST_DIR` の 2 段上）を持つ。
- **今ある規律**（`file_length_guard_test.rs`）: 例外表は `const`・各項目に理由・「もう事情が無い項目は赤」（減る向きにだけ動く）・「0 件なら緑」への較正として、表の全項目に実体があること・実測と表の完全一致・既知の 1 件を外すと赤になる自己検査。要件 4.7〜4.9 はこの型そのまま。失敗の文は日本語で、直し方を書く（要件 4.2・4.3 と同じ形）。
- **全体テストでの回り方**: `tools/test-all.ps1` は `cargo test --workspace --no-fail-fast -j 4`（段「x64 workspace tests」）と、道具の `-SelfTest` の段（`encoding-check.ps1`・`release-notes.ps1 -SelfTest`・`roadmap-overview.ps1 -SelfTest` など）を回す。どちらに置いても要件 4.11 を満たせる。
- **ワークスペースは `members = ["crates/*"]`**。新しいクレートを足すと自動で一員になり、`Cargo.lock` に項目が増える＝要件 6.3 に反する。
- **同じウェーブの見直しの受け持ち**: `log-capture-kit` も `tools/` も C17 の 7 本の受け持ちの外。後のウェーブの `audit-test-kits`（`crates/log-capture-kit/` を受け持つ）と `audit-tools-scripts`（`tools/` の `perf/` の外・`test-all.ps1` を含む）は、今は brief だけ。どちらに置いても、その見直しの範囲が 1 ファイル増える。
- **1,000 行の番人**: `crates/**/*.rs` に置けば行数の見張りの対象になる（新しいファイルは 1,000 行以下）。
- **道具の文字の決まり**: `tools/*.ps1` と `.claude/skills/**/*.ps1` は `encoding-check.ps1` の規則 A で ASCII だけ。PowerShell で書くなら、案内の見出し（日本語）を `` `u{...} `` で組む必要がある。

## 3. 要件と今の姿の対応

| 要件 | 今あるもの | 足りないもの | 印 |
|---|---|---|---|
| 1.1〜1.3 `docs/` の始末 | `docs/click_through.md` 1 つ・相対リンク無し | 移すだけ（`git mv`） | 足りない |
| 2.1・2.2 名前の決まり | 無し | 案内に決まりの節 | 足りない |
| 2.3・2.4 改名 3 つ | — | `git mv`・自分の名前の行 | 足りない／決まりごと（大文字と小文字だけの改名） |
| 2.5〜2.7 例外の表 | 無し | 表（名前・理由・行方）と 2 行 | 足りない |
| 3.1・3.2 置き方の決まり | 決まりの節（3 行） | 書き足しと「機械の検査は無い」の書き替え | 足りない |
| 3.3 直下の表 | 表あり・実物と合う | 改名した 2 行の名前・`click-through.md` の行 | 足りない（小） |
| 3.4 対応の表 | 無し | 4 行の表 | 足りない |
| 3.5 棚卸 | — | `ssp-mcp/` の行など役目の書き方（2.6） | 足りない（小） |
| 3.6 `structure.md` | 18 行目に `docs/` | `docs/` を消し、`doc/README.md` を正本と名指す | 足りない |
| 4.1〜4.11 照合テスト | 型（`log-capture-kit/tests/`）・`workspace_root()` | 試験 1 本 | 足りない／分からない（置き場・表の書式） |
| 5.1〜5.4 名指しの直し | 一覧は 2.4 | 9 ファイル約 15 行の書き替えと最後の検索 | 足りない（小）／決まりごと（検索語） |
| 5.5 振る舞いを変えない | 開くコードは 4 つを読まない | 全体テストで確かめる | 揃っている |
| 6.1〜6.4 ウェーブの約束 | 4 つの名指しは見直しの受け持ちに 0 件 | 置き場を見直しの外にする | 揃っている（置き場の選び方だけ） |
| 6.5 後続の起票 | `COMPAT_ARCHITECTURE.md` の名指しは今、クレート 18・未完了の spec 64・steering 6・`doc/` 15（`doc/archive/` を除く）・スキル 1（`kiro-next/SKILL.md`）・完了済み 466 | 完了のときの `/kiro-discovery` | 完了のときの仕事 |

## 4. 作り方の案

### 案 A: 今の見張りの置き場に足す（`crates/log-capture-kit/tests/doc_tree_guard_test.rs`）

- 足すのは試験ファイル 1 つ。`mod workspace_scan;` で `workspace_root()` を使い、`std::fs::read_dir` で `doc/` を数え、`doc/README.md` を読む。依存は足さない（標準ライブラリだけ・日付の型は 1 行の手書きの照合で足りる）。
- 判定は純粋な関数（実物の名前の一覧と案内の文字列を受けて、食い違いの一覧を返す）にし、自己検査（要件 4.9）は実物の一覧から 1 行を外す／決まりに合わない名前を 1 つ足して同じ関数に通す。`file_length_guard_test.rs` と同じ形。
- 良い所: 「ワークスペース全体の見張りの置き場」という今の決めに沿う。`cargo test --workspace` で回り、`-p log-capture-kit` で単独でも回せる。日本語の見出しをそのまま書ける。依存も `Cargo.lock` も変わらない。
- 気になる所: クレートの名前と説明は「ログ捕捉」で、文書の見張りは中身が違う（ただし `temp_path_guard_test.rs` や `sample_path_guard_test.rs` も、ログと関係のない見張りとして既にここに住む）。後の `audit-test-kits` の範囲が 1 ファイル増える。

### 案 B: 道具のスクリプトにする（`tools/doc-tree-check.ps1` ＋ `test-all.ps1` に段を 1 行）

- `encoding-check.ps1`・`release-notes.ps1 -SelfTest` と同じ形で、`-SelfTest` に較正を持つ。
- 良い所: Rust の外のことは道具で、という見方に合う。`git ls-files` で追跡しているファイルだけを数えることもできる。
- 気になる所: ASCII だけの決まりで、日本語の見出しを `` `u{...} `` で組む・UTF-8 の読みを気にする必要がある（読む側を直すという方針にも気を配る）。`test-all.ps1` は後の `audit-tools-scripts` の受け持ち。いつもの「`cargo test` の赤だけを回し直す」流れに乗らない。

### 案 C: 今 `doc/` を読んでいるクレートに足す（`crates/ukadoc-survey/` の試験）

- 良い所: 既に `doc/` を読む入出力の層がある。
- 気になる所: 網羅台帳の道具で、`doc/` の置き方の見張りとは役目が違う。後の `audit-ukadoc-survey` の範囲に入る。勧めにくい。

### 取れない案

- 新しいクレート（`crates/doc-tree-guard/` など）: `members = ["crates/*"]` で `Cargo.lock` が変わる（要件 6.3）。
- C17 の見直しの受け持つクレート（`areka-parsers`・`areka-sakura`・`areka-emo-present`・`wintf`・`areka-ghost`・`areka-nar`・`areka-user-data`・`areka-emo-compose`・`areka-seriko`）と `areka-emo-text`（要件 6.2〜6.4）。

### 改名と名指しの直しの進め方（どの案でも同じ）

1. `git mv` で 4 つを動かす（大文字と小文字だけの 2 つは 2 段で動かす案もある・議題 7）。
2. 2.4 の「歴史の外」の行を書き替える。
3. 案内に決まり・例外の表・対応の表を足し、直下の表を直す。`structure.md` を直す。
4. 照合テストを足す。
5. 最後に、歴史の場所で当たりが出ることを確かめてから、歴史の外で 0 件を確かめる（要件 5.4）。

## 5. 規模と危うさ

- **規模: S**（ファイルの移動 4・書き替え約 15 行・案内の書き足し・試験 1 本）。今ある型をそのまま使い、外とのつなぎは無い。brief の見立て（6〜10 タスク）の下の方に入る。
- **危うさ: 低〜中**。型は揃い、名指しは少ない。中にする理由は 2 つ: ⑴ Windows（`core.ignorecase=true`）での大文字と小文字だけの改名（`CONSTITUTION.md`→`constitution.md`・`PASTA_PROFILE.md` は綴りも変わるので当たらない）は、ほかの作業場で取り込んだときに古い字のまま残るおそれがある（未確認・7 の調べ）。照合テストはディスクの実際の字で数えるので、残った作業場では赤になる。⑵ 表の書式を照合テストが読むので、後から案内を書き直す人が書式を崩すと赤になる（書式の決まりも案内に書く必要がある）。

## 6. 設計で決めること（議題の候補）

1. **照合テストの置き場**: 案 A（`log-capture-kit/tests/`）・案 B（`tools/` の道具）・案 C（`ukadoc-survey`）。
2. **数える実物**: ディスク（`read_dir`・追跡外のファイルも数える＝置いただけで表に足していないものを手元でも赤にできる）か、git の索引（`git ls-files`・追跡しているものだけ・試験が git を呼ぶ）か。要件 4.10 の「リポジトリの中のファイルだけを読む」とどちらが合うか。
3. **案内の表の書式**: 照合テストが表を見分ける手がかり（見出しの文字）、例外の表の列（名前・理由・行方）、対応の表の列、フォルダの行の書き方（末尾 `/`）。書式の決まりそのものも案内に書くか。
4. **roadmap の本 spec を述べる行の扱い**: 要件 5.3 が名指すのは「台帳の行」（564）だけ。版の段の表（39）・ウェーブ C17 の行（218）・棚卸㉞の記録（634）・チェックの行（1736）も「本 spec の仕事として述べる所」に入れて残すか。
5. **roadmap の「利用統計とノベルゲーム基盤」の節（1721・1734）と `doc/ideas/novel-platform.md`（48・54）の `doc/PASTA_PROFILE.md`**: どちらも「その場で直した」という過去の記録だが、置き場は歴史の場所の外。要件 5.2 のとおり新しいパスへ直す（読み手が今のファイルへ辿れる）か、記録として残すか。
6. **`doc/CONSTITUTION.md` の見出しの語 `CONSTITUTION`**: ファイル名ではなく語なので、直さない（中身を変えない・要件 2.4）か、名前に合わせて直すか。最後の 0 件の検索は `CONSTITUTION.md` で探すなら当たらない。
7. **大文字と小文字だけの改名の手順**: `git mv` 1 回か、一度別の名前を挟む 2 段か。ほかの作業場で古い字が残ったときの扱い（照合テストの赤の文に「作業場の字を直す」を添えるか）。
8. **名前の決まりの読み方**: 字の種類だけ（`[a-z0-9.-]` だけ）か、つなぎ方まで（先頭・末尾・続けてのハイフンや点を許さない）か。今ある名前はどちらでも合う。
9. **例外の表のフォルダの行の「もう決まりに合っている」（要件 4.7）**: フォルダ `fragments` の名前は決まりに合うので、名前だけを見ると今すぐ赤になる。フォルダの行は「下に決まりに合わない名前が 1 つも無くなったら赤」と読むのが要件の意図に合う。
10. **最後の検索の語と道具**: `docs/click_through`・`click_through.md`・`CONSTITUTION.md`・語幹 `PASTA_PROFILE`（波括弧の書き方を拾う）・`equivalence_evidence`、大文字と小文字を区別、URL の `docs/` を除く、追跡しているファイルに限る（`git grep`）。手順として tasks に書くか、照合テストに入れるか（入れると歴史の外の名指しの 0 件が常時の試験になるが、要件 4 の範囲を超える）。
11. **表の役目の書き方の直し（要件 3.5）の範囲**: `ssp-mcp/` の行に `areka-tools.md` を足す、`pasta-real-run.md` の行の時制。どこまで直すか。

### 6.1 要件の話し合いでの振り分け（2026-11-05）

- **設計の段で決める**: 1（置き場・推しは案 A）・2（数える実物）・3（表の書式と、書式の決まりを案内に書くこと）・7（大文字と小文字だけの改名の手順）・8（名前の決まりの読み方）・10 の検索の語と道具（手順として tasks に書く）。
- **要件で決めた（自明な直し）**: 4（roadmap で本 spec の仕事を述べる行は残す＝要件 5.3）・6（見出しの語は変えない＝要件 2.4）・9（フォルダの行の読み方＝要件 4.7）。
- **要件の今の文のまま**: 5（歴史の場所の外なので新しいパスへ直す＝要件 5.2）・11（表の役目の書き方は棚卸で直す＝要件 3.5。`ssp-mcp/` の行に `areka-tools.md` を足すのは案内の文だけで、名前と置き場は変えない）。
- **開発者が推奨案で承認（2026-11-05）**: 名前の決まりは全部に効かせ大文字の 2 つも改名（完了のとき、ぱすた開発のセッションへ知らせる＝要件 5.6）・`docs/click_through.md` は `doc/click-through.md` へ・完了済みの spec の古いパスは歴史として残す・`COMPAT_ARCHITECTURE.md` は例外の表に載せ完了のとき改名の spec を起票・古い名前の見張りは完了のときの 1 回だけ（10 の後半は試験に入れない）。

## 7. 調べが要ること（設計の段へ）

- **大文字と小文字だけの改名を、Git for Windows で別の作業場が取り込んだときの振る舞い**（古い字が残るか・`git pull`／`git merge` のときに何が起きるか）。手元で 1 度試してから手順を決める。
- **照合テストが走る間に `doc/` の直下へ書く試験や道具が無いか**（全体テストの中で `roadmap-overview.ps1 -SelfTest` などが `doc/` に書くと、追跡外のファイルが一時に増える）。今の読みでは、`roadmap-overview.ps1` の既定の書き先は型の行に当たり、ほかに `doc/` の直下へ書くものは見当たらない。

## 8. リポジトリの外（本 spec では直さない・知らせるだけ）

- ぱすたのゴーストの開発のリポジトリと、開発者のメモ（Claude の記憶のファイル）には `doc/PASTA_PROFILE.md` の名指しがある。外なので直さない（要件の範囲外）。案内の対応の表（要件 3.4）で辿れる。

## 9. 設計の段の調べと決め（2026-11-05）

- 調べの型: 軽い調べ（今ある仕組みへの足し）。外の道具やクレートは足さないので、ネットの調べはしない。
- 使った手引き: `kiro-spec-design` の規則（`design-principles.md`・`design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`）とひな形。ほかの領域の手引きは当たらない（画面も外とのつなぎも無い）。

### 9.1 大文字と小文字だけの改名（7 の 1 つ目）

- 試した所: `target/doc-tree-tidy-case-test/` の使い捨てのリポジトリ（試した後に消した）。Git for Windows 2.55・`core.ignorecase=true`。
- 試したこと: 元のリポジトリで `git mv doc/CONSTITUTION.md doc/constitution.md` を 1 回打ってコミットし、⑴ 別の写しで `git pull`、⑵ 別の作業場で `git merge main`、⑶ 同じファイルの中身を直した枝で `git merge main`、⑷ 別の枝で `git rebase main`。
- 分かったこと: `git mv` 1 回で索引もディスクも新しい字になる。⑴〜⑷ のどれでも、ディスクの字が `constitution.md` に変わり、古い字は残らない。⑶ は名前の変更として追われ、中身の直しも入る。
- 決め: 間に別の名前を挟む 2 段の手順は取らない。`git mv` 1 回。照合テストの赤の文に「作業場の字を直す」は添えない（git が字を直すので要らない。万一残っても照合テストがディスクの字で `BadName` を出す）。
- 気づき: Windows は名前の大文字と小文字を区別しないので、古い字のリンクは手元では開けてしまう。最後の検索は大文字と小文字を区別して行う（`git grep` の既定）。

### 9.2 全体テストの間に `doc/` へ書くもの（7 の 2 つ目）

- `crates/ukadoc-survey/` の試験は書き手を差し替えてファイルを作らない（`cli/generate_tests.rs` の注記）。`doc/` の下へ書く試験は見当たらない。
- `roadmap-overview.ps1` の既定の書き先 `doc/roadmap-overview-{updated_at}.html` は直下の表の置き字の行に当たる。全体テストの段は順に回る。
- `.gitignore` に `doc/` の下を外す行は無く、`doc/` の下に追跡外のファイルは無い。`.github/workflows/` に `cargo test` を回す流れは無い（照合テストは手元の全体テストだけで回る）。
- 決め: 照合テストはディスクで数えてよい。ほかの試験と並んで走っても結果は変わらない。

### 9.3 最後の検索の語の試し（6 の 10）

- 除く場所（歴史の場所と本 spec の文書）を付けて `git grep` で全域を探した。`click_through.md`・`CONSTITUTION.md`・`PASTA_PROFILE`・`equivalence_evidence`・URL を除いた `docs/`（`-P '(?<![\w./:-])docs/'`）の当たりは、2.4 の「歴史の外」の表と同じだった。
- `.kiro/steering/roadmap.md` で本 spec の仕事を述べる 5 行（39・218・564・634・1736）は、どれも `doc-tree-tidy` の語を含む。残してよい行を「`doc-tree-tidy` を含む行」と語で決められる（行番号は動くので使わない）。

### 9.4 決めたこと

| 議題 | 選んだもの | 選ばなかったもの | 理由 |
|---|---|---|---|
| 照合テストの置き場（6 の 1） | 案 A `crates/log-capture-kit/tests/doc_tree_guard_test.rs`・`workspace_root()` を使う | 案 B（`tools/` の PowerShell）・案 C（`ukadoc-survey`） | 今ある見張りの置き場で、型（例外・較正・自己検査）がそのまま使える。日本語の見出しをそのまま書ける。依存も `Cargo.lock` も変わらない。新しいファイルなので、`file_length_guard_test.rs` の例外表を直すかもしれない見直しとぶつからない |
| 数える実物（6 の 2） | ディスク（`read_dir`） | git の索引（`git ls-files`） | 要件 4.10 の「リポジトリの中のファイルだけを読む」に合い、試験が git を呼ばずに済む。置いただけで表に足していないファイルを、置いた人の手元で赤にできる。追跡外のファイルは今 0 で、書き込む試験も無い（9.2） |
| 表の見分け方（6 の 3） | 見出しの文字の完全一致で表を見分け、1 列目のバッククォートの名前を取る。直下の表は `doc/` からの名前（今のまま）、例外の表と対応の表は根からのパス | 全部の表を同じ起点にする | 対応の表には `doc/` の外の `docs/click_through.md` が載るので根から書く必要がある。例外の表もそろえる。直下の表は今の書き方を変えない |
| 表の書き方を案内に書くか（6 の 3） | 書く（「決まり」の節に「表の書き方」） | 試験の注記だけ | 案内を書き直す人が見出しを変えると赤になる。読む人の目に入る所に書く |
| 名前の決まりの読み方（6 の 8） | 字の種類だけ（`a-z`・`0-9`・`-`・`.`） | つなぎ方まで（先頭・末尾・続けてのハイフンや点を許さない） | 要件 2.1 が書くのは字の種類で、つなぎ方の決まりは無い。今の名前はどちらでも合う。足すと要件に無い決まりになる |
| 1 列目が崩れた行 | 赤（`MalformedRow`） | 黙って読み飛ばす | 読み飛ばしても別の赤になるが、何が悪いかが分かりにくい |
| 例外の行の「要らなくなった」（4.7） | 「覆う破れが 1 つも無い行」の 1 つの規則で、無い・もう合っている・フォルダの下に破れが無い、の 3 通りをまとめて見る | 3 通りを別々に判定する | 判定が 1 つで済み、ファイルの行とフォルダの行で同じ規則になる |
| 自己検査の比べ方（4.9） | 実データの基準との差がちょうど 1 件 | 実データが緑であることを前提にした絶対の比較 | 実データが別の理由で赤のときも、自己検査は自分の 1 件だけを確かめられる |
| 最後の検索（6 の 10） | `git grep`・大文字と小文字を区別・5 つの語・先に歴史の場所で当たりを確かめる・残ってよいのは roadmap の `doc-tree-tidy` を含む行と対応の表の行。tasks の最後のタスクの手順として書く | 照合テストに入れる | 要件 5.4 と要件の話し合い（完了のときの 1 回だけ）のとおり |
| 棚卸の直し（3.5・6 の 11） | `ssp-mcp/` の行に `areka-tools.md` を足し、`pasta-real-run.md` の行の時制を直す | `ssp-mcp/` の行だけ | どちらも実物と食い違う役目の文で、要件 3.5 の「食い違っていれば直す」に当たる |
| 完了の手当ての置き場 | tasks の最後のタスク（検索・起票・行方の書き替え）。「ぱすた開発」への知らせは進行役 | `/kiro-complete` の中で行う | 例外の決まりは本 spec の文書の中に置く（共通の手順は直さない）。伝言は外のセッションへの連絡なので進行役が持つ |

### 9.5 まとめ直し（一般化・作るか使うか・削る）

- 一般化: 例外の行の「要らなくなった」の 3 通りを 1 つの規則（覆う破れが無い行）にまとめた。直下の表の行と実物の突き合わせも、置き字の行と普通の行を同じ `row_matches` で読む。
- 作るか使うか: 表の読みと日付の型の照合は標準ライブラリで数行に収まる。正規表現や Markdown のクレートは足さない（依存を足さない約束・要件 6.3）。根の場所は既存の `workspace_root()` を使う。
- 削ったもの: 対応の表を照合テストで読むこと（要件に無い）。「理由」「行方」の列が空でないかの判定（書くのは案内の仕事）。表の読み方を道具箱 `workspace_scan` へ出すこと（ほかに使う当てが無い）。古い名前の常時の見張り（要件の話し合いで外した）。

### 9.6 危うさと手当て

- `.kiro/steering/structure.md` の 18 行目はとても長い 1 行で、ほかの spec が同じ行を直すと取り込むときにぶつかりやすい。手当て: 直すのは `docs/` の項を消すことだけに絞り、正本の名指しは Workspace Root の別の行に足す。
- 本 spec の完了の後に、ほかの枝が古いパスを新しく書くことがある。手当て: 案内の対応の表で辿れる。照合テストは古いパスの名指しを見ないので、そこは見落とす（要件の話し合いで受け入れた範囲）。
- 同じウェーブの見直しが `doc/` の下に決まりに合わない名前を足すと、本 spec の着地の後に取り込んだ枝で赤になる。手当て: 意図どおり（その枝で名前を直すか例外の表に足す）。見直しの約束は `doc/`・`docs/` の名指しを足さない・変えないこと。

## 10. 設計の話し合いの決め（2026-11-05・開発者が推奨案で一括承認）

- 照合テストの置き場は `crates/log-capture-kit/tests/doc_tree_guard_test.rs`（今ある見張りの置き場・依存も新しいクレートも要らない・日本語の見出しをそのまま書ける）。`tools/` の道具にする案は取らない。
- 実物はディスクで数える（`read_dir`）。表への行の足し忘れを置いたその場で見つけるため。手元にだけあるファイルで赤になる件は、赤の文に「追跡していないファイルなら取り除く」を添えて手当てする。`git ls-files` で数える案は取らない。
- 名前の決まりは字の種類（`a-z`・`0-9`・`-`・`.`）だけを見る。先頭・末尾・続けてのハイフンや点は見ない（今ある名前はどちらでも合い、決まりが短いほど後から文書を足す道具が合わせやすい）。
