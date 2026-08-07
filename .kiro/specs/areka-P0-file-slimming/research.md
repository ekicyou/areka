# Gap Analysis: areka-P0-file-slimming

> 実施 2026-08-07（**第 2 版・スコープ拡大に伴う全面再計測**）/ 対象ブランチ `claude/areka-p0-file-slimming-64d065`（`a1f0f31`・作業ツリー clean）
> 入力: `requirements.md`（確定済・本書は改変しない。**対象がリポジトリ全 Rust ソースへ拡大された版**）／`brief.md`／`.kiro/steering/{product,tech,structure,workflow,logging,focus}.md`
> 計測は本ブランチの **`target/` と `.claude/worktrees/` を除いたリポジトリ配下の `*.rs` 全数**（`#[cfg(test)]` ブロックの波括弧を、文字列・raw 文字列・文字リテラル・行/ブロックコメントを除外して追跡する自前スキャナ）で再取得した。
>
> **第 1 版（`crates/*/src/**/*.rs` 限定・387 ファイル）からの差分**は §0 に集約した。第 1 版で確定済みの `src/` 所見は本書へ全て引き継いでいる（数値はいずれも再計測で完全一致）。
>
> **動機の枠組み**: 本 spec の動機は「**1 ファイルの行数が大きすぎることそのもの**」である（開発者裁定・requirements Project Description）。アンカードリフトを主因とする旧説明は要件側で既に是正済みであり、本書もその枠組みで書かれている。

---

## 0. スコープ拡大で何が変わったか（第 1 版との差分・要約）

| # | 変化 | 第 1 版の記述 | 第 2 版の実測 | 影響 |
|---|---|---|---|---|
| 0.1 | **母数** | `crates/*/src/` 387 ファイル / 189,190 行 / テストモジュール 92,868 | **全域 619 ファイル / 257,134 行 / テストモジュール 99,103** | 対象母数は +232 ファイル・+67,944 行だが、**テストモジュールは +6,235 行（+6.7%）しか増えない** |
| 0.2 | **必須対象（テストモジュール > 500 行）** | 48 本 / テストモジュール 66,830 | **49 本 / テストモジュール 68,921** | 増分は **1 本のみ**＝`crates/areka-ghost/tests/ghost/spine_e2e_test.rs`（総 2,574 / 本体 483 / テストモジュール 2,091 / 10 テストモジュール）。**48 本の src 一覧は完全に不変**（要件 1.2 の「最低でも 48 本を包含する」を満たす） |
| 0.3 | **`benches/` と `build.rs`** | 未評価 | **リポジトリに 1 つも存在しない**（`find` 全数・`Cargo.toml` に `[[bench]]` 宣言も 0） | 要件 1.1/1.2 が列挙する 5 種のうち 2 種は**空集合**。design で「該当なし」と明記すれば足りる |
| 0.4 | **`#[path]` の前例** | 「`crates/*/src/` に 0 件」＝案 C は新規規約導入 | 同左だが、**`crates/*/tests/` では `#[path]` が既存の唯一の標準**（属性実測 115 箇所＝wintf 67・dola 30・kanade 10・ghost 8）。steering `structure.md` L135 が「`tests/{domain}.rs` — `#[path]` による `mod` 宣言のみ」と明文化済 | **案 C の評価が反転しうる**。スコープ拡大により `#[path]` は「リポジトリに前例のない形式」ではなく「`src/` に未導入なだけの既定形式」になった（§4・設計判断 #1） |
| 0.5 | **`#[path]` 読込ファイルの子モジュール解決規則** | 未評価 | **実測（rustc 1.97.1）: `#[path]` で読み込まれたファイルの子モジュールは、そのファイル自身のディレクトリに解決される**（`tests/ghost/spine_e2e_test.rs` の `mod tests;` は `tests/ghost/tests.rs` を探し、`tests/ghost/spine_e2e_test/tests.rs` は探さない＝E0583） | **案 A が `tests/` で単独では成立しない**（明示 `#[path]` の併記が必須＝「単一方式」が崩れる）。§2.3.3・§4 |
| 0.6 | **テスト証跡の被覆** | `cargo test --workspace -- --list` で名前集合比較 | **`examples/` のテストモジュールは `--list` に一切現れない**（実測）。`--all-targets` を付ければ現れるが**doctest が落ちる**（実測） | 要件 2.2/2.3 の証跡は **3 本立て**（既定 `--list` ／ `--all-targets --list` ／ `build --all-targets`）が要る（§2.10・設計判断 #6） |
| 0.7 | **`cargo test -p areka --examples` は既に赤** | 未評価 | **main（`247d48a`）で再現確認済**: `E0433: cannot find 'input_events' in 'crate'` @ `crates/areka/examples/../src/placement/spawn.rs:879`（＝`spawn.rs` の**テストモジュールの中**・テストモジュールは :427 開始）。`spawn.rs:871` に「本テストは `#[cfg(test)]` ゆえ `crate::` パス使用可（example の `#[path]` include 不変条件は非テストコード限定）」という**明示の先行裁定**が残る | **本 spec 以前からの既存状態**。移設の前後で変わらない（移設してもテストモジュールは同じ条件で取り込まれる）。ただし要件 2.2 の証跡手順を `--all-targets` 一本にはできない（§2.10・R-9） |
| 0.8 | **新たな本番本体分割候補** | follow.rs / frame.rs の 2 本 | **増えない**。`src/` で本番本体 ≥ 1,000 行は follow.rs(1,996)・frame.rs(1,497) の 2 本のみ（3 位は presenter.rs の 1,042）。それ以外の 1,000 行超は全て**テストコード**（`spine.rs` 2,503・`decode_tests.rs` 1,395・`golden_tests.rs` 1,356＝いずれも要件 1.4 の除外対象、および `tests/`・`examples/` のファイル） | 要件 4.1/4.5 の 2 本限定は実測に忠実。§2.12 |
| 0.9 | **テストコードの位置構造** | src 全域で「テストモジュールの後に本番コード」は `wintf/src/ecs/world/mod.rs` の 1 件のみ | **全域（619 ファイル）でも同じ 1 件のみ**。`spine_e2e_test.rs` の 10 テストモジュールの“間”は**全てコメントバナー**であり、コードではない | 「全テストモジュールが末尾に連続」という不変条件はリポジトリ全域で成立。移設は末尾切り出しに還元できる |
| 0.10 | **非 `mod` `#[cfg(test)]` 項目** | 44 件 | **40 件・全て `src/`**（`tests/`・`examples/` に 0 件）。第 1 版の 44 は `pub(crate) mod` 4 件を誤って非 `mod` 側へ数えていた分の過大計上 | 要件 1.6 の対象範囲はスコープ拡大で**増えない**。§2.5 |

---

## 1. Analysis Summary

- **スコープ拡大は「母数」を大きくしたが「作業量」はほぼ変えない。** 全域 619 ファイル 257,134 行のうちテストモジュールは 99,103 行（38.5%）だが、そのうち **93.7%（92,868 行）は依然として `src/`** にある。テストモジュール 500 行超の必須対象は **48 → 49 本**（+1）にしかならず、増分は `crates/areka-ghost/tests/ghost/spine_e2e_test.rs`（テストモジュール 2,091）1 本だけである。`benches/`・`build.rs` は**リポジトリに存在しない**。
- **拡大が実際に変えたのは「移設方式の裁定材料」である。** `crates/*/tests/` は steering `structure.md` L133-140 により **`#[path]` による `mod` 宣言のみのエントリポイント**という規約で運用されており、`#[path]` 属性は 115 箇所 / 16 ファイルで使われている。第 1 版が案 C（`#[path]` フラット兄弟）に付けた「リポジトリに前例ゼロの新規規約」という減点は、**`src/` 限定の話に縮む**。逆に案 A（`foo/tests.rs` を素の `mod tests;` で引く）は、`#[path]` で読み込まれたファイルでは**素の `mod` では引けない**（子モジュールはそのファイル自身のディレクトリに解決される・実測 E0583）ため、`tests/` 側では明示 `#[path]` の併記が必須になり、要件 3.1 の「単一方式」が崩れる。
- **テスト証跡の被覆に穴が開いた。** `examples/` のテストモジュール（798 行・6 ファイル）は `cargo test --workspace -- --list` に**一切現れない**。`--all-targets` を付けると現れるが**doctest が列挙されなくなる**（両方とも実測）。さらに `cargo test -p areka --examples` は **main の時点で既にコンパイルエラー**（例が `#[path]` include する `src/placement/spawn.rs` のテストモジュールが `crate::input_events` を参照するため）。要件 2.2/2.3 の証跡は「既定 `--list`（doctest 込み）＋ `--all-targets --list`（examples 込み・areka を除外）＋ `cargo build --workspace --all-targets`」の組み合わせで設計する必要がある。
- **本番本体分割の対象は増えない。** `src/` で本番本体 1,000 行超は `follow.rs`(1,996) と `frame.rs`(1,497) の 2 本だけで、3 位は 1,042 行（presenter.rs）。`tests/`・`examples/` の大きいファイル（最大 `spine_e2e_test.rs` 2,574・`kanade/common/mod.rs` 1,657・`choice_test.rs` 1,563・`emo-text-layer.rs` 1,434）は**全量がテストコードまたはデモコード**であり、要件 4.5 の「本番本体」ではない。本 spec が機械的に短くできるものは無い（§2.12 に「では何ができるか」を整理）。
- **動機が「行数そのもの」である以上、テストモジュールを 1 ファイルへ出すだけでは目的の半分しか達成できない。** 移設シミュレーション（1 テストモジュール＝1 ファイル）の結果、**リポジトリ最大ファイルは 8,472 → 6,476 行（follow のテストモジュール）にしかならず、2,000 行超は 17 → 9 本、1,000 行超は 54 → 39 本**。単テストモジュールの巨大ファイル（follow 6,476・presenter 4,375・frame 3,163・layout 2,545・steady 2,383・viewbox_draw 2,305・drive 2,278）は**テストモジュールの中を分割しない限り縮まない**。テストコードの中身は本 spec の Out of scope（要件 5.1・`test-cage-determinism` の領分）なので、**「どこまでを成果とするか」は要裁定**（設計判断 #12・本書で新規に提起）。

---

## 2. Current State Investigation（全数実測 2026-08-07・第 2 版）

### 2.1 規模（リポジトリ全域・`target/` と `.claude/worktrees/` を除く全 `*.rs`）

| 種別 | ファイル数 | 総行数 | `#[cfg(test)] mod {...}` 行数 | テストモジュール率 |
|---|---:|---:|---:|---:|
| `crates/*/src/**` | 387 | 189,190 | **92,868** | 49.1% |
| `crates/*/tests/**` | 198 | 53,798 | 5,437 | 10.1% |
| `crates/*/examples/**` | 34 | 14,146 | 798 | 5.6% |
| `crates/*/benches/**` | **0** | 0 | 0 | — |
| `build.rs` | **0** | 0 | 0 | — |
| `crates/` 外の `*.rs` | **0** | 0 | 0 | — |
| **合計** | **619** | **257,134** | **99,103** | 38.5% |

- **`benches/` と `build.rs` はリポジトリに 1 つも存在しない**（`find . -name build.rs` / `-type d -name benches` がいずれも 0 件・`Cargo.toml` に `[[bench]]` 宣言も 0 件）。要件 1.1/1.2 が列挙する 5 種のうち 2 種は空集合である。
- `crates/` 外（リポジトリルート・`.kiro/`・`docs/` 等）に `*.rs` は無い。
- テストモジュールの 93.7%（92,868 / 99,103）は `src/` に集中しており、**スコープ拡大はテストモジュールの総量を +6.7% しか動かさない**。

`tests/` のクレート別内訳（上位）:

| クレート | ファイル | 総行 | テストモジュール |
|---|---:|---:|---:|
| `wintf` | 89 | 16,441 | 54 |
| `dola` | 62 | 14,531 | 2,705 |
| `areka-kanade` | 11 | 7,392 | 256 |
| `areka-ghost` | 9 | 5,442 | **2,422** |
| `areka-emo-text` | 9 | 4,796 | 0 |
| `shiori-host32-host` | 5 | 1,795 | 0 |
| `areka-seriko` | 5 | 1,684 | 0 |
| その他 5 クレート | 8 | 1,717 | 0 |

`examples/` のクレート別内訳:

| クレート | ファイル | 総行 | テストモジュール |
|---|---:|---:|---:|
| `wintf` | 16 | 4,592 | 0 |
| `areka` | 5 | 3,916 | 446 |
| `pilot` | 11 | 3,841 | 352 |
| `areka-emo-text` | 2 | 1,797 | 0 |

### 2.2 必須対象（テストモジュール > 500 行）= **49 本 / テストコード合計 68,921 行**

**`src/` 48 本（第 1 版から完全に不変・テストコード合計 66,830）** — `PLAIN` = 素の `foo.rs`（ディレクトリモジュール化すればパスが変わる）／`MODRS` = `mod.rs`／`ROOT` = `lib.rs`・`main.rs`。

**crates/areka** — 13 本 / テストモジュール 21,056
| パス（`src/` 以下） | 総行 | 本体 | テストモジュール | 形 | テストモジュール数 |
|---|---:|---:|---:|---|---:|
| `placement/follow.rs` | 8,472 | 1,996 | 6,476 | PLAIN | 1 |
| `emo2_boot/frame.rs` | 4,660 | 1,497 | 3,163 | PLAIN | 1 |
| `input_events/balloon.rs` | 2,825 | 829 | 1,996 | PLAIN | 1 |
| `placement/mod.rs` | 1,899 | 563 | 1,336 | MODRS | 1 |
| `placement/spawn.rs` | 1,582 | 426 | 1,156 | PLAIN | 1 |
| `placement/persist.rs` | 1,535 | 465 | 1,070 | PLAIN | 1 |
| `placement/resolver.rs` | 1,306 | 295 | 1,011 | PLAIN | 1 |
| `emo2_boot/move_cue.rs` | 1,634 | 700 | 934 | PLAIN | **4** |
| `placement/measure.rs` | 1,387 | 465 | 922 | PLAIN | 1 |
| `main.rs` | 1,842 | 941 | 901 | ROOT | **7** |
| `emo2_boot/assets.rs` | 1,225 | 405 | 820 | PLAIN | 1 |
| `input_events/mod.rs` | 1,164 | 433 | 731 | MODRS | 1 |
| `placement/source.rs` | 819 | 279 | 540 | PLAIN | 1 |

**crates/areka-emo-text** — 7 本 / テストモジュール 12,411（すべて PLAIN）: `layout.rs` 3,294/749/2,545・`viewbox_draw.rs` 3,090/785/2,305・`actor.rs` 2,967/858/2,109（**2 テストモジュール**）・`viewbox.rs` 2,498/749/1,749・`draw.rs` 2,293/963/1,330・`choice.rs` 1,749/550/1,199（**3 テストモジュール**）・`state.rs` 1,630/456/1,174

**crates/areka-emo-present** — 4 本 / テストモジュール 7,563（PLAIN）: `presenter.rs` 5,417/1,042/4,375・`balloon.rs` 2,264/632/1,632・`cache.rs` 1,100/193/907・`scale.rs` 876/227/649

**crates/areka-kanade** — 6 本 / テストモジュール 7,150: `schedule/steady.rs` 3,286/903/2,383・`schedule/mod.rs` 2,176/681/1,495（MODRS・**2 テストモジュール**）・`schedule/boot.rs` 1,406/288/1,118・`actor.rs` 1,318/370/948・`shiori/real.rs` 903/280/623・`schedule/events.rs` 993/410/583

**crates/areka-sakura** — 2 本 / テストモジュール 3,824: `drive.rs` 2,808/530/2,278・`compile.rs` 1,867/321/1,546

**crates/areka-emo-compose** — 3 本 / テストモジュール 3,715: `plan.rs` 2,203/667/1,536・`scale.rs` 1,778/467/1,311・`fold.rs` 1,132/264/868

**crates/areka-seriko** — 3 本 / テストモジュール 3,469: `actor.rs` 2,331/484/1,847・`state.rs` 1,576/518/1,058・`looper.rs` 939/375/564

**crates/areka-ghost** — 3 本 / テストモジュール 2,903: `dispatcher.rs` 1,856/420/1,436・`runtime.rs` 1,613/650/963・`ticker.rs` 823/319/504

**crates/wintf** — 4 本 / テストモジュール 2,520: `ecs/window_proc/window_pos.rs` 1,160/444/716・`ecs/clickthrough/controller.rs` 1,092/455/637・`ecs/window_proc/dpi_helpers.rs` 746/148/598・`ecs/layout/systems/monitor_systems.rs` 1,050/481/569

**crates/areka-sylphya** — 1 本 / テストモジュール 866: `actor.rs` 1,587/721/866（**3 テストモジュール**）
**crates/dola** — 1 本 / テストモジュール 758: `cue/command.rs` 1,089/331/758
**crates/shiori-host32-helper** — 1 本 / テストモジュール 595: `main.rs` 1,114/519/595（ROOT・**4 テストモジュール**）

**`src/` 外 1 本（新規・テストコード合計 2,091）**

| パス | 総行 | 本体 | テストモジュール | 形 | テストモジュール数 |
|---|---:|---:|---:|---|---:|
| `crates/areka-ghost/tests/ghost/spine_e2e_test.rs` | 2,574 | 483 | **2,091** | **PATHMOD** | **10** |

`PATHMOD` = `crates/areka-ghost/tests/ghost.rs`（統合テスト入口）から `#[path = "ghost/spine_e2e_test.rs"] mod spine_e2e_test;` で読み込まれるファイル。詳細は §2.3.3。

**形の内訳（要件 3.5 の「パスが変わるファイル」の母数）**: PLAIN **43** / MODRS **3** / ROOT **2** / PATHMOD **1**。
**クレート数 12**（`areka-ghost` の新規 1 本は既に対象クレート）＝要件 7.1 のコミット粒度は最大 12 論理コミットで不変。対象 0 本のクレート: `areka-parsers`・`areka-actor`・`areka-talk`・`shiori-abi`・`shiori-host32-{ipc,host,testdll}`・`shiori4-testdll`・`areka-emo-atlas`・`pilot`。

### 2.3 既存前例（要件 3.4 の対比材料）

#### 2.3.1 `src/` の 2 系統（第 1 版から不変）

`crates/*/src/` に `#[path = ...]` **属性は 1 件も無い**（10 件のヒットは全てコメント中の言及）。一方、素の `#[cfg(test)] mod <name>;`（宣言のみ・実体は別ファイル）は **55 箇所 / 26 宣言ファイル**（うち `pub(crate) mod` 3 件）。実体ファイルの配置は 2 系統:

| 系統 | 実例 | 宣言元 | テストファイル |
|---|---|---|---|
| **(a) 同一ディレクトリ・フラット** | `areka-parsers/src/{sakura,shell,balloon,charset,kv,package}/*_tests.rs`（20 本）・`areka-emo-compose/src/{composer,golden,log_firing}_tests.rs`・`areka/src/shiori_{,lifecycle_,reference_}e2e_tests.rs`・`areka-sylphya/src/ledger_key_determinism_tests.rs` | `mod.rs` / `lib.rs` / `main.rs` | 宣言元と同じディレクトリ |
| **(b) ディレクトリモジュール `{module}/tests.rs`** | `dola/src/runtime/{instance_manager,interpolator,loop_controller,subscription_manager,timeline_manager}/tests.rs`・`wintf/src/ecs/{drag/state,graphics,layout/hit_region,layout/hit_test,pointer/dispatch,pointer/types,widget/bitmap_source,widget/text/typewriter,window/window_pos}/tests.rs`・`areka/src/emo2_boot/spine.rs` | `{module}/mod.rs` | `{module}/tests.rs` |

**(a) は宣言元が `mod.rs`/`lib.rs`/`main.rs`（＝ディレクトリの module root）だから `#[path]` 無しで成立している。** 素の `foo.rs` からフラットな `foo_tests.rs` を引くには `#[path]` が要る——これが 43 本の PLAIN に効く分岐点。

`foo.rs` と `foo/` の共存前例も既にある: `crates/areka-emo-atlas/src/decode.rs`（`pub mod wic_arm;`）＋ `crates/areka-emo-atlas/src/decode/wic_arm.rs`。Rust 2018+ で合法であることが本リポジトリ内で実証済み。

分離済みテストファイルは既に大きいものがある: `areka/src/emo2_boot/spine.rs` **2,503 行**・`areka-parsers/src/shell/decode_tests.rs` **1,395 行**・`areka-emo-compose/src/golden_tests.rs` **1,356 行**。**分離してもテストファイル自体が 1,000〜2,500 行になる**という実例であり、設計判断 #12 の材料になる。

#### 2.3.2 `tests/` の `#[path]` 系統（**スコープ拡大で新規に評価対象となった前例**）

steering `structure.md` L133-140:
> #### Integration Tests (`tests/` directory)
> - **File name**: `{feature}_{type}_test.rs` or `{feature}_test.rs`
> - **Entry point**: `tests/{domain}.rs` — `#[path]` による `mod` 宣言のみ、テストロジックは含まない
> - **Common helpers**: `tests/{domain}/common/mod.rs`

実測（行頭 `#[path` 属性の全数・コメント言及を除く）: `crates/*/tests/` に **115 箇所 / 16 ファイル**（`wintf` 67 / 8 ファイル・`dola` 30 / 6・`areka-kanade` 10 / 1・`areka-ghost` 8 / 1）、`crates/*/examples/` に **11 箇所**（`pilot` 7・`areka` 4）、`crates/*/src/` に **0 箇所**。統合テストの入口 45 本のうち、ドメイン束ね型（`ghost.rs`・`kanade.rs`・`dola/tests/{compile,cue,general,runtime,trigger,validation}.rs`・`wintf/tests/{com,drag,ecs,graphics,layout,visual,widget,win_app,window}.rs`）は**全て `#[path]` 宣言のみ**で構成されている。

→ **`#[path]` は本リポジトリにおいて「前例のない形式」ではない。** `src/` に未導入なだけであり、`tests/` では steering が名指しする唯一の標準形である。第 1 版が案 C に付けていた「新規規約導入」という減点は、スコープ拡大により**大幅に弱まる**（§4・設計判断 #1）。

#### 2.3.3 `#[path]` 読込ファイルの子モジュール解決規則（**実測・案 A に効く決定的制約**）

rustc 1.97.1 でスクラッチクレートを作り、統合テストの構造を再現して実測した:

| 宣言 | 子モジュールの解決先 | 結果 |
|---|---|---|
| `tests/dom.rs` に `#[path="dom/a.rs"] mod a;` → `a.rs` 内で `mod sub;` | rustc は `tests/dom/sub.rs` または `tests/dom/sub/mod.rs` を探す | **E0583（`tests/dom/a/sub.rs` は探さない）** |
| 同上 → `a.rs` 内で `#[cfg(test)] mod a_tests;` | `tests/dom/a_tests.rs` | ✅ 成立（テスト名 `a::a_tests::*`） |
| 同上 → `b.rs` 内で `#[cfg(test)] #[path="b_tests.rs"] mod tests;` | `tests/dom/b_tests.rs` | ✅ 成立（テスト名 `b::tests::*`） |
| `tests/dom.rs` に `#[path="dom/c/mod.rs"] mod c;` → `c/mod.rs` 内で `mod tests;` | `tests/dom/c/tests.rs` | ✅ 成立（テスト名 `c::tests::*`） |

**含意**:
- **案 A（`foo/tests.rs` を素の `mod tests;` で引く）は `#[path]` 読込ファイルでは成立しない。** `spine_e2e_test.rs` に `mod tests;` と書くと `tests/ghost/tests.rs` を探しに行き、しかも同ディレクトリの兄弟ファイル（`recorder.rs` 等）と衝突する。案 A を `tests/` へ適用するには `#[path = "spine_e2e_test/s1_boot_success.rs"]` のような**明示 `#[path]` の併記が必須**であり、要件 3.1 の「単一方式」が `src/` と `tests/` で割れる。
- **案 C（`#[path]` フラット兄弟）は `src/` と `tests/` で文言まで同一に書ける。** `#[cfg(test)] #[path = "<basename>_<テストモジュール名>.rs"] mod <テストモジュール名>;` が両方で成立する。
- **案 B（ディレクトリモジュール化）も `tests/` で成立する**が、入口 `tests/ghost.rs` の `#[path = "ghost/spine_e2e_test.rs"]` を `#[path = "ghost/spine_e2e_test/mod.rs"]` へ書き換える必要がある（＝要件 3.5 の「パスが変わるファイル」に PATHMOD 1 本が加わり、かつ**入口ファイルの編集**が発生する）。

#### 2.3.4 `examples/` の `#[path]` include（要件 2.7・4.x への制約）

`crates/areka/examples/window-placement.rs:107` と `collision-probe.rs:231` は `#[path = "../src/placement/mod.rs"] mod placement;` で placement モジュール木ごと私有 include する。`collision-probe.rs:218,224` は `emo2_boot/{target_map,hit_region}.rs` も同様に include する。[[areka-examples-path-include-no-crate-paths]] の規律により、**include される本番コードは `crate::` パスを持てない**（`follow.rs` の本番本体は `crate::` を 1 件も使っていない）。

`pilot/examples/shiori-host-32/` は `#[path = "ipc.rs"]` 等で**同一ディレクトリの兄弟ファイル**を共有する（x64 親と i686 helper の 2 ターゲットが同一 `ipc.rs` を取り込む単一ソース化）。これも案 C と同形の「フラット兄弟＋`#[path]`」であり、前例として数えられる。

**スコープ拡大が examples に追加する作業は無い**: `examples/` のテストモジュールは 6 ファイル・798 行で、最大が `crates/areka/examples/mock-shell.rs` の 446 行＝**500 行閾値未満**。全て必須対象外である（要件 1.5 により任意移設は許容）。

### 2.4 テストコードの位置構造（リポジトリ全域・619 ファイル全数走査）

「最初のテストモジュールが始まった行より後に、テストモジュールに属さない**非空・非コメント**行が残るか」を全数走査した結果:

- **該当 1 ファイルのみ**: `crates/wintf/src/ecs/world/mod.rs`（テストモジュールの後ろに `impl std::fmt::Debug for EcsWorld` 5 行 / L710-714）。当ファイルはテストモジュール 123 行＝必須対象外。
- **必須対象 49 本では 0 件**。すべてのテストモジュールがファイル末尾側に連続して並んでいる。
- `spine_e2e_test.rs` は 10 テストモジュールを持つが、テストモジュールとテストモジュールの“間”にあるのは `// ===== S2: 接続失敗シナリオ =====` 型の**コメントバナーのみ**であり、コードは 1 行も無い。冒頭 L1-320 が共有 fixture（`spin_pumping_ticks`／`ScriptedShioriBackend`／`RecordingSink`）で、L321 以降はテストモジュールとバナーの交互。**構造上は「先頭に fixture、以降すべてテストモジュール」**である。

テストモジュールの形式内訳（全 619 ファイル）:

| 形式 | src | tests | examples | 計 |
|---|---:|---:|---:|---:|
| ファイル内テストモジュール `#[cfg(test)] mod X { ... }` を持つファイル | 200 | 14 | 6 | 220 |
| ファイル内テストモジュールブロック数 | 229 | 28 | 6 | 263 |
| 外部ファイル宣言 `#[cfg(test)] mod X;` | 52 | 0 | 0 | 52 |
| 外部ファイル宣言 `#[cfg(test)] pub(crate) mod X;` | 3 | 0 | 0 | 3 |
| `#[cfg(test)]` が `mod` 以外に付く項目 | 40 | 0 | 0 | 40 |

テストモジュールが複数あるファイル（全域 17 本・必須対象は太字）:

| ファイル | 種別 | テストモジュール数 | テストモジュール名（行範囲・行数） |
|---|---|---:|---|
| **`areka-ghost/tests/ghost/spine_e2e_test.rs`** | tests | **10** | `tests`(321-459,139) `broadcast_relevance_partition`(473-643,171) `s1_boot_success`(652-934,283) `s2_connect_failure`(945-1103,159) `s3_helper_liveness_detected`(1116-1386,271) `s4_close_handshake`(1415-1667,253) `s5_close_deadline`(1707-2006,300) `s6_full_disconnect`(2033-2236,204) `global_log_probe`(2248-2330,83) `s7_second_boot_record_present`(2347-2574,228) |
| **`areka/src/main.rs`** | src | 7 | `startup_window_tests`(899-1068,170) `seam_tests`(1077-1292,216) `config_input_tests`(1294-1360,67) `ghost_wiring_tests`(1368-1443,76) `restore_seam_tests`(1453-1578,126) `persist_wiring_seam_tests`(1589-1677,89) `monitor_snapshot_seam_tests`(1686-1842,157) |
| **`areka/src/emo2_boot/move_cue.rs`** | src | 4 | `tests`(671-948,278) `move_sink_tests`(954-1069,116) `apply_move_tests`(1081-1429,349) `move_severity_log_tests`(1444-1634,191) |
| **`shiori-host32-helper/src/main.rs`** | src | 4 | `resolve_param_tests`(517-568,52) `classify_tests`(570-660,91) `load_ack_tests`(662-689,28) `loopback_tests`(691-1114,424) |
| `dola/tests/compile/boundary_test.rs` | tests | 4 | `numeric_boundary_tests`・`keyframe_name_collision_tests`・`stack_and_capacity_tests`・`builder_boundary_tests` |
| **`areka-emo-text/src/choice.rs`** | src | 3 | `tests`(537-1121,585) `style_resolve_tests`(1129-1339,211) `decorate_tests`(1347-1749,403) |
| **`areka-sylphya/src/actor.rs`** | src | 3 | `tests`(675-1028,354) `actor_integration_tests`(1036-1379,344) `actor_criteria_cage`(1420-1587,168) |
| `areka-ghost/src/shiori_inproc.rs` / `areka-parsers/src/package/resolve.rs` / `shiori-host32-host/src/parent_window.rs` / `wintf/src/ecs/graphics/tests.rs` / `dola/tests/trigger/runtime_test.rs` | 各 | 3 | いずれもテストモジュール 500 行未満（対象外） |
| **`areka-emo-text/src/actor.rs`** | src | 2 | `tests`(858-939,82) `runtime_tests`(941-2967,2027) |
| **`areka-kanade/src/schedule/mod.rs`** | src | 2 | `tests`(670-1554,885) `log_firing_tests`(1567-2176,610) |
| `areka-emo-text/src/lib.rs` / `areka-sylphya/src/reader.rs` / `shiori-host32-ipc/src/lib.rs` | src | 2 | いずれも対象外 |

**必須対象 49 本のうち複数テストモジュールは 8 本**（第 1 版の 7 本 ＋ `spine_e2e_test.rs`）。要件 1.3 と要件 2.4 の衝突（§3・設計判断 #2）は、`spine_e2e_test.rs` の 10 テストモジュールが加わることで**より鮮明になる**——10 テストモジュールを 1 ファイルへ入れ子集約すれば `spine_e2e_test::tests::s1_boot_success::*` のようにテスト名が全面的に変わる。

### 2.5 `#[cfg(test)]` 非 `mod` 項目 **40 件**（要件 1.6 の対象・全数・**全て `src/`**）

`tests/` と `examples/` には 1 件も無い（統合テスト・example ターゲットでは `cfg(test)` の意味論が異なり、`#[cfg(test)]` を項目に付ける動機がないため）。

| 分類 | 件数 | file:line（全数） | 移設可否の見立て |
|---|---:|---|---|
| `impl` ブロック内のテスト専用 inherent メソッド（`pub*` 系） | **15** | `areka/src/emo2_boot/frame.rs:301,316,328,337,346`（`drain_move_directives`/`read_back_target`/`drain_received`/`apply_present`/`balloon_model_scopes`）・`areka/src/shiori_host.rs:292,309`・`areka-emo-text/src/{segment.rs:77, surface.rs:381, draw.rs:692}`・`dola/src/runtime/subscription_manager/mod.rs:104`・`wintf/src/ecs/widget/bitmap_source/{systems.rs:49, task_pool.rs:91,97}`・`wintf/src/runtime/window_registry.rs:99` | **移設不可**（inherent impl は本体側にしか置けない。`impl` をテストファイルへ切り出せば「本体の私有フィールドへ触るメソッドを別ファイルで定義」となり成立はするが、可視性の緩和を招く） |
| 自由関数（テスト専用ヘルパ） | **10** | `areka-emo-text/src/draw.rs:430,894,930,943`・`areka-emo-text/src/viewbox_draw.rs:154`・`areka/src/input_events/mod.rs:85`（`with_clock`）・`shiori-host32-helper/src/main.rs:418,424`・`shiori-host32-host/src/parent_window.rs:345,351` | 移設可能だが本体側の型に密着 |
| `use` 宣言（テスト時のみ必要な import） | **9** | `areka-emo-text/src/draw.rs:75,77,106,108,110,112,114,116,118`（全 9 件が同一ファイルに集中） | **本体側残置が自然**（`DrawExecutor` 等のテスト専用型が本体に在るため） |
| `impl` ブロック | 2 | `areka-emo-text/src/draw.rs:707`・`areka/src/placement/source.rs:77`（`impl GhostTitles`） | 移設可能だが本体からの参照が生じると壊れる |
| 構造体フィールド | 2 | `areka-emo-text/src/viewbox_draw.rs:117,147`（`fail_next_render` フィールド／初期化） | **移設不可**（本体の内部状態） |
| `struct` 定義 | 1 | `areka-emo-text/src/draw.rs:541`（`FormatKey`） | 移設可能 |
| 分岐 | 1 | `areka-emo-text/src/viewbox_draw.rs:485`（`if self.fail_next_render`） | **移設不可** |

> 第 1 版の「44 件」は `pub(crate) mod`（in-file 1 件＝`shiori-host32-host/src/lifecycle.rs:272`、宣言のみ 3 件）を非 `mod` 側へ誤計上していた分の過大計上である。正しくは **40 件**で、`pub(crate) mod` 4 件は `mod` テストモジュールとして集計済み（§2.4 の形式内訳に反映）。

**テスト間の状態汚染との関係**: `viewbox_draw.rs` の `fail_next_render` は本番構造体に埋め込まれた注入シームであり、[[obsolete-vs-broken-test-policy]]／要件 5.1（時刻注入シームの変更禁止）に照らして**本 spec では触れない**のが正しい。`test-cage-determinism`（W6.9）へ所見として送る候補。

### 2.6 follow.rs 本番本体 1,996 行の責務シーム（要件 4.1）

| 行範囲 | 責務 | 主要項目 | 概算 |
|---|---|---|---:|
| 65-230 | **アンカー射影ポリシー** | `pub trait DragPositionPolicy` / `pub struct BottomSnapPolicy` / `impl` / `pub fn project_anchor` / `pub struct Anchored` | ~166 |
| 232-900 | **ドラッグ＋バルーン追従** | `pub struct BalloonFollow` / `on_char_drag` / `on_char_drag_end` / `policy_mapped_position` / `BalloonFollowTrigger` / `follow_balloon` / `guard_balloon_position` / `on_balloon_drag` / `on_balloon_drag_end` | ~670 |
| 904-1475 | **窓移動・リサイズ API と反映** | `pub fn move_window_to` / `pub fn resize_window_to` / `pub fn anchor_changed_system` / `enqueue_window_set_pos` / `log_window_move` | ~572 |
| 1476-1588 | **モニタ work area 解決** | `pub struct MonitorSnapshot` / `pub fn work_area_for_window` / `pub enum WorkAreaResolution` / `pub fn work_area_for_window_with_origin` | ~113 |
| 1589-1926 | **可視性ガード** | `pub enum VisibilityVerdict` / `pub fn guard_visibility` / `rect_at` / `rects_intersect` / `intersects_any_work_area` / `clamp_x_into` / 3 定数タグ / `route_applies_visibility_guard` / `apply_visibility_guard` / `evaluate_visibility_guard` | ~338 |
| 1927-1996 | 補助 API | `pub fn resize_window_keep_position` | ~70 |

→ 5 シーム。最大は「ドラッグ＋バルーン追従」670 行で、要件 4.2 の目安 1,000 行を全シームが満たす。

**制約（重要）**: `follow.rs` の**本番本体は `crate::` パスを 1 件も使っていない**（全数確認・本体 0 件／13 件はすべてテストモジュールの中 L2008 以降）。これは `crates/areka/examples/window-placement.rs:107` と `collision-probe.rs:231` が `#[path = "../src/placement/mod.rs"] mod placement;` で placement 木ごと include するためで（[[areka-examples-path-include-no-crate-paths]]）、`follow.rs:1927` に「examples が `#[path]` include するため、本体未使用ビルドでも必要」という `#[allow(dead_code)]` 注記も残る。**分割後のサブモジュールも `crate::` 不使用を守らねば example のビルド（`cargo build --examples`）が壊れる。**

`follow::` の外部参照は 26 箇所（`main.rs` 6・`placement/mod.rs` 4・`placement/spawn.rs` 3・`placement/persist.rs` 3・`emo2_boot/move_cue.rs` 3・`emo2_boot/frame.rs` 3・`examples/collision-probe.rs` 3・`examples/window-placement.rs` 1）。`follow` を facade に `pub use` 再輸出すれば呼び出し側は **0 箇所変更**で済む（要件 4.3 を満たす最短経路）。

### 2.7 frame.rs 本番本体 1,497 行の責務シーム（要件 4.1）

| 行範囲 | 責務 | 主要項目 | 概算 |
|---|---|---|---:|
| 64-180 | **attach 計画** | `pub struct AttachPlan` / `pub struct PlannedAttach` / `pub fn plan_attachments` | ~117 |
| 181-368 | **配線コンテナ** | `pub struct Emo2Wiring` ＋ `impl`（**301-346 に `#[cfg(test)]` メソッド 5 件が内在**） | ~188 |
| 369-621 | **attach 相** | `pub fn run_attach_phase` / `connect_balloon_text` | ~253 |
| 622-1044 | **DPI 相** | `AuthorDpis` / `GhostWindowKind` / `GhostWindowClass` / `classify_ghost_window` / `reconcile_window_size` / `trait ScaleReportSource` / `DpiChangedQuery` / `dpi_phase_with` / `reproject_char_window_at_current_size` / `pub fn run_dpi_phase` | ~423 |
| 1045-1191 | **テキスト scale 相** | `pub fn run_text_scale_phase` / `reconcile_reported_sizes` | ~147 |
| 1192-1258 | **drain 相** | `pub fn run_drain_phase` / `pub fn run_move_drain_phase` | ~67 |
| 1259-1407 | **resnap** | `resnap_from_sizes` / `resnap_shell_targets` / `trait PhysicalSizeSource` / `resnap_with` | ~149 |
| 1408-1465 | **テキスト相** | `resolve_talk_time` / `pub fn run_text_phase` | ~58 |
| 1466-1497 | **フレーム統合** | `pub fn emo2_frame_system` | ~32 |

→ 9 シーム（brief の「7 フェーズ」より細かい）。`frame::` の外部参照は 10 箇所（`input_events/balloon.rs`・`input_events/mod.rs`・`tests/emo2_real_run.rs` 等）。

**制約**: `Emo2Wiring` の `impl` に `#[cfg(test)]` メソッドが 5 件混ざっており、`Emo2Wiring` を別サブモジュールへ移すと、それらのメソッドが触る私有フィールドの可視性を `pub(super)` 等へ緩めるか、`impl` を同じサブモジュールへ同伴させる必要がある（要件 2.5「公開 API 不変」はクレート外観測が基準なので `pub(crate)`/`pub(super)` の内部調整は許容範囲だが、design で明示すべき）。

### 2.8 ログ target とモジュールパスの結合（要件 2.7「挙動変更ゼロ」への含意）

`tracing` の既定 target はモジュールパスであり、本リポジトリは `RUST_LOG` ディレクティブをモジュールパスで書く運用（`structure.md`／`logging.md` L109-118・`RUST_LOG="wintf::ecs::graphics=debug"` 等）。`placement/diag.rs:62` は `pub const DIAG_TARGET: &str = "areka::placement::diag"` を**テストモジュールで固定**している（`diag.rs:371` の `assert_eq!`）。

- **テストモジュールの移設では target は変わらない**（`follow.rs` の `mod tests` を別ファイルへ出してもモジュールパスは `areka::placement::follow::tests` のまま）。→ 安全。案 A・B・C いずれも同じ。
- **本体分割では変わる**（`follow.rs` の項目を `follow/visibility.rs` へ移すと既定 target が `areka::placement::follow::visibility` になる）。`RUST_LOG=...areka::placement::follow=debug` のような**前置一致フィルタは吸収する**が、`target ==` 完全一致で判定するテストモジュールがあれば壊れる。
  - `follow.rs`／`frame.rs` のテストモジュールを全数確認: target 文字列の完全一致判定は **無し**（`frame.rs:1977` の capture layer は `level=` で数えるのみ、`follow.rs:6787` は `EnvFilter` の前置ディレクティブ `areka::placement::diag=debug` を使う）。
  - ただし `wintf/src/ecs/window_proc/window_pos.rs:460` のように複数 target を並べた EnvFilter 文字列が他所にもある。分割対象 2 本の項目が発する target 名を design で列挙し、全 EnvFilter 文字列と突合すること（**Research Needed R-3**）。**スコープ拡大により、突合先に `crates/*/tests/**` の EnvFilter 文字列も加わる。**

### 2.9 移設の「無変更性」を実際に阻む唯一の機械的差分＝インデント

`mod tests { ... }` を別ファイルの module root へ出すと、テストコード本文は**一律 4 スペースの de-indent** が要る（そうしないと rustfmt 差分と読みにくさが残り、`mod tests {}` をテストファイル内に再度書けばテスト名が `tests::tests::*` へ変わる）。**68,921 行**が空白差分として動くため:

- 要件 2.4 の「内容不変」の検証は**空白非依存の比較**（`git diff -w` / 行の `lstrip()` 正規化比較）で行う必要がある（要件 2.4 に明文化済）。
- 逆に言えば、`lstrip()` 正規化後の完全一致は**極めて強い静的証跡**になる（[[areka-evidence-classes-static-equals-real-machine]] の「静的構造証跡」に相当）。design でこの照合スクリプトを成果物に含めることを推奨。

### 2.10 テスト証跡の採取範囲（要件 2.2/2.3）— **スコープ拡大で被覆の穴が判明**

本環境 `cargo 1.97.1 / rustc 1.97.1` でスクラッチクレートを作り、`src/`・`tests/`・`examples/` の 3 種を持つ構成で `--list` の意味論を実測した。

**実測 1: 既定の `cargo test -- --list`**

```
     Running unittests src\lib.rs (target\debug\deps\probe-<hash>.exe)
bar::tests::path_form: test
deep::inner::tests::nested_path_form: test
foo::tests::dir_form: test
3 tests, 0 benchmarks
     Running tests\dom.rs (target\debug\deps\dom-<hash>.exe)
a::a_tests::a_plain_flat_sibling: test
b::tests::b_path_flat_form: test
2 tests, 0 benchmarks
   Doc-tests probe
...
```

- lib / bin の in-source テストモジュール・**統合テスト（`tests/`）**・**doctest** を列挙する。
- **`examples/` のテストモジュールは 1 つも現れない**（`examples/demo.rs` に `#[cfg(test)] mod tests` を置いても列挙されない）。

**実測 2: `cargo test --all-targets -- --list`**

```
     Running unittests src\lib.rs ...           （lib テストモジュール）
     Running tests\dom.rs ...                   （統合テスト）
     Running unittests examples\demo.rs ...     （★ examples のテストモジュールが現れる）
tests::example_cage: test
1 test, 0 benchmarks
```
- **`examples/` のテストモジュールが列挙されるようになる**。
- **`Doc-tests` セクションが消える**（`--all-targets` は doctest を含まない・cargo の既知仕様）。

**実測 3: `crates/areka` の examples は test モードでコンパイルできない（既存状態）**

main（`247d48a`）で `cargo test -p areka --examples --no-run` を実行:

```
error[E0433]: cannot find `input_events` in `crate`
   --> crates\areka\examples\..\src\placement\spawn.rs:879:16
879 |         crate::input_events::attach_char_pointer_handlers(&mut world);
error: could not compile `areka` (example "window-placement" test)
error: could not compile `areka` (example "collision-probe" test)
```

- `spawn.rs` のテストモジュールは **L427 開始**であり、L879 は**テストモジュールの中**。example は `#[path]` で `src/placement/mod.rs` 木を include するため、`cfg(test)` が立つ test モードでは**テストモジュールごと取り込まれる**。テストモジュールは `crate::input_events::...` を参照するが、example のクレートルートには `placement` しか無い。
- `spawn.rs:871` に「本テストは `#[cfg(test)]` ゆえ `crate::` パス使用可（example の `#[path]` include 不変条件は非テストコード限定）」という**明示の先行裁定**が残っている。すなわちこれは事故ではなく既定の設計判断であり、**本 spec 着手前から存在する既存状態**である。
- 一方 `cargo build -p areka --examples`（非 test モード・`cfg(test)` オフ）は **11.28s で成功**。`cargo test -p wintf -p pilot -p areka-emo-text --examples --no-run` も**全て成功**（18 example バイナリが test ハーネスとしてビルドされた）。
- **移設の前後でこの状態は変わらない**（テストモジュールを別ファイルへ出しても `#[cfg(test)] mod ...;` 経由で同じ条件下に取り込まれる）。ただし証跡手順を `--all-targets` 一本にはできない。

**推奨する証跡採取（PowerShell・移設前／後で同一手順・3 本立て）**:

```powershell
# (1) 既定: lib/bin テストモジュール ＋ 統合テスト ＋ doctest
cargo test --workspace --no-fail-fast -- --list 2>&1 |
  Select-String -Pattern ': test$|: benchmark$' |
  ForEach-Object { $_.Line } | Sort-Object | Set-Content before_default.txt

# (2) examples のテストモジュールを含める（areka は既存の E0433 ゆえ除外／--all-targets は doctest を落とす）
cargo test --workspace --exclude areka --all-targets --no-fail-fast -- --list 2>&1 |
  Select-String -Pattern ': test$|: benchmark$' |
  ForEach-Object { $_.Line } | Sort-Object | Set-Content before_alltargets.txt

# (3) コンパイル被覆（テストを列挙しないターゲットの回帰検出）
cargo build --workspace --all-targets 2>&1 | Set-Content before_build.txt

# 移設後に after_*.txt を同一手順で採取し
Compare-Object (Get-Content before_default.txt)    (Get-Content after_default.txt)     # 出力ゼロ＝一致
Compare-Object (Get-Content before_alltargets.txt) (Get-Content after_alltargets.txt)  # 同上
```

**被覆の明細（design で明記すべき表）**:

| 対象 | (1) 既定 `--list` | (2) `--all-targets --list` | (3) `build --all-targets` |
|---|---|---|---|
| `src/` の in-source テストモジュール（lib/bin） | ✅ 列挙 | ✅ 列挙 | ✅ コンパイル |
| `tests/` 統合テスト | ✅ 列挙 | ✅ 列挙 | ✅ |
| doctest | ✅ 列挙 | ❌ **落ちる** | ❌ |
| `examples/` のテストモジュール（wintf/pilot/emo-text） | ❌ **現れない** | ✅ 列挙 | ❌（非 test モード） |
| `examples/` のテストモジュール（areka の 5 本） | ❌ | ❌ **E0433 で既存赤** | ✅（非 test モードなら緑） |
| `benches/` | — | — | — （リポジトリに存在しない） |

**その他の注意点（design で解消が要る）**:
1. `--list` はビルドを要求する。本ワークツリーには `target/` が無く、初回は**フルコールドビルド**になる（`bevy_ecs` + `windows-rs` 系）。移設前スナップショットの採取タイミングを実装開始前に固定すること。
2. `cargo test --workspace` の全緑判定（要件 2.1）は **i686 の host-32 成果物が先に要る**（[[workspace-test-needs-i686-host32-artifacts]]）。`--list` だけならテスト本体を走らせないので不要だが、要件 2.1 の全緑は別途 i686 ビルド後に取る必要がある。
3. **doctest の行番号問題は実質解消した**（R-4 の回答）。`--list` の doctest 行は `src\lib.rs - add (line 2): test` と行番号を含むが、(a) **テストモジュールの移設は doc コメント（本体側）を動かさない**、(b) 本体分割の対象 `follow.rs`／`frame.rs` には**コードフェンスが 1 つも無い**、(c) `crates/areka` は `[[bin]]` のみで **lib ターゲットを持たない**（doctest は lib ターゲットでしか走らない）。必須対象 49 本のうちコードフェンスを持つのは `choice.rs`・`scale.rs`(emo-compose)・`balloon.rs`(emo-present)・`dpi_helpers.rs` の 4 本だが、**すべて ` ```text ` で実行対象外**。→ doctest 名は本 spec の全作業を通じて不変。
4. `--no-fail-fast` を付けないと、いずれかのターゲットのビルドや列挙が失敗した時点で以降が採取されない。

### 2.11 実装ウェーブの空白（要件 5.4）

`git worktree list` 実測（2026-08-07）:
- 現行ワークツリー 4 本: `main`(`247d48a`) / 本ブランチ `claude/areka-p0-file-slimming-64d065`(`a1f0f31`) / **`claude/areka-p0-file-slimming-e4f098`（同一 spec の重複ワークツリー・`f657d84`）** / `claude/epic-kepler-bdbee8`(`ce7d165`＝main の PR#102 相当)。
- **他 spec の実装ブランチは 1 本も走っていない**（W5.95＝実装ウェーブ空白期は成立）。
- ただし同一 spec の重複ワークツリー e4f098 が存在する。着手前にどちらを正とするか確認が要る（**Research Needed R-5**）。
- `.kiro/specs/` 直下の active spec は本 spec を含め 16 本（すべて文書フェーズ）。

### 2.12 `src/` 外の大きいファイル — 「テストモジュールメトリクスが取りこぼす肥大」（**新規**）

`tests/` と `examples/` のファイルは**大半が全量テストコード／デモコードであり `#[cfg(test)]` テストモジュールを持たない**（統合テストは crate 全体が test ターゲットなので `#[cfg(test)]` を書く必要が無い）。したがって要件 1.1 の「テストモジュール 500 行超」という判定式は、これらのファイルに対して**ほぼ発火しない**。実際、`src/` 外でテストモジュール 500 行超は `spine_e2e_test.rs` 1 本のみである。

**`src/` 外で総行数が大きいファイルの全数（総行 ≥ 500）**:

| パス | 総行 | 本体 | テストモジュール | 種別 | 本 spec が機械的にできること |
|---|---:|---:|---:|---|---|
| `areka-ghost/tests/ghost/spine_e2e_test.rs` | 2,574 | 483 | **2,091** | tests | **✅ 必須対象**。10 テストモジュールを分離すれば本体 483 行＋最大 300 行のテストファイル 10 本になる |
| `areka-kanade/tests/kanade/common/mod.rs` | 1,657 | 1,401 | 256 | tests | テストモジュール 256 行＝閾値未満。本体 1,401 行は**テスト支援ヘルパ**（本番本体ではない）→ 要件 4.5 により対象外 |
| `areka-kanade/tests/kanade/choice_test.rs` | 1,563 | 1,563 | 0 | tests | テストモジュールゼロ＝**判定式が発火しない**。全量が `#[test]` 群 |
| `areka-emo-text/examples/emo-text-layer.rs` | 1,434 | 1,434 | 0 | examples | 同上（デモバイナリ） |
| `areka/examples/emo-present.rs` | 1,168 | 1,168 | 0 | examples | 同上 |
| `areka-kanade/tests/kanade/mouse_test.rs` | 1,116 | 1,116 | 0 | tests | 同上 |
| `dola/tests/cue/runtime_test.rs` | 1,102 | 1,102 | 0 | tests | 同上 |
| `areka/examples/collision-probe.rs` | 1,063 | 1,063 | 0 | examples | 同上 |
| `areka-kanade/tests/kanade/close_test.rs` | 1,009 | 1,009 | 0 | tests | 同上 |
| `pilot/examples/pilot-clickthrough-alpha-toggle/main.rs` | 986 | 901 | 85 | examples | テストモジュール 85 行＝閾値未満 |
| `areka/examples/mock-shell.rs` | 908 | 462 | 446 | examples | テストモジュール 446 行＝**閾値未満**（要件 1.5 で任意移設は可） |
| `areka-kanade/tests/kanade/steady_test.rs` | 897 | 897 | 0 | tests | 判定式が発火しない |
| `areka-emo-text/tests/attach_wiring_test.rs` | 873 | 873 | 0 | tests | 同上 |
| `dola/tests/cue/sheet_test.rs` | 774 | 774 | 0 | tests | 同上 |
| `areka-ghost/tests/ghost/inproc_e2e_test.rs` | 726 | 726 | 0 | tests | 同上 |
| `dola/tests/cue/schedule_test.rs` | 721 | 721 | 0 | tests | 同上 |
| `pilot/examples/shiori-host-32/main.rs` | 666 | 666 | 0 | examples | 同上 |
| `areka-emo-text/tests/viewbox_blit_spike.rs` | 649 | 649 | 0 | tests | 同上 |
| `areka-seriko/tests/loop_integration.rs` | 635 | 635 | 0 | tests | 同上 |
| `areka-emo-text/tests/draw_readback_test.rs` | 634 | 634 | 0 | tests | 同上 |
| `areka-emo-text/tests/emo2_fixture_e2e_test.rs` | 587 | 587 | 0 | tests | 同上 |
| `wintf/examples/taffy_flex_demo/setup.rs` | 585 | 585 | 0 | examples | 同上 |
| `areka-emo-text/tests/pipeline_test.rs` | 554 | 554 | 0 | tests | 同上 |
| `wintf/examples/taffy_flex_demo/diagnostics.rs` | 547 | 547 | 0 | examples | 同上 |
| `areka-emo-text/tests/scale_invariance_test.rs` | 535 | 535 | 0 | tests | 同上 |
| `areka-ghost/tests/ghost/snapshot_capture_test.rs` | 526 | 400 | 126 | tests | テストモジュール 126 行＝閾値未満 |
| `wintf/tests/window/monitor_hierarchy_test.rs` | 505 | 505 | 0 | tests | 判定式が発火しない |
| `wintf/tests/layout/feedback_loop_convergence_test.rs` | 504 | 504 | 0 | tests | 同上 |

**この spec が機械的にできること／できないことの結論**:

1. **できる（必須）**: `spine_e2e_test.rs` 1 本の 10 テスト分離。本体 483 行（共有 fixture）＋テストファイル 10 本（83〜300 行）に分かれ、リポジトリで 10 番目に大きいファイルが消える。
2. **できる（任意・要件 1.5）**: `mock-shell.rs`(446)・`kanade/common/mod.rs`(256)・`recorder.rs`(205)・`snapshot_capture_test.rs`(126) 等、閾値未満のテストモジュールの同時移設。
3. **できない（機械的手段が無い）**: テストモジュールゼロの巨大テストファイル（`choice_test.rs` 1,563 等 20 本超）。これらは**「テストモジュールを外へ出す」という本 spec の操作が定義できない**（同居している本番コードが存在せず、ファイル全体が既にテストコードだけである）。短くするには「テストを責務単位で複数ファイルへ分ける」という**内容判断**が要り、要件 2.4（内容不変）・要件 4.5（follow/frame 以外の分割禁止）の双方に抵触する。
   - ただし**構造的コストは低い**: `tests/{domain}.rs` の `#[path]` 宣言を 1 行増やすだけでファイルを追加でき、テスト名は `<mod path>::<fn>` なので**モジュール名を変えなければ名前も保存される**。将来 spec の受け皿として design の「Out of scope の理由」に明記する価値がある。
4. **できない（対象外）**: `examples/` のデモコード本体（`emo-text-layer.rs` 1,434 等）。本番本体でもテストテストモジュールでもない。

### 2.13 移設後のファイルサイズ・シミュレーション（**新規・動機「行数そのもの」への直接の答え**）

「テストモジュール 500 行超の 49 本について、**1 テストモジュール = 1 ファイル**で外出しした場合」のリポジトリ全体のファイルサイズ分布を計算した:

| 指標 | 移設前 | 移設後 | 変化 |
|---|---:|---:|---|
| リポジトリ最大ファイル | 8,472（`follow.rs`） | **6,476**（`follow` のテストファイル） | **−24% にとどまる** |
| 2,000 行以上のファイル数 | 17 | **9** | −8 |
| 1,000 行以上のファイル数 | 54 | **39** | −15 |
| ファイル総数 | 619 | 695 | +76 |

移設後の上位 10 ファイル:

| 順 | 行数 | 実体 |
|---:|---:|---|
| 1 | 6,476 | `placement/follow.rs` のテストモジュール |
| 2 | 4,375 | `emo-present/presenter.rs` のテストモジュール |
| 3 | 3,163 | `emo2_boot/frame.rs` のテストモジュール |
| 4 | 2,545 | `emo-text/layout.rs` のテストモジュール |
| 5 | 2,503 | `emo2_boot/spine.rs`（既存の分離済みテストモジュール・不変） |
| 6 | 2,383 | `kanade/schedule/steady.rs` のテストモジュール |
| 7 | 2,305 | `emo-text/viewbox_draw.rs` のテストモジュール |
| 8 | 2,278 | `sakura/drive.rs` のテストモジュール |
| 9 | 2,027 | `emo-text/actor.rs` の `runtime_tests` テストモジュール |
| 10 | 1,998 | `placement/follow.rs` の本体（分割前） |

**含意**: テストモジュールを持つ 49 本のうち **41 本は単テストモジュール**であり、「1 テストモジュール＝1 ファイル」に分けてもテストファイル 1 本がそのまま巨大なまま残る。要件の動機（「1 ファイルの行数が大きすぎることそのもの」）を額面どおり読むと、**テストモジュールの外出しだけでは目的の 1/4 しか達成しない**。目的を達成する追加手段は次の 3 つで、いずれも本 spec の現行 Out of scope に触れる:

- (i) **テストモジュール自体をテーマ別に分割する**（例: follow の 6,476 行を `follow/tests/{drag,move,visibility,work_area}.rs` へ）→ テスト名が変わる（要件 2.4 抵触）。ただし「テストモジュールの**中身**は 1 行も変えず、`mod tests { ... }` の入れ子を増やす」形なら名前は `follow::tests::drag::*` になるだけで、テスト**関数名**は保存される。
- (ii) **本体分割に伴ってテストモジュールをサブモジュール単位へ分配する**（要件 4.6 の選択肢②）→ follow/frame の 2 本だけに効く。テスト名は変わる。
- (iii) **現状を受け入れ、成果を「本番ファイルが本番コードだけになること」と定義し直す**→ 追加作業ゼロ。最大ファイルは 6,476 のまま。

**この 3 択は本書で新規に提起する設計判断 #12 である。** requirements は (iii) と読める書き方（要件 2.4 のテスト名不変を厳格に読むと (i)(ii) が封じられる）だが、動機の文（「行数が大きすぎることそのもの」）は (i) を要求しているようにも読める。裁定が要る。

---

## 3. Requirements → Asset Map

| 要件 | 対応する既存資産 | ギャップ | タグ |
|---|---|---|---|
| **1.1** 全 Rust ソースのうちテストモジュール 500 行超を外出し | `src/` の (a)(b) 2 系統 55 箇所 / 26 ファイル、`tests/` の `#[path]` 系統 107 箇所 | 49 本すべてが未適用。`areka`・`areka-emo-*`・`areka-kanade`・`areka-sakura`・`areka-seriko`・`areka-ghost/tests` は前例ゼロ | Missing |
| **1.2** リポジトリ全域で全数再計測 | 本書 §2.1/§2.2 で完了（**49 本・68,921 行**・うち src 48 本 66,830 行は要件記載値と完全一致） | **`benches/`・`build.rs` は空集合**である旨を design に明記する必要 | ✅ 解消 |
| **1.3** 複数テストモジュールの同一テストファイルへの集約 | **8 本**が該当（§2.4・第 1 版の 7 本＋`spine_e2e_test.rs` の 10 テストモジュール） | **要件 2.4（テスト名不変）と衝突**。「同一ファイル」を「同一ディレクトリ」に読み替えるか、名前変更を許すかの裁定が要る | **Constraint / 要裁定** |
| **1.4** 既に外出し済のファイルを除外 | `emo2_boot/spine.rs`（2,503）・`shell/decode_tests.rs`（1,395）・`golden_tests.rs`（1,356）ほか 26 宣言ファイル 55 箇所 | 除外一覧そのものは §2.3.1 で確定済。追加作業なし | ✅ 解消 |
| **1.5** テストモジュール 500 行以下は任意 | テストモジュールを持つ 220 ファイル中 171 本が該当（src 152・tests 13・examples 6） | 任意移設をどこまで広げるかが未定。`src/` 外では `mock-shell.rs`(446)・`kanade/common/mod.rs`(256)・`recorder.rs`(205) 等 | Unknown |
| **1.6** 非 `mod` `#[cfg(test)]` の裁定 | §2.5 に全数 **40 件**（分類 7 種・全て `src/`） | 15 件（inherent メソッド）・2 件（フィールド）・1 件（分岐）は**構造的に移設不可**。裁定は「原則残置＋例外を列挙」に落ちる見込み。**スコープ拡大で件数は増えない** | **要裁定** |
| **2.1** `cargo test --workspace` 全緑 | `workflow.md` L35 の Test Gate | i686 host-32 成果物の事前ビルドが前提（既存 DoD）。ワークツリーに `target/` 無し＝コールドビルド | Constraint |
| **2.2/2.3** テスト総数一致の証跡 | 前例なし（過去 spec は `test result: ok` の貼付のみ） | **`-- --list` の名前集合完全一致比較**が最良だが、**既定 `--list` は examples を落とし、`--all-targets` は doctest を落とす**（§2.10 実測）。**3 本立ての手順**が要る。`crates/areka` の examples は test モードで既存赤 | Missing（手段は確立済・**手順は要設計**） |
| **2.4** テストモジュール内容不変 | — | 4 スペース de-indent が **68,921 行**で発生。空白非依存比較が必須（要件 2.4 に明文化済） | Constraint |
| **2.5** 公開 API 不変 | `follow::` 26 参照 / `frame::` 10 参照 / `spine_e2e_test::RecordingSink` を 4 ファイルが参照 | `pub use` 再輸出で呼び出し側 0 変更が可能（§2.6）。`spine_e2e_test` はテストモジュールを外へ出しても `RecordingSink`（本体側 L1-320）は残るため参照は不変 | Low risk |
| **2.6** `cargo build` 警告非増加 | `follow.rs:149,217,1230,1927` 等の `#[allow(dead_code)]` 群 | 分割で `#[allow]` の適用範囲が変わると新規警告が出うる（example 専用の `resize_window_keep_position` 等） | Constraint |
| **2.7/2.8** 挙動不変・可視性側で解決 | — | 本体分割で `tracing` 既定 target が変化（§2.8）。テストモジュール移設のみなら不変 | Constraint |
| **3.1-3.3** 単一方式・規則から一意・in-crate 維持 | `src/` に (a)(b) 2 系統、`tests/` に `#[path]` 系統が併存＝既に「単一方式」ではない | **案 A は `#[path]` 読込ファイルで素の `mod` が使えない**（§2.3.3）ため、`src/` と `tests/` を跨ぐ単一方式は **案 B か案 C** に絞られる | **要裁定（拡大で選択肢が絞られた）** |
| **3.4** 候補方式の対比記録 | §4 に 3 案を実測付きで整理（`src/`・`tests/` 双方での成立性を含む） | design で採否を記録するのみ | ✅ 材料あり |
| **3.5** パスが変わる本番ファイル全数一覧 | §2.2 の形内訳（PLAIN 43 / MODRS 3 / ROOT 2 / PATHMOD 1） | 案 A・案 C では **0 本**、案 B では **44 本＋入口 1 ファイルの編集**（`tests/ghost.rs`） | ✅ 材料あり |
| **4.1-4.6** 本番本体分割 2 本 | §2.6（follow 5 シーム）・§2.7（frame 9 シーム） | `crate::` 不使用規律（example include）・`Emo2Wiring` 内 `#[cfg(test)]` メソッド・tracing target 変化の 3 制約。**スコープ拡大で新規候補は発生しない**（§0.8・§2.12） | **Constraint（最難所）** |
| **5.1-5.5** 隣接 spec 非侵襲 | 実装ブランチ 0 本（§2.11） | `viewbox_draw.rs` の `fail_next_render`、**`cargo test -p areka --examples` の既存 E0433**（§2.10 実測 3）など、cage / 所有 spec へ送る所見の登記先が未定 | Unknown |
| **6.1-6.4** steering 明文化・実測更新 | `structure.md` L142-145（Unit Tests）／L133-140（Integration Tests・`#[path]` 規約）／L204（モジュール分割パターン） | **現行 steering は `{module}/tests.rs` を分離の型として名指し済**（L144）。案 A/C を採るなら追記でなく**書き換え**が要る。**L133-140 の `#[path]` 規約は案 C の追い風** | Constraint |
| **7.1-7.3** クレート単位コミット | [[areka-commit-as-you-go]] | 対象は **12 クレート**（不変）＝テスト分離 12 コミット＋本体分割 2 コミット（+ steering/brief 1） | ✅ 材料あり |

### 3.1 要件前提の検証状況

要件の Project Description は 2026-08-07 の実測を受けて既に是正済みであり（「本 spec の動機は単純で、1 ファイルの行数が大きすぎることそのものである」＋アンカードリフト主因説の撤回注記）、本書の実測と矛盾しない。第 2 版でも次を再確認した:

- テストモジュールの後ろに本番コードが残るのは**リポジトリ全域で 1 件**（`wintf/src/ecs/world/mod.rs` の `impl Debug` 5 行）。必須対象 49 本では 0 件。
- `spine_e2e_test.rs` の 10 テストモジュールの“間”はコメントバナーのみでコードは 0 行。
- したがって「テストコードの増減が本番本体のアンカーを動かす」という因果はリポジトリ全域で成立しない。

**新たに提起する事実性の論点**は §2.13（移設だけでは最大ファイルが 6,476 行までしか縮まない）であり、これは動機「行数そのもの」と成果の距離に関わる（設計判断 #12）。

---

## 4. Implementation Approach Options（要件 3.4 の対比材料）

3 案とも本環境（rustc 1.97.1）で**実際にコンパイル・列挙して動作確認済**。テスト名はいずれも inline テストモジュールと同一（`<module path>::<テストモジュール名>::<fn>`）。**第 2 版では `src/` に加えて `tests/`（`#[path]` 読込ファイル）での成立性も実測した**。

### 案 A: 素の `#[cfg(test)] mod tests;` ＋ `foo/tests.rs`（ディレクトリ子ファイル・**本番ファイル移動なし**）

```rust
// crates/areka/src/placement/follow.rs（パス変更なし・末尾）
#[cfg(test)]
mod tests;
// → crates/areka/src/placement/follow/tests.rs
```

- **`src/` での成立**: ✅（実測済・テスト名 `foo::tests::dir_form`）。前例は `foo.rs`+`foo/` 共存が `areka-emo-atlas/src/decode.rs` の 1 例。
- **`tests/` での成立**: ❌ **素の `mod` では成立しない**。`spine_e2e_test.rs` は `#[path]` 読込ゆえ子モジュールが `tests/ghost/` に解決される（§2.3.3・E0583 実測）。`#[path = "spine_e2e_test/tests.rs"]` の**明示併記が必須**。
- **本番ファイルのパス変更**: **0 本**。
- ✅ 本番ファイル無移動 ／ steering L144 の `{module}/tests.rs` 表記と一致
- ❌ **`src/` と `tests/` で書き方が割れる＝要件 3.1「単一方式」に抵触**（拡大で新たに顕在化した致命的な減点）
- ❌ `follow/` のように「tests.rs しか入っていないディレクトリ」が 41〜43 個できる

### 案 B: ディレクトリモジュール化 `foo/mod.rs` ＋ `foo/tests.rs`

```
crates/areka/src/placement/follow.rs → follow/mod.rs + follow/tests.rs
crates/areka-ghost/tests/ghost/spine_e2e_test.rs → spine_e2e_test/mod.rs + spine_e2e_test/{s1_boot_success,...}.rs
                                                （入口 tests/ghost.rs の #[path] も書き換え）
```

- **`src/` での成立**: ✅ 前例最多（dola `runtime/*` 5 本・wintf `ecs/*` 9 本）。`structure.md` L144 が「Separated」の型として名指しする唯一の形。L204 の「肥大化したファイルは `{module}/mod.rs` + サブモジュール」方針とも一致。
- **`tests/` での成立**: ✅ **実測済**（`#[path = "dom/c/mod.rs"] mod c;` → `c/tests.rs` が解決・テスト名 `c::tests::c_dir_module_form`）。ただし**入口ファイルの `#[path]` 文字列を書き換える必要がある**。
- **本番ファイルのパス変更**: **44 本**（PLAIN 43 ＋ PATHMOD 1）＋ 入口 `tests/ghost.rs` の 1 行編集。
- ✅ リポジトリで最も見慣れた形 ／ steering 無改訂で済む ／ 本体分割（要件 4）と自然に合流（`follow/mod.rs` + `follow/drag.rs` …）
- ❌ 44 本のパス変更＝他 spec の brief アンカーが 44 ファイル分、一度だけ全滅する
- ❌ `mod.rs` が 44 個増える（Rust 2018 の `mod.rs` 忌避の背景そのもの）

### 案 C: `#[cfg(test)] #[path = "foo_tests.rs"] mod tests;`（フラット兄弟ファイル）

```rust
// crates/areka/src/placement/follow.rs（パス変更なし・末尾）
#[cfg(test)]
#[path = "follow_tests.rs"]
mod tests;
// → crates/areka/src/placement/follow_tests.rs

// crates/areka-ghost/tests/ghost/spine_e2e_test.rs（パス変更なし・末尾）
#[cfg(test)]
#[path = "spine_e2e_test_s1_boot_success.rs"]
mod s1_boot_success;
// → crates/areka-ghost/tests/ghost/spine_e2e_test_s1_boot_success.rs
```

- **`src/` での成立**: ✅ 実測済（`src/deep/inner.rs` の `#[path="inner_tests.rs"]` → `src/deep/inner_tests.rs`・テスト名 `deep::inner::tests::nested_path_form`）。
- **`tests/` での成立**: ✅ 実測済（`#[path]` 読込ファイル `dom/b.rs` の `#[path="b_tests.rs"]` → `tests/dom/b_tests.rs`）。**`src/` と文言まで同一に書ける唯一の案**。
- **前例**: 出力ファイル名の型（`*_tests.rs` フラット配置）は `areka-parsers`（20 本）・`areka-emo-compose`（3 本）・`areka/src/shiori_*_e2e_tests.rs`（3 本）で既存。**`#[path]` 属性そのものは `crates/*/tests/` に 115 箇所・`crates/*/examples/` に 11 箇所の前例があり、steering `structure.md` L135 が統合テスト入口の唯一の形として明文化している。**`crates/*/src/` に限れば前例 0 件。
- **本番ファイルのパス変更**: **0 本**。
- ✅ ディレクトリを一切増やさない ／ テストファイルが本番ファイルの真横に並ぶ（`follow.rs` / `follow_tests.rs`）
- ✅ 本体分割との干渉なし（`follow/` を本番サブモジュール専用にできる）
- ✅ **`src/` と `tests/` で同一の書き方＝要件 3.1「単一方式」を最も素直に満たす**
- ❌ `crates/*/src/` に限れば新規の規約導入。`#[path]` はモジュール解決の直感を壊すため一般に忌避される
- ❌ `mod.rs`/`lib.rs`/`main.rs`（5 本）では `#[path]` が不要（素の `mod foo_tests;` で足りる）ため、厳密には冗長（「`#[path]` を常に明示的に書く」で形式統一は可能）
- ❌ steering L144 の「Separated: `{module}/tests.rs`」を**書き換える**必要がある

### 比較表（第 2 版・`tests/` 列を追加）

| 観点 | A: `foo/tests.rs`（素の `mod`） | B: `foo/mod.rs`+`foo/tests.rs` | C: `#[path]` フラット |
|---|---|---|---|
| **`src/` での成立** | ✅ | ✅ | ✅ |
| **`tests/`（`#[path]` 読込）での成立** | ❌ **明示 `#[path]` 併記が必須** | ✅（入口の `#[path]` 書き換えが要る） | ✅（`src/` と同一文言） |
| 要件 3.1「単一方式」 | **✗ 割れる** | ○ | **◎** |
| 本番ファイルのパス変更（要件 3.5） | **0 本** | **44 本＋入口 1 編集** | **0 本** |
| リポジトリ内の既存前例 | 宣言形式◎ / `foo.rs`+`foo/` 共存△（1 例） | ◎（src 14 モジュール・tests 0） | `#[path]` は src **0 件** / tests **115 件** / examples **11 件** |
| steering との整合 | 表記一致・前提は要追記 | **完全一致（無改訂可）** | L144 の書き換えが必要・L135 とは整合 |
| 規則の一意性（要件 3.2） | ◎（`src/` 内では 1 規則） | ◎ | ◎（全域 1 規則） |
| 複数テストモジュール 8 本の扱い | ファイル分割（名前保存） | ファイル分割（名前保存） | ファイル分割（名前保存） |
| 本体分割（要件 4）との合流 | ◎ | ◎（最も自然） | ◎（`follow/` を本体専用にできる） |
| 増える空ディレクトリ | 41〜43 個 | 0（本体が入る） | 0 |
| レビュー・可読性 | ○ | ○ | ◎（真横に並ぶ） |
| 移設作業そのもののリスク | 低（末尾切り＋de-indent） | 低＋`git mv` 44 本 | 低 |

**いずれの案でも共通して要る作業**: テストコード本文の 4 スペース de-indent（68,921 行）／テストモジュール先頭の `use super::*` 等はそのまま有効（`use super::` は `src/` 434 箇所・リポジトリ全域 582 箇所で、移設後も同じモジュール関係を維持）／`use crate::...`（`src/` 737 箇所・全域 751 箇所）もクレートルート相対なので不変。

**スコープ拡大による評価の変化（要約）**: 第 1 版では A と C が拮抗し B が「前例最多だが 43 本のパス変更」で減点、という構図だった。第 2 版では **A が要件 3.1 に抵触して脱落気味**になり、**C が「`src/` に前例ゼロ」から「`src/` に未導入なだけで `tests/` では標準」へ格上げ**された。B は「44 本のパス変更＋入口編集」とコストが微増した。

### 本体分割（要件 4）の選択肢

| | D1: facade 再輸出型 | D2: 純粋移動型 |
|---|---|---|
| 形 | `follow.rs`（または `follow/mod.rs`）に `pub use drag::*;` 等を置き、外部から見た `placement::follow::X` を維持 | 項目を `follow::drag::X` へ移し、呼び出し側 26 箇所を追随 |
| 要件 4.3 適合 | ◎（可視性同一・呼び出し側 **0 変更**） | ○（「呼び出し側の変更をモジュールパスの追随に限る」を満たす） |
| tracing target | 変化する（実体が移るため）。§2.8 の突合が要る | 同左 |
| `crate::` 不使用規律 | サブモジュールにも波及 | 同左 |
| example `#[path]` include | `placement/mod.rs` 経由で自動追随 | 同左 |

**テストモジュールの配置（要件 4.6）**: 分割後 `follow` のテストモジュール 6,476 行を ① 1 本に集約（テスト名 `follow::tests::*` 完全保存・**6,476 行の巨大ファイルが残る**）か、② サブモジュール単位に分配（`follow/drag/tests.rs` 等・**テスト名が `follow::drag::tests::*` へ変わり要件 2.4 に抵触**）か。要件 2.4 を厳格に読めば ① 一択だが、§2.13 のとおり ① を選ぶとリポジトリ最大ファイルは 6,476 行のまま残る。設計判断 #4 と #12 は連動する。

---

## 5. Effort & Risk

| 作業単位 | 規模 | Effort | Risk | 根拠 |
|---|---|---|---|---|
| テスト分離 `src/`（48 本 / 12 クレート / 66,830 行） | 12 論理コミット | **M（3–7 日）** | **Low** | 全テストモジュールが末尾＝末尾切り＋de-indent の機械作業。interleaved 0 件（§2.4）。`--list` 名前集合比較で回帰を即検出できる |
| テスト分離 `tests/`（1 本 / 10 テストモジュール / 2,091 行） | 既存 `areka-ghost` コミットへ同梱 | **S（1 日未満）** | **Low** | `#[path]` 読込ファイルの子モジュール解決規則（§2.3.3）を守れば機械作業。`RecordingSink` 等の fixture は本体側に残るため 4 ファイルの `crate::spine_e2e_test::` 参照は不変 |
| `follow.rs` 本体分割（1,996 行→5 シーム） | 1 コミット | **S–M** | **Medium** | `crate::` 不使用規律・example `#[path]` include・`#[allow(dead_code)]` 群・tracing target 変化 |
| `frame.rs` 本体分割（1,497 行→9 シーム） | 1 コミット | **S–M** | **Medium** | `Emo2Wiring` impl 内の `#[cfg(test)]` メソッド 5 件と私有フィールド可視性 |
| 証跡採取（前後 `--list` **3 本立て** + 全緑） | — | **S–M** | **Medium** | コールドビルド時間・i686 host-32 成果物の事前ビルド・`--all-targets` の areka 除外運用（§2.10） |
| steering 追記 / brief 実測更新 | 1 コミット | **S** | **Low** | `structure.md` L133-140 / L142-145 / L204 の書き換え要否は案の選択次第 |
| **合計** | 15 前後のコミット | **M〜L（5 日〜2 週）** | **Low–Medium** | 案 B を採ると `git mv` 44 本と他 spec アンカー全滅の一度きりコストが乗る |

**最大の非機械的リスク**は本体分割 2 本（要件 4）であり、テスト分離（要件 1）ではない。要件 7.3 が両者のコミット分離を求めているのは実測上も正しい。**スコープ拡大が加えた Effort は S 1 件（`spine_e2e_test.rs`）と証跡手順の複雑化のみ**で、全体規模はほぼ変わらない。

---

## 6. Research Needed（design フェーズへ持ち越し）

- **R-1**: 案 A/C を採る場合、既存 55 箇所の (a)(b) 2 系統を新方式へ揃え直すか、新規 49 本のみ揃えるか。要件 3.1「全ての移設対象へ同一方式」の「移設対象」の外延（既存分離済ファイルを含むか）。
- **R-2**: 要件 1.5 の任意移設をどこまで広げるか。`placement/`（9 本中 7 本が必須）・`emo2_boot/`（11 本中 3 本が必須）のようにディレクトリ内で混在が残る箇所の一貫性方針。**拡大で `tests/`・`examples/` の閾値未満テストモジュール（計 19 ファイル＝tests 13・examples 6。上位は `mock-shell.rs` 446 / `kanade/common/mod.rs` 256 / `recorder.rs` 205 / `snapshot_capture_test.rs` 126 / `pilot-clickthrough-alpha-toggle/main.rs` 85）も候補に入った。**
- **R-3**: 本体分割後の新 target 名（`areka::placement::follow::*` / `areka::emo2_boot::frame::*` の子）を列挙し、リポジトリ全体の `EnvFilter` 文字列・`RUST_LOG` 手順書・実機サインオフ grep 語と突合する（`wintf/src/ecs/window_proc/window_pos.rs:460` 等）。**突合先に `crates/*/tests/**`（198 ファイル）も含める。**
- **R-4**: ~~doctest の行番号問題~~ → **解消済**（§2.10 注意点 3）。対象 2 本にコードフェンス 0 件・`crates/areka` に lib ターゲット無し・必須対象 49 本のフェンスは全て ` ```text `。design には「doctest 名は不変」と根拠付きで書けばよい。
- **R-5**: 重複ワークツリー `claude/areka-p0-file-slimming-e4f098`（`f657d84`）の扱い。実装をどちらのブランチで行うか（要件 5.4 の確認手順に含める）。
- **R-6**: `crates/wintf/src/ecs/world/mod.rs` の唯一の「テストモジュールの後に本番コード」ケース（`impl Debug` 5 行 / L710-714）を、必須対象外でも先に是正して「全テストモジュール末尾」を不変条件として steering に書けるようにするか。
- **R-7**: de-indent 差分の検証スクリプト（移設前ファイルのテストモジュール領域と移設後テストファイルを `lstrip()` 正規化して完全一致を確認）を成果物に含めるか。含めるなら置き場（`scripts/` は現状不在）。
- **R-8**: 要件 2.6（`cargo build` 警告非増加）の基準値をいつ採るか。`#[allow(dead_code)]` の適用範囲が分割で変わる箇所（`follow.rs:149,217,1230,1927`）の扱い。**`cargo build --workspace --all-targets` を基準コマンドにするか（examples/tests も警告対象に入る）。**
- **R-9**（**新規**）: `cargo test -p areka --examples` の既存 E0433（§2.10 実測 3）の登記先。`spawn.rs:871` の先行裁定（テストコードは `crate::` 可）に照らせば**仕様どおりの既存状態**だが、本 spec が「全 Rust ソースを対象」と宣言する以上、証跡手順から areka の examples を除外する理由として design に記録が要る。所見の送り先候補は `test-cage-determinism`（テストコードの中身）か、新規の追跡 brief か。
- **R-10**（**新規**）: `benches/` と `build.rs` が存在しないことを design/tasks でどう扱うか。要件 1.1/1.2 の列挙をそのまま残して「該当 0 件」と記すか、将来追加されたときの規律だけ steering に書くか。
- **R-11**（**新規**）: `spine_e2e_test.rs` の 10 テストモジュールは統合テスト（crate 全体が test ターゲット）内にあるため、`#[cfg(test)]` は**常に真**で意味論的には冗長である。移設に際して `#[cfg(test)]` 属性を残すか落とすかを裁定する必要がある（落とすと「テストモジュール」の定義から外れて次回計測に現れなくなる＝計測の一貫性に影響）。同様の冗長 `#[cfg(test)]` は `tests/` 全体で 14 ファイル（`dola/tests/` 9・`areka-ghost/tests/` 3・`areka-kanade/tests/` 1・`wintf/tests/` 1）、`examples/` に 6 ファイルある。

---

## 7. 設計判断項目（要件ディスカッションへ送る・番号付き）

1. **移設方式の裁定（要件 3.1/3.4/3.5）** — 案 A（`foo/tests.rs`・本番移動 0）／案 B（`foo/mod.rs`・本番移動 44＋入口 1 編集）／案 C（`#[path]` フラット・本番移動 0）。**スコープ拡大で判断軸が変わった**: (i) 案 A は `#[path]` 読込ファイル（`tests/`）で素の `mod` が使えず**要件 3.1 の「単一方式」に抵触する**（§2.3.3 実測）、(ii) 案 C の「`#[path]` は前例ゼロ」という減点は `tests/` に 115 箇所・steering L135 の明文規約という前例により**大幅に弱まった**。実質的な争点は **B（見慣れた形・44 本のパス変更）vs C（パス変更ゼロ・`src/` に新規属性）** に絞られる。
2. **複数テストモジュール 8 本の集約単位（要件 1.3 × 2.4 の衝突）** — 「同一のテストファイル」を字義どおり 1 ファイルにするとテスト名が `tests::tests::*` へ変わる。(i)「同一ディレクトリへの集約」と読み替えテストモジュール 1 つ＝1 ファイルにする、(ii) テスト名変更を許容して 1 ファイルへ入れ子集約する、のいずれか。**(i) 推奨**（名前集合一致という最強の証跡を捨てずに済む）。**`spine_e2e_test.rs` の 10 テストモジュールが加わり、(ii) を採った場合の名前変更規模が大きくなった。**
3. **非 `mod` `#[cfg(test)]` 40 件の裁定（要件 1.6）** — 15 件（inherent メソッド）・2 件（フィールド）・1 件（分岐）は構造的に移設不可。「原則本体残置・移設対象は `mod` テストモジュールのみ」と裁定し、40 件の全数一覧（§2.5）を design に転記するのが最短。`viewbox_draw.rs` の `fail_next_render` は `test-cage-determinism` へ送る所見候補。**全 40 件が `src/` にあり、拡大で件数は増えない。**
4. **本体分割 2 本のテストモジュールの配置（要件 4.6）** — ① 分割後も 1 本集約（テスト名完全保存・6,476 行のテストファイルが残る）／② サブモジュール単位へ分配（テスト名が変わる＝要件 2.4 の緩和が要る）。**設計判断 #12 と連動**。
5. **本体分割の形（要件 4.3）** — D1 facade 再輸出（`pub use` で呼び出し側 0 変更）／D2 純粋移動（26+10 箇所を追随）。要件 4.3 は「呼び出し側の変更をモジュールパスの追随に限る」なので両方可。
6. **証跡の採取手順（要件 2.2/2.3）** — **拡大により単一コマンドでは被覆できない**（§2.10）。(a) 既定 `--list`（doctest 込み・examples 落ち）＋(b) `--all-targets --list`（examples 込み・doctest 落ち・areka 除外）＋(c) `cargo build --workspace --all-targets` の 3 本立てを採るか、(b) を諦めて「examples のテストモジュールは証跡対象外」と明記するか。**証跡の強度**（総数一致 vs 名前集合完全一致）も併せて裁定する。名前集合完全一致は追加コストほぼゼロで要件 2.4 の一部も同時に担保する。
7. **テストモジュール内容不変の検証方法（要件 2.4）** — 4 スペース de-indent が 68,921 行に必ず入るため、レビューは空白非依存比較（`git diff -w` または `lstrip()` 正規化スクリプト）で行う。スクリプトを成果物に含めるか（R-7）。
8. **steering の追記先と書き換え範囲（要件 6.1/6.2）** — `structure.md` L142-145「Unit Tests (in-source `#[cfg(test)]`)」が第一候補。案 A/C を採る場合は既存の「Separated: `{module}/tests.rs` — ディレクトリモジュール化パターン」の記述を**書き換える**必要がある。**加えて L133-140「Integration Tests」にも、`tests/` 配下のテスト分離規約（`#[path]` 読込ファイルの子モジュール解決規則を含む）を書く必要がある**（拡大で新規）。L204 の「肥大化ファイルは `{module}/mod.rs` + サブモジュール」も本体分割の形と突合が要る。
9. **任意移設の範囲（要件 1.5）** — ディレクトリ内で必須対象と非対象が混在する箇所（`placement/` 9 本中 7 本必須・`emo2_boot/` 11 本中 3 本必須・`areka-emo-text/` ほぼ全数必須）で、揃えるか混在を許すか。**拡大で `tests/`・`examples/` の閾値未満テストモジュール 19 ファイル（tests 13・examples 6・`mock-shell.rs` 446 等）も判断対象に入った。**
10. **重複ワークツリーの整理（要件 5.4・R-5）** — `claude/areka-p0-file-slimming-e4f098` が同一 spec で並存している。実装着手前にどちらを正とするか確定が要る。
11. **`benches/`・`build.rs` の扱い（新規・R-10）** — リポジトリに存在しない。要件 1.1/1.2 の列挙をそのまま残し design に「該当 0 件」と記すか、将来追加時の規律だけ steering に書くか。
12. **「行数そのもの」という動機に対して、テストモジュールの外出しだけで足りるかの裁定（新規・§2.13）** — 移設シミュレーションの結果、**リポジトリ最大ファイルは 8,472 → 6,476 行（−24%）にしかならない**（2,000 行超 17→9・1,000 行超 54→39）。41 本が単テストモジュールのため、テストファイル 1 本が巨大なまま残る。選択肢は (i) テストモジュールをテーマ別の入れ子 `mod` へ分割してファイルも分ける（テスト**関数名**は保存されるがモジュールパスは伸びる＝要件 2.4 の解釈次第）、(ii) 本体分割 2 本に限ってテストモジュールをサブモジュール単位へ分配（要件 4.6 の②）、(iii) 現状を受け入れ成果を「本番ファイルが本番コードだけになること」と定義する（追加作業ゼロ）。**要件の文面は (iii) と読めるが、動機の文（「行数が大きすぎることそのもの」）は (i) を求めているようにも読める。この裁定は本 spec の成果定義そのものに関わるため、design より前に決める価値がある。**
13. **統合テスト内の冗長 `#[cfg(test)]` の扱い（新規・R-11）** — `tests/` 配下（crate 全体が test ターゲット）の `#[cfg(test)]` は常に真で意味論的に冗長。`spine_e2e_test.rs` の 10 テストモジュールほか `dola/tests/` 13 ファイル・`examples/` 6 ファイルが該当。移設時に属性を残すか落とすか（落とすと次回のテストモジュール計測に現れなくなり、計測の一貫性が失われる）。
