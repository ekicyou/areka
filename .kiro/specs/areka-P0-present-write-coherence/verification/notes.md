# 残余検証の記録: areka-P0-present-write-coherence

> **性格**: 本仕様は 2026-08-27 の要件討議による**見送り＋登記**の裁定に従い、実行時の挙動を一切変更しない（是正コード 0 行）。本文書は設計 C4 が定めた残余検証 V1〜V5 の実行結果を残す記録であり、**合格の宣言ではない**。
> **書き方の規律**: 本文書に載せる file:line は**すべてその場で現物を読んで得た実測値**である。設計文書・調査文書に書かれた値を写して埋めてはならない（この仕様群は陳腐化した file:line に通算 8 度足を取られている）。
> **許可集合の内側**: 本文書は `.kiro/specs/areka-P0-present-write-coherence/` 配下にあり、V2 の許可集合の内側である。**V2（task 2.4）の差分採取より前にコミットして作業ツリーを確定させること**。

## 検証の基準点

| 項目 | 値 |
|---|---|
| 実測日 | 2026-08-27 |
| 対象 HEAD | `18db890f`（`feat(areka-P0-present-write-coherence): ワークスペース全体テストの前提を整える (task 1)`） |
| 既定枝 | `origin/main` |
| 調査文書の実測時 HEAD | `a6d27c73`（`research.md` 冒頭） |
| `a6d27c73`→`18db890f` の `crates/` 差分 | **0 ファイル**（`git diff --stat a6d27c73 HEAD -- crates/` が空） |

---

## V1: ワークスペース全体テスト（要件 7.7）

実施日 2026-08-27。対象 HEAD `2b124d33`（`feat(areka-P0-present-write-coherence): 上流アンカーの実測再確認と検証記録の起票 (task 2.1)`）。実行時点の作業ツリー差分は `git status --porcelain` で `M vendors/pasta` の 1 行のみ——これは本仕様の着手前からの汚れであり、本仕様は触っていない（V4-c と同じ扱い）。

### 実行

| 項目 | 値 |
|---|---|
| コマンド | `cargo test --workspace` |
| シェル | **PowerShell 7**（`Start-Process` 経由で stdout／stderr を別ファイルへ分離採取）。Git Bash は使っていない——本リポジトリは Git Bash の `link.exe` が MSVC のリンカを覆う既知の罠を持つため |
| 作業ディレクトリ | ワークツリー直下（`…\worktrees\areka-p0-scope-chain-gap-004c39`） |
| ツールチェイン | `cargo 1.98.0 (797e8a9bc 2026-08-05)` ／ `rustc 1.98.0 (88d9e12ae 2026-08-18)` |
| 所要 | 456.6 秒（ビルド込み） |
| **終了コード** | **0** |

> **終了コードの出所（後から再導出できない）**: 上の `0` は実行時のプロセスオブジェクトから読んだ値である。採取したログは stdout ／ stderr の中身だけで**終了コードを含まない**——後から `ws-test.out.log` ／ `ws-test.err.log` を読み直しても、この `0` は再導出できない。ログに見当たらないことをもって記録の誤りと早合点しないこと。ログ側から裏が取れるのは下の「結果」節の数量である。

### 前提の確認（実行直前に、実行と同一のシェルで採取）

**⑴ i686 host-32 成果物**（task 1 で整備済み。再ビルドはしていない）

| ファイル | サイズ | PE Machine | 更新時刻 |
|---|---|---|---|
| `target\i686-pc-windows-msvc\debug\shiori-host32-helper.exe` | 271,872 B | `0x014C`（i386） | 2026/08/27 22:04:49 |
| `target\i686-pc-windows-msvc\debug\shiori.dll` | 160,768 B | `0x014C`（i386） | 2026/08/27 22:04:48 |

**⑵ 環境変数の上書きが無いこと**（**この確認を省くと結果が読めない**）

```
HOST32_HELPER_EXE=[]
HOST32_TESTDLL_DLL=[]
set-count=0        # Get-ChildItem Env: の HOST32* が 0 件
```

`HOST32_HELPER_EXE`／`HOST32_TESTDLL_DLL` は解決器（`crates/shiori-host32-host/tests/lifecycle_cyclic_e2e.rs:96-107`・`:137-148`、`crates/shiori-host32-helper/src/shiori_proxy.rs:415-425`）で target ディレクトリ探索より**優先**され、しかも解決に失敗したときは既定へ落ちずに **panic する**。両方とも**未設定**であることを実行と同じシェルで確認した上で走らせた——設定されていれば上表の成果物ではなく別のパスが測られる。

### 結果

| 項目 | 値 |
|---|---|
| `test result:` 行（＝テストターゲット数） | **95**（実行バイナリ **74** ＋ Doc-tests **21**） |
| うち `test result: ok` | **95**（**例外なし**。`ok` 以外の `test result:` 行は 0 行） |
| 通過 | **5,894** |
| 失敗 | **0** |
| ignored | **36**（内訳の全て＝26／3／2／2／1／1／1 の 7 ターゲット） |
| measured ／ filtered out | **0** ／ **0** |

### 空振りでないことの確認（終了コード 0 は何も走らせなくても出る）

- `running N tests` 行が **95 本**あり、`test result:` 行の 95 と一致する。うち `running 0 tests` は 19 本（テストを持たない bin ターゲット・空の Doc-tests）で、残り 76 本が実際にテストを実行している。
- **i686 前提を要する e2e が現に走っている**——`tests\lifecycle_cyclic_e2e.rs`・`tests\lifecycle_kill_e2e.rs`・`tests\shiori_load_e2e.rs`・`tests\shiori_request_e2e.rs` の 4 ターゲット。前提⑴が欠けていればここが赤になる（この一文が当てはまるのは、この 4 件に対してだけである）。
  - なお `shiori_lifecycle_e2e_tests::*` を**この 4 件に混ぜてはならない**。当該モジュールの宣言は `crates/areka/src/main.rs:94`（`#[cfg(test)] mod shiori_lifecycle_e2e_tests;`）であり、`shiori-host32-host` ではなく **`areka` クレートの中**にある（実行ターゲットも `unittests src\main.rs`）。中身も in-process のモック駆動で、`grep -cEi "host32|HOST32|helper|i686|host-32" crates/areka/src/shiori_lifecycle_e2e_tests.rs` は **0** を返す——i686 成果物への依存を一切持たないため、**前提⑴が欠けてもここは赤にならない**。i686 前提の証跡としては数えられない。
- GPU 実描画／readback 系のテスト名も `... ok` の側に並んでおり、GPU 環境も成立している。採取コマンドと実測値は次のとおり（採取した stdout ログ `ws-test.out.log` に対して実行）。
  - `grep -E "\.\.\. ok$" ws-test.out.log | grep -cEi "readback|d2d|swap_chain"` → **35**
  - 同じ語集合を `... FAILED` ／ `... ignored` の側で数えると **0**（`grep -E "\.\.\. (FAILED|ignored)" ws-test.out.log | grep -cEi "readback|d2d|swap_chain"`）。
  - この **35** は**テスト名に含まれる語による近似**であって、GPU 経路を通るテストの全数ではない。`gpu`・`offscreen` も語としては当たるが、当たるのは中身が GPU 描画でないもの（`run_attach_phase_without_gpu_does_not_attach_or_consume_assets` 1 件と `an_offscreen_projection_input_*` 2 件＝窓の画面外投影の話）だけなので、語集合から外してある。語集合を変えれば数は変わる——**数だけを引用せず、必ず上のコマンドと一緒に読むこと。**
- 採取ログの実体: stdout 520,823 B ／ stderr 9,764 B（スクラッチパッドに保存。**仕様ディレクトリの外**に置いた——本仕様の差分集合を汚さないため）。

### 付随して出た出力（いずれも赤ではない）

- ビルド警告 1 件（`shiori4-testdll` のリンカ stdout がインポートライブラリ生成を報告するもの）。本仕様と無関係のビルド既存事項。
- host-32 e2e の stderr に `[helper] LOAD 失敗（観測・ack[0]）: LoadReturnedFalse` と `… LoadLibraryFailed(HRESULT(0x8007007E))` の 2 行。これは**失敗経路を意図的に踏むテストが出す観測ログ**であり、当該ターゲットは `test result: ok` を返している。

### 判定と、その射程（**ここを混ぜてはならない**）

- **要件 7.7（ワークスペース全体のテストが通る状態で完了する）は充足した。** 95 ターゲットすべてが `test result: ok`・失敗 0・終了コード 0。
- **この緑は要件 2.1 の充足ではない。** 本仕様は 2026-08-27 の裁定により**是正コード 0 行**であり、絵と窓が同じ提示フレームで揃うかという合否量（`visualize_to_write_us`・`flush_total_us`）は**未達のまま登記されている**（requirements.md「未達の登記」節）。上の緑は「本仕様が何も壊していない」ことの証拠であって、未達が解消した証拠ではない。**完了報告で合否量と同じ息で並べてはならない。**
- 併せて、この緑を「十分性の証拠」として読まないこと。本仕様群では全緑が実在の欠陥を素通りさせた事例が二度ある。ここに記すのは**数量と、その数量が何を検査していないか**であり、それ以上ではない。

---

## V2: コード非接触の証跡（要件 6.1-6.7・8.3）

実施日 2026-08-27。

> **採取の順序と、本節自身の扱い（先に読むこと）**: 本節の採取はすべて、task 2.1〜2.3 の仕様文書変更を**コミットして作業ツリーを確定させた後**に実行した（採取時の `git status --porcelain` は ` M vendors/pasta` の 1 行のみ）。ただし**本節を書き込む行為そのもの**が `verification/notes.md` に未コミット差分を 1 件作る——下に記す値は書き込みより**前**の状態を写したものであり、本節の記載後に `git status --porcelain` を実行すれば `M .kiro/specs/areka-P0-present-write-coherence/verification/notes.md` が 1 行増える。これは**許可集合の内側**の変更であり、本タスクのコミットで解消する。数え直した読者が「1 行のはずが 2 行ある」と読み違えないための注記である。
> **採取の規律（V4-c と同じ）**: git は**実在しないパスを渡されても空出力・終了コード 0 を返す**。ゆえに本節では、差分を採るすべての pathspec について**先に現物が解決することを確かめ**、さらに**同じ検査が赤を出せること**を対照で示してから「差分なし」を記録した。確かめずに採った緑は、何も検査していない緑と見分けがつかない。

### V2-0: 採取の基準点

| 項目 | 値 | 採取コマンド |
|---|---|---|
| 対象 HEAD | **`946d0aa0`**（`feat(areka-P0-present-write-coherence): steering 編成表の裁定反映を確認 (task 2.3)`） | `git rev-parse HEAD` ／ `git log -1 --oneline` |
| 既定枝 `origin/main` | **`a6d27c73`** | `git rev-parse origin/main` |
| 枝名 | `claude/areka-p0-present-write-coherence-8308e6` | `git rev-parse --abbrev-ref HEAD` |
| 本枝が `origin/main` に加えるコミット | **13 本** | `git log --oneline origin/main..HEAD \| wc -l` |
| 採取時の作業ツリー差分 | **1 行**（` M vendors/pasta`） | `git status --porcelain \| wc -l` |

- `origin/main` の `a6d27c73` は、調査文書 `research.md` 冒頭が実測時 HEAD として記す sha と**同一**である（本文書冒頭「検証の基準点」表と一致）。すなわち本枝の分岐点以降、既定枝は動いていない。

### V2-1: 差分ファイル一覧（全行）

```
$ git diff --name-only origin/main...HEAD
.kiro/specs/areka-P0-present-write-coherence/design-validation.md
.kiro/specs/areka-P0-present-write-coherence/design.md
.kiro/specs/areka-P0-present-write-coherence/requirements.md
.kiro/specs/areka-P0-present-write-coherence/research.md
.kiro/specs/areka-P0-present-write-coherence/spec.json
.kiro/specs/areka-P0-present-write-coherence/tasks.md
.kiro/specs/areka-P0-present-write-coherence/verification/notes.md
.kiro/steering/roadmap.md

$ git diff --name-only origin/main...HEAD | wc -l
8
```

**全 8 行**である。内訳は本仕様の spec ディレクトリ配下 **7 行**（`design-validation.md`・`design.md`・`requirements.md`・`research.md`・`spec.json`・`tasks.md`・`verification/notes.md`）と `.kiro/steering/roadmap.md` **1 行**。**`crates/` 配下・`Cargo.toml`・`vendors/` は 1 行も現れない。**

### V2-2: 判定⑴——許可集合外 **0 件**

許可集合の定義は design.md `Boundary Commitments` → `Out of Boundary` 第 1 項の逐語である——**`.kiro/specs/areka-P0-present-write-coherence/` 配下＋`.kiro/steering/roadmap.md`**（「この 2 つが許可集合の唯一の定義であり、V2 がこれを機械で検査する」）。他の節・他の文書の記述を許可集合の定義として持ち込んでいない。

```
$ git diff --name-only origin/main...HEAD \
    | grep -v -E '^\.kiro/specs/areka-P0-present-write-coherence/|^\.kiro/steering/roadmap\.md$'
（出力なし）

$ git diff --name-only origin/main...HEAD \
    | grep -v -E '^\.kiro/specs/areka-P0-present-write-coherence/|^\.kiro/steering/roadmap\.md$' | wc -l
0
```

- **判定: 許可集合外 0 件。**（8 行すべてが許可集合の内側。）
- **終了コードで読まないこと**: `grep -v` は 1 行も残らないとき**終了コード 1** を返す。ここでの空は `wc -l` の **`0`** で取っている——終了コードを合否に使うと、この検査は「正常な緑」を「失敗」と読む。

### V2-3: 判定⑵——接触禁止集合 **0 件**（7 項目・項目別）

#### ⒜ pathspec の実在確認（差分採取の直前に実行）

```
$ for p in crates/areka-emo-present/src/presenter/show.rs \
           crates/areka-emo-present/src/mount.rs \
           crates/wintf/src/ecs/window/command.rs \
           crates/wintf/src/runtime/tick_bridge.rs \
           crates/wintf/src/ecs/window/transition_diag.rs ; do
    [ -f "$p" ] && echo "EXISTS  $p" || echo "MISSING $p" ; done
EXISTS  crates/areka-emo-present/src/presenter/show.rs
EXISTS  crates/areka-emo-present/src/mount.rs
EXISTS  crates/wintf/src/ecs/window/command.rs
EXISTS  crates/wintf/src/runtime/tick_bridge.rs
EXISTS  crates/wintf/src/ecs/window/transition_diag.rs

$ ls -1 crates/areka/src/placement/transition_judge*.rs
crates/areka/src/placement/transition_judge.rs
crates/areka/src/placement/transition_judge_coalesced_follow_tests.rs
crates/areka/src/placement/transition_judge_frame_tests.rs
crates/areka/src/placement/transition_judge_negative_tests.rs
crates/areka/src/placement/transition_judge_reobservation_tests.rs
crates/areka/src/placement/transition_judge_test_support.rs
crates/areka/src/placement/transition_judge_tests.rs
crates/areka/src/placement/transition_judge_verdict.rs
crates/areka/src/placement/transition_judge_verdict_tests.rs
$ ls -1 crates/areka/src/placement/transition_judge*.rs | wc -l
9

$ ls -1 Cargo.toml crates/*/Cargo.toml | wc -l
25
$ find crates -name Cargo.toml -not -path "*/target/*" | wc -l
24
$ find crates -name Cargo.toml -not -path "*/target/*" | grep -v "^crates/[^/]*/Cargo.toml$"
（出力なし）
```

- **5 ファイルとも EXISTS。** glob 2 件も実体へ展開した——`transition_judge*.rs` が **9 ファイル**、`Cargo.toml` 群が **25 ファイル**（ルート 1 ＋ `crates/*/Cargo.toml` 24）。
- 設計の字面は `crates/**/Cargo.toml`（再帰）だが、`find` の結果は **24 件すべてが 1 階層目の `crates/<crate 名>/Cargo.toml`** であり、より深い階層の `Cargo.toml` は存在しない（上の `grep -v` が空）。したがって `crates/*/Cargo.toml` と `crates/**/Cargo.toml` は**同じ 24 ファイルへ展開する**——一段浅い glob を使ったことで取りこぼした対象は無い。
- glob の 2 項目は、以下の差分採取でも**展開後の 9 ／ 25 ファイルを明示的に列挙して**渡した。シェルの glob 展開に頼ると、展開が 0 件になったとき pathspec が消えて「差分なし」に見える。

#### ⒝ 項目別の差分（各項目を**単独の pathspec で**採取）

採取コマンド（各項目 `<item>` について）:

```
git diff --name-only origin/main...HEAD -- <item> | wc -l
git diff --stat      origin/main...HEAD -- <item>
```

| # | 接触禁止集合の項目 | 実在 | `--name-only \| wc -l` | `--stat` | 判定 |
|---|---|---|---|---|---|
| 1 | `crates/areka-emo-present/src/presenter/show.rs` | EXISTS | **0** | 出力なし | **差分なし** |
| 2 | `crates/areka-emo-present/src/mount.rs` | EXISTS | **0** | 出力なし | **差分なし** |
| 3 | `crates/wintf/src/ecs/window/command.rs` | EXISTS | **0** | 出力なし | **差分なし** |
| 4 | `crates/wintf/src/runtime/tick_bridge.rs` | EXISTS | **0** | 出力なし | **差分なし** |
| 5 | `crates/wintf/src/ecs/window/transition_diag.rs` | EXISTS | **0** | 出力なし | **差分なし** |
| 6 | `crates/areka/src/placement/transition_judge*.rs`（**9 ファイルを列挙して渡す**） | 9 ファイルへ展開 | **0** | 出力なし | **差分なし** |
| 7 | `Cargo.toml` ＋ `crates/*/Cargo.toml`（**25 ファイルを列挙して渡す**） | 25 ファイルへ展開 | **0** | 出力なし | **差分なし** |

- 項目 2 は **`crates/areka-emo-present/src/mount.rs`**（`src/` 直下）である。`crates/areka-emo-present/src/presenter/mount.rs` は**実在しない**——task 2.1 以前の設計文書はこの実在しないパスを指しており、そのままでは項目 2 の検査が空振りしていた（V4-e の是正 1 件）。本節はその是正後のパスで採取している。

#### ⒞ 総計（項目別とは別建ての、包含関係にある検査）

```
$ git diff --name-only origin/main...HEAD -- crates/ Cargo.toml | wc -l
0
```

**0 行。** 接触禁止集合の 7 項目はすべて `crates/` 配下かルート `Cargo.toml` に属するので、この 0 は項目別 7 件の 0 を包含する。**ただし総計だけでは項目別の証跡にならない**（1 項目が空振りしても総計は 0 のままである）ため、⒝ の項目別採取を正本とし、⒞ は整合の確認として置く。

### V2-4: 対照——**この検査が赤を出せること**の確認

「差分なし」を証拠として使うには、同じコマンドが差分のある対象に対して**赤（非空）を返す**ことを示さねばならない。

**⑴ 陽性対照**（差分のある許可集合内のファイルへ同じコマンドを当てる）:

```
$ git diff --stat origin/main...HEAD -- .kiro/specs/areka-P0-present-write-coherence/design.md
 .../areka-P0-present-write-coherence/design.md     | 322 +++++++++++++++++++++
 1 file changed, 322 insertions(+)

$ git diff --name-only origin/main...HEAD -- .kiro/specs/areka-P0-present-write-coherence/design.md | wc -l
1
```

**非空を返した。** ゆえに V2-3 の「出力なし・0 行」は、検査が働いた上での空である。

**⑵ 空振り対照**（実在しないパスを渡すと何が起きるかの実演）:

```
$ git diff --stat origin/main...HEAD -- crates/areka-emo-present/src/presenter/mount.rs
（出力なし）
$ echo $?
0
$ ls crates/areka-emo-present/src/presenter/mount.rs
ls: cannot access 'crates/areka-emo-present/src/presenter/mount.rs': No such file or directory
```

**実在しないパスに対して git は空出力・終了コード 0 を返す。** V2-3 ⒜ の実在確認を省いていれば、この出力は「差分なし」と見分けがつかなかった。⑴ と ⑵ を並べて初めて、上表の緑が読める。

### V2-5: 判定⑶——想定外の作業ツリー差分 **0 件**と、`vendors/pasta` の扱い

**⑴ 作業ツリー**

```
$ git status --porcelain
 M vendors/pasta

$ git status --porcelain | wc -l
1
```

**1 行のみ**であり、それは `vendors/pasta`（サブモジュールのポインタ）である。これは**本仕様の着手前から動いている汚れ**であり、本仕様は触っていない（V1・V4-c と同じ扱い）。**想定外の作業ツリー差分は 0 件。**

**⑵ `vendors/pasta` が本仕様のどのコミットにも含まれないこと**

```
$ git log --name-only origin/main..HEAD -- vendors/pasta
（出力なし）

$ git log --name-only --pretty=format:"" origin/main..HEAD | grep -c "vendors"
0

$ git log --name-only --pretty=format:"--- %h %s" origin/main..HEAD \
    | grep -v '^---' | grep -v '^$' | sort -u
.kiro/specs/areka-P0-present-write-coherence/design-validation.md
.kiro/specs/areka-P0-present-write-coherence/design.md
.kiro/specs/areka-P0-present-write-coherence/requirements.md
.kiro/specs/areka-P0-present-write-coherence/research.md
.kiro/specs/areka-P0-present-write-coherence/spec.json
.kiro/specs/areka-P0-present-write-coherence/tasks.md
.kiro/specs/areka-P0-present-write-coherence/verification/notes.md
.kiro/steering/roadmap.md
```

3 本目のコマンドは **13 コミットそれぞれの変更ファイルを 1 本残らず集めて重複を除いた集合**であり、**8 パス**——V2-1 の差分一覧と完全に一致する。すなわち `vendors/pasta` は**13 コミットのいずれにも現れない**（`grep -c "vendors"` が **0**）。差分一覧に現れないだけでなく、**中間コミットで一度触れて後で戻した、という経路も存在しない**ことがここで確定する。

**⑶ 本枝が加える 13 コミット**（採取＝`git log --oneline origin/main..HEAD`）

```
946d0aa0 feat(areka-P0-present-write-coherence): steering 編成表の裁定反映を確認 (task 2.3)
957358ea feat(areka-P0-present-write-coherence): ワークスペース全体テストの実行と結果の記録 (task 2.2)
2b124d33 feat(areka-P0-present-write-coherence): 上流アンカーの実測再確認と検証記録の起票 (task 2.1)
18db890f feat(areka-P0-present-write-coherence): ワークスペース全体テストの前提を整える (task 1)
ce4204a3 docs(areka-P0-present-write-coherence): generate tasks (tasks.md)
bd9f782f docs(areka-P0-present-write-coherence): fix obvious issues in design
8aaee11f docs(areka-P0-present-write-coherence): add design validation report
20dd6aef docs(areka-P0-present-write-coherence): generate design (design.md)
3bb48834 docs(areka-P0-present-write-coherence): resolve discussion #2 - defer remediation, register shortfall
f2aedc3e docs(areka-P0-present-write-coherence): resolve discussion #1 - B-3 excluded as major restructuring
f76460bd docs(areka-P0-present-write-coherence): fix obvious issues in requirements
4131e4fb docs(areka-P0-present-write-coherence): add gap analysis
abb93236 chore(areka-P0-present-write-coherence): initialize spec (spec.json, requirements.md)
```

### V2-6: 要件 6 の各条を、現物の差分で確認する

要件 6.4／6.6／6.7 は「本仕様が触っていないファイルの中身が壊れていないこと」を求める条項である。**設計文書の文言から自明と断じず、当該ファイルを名指しして差分 0 を採った。** ファイルの所在は本節でその場に特定したものであり、設計文書・調査文書から写していない（特定に用いたコマンドは表の下に併記）。

| 受入基準 | 守るべき現物（**実測した字面**） | ファイル（**その場で特定**） | 実在 | `git diff --name-only origin/main...HEAD -- <file> \| wc -l` |
|---|---|---|---|---|
| 6.1 atom の窓書込の形 | 指令の合流・一括適用・書込経路の語彙・整合ゲート | `crates/wintf/src/ecs/window/command.rs` | EXISTS | **0** |
| 6.2 当たり判定の原点 | 配置契約と寸の直接反映 | `crates/areka-emo-present/src/mount.rs` | EXISTS | **0** |
| 6.2 バルーン追従の基準 | 並走 bod のファイル素を含む配置一式 | `crates/areka/src/placement/`（配下全体） | ディレクトリ実在 | **0** |
| 6.4 定常アロケーション 0 | `show.rs:344` の注記行——「成立させた定常状態のアロケーション 0（Requirement 10.4）が壊れる」 | `crates/areka-emo-present/src/presenter/show.rs` | EXISTS | **0** |
| 6.4 段階別計時ログの発行 | `timing.rs:56` = `pub(super) const PERF_LINE_MESSAGE: &str = "perf(apply_show): 段階別計時";` | `crates/areka-emo-present/src/presenter/timing.rs` | EXISTS | **0** |
| 6.6 tick の門の既定（無効） | `world/mod.rs:405` = `tick_gate_enabled: false,`（構築時の既定値） | `crates/wintf/src/ecs/world/mod.rs` | EXISTS | **0** |
| 6.6 門の判断（無効なら必ず回す） | `tick_gate.rs:156` = `if !i.gate_enabled {` | `crates/wintf/src/ecs/world/tick_gate.rs` | EXISTS | **0** |
| 6.6 既定の上書き口（本採用は行わない） | `AREKA_TICK_GATE` の読み口。既定を動かすのは環境変数指定時のみ | `crates/areka/src/tick_gate_config.rs` | EXISTS | **0** |
| 6.7 起床旗 | `tick_wake.rs:1` = 「起床の旗——『次の画面更新で仕事があるか』をプロセス共有のビット集合で持つ」 | `crates/wintf/src/ecs/world/tick_wake.rs` | EXISTS | **0** |
| 6.7 観測の相名 | `transition_diag.rs:328` = `pub enum WriteStage {` ／ `:347` = `pub enum FlushStage {` | `crates/wintf/src/ecs/window/transition_diag.rs` | EXISTS | **0** |
| 6.7 観測の相名（面の側） | `transition_record.rs:121` = `pub(super) enum SurfaceStage {` | `crates/areka-emo-present/src/presenter/transition_record.rs` | EXISTS | **0** |

ファイルの所在と字面の特定に用いたコマンド:

```
grep -rn "定常状態のアロケーション" crates/areka-emo-present/src/presenter/show.rs
grep -n  "PERF_LINE_MESSAGE"        crates/areka-emo-present/src/presenter/timing.rs
grep -n  "tick_gate_enabled"        crates/wintf/src/ecs/world/mod.rs
grep -n  "gate_enabled"             crates/wintf/src/ecs/world/tick_gate.rs
grep -rn "AREKA_TICK_GATE" --include=*.rs crates/
grep -n  "pub enum"                 crates/wintf/src/ecs/window/transition_diag.rs
grep -rn "enum SurfaceStage" --include=*.rs crates/
sed -n '344p'      crates/areka-emo-present/src/presenter/show.rs
sed -n '56p'       crates/areka-emo-present/src/presenter/timing.rs
sed -n '405p'      crates/wintf/src/ecs/world/mod.rs
sed -n '1p'        crates/wintf/src/ecs/world/tick_wake.rs
sed -n '328p;347p' crates/wintf/src/ecs/window/transition_diag.rs
sed -n '121p'      crates/areka-emo-present/src/presenter/transition_record.rs
```

- **6.3 は不適用**（B-4 を採用していないため While 節の前件が偽）。したがって「当たり判定の原点が遷移中も遷移後も規約どおりであることの確認記録」は発生しない。
- **6.5**（既存の決定論テストを壊れたまま残さない）は、`crates/` 配下の差分が **0 ファイル**であることに加え、V1 のワークスペース全体テストが **95 ターゲット全て `test result: ok`・失敗 0** であることで満たされる。退役／更新の判断を要するテストは 1 件も生じていない——**テストファイルを 1 本も触っていない**ため、判断の対象そのものが存在しない。
- **8.3**（上限・判定器・観測語彙を書き換えることで未達を消さない）は、項目 5・6 の差分 0（`transition_diag.rs`・`transition_judge*.rs` 9 本）で構造的に成立する。上限定数 16,667µs そのものの現物確認は V4-b にある。

### V2-7: 判定

- **⑴ 許可集合外 0 件**（採取＝`git diff --name-only origin/main...HEAD | grep -v -E '^\.kiro/specs/areka-P0-present-write-coherence/|^\.kiro/steering/roadmap\.md$' | wc -l` → **0**）。差分 8 ファイルはすべて許可集合の内側。
- **⑵ 接触禁止集合 0 件**（採取＝7 項目それぞれへの `git diff --name-only origin/main...HEAD -- <item> | wc -l` → いずれも **0**。総計 `-- crates/ Cargo.toml` も **0**）。7 項目とも**実在を確かめた上での 0** であり、同じ検査は `design.md` に対して `1 file changed, 322 insertions(+)` を返す。
- **⑶ 想定外の作業ツリー差分 0 件**（採取＝`git status --porcelain | wc -l` → **1**、その 1 行は `vendors/pasta`）。`vendors/pasta` は本枝が加える **13 コミットのいずれにも現れない**（`git log --name-only origin/main..HEAD -- vendors/pasta` が空・全コミットの変更ファイル集合 8 パスに `vendors` の語が **0** 件）。
- **要件 6.1・6.2・6.4・6.5・6.6・6.7・8.3 は、いずれも現物の差分 0 として成立している。** 6.3 は B-4 不採用により不適用。

**この 3 判定の射程（混ぜてはならない）**——ここで確定するのは「**本仕様が実行時の挙動に関わるものを何一つ変更していない**」ことだけである。**未達（`visualize_to_write_us`・`flush_total_us` の違反 40 件）が解消した証拠ではない。** 本仕様は 2026-08-27 の裁定により是正コード 0 行であり、未達は `requirements.md`「未達の登記」節に**引受先なし**で登記されたまま残る。完了報告でこの 3 つの 0 を合否量と同じ息で並べてはならない（要件 8.5）。

---

## V3: 上流アンカーの実測再確認（要件 9.5）

対象ファイル: `crates/areka-emo-present/src/presenter/show.rs`

採取コマンド:

```
grep -n "fn apply_show\|set_visible\|set_bounds\|SurfaceStage::Visualize" crates/areka-emo-present/src/presenter/show.rs
sed -n '46p;375p;381p;389,398p'                                            crates/areka-emo-present/src/presenter/show.rs
```

| # | アンカー | 設計の記載 | 実測 | 現物の字面 | ドリフト |
|---|---|---|---|---|---|
| A1 | 適用の起点 | `:46` | **`:46`** | `    pub(super) fn apply_show(` | **なし** |
| A2 | 可視化 | `:375` | **`:375`** | `            mount.set_visible(world, true);` | **なし** |
| A3 | 寸の反映 | `:381` | **`:381`** | `        mount.set_bounds(world, size);` | **なし** |
| A4 | 観測レコードの発行 | `:392` | **`:392`** | `                stage: SurfaceStage::Visualize,` | **なし** |

- A4 のレコード発行ブロックは `:389` の `if observe_surface {` から始まり、`stamp: stamp_of(world)`（`:391`）・`stage: SurfaceStage::Visualize`（`:392`）・`target_id`（`:393`）・`size: Some(size)`（`:394`）を持つ。調査文書 §0-1 の「`Visualize` 発行:389-398（`stage` は :392）」と一致する。
- **偽陽性の排除**: `set_visible` は `:40`・`:263` にも現れるがいずれも doc コメント／通常コメント行であり、`:184` は不可視化（`false`）側である。可視化の段のアンカーは `:375`（`true`）である。
- **判定: 4 アンカーとも現存・ドリフトなし。**（**この 0 件はアンカーの line 記述に限った範囲である**——設計文書・調査文書のうち V3・V4-a が対象とする 7 アンカーの行番号に更新箇所が 0 件、という意味。アンカー以外の記述の是正は V4-e に 1 件あり、これは別枠である。）

---

## V4: 判定器・語彙・上限の非接触（要件 8.3・5.4）と失効条件の判定

### V4-a: 3 アンカーの実測

| # | アンカー | ファイル | 設計の記載 | 実測 | 現物の字面 | ドリフト |
|---|---|---|---|---|---|---|
| B1 | 飽和減算 | `crates/areka/src/placement/transition_judge.rs` | `:817` | **`:817`** | `                    .map(\|write_us\| write_us.saturating_sub(*visualize_us))` | **なし** |
| B2 | 合否量を armed にする構成子 | `crates/areka/src/placement/transition_judge_verdict.rs` | `:169` | **`:169`** | `    pub const fn signoff() -> Self {` | **なし** |
| B3 | 観測の時刻起点 | `crates/wintf/src/ecs/window/transition_diag.rs` | `:692` | **`:692`** | `pub fn since_tick_start_us() -> u64 {` | **なし** |
| B3′ | 刻印の組み立て | 同上 | `:703` | **`:703`** | `pub fn stamp() -> Stamp {` | **なし** |

- B1 の周辺（`:810-822`）は同一フレームの窓ごとに `write_us - visualize_us` を飽和減算で取り、その最大値を `summary.visualize_to_write_us` へ入れる形のまま。**可視化が書込より後になれば 0＝満点になる**という手渡し罠①の前提は現在も成立している。
- B2 の本体（`:169-175`）は `visualize_to_write_us_max: Some(VISUALIZE_TO_WRITE_US_MAX)` と `flush_total_us_max: Some(FLUSH_TOTAL_US_MAX)` の **2 量とも armed** のまま。調査文書 §5-③ の記述と一致する。
- B3 は `TICK_MIRROR` を読んで tick 開始からの経過を返す形のまま。`stamp()`（`:703-708`）は `frame: current_frame()` と `t_us: since_tick_start_us()` を組む。**`t_us` が tick 起点である**という手渡し罠②の前提は現在も成立している。

### V4-b: 上限 16,667µs

| 定数 | ファイル:行 | 字面 |
|---|---|---|
| `VISUALIZE_TO_WRITE_US_MAX` | `crates/areka/src/placement/transition_judge_verdict.rs:90` | `pub const VISUALIZE_TO_WRITE_US_MAX: u64 = 16_667;` |
| `FLUSH_TOTAL_US_MAX` | `crates/areka/src/placement/transition_judge_verdict.rs:99` | `pub const FLUSH_TOTAL_US_MAX: u64 = 16_667;` |

- 当該ファイルの `origin/main` における最終変更コミットは **`c7b6c829`（atom・PR#114）** であり、本仕様の着手以降に触れられていない。
- **判定: 上限は 16,667µs のまま不変。** 本仕様は上限を緩めていない（要件 3.4・8.3）。

### V4-c: 差分の不在

> **採取の規律（この節が一度壊れた箇所）**: git は**実在しないパスを渡されても空出力・終了コード 0 を返す**。ゆえに「差分なし」を記録する前に、渡す各パスが現物として解決することを先に確かめる。確かめずに採った緑は、何も検査していない緑と見分けがつかない。

#### パスの実在確認（採取の直前に実行）

```
$ for p in crates/areka-emo-present/src/presenter/show.rs \
           crates/areka-emo-present/src/mount.rs \
           crates/wintf/src/ecs/window/command.rs \
           crates/wintf/src/runtime/tick_bridge.rs \
           crates/wintf/src/ecs/window/transition_diag.rs ; do
    [ -f "$p" ] && echo "EXISTS  $p" || echo "MISSING $p" ; done
EXISTS  crates/areka-emo-present/src/presenter/show.rs
EXISTS  crates/areka-emo-present/src/mount.rs
EXISTS  crates/wintf/src/ecs/window/command.rs
EXISTS  crates/wintf/src/runtime/tick_bridge.rs
EXISTS  crates/wintf/src/ecs/window/transition_diag.rs

$ ls -1 crates/areka/src/placement/transition_judge*.rs | wc -l
9
$ ls -1 Cargo.toml crates/*/Cargo.toml | wc -l
25
```

**5 ファイルとも EXISTS・glob 2 件も実体を展開した**（`transition_judge*.rs` が 9 ファイル、`Cargo.toml` 群が 25 ファイル）。以下の採取はすべて解決済みのパスに対するものである。

#### 採取コマンドと結果

```
$ git diff --name-only origin/main...HEAD
.kiro/specs/areka-P0-present-write-coherence/design-validation.md
.kiro/specs/areka-P0-present-write-coherence/design.md
.kiro/specs/areka-P0-present-write-coherence/requirements.md
.kiro/specs/areka-P0-present-write-coherence/research.md
.kiro/specs/areka-P0-present-write-coherence/spec.json
.kiro/specs/areka-P0-present-write-coherence/tasks.md
.kiro/steering/roadmap.md

$ git diff --name-only origin/main...HEAD -- crates/ Cargo.toml | wc -l
0

$ git diff --stat origin/main...HEAD -- \
    crates/areka/src/placement/ \
    crates/wintf/src/ecs/window/transition_diag.rs \
    crates/areka-emo-present/src/presenter/show.rs \
    crates/areka-emo-present/src/mount.rs \
    crates/wintf/src/ecs/window/command.rs \
    crates/wintf/src/runtime/tick_bridge.rs
（出力なし・終了コード 0）

$ git status --porcelain
 M .kiro/specs/areka-P0-present-write-coherence/design.md
 M vendors/pasta
?? .kiro/specs/areka-P0-present-write-coherence/verification/
```

#### 接触禁止集合 7 項目の項目別判定

各項目を**単独の pathspec で**採取した（項目がまとめて 1 コマンドに入ると、1 つが空振りしても他項目の緑に紛れて見えない）。

| # | 項目 | 実在 | `git diff --stat origin/main...HEAD -- <項目>` |
|---|---|---|---|
| 1 | `crates/areka-emo-present/src/presenter/show.rs` | EXISTS | 出力なし＝**差分なし** |
| 2 | `crates/areka-emo-present/src/mount.rs` | EXISTS | 出力なし＝**差分なし** |
| 3 | `crates/wintf/src/ecs/window/command.rs` | EXISTS | 出力なし＝**差分なし** |
| 4 | `crates/wintf/src/runtime/tick_bridge.rs` | EXISTS | 出力なし＝**差分なし** |
| 5 | `crates/wintf/src/ecs/window/transition_diag.rs` | EXISTS | 出力なし＝**差分なし** |
| 6 | `crates/areka/src/placement/transition_judge*.rs` | 9 ファイルへ展開 | 出力なし＝**差分なし** |
| 7 | `Cargo.toml` ＋ `crates/*/Cargo.toml` | 25 ファイルへ展開 | 出力なし＝**差分なし** |

- **対照（この検査が赤を出せることの確認）**: 同じコマンドを `.kiro/specs/areka-P0-present-write-coherence/design.md` に対して実行すると `1 file changed, 322 insertions(+)` を返す。上表の「出力なし」は、検査が働いた上での空である。
- `transition_judge*.rs`・`transition_diag.rs` は差分一覧に**現れない**（差分 0）。要件 5.4（レコード語彙の文言・フィールド名を変更しない）・要件 8.3（判定器を書き換えない）は構造的に成立している。
- 作業ツリーの `vendors/pasta`（サブモジュールのポインタ）は**本仕様の着手前から動いている汚れであり、本仕様は触っていない。本仕様のどのコミットにも含めない。** 残る 2 行（`design.md` の変更・`verification/` の未追跡）は **task 2.1 自身の編集**であり、V2 の採取前にコミットして作業ツリーを確定させる。
- 本節は task 2.1 時点の暫定確認である。証跡としての正本は V2（task 2.4）の採取値。

### V4-d: 失効条件（Revalidation Triggers）4 項目の判定

設計 Boundary Commitments の Revalidation Triggers 表の各行を現物で検査した。

| # | 失効条件 | 実測 | 該当 |
|---|---|---|---|
| T1 | 窓書込の刻印位置が変わる（`write` レコードが `EndDeferWindowPos` の**前**に発行される形になる） | `crates/wintf/src/ecs/window/command.rs`: `flush()` `:724` → `begin` レコード `:742` → `apply_as_batch` 呼出 `:757`（その内側 `:433` で `EndDeferWindowPos`）→ **戻った後** `:776-795` の `if observe { for (index, cmd) … stage: WriteStage::Flush(:780) … }` で指令ごとの `write` レコード → `end` レコード `:800`。刻印は依然として `EndDeferWindowPos` の**後** | **なし** |
| T2 | 窓書込 flush の駆動位置が変わる（`tick_bridge.rs` のスケジュール外駆動をやめる） | `crates/wintf/src/runtime/tick_bridge.rs:258` `crate::ecs::window::flush_window_pos_commands();`。直前 `:257` のコメントは「World 借用スコープ終了後に SetWindowPos コマンドをフラッシュ（省略の回も必ず）」。スケジュール（`world.try_tick_world()` `:246`）を回す借用スコープの**外**のまま | **なし** |
| T3 | 判定器の `saturating_sub`／`Bounds::signoff()` の armed 量が変わる | V4-a の B1（`:817`・飽和減算のまま）・B2（`:169`・2 量とも armed のまま）。V4-c のとおり当該ファイルに差分 0 | **なし** |
| T4 | 可視化の段の位置（`show.rs` の `set_visible`／`set_bounds`／`Visualize` 発行）が動く | V3 の A2（`:375`）・A3（`:381`）・A4（`:392`）がいずれも設計の記載どおり | **なし** |

- 追加行（上限 16,667µs の変更）は失効条件ではなく**要件 8.3 違反**の扱いである。V4-b のとおり変更されていない。
- **判定: 失効条件 4 項目とも該当なし。** 却下理由 R1（B-3 が構造的に届かない）・R2（B-3′ が要件 1.2⑵ に該当）の根拠は現在も有効であり、開発者へ再着手の可否を上げる事由は発生していない。

### V4-e: 記述更新の要否（要件 9.5）

- **line 記述（行番号）で更新を要したものは 0 件である。** 7 アンカーすべてが記載値と一致した。**この 0 件は「7 アンカーの行番号」に限った範囲であり、設計文書全体に是正が不要だった、という意味ではない**（下の是正 1 件がある）。
- **是正 1 件（適用済み）**: 設計文書の**接触禁止集合**（`design.md` の該当ブロック）と **Out of Boundary** の 2 箇所が、実在しないパス `crates/areka-emo-present/src/presenter/mount.rs` を指していた。実パス **`crates/areka-emo-present/src/mount.rs`** へ是正した（`crates/areka-emo-present/src/presenter/` 配下に `mount.rs` は無い——現物は `budget.rs`・`budget_tests.rs`・`hit.rs`・`hub.rs`・`read.rs`・`refresh.rs`・`show.rs`・`target.rs`・`timing.rs`・`timing_tests.rs`・`transition_record.rs`・`transition_record_tests.rs`・`visibility.rs` の 13 本）。
  - **なぜ効くか**: **実在しない pathspec を渡された git は空出力・終了コード 0 を返す**ため、この 1 項目を含む項目別検査は「差分なし」と読める緑を返しながら**実際には何も検査していない**。task 2.4 は接触禁止集合を項目ごとに歩いて「差分なし」を記録するので、7 項目のうち 1 項目が空振りしたまま通ってしまう。結論（`crates/` 全体の `origin/main` 比差分が 0 ファイル）自体は `mount.rs` を包含しており揺るがないが、**項目別の証跡が壊れていた**。
  - 是正の範囲は**パス文字列のみ**。裁定・却下理由・要件参照・結論は 1 文字も変えていない。
  - 是正後の再採取は V4-c の実在確認と項目別判定表に記録した（7 項目とも実在を確かめた上で差分なし・対照として `design.md` は差分を返す）。
- 補足（更新は行わなかった）: 調査文書 §1.3 は `write` レコードを「`:775-796`」と書くが、実測の `if observe { … }` ブロックは **`:776-795`** である。前後 1 行ずつ広く取った括り方であって、コードの移動によるドリフトではない（`a6d27c73`→`18db890f` の `crates/` 差分は 0）。「刻印は `EndDeferWindowPos` が戻った後」という結論は影響を受けない。**更新は task 2.1 の指示（ずれがあれば実測値へ更新する）の対象外と判断して見送った**——ずれの実体が無いため。

---

## V5: steering 追随の確認（要件 8.4）

実施日 2026-08-27。対象 `.kiro/steering/roadmap.md`（全 133 行・`wc -l < .kiro/steering/roadmap.md` → **133**）。

> **本節の位置づけ**: `roadmap.md` は要件段階で追随済みであり、実装フェーズでは**検証対象であって編集対象ではない**（設計 File Structure Plan「既に追随済み（本フェーズでは**検証対象**・編集しない）」表）。本節は追随の**確認**の記録であり、本タスクは `roadmap.md` を 1 行も変更していない。
> **行番号の出所**: 以下の行番号は**すべてその場で採取した実測値**である。設計文書「既に追随済み」表および Testing Strategy V5 行が記載する `:67`／`:82`／`:89` を写していない。

### V5-a: 3 箇所の実測

採取コマンド（ワークツリー直下で実行）:

```
grep -n "present-write-coherence" .kiro/steering/roadmap.md
grep -n "W6.95\*\*（4 本並走）"    .kiro/steering/roadmap.md
grep -n "pwc⇄bod"                  .kiro/steering/roadmap.md
sed -n '67p;82p;89p'               .kiro/steering/roadmap.md
```

| # | 箇所 | 設計の記載 | 実測 | 判定 |
|---|---|---|---|---|
| R1 | M1 残工程ゴール表の本仕様の行 | `:67` | **`:67`** | **反映済み** |
| R2 | ウェーブ編成表の W6.95 の本仕様の行 | `:82` | **`:82`** | **反映済み** |
| R3 | 干渉台帳の同居ペア行（pwc⇄bod） | `:89` | **`:89`** | **反映済み** |

- **偽陽性の排除**: `grep -n "present-write-coherence"` は **4 行**（`:57`・`:67`・`:82`・`:125`）を返す。`:57` は W6.75 atom の完了サマリが引受先として名を挙げている行、`:125` は追記台帳(75)の 1 行要約であり、いずれもゴール表・編成表の行ではない。`grep -n "W6.95\*\*（4 本並走）"` は **4 行**（`:82`〜`:85`）を返し、本仕様の行は先頭の `:82`（他は bod／zsp／bvc）。
- **ドリフトなし**——3 箇所とも設計文書の記載値と一致した。したがって設計文書「既に追随済み」表の行番号は**陳腐化していない**（是正すべき記載は無い）。

#### R1: ゴール表の行（`:67`）の現物の字面

```
| 見た目 | 遷移中に絵と窓が同じ提示フレームで揃う（要件 4.2 の実機側・可視化→書込の隙間 0.21〜0.31 秒） | `present-write-coherence` | **W6.95**（4 本並走の 1 本目・「cage の後」＝W6.9 完走で充足・棚卸⑪申し送りは pwc brief）・**要件討議裁定 2026-08-27＝見送り＋登記で確定**（B-3/B-4 とも不採用・実行時挙動不変・是正は行わずゴールは未達のまま登記して閉じる） |
```

- 裁定の 3 要素——**見送り＋登記**・**B-3/B-4 とも不採用**・**実行時挙動不変**——がいずれも字面として載っている。加えて「**ゴールは未達のまま登記して閉じる**」と書かれており、**ゴール行が達成扱いに書き換えられていない**（要件 8.5 の趣旨＝合格と読める書き方をしない、が編成表の側でも守られている）。

#### R2: W6.95 の編成行（`:82`）の裁定部分

行末に次の裁定ブロックが追記されている（逐語）:

```
**→ 要件討議裁定 2026-08-27＝見送り＋登記で確定**——⑴ 文言どおりの B-3（可視化を窓書込の直前へ）は刻印位置の構造から上限 16,667µs に届かない（隙間の下限＝`flush_total_us` 143,231µs 以上）⑵ 届く唯一の形 B-3′（flush 完了後に可視化を駆動）は pwc 要件 1.2⑵「tick と flush の駆動関係の変更」該当＝大改造で除外 ⑶ B-4 は合否量に効かず当たり判定原点への接触リスクが費用に見合わず却下。**実行時挙動不変・未達 40 件と引受先なしの登記が正本＝pwc requirements.md「未達の登記」節**
```

- 却下理由 R1（B-3 が構造的に届かない）・R2（B-3′ が要件 1.2⑵ 該当）・B-4 却下の 3 点が、requirements.md「裁定の記録」と同じ形で載っている。
- **引受先なし**が編成表の側にも明記されている（要件 8.2＝ウェーブ名を引受先として書かない、の遵守。「W8 へ」「e2e で拾う」のような後送先は書かれていない）。
- 同じ行の前半（着手前に書かれた条件）は**書き換えられていない**——`接触面は presenter/show.rs の可視化の段`（`sed -n '82p' … | grep -c "接触面は"` → **1**）・`tick 構造の大改造に及ぶなら atom 要件 9.3 に従い分割を再裁定する`（同 grep -c → **1**）はそのまま残り、末尾の裁定ブロックがその条件の**帰結**として接続している。⑵ の `要件 1.2⑵` の字面も現物にある（同 grep -c → **1**）。

#### R3: 干渉台帳の同居ペア行（`:89`）

`sed -n '89p' .kiro/steering/roadmap.md` の全文:

```
- **pwc⇄bod（W6.95 同居ペア）**〔ファイル素（pwc＝`presenter/show.rs`／`mount.rs`・bod＝`placement/follow` 系＋`windowposition.rs`＋`persist.rs`）。**要ウォッチ→解消（2026-08-27）**: pwc が B-4 を採る場合のみ `mount.rs` 配置契約・当たり判定原点で bod の関心事に意味論上近接する条件付きウォッチだったが、pwc 要件討議で B-4 却下（見送り＋登記）が確定したため解消——bod 側は pwc の登記のみ参照でよい〕
```

### V5-b: 要ウォッチの解消の登記（要件 9.2）

- **判定: 解消として登記済み。** 台帳 `:89` は **`要ウォッチ→解消（2026-08-27）`** と札を書き換えた上で、解消の理由を **「pwc 要件討議で B-4 却下（見送り＋登記）が確定したため」** と逐語で書いている。**B-4 の却下が理由として明示されている**——札だけ消して理由が残らない形にはなっていない。
- 元のウォッチが**条件付き**（「pwc が B-4 を採る場合のみ」）であったことも保存されており、失効の機序（条件節の前件が偽になった）が後から読める。
- 下流への指示も同じ行にある——「bod 側は pwc の登記のみ参照でよい」。
- 設計側の対応記述と一致する: design.md Requirements Traceability の 9.1〜9.3 の並びにある **9.2 行**が「**不適用**（B-4 不採用）。`roadmap.md:89` の要ウォッチは解消として登記済み」と書いており、**参照している行番号 `:89` も実測と一致する**。
- **pwc が登場する他の台帳行の確認**（`grep -n "pwc" .kiro/steering/roadmap.md` → 17 行。うち干渉台帳の生存ペア行は `:89`・`:90`・`:93` の 3 本）:
  - `:90` zsp⇄pwc——弱接触 2 点。⑴ は「pwc は読むだけ・改変予定なし」と判別済み、⑵ は COMPAT §8 の行隣接。**本裁定で状態が変わる項目を含まない**（B-4 に依存していない）。
  - `:93` bvc⇄pwc／bvc⇄bod——「**ウォッチ事項なし**」。
  - すなわち**本裁定で解消すべき要ウォッチは `:89` の 1 件のみ**であり、それが解消として登記されている。取り残しは無い。

### V5-c: 担当する範囲と担当しない範囲の切れ目（要件 9.1・9.3）

#### 担当する範囲＝可視化の段

| 出所 | 記述 |
|---|---|
| `roadmap.md:82` | `接触面は presenter/show.rs の可視化の段（apply_show:46 の末尾＝set_visible:375／set_bounds:381／Visualize 発行:392）` |
| design.md `Boundary Commitments` → `This Spec Owns` | 裁定の登記・未達の登記・不適用要件の確定・残余検証の定義・将来仕様への手渡し（＝**可視化の段をめぐる裁定**が本仕様の所有物） |
| requirements.md 9.1 | 「可視化の段（いつ絵を見せるか）を自身の担当とし、窓書込の駆動（tick の相順）を担当しない」 |

- **一致している。** ただし読み違えを防ぐために二段であることを明記しておく——**責務としての担当範囲＝可視化の段**（要件 9.1）であるのに対し、**改変集合＝空**である（見送り裁定により design.md `Out of Boundary` は同じ `show.rs` を「**読むだけで変えない**」側に置く）。編成表 `:82` が「接触面」と書き、設計が「Out of Boundary」と書くのは**矛盾ではなく、担当範囲と改変集合が別だから**である。roadmap 側でもこの二段は末尾の「実行時挙動不変」で閉じている。

#### 担当しない範囲 3 件と、その担当仕様の実在

実在確認の採取コマンド:

```
for d in areka-P0-balloon-offset-dpi areka-P0-tick-gate-adoption areka-P0-draw-load-parity; do
  for base in .kiro/specs .kiro/specs/completed; do
    [ -d "$base/$d" ] && echo "EXISTS  $base/$d"
  done
done
→ EXISTS  .kiro/specs/areka-P0-balloon-offset-dpi
  EXISTS  .kiro/specs/areka-P0-tick-gate-adoption
  EXISTS  .kiro/specs/completed/areka-P0-draw-load-parity
```

| # | 担当しない範囲 | 担当仕様 | **実在するディレクトリ**（実測） | 中身 | roadmap 側の登記 | design 側の登記 |
|---|---|---|---|---|---|---|
| N1 | tick の相順／tick の門の本採用 | `tick-gate-adoption` | `.kiro/specs/areka-P0-tick-gate-adoption` | `brief.md` 1 本（`ls … \| wc -l` → **1**） | `:58`（採取＝`sed -n '58p' … \| grep -c 'tick-gate-adoption. が引受け済み'` → **1**）・`:97`・`:121`・`:133` | Non-Goals（「tick の門の本採用（`tick-gate-adoption` の担当）」）＋ Out of Boundary（`tick_bridge.rs` の flush 駆動） |
| N2 | フレーム駆動の CPU 負荷 | `draw-load-parity`（完了） | `.kiro/specs/completed/areka-P0-draw-load-parity` | 9 エントリ（`ls … \| wc -l` → **9**） | `:58`（`grep -n 'draw-load-parity' …` → 58 行目に完了サマリ「**W6.9 draw-load-parity ✅**（08-23）」） | Non-Goals（「フレーム駆動の CPU 負荷（`draw-load-parity` が閉じた）」） |
| N3 | バルーン追従オフセットの k 倍 | `balloon-offset-dpi` | `.kiro/specs/areka-P0-balloon-offset-dpi` | `brief.md` 1 本（`ls … \| wc -l` → **1**） | `:68`（ゴール表の bod 行＝`DPI 遷移時の BalloonFollow.offset スケール意味論確定`）・`:83`（W6.95 の bod 編成行） | Non-Goals（「バルーン追従オフセットの k 倍（`balloon-offset-dpi` の担当）」）＋ Out of Boundary（並走 3 仕様のファイル素＝`placement/follow` 系） |

- **3 件とも、編成表と設計の境界節で切れ目が一致している。** 担当仕様名も両文書で同一（ウェーブ名や「W8 で」のような曖昧な後送先は**どちらにも無い**）。
- **3 件とも担当仕様がディレクトリとして実在する**（上の `EXISTS` 3 行）。**ウェーブ名を担当者と読み替えた箇所は無い。**
- **ただし限界を明記する**: N1・N3 のディレクトリは現状 `brief.md` 1 本のみ＝**起票済みで要件以降は未生成**である。「担当仕様として実在する」ことは確認できたが、「その仕様が当該範囲を要件として引き受けている」ことまでは**本節では確認していない**（N1 の申し送り正本は dlp `requirements.md` 改訂欄、N3 は bod brief 追記(70)＋棚卸⑩ブロック——いずれも roadmap `:58`／`:83` が名指しする所在であり、本節はその所在の記載があることまでを見た）。
- **混同しないこと**: ここで実在を確かめた 3 仕様は**境界の相手**（担当しない範囲の受け皿）であって、**本仕様の未達 40 件の引受先ではない**。未達の引受先は要件 8.2 のとおり **なし**であり（requirements.md「未達の登記」節・roadmap `:82` の「引受先なしの登記が正本」）、N1〜N3 をもって未達が誰かに渡ったと読んではならない。

### V5-d: 判定

- **要件 8.4（改訂を設計・境界節・steering の編成表まで追随させる）は充足している。** 3 箇所（`:67`・`:82`・`:89`）とも見送り＋登記の裁定を反映しており、実測行番号は設計文書の記載と一致（ドリフト 0）。
- **要件 9.2 は不適用として正しく閉じている**——台帳 `:89` の要ウォッチが B-4 却下を理由に**解消**と登記済み。
- **要件 9.1・9.3 の切れ目は編成表と設計の境界節で一致**しており、担当しない 3 範囲の担当仕様はいずれも実在のディレクトリとして確認できた。
- **不一致・ドリフトは 1 件も検出しなかった。** 本節は `roadmap.md` を 1 行も変更していない（検証対象であって編集対象ではない）。

---

## 登記の突合⑴: 不適用と生きている受入基準の確定（task 3.1）

実施日 2026-08-27。対象 HEAD `a6afd4f3`（`feat(areka-P0-present-write-coherence): コード非接触の証跡を採取 (task 2.4)`）。作業ツリー差分は `git status --porcelain` → ` M vendors/pasta` の 1 行のみ（V2-5 と同じ扱い）。

> **本節が塞ぐ誤読**: 見送り裁定により**発動しない**受入基準（採用時条項）を、**未達**の受入基準と読むこと。両者は別物である——未達は「課したが届かなかった」、不適用は「そもそも課していない」。本仕様の未達は要件 2.1 に紐づく **40 件だけ**であり、それ以外の 22 件は不適用であって未達ではない。
> **本節の限界**: 本節は**文書どうしの整合**しか見ていない。「その分類が妥当か」（＝見送り裁定そのものの当否）は要件討議 #2 の開発者裁定であり、本節は再審しない。
> **数の出所**: 本節に載せる集合と件数は**すべて隣に採取コマンドを置く**。文書に書かれた件数を写して埋めていない。

### 突合⑴-0: 採取の基準点と、突合の対象 3 文書

採取コマンド（すべて仕様ディレクトリ `.kiro/specs/areka-P0-present-write-coherence/` を作業ディレクトリとして実行。シェルは Git Bash）:

```
git log -1 --format='%h %s'
git status --porcelain
wc -l requirements.md design.md tasks.md
```

| 文書 | 突合における役割 | `wc -l` |
|---|---|---|
| `requirements.md` | 不適用一覧の正本（末尾「見送りにより不適用となる要件」節 `:227`〜`:235`） | **237** |
| `design.md` | トレーサビリティ表の正本（`## Requirements Traceability` `:178`・データ行 `:182`〜`:237`） | **322** |
| `tasks.md` | 担当タスクの正本（各タスクの `_Requirements:` 行） | **88** |

### 突合⑴-1: 受入基準の全集合と、両文書の ID 空間の一致

まず**突合の土俵が同じであること**を先に確かめる。片方が持っていない ID があれば、その先の集合差は意味を持たない。

採取コマンド:

```
awk '/^### Requirement [0-9]+:/ {r=$3; sub(/:.*/,"",r)} /^[0-9]+\. / {if(r!=""){n=$1; sub(/\./,"",n); print r"."n}}' requirements.md > /tmp/req_all.txt
awk -F'|' '/^\| [0-9]+\.[0-9]+ \|/ {id=$2; gsub(/ /,"",id); print id}' design.md > /tmp/des_all.txt
wc -l < /tmp/req_all.txt ; wc -l < /tmp/des_all.txt
diff <(sort /tmp/req_all.txt) <(sort /tmp/des_all.txt) && echo "IDENTICAL: 0 diff"
```

出力:

```
56
56
IDENTICAL: 0 diff
```

- **受入基準は全 56 件**（要件別の内訳＝`cut -d. -f1 /tmp/req_all.txt | uniq -c` → 要件 1:6／2:6／3:8／4:6／5:6／6:7／7:7／8:5／9:5）。
- **design のトレーサビリティ表はデータ行 56 行ちょうどで、ID 空間が requirements と完全一致**（差 0）。取りこぼした受入基準も、存在しない ID を立てた行も無い。

### 突合⑴-2: 表 A — 不適用の受入基準（**23 件**）

#### 2 つの集合の作り方（**粒度が違うので、そのままでは diff できない**）

- **design 側**は受入基準 ID ごとに 1 行あり、不適用の行は実現要素セルの**冒頭が `**不適用**`** で始まる。ID 単位でそのまま抽出できる。
- **requirements 側は ID 単位ではない**。`:229`〜`:235` の 7 本の箇条のうち、ID を明示列挙しているのは要件 2（`:229`）と要件 4（`:230`）だけで、要件 3・5・7 は**除外の条件を文で述べて生存者を名指しする形**、要件 6・9 は**要件番号のまとめ**である。したがって requirements 側は「箇条の述べる除外条件を受入基準の本文へ機械的に当てて ID へ展開する」必要がある。展開規則は下表に固定し、規則ごとに採取コマンドを置いた。

| 出所 | 箇条の該当部（逐語） | ID への展開規則 | 得られる ID |
|---|---|---|---|
| `:229` 要件 2 | `2.6 発動＝2.1 は課さない。2.2〜2.5 は採用時条項のため不適用。` | **明示列挙**（展開不要） | 2.1, 2.2, 2.3, 2.4, 2.5 |
| `:230` 要件 4 | `4.1〜4.5 は…存続。**4.6 は…発動しない**` | **明示列挙**（展開不要） | 4.6 |
| `:231` 要件 3 | `是正を行わないため是正前後の実機比較・実機サインオフは実施しない` | 要件 3 の受入基準のうち**本文に「実機採取」または「サインオフ」を含むもの**（＝その実施を要求する条項） | 3.1, 3.2, 3.3, 3.6, 3.8 |
| `:232` 要件 5 | `提示側観測点は追加しない。5.1（…）は登記事項として存続する。` | 要件 5 の受入基準のうち**本文に「観測点」を含むもの**（＝観測点の追加を前提とする条項） | 5.2, 5.3, 5.5, 5.6 |
| `:233` 要件 7 | `是正で新たに生じる判断分岐が無いため新規の決定論テストは無い。7.7（…）のみ完了条件として生きる。` | 要件 7 の受入基準のうち**新規の決定論テストの存在を前提とする条項**（作成・内容・配置の義務）。※ 後述のとおり 7.6 は該当しない | 7.1, 7.2, 7.3, 7.4, 7.5 |
| `:234` 要件 6・9 | `コード非接触のため自明に成立` | 要件 6・9 の受入基準のうち **EARS 条件節（Where／If／When）で始まり、その前件が偽であるもの**（B-4 不採用・新規採取なし） | 6.3, 9.2, 9.4 |
| `:235` 要件 8.4 | `steering roadmap.md … 追随済み` | 不適用の宣言ではない（充足の報告） | — |

規則を当てた採取コマンドと出力（`/tmp/req_txt.tsv` は突合⑴-1 と同じ awk に受入基準の本文を第 2 列として持たせたもの＝`awk '/^### Requirement [0-9]+:/ {r=$3; sub(/:.*/,"",r)} /^[0-9]+\. / {if(r!=""){n=$1; sub(/\./,"",n); print r"."n"\t"$0}}' requirements.md > /tmp/req_txt.tsv`）:

```
$ awk -F'\t' '$1 ~ /^3\./' /tmp/req_txt.tsv | grep -E "実機採取|サインオフ" | cut -f1 | tr '\n' ' '
3.1 3.2 3.3 3.6 3.8
$ awk -F'\t' '$1 ~ /^3\./' /tmp/req_txt.tsv | grep -vE "実機採取|サインオフ" | cut -f1 | tr '\n' ' '
3.4 3.5 3.7
$ awk -F'\t' '$1 ~ /^5\./' /tmp/req_txt.tsv | grep -E "観測点" | cut -f1 | tr '\n' ' '
5.2 5.3 5.5 5.6
$ awk -F'\t' '$1 ~ /^5\./' /tmp/req_txt.tsv | grep -vE "観測点" | cut -f1 | tr '\n' ' '
5.1 5.4
$ awk -F'\t' '$1 ~ /^[69]\./ {t=$2; sub(/^[0-9]+\. /,"",t); if (t ~ /^(Where|If|When) /) printf "%s ", $1}' /tmp/req_txt.tsv
6.3 9.2 9.4
$ awk -F'\t' '$1 ~ /^[69]\./ {t=$2; sub(/^[0-9]+\. /,"",t); if (t !~ /^(Where|If|When) /) printf "%s ", $1}' /tmp/req_txt.tsv
6.1 6.2 6.4 6.5 6.6 6.7 9.1 9.3 9.5
```

要件 3 の規則が拾った 5 件のうち 3.2・3.6・3.8 は**主語そのものが `The 実機採取 shall`** であり、3.1・3.3 は述語が `atom のサインオフ手順書に従った実機採取で判定し` ／ `既存のサインオフ判定で行い` である。落ちた 3 件（3.4・3.5・3.7）はいずれも採取の実施を要求せず、上限を緩めない・未測定を合格へ丸めない・目視と機械判定の食い違いを記録する、という**読み方と禁止の条項**である。

#### ⚠ 道具の罠——**素朴なトークン一致は「食い違い 0 件」を偽って出す**

design 側を「実現要素セルに文字列 `不適用` を含む行」で抽出すると **23 件**が返り、requirements 側の 23 件と**対称差 0 件で一致してしまう**。しかしこれは誤りである。全 23 行を目で確かめると、混入していたのは 4.6 の行であり、そのセルは自分自身を不適用と宣言していない:

```
$ awk -F'|' '/^\| [0-9]+\.[0-9]+ \|/ && $4 ~ /不適用/ {id=$2; gsub(/ /,"",id); printf "%-5s|%s\n", id, $4}' design.md
…
4.6  | C3（3.x 不適用に伴い、本仕様の完了宣言は「合格」ではなく「未達の登記」であることを明示＝…） 
…
```

**`3.x 不適用に伴い` の「不適用」を拾った偽陽性**である。しかもこの偽陽性は、後述する本物の差（4.6）をちょうど埋める位置に立つため、**間違った抽出のほうが「一致」という気持ちのよい結論を出す**。素朴な抽出を採っていたら、本節は誤った根拠で結論を書いていた。正しい抽出規則は**セル冒頭の `**不適用**` マーカー**である:

```
$ awk -F'|' '/^\| [0-9]+\.[0-9]+ \|/ {id=$2; gsub(/ /,"",id); c=$4; sub(/^ +/,"",c); if (c ~ /^\*\*不適用\*\*/) print id}' design.md | sort -u > /tmp/des_na_strict.txt
$ wc -l < /tmp/des_na_strict.txt
22
```

上の偽陽性検査は 23 行を全文で出力しており、`**不適用**` が冒頭に立たない行は **4.6 の 1 行だけ**であることをその出力で確認した（残る 22 行はすべて `**不適用**（…）` で始まる）。

#### 対称差の採取

```
$ wc -l < /tmp/req_na.txt          # requirements 側（上表の規則で展開）
23
$ wc -l < /tmp/des_na_strict.txt   # design 側（冒頭マーカー）
22
$ comm -3 /tmp/req_na.txt /tmp/des_na_strict.txt
4.6
$ comm -3 /tmp/req_na.txt /tmp/des_na_strict.txt | wc -l
1
```

**対称差は 1 件＝`4.6` のみ**（requirements 側にあり、design のトレーサビリティ表の**マーカー**には無い）。この 1 件の性格は次節で裁定する——結論だけ先に書くと、**分類（発動する／しない）は両文書で一致しており、差は design 表がマーカーを置いていないという表記の差である**。

#### 表 A（不適用 23 件・requirements 側の根拠と design 側の逐語）

| # | ID | requirements 側の根拠 | design 側の記載（実現要素セル逐語） | 不適用の型 |
|---|---|---|---|---|
| 1 | 2.1 | `:229` 明示（`2.1 は課さない`） | `**不適用**（2.6 発動）。未達として C1 が登記` | 2.6 発動（見送り裁定） |
| 2 | 2.2 | `:229` 明示（`採用時条項のため不適用`） | `**不適用**（採用時条項・B-3 不採用）` | 採用時条項（B-3 不採用） |
| 3 | 2.3 | `:229` 明示 | `**不適用**（採用時条項・B-4 不採用）` | 採用時条項（B-4 不採用） |
| 4 | 2.4 | `:229` 明示 | `**不適用**（一時配置を導入しない）` | 採用時条項（B-4 不採用の帰結） |
| 5 | 2.5 | `:229` 明示 | `**不適用**（同上）` | 同上 |
| 6 | 3.1 | `:231` 規則（述語に「サインオフ手順書に従った実機採取」） | `**不適用**（是正しないため合否判定を行わない）。C3 が明示` | 是正なし＝合否判定なし |
| 7 | 3.2 | `:231` 規則（主語が `The 実機採取`） | `**不適用**（同上・起点実測 8 遷移が「前」の保全記録）` | 新規採取なし |
| 8 | 3.3 | `:231` 規則（述語に「既存のサインオフ判定」） | `**不適用**（同上）。判定器は非接触＝要件 8.3 の側で保たれる` | 新規採取なし |
| 9 | 3.6 | `:231` 規則（主語が `The 実機採取`） | `**不適用**（新規採取を行わない）` | 新規採取なし |
| 10 | 3.8 | `:231` 規則（主語が `The 実機採取`） | `**不適用**（是正が無いため「後」が存在しない）` | 是正なし＝「後」が無い |
| 11 | **4.6** | `:230` 明示（`発動しない`） | **マーカーなし**。`C3（…合格宣言を行わないので双方要求の趣旨は破られない）`＋本文 `:272` の「要件 4.6 の読み」 | 合格宣言なし ← **対称差の 1 件** |
| 12 | 5.2 | `:232` 規則（本文に「観測点」） | `**不適用**（名指ししない）` | 観測点追加なし |
| 13 | 5.3 | `:232` 規則 | `**不適用**（観測点を追加しない）。C2 の R4 が理由を保持` | 観測点追加なし |
| 14 | 5.5 | `:232` 規則 | `**不適用**（追加しない）` | 観測点追加なし |
| 15 | 5.6 | `:232` 規則 | `**不適用**（同上）` | 観測点追加なし |
| 16 | 6.3 | `:234` 規則（`Where B-4 を採用したとき`） | `**不適用**（B-4 不採用）` | 採用時条項（B-4 不採用） |
| 17 | 7.1 | `:233` 規則（新規テストの網羅義務） | `**不適用**（是正が無く新たな判断分岐が 0 個）` | 是正なし |
| 18 | 7.2 | `:233` 規則（`Where B-4 を採用したとき`） | `**不適用**（B-4 不採用）` | 採用時条項（B-4 不採用） |
| 19 | 7.3 | `:233` 規則（`Where B-3 を採用したとき`） | `**不適用**（B-3 不採用）` | 採用時条項（B-3 不採用） |
| 20 | 7.4 | `:233` 規則（新規テストの形の義務） | `**不適用**（是正が無い。…趣旨は C3 が保つ）` | 是正なし |
| 21 | 7.5 | `:233` 規則（新規テストの配置義務） | `**不適用**（新規テストを作らない）` | 新規テストなし |
| 22 | 9.2 | `:234` 規則（`Where B-4 を採用して配置契約に触れるとき`） | `**不適用**（B-4 不採用）。roadmap.md:89 の要ウォッチは解消として登記済み` | 採用時条項（B-4 不採用） |
| 23 | 9.4 | `:234` 規則（`When tick の門が有効な状態で…`） | `**不適用**（新規採取を行わない。門の既定は OFF のまま非接触）` | 新規採取なし |

- **不適用 23 件のうち、未達として登記されている量に紐づくものは 2.1 の 1 件のみであり、その量は要件 8 の登記へ移されている。** 未達 40 件が紐づく受入基準は **2.1 のみ**であり、その 2.1 は「2.6 発動により**課さない**」＝不適用として扱われた上で、量そのものは要件 8 の登記へ移されている（`requirements.md` 末尾節）。**「不適用にしたから未達が消えた」形にはなっていない**——量は登記に残っている。
- 不適用の型は 7 種に収まる——2.6 発動／採用時条項（B-3 不採用）／採用時条項（B-4 不採用）／是正なし／新規採取なし／観測点追加なし／合格宣言なし。いずれも**見送り裁定から一意に導かれる**もので、個別の裁量で落とした条項は無い。

### 突合⑴-3: 合格の宣言を前件とする条項の裁定

#### 「合格」に触れる受入基準は 5 件、うち**合格宣言の成立を前件とするのは 4.6 のみ**

```
$ awk -F'\t' '{t=$2; if (t ~ /合格|完了の宣言/) print $1}' /tmp/req_txt.tsv | tr '\n' ' '
3.1 3.5 4.6 7.6 8.5
```

| ID | 合格との関係 | 発動するか | 両文書の扱い |
|---|---|---|---|
| 3.1 | `決定論テストの結果だけでは合格としない`＝**実機採取の実施義務** | しない | 両文書とも不適用（表 A #6） |
| 3.5 | `合格へ丸めず`＝**丸めの禁止** | **する**（禁止は常時効く） | 両文書とも生存（design → C1） |
| **4.6** | `完了の宣言に実機採取と決定論テストの双方の結果を要求`＝**合格を宣言する場合の要求** | **しない**（前件が成立しない） | 下記のとおり**読みは一致・表記のみ差** |
| 7.6 | `決定論で合格判定しない`＝**判定の禁止** | **する**（禁止は常時効く） | 両文書とも生存（design → C3） |
| 8.5 | `未達を残したまま完了するとき…合格と読める書き方をしない`＝**書き方の禁止** | **する**（本仕様はまさにこの形で閉じる） | 両文書とも生存（design → C3・task 3.3 が担当） |

#### 4.6 の両文書の逐語

`requirements.md:230`:

```
- 要件 4: 4.1〜4.5 は未達の登記の読み方として存続。**4.6 は「合格を宣言する場合に実機と決定論の双方の結果を要求する」条項であり、本仕様は合格を宣言しない（閉じ方は未達の登記）ため発動しない**（設計文書「完了報告の規律」の読みと対・2026-08-27 設計討議で追記）。
```

`design.md:272`（「完了報告の規律」節の末尾・**要件 4.6 の読み**）:

```
**要件 4.6 の読み（設計側で明示する解釈）**: 4.6 は「完了の宣言に実機採取と決定論テストの双方の結果を要求する」と述べるが、これは**是正の合格を宣言する場合の要求**である。見送りの裁定により要件 3 の実機採取は実施されない（`requirements.md`「見送りにより不適用となる要件」）ため、本仕様は**合格を宣言しない**——閉じ方は「未達 40 件の登記」であり、片方の結果をもって合格と読み替える行為そのものが起きない。
```

`design.md:207`（トレーサビリティ表の 4.6 行・実現要素セル）:

```
C3（3.x 不適用に伴い、本仕様の完了宣言は「合格」ではなく「未達の登記」であることを明示＝合格宣言を行わないので双方要求の趣旨は破られない）
```

#### 判定

- **両文書は同じ読みを述べている。** ⑴ 4.6 は合格を宣言する場合の要求である ⑵ 本仕様は合格を宣言しない ⑶ ゆえに双方要求は発動しない——この 3 段が両方に揃っている。requirements 側は結論を **`発動しない`**、design 側は同じ結論を **`趣旨は破られない`**（＝前件が成立しないので破りようがない）と書いている。**語法が違うだけで、指している状態は同一**である。
- したがって**分類（発動する／しない）の食い違いは 0 件**。差は **design のトレーサビリティ表が 4.6 行に `**不適用**` マーカーを置いていない**という**表記の差 1 件**である。
- **この差を実質的な不一致と読まない根拠**は 2 つ。⑴ requirements 側が自ら `（設計文書「完了報告の規律」の読みと対）` と書いており、**design 側の表現が表のマーカーではなく本文の段落である前提**で対応づけている。⑵ design の C3 コンポーネント行（`:247`）の Req 対応列が `2.6, 4.4, 4.6, 7.4, 7.6, 8.5` と 4.6 を含み、C3 の Intent は `発動しない受入基準と生きている受入基準を一覧化する`——**4.6 は C3 の管轄物として登録されている**。
- **本節は両文書のいずれも改訂しない。** `requirements.md`・`design.md` は本タスクの編集対象外であり、上記は分類の不一致ではないため是正を要さない。**ただし将来 design のトレーサビリティ表だけを機械で読む道具が現れると、4.6 は生存側に分類される**——後続への申し送りとして突合⑴-6 に残す。

#### 鏡像の注意——7.6 は「7.7 のみ生きる」の巻き添えにしてはならない

`requirements.md:233` は `7.7（…）のみ完了条件として生きる` と書く。この `のみ` を要件 7 全体に掛けて読むと 7.6 まで不適用に落ちるが、**それは誤読である**——限定は `完了条件として` に掛かっており、7.6 は完了条件ではなく**禁止条項**（`実機の目視でしか判らない効果…を決定論で合格判定しない`）だからである。design は 7.6 を `C3（決定論の結果を提示フレーム一致の証拠にしない・4.4 と対）` として**生存側**に置き、`7.7` の行にだけ `**C4（本仕様で唯一生きている完了条件）**` と書く——**`唯一生きている完了条件` という限定の掛け方が requirements の `のみ完了条件として生きる` と字句レベルで一致している**。よって 7.6 は両文書とも生存であり、対称差には現れない（実際 `comm -3` の出力に 7.6 は無い）。4.6 と 7.6 は**限定の掛かり先を読み違えると分類が反転する対の 2 件**であり、本節はどちらもその場で本文を読んで裁定した。

### 突合⑴-4: 表 B — 生きている受入基準（**33 件**）→ 担当タスク

生存集合は全 56 件から不適用 23 件を引いて得る。担当は `tasks.md` の各タスクの `_Requirements:` 行を正本とする。

採取コマンド:

```
comm -23 <(sort -u /tmp/req_all.txt) /tmp/req_na.txt > /tmp/live_req.txt
grep -o '_Requirements:[^_]*' tasks.md | tr ',' '\n' | grep -oE '[0-9]+\.[0-9]+' | sort -u > /tmp/task_ids.txt
wc -l < /tmp/live_req.txt
wc -l < /tmp/task_ids.txt
comm -23 /tmp/live_req.txt /tmp/task_ids.txt | wc -l              # 担当タスクを持たない生存基準
comm -23 <(sort -u /tmp/req_all.txt) /tmp/task_ids.txt | wc -l    # 全 56 件のうち tasks.md に現れない ID
```

出力:

```
33      # 生存
56      # tasks.md の _Requirements に現れる ID の和集合
0       # 担当タスクを持たない生存基準
0       # tasks.md に現れない受入基準
```

- **生存 33 件・担当タスクを持たない生存基準 0 件。**
- さらに強い事実として、**全 56 件が漏れなく `tasks.md` のどれかの `_Requirements:` に現れる**（不適用 23 件も含めて）。不適用側も同じ awk で逆引きして確認した——不適用 23 件のうち 22 件は task 3.1（本タスク）が、9.2 のみ task 2.3 が担当し、**担当なしは 0 件**。すなわち**受入基準は一つ残らずどこかのタスクに紐づいており、「仕様の内側から見えない受入基準」は 1 件も無い**。

対応表（タスク側から逆引き。同一 ID を複数タスクが担当する場合はすべて列挙）:

```
awk 'BEGIN{while((getline l < "/tmp/live_req.txt")>0){live[l]=1; ord[++c]=l}}
/^- \[[ x]\] [0-9]/ {match($0,/\] [0-9.]+/); tid=substr($0,RSTART+2,RLENGTH-2); sub(/\.$/,"",tid)}
/_Requirements:/ {line=$0; gsub(/.*_Requirements: /,"",line); gsub(/_.*$/,"",line); n=split(line,a,","); for(i=1;i<=n;i++){gsub(/ /,"",a[i]); if(a[i] in live) own[a[i]]=own[a[i]] (own[a[i]]?" / ":"") tid}}
END{orph=0; for(i=1;i<=c;i++){k=ord[i]; o=(k in own)?own[k]:"***ORPHAN***"; if(o=="***ORPHAN***")orph++; printf "%-5s -> %s\n", k, o} printf "生存 %d 件 / 担当なし %d 件\n", c, orph}' tasks.md
→ 末尾行: 生存 33 件 / 担当なし 0 件
```

| ID | 生きている内容 | 担当タスク |
|---|---|---|
| 1.1 | B-3 の規模見積りを作成し記録する | 3.2 |
| 1.2 | 「大改造」の 3 定義 | 3.2 |
| 1.3 | 該当時は B-3 を採らず分割を再裁定し結論を記録 | 3.2 |
| 1.4 | 候補を C8 の表の内側に限る | 3.2 |
| 1.5 | B-3・B-4・見送りの一意な裁定と、採らなかった理由 | 3.2 |
| 1.6 | 上流の食い違いに対する読みの明記 | 3.2 |
| 2.6 | 見送り時は 2.1 を課さず要件 8 の登記へ替える（**発動済み**） | 3.1 |
| 3.4 | 上限 16,667µs を緩めない | 2.1 |
| 3.5 | 未測定を合格へ丸めない | 3.2 |
| 3.7 | 目視所見と機械判定の一致・食い違いの記録 | 3.2 |
| 4.1 | 合否量は `visualize_to_write_us` | 3.2 |
| 4.2 | `flush_total_us` は測るが合否に載せない | 3.2 |
| 4.3 | 残違反の件数と値・引受先の明示 | 3.2 |
| 4.4 | 決定論 PASS を提示フレーム一致の証拠と読まない | 3.2 |
| 4.5 | 意味が変わった量で前後比較しない | 3.2 |
| **5.1** | **見え方の順序を未特定として扱う**（登記事項として存続） | 3.2 |
| 5.4 | 既存レコード語彙の文言・フィールド名を変更しない | 2.1 |
| 6.1 | atom の窓書込の形を変更しない | 2.4 |
| 6.2 | 当たり判定原点・バルーン追従基準を変更しない | 2.4 |
| 6.4 | 定常アロケーション 0 と段階別計時ログを壊さない | 2.4 |
| 6.5 | 壊れた既存テストを残さない | 2.4 |
| 6.6 | tick の門の既定を変更せず本採用しない | 2.4 |
| 6.7 | 起床旗と相名は読むだけ | 2.4 |
| 7.6 | 目視でしか判らない効果を決定論で合格判定しない（禁止条項） | 3.1 |
| **7.7** | **ワークスペース全体のテストが通る状態で完了する**（唯一生きている完了条件） | 1 / 2.2 |
| **8.1** | **未達の量を要件文書へ登記** | 3.2 |
| **8.2** | **引受先を実在の仕様として名指しするか、無いことを明示** | 3.2 |
| **8.3** | **上限・判定器・観測語彙を書き換えて未達を消さない** | 2.1 / 2.4 / 3.2 |
| **8.4** | **改訂を設計・境界節・steering まで追随させる** | 2.3 |
| **8.5** | **未達を残した完了は合格と読める書き方をしない** | 3.3 |
| 9.1 | 可視化の段を担当し tick の相順は担当しない | 2.3 |
| 9.3 | CPU 負荷とバルーン k 倍を担当しない | 2.3 |
| 9.5 | 上流アンカーの実測再確認と、ずれていれば記述更新 | 2.1 |

- **task 3.1 が名指しで確認するよう求めた 3 系統は、いずれも担当を持つ**——⑴ ワークスペース全体テスト＝**7.7**（task 1 が前提整備・task 2.2 が実行）⑵ 見え方の順序を未特定として扱うこと＝**5.1**（task 3.2）⑶ 登記の各条＝**8.1〜8.5**（8.1・8.2 が task 3.2、8.3 が task 2.1／2.4／3.2 の 3 本、8.4 が task 2.3、8.5 が task 3.3）。
- 8.3（未達を消す方向の書き換えの禁止）だけが**3 タスクに分散**している。これは偶然ではなく、上限・判定器・観測語彙の非接触が V4（task 2.1）、差分集合の非接触が V2（task 2.4）、登記の文面が task 3.2 と、**同じ禁止を 3 つの角度から押さえている**ためである。
- 4.6 は表 B に**含めていない**（突合⑴-3 の裁定により不適用側）。ただし仮に生存側に置いたとしても担当は task 3.1 であり（`_Requirements:` に 4.6 がある）、**どちらに分類しても「担当なし 0 件」の結論は変わらない**。

### 突合⑴-5: 対照——**この突合が赤を出せること**の確認

「食い違い 0 件」「担当なし 0 件」は、突合そのものが空振りしていても出る。両方の判定に既知の赤を 1 本ずつ当てた。

```
$ grep -v '^7\.3$' /tmp/des_na_strict.txt | { cat; echo "9.9"; } | sort -u > /tmp/des_na_fake.txt
$ comm -3 /tmp/req_na.txt /tmp/des_na_fake.txt
4.6
7.3
	9.9
$ comm -3 /tmp/req_na.txt /tmp/des_na_fake.txt | wc -l
3

$ grep -v '^5\.1$' /tmp/task_ids.txt > /tmp/task_ids_fake.txt
$ comm -23 /tmp/live_req.txt /tmp/task_ids_fake.txt
5.1
$ comm -23 /tmp/live_req.txt /tmp/task_ids_fake.txt | wc -l
1
```

- 集合差の突合は**片側の欠落（7.3）と片側の余剰（9.9）を両方とも検出**し、左右のどちらに寄っているかも `comm` の桁位置で区別できた（`9.9` だけ 1 タブ字下げ＝右側のみ）。
- 生存→担当の突合は**担当を 1 件抜くと ORPHAN として現れる**。
- なお本節では**偽の対照を作るまでもなく、実際に赤が 1 度出ている**——突合⑴-2 の素朴な抽出（23 件で一致）と厳密な抽出（22 件・差 1 件）の食い違いがそれであり、道具の側の欠陥を本番の判定より先に捕まえた。

### 突合⑴-6: 判定

- **観察可能な完了その 1 は充足**——不適用の受入基準 ID の一覧が表 A として残り（**23 件**）、requirements と design の**分類の食い違いは 0 件**である。集合の**表記**の差は 1 件（4.6・design トレーサビリティ表のマーカー欠落）だが、両文書は 4.6 について同じ読み（合格を宣言しないため発動しない）を逐語で述べており、**両文書が「この仕様が何であるか」について食い違ってはいない**。
- **観察可能な完了その 2 は充足**——生きている受入基準とその担当タスクの対応表が表 B として残り（**33 件**）、**担当タスクを持たない生存基準は 0 件**である。さらに全 56 件が漏れなくいずれかのタスクに紐づく（不適用側の担当なしも 0 件）。
- **合格宣言を前件とする条項は 4.6 の 1 件のみ**であり、両文書とも「合格を宣言しないため発動しない」と読める。合格に触れる他の 4 件のうち 3.1 は不適用、3.5・7.6・8.5 は**禁止条項として常時生きており**、本仕様の閉じ方（未達の登記）にそのまま効く。
- **不適用と未達は混ざっていない。** 未達 40 件は要件 2.1 に紐づく量として登記に残っており、2.1 を不適用に分類したことでその量が帳簿から消える形にはなっていない。
- **本節は `requirements.md`・`design.md`・`tasks.md` のいずれも 1 行も変更していない**（本タスクの編集対象は本記録のみ）。
- **後続への申し送り 1 件**: design のトレーサビリティ表**だけ**を機械で読むと 4.6 は生存側に分類される。4.6 の分類は `design.md:272` の「要件 4.6 の読み」段落まで読まないと確定しない。加えて、実現要素セルに文字列 `不適用` を含むかで抽出すると 4.6 が偽陽性で拾われ、**間違った抽出のほうが「一致」という結論を出す**。抽出はセル冒頭の `**不適用**` マーカーで行うこと。

---

## 登記の突合⑵: 裁定・却下理由・未達・手渡し（task 3.2）

実施日 2026-08-27。対象 HEAD `48c8a807`（`feat(areka-P0-present-write-coherence): 不適用と生存の受入基準の確定を検証 (task 3.1)`）。作業ツリー差分は `git status --porcelain` → ` M vendors/pasta` の 1 行のみ（V2-5・突合⑴ と同じ扱い）。

> **本節が塞ぐ誤読**: 「見送りの結論だけが残り、なぜ採らなかったのかと、何がどれだけ未達で誰も引き受けていないのかが、後から読めなくなる」形。本仕様群の履歴が最悪と記す形は **未達が仕様の内側から見えないこと**であり、本節はその番人である。
> **本節の限界**: 見ているのは ⑴ 却下理由が構造的事実と対で残っているか ⑵ その事実が現物のコードと一致するか ⑶ 要件文書と設計文書の数値が同じか ⑷ 手渡しの指す先が実在するか、の 4 点だけである。**裁定そのものの当否（見送りが正しかったか）は要件討議 #2 の開発者裁定であり、本節は再審しない。**
> **数の出所**: 本節に載せる値はすべて隣に採取コマンドを置く。文書に書かれた値を写して埋めていない。

### 突合⑵-0: 採取の基準点

| 項目 | 値 | 採取コマンド |
|---|---|---|
| 対象 HEAD | **`48c8a807`** | `git rev-parse HEAD` ／ `git log -1 --format='%h %s'` |
| 既定枝 `origin/main` | **`a6d27c73`** | `git rev-parse origin/main` |
| 枝名 | `claude/areka-p0-present-write-coherence-8308e6` | `git rev-parse --abbrev-ref HEAD` |
| 本枝が加えるコミット | **15 本** | `git log --oneline origin/main..HEAD \| wc -l` |
| 作業ツリー差分 | **1 行**（` M vendors/pasta`） | `git status --porcelain \| wc -l` |

> **本節自身の扱い**（V2 と同じ注記）: 本節を書き込む行為そのものが `verification/notes.md` に未コミット差分を 1 件作る。上の 1 行は書き込みより**前**の状態であり、書き込み後に数え直すと 2 行になる。許可集合の内側の変更であり、本タスクのコミットで解消する。

---

### 突合⑵-1: 4 候補の却下理由 ⇄ 構造的事実 ⇄ task 2.1 の実測

`design.md` Architecture「却下の登記（4 候補）」（`grep -n "却下の登記" design.md` → **2 行**＝`:109`（本節の見出し）と `:246`（C2 の所在列が本節を指す行）。データ行は `:113`〜`:116`）の 4 行それぞれについて、**却下理由**と**根拠となる構造的事実**の 2 つが揃っていること、およびその事実が**現物のコードと一致する**ことを検査した。

**⑴ 「理由」と「事実」が対で揃っているかの機械的検査**——表は 4 列（`#`／`候補`／`却下理由`／`根拠となる構造的事実`）で、後ろ 2 列が両方とも非空であることを確かめる:

```
$ awk -F'|' '/^\| R[1-4] \|/ {r=$4; f=$5; gsub(/^ +| +$/,"",r); gsub(/^ +| +$/,"",f);
             printf "%s 却下理由=%d 構造的事実=%d\n", $2, length(r), length(f)}' design.md
 R1  却下理由=81 構造的事実=196
 R2  却下理由=177 構造的事実=276
 R3  却下理由=272 構造的事実=601
 R4  却下理由=162 構造的事実=187
```

**4 行とも 2 列が非空**＝理由だけの行・事実だけの行は無い（上の数値は `awk` の `length()` が返す**バイト長**であって文字数ではない。ここで使うのは「0 でないこと」だけである）。

**⑵ 構造的事実の現物突合**（各事実を、設計文書からではなく**その場で現物を読んで**確かめた）:

| # | 候補 | 却下理由（design `:113`-`:116` の逐語要旨） | 根拠となる構造的事実 | **本節の現物実測** | task 2.1 の実測との一致 |
|---|---|---|---|---|---|
| R1 | **B-3（文言どおり）**＝可視化を窓書込の直前まで遅らせる | 上限 16,667µs に**構造的に届かない**。是正しても未達は残る | 窓書込の刻印は `EndDeferWindowPos` が戻った後。残る隙間の下限＝`flush_total_us` 143,231µs 以上＝上限の 8.6 倍以上 | `command.rs`: `flush()` `:724` → `begin` レコード `:742` → `apply_as_batch` 呼出 **`:757`**（その内側 **`:433`** が `EndDeferWindowPos(batch)`）→ **戻った後** `:776` `if observe {` ／ `:777` `for (index, cmd) …` ／ `:780` `stage: WriteStage::Flush,` → `end` レコード `:800`。**刻印は依然として `EndDeferWindowPos` の後**。下限比は `143231/16667 = 8.59` | **一致**（V4-d の T1「該当なし」と同じ現物・行番号も一致） |
| R2 | **B-3′**＝flush 完了後に可視化の反映を駆動する | 要件 1.2⑵「tick の駆動と窓書込 flush の駆動の関係を変更する」に**該当**＝大改造。要件 1.3 により候補から落ちる | 上限へ届く唯一の置き場が `EndDeferWindowPos` の後であり、そこへの駆動は flush の後ろに新しい駆動を足す行為。加えて wintf は areka を知らないため既定 None のコールバック口という前例の無い仕組みが要る | `tick_bridge.rs`: `world.try_tick_world()`（13 本スケジュール）が **`:246`**＝`world.try_borrow_mut()`（**`:241`**）の借用スコープの**内**。`crate::ecs::window::flush_window_pos_commands();` が **`:258`**＝借用スコープ（`match` 式が **`:256`** の `    };` で閉じる）の**外**。`:257` のコメント逐語＝`// World 借用スコープ終了後に SetWindowPos コマンドをフラッシュ（省略の回も必ず）。`／ **wintf → areka の依存は無い**——`grep -n "areka" crates/wintf/Cargo.toml` は 3 行返すがいずれも `#` で始まるコメント（非コメント行 **0**）、`grep -rn 'areka_' --include='*.rs' crates/wintf/src/` は **0 件** | **一致**（V4-d の T2「該当なし」と同じ現物） |
| R3 | **B-4**＝遷移中だけサーフェスを窓内下端中央へ置く補償 | 合否量 `visualize_to_write_us` を **1µs も動かせない**（要件 2.1 を原理的に満たせない）一方、3 つの新しい壊し方を持ち込む。切替頻度が稀で無視できる優先度に対し費用と釣り合わない | 判定量は可視化と書込の**時刻差**しか見ないが、B-4 はどちらの時刻にも触れない | **判定量の入力は時刻だけである**——`transition_judge.rs:817` = `.map(\|write_us\| write_us.saturating_sub(*visualize_us))`。左右の入力の出所は `:679`-`:680`（`last_write_us_per_window_frame` / `.insert((window, record.stamp.frame), record.stamp.t_us);`）と `:709`（`.push((record.stamp.frame, record.stamp.t_us));`）＝**どちらも `record.stamp.t_us`（刻印の時刻）だけ**であり、位置・寸・オフセットのフィールドは 1 つも入らない。合否束は `transition_judge_verdict.rs:169` `pub const fn signoff() -> Self {`（本体は `:170` の `Self {` から `:175` まで。**`:171`** `visualize_to_write_us_max: Some(VISUALIZE_TO_WRITE_US_MAX),` と **`:172`** `flush_total_us_max: Some(FLUSH_TOTAL_US_MAX),` で 2 量とも armed）。**位置の補償は入力に触れないので値を動かせない**／3 つの壊し方の前提も現物にある——`mount.rs:76` `offset: Offset { x: 0.0, y: 0.0 },`・`:244` `arr.offset = Offset { x: 0.0, y: 0.0 };`（常時ゼロ）、番兵テスト `presenter_upload_failure_tests.rs` は実在し `grep -c "offset"` → **17** | **一致**（V4-a の B1・B2 と同じ現物・行番号も一致） |
| R4 | **観測のみ先行**＝提示側の観測点を足して見え方の順序を初めて機械で名指しする | **いずれ直す前提が無い**ため採らない。観測点は是正の準備であって、是正しないなら費用だけが残る | 提示（合成器）を観測する仕組みはリポジトリに 1 つも無く、GPU 合成窓で「画面に出た時刻」を返す手段自体が未調査（`research.md` §8-①） | **本節が独自に走査した結果、提示時刻の観測は 1 件も無い**——`GetFrameStatistics` **0**／`DwmGetCompositionTimingInfo` **0**／`PresentCount` **0**／`SyncQPCTime` **0**／`GetLastPresentCount` **0**／`DXGI_FRAME_STATISTICS` **0**／`present_time` **0**／`PresentTime` **0**（いずれも `crates/` 全域・`--include='*.rs'`）。`CompositionTarget` は **4 件**だが内訳は pilot 例の `IDCompositionTarget` 3 件（`pilot-clickthrough-alpha-toggle/main.rs:33`・`:155`・`:475`）と `visual_sync.rs:238` のコメント 1 件＝**時刻を返す口ではない**。`.Present(` は **3 件**（`chain.rs:352`・`swapchain_spike.rs:151`・`surface.rs:314`）だが、いずれも**提示を行う呼出**であって提示時刻を**読む**呼出ではない。観測語彙の側も、`transition_diag.rs` の `kind=` は `enqueue`／`flush`／`monitor`／`msg`／`shell`／`write` の **6 語**（下の `grep -o 'kind=[a-z_]*' … | sort -u` は **7 行**返すが、1 行は語の付かない裸の `kind=`＝レコード書式の雛形側の出現であり、段の名前ではない）、面の側 `transition_record.rs:121` `pub(super) enum SurfaceStage {` は `Upload`／`Visualize`／`Skipped` の 3 段——**最後の観測はアプリ自身の可視化呼出であり、「画面に出た」を測る段は存在しない** | **一致**（`research.md` §8-①（`:255`）の「リポジトリに DWM 合成・提示の観測は 1 つも無い」を独立に再現） |

現物突合に用いた採取コマンド（すべてワークツリー直下で実行）:

```
sed -n '724p;742p;757p;433p;776,777p;780p;800p' crates/wintf/src/ecs/window/command.rs
grep -n "Instant::now\|elapsed_us"              crates/wintf/src/ecs/window/command.rs
sed -n '230,262p'                               crates/wintf/src/runtime/tick_bridge.rs
grep -n "areka" crates/wintf/Cargo.toml | grep -v "^[0-9]*:#"
grep -rn 'areka_' --include='*.rs' crates/wintf/src/ | wc -l
sed -n '808,824p'                               crates/areka/src/placement/transition_judge.rs
sed -n '679,685p;705,710p'                      crates/areka/src/placement/transition_judge.rs
sed -n '165,178p'                               crates/areka/src/placement/transition_judge_verdict.rs
grep -n "offset" crates/areka-emo-present/src/mount.rs
grep -c "offset" crates/areka-emo-present/src/presenter_upload_failure_tests.rs
for pat in GetFrameStatistics DwmGetCompositionTimingInfo PresentCount SyncQPCTime \
           GetLastPresentCount DXGI_FRAME_STATISTICS present_time PresentTime CompositionTarget ; do
  printf "%-28s %s\n" "$pat" "$(grep -rn "$pat" --include='*.rs' crates/ | wc -l)" ; done
grep -rn "\.Present(" --include='*.rs' crates/
grep -o 'kind=[a-z_]*' crates/wintf/src/ecs/window/transition_diag.rs | sort -u
sed -n '121,128p' crates/areka-emo-present/src/presenter/transition_record.rs
```

**⑶ 要件文書と設計文書が 4 候補について食い違っていないこと**

`requirements.md`「裁定の記録」**4**（`:40`）と **5**（`:41`）が裁定の正本である。design の R1〜R4 と行ごとに突き合わせた（採取＝`sed -n '40p;41p' requirements.md` ／ `sed -n '113,116p' design.md`）:

| 候補 | requirements 側（逐語の要旨・出所行） | design 側（出所行） | 食い違い |
|---|---|---|---|
| B-3（文言どおり） | `:40` — `窓書込の刻印は OS 呼出（EndDeferWindowPos）が戻った後に発行され、OS 呼出自体が 143,231µs 以上を占めるため、文言どおりの B-3（可視化を窓書込の直前へ）では上限 16,667µs に構造的に届かない` | `:113`（R1）— 同じ事実・同じ下限値 143,231µs | **なし** |
| B-3′ | `:40` — `上限へ届く唯一の形＝flush 完了後に可視化の反映を駆動する形（B-3′）は…要件 1.2⑵…に該当すると裁定。よって要件 1.3 により B-3（B-3′ を含む）は候補から落ちる` | `:114`（R2）— 同じ条項番号（1.2⑵）・同じ帰結（要件 1.3 で落ちる） | **なし** |
| B-4 | `:41` — `合否量 visualize_to_write_us を動かせず（…要件 2.1 を原理的に満たせない＝ギャップ分析 §5-①）` ＋ 3 つの壊し方を逐語列挙 ＋ `費用と釣り合わない` | `:115`（R3）— `1µs も動かせない`・同じ 3 つの壊し方・同じ費用対効果の理由 | **なし** |
| 観測のみ先行 | `:41` — `観測のみ先行（提示側観測点の追加）も採らない——いずれ直す前提が無いため` | `:116`（R4）— `いずれ直す前提が無いため採らない` | **なし** |

- **4 候補とも、却下理由が両文書で同一であり、構造的事実は現物と一致する。** 要件 1.1〜1.5 は登記として完結している。要件 1.6（上流の食い違いに対する読みの明記）は `requirements.md` 裁定の記録 1（`:37`）が「規模を先に測る対象は B-3・採否は 1.5 の裁定で一意に決める」と明記しており、design `:186`-`:187` のトレーサビリティが同じ読みを参照として保っている。
- **一点だけ粒度の差がある**（食い違いではない）: requirements 裁定の記録 5 は B-3 を 1 項目にまとめ「裁定の記録 4 のとおり要件 1.2⑵ に該当」と書くが、厳密には **1.2⑵ 該当は B-3′ の側**であり、文言どおりの B-3 は**構造的に届かない**ことで落ちている。裁定の記録 4（`:40`）はその 2 つを分けて書いており、design は R1／R2 として**分けて登記している**。すなわち design は requirements を細かくした形であって、逆向きの主張はしていない。この方向の粒度差は登記を弱めない（申し送りとして突合⑵-5 に残す）。

---

### 突合⑵-2: 数値の突合（要件文書 ⇄ 設計文書）

未達の登記に現れる値を、両文書から**機械で抜いて**突き合わせた。

**⑴ 登記表の行を、列ごとに正規化して差分を取る**

対象は `requirements.md` 末尾「未達の登記」節の表（`grep -n "^## 未達の登記" requirements.md` → **`:209`**・データ行 `:217`-`:218`）と `design.md`「未達の登記（設計側の反映）」の表（`grep -n "未達の登記（設計側の反映）" design.md` → **2 行**＝`:245`（C1 の所在列）と `:251`（本節の見出し）。データ行は `:257`-`:258`）。

```
$ norm() { sed 's/\*\*//g; s/ //g'; }
$ awk -F'|' '/^\| `(visualize_to_write_us|flush_total_us)` \|/ {print $2"|"$3"|"$4"|"$5}' requirements.md | norm
`visualize_to_write_us`|210,329…306,301µs（上限の12.6〜18.4倍）|16,667µs|32窓中32件（上限以下0件）
`flush_total_us`|143,231…231,910µs|16,667µs|8遷移中8件（もとより本仕様の合否外＝要件4.2）

$ awk -F'|' '/^\| `(visualize_to_write_us|flush_total_us)` \|/ {print $2"|"$3"|"$4"|"$5}' design.md | norm
`visualize_to_write_us`|210,329…306,301µs（上限の12.6〜18.4倍）|16,667µs|32窓中32件（上限以下0件）
`flush_total_us`|143,231…231,910µs|16,667µs|8遷移中8件
```

第 1 行は**逐語で完全一致**。第 2 行の差は `（もとより本仕様の合否外＝要件4.2）` の有無だけで、design はこれを**第 5 列（合否）**に持つ:

```
$ awk -F'|' '/^\| `flush_total_us` \|/ {print $6}' design.md
 もとより合否外（要件 4.2）
```

すなわち**同じ内容が別の列に置かれているだけ**であり、値の食い違いではない。

**⑵ 数値トークンだけを抜いた列の突合**（列の割り方の違いを消して比べる）:

```
$ nums() { awk -F'|' '/^\| `(visualize_to_write_us|flush_total_us)` \|/ {print $2"|"$3"|"$4"|"$5}' "$1" \
             | grep -oE '[0-9]+([,.][0-9]+)*' | tr '\n' ' '; echo; }
$ nums requirements.md
210,329 306,301 12.6 18.4 16,667 32 32 0 143,231 231,910 16,667 8 8 4.2
$ nums design.md
210,329 306,301 12.6 18.4 16,667 32 32 0 143,231 231,910 16,667 8 8
```

**数値の並びは、末尾の `4.2`（＝要件番号であって量ではない・design では第 5 列にある）を除いて完全一致。**

**⑶ 項目別の突合表**（採取＝`sed -n '217p;218p;220p;224p' requirements.md` ／ `sed -n '257p;258p;260p;261p' design.md`）

| 項目 | 要件文書の値（出所） | 設計文書の値（出所） | 一致 |
|---|---|---|---|
| `visualize_to_write_us` の実測域 | **210,329 … 306,301µs**（`:217`） | **210,329 … 306,301µs**（`:257`） | **一致** |
| 同・上限との比 | **上限の 12.6〜18.4 倍**（`:217`） | **上限の 12.6〜18.4 倍**（`:257`） | **一致** |
| 同・上限 | **16,667µs**（`:217`） | **16,667µs**（`:257`） | **一致** |
| 同・違反件数 | **32 窓中 32 件**（上限以下 0 件）（`:217`） | **32 窓中 32 件**（上限以下 0 件）（`:257`） | **一致** |
| `flush_total_us` の実測域 | **143,231 … 231,910µs**（`:218`） | **143,231 … 231,910µs**（`:258`） | **一致** |
| 同・上限 | **16,667µs**（`:218`） | **16,667µs**（`:258`） | **一致** |
| 同・違反件数 | **8 遷移中 8 件**（`:218`） | **8 遷移中 8 件**（`:258`） | **一致** |
| 同・合否上の位置づけ | `もとより本仕様の合否外＝要件 4.2`（`:218` 第 4 列） | `もとより合否外（要件 4.2）`（`:258` 第 5 列） | **一致**（列位置のみ差） |
| 違反の合計 | **計 40 件**（`:220`） | **計 40 件**（`:260`） | **一致** |
| 引受先 | **引受先なし**（`:224`・要件 8.2 の明示） | **引受先なし**（`:261`・要件 8.2 の明示） | **一致** |
| 引受先でない仕様の名指し | `tick-gate-adoption` は tick の門の本採用が担当であり本量は担当しない（`:224`） | 同趣旨の同文（`:261`） | **一致** |
| 症状の持続 | `最大 0.3 秒程度`（`:220`） | `最大 0.3 秒程度`（`:260`） | **一致** |

**⑷ トークン出現数の交差確認**（表の外の本文にも同じ値が書かれているので、文書全体で数える）:

```
$ for tok in "210,329" "306,301" "12.6" "18.4" "16,667" "32 窓中 32 件" "143,231" "231,910" \
             "8 遷移中 8 件" "40 件" "引受先なし"; do
    printf "%-16s req=%s des=%s\n" "$tok" "$(grep -c -- "$tok" requirements.md)" "$(grep -c -- "$tok" design.md)"; done
210,329          req=3 des=1
306,301          req=3 des=1
12.6             req=3 des=1
18.4             req=3 des=1
16,667           req=10 des=6
32 窓中 32 件    req=1 des=1
143,231          req=3 des=3
231,910          req=2 des=1
8 遷移中 8 件    req=1 des=1
40 件            req=3 des=8
引受先なし       req=1 des=7
```

**11 トークンすべてが両文書に 1 件以上ある。** 出現**回数**は文書の役割が違うので一致を求めない——requirements は起点の実測表（`:23`-`:29`）と登記表（`:215`-`:218`）の 2 箇所で書き、design は登記を 1 箇所に集めたうえで却下理由・完了報告の規律から繰り返し参照する。

**⑸ 上限 16,667µs が現物の定数と一致すること**

```
$ ls -l crates/areka/src/placement/transition_judge_verdict.rs
-rw-r--r-- … 40387 … crates/areka/src/placement/transition_judge_verdict.rs
$ grep -rn "16_667" --include='*.rs' crates/
crates/areka/src/placement/transition_judge_verdict.rs:90:pub const VISUALIZE_TO_WRITE_US_MAX: u64 = 16_667;
crates/areka/src/placement/transition_judge_verdict.rs:99:pub const FLUSH_TOTAL_US_MAX: u64 = 16_667;
```

- **リポジトリ全域で `16_667` は 2 箇所のみ**であり、両方とも本仕様が登記する上限と同値。**文書の 16,667µs は現物の定数と一致している。**
- 2 量とも同じ上限を当てることも現物どおり（`Bounds::signoff()` `:169`-`:175` が両方に `Some(...)` を入れる＝V4-a の B2）。

**⑹ 比と件数が算術で再導出できること**（値を写しただけでないことの裏取り）:

```
$ awk 'BEGIN{printf "210329/16667=%.2f  306301/16667=%.2f  143231/16667=%.2f  231910/16667=%.2f\n",
        210329/16667, 306301/16667, 143231/16667, 231910/16667}'
210329/16667=12.62  306301/16667=18.38  143231/16667=8.59  231910/16667=13.91
```

- `12.62`／`18.38` は登記の **12.6〜18.4 倍**と一致する。
- `flush_total_us` の**最小値 143,231µs が既に上限 16,667µs を超えている**ので、**8 遷移すべてが違反＝8/8** は算術から出る（件数を数え直さなくても成立する）。
- `143231/16667 = 8.59` は R1 の構造的事実が言う **8.6 倍以上**と一致する。
- 合計 **32 ＋ 8 = 40 件**。

**⑺ 数量の正本（atom 確定台帳 §11）との突合**

両文書は数量の正本を atom 確定台帳 `mechanism-ledger.md` §11 と名指す（`grep -n "§11" requirements.md` → `:21`・`:220`／同 design.md → `:52`・`:321`）。**その正本が実在し、同じ値を持つことを確かめた**:

```
$ ls -l .kiro/specs/completed/areka-P0-dpi-transition-atomicity/mechanism-ledger.md
-rw-r--r-- … 129004 …（実在）
$ grep -n "^## 11\." .kiro/specs/completed/areka-P0-dpi-transition-atomicity/mechanism-ledger.md
549:## 11. 判定に載らない症状（task 7.3・2026-08-22）
$ grep -n "210,329\|306,301\|143,231\|231,910" .kiro/specs/completed/areka-P0-dpi-transition-atomicity/mechanism-ledger.md
579:| **その隙間**（＝`visualize_to_write_us`） | … | **210,329〜306,301µs**（上限 16,667µs の **12.6〜18.4 倍**） |
602:> … 下限は **210,329µs**（遷移 #7・`scope=1 char`）。倍率は **12.6〜18.4 倍** |
652:| 一括書込の総所要（`flush stage=end` の `total_us`） | … | 143,231〜231,910µs（平均 **188,711**…） | …
654:| 窓ごとの `visualize_to_write_us` | … | 210,329〜306,301µs（平均 **255,345**・32 件） | …
661:- **ゆえに C8 の梯子は次段（B-4）の条件を満たしたままである**——…現況 max は **306,301µs＝上限の 18.4 倍**。…梯子は引受先 `areka-P0-present-write-coherence` が続ける。
```

（**該当は 5 行**——`grep -c` で 5 を確認済み。各行の内側は `…` で省略しているが、**行そのものは 1 本も落としていない**。とりわけ `:661` は登記上重要である：atom は C8 の梯子の次段を**本仕様へ引き渡した**と書いており、その本仕様が見送りを裁定した結果、**梯子の引受先が居なくなった**——これが「引受先なし」の来歴そのものである。）

台帳 `:581` は `- **上限以下で終わった窓は 32 窓中 1 つも無い。**`、`:624` は `signoff の visualize_to_write_us は FAIL……（40 件の違反のうち 32 件がこれ）`、`:643` は `SIGNOFF-BOUNDS: FAIL (違反 40 件)` を持つ（採取＝`sed -n '581p;624p;643p' …`）。**実測域・比・32 件・40 件のいずれも正本と一致する。**

**判定: 要件文書と設計文書の数値の食い違いは 0 件。** 差は「`flush_total_us` の合否外という註記が第 4 列にあるか第 5 列にあるか」という**列位置の差 1 件**のみで、値・件数・上限・比・引受先の有無はすべて同一である。

---

### 突合⑵-3: 将来仕様への手渡し（3 点）の実在確認

`design.md`「将来仕様への手渡し（再着手の前提）」（`grep -n "将来仕様への手渡し" design.md` → **6 行**＝`:40`（境界節 This Spec Owns）・`:65`（Revalidation Triggers が罠②を指す行）・`:107`（Key Decisions D7）・`:175`（System Flows の mermaid ノード）・`:249`（C5 の所在列）・**`:274`（本体節の見出し）**）の本体節 `:274`-`:282` を検査した。

| # | 手渡し | 逐語（出所） | 実在／正確さの証明 | 判定 |
|---|---|---|---|---|
| ⑴ | **素材の所在** | `:278` — `research.md が規模見積りの正本である。§1（現状構造の実測と隙間の内訳）・§3（B-3 の接触集合・破る順序制約 6 本・追加が必要な仕組み 5 点）・§5（判定器の落とし穴 3 件）・§6（影響を受ける既存テスト 5 件）・§7（工数と危険度）・§8（未調査 5 件）` | **名指しされた 6 節がすべて実在**（下の節番号突合）。加えて **`research.md` を指す参照は両文書とも 1 つも空振りしない** | **揃っている** |
| ⑵-① | **罠①（判定量の飽和）** | `:279` — `visualize_to_write_us は同一フレーム内の write_us - visualize_us を飽和減算で求める（crates/areka/src/placement/transition_judge.rs:817）。可視化を書込の後ろへ動かすと値は 0＝満点になる。これは未測定でも違反でもなく満点として通る……置き場を決めるより先に、合格の意味をどう保つかを裁定すること` | **現物と一致**——`sed -n '817p' crates/areka/src/placement/transition_judge.rs` → `                    .map(\|write_us\| write_us.saturating_sub(*visualize_us))`。飽和減算であり、`visualize_us > write_us` なら **0** を返す。値が 0 になれば上限 16,667µs を下回るので**違反として立たず、未測定にも落ちない**（未測定は同一フレームに書込が無いときの経路＝`transition_judge_verdict.rs` の `Unmeasured` 側）。**「満点として通る」という主張は現物の意味論どおり** | **揃っている・主張も正確** |
| ⑵-② | **罠②（`t_us` の起点）** | `:280` — `t_us はプロセス開始からではなく当該 tick の開始からの経過である（crates/wintf/src/ecs/window/transition_diag.rs:692・:703）。起点実測の 22,297〜339,998µs はすべて 1 つの tick の内側で起きており……この読み替えを持たずに見積もると規模が 1 桁ずれる` | **現物と一致**——`:692` = `pub fn since_tick_start_us() -> u64 {`（直前の doc `:689` = `/// tick 開始からの経過（µs・**World を借りられない観測点専用**）。`・本体は `TICK_MIRROR` を読んで `elapsed_us(start)` を返す）、`:703` = `pub fn stamp() -> Stamp {`（`:705`-`:706` = `frame: current_frame(),` ／ `t_us: since_tick_start_us(),`）。**刻印の `t_us` が tick 起点であることは組み立て箇所で確定する**——プロセス起点の時刻を入れる経路は無い | **揃っている・主張も正確** |
| ⑶ | **未調査で残る 1 点** | `:281` — `EndDeferWindowPos 単独の所要は台帳に無い（OS 側の内訳は未特定）。research.md §8-② のとおり、その直前・直後に計時点を足せば確定でき、この 1 点の実測が B-3′ の到達可否を確定させる` | **「未調査」であることを現物で確かめた**——`grep -n "Instant::now\|elapsed_us" crates/wintf/src/ecs/window/command.rs` は **6 行＝3 対**（`:397`/`:413`＝`DeferWindowPos` **投入だけ**の所要 `call_us`、`:460`/`:474`＝縮退 `SetWindowPos` 経路、`:738`/`:803`＝flush **区間全体**の `total_us`）。**`EndDeferWindowPos(batch)`（`:433`）を挟む計時対は 1 つも無い**＝単独所要は現に測られていない。／確定手段の所在も実在——`research.md` §8-②（`:257`）が `EndDeferWindowPos の直前・直後に計時点を足せば確定できる` と書いている | **未知として登記済み・確定手段の所在も実在** |

**節番号の突合（実在しない節を指していないこと）**——task 2.1 が見つけた「実在しない `presenter/mount.rs` を指す」欠陥と同型の欠陥を機械で排除する:

```
$ grep -nE '^## [0-9]+\.' research.md
8:## 0. 要旨（5 点）
18:## 1. 現状の構造（実測）
76:## 2. 要件→資産マップ
99:## 3. B-3 の規模見積り（Requirement 1.1 の素材）
150:## 4. 実装アプローチの選択肢
198:## 5. 判定器の落とし穴（設計討議へ必ず上げる）
225:## 6. 既存の決定論テストへの影響（Requirement 6.5 の素材）
241:## 7. Effort / Risk
253:## 8. Research Needed（設計フェーズへ持ち越す）
267:## 9. 設計判断項目（要件ディスカッションへ）
282:## 10. 設計フェーズの記録（2026-08-27・`design.md` 生成）

$ grep -oE '^## ([0-9]+)\.' research.md | grep -oE '[0-9]+' | sort -n | tr '\n' ' '
0 1 2 3 4 5 6 7 8 9 10
$ grep -oE '§[0-9]+' design.md       | grep -oE '[0-9]+' | sort -un | tr '\n' ' '
0 1 3 5 6 7 8 9 10 11
$ grep -oE '§[0-9]+' requirements.md | grep -oE '[0-9]+' | sort -un | tr '\n' ' '
0 1 3 5 11
```

- design が名指す節番号は **{0,1,3,5,6,7,8,9,10,11}**、requirements は **{0,1,3,5,11}**。このうち **§11 は `research.md` ではなく atom 確定台帳を指す**参照である（`grep -n "§11" design.md` → `:52` `atom 確定台帳 §11（数量の正本）`・`:321` `mechanism-ledger.md §11（数量の正本）`／同 requirements.md → `:21` `atom 確定台帳 mechanism-ledger.md §11`・`:220` `正本＝atom 確定台帳 §11`）。**§11 を除けば、両文書が `research.md` に対して名指す節はすべて実在する（空振り 0 件）。**
- design `:5` の `ギャップ分析 §0〜§9` という**範囲指定**も、実在集合が `0`〜`10` の連番なので**穴が無い**（上の連番出力がそれを示す）。
- **requirements（§1・§3・§5）と design（§1・§3・§5・§6・§7・§8）の差は包含関係であり、食い違いではない**——design が素材の所在をより広く渡している。狭い側が広い側と矛盾する主張をしている箇所は無い。

**判定: 手渡し 3 点はすべて揃っており、指す先はすべて実在し、2 つの罠は現物の意味論と一致する。** 罠①・罠②はいずれも「知らずに着手すると、直っていないのに PASS が出る／規模を 1 桁誤る」種類の事実であり、**単に書かれているだけでなく、書かれている内容が正しい**ことをその場で現物から確かめた。

---

### 突合⑵-4: 上限・判定器・観測語彙のいずれも書き換えられていないこと（要件 8.3・3.4・5.4）

task 2.1（V4）・task 2.4（V2）の結果を本節の基準点 HEAD `48c8a807` で**採り直した**（あの採取以降にコミットが増えているため、写さずに採り直す）。

**⑴ pathspec の実在確認（差分採取の直前に実行）**

```
$ for p in crates/areka/src/placement/transition_judge.rs \
           crates/areka/src/placement/transition_judge_verdict.rs \
           crates/wintf/src/ecs/window/transition_diag.rs \
           crates/wintf/src/ecs/window/command.rs \
           crates/wintf/src/runtime/tick_bridge.rs ; do
    [ -f "$p" ] && echo "EXISTS  $p" || echo "MISSING $p" ; done
EXISTS  crates/areka/src/placement/transition_judge.rs
EXISTS  crates/areka/src/placement/transition_judge_verdict.rs
EXISTS  crates/wintf/src/ecs/window/transition_diag.rs
EXISTS  crates/wintf/src/ecs/window/command.rs
EXISTS  crates/wintf/src/runtime/tick_bridge.rs
$ ls -1 crates/areka/src/placement/transition_judge*.rs | wc -l
9
```

**⑵ 差分の採取**

```
$ git diff --name-only origin/main...HEAD -- crates/ | wc -l
0

$ git diff --name-only origin/main...HEAD
.kiro/specs/areka-P0-present-write-coherence/design-validation.md
.kiro/specs/areka-P0-present-write-coherence/design.md
.kiro/specs/areka-P0-present-write-coherence/requirements.md
.kiro/specs/areka-P0-present-write-coherence/research.md
.kiro/specs/areka-P0-present-write-coherence/spec.json
.kiro/specs/areka-P0-present-write-coherence/tasks.md
.kiro/specs/areka-P0-present-write-coherence/verification/notes.md
.kiro/steering/roadmap.md

$ git diff --name-only origin/main...HEAD -- $(ls -1 crates/areka/src/placement/transition_judge*.rs | tr '\n' ' ') | wc -l
0
$ git diff --name-only origin/main...HEAD -- crates/wintf/src/ecs/window/transition_diag.rs | wc -l
0
$ git diff --name-only origin/main...HEAD -- crates/areka/src/placement/transition_judge_verdict.rs | wc -l
0
```

| 対象 | 実在 | 差分 | 対応する受入基準 |
|---|---|---|---|
| **上限** `VISUALIZE_TO_WRITE_US_MAX` `:90` ／ `FLUSH_TOTAL_US_MAX` `:99`（ともに `= 16_667;`） | EXISTS（`transition_judge_verdict.rs`） | **0** | 3.4・8.3 |
| **判定器** `transition_judge*.rs`（**9 ファイルを列挙して渡す**） | 9 ファイルへ展開 | **0** | 8.3 |
| **観測語彙** `transition_diag.rs` | EXISTS | **0** | 5.4・8.3 |
| `crates/` 全体（上 3 者を包含する検査） | ディレクトリ実在 | **0** | 6.1-6.7 |

**⑶ 対照——この検査が赤を出せること**

```
$ git diff --stat origin/main...HEAD -- .kiro/specs/areka-P0-present-write-coherence/design.md
 .../areka-P0-present-write-coherence/design.md     | 322 +++++++++++++++++++++
 1 file changed, 322 insertions(+)

$ git diff --stat origin/main...HEAD -- crates/areka/src/placement/transition_judge_NONEXISTENT.rs
（出力なし）
$ echo $?
0
$ ls crates/areka/src/placement/transition_judge_NONEXISTENT.rs
ls: cannot access 'crates/areka/src/placement/transition_judge_NONEXISTENT.rs': No such file or directory
```

**陽性対照は非空を返し、空振り対照は空・終了コード 0 を返す。** ⑴ の実在確認を省いていればこの 2 つは見分けがつかなかった——上表の 0 は、**渡した pathspec が現に解決することを確かめた上での 0** である。

**⑷ 上限の値そのものが動いていないこと**（差分 0 だけでは「元から緩かった」可能性を排除できないので、値を直接読む）

```
$ grep -rn "16_667" --include='*.rs' crates/
crates/areka/src/placement/transition_judge_verdict.rs:90:pub const VISUALIZE_TO_WRITE_US_MAX: u64 = 16_667;
crates/areka/src/placement/transition_judge_verdict.rs:99:pub const FLUSH_TOTAL_US_MAX: u64 = 16_667;
```

**判定: 上限・判定器・観測語彙のいずれも書き換えられていない。** 要件 8.3（上限・判定器・観測語彙を書き換えることで未達を消さない）・3.4（上限を緩めない）・5.4（レコード語彙の文言とフィールド名を変更しない）は、**現物の差分 0 ＋ 定数の実値**で成立している。未達 40 件は判定の側を動かして消されたのではなく、**登記として残されている**。

---

### 突合⑵-5: 判定

- **⑴ 4 候補の却下理由は、いずれも構造的事実と対で残っている**（`awk` による 2 列非空検査＝4/4）。その事実は本節が現物を読んで確かめた結果と一致し、**task 2.1 の実測（V3・V4）とも一致する**——R1 は V4-d の T1、R2 は同 T2、R3 は V4-a の B1・B2 と同じ行番号・同じ字面である。R4 は本節が独立に走査して「提示時刻の観測は 1 件も無い」を再現した。**要件文書と設計文書が 4 候補について食い違う箇所は 0 件。**
- **⑵ 要件文書と設計文書の数値の食い違いは 0 件。** 実測域・上限との比・上限・違反件数（**32 窓中 32 件**／**8 遷移中 8 件**）・合計 **40 件**・**引受先なし**——すべて同一である。差は註記の列位置 1 件のみ。比と件数は算術で再導出でき（`12.62`／`18.38`／`8.59`／最小値が上限超ゆえ 8/8）、数量の正本（atom 確定台帳 §11）とも一致する。上限 16,667µs は現物の定数 2 本（`transition_judge_verdict.rs:90`・`:99`）と一致する。
- **⑶ 将来仕様への手渡しは 3 点とも揃っている。** 素材の所在（`research.md` §1・§3・§5・§6・§7・§8＝**全節が実在・空振り 0 件**）・2 つの罠（飽和減算 `transition_judge.rs:817`／`t_us` の tick 起点 `transition_diag.rs:692`・`:703`＝**どちらも現物の意味論と一致**）・未調査の 1 点（`EndDeferWindowPos` 単独所要＝**計時対が現に無いことを確かめた**上で、確定手段の所在 `research.md` §8-② が実在）。
- **⑷ 上限・判定器・観測語彙のいずれも書き換えられていない**（実在を確かめた上での差分 0・対照は赤を出す・定数の実値は `16_667` のまま）。
- **本節は `requirements.md`・`design.md`・`research.md`・`tasks.md` のいずれも 1 行も変更していない**（本タスクの編集対象は本記録のみ）。

**この判定の射程（混ぜてはならない）**——ここで確定するのは「**裁定・却下理由・未達・手渡しの登記が、両文書のあいだで矛盾なく完結している**」ことだけである。**未達 40 件が解消した証拠ではない。** 本仕様は是正コード 0 行であり、`visualize_to_write_us` は上限の 12.6〜18.4 倍のまま、`flush_total_us` は 8 遷移すべてが上限超のまま、**引受先は無い**。既存のサインオフ判定がこの 2 量の違反を報告し続けるのは正常な状態である（要件 8.3）。**完了報告でこの「食い違い 0 件」を合否量と同じ息で並べてはならない**（要件 8.5）。

**後続への申し送り 1 件**: `requirements.md` 裁定の記録 5（`:41`）は B-3 と B-3′ を 1 項目にまとめて「要件 1.2⑵ に該当」と書く。**厳密には 1.2⑵ 該当は B-3′ の側**であり、文言どおりの B-3 は構造的非到達で落ちている（裁定の記録 4（`:40`）と design の R1／R2 はこれを分けて書いている）。将来 `requirements.md` の裁定の記録 5 **だけ**を読むと「文言どおりの B-3 も大改造だから落ちた」と読める——**却下の正確な内訳は design `:113`-`:114` の R1／R2 を読むこと。** 突合⑴-6 の申し送り（design のトレーサビリティ表だけを読むと 4.6 が生存側に見える）と対をなす、**片方の文書だけを読むと分類が粗くなる**形の 2 件目である。

**後続への申し送り 2 件目**: 手渡しの素材の所在は design の中で **2 箇所**にあり、名指しする節が食い違う——本体（`design.md:278`）は `research.md` §1・§3・§5・**§6・§7・§8** を挙げるが、C5 のトレーサビリティ行（`design.md:249`）は §1・§3・§5・**§10** を挙げる。**どちらの節も実在するので空振りではない**（`research.md` は §0〜§10 が連番で存在）。ただし C5 の行だけを見て辿った読み手は、工数と危険度（§7）・影響を受ける既存テスト（§6）・未調査事項（§8）ではなく設計フェーズの合成結果（§10）に着く。**再着手仕様は本体の `:278` の並びを正とすること。**
