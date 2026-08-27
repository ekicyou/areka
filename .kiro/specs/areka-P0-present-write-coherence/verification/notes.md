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

> **未実施。task 2.2 が本節を埋める。** 前提（i686 host-32 成果物）は task 1 で整備済み。
> 記載すべき内容: 実行コマンド・全体の通過数・バイナリ数・`test result: ok` の有無。赤が出た場合は設計 Error Handling 第 1 項の手順（前提確認 → 原因ファイルを名指し → 許可集合の外であることを file 単位で提示）。

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

> **未実施。task 2.3 が本節を埋める。**
> 記載すべき内容: `.kiro/steering/roadmap.md` のゴール表・W6.95 の編成行・干渉台帳の同居ペア行の 3 箇所が見送りの裁定を反映していることの現物確認（行番号は**その場で実測すること**。設計文書に記載の `:67`／`:82`／`:89` を写さない）。干渉台帳の pwc⇄bod の要ウォッチが B-4 却下により解消と登記されていることの確認。担当範囲と非担当範囲の切れ目が編成表と設計の境界節で一致していることの確認。
