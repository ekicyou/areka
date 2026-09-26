---
name: kiro-complete
description: 'Kiro仕様駆動開発のSpec完了ワークフローを実行する。未解決問題の棚卸（軽微なものはその場で解決・残りは/kiro-discoveryで起票）→DoD静的ゲート→コミット→completedフォルダ移動→spec.json更新→参照パス修正（spec文書＋ソース全域）→アーカイブのコミット→全体テスト1回（整形・ライセンス込み・移動後）を裏で起動し、その待ち時間にROADMAP更新・スキルドキュメント同期・PR文面の下書きを並行実施→判定と最終コミット→PR作成→squashマージまでを中断なく完遂する。mainへの統合はこのPRが唯一の経路（直push禁止）。Use when: 実装完了を承認する, 承認してください, 完了を承認, spec承認, approve implementation, kiro承認完了。DO NOT USE when: 実装が完了したのみ（承認の明示がない場合）、タスクが終わっただけ'
allowed-tools: Bash, Read, Write, Edit, Glob, Grep, AskUserQuestion
argument-hint: <feature-name>
---

# Kiro Spec 完了ワークフロー

## 発動条件（必須）

> **⚠️ このスキルは開発者の明示的「承認」がある場合にのみ発動する。**

### ✅ 発動する（承認の明示がある）
- 「実装完了を**承認**します」「**承認**してください」
- 「このspecを**承認**する」「**approve**」「kiro **承認**完了」

### ❌ 発動しない（承認の明示がない）
- 「実装が完了した」「タスクが全部終わった」
- 「spec完了」「アーカイブしてほしい」などの曖昧な表現のみ
- AIが自律的に「完了したと判断」した場合

### 承認が不明瞭なとき
発動を迷う場合は **発動しない**。必要なら開発者に確認する:
> 「実装完了の承認をいただけますか？承認いただいた場合、完了ワークフローを実行します。」

---

## いつ使うか
- 開発者が上記「発動する」に該当する承認を**明示的に**宣言したとき
- tasks.md の全タスクが `[x]` 完了している状態で使用
- 設計文書リフレッシュが完了した後

## 完了基準の権威（ベースライン内蔵 + 任意の workflow.md）

> **このスキルはベースラインの完了基準（DoD）・コミット規約を内蔵し、単体で機能する。**
> 完了基準（DoD）、コミット規約、ドキュメント更新判定の既定値はこのスキル内に定義する（下記）。
> **`.kiro/steering/workflow.md` が存在する場合のみ**、それを追加の権威として優先し、定義されたゲートをベースラインに上乗せする。存在しなければベースラインのみで完了できる。

## 哲学
- **中断せず一連で完遂する** — 全ステップを止めずに実行
- **VSCodeの変更ファイル確定挙動を回避** — spec.json編集は移動後に行う
- **ベースライン完了基準を内蔵** — DoD・コミット規約はこのスキルが既定値を持つ。`.kiro/steering/workflow.md` が存在すればそれを優先（任意）
- **mainへの統合はPRが唯一の経路** — フィーチャーブランチはハーネスのワークツリーが供給。1 feature = 1 branch = 1 PR。`{default-branch}` への直接 push は一切行わない
- **繰り返し仕様は移動しない** — 繰り返し実行型の仕様は常に `.kiro/specs/` 直下に留まる
- **移動はコードを壊し得る** — ソースが spec 文書を実ファイル読みしていればアーカイブ移動で壊れる。参照検索はソース全域まで及ぼし（ステップ4-2）、**全体テストは移動をコミットした後に回す**（ステップ5）
- **全体テストは 1 回・移動の後で回す** — 全体テストの 1 回は約 19 分かかる。以前は移動の前（DoD ゲート）と後（移動後ゲート）で 2 回回していたが、**移動後の 1 回は移動前の 1 回を包含する**（同じコードに加えて移動の影響まで見る）ので、整形・ライセンスも込みで移動後の 1 回にまとめる。確かめる範囲は減らず、待ち時間が半分になる
- **待ち時間は並行レーンで使う** — 全体テストは裏で回し、その間にテストの結果に左右されない作業（ROADMAP・steering・スキル文書の更新、PR 文面の下書き、`gh` の認証確認）を進める。並行レーンで触ってよいものは**テストが読まないファイルだけ**に限る（ステップ6）
- **全テストは全体テストのスクリプト `tools/test-all.ps1` で回す** — `cargo test --workspace` を直に叩くと i686 の準備が抜け、終了コードも見落としやすい（ステップ5）

## 前提条件
- `.kiro/specs/{feature}/tasks.md` の全タスクが完了
- 設計文書と最終実装の整合確認済み

## 例外: 繰り返し仕様

リリース手順やレビュー・ループのような繰り返し実行型仕様は `completed/` に**移動しない**。

判定基準:
- spec.json や requirements.md に「繰り返し」「repeatable」「定期実行」「ループ」等の記述がある
- `/kiro-impl` のたびにタスクがリセットされる設計

繰り返し仕様の場合:
1. 冒頭ステップ・ステップ0〜2 を実行する
2. ステップ3〜4（移動・spec.json・参照パス）をスキップ（`completed/` へは移動しない）
3. tasks.md のチェックボックスをリセット（全 `[x]` → `[ ]`）してコミットし、ステップ5（全体テストの起動）・ステップ6（並行レーン）・ステップ7（判定と最終コミット）を実行する
4. ステップ8（リモート同期＝PR ベース）を実行

---

## 全体の流れと時間の使い方

```
冒頭 棚卸・その場の修正・起票 ─┐
0  portable context・gh 確認    │ 直列（数分）
1  DoD 静的ゲート（Spec 等）    │
2  未コミットのコミット         │
3  completed/ へ移動・spec.json │
4  参照パスの更新               │
5  アーカイブのコミット → 全体テスト（-Format -License）を裏で起動 ─┐
6  並行レーン: ROADMAP／steering／スキル文書・PR 文面の下書き      │ 並行（約 19 分）
7  完了通知を待って判定 → 最終コミット ←────────────────────────┘
8  PR 作成 → squash マージ
```

- **全体テストはステップ5 の 1 回だけ**。これが Format / Test / License のゲートであり、同時に移動後のゲートでもある。
- ステップ5 より前に、**テストが読むものをすべて確定させる**（コード・起票による spec ディレクトリの増減・参照パスの書き換え）。ステップ5 より後は、テストが読まないものしか変えない（ステップ6 の許可範囲）。

---

## 手順

### 冒頭ステップ: 未解決問題の棚卸（軽微はその場で解決・残りは `/kiro-discovery` で起票）

DoD ゲートより前に、**実装中に発生した未解決問題のうち未起票のもの**を棚卸しし、**直ちに実施可能な軽微なものはこのブランチでその場で解決**し、残りを `/kiro-discovery` で起票する。完了してアーカイブした後では拾い漏れに気付けなくなるので、必ず最初に行う（繰り返し仕様でも行う）。その場の修正も全体テスト（ステップ5）の対象になるよう、テストの起動より前に済ませる。

1. **未解決問題を洗い出す**。拾う元は次の通り。
   - `tasks.md` の `## Implementation Notes`・`_Blocked:_` の付いたタスク
   - `design.md` の Open Questions / Risks
   - 実装・レビュー・最終検証（`/kiro-validate-impl`）の報告に残った未対応の指摘・先送り
   - 会話の中で「別 spec で」「後で」と決めたもの
2. **起票済みかを確かめる**。`.kiro/specs/`（`completed/` を含む）の brief.md と `.kiro/steering/roadmap.md` を検索し、既に担当 spec・roadmap の行があるものは除く。本 spec の中で解決済みのものも除く。**「直さない」と裁定済みのものも除く**（触るコードのコメント・roadmap・完了 spec の台帳に取り下げの記録が無いかを確かめる）。
3. **軽微なものはその場で解決する**。次の**すべて**を満たすものを「軽微・直ちに実施可能」とする（1 つでも外れれば手順 4 の起票へ）。
   - 開発者の裁定・設計判断・要件の追加や改訂を要しない（直し方が 1 通りに決まる）
   - 変更が小さく閉じている（目安: 数ファイル・数十行。文言・doc・テストの穴・局所的なバグ修正など）
   - 進行中の他 spec の担当範囲（brief・roadmap の行）と重ならない
   - 公開 API・データ形式・依存（`Cargo.toml`）を変えない

   解決の作法:
   - バグ修正には、直す前に赤になる決定論テストを 1 本添える（doc・文言だけの修正は不要）。
   - 修正は 1 件 1 コミット（`fix(<feature>): …` 等）で、ステップ2 の前に積む。
   - 着手して上の条件を外れると分かったら（裁定が要る・波及が広い・他のテストが赤になる）、その修正を戻して手順 4 の起票へ回す。無理に押し込まない。
   - 解決したものは `tasks.md` の `## Implementation Notes` に「完了時にその場で解決」として 1 行ずつ記録し、PR の body（ステップ8）にも載せる。
4. **残った未起票のものを `/kiro-discovery` で起票する**。Skill ツールで `kiro-discovery` を起動する。1 件ずつでも、まとめて 1 回でもよい。
   - **起票は全体テストの起動（ステップ5）より前に済ませる。** 起票は `.kiro/specs/` の直下に spec ディレクトリ（brief.md 付き）を作る。テストの中にはこの直下を 2 回数えて突き合わせるもの（`crates/ukadoc-survey/tests/consistency/documents_non_vacuity.rs` の数え直し）があり、テストの最中にディレクトリが増えると偶発的に赤になる。
5. 起票した brief.md・roadmap.md の変更はステップ2でまとめてコミットする。**その場で解決した件数と起票した件数をそれぞれ明示して記録する。0 件なら「0 件」と書く**（黙って飛ばさない）。

### ステップ0: 決定的解決（portable context）

リモート操作で用いる `{remote}` と `{default-branch}` を、**固定優先順序で1回だけ決定的に解決**する。各値はちょうど1つの結果（または明示的なスキップ）に収束させ、推測しない。解決した値は以降のステップ（特にステップ8）で `origin`/`main` のハードコードの代わりに再利用する。この優先順序は `kiro-start` の「Step 0: Resolve portable context」と整合している。

1. **デフォルトリモート（`{remote}`）**: `git remote` を実行し、以下の固定ルールを適用する。
   - `origin` が存在する → `{remote}` = `origin`。
   - そうでなく、リモートがちょうど1つだけ存在する → `{remote}` = そのリモート。
   - それ以外（リモートなし、または `origin` を含まない複数リモート）→ `{remote}` = none。リモート操作はすべてスキップ扱いとする（一度だけ警告する）。

   ```powershell
   $remotes = git remote
   if ($remotes -contains "origin") { $remote = "origin" }
   elseif (@($remotes).Count -eq 1) { $remote = $remotes }
   else { $remote = $null }  # none: リモート操作はスキップ
   ```

2. **デフォルトブランチ（`{default-branch}`）**: 以下の固定優先順序で決定的に解決する。
   - `{remote}` が解決済みなら、`git symbolic-ref --quiet --short refs/remotes/{remote}/HEAD` を読み、先頭の `"{remote}/"` プレフィックスを除去した名前。
   - それが空で、ローカルに `main` ブランチが存在する → `{default-branch}` = `main`。
   - そうでなく、ローカルに `master` ブランチが存在する → `{default-branch}` = `master`。
   - それ以外 → `{default-branch}` = 現在のブランチ。

   ```powershell
   $defaultBranch = $null
   if ($remote) {
     $defaultBranch = git symbolic-ref --quiet --short "refs/remotes/$remote/HEAD"
     if ($defaultBranch) { $defaultBranch = $defaultBranch -replace "^$remote/", "" }
   }
   if (-not $defaultBranch) {
     if (git show-ref --verify --quiet "refs/heads/main") { $defaultBranch = "main" }
     elseif (git show-ref --verify --quiet "refs/heads/master") { $defaultBranch = "master" }
     else { $defaultBranch = git branch --show-current }
   }
   ```

   `{default-branch}` は1つの具体的なブランチ名として確定し、以降で再評価しない。

3. **既定ブランチとの差を確かめる**: `git fetch {remote}` のあと `git log --oneline HEAD..{remote}/{default-branch}` が空でなければ、既定ブランチが先へ進んでいる。PR が衝突しないか（`git merge-tree --write-tree HEAD {remote}/{default-branch}` が衝突を報告しないか）を確かめ、衝突するならテストより前に取り込む（ハーネスのワークツリーなら `sync_with_base_branch`、それ以外は `git merge`）。取り込みはコードを変えるので、**全体テストの起動（ステップ5）より前**に済ませる。

> **以降のステップは、`origin`/`main` のハードコードではなく、ここで解決した `{remote}` / `{default-branch}` を用いる前提とする。** `{remote}` が none の場合、リモート同期（ステップ8）は安全にスキップし警告する。

### ステップ1: DoD 静的ゲート

テストを回さずに判定できるゲートをここで見る。**Format / Test / License のゲートはステップ5 の全体テスト 1 回が担う**（移動の後で回すため）。

1. **任意**: `.kiro/steering/workflow.md` が存在すれば読み込み、追加の DoD ゲートを取り込む。存在しなければスキップ。テストを伴う追加ゲートはステップ5 の判定に合わせて見る。
2. **Spec Gate**: 当該 spec の `tasks.md` が全タスク `[x]` 完了であること（親の項目のチェックボックスも含む）。
3. workflow.md がある場合は、そこで定義された追加ゲート（例: Doc / Steering 等）のうちテストを伴わないものを順に検証する。
4. **いずれかのゲートが失敗した場合**: ワークフローを中断し、開発者に報告。

### ステップ2: 未コミットファイルのコミット

実装中の変更をすべてコミットする。コミットメッセージ形式は下記のベースライン規約（`<type>({feature-name}): <要約>`）に従う。`.kiro/steering/workflow.md` が存在すればその規約を優先する。

```powershell
git add -A
git commit -m "<type>({feature-name}): 実装完了

- 変更の要約（箇条書き）"
```

未コミットの変更が無ければ何もしない。

### ステップ3: completedフォルダへの移動と spec.json の更新

#### 3-1. 移動

```powershell
New-Item -ItemType Directory -Path ".kiro/specs/completed" -Force | Out-Null
Move-Item ".kiro/specs/{feature-name}" ".kiro/specs/completed/"
```

**重要**: 移動の前に spec.json を**編集しない**。VSCodeが編集中のファイルを追跡しており、移動前に編集すると移動操作と競合してファイルが元の場所に復活する。

#### 3-2. spec.jsonのステータス更新

**移動完了後に** spec.json を更新する。areka の `spec.json` スキーマ（`.kiro/steering/kiro-spec-schema.md`）に従い、以下を変更:

```json
{
  "phase": "completed",
  "updated_at": "YYYY-MM-DDTHH:MM:SSZ"
}
```

- スキーマ規約: **`phase` 変更時は必ず `updated_at` も更新する**。
- 任意で `approvals.implementation` を更新してよい（`completed: true`, `completed_at: "YYYY-MM-DDTHH:MM:SSZ"`）。
- `"status"` フィールドは使用しない。`"phase": "completed"` で完了を表す。

### ステップ4: 参照パスの更新

このspecを参照している箇所のパスを更新する。**参照元は spec 文書だけではない。** ソースコードが spec 文書を実ファイルとして読んでいる場合、アーカイブ移動でその読み込みが壊れ、テストが赤くなる。**ソースの書き換えはテストが読むものなので、全体テストの起動（ステップ5）より前に済ませる。**

#### 4-1. spec 文書からの参照を検索

```powershell
Get-ChildItem ".kiro/specs" -Filter "*.md" -Recurse |
  Where-Object { $_.FullName -notlike "*completed*" } |
  Select-String -Pattern "{feature-name}" |
  Select-Object -ExpandProperty Path | Sort-Object -Unique
```

#### 4-2. ソースコードからの参照を検索（必須）

**4-1 だけではビルドを壊す参照を取りこぼす。** ソース全域（areka では `crates/**/*.rs`）を走査する。

```powershell
# (a) 本命: 移動する feature-name をソース全域から検索する
git grep -n "{feature-name}" -- crates

# (b) 衛生チェック: spec パス全般の参照も一度見ておく（他 spec の移動漏れが混ざっていることがある）
git grep -n "\.kiro/specs" -- crates
```

- **(a) を必ず実行する。** これが今回の移動で壊れ得る参照そのもの。
- (b) は補助。`.kiro/specs` という汎用語はスキル文書などが大量にヒットするため、**`crates`（ソース配下）に絞って**実行する。リポジトリ全域へ広げると kiro スキル自身のヒットに埋もれて使い物にならない。
- `git grep` はサブモジュールへ降りない。`vendors/` 配下は別リポジトリの spec を指しているので対象外。
- **ソースが `crates/` 以外にあるリポジトリ**では、そのソースルート（`src`・`lib`・`app` 等）へ読み替える。`.kiro/` と `doc/` を除いた追跡ファイル全域を走査してもよい。
- 検出がゼロでも「検索を実行した」ことをチェックリストに記録する。

#### 4-3. 検出した参照の仕分け（必須）

検出行を **2 種類に仕分ける**。この判定を省略しない。

| 種別 | 見分け方 | 対応 |
|---|---|---|
| **コメント内の参照**（無害） | `//!` `///` `//` `#` などのコメント行・docコメント・文章中の出典表記 | **放置可**。正確さのために更新してもよいが、しなくてもビルドは壊れない |
| **実ファイル読み**（ビルドを壊す） | パス文字列が `std::fs::read_to_string` / `include_str!` / `File::open` / `fs::read` などへ渡る | **必ず更新**する |

- **(a) は数十行ヒットするのが普通**（実装コードは spec を出典としてコメントで名指しするため）。ヒット数の多さに怯まず、下の絞り込みで実ファイル読みだけを取り出す。

**絞り込みは「コメント行を落とす」で行う。** 実ファイル読みのパスは必ずコード行（文字列リテラル）に現れるので、コメント行を除けば候補だけが残る。

```powershell
# ヒット行からコメント行を落とす。残った行が実ファイル読みの候補
git grep -n "{feature-name}" -- crates |
  Select-String -NotMatch -Pattern ':\d+: *(//|#|\*)'
```

- この絞り込みは areka の実例で検証済み: PR #114 の spec は (a) が 32 行ヒットするが、コメント行を落とすと**実ファイル読みの 1 行だけ**が残る。別の完了 spec では残りゼロ（＝全てコメント参照）になる。
- **読み込み API 名（`read_to_string` 等）での絞り込みは当てにならない。** パスが定数へ切り出されていると API 呼び出しと別行になり、空振りする（PR #114 がまさにこの形）。**空振りは「無い」の証明にならない。**
- 残った行が定数定義（例: `const PROCEDURE_RELATIVE_PATH: &str = "…";`）だった場合は、**定数名で再 grep して使用箇所まで追う**。

```powershell
git grep -n "PROCEDURE_RELATIVE_PATH" -- crates
```

- **コメント参照は放置してもテストは緑のまま。実ファイル読みだけがビルド／テストを赤にする。**
- 相対パス（`env!("CARGO_MANIFEST_DIR")` 起点の `../../.kiro/specs/…` など）も同様に対象。

#### 4-4. パスの一括置換

`.kiro/specs/{feature-name}/` → `.kiro/specs/completed/{feature-name}/`

対象は 4-1 の spec 文書と、4-3 で「実ファイル読み」に仕分けたソース行。

#### 4-5. 親仕様への完了マーク

親仕様のdesign.mdに完了ステータス（✅）を反映する（該当する場合）。

### ステップ5: アーカイブのコミットと全体テストの起動

#### 5-1. アーカイブのコミット

移動・spec.json・参照パスの更新をコミットする。**このコミットが全体テストの検査対象になる**（テストが読むものはここで確定する）。

```powershell
git add -A
git commit -m "chore({feature-name}): spec完了・アーカイブ"
git rev-parse --short HEAD   # 起動時のコミット。ステップ7-3 の差分確認の起点として控える
```

#### 5-2. 全体テストを裏で起動（Format / Test / License / 移動後の各ゲートを 1 回で）

全テストは**必ず全体テストのスクリプト `tools/test-all.ps1` で実行する**。`cargo test --workspace` を直に叩かない——i686 の成果物が無いと host-32 の e2e が panic し、i686 でしか走らないテストは回らず、`| Select-String` で絞れば終了コードが失われて赤でも通って見える。スクリプトは段が赤でも最後まで回して末尾に段ごとの合否を一覧し、1 つでも赤なら終了コード 1 で終わる。

```powershell
pwsh -NoProfile -File tools/test-all.ps1 -Format -License   # Bash／PowerShell ツールの run_in_background で起動する
```

- **必ず `run_in_background` で起動し、完了通知を待つ**（約 19 分・2026-09-24 実測で x64 全テストだけ 838 秒）。前面で回すとツールの時間切れで途中の結果しか残らないうえ、待ち時間に何もできない。起動したらすぐステップ6 へ進む。
- **段**: i686 ターゲット導入 → i686 成果物ビルド（helper・偽 DLL 2 つ）→ `cargo fmt --all`（整形・`-Format`）→ fmt --check → x64 ワークスペース全テスト（`--no-fail-fast -j 4`）→ i686 テスト（host-32 系）→ `cargo deny check` → `cargo about generate`（`-License`）。
- **Format Gate**: `-Format` が「`cargo fmt --all` で整形 → `--check` で確認」をテストの前の段で行う。整形差分はステップ7 のコミットに取り込む。`cargo fmt` が構文エラーで失敗したら中断して報告する（コンパイルできないコードが残っている）。Rust ワークスペースでないリポジトリはチェックリストに「(整形対象不在により省略)」と注記する。
- **License Gate**: MIT 配布を守るライセンス健全性ゲート。`-License` がテストの後に直列で回す（テストと同時に回すと rustc がメモリ不足で落ちる）。
  - (a) 汚染ゲート `cargo deny check`: 強コピーレフト（GPL/LGPL/AGPL/MPL 等）や許可外ライセンスの混入を検出する（`deny.toml` があるときだけ意味を持つ）。
  - (b) 第三者謝辞の再生成 `cargo about generate --workspace about.hbs -o THIRD-PARTY-NOTICES.md`（`about.toml` があるときだけ）。差分は依存が変わった証跡で、ステップ7 のコミットに含める。
  - ツール未導入時は `cargo install cargo-deny --locked` / `cargo install cargo-about --features cli --locked` で導入してから回す。`deny.toml`・`about.toml` が無いリポジトリでは「(設定不在により省略)」と注記する。
- **移動後のゲートを兼ねる**: このテストは移動をコミットした後に走るので、ソースからの spec 文書の実ファイル読みがパスの追随漏れで壊れていれば、ここで赤になる。**スキップしない**（以前の「直近の実行結果により省略」は廃止。移動というテスト対象への変更が必ず間に挟まるため）。
- **スクリプトが無いリポジトリ**（このスキルを別のリポジトリへ移したとき）: `cargo fmt --all`・`cargo fmt --all -- --check`・`cargo test --workspace --no-fail-fast` を 1 本のコマンドにつないで裏で起動し、License Gate は同じコマンドの末尾に直列でつなぐ。

### ステップ6: 並行レーン（全体テストの待ち時間に行う作業）

全体テストが裏で走っている間に、**テストの結果に左右されず、テストが読むものを変えない作業**を進める。

#### 並行レーンの規則（必須）

- **触ってよいもの（書き込み可）**: `.kiro/steering/*.md`（roadmap・focus・workflow など）・`doc/ROADMAP.md`・`.claude/skills/**`・完了 spec の文書（`.kiro/specs/completed/{feature-name}/` の中の既存ファイル）・スクラッチパッド（PR 文面の下書き）。
- **触ってはいけないもの**:
  - ソース・テスト・ビルド設定（`crates/**`・`tools/**`・`Cargo.toml`・`Cargo.lock`・`deny.toml`・`about.toml`）——テストの検査対象そのもの。
  - `THIRD-PARTY-NOTICES.md`——スクリプトが最後に書く。
  - **テストが実ファイルとして読む文書**（`doc/ukadoc-coverage/**` など）。触る前に `git grep -n "<そのファイルのパス>" -- crates tools` でソースから読まれていないことを確かめる（ヒットがコメント行だけなら可）。
  - **`.kiro/specs/` の直下でのディレクトリの追加・削除・改名**——`crates/ukadoc-survey/tests/consistency/documents_non_vacuity.rs` がこの直下を 2 回数えて突き合わせるので、テストの最中に増減すると偶発的に赤になる。起票（冒頭ステップ）と移動（ステップ3）はテストの起動より前に済ませてある。
- **`cargo` を回さない**（ビルドの取り合いでメモリ不足になり、全体テストが偽の赤になる）。確認は `git grep`・読み取りだけで行う。
- 作業の途中でソースの変更が要ると分かったら、並行レーンでは直さず**控えておく**。全体テストの完了後に直し、全体テストを回し直す（ステップ7-4）。

#### 6-1. ROADMAP更新

ロードマップの正本を確認し、完了したSpecが記載されているか判定する（参照タイミングは `.kiro/steering/focus.md`）。`.kiro/steering/workflow.md` がロードマップの正本（areka では `.kiro/steering/roadmap.md`・`doc/ROADMAP.md` はポインタ stub）を定めていればそれに従う。定めが無ければ `doc/ROADMAP.md` を見る。

**スコープ判定（優先順位）**:
1. `requirements.md` に明示的なROADMAP項目との紐付け記述がある場合
2. 開発者が直接指示した場合
3. ロードマップの仕様テーブルにこの feature-name が含まれる場合
4. 判断に迷う場合は開発者に確認

**スコープ内の場合**:
- **6-1-1. 仕様テーブルの状態列を更新**: 該当行のパスを `completed/{feature-name}` に、状態列を ⚪ 未着手 / 🔵 進行中 → ✅ 完了 に更新。
- **6-1-2. プログレスサマリーを更新**: 「完了済み仕様」を +1、該当すれば「アクティブ仕様(P0)」を −1。
- **6-1-3. フェーズ進捗率を更新**: Phase内の完了状況に応じて進捗バーとパーセンテージを更新（必要な場合）。

**スコープ外の場合**: スキップ。

#### 6-2. スキルドキュメント更新

変更領域に関連するスキルドキュメント（`.claude/skills/` 配下で当該機能を解説するもの）が存在すれば、整合性を確認・更新する。該当がなければスキップ。`.kiro/steering/workflow.md` に「スキルドキュメント更新検討」の指示があればそれに従う。

#### 6-3. ステアリング・ドキュメント更新

当該変更で陳腐化する steering（`.kiro/steering/*.md`）やドキュメントがあれば更新する。`.kiro/steering/workflow.md` に「ドキュメント保守 > 更新チェックリスト」があればそれに従う。並行レーンの規則の「触ってはいけないもの」に当たる文書は、全体テストの完了後（ステップ7）に回す。

#### 6-4. PR 文面の下書き

ステップ8 で使う squash の subject／body と PR の body を、スクラッチパッドのファイルへ下書きする（作り方はステップ8 の「squash メッセージ生成」）。冒頭ステップでその場で解決した件・起票した件もここに載せる。`gh auth status` もここで確かめておく（ステップ8 の PR 可否判定の 3 番目）。

#### 6-5. 待ち方

並行レーンの作業が済んだら、全体テストの完了通知を待つ。**経過をポーリングしない**（完了すると通知が来る）。並行レーンで書いた変更はまだコミットしない（ステップ7-2 でまとめてコミットする）。

### ステップ7: 判定と最終コミット

#### 7-1. 全体テストの判定

- **判定は終了コードと末尾の一覧だけで行う**。ログの途中の `test result:` を拾って合否を決めない。一覧の見出しの「検査したコミット」がステップ5-1 で控えたコミットと一致することを確かめる。
- **緑**: 7-2 へ進む。
- **赤**: 赤の段のログを読んで原因を仕分ける。
  - **ファイルが見つからない系（spec 文書の実ファイル読み）**: ステップ4 の参照パスの追随漏れ。ステップ4-2／4-3 に戻って仕分けをやり直し、修正を追加コミットしてから全体テストを回し直す（7-4）。
  - **`i686 成果物ビルド` が赤**: host32 の e2e も連鎖して赤になる（成果物が無いと panic する設計）。まず i686 のビルドを直す。
  - **rustc の `memory allocation ... failed`・壊れた rmeta の `E0463`／`E0786`**: テストの失敗ではなくメモリ不足。並行レーンで `cargo` を回していないか確かめ、回し直す。
  - **`cargo deny check` が赤**: 下の「License Gate 失敗時」。
  - **それ以外（コードの赤）**: ワークフローを中断し開発者に報告する。コミットはすべてローカルなので、そのまま直して回し直せる。**赤のままステップ8（PR 作成・マージ）へ進まない。**

#### 7-2. 最終コミット

全体テストが書いた差分（`-Format` の整形・`THIRD-PARTY-NOTICES.md`）と、並行レーンで書いた変更（ROADMAP・steering・スキル文書）をコミットする。

```powershell
git status --short   # 何が変わったかを先に見る
git add -A
git commit -m "chore({feature-name}): 完了の文書更新（ROADMAP・steering・整形）"
```

#### 7-3. 検査後の変更がテストの外にあることの確認（必須）

ステップ5-1 で控えたコミットから今までの変更が、**全体テストの結果を変えないもの**だけであることを機械で確かめる。

```powershell
git diff --name-only <5-1 で控えたコミット> HEAD
```

- 許される変更: 並行レーンで触ってよいもの（`.kiro/steering/**`・`doc/ROADMAP.md`・`.claude/skills/**`・`.kiro/specs/completed/{feature-name}/**`）・`THIRD-PARTY-NOTICES.md`・`-Format` の整形による `*.rs` の変更（整形はテストの前の段で行われ、その後のテストが整形後のコードを検査している）。
- **それ以外が 1 つでもあれば** 7-4 へ（全体テストの回し直し）。

#### 7-4. 回し直し（必要なときだけ）

7-1 で赤だったとき、または 7-3 でテストの外に出ない変更が見つかったときは、修正をコミットしてから全体テストを回し直す（依存が変わっていなければ `-License` は不要・`-Format` は付ける）。回し直しの間も並行レーンの規則を守る。緑になるまでステップ8 へ進まない。

### ステップ8: リモート同期（PR ベース）

> **手順実体**: リモート同期は **PR（Pull Request）ベース**であり、本セクションがその手順実体である。`.kiro/steering/workflow.md` に同等のブランチ戦略が定義されていればそれを優先する。

> **前提**: 本ステップは `origin`/`main` をハードコードせず、ステップ0で解決した `{remote}` / `{default-branch}` を用いる。フィーチャーブランチ／ワークツリーは Claude Code（ハーネス）が供給しており、このスキルは自前でブランチ／ワークツリーを作成・削除しない。1つの feature = 1つのブランチ = 1つの PR とし、完了時に1回だけ PR を作成して squash マージする。**`{default-branch}` への直接 push は一切行わない。**

確認不要。現在のブランチを判定し、以下を中断なく実行する。

```powershell
$branch = git rev-parse --abbrev-ref HEAD   # 現在の作業ブランチ（ハーネス供給）
```

#### PR 可否判定

以下を**すべて満たす**ときのみ PR を作成・マージする（PR 可）:

1. 現在ブランチが `{default-branch}` 以外（非デフォルトブランチ）。
2. ステップ0で解決した `{remote}` が none でない（リモートあり）。
3. `gh` が認証済み（`gh auth status` が成功。ステップ6-4 で確認済み）。

いずれかが欠ける場合は **PR 不可** とし、下記「フォールバック（PR 不可時）」へ進む。

#### PR 可: push → PR 作成 → squash マージ → リモートブランチ削除

```powershell
# 1. 現在ブランチを push して PR を作成（base = {default-branch}, head = 現在ブランチ）
git push -u {remote} $branch
gh pr create --base {default-branch} --head $branch --title "<subject>" --body-file <6-4 の PR body の下書き>

# 2. squash マージ（--squash 固定、--delete-branch でリモートブランチを API 削除）
#    --subject / --body は下記「squash メッセージ生成」に従って供給する（6-4 の下書き）
gh pr merge --squash --delete-branch --subject "<subject>" --body-file <6-4 の squash body の下書き>
```

- **マージ成否はマージ API の結果のみで判定する。** `gh pr merge` の成否がマージ成否であり、それ以外の警告でマージ成功を覆さない。
- **リモートブランチ削除**: `gh pr merge --delete-branch` が **PR マージ成功後に** API でリモート feature ブランチを削除する。
- **ローカル後始末警告は非致命**: `--delete-branch` のローカル削除試行は、カレントワークツリーでブランチがチェックアウト中のため**ブロックされ警告を出す**ことがある。これは**非致命**でありマージ成功（API 結果）を覆さない。リモートブランチは API により削除済みである。
- **ローカルブランチ／ワークツリーの後始末はハーネスへ委譲**: このスキルは自分のワークツリー／カレントブランチを削除しない（構造的に不可）。ローカルブランチ・ワークツリーの teardown はハーネスがセッション/タスク境界で実施する。

**squash メッセージ生成**（`gh pr merge --squash` の `--subject` / `--body`・ステップ6-4 で下書きする）:
- 固定文言にせず、**分岐点以降のコミット履歴を要約**して作成する。
- 手順:
  1. `git log --no-merges --pretty=format:"%h %s%n%b" {default-branch}..HEAD`（= `merge-base..HEAD`）で分岐点以降の全コミットを取得
  2. 対象 spec の `requirements.md` / `design.md` のタイトル・概要も参照し意図を補強
  3. 以下の形へ再構成:
     - **subject**（`--subject`）: `<type>({feature-name}): <機能全体を1文で表す要約>`
     - **body**（`--body`）: 主な開発仕様・変更内容を箇条書き（3〜7項目目安）。関連コミットは統合し、`fixup`/typo/WIP 等の些末な履歴は集約・省略。個々のコミット羅列ではなく「何を・なぜ作ったか」の開発単位で再構成する。
- 下書きの時点（ステップ6）ではステップ7 のコミットがまだ無い。ステップ7 で履歴が増えたら（整形・回し直しの修正）、PR を作る前に下書きへ 1 行足す。

#### フォールバック（PR 不可時）

現在ブランチが `{default-branch}` である / `{remote}` が none（リモートなし・オフライン）/ `gh` 未認証 のいずれかの場合:

- **警告を出力**し、PR 作成・push を**スキップ**する。
- ローカルコミットは**そのまま保持**して継続する。
- **`{default-branch}` への直接 push は一切行わない。**

#### 中断条件

PR の**作成またはマージ（API）が失敗**した場合（コンフリクト / mergeable でない / 権限不足等）は、**ブランチを削除せず**処理を中断し開発者へ報告する（復旧可能性を確保するため）。中断するのは**マージ API が失敗したとき**のみであり、`--delete-branch` のローカル削除警告（非致命）とは区別する。

---

## 完了チェックリスト

```
- [ ] 冒頭ステップ: 実装中の未解決問題のうち未起票のものを棚卸し済み——軽微・直ちに実施可能なものはその場で解決（バグはテスト付き・1 件 1 コミット・Implementation Notes に記録）、残りは `/kiro-discovery` で起票済み（解決件数・起票件数をそれぞれ記録。0 件なら「0 件」）。起票は全体テストの起動より前
- [ ] ステップ0で `{remote}` / `{default-branch}` を決定的解決済み・既定ブランチとの衝突なし（衝突すればテスト前に取り込み済み）
- [ ] DoD 静的ゲート通過（Spec Gate。workflow.md が存在すればテストを伴わない追加ゲートも）
- [ ] 未コミットファイルをコミット済み（ステップ2）
- [ ] completedフォルダへ移動済み・spec.json の phase を "completed" に更新済み + updated_at 更新（ステップ3）※繰り返し仕様はスキップ
- [ ] 参照パス更新済み（ステップ4）※繰り返し仕様はスキップ
      - [ ] spec 文書の参照を検索（4-1）
      - [ ] ソース全域（`crates/**/*.rs` 等）を `git grep "{feature-name}"` で検索（4-2(a)。検出ゼロでも実施を記録）
      - [ ] 検出行を「コメント参照（放置可）／実ファイル読み（要更新）」に仕分け済み（4-3）
- [ ] アーカイブのコミット済み・起動時のコミットを控えた（ステップ5-1）
- [ ] 全体テスト 1 回: `pwsh -NoProfile -File tools/test-all.ps1 -Format -License` を裏で起動し、終了コード 0・一覧の「検査したコミット」が 5-1 と一致（ステップ5-2・7-1。Format / Test / License / 移動後の各ゲートを兼ねる。スキップ不可。無いリポジトリは `cargo fmt` + `cargo test --workspace --no-fail-fast` + License の 2 コマンド）
- [ ] License Gate（上の `-License`）: `cargo deny check` 成功 + 謝辞再生成（deny.toml/about.toml があるリポジトリ。無ければ「設定不在により省略」）
- [ ] 並行レーン（ステップ6）: ROADMAP・スキル文書・steering の更新と PR 文面の下書きを全体テストの待ち時間に実施。触ったのは許可範囲だけ・`cargo` を回していない・`.kiro/specs` 直下のディレクトリを増減していない
- [ ] 最終コミット済み（ステップ7-2）
- [ ] 検査後の変更がテストの外にあることを `git diff --name-only <5-1> HEAD` で確認済み（ステップ7-3。外に出る変更があれば 7-4 で回し直して緑）
- [ ] リモート同期完了（ステップ8、PR ベース。解決した `{remote}`/`{default-branch}` を使用）
      - PR 可（非デフォルトブランチ かつ `{remote}` あり かつ `gh` 認証あり）: push → `gh pr create --base {default-branch} --head <current>` → `gh pr merge --squash --delete-branch --subject … --body …`（メッセージは `merge-base..HEAD` 履歴を要約）。マージ成否は API 結果のみで判定し、`--delete-branch` のローカル削除警告は非致命として継続。リモートブランチは API 削除、ローカルブランチ／ワークツリーはハーネス teardown へ委譲
      - PR 不可（`{default-branch}` 上 / `{remote}` none / `gh` 未認証）: 警告して PR・push スキップ、ローカルコミット保持（`{default-branch}` への直接 push なし）
      - PR 作成／マージ（API）失敗: ブランチを残し中断・報告
```

---

## エラー回避

### VSCode変更確定問題
- **症状**: 移動したファイルが元の場所に復活する
- **対策**: spec.jsonは必ずステップ3-2（移動後）で編集。移動前に編集しない

### 参照パス更新漏れ
- **症状**: 後続specが旧パスで参照しファイルが見つからない
- **対策**: ステップ4-1 で `Select-String` による網羅的検索を実施

#### ソースコードからの spec 文書読み込みが壊れる（静かに赤くなる）
- **症状**: アーカイブ移動の後、コードが `std::fs::read_to_string` / `include_str!` などで spec 文書を実ファイルとして読んでいるテストが「ファイルが見つからない」で失敗する
- **原因（構造的に見えない）**: spec 文書側の検索（ステップ4-1）はソースを見ないので、この参照は検索網に一度も掛からない。以前はテストを移動より前にも回していたので、「移動前は全緑」と「移動で赤化」が同じ手順の中で両立していた
- **実例**: `crates/areka/src/placement/transition_signoff_procedure_tests.rs` の定数 `PROCEDURE_RELATIVE_PATH` が `.kiro/specs/areka-P0-dpi-transition-atomicity/signoff-procedure.md` を読んでいた。PR #114 の完了ワークフローで当該 spec が `completed/` へ移動したがパスが追随せず、`main` 上で 5 件のテストが赤のまま放置された（別作業で偶然踏むまで誰も気づかなかった）
- **対策**: ステップ4-2 のソース全域 grep と 4-3 の仕分けを必ず実施し、**全体テストは移動をコミットした後に回す**（ステップ5）。仕分けの要点は「**コメント参照は放置可・実ファイル読みだけがビルドを壊す**」

### 並行レーンで起きること
- **症状**: 全体テストの `ukadoc-survey` の整合テスト（「一覧が .kiro/specs の直下を独立に数え直した結果と食い違う」）が、コードを変えていないのに赤になる
- **原因**: テストの最中に `.kiro/specs/` の直下へ spec ディレクトリを足した・消した（起票や移動を並行レーンでやった）。テストは直下を 2 回数えて突き合わせる
- **対策**: 起票（冒頭ステップ）と移動（ステップ3）は全体テストの起動より前に済ませる。並行レーンでは直下のディレクトリを増減しない。踏んだら回し直す（ステップ7-4）
- **症状**: 全体テストがメモリ不足（`memory allocation ... failed`・`E0463`／`E0786`）で落ちる
- **原因**: 並行レーンで `cargo` を回した
- **対策**: 並行レーンでは `cargo` を回さない。回し直す

### コミット漏れ
- **症状**: pushしたが変更が反映されていない
- **対策**: 各コミット前に `git status --short` で確認

### テスト失敗時
- **症状**: `tools/test-all.ps1`（または `cargo test --workspace`）が失敗
- **切り分け**: ステップ7-1 の仕分けに従う
- **対策**: コードの赤ならワークフローを中断し開発者に報告。修正後に回し直す（ステップ7-4）

### License Gate 失敗時
- **症状**: `cargo deny check` が失敗（許可外ライセンス・強コピーレフト混入を検出）
- **対策**: ワークフローを中断し開発者に報告。混入 crate と経路を提示し、依存の差し替え／除外か allowlist 妥当性の再検討を仰ぐ。**安易に allow を広げて通さない**（MIT 配布の前提が崩れる）
- **症状**: `cargo deny` / `cargo about` が未導入（`no such command`）
- **対策**: `cargo install cargo-deny --locked` / `cargo install cargo-about --features cli --locked` で導入後に再実行
- **症状**: `cargo about generate` が `failed to satisfy license requirements` で失敗
- **対策**: 出荷対象外の dev-dependency が混入していないか確認（`about.toml` の `ignore-dev-dependencies = true`）。真に出荷される新規ライセンスなら permissive 性を確認のうえ `about.toml` の `accepted` と `deny.toml` の `allow` を揃えて追記

### リモート同期関連（ステップ8 PR ベース）

#### PR 作成失敗
- **症状**: `gh pr create` が失敗（既存 PR との衝突 / push 権限不足 / ネットワーク等）
- **対策**: **ブランチを削除せず**中断して開発者へ報告。既存 PR がある場合はその PR を確認して再マージを検討

#### マージ不可（mergeable でない / API 失敗）
- **症状**: `gh pr merge --squash` が失敗（コンフリクトで mergeable=false / 必須チェック未通過 / 権限不足等）
- **対策**: **ブランチを削除せず**中断して開発者へ報告。コンフリクトは GitHub 上または別途解決のうえ再実行。中断判定は**マージ API の結果のみ**で行い、`--delete-branch` のローカル削除警告とは混同しない

#### gh 未認証 / リモートなし（PR 不可）
- **症状**: `gh auth status` が失敗、または `{remote}` が none
- **対策**: 警告を出力し PR・push をスキップ。ローカルコミットは保持して継続する。**`{default-branch}` への直接 push は行わない**

#### `{default-branch}` 上で承認された
- **症状**: 現在ブランチが `{default-branch}`（PR の head に使えない）
- **対策**: 警告を出力し PR・push をスキップ、ローカルコミット保持。通常はハーネス供給の非デフォルトブランチ上で完了する想定

#### `--delete-branch` のローカル削除警告
- **症状**: `gh pr merge --delete-branch` がローカルブランチ削除を試みてブロックされ警告を出す（カレントワークツリーでチェックアウト中のため）
- **対策**: **非致命として無視し継続**。リモートブランチは API で削除済み。ローカルブランチ／ワークツリーの後始末はハーネスのワークツリー teardown に委ねる（このスキルは自分のワークツリーを削除しない／できない）
