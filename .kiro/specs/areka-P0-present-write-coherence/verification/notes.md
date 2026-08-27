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

> **未実施。task 2.4 が本節を埋める。** 本節は仕様文書の変更をすべてコミットして作業ツリーを確定させた**後**に採取する（本文書自身のコミットを含む）。
> 記載すべき内容: `git diff --name-only origin/main...HEAD` の全行・許可集合外 0 件の判定・接触禁止集合 7 項目の差分 0 の判定・`git status --porcelain` による想定外の作業ツリー差分 0 件の判定。
> **採取の必須手順**: 接触禁止集合の 7 項目は、**各パスが現物として解決することを先に確かめてから**採取すること。git は実在しないパスに対して空出力・終了コード 0 を返すため、確かめずに採った「差分なし」は空振りと区別できない（task 2.1 で実際に 1 項目がこの状態だった。V4-e 参照）。
> なお本節の採取に先立つ暫定確認は V4 の「差分の不在」に記録してある（task 2.1 時点のもので、証跡としては V2 の採取値が正本になる）。

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
