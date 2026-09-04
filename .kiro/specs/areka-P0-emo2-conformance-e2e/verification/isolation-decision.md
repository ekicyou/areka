# 間欠的な赤の隔離裁定の記録

> **本書は隔離裁定の記録である。** §1（着手前の確認）は**着手時点で埋め切る節**であり、実測とその再現手順を持つ。§2〜§4 は隔離の作業（相 2）で埋める骨格であり、値の欄は `（未記入）` で始まる。空欄は「まだ観測していない」を意味し、合格でも不合格でもない。
>
> - 対象仕様: `areka-P0-emo2-conformance-e2e`
> - 裁定の根拠: requirements.md **9.1〜9.8**・**12.1**・**12.3**／design.md **D11 間欠的な赤の隔離裁定**（裁定表・門の形・失う被覆と引受先・妥当性の確認・門を付けない対象・編集集合の外に触れる範囲）
> - 根治の引受先: `.kiro/specs/areka-P0-zorder-chain-residue/brief.md` の A-1（`:15`）・A-2（`:16`）。同 brief `:34` が「e2e が先に踏んだ場合は e2e が隔離裁定を行い、根治は本 spec」という分担を明記している。**本仕様は根治を行わない**（R9.5）

---

## 1. 着手前の確認（R9.8・R12.1・R12.3）

### 1.1 採取の同定

| 欄 | 値 |
|---|---|
| 実施日 | 2026-09-03 |
| 実施した作業ツリー | `C:/home/maz/git/areka/.claude/worktrees/areka-p0-emo2-conformance-7b2e56` |
| 本仕様のブランチと HEAD | `claude/areka-p0-emo2-conformance-7b2e56` ＝ `84b84acb` |
| 比較の基準 | `origin/main` ＝ `c999acc5b6623c061b291c1dbcc0d42156746490` |
| 併走の一覧の取得元 | `git worktree list`（10 本＝本仕様を含む） |

### 1.2 本仕様が編集集合の外で触れる 2 ファイル（R9.8）

隔離の門を付ける先は次の 2 ファイルに限る。**門の付与のみで、判定ロジックは 1 行も変えない**（R9.5・R12.2）。

| # | ファイル | 実在確認（本ブランチ・2026-09-03） |
|---|---|---|
| 例外 A | `crates/wintf/src/ecs/window/zorder_pair_maintain_always_on_top_tests.rs` | 実在・794 行 |
| 例外 B | `crates/wintf/src/runtime/tick_bridge.rs` | 実在・503 行 |

実在の確認は `[ -f <path> ] && wc -l < <path>` で行った（存在しないパスに対する走査は空出力になり「接触なし」と区別が付かないため、**先に実在を証明してから**接触の有無を測る）。

### 1.3 測定の方法と較正

併走の編集集合は **2 通りで測る**。片方だけでは不完全だからである。

| 測り方 | コマンド | 何を捉えるか | 何を捉えないか |
|---|---|---|---|
| 確定済みの作業 | `git diff origin/main...<branch> --name-only`（三点＝分岐点との差分） | ブランチに commit 済みの編集集合 | まだ commit されていない作業 |
| 未確定の作業 | `git -C <worktree-path> status --porcelain` | その作業ツリーの作業中の編集 | 他の作業ツリーの commit 済み作業 |

接触の判定に使った走査（両方に同一のものを当てた）:

```
grep -E 'zorder_pair_maintain_always_on_top_tests\.rs|runtime/tick_bridge\.rs'
```

**較正（走査が生きていることの証明）**——「0 件」は走査が壊れていても出る。ゆえに同じ走査で赤（＝一致）を作れることを先に示した。

| 対照 | 入力 | 結果 |
|---|---|---|
| 陽性対照 | `git ls-files`（本ブランチの追跡ファイル全件） | **2 件一致**（例外 A・例外 B の両方を逐語で拾った） |
| 陰性対照 | `crates/areka-emo-text/src/layout.rs` の 1 行 | 0 件 |

### 1.4 併走系統の編集集合（実測・2026-09-03）

`git worktree list` が挙げた 10 本すべてを測った。「例外 2 ファイルへの接触」欄は §1.3 の走査を当てた結果である。

| # | 作業ツリー | ブランチ | 確定済み（`origin/main...` の件数） | 未確定（`status --porcelain` の件数） | 例外 2 ファイルへの接触 |
|---|---|---|---|---|---|
| 1 | `C:/home/maz/git/areka` | `main`（`9fdf8307`） | 0 | 1（`vendors/pasta` のサブモジュール差分のみ） | **0** |
| 2 | `.claude/worktrees/areka-p0-cursor-tag-canon-c24d2c` | `claude/areka-p0-cursor-tag-canon-c24d2c` | 6 | 0 | **0** |
| 3 | `.claude/worktrees/areka-p0-ukadoc-survey-1617ba` | `claude/areka-p0-ukadoc-survey-1617ba` | 68 | 0 | **0** |
| 4 | `.claude/worktrees/areka-p0-ukadoc-survey-33754d` | `claude/areka-p0-ukadoc-survey-33754d` | 6 | 0 | **0** |
| 5 | `.claude/worktrees/areka-p0-ukadoc-survey-57d0e2` | `claude/areka-p0-ukadoc-survey-57d0e2` | 6 | 0 | **0** |
| 6 | `.claude/worktrees/areka-p0-ukadoc-survey-d2d757` | `claude/areka-p0-ukadoc-survey-d2d757` | 6 | 0 | **0** |
| 7 | `.claude/worktrees/epic-kepler-bdbee8` | `claude/epic-kepler-bdbee8`（`5046cdc1`） | 0 | 0 | **0** |
| 8 | `.claude/worktrees/makoto-2-0-spec-launch-16e9c7` | 分離 HEAD（`1d21455f`） | 3 | 0 | **0** |
| 9 | `.claude/worktrees/sakura-bare-tag-lexer-bbdf8c` | `claude/sakura-bare-tag-lexer-bbdf8c` | 12 | 1（`doc/COMPAT_ARCHITECTURE.md`） | **0** |
| 10 | `.claude/worktrees/areka-p0-emo2-conformance-7b2e56` | `claude/areka-p0-emo2-conformance-7b2e56`（本仕様） | 7 | 0 | **0**（門はまだ付けていない） |

編集集合の内訳（例外 2 ファイルとの関係を読むために必要な範囲だけを挙げる）:

- **#2 cursor-tag-canon**（6 件）——すべて `.kiro/specs/areka-P0-cursor-tag-canon/` の仕様文書。コードへの接触は 0 件。
- **#3 ukadoc-survey-toolkit**（68 件）——`crates/ukadoc-survey/`（新規クレート）と `.kiro/specs/areka-P0-ukadoc-survey-toolkit/` のみ。この 2 つの接頭辞を除くと**残り 0 件**（`grep -v` で除いた結果が空であることを確認した）。既存クレートへの接触は 0 件。
- **#4／#5／#6 ukadoc-survey-{assets,property,shiori}**（各 6 件）——それぞれ自分の仕様文書ディレクトリのみ。
- **#8 makoto-2-0-spec-launch**（3 件）——`.kiro/specs/areka-P0-makoto-dll-host/brief.md`・`.kiro/specs/areka-P0-translate-pipeline/brief.md`・`.kiro/steering/roadmap.md`。
- **#9 sakura-bare-tag-lexer**（12 件）——自分の仕様文書 6 件と `crates/areka-parsers/src/sakura/` の 6 件（`lexer.rs`・`decode.rs`・`parse.rs`＋兄弟テスト 3 件）。`wintf` への接触は 0 件。

### 1.5 結論（R9.8 の「着手前に確かめて記録する」）

**本仕様以外の 9 本の作業ツリーのいずれも、確定済み・未確定のどちらの側でも、例外 A／例外 B に 1 件も接触していない。** ゆえに本仕様が両ファイルへ門を付けても併走と衝突しない。

この確認は後から同じ手順で引き直せる。引き直すときは §1.3 の較正（陽性対照が 2 件・陰性対照が 0 件）を先に通すこと。較正を通さない「0 件」は根拠にならない。

### 1.6 干渉台帳との突合（実測が台帳より多く数えた件）

design.md「編集集合の外に触れる範囲（R9.8・R12.3）」は、ロードマップの干渉台帳（`roadmap.md:97-104`）が挙げる**同時期の 3 系統**（`cursor-tag-canon`／調査系／`sakura-bare-tag-lexer`）を根拠に非接触を述べている。今回の実測は**それより多くのブランチを数えた**。差の内訳を、黙って辻褄を合わせずに記す。

| 実測したブランチ | 台帳の扱い | 判定 |
|---|---|---|
| `claude/areka-p0-cursor-tag-canon-c24d2c` | 台帳の「cursor-tag」＝**台帳が数えている** | 生きている併走。接触 0 |
| `claude/areka-p0-ukadoc-survey-{1617ba,33754d,57d0e2,d2d757}`（4 本） | 台帳の「toolkit」＋「survey 4 本」＝**台帳が数えている**（design の言う「調査系」はこの 4 本を 1 系統にまとめた呼び方） | 生きている併走。接触 0 |
| `claude/sakura-bare-tag-lexer-bbdf8c` | 台帳の「⓪ lexer 修正」＝**台帳が数えている** | 生きている併走。接触 0 |
| `main`（`9fdf8307`） | 台帳の行に無い | **`origin/main` の先祖**。`git merge-base --is-ancestor 9fdf8307 origin/main` が成功（終了コード 0）。ローカル `main` は `origin/main` より後ろに居るだけで、独自の編集を 1 件も持たない（`origin/main...main` の件数が 0） |
| `claude/epic-kepler-bdbee8`（`5046cdc1`） | 台帳の行に無い | **取り込み済み＝古い作業ツリー**。`git merge-base --is-ancestor claude/epic-kepler-bdbee8 origin/main` が成功（終了コード 0）。`origin/main...` の件数も 0。`5046cdc1` は `charset-canon` の起票（PR#131）の squash マージ commit である（`git log -1 --format='%s' 5046cdc1` で逐語確認） |
| `.claude/worktrees/makoto-2-0-spec-launch-16e9c7`（分離 HEAD `1d21455f`） | 台帳の行に無い | **生きている・未取り込み**（`is-ancestor` が失敗）。編集集合は 3 件で、`wintf` への接触は 0 件。ただし `.kiro/steering/roadmap.md` を共有する（§1.8） |
| （台帳が挙げる `channels`） | 台帳が数えている | **今日この時点で作業ツリーが存在しない**（`git worktree list` に該当なし）。ロードマップ再評価で W14 へ移動済み（`roadmap.md:90`） |

**まとめ**——台帳の 3 系統は生きている併走 6 本（＝ cursor-tag 1 本・調査系 4 本・lexer 1 本）を漏れなく覆っている。台帳に行が無い 3 本のうち 2 本（`main`・`epic-kepler`）は `origin/main` の先祖であり**併走ではない**。残る 1 本（makoto）だけが台帳の行を持たない生きた併走であり、それも `wintf` には触れていない。したがって design.md の非接触の主張は**測り直した後も成立する**が、台帳の網羅性そのものは makoto のぶんだけ足りていない。

### 1.7 本仕様の編集集合の宣言（R12.1）

design.md の `File Structure Plan`（`Directory Structure`／`Modified Files`）が定める編集集合を、実在確認つきで宣言する。**この一覧の外に触れるのは、下表の事前登記済みの例外 2 件——R9.2 の 1 ファイルと、R9.8 の 2 ファイル（§1.2 の例外 A／例外 B）——だけである。**

| 区分 | パス | 内容 | 実在確認（2026-09-03） |
|---|---|---|---|
| 新規 | `crates/areka/src/emo2_boot/spine_conformance_lap_tests.rs` | 一周の走行本体（`#[test]` 1 本・判定はここだけが行う） | **未在**（これから作る） |
| 新規 | `crates/areka/src/emo2_boot/spine_conformance_script.rs` | 台本と期待列の逐語 | **未在**（これから作る） |
| 新規 | `crates/areka/src/emo2_boot/spine_conformance_support.rs` | 段の駆動ヘルパと突合ヘルパ | **未在**（これから作る） |
| 改変 | `crates/areka/src/emo2_boot/spine.rs` | 末尾へ接続宣言 3 本＋受け口へ記録の追補 1 か所（予算 40 行以内） | 実在・930 行 |
| 新規（文書） | `.kiro/specs/areka-P0-emo2-conformance-e2e/verification/lap-procedure.md` | 実機一周走行の手順書 | 未在（これから作る） |
| 既存（文書） | `.kiro/specs/areka-P0-emo2-conformance-e2e/verification/acceptance-record.md` | 走行の受入記録（様式は task 1.2 で設置済み） | 実在 |
| 新規（文書） | `.kiro/specs/areka-P0-emo2-conformance-e2e/verification/isolation-decision.md` | **本書** | 本タスクで新設 |
| 新規（文書） | `.kiro/specs/areka-P0-emo2-conformance-e2e/verification/m1-completion.md` | 完成判定と宣言 | 未在（これから作る） |
| 改変（文書） | `.kiro/specs/areka-P0-emo2-conformance-e2e/` の requirements／design／tasks／brief | 本仕様の記録 | 実在（brief は 220 行） |
| 改変（文書） | `doc/emo2-conformance-scope.md` | 実物定義の文書（`:24` の訂正＝R11.1・充足済み注記＝R11.2） | 実在・92 行 |
| 改変（文書） | `.kiro/steering/roadmap.md` | M1 の節を閉じる・干渉台帳の e2e 行を新前提へ書き換える・申し送り生存先の行を閉じる | 実在・150 行 |

**事前登記済みの例外 2 件（R12.1 が明示的に認めた範囲）**

| 例外 | パス | 触る内容 | 根拠 | 実在確認 |
|---|---|---|---|---|
| R9.2 | `crates/areka-ghost/tests/ghost/spine_e2e_test_s3_helper_liveness_detected.rs` | `:174-185` の「待たずに数える」形を、既存の手本（`crates/areka/src/emo2_boot/spine_boot_smoke_tests.rs:32-36`）と同じ「条件が満たされるまで待つ」形へ更新する。**隔離ではなく更新**（被覆を 1 つも失わない） | requirements.md 9.2・12.1／design.md D11 裁定表 系統 ⑴ | 実在・277 行。`:174-185` に `boot_prefix_len` を待ちなしで数える `assert_eq!` が在ることを逐語で確認 |
| R9.8 | 例外 A・例外 B（§1.2） | 理由付きの `#[ignore]` と環境変数の門を付ける。**判定ロジックは 1 行も変えない** | requirements.md 9.3・9.8・12.1／design.md D11「門の形（R9.3）」 | §1.2 のとおり実在。門を付ける先の 3 本（例外 A の 2 本・例外 B の 1 本）も §2 で逐語確認済み |

### 1.8 共有ファイルの相互確認（R12.3）

例外 2 ファイルの外で、本仕様の編集集合と併走の編集集合が**重なるファイルが 1 つある**。

| ファイル | 本仕様の用途 | 併走側 | 扱い |
|---|---|---|---|
| `.kiro/steering/roadmap.md` | M1 の節を閉じる・干渉台帳の e2e 行の書き換え・申し送り生存先の行を閉じる（R11.2・R11.4） | `.claude/worktrees/makoto-2-0-spec-launch-16e9c7`（分離 HEAD `1d21455f`）が同ファイルを改変済み（追記(91) 相当の起票行） | **後着が rebase して吸収する**。両者とも追記であり、節が異なる（本仕様＝M1 の節と干渉台帳／makoto＝M2 の起票追記）。本仕様が roadmap を触るのは相 2 の最終段（完成宣言）であり、その時点で `origin/main` へ引き直してから書く |

その他の照合（いずれも重なり 0）:

- `doc/emo2-conformance-scope.md` ——併走 9 本のうち、確定済み・未確定のどちらでも触れているブランチは 0 本。
- `crates/areka/src/emo2_boot/` ——同じく 0 本。
- `doc/COMPAT_ARCHITECTURE.md` ——`sakura-bare-tag-lexer` が未確定で 1 件持つが、**本仕様の編集集合には入っていない**ため重なりは生じない。

### 1.9 門を付けない対象（R9.7）

「門を持たないこと」を**自ら要件として固定している**テストには門を付けない。隔離は無差別には行わない。

| 例 | 逐語の確認（2026-09-03） |
|---|---|
| `crates/areka-emo-present/src/presenter/budget_tests.rs:50` | 実在・1,081 行。`:47-50` に「# 実時間を合否条件に使わない（Requirement 6.2）／本檻は時刻にも実行速度にも一切触れない（回数・ポインタ・寸法のみ）。純 x64 の常設テストであり、環境変数ゲートも `#[ignore]` も持たない（Requirement 6.3）」と書かれていることを確認した |

本仕様が門を付けるのは §2 の裁定表で「明示実行の門へ隔離する」と裁定した 3 本だけである。それ以外のテストは、間欠的に赤くなる疑いがあっても本仕様では触らない。

---

## 2. 裁定表（R9.1・R9.3）

design.md D11 の裁定表を本記録の正本として写す。**「走行結果」欄は §4 の妥当性の確認が終わってから埋める。**

| 系統 | 実体 | 裁定 | 理由 | 適用したコミット | 走行結果 |
|---|---|---|---|---|---|
| ⑴ 記録が非空になるのを待つだけで、直後の呼出数を待たずに数える | `crates/areka-ghost/tests/ghost/spine_e2e_test_s3_helper_liveness_detected.rs:174-185` | **更新する**（隔離しない） | 同じ確認を「条件が満たされるまで待つ」形で書いた手本が既に在る（`crates/areka/src/emo2_boot/spine_boot_smoke_tests.rs:32-36`）。移植は機械的で、**被覆を 1 つも失わない** | （未記入） | （未記入） |
| ⑵ 実窓の重なり順が他プロセスの可視窓に割り込まれる | `crates/wintf/src/ecs/window/zorder_pair_maintain_always_on_top_tests.rs:369`（`pair_fix_commands_keep_a_pair_inside_the_band_it_already_shares`）・同 `:740`（`the_top_of_normal_band_fix_keeps_a_real_owned_pair_adjacent_and_out_of_the_band`） | **明示実行の門へ隔離する** | 実窓の位置関係を直に測るため、環境の窓に割り込まれると崩れる。根治は判定の作り直しであり本仕様の範囲外 | （未記入） | （未記入） |
| ⑶ 画面同期の通知の壁時計期限が負荷で飢える | `crates/wintf/src/runtime/tick_bridge.rs:346`（`vblank_notifies_listener_then_joins_on_drop`・期限は `:353-356` の 500ms） | **明示実行の門へ隔離する** | 期限そのものが判定に効く形であり、待つ形へ書き換えると判定の意味が変わる（＝根治にあたる） | （未記入） | （未記入） |
| ⑷ `emo2_boot` 側の同形 2 本 | `crates/areka/src/emo2_boot/spine_boot_smoke_tests.rs:46-49`・`spine_talk_close_tests.rs:306-309` | **触らない** | いずれも「条件が揃うまで待つ」形で既に書かれており、期限は打ち切りの上限にすぎない | 該当なし | 該当なし |

**門の形（R9.3）**——既存の書き方をそのまま使う。理由付きの `#[ignore]` と環境変数の併用であり、手本は `crates/areka/src/placement/transition_signoff_tests.rs:58-61`。**判定ロジックは 1 行も変えない**（R9.5・R12.2）。

**行番号の逐語確認（2026-09-03・本ブランチ `84b84acb`）**——上表の 3 本はいずれも `#[test]` 属性の行が裁定表の行番号と一致することを確認した（例外 A の `:369`・`:740`、例外 B の `:346`）。門を付ける作業（task 4.3）の前に、同じ確認をもう一度行うこと。行番号は併走の rebase でずれ得る。

---

## 3. 失う被覆と引受先（R9.4）

> **骨格。** 本節の本体（research §10.1 の表の写し）は task 4.4 で埋める。ここでは design.md D11「失う被覆と引受先（R9.4）」が示す要旨と、引受先の実在だけを先に置く。

### 3.1 要旨（design.md D11 より）

- `zorder_pair_maintain_always_on_top_tests.rs:369` を止めると、3 つの挿入位置がいずれも常時最前面の帯の所属を変えないことを実 OS に対して主張しなくなる。対照 2 本（印の読み取りが常に真ではないこと・挿入位置が帯を跨ぐと OS が引き込むこと）も同時に止まり、残る単体判定が反証不能になる。
- 同 `:740` を止めると、実の所有リンクが実の持ち上げ後も隣接を保つことを測る唯一のテストが既定で走らなくなる。所有リンク無しの対照も止まる。
- `tick_bridge.rs:346` を止めると、実 DWM の垂直同期通知が待機側へ届くことと、同期スレッドが清く畳まれることの証跡が既定で止まる。隣の登録・命名だけを見るテストは代替にならない。

### 3.2 失う被覆の表（task 4.4 で埋める）

| 止めるテスト | 失う主張 | 同時に止まる対照 | 残る判定で代替できるか | 引受先 |
|---|---|---|---|---|
| `zorder_pair_maintain_always_on_top_tests.rs:369` | （未記入） | （未記入） | （未記入） | （未記入） |
| 同 `:740` | （未記入） | （未記入） | （未記入） | （未記入） |
| `tick_bridge.rs:346` | （未記入） | （未記入） | （未記入） | （未記入） |

### 3.3 引受先の実在確認（R7.6 と同じ規律）

| 引受先 | 項目 | 実在確認（2026-09-03） |
|---|---|---|
| `.kiro/specs/areka-P0-zorder-chain-residue/brief.md` | **A-1**（`:15`）＝既存ペア機構の実窓の檻が 3 プロセス同時 regime で稀に赤。実体として `zorder_pair_maintain_always_on_top_tests.rs:767`・同 `:411` を挙げている | 実在。A-1 の行を逐語で確認した |
| 同上 | **A-2**（`:16`）＝壁時計期限の飢餓。実体として `tick_bridge.rs:355`・`spine_boot_smoke_tests.rs:46`・`spine_talk_close_tests.rs:306` を挙げている | 実在。A-2 の行を逐語で確認した |
| 同上 `:34` | 分担の明記＝「A-1／A-2 は、e2e が先に踏んだ場合は e2e が隔離裁定（除外 or 更新）を行い根治は本 spec」 | 実在。逐語で確認した |

**本仕様は根治を行わない**（R9.5）。判定ロジックには触れない。

---

## 4. 妥当性の確認（R9.6）

> **骨格。** 走行は task 4.2（隔離前）と task 4.4（隔離後）で行い、結果をここへ書く。

### 4.1 走行回数の事前宣言

**隔離前に 3 回・隔離後に 3 回**。この 3 回は**上限であり、着手前に確定させたものである**。決着が付かない長時間の反復は行わない。

**3 回で決着しない場合の扱い**——「決着せず」と記録し、**裁定は隔離のままとする**。理由は、隔離の目的が完成判定の走行を汚さないことであって、間欠性の解明ではないためである（間欠性の解明は §3.3 の引受先が持つ）。

### 4.2 隔離前の走行（3 回・上限）

> **本節は門を付ける前（task 4.2）に採った生の結果である。** task 4.3 が門を付けた後は、既定の走行でこの 3 本が走らなくなるため、**同じ観測は二度と採れない**。本節が存在する理由はそこにある。

#### 4.2.1 走行前に定めた判定基準

**データを見る前に**次の 3 分類を確定させた（見てから基準を選ばないため）。

| 分類 | 条件 | 意味 |
|---|---|---|
| 決着（緑） | 3 回すべて緑 | この観測条件では赤が現れなかった |
| 決着（赤） | 3 回すべて赤 | 間欠ではなく常に赤（＝別の原因） |
| 決着せず | 緑と赤が混じる（赤が 1 回または 2 回） | 混在そのものが間欠性の実測。ただし 3 回では発生率を確定できない |

「決着せず」となった場合の扱いは §4.1 のとおり——**裁定は隔離のままとする**。

**緑で決着しても「間欠でない」ことの証明にはならない。** 3 回という上限（R9.6）は発生率を測る設計になっておらず、緑 3 回が言えるのは「この観測条件では赤を引かなかった」までである。

#### 4.2.2 対象 3 本の同定（本ブランチ HEAD ＝ `3e7414fc`・2026-09-04）

| 記号 | テスト関数名 | 現在の `file:line`（`#[test]` 属性の行） | design.md D11 の引用との差 |
|---|---|---|---|
| A1 | `pair_fix_commands_keep_a_pair_inside_the_band_it_already_shares` | `crates/wintf/src/ecs/window/zorder_pair_maintain_always_on_top_tests.rs:369`（`fn` は `:370`） | **ずれ無し**（D11 の `:369` と一致） |
| A2 | `the_top_of_normal_band_fix_keeps_a_real_owned_pair_adjacent_and_out_of_the_band` | 同 `:740`（`fn` は `:741`） | **ずれ無し**（D11 の `:740` と一致） |
| B1 | `vblank_notifies_listener_then_joins_on_drop` | `crates/wintf/src/runtime/tick_bridge.rs:346`（`fn` は `:347`） | **ずれ無し**（D11 の `:346` と一致） |

壁時計期限の位置も併せて確認した——B1 の 500ms 期限は `tick_bridge.rs:353-358`（`:353` が説明のコメント・`:354` が `wait_timeout(Duration::from_millis(500))`・`:355-358` が `assert!`）。requirements.md 9.3 の引用 `:353-356` はこの範囲の内側を指しており、矛盾しない。

**根治の引受先（residue brief）が挙げる行番号との関係**——`.kiro/specs/areka-P0-zorder-chain-residue/brief.md` の A-1（`:15`）は `:411`・`:767` を、A-2（`:16`）は `tick_bridge.rs:355` を挙げる。実測すると次のとおりで、**指している先は同じ 3 本である**（`#[test]` 行ではなく本体の判定行を挙げているだけ）。

| brief の引用 | 実測した所在 | 逐語 |
|---|---|---|
| `zorder_pair_maintain_always_on_top_tests.rs:411` | **A1 の本体の内側**（`:370` の `fn` に属する） | `assert!(` ——直下 `:412-413` が対照①の `!is_always_on_top(control)` と「対照①: 印の読み取りが常に真を返しているわけではない」 |
| 同 `:767` | **A2 の本体の内側**（`:741` の `fn` に属する） | `assert_eq!(` ——直下 `:768-770` が `measure_window_below(balloon)` と `Some(character)` と「持ち上がり幅によらず、バルーンはキャラのすぐ手前に居るはず（要件 1.2）」 |
| `tick_bridge.rs:355` | **B1 の本体の内側**（`:347` の `fn` に属する） | `assert!(` ——直下 `:356-357` が `got.is_some(),` と `"expected a vblank notify within 500ms (is DWM running?)"` |

同定の最終確認として `cargo test -p wintf --lib -- --list` を実行し、下記 3 つの完全名が実在すること（列挙 951 本のうちの 3 本）を確かめた。

- A1 ＝ `ecs::window::zorder_pair_maintain::always_on_top_tests::pair_fix_commands_keep_a_pair_inside_the_band_it_already_shares`
- A2 ＝ `ecs::window::zorder_pair_maintain::always_on_top_tests::the_top_of_normal_band_fix_keeps_a_real_owned_pair_adjacent_and_out_of_the_band`
- B1 ＝ `runtime::tick_bridge::tests::vblank_notifies_listener_then_joins_on_drop`

#### 4.2.3 走行の形

1 回の走行は **1 本だけ**を対象にする（3 本を互いから隔離するため）。コマンドの形は次のとおりで、`<完全名>` に §4.2.2 の 3 つを入れる。

```
cargo test -p wintf --lib -- --exact <完全名> --nocapture --test-threads=1
```

テスト実行体は `target\debug\deps\wintf-991cffaa4b12d45b.exe`。事前に `cargo test -p wintf --lib --no-run`（3 分 36 秒）で用意しており、**この構築は走行回数に数えない**。各回の出力が `Finished ... in 0.2s` で始まることが、走行中に再構築が起きていないことを示す。

#### 4.2.4 機械の状態

同一機械では別の Claude セッションが兄弟の作業ツリーで `cargo` を走らせ得るため、負荷は本来こちらの制御下に無い。**今回については、走行の直前と直後のいずれでも `cargo`／`rustc`／`link` のプロセスが 0 個であった**（`Get-Process` で計数）。動いていたのは `claude` 18 個と `node` 6 個のみである。作業ツリーは 10 本存在するが、この時間帯に構築やテストを走らせていたものは無い。**負荷を意図的に足しても避けてもいない**——素の状態をそのまま測った。

**これは観測条件の限界である。** design.md D11 が挙げる赤の機序は、⑵「実窓の重なり順を**他プロセスの可視窓**に割り込まれる」・⑶「壁時計期限が**負荷で**飢える」であり、いずれも他プロセスの窓と負荷が在る状態で現れる。今回の 3 回はその状態を欠いている。

#### 4.2.5 走行結果（生の値）

| 回 | 実施日時 | 対象（`--exact` の引数） | 終了コード | 集計 | 壁時計 | 結果 | 赤くなったテスト |
|---|---|---|---|---|---|---|---|
| A1-1 | 2026-09-04 19:30:33 | A1 | 0 | `1 passed; 0 failed; 0 ignored; 0 measured; 950 filtered out; finished in 0.01s` | 0.3s | **緑** | 無し |
| A1-2 | 2026-09-04 19:30:34 | A1 | 0 | `1 passed; 0 failed; 0 ignored; 0 measured; 950 filtered out; finished in 0.01s` | 0.3s | **緑** | 無し |
| A1-3 | 2026-09-04 19:30:34 | A1 | 0 | `1 passed; 0 failed; 0 ignored; 0 measured; 950 filtered out; finished in 0.01s` | 0.3s | **緑** | 無し |
| A2-1 | 2026-09-04 19:30:34 | A2 | 0 | `1 passed; 0 failed; 0 ignored; 0 measured; 950 filtered out; finished in 0.01s` | 0.3s | **緑** | 無し |
| A2-2 | 2026-09-04 19:30:35 | A2 | 0 | `1 passed; 0 failed; 0 ignored; 0 measured; 950 filtered out; finished in 0.01s` | 0.3s | **緑** | 無し |
| A2-3 | 2026-09-04 19:30:35 | A2 | 0 | `1 passed; 0 failed; 0 ignored; 0 measured; 950 filtered out; finished in 0.01s` | 0.3s | **緑** | 無し |
| B1-1 | 2026-09-04 19:30:35 | B1 | 0 | `1 passed; 0 failed; 0 ignored; 0 measured; 950 filtered out; finished in 0.00s` | 0.3s | **緑** | 無し |
| B1-2 | 2026-09-04 19:30:36 | B1 | 0 | `1 passed; 0 failed; 0 ignored; 0 measured; 950 filtered out; finished in 0.01s` | 0.3s | **緑** | 無し |
| B1-3 | 2026-09-04 19:30:36 | B1 | 0 | `1 passed; 0 failed; 0 ignored; 0 measured; 950 filtered out; finished in 0.00s` | 0.3s | **緑** | 無し |

各回の出力に `test <完全名> ... ok` の行が在ることを逐語で確認した（例：A1-1 は `test ecs::window::zorder_pair_maintain::always_on_top_tests::pair_fix_commands_keep_a_pair_inside_the_band_it_already_shares ... ok`）。**「1 passed」であって「0 passed」ではない**——名前の綴り違いで 1 本も走らないまま緑になる形ではないことを、この 1 行が排除している。

**赤の逐語出力は 1 件も無い。** 9 回すべてが `0 failed` であったため、「赤くなったテスト」欄の「無し」は**事象が発生しなかった**ことを意味する（記録漏れではない）。

走行は **3 本 × 3 回 ＝ 9 回で打ち切った**。R9.6 の上限どおりであり、結果が曖昧に見えても追加の走行は行っていない。

#### 4.2.6 分類（§4.2.1 の基準の適用）

| 記号 | 緑／赤の内訳 | 分類 | 適用した基準 |
|---|---|---|---|
| A1 | 緑 3・赤 0 | **決着（緑）** | 「3 回すべて緑」 |
| A2 | 緑 3・赤 0 | **決着（緑）** | 「3 回すべて緑」 |
| B1 | 緑 3・赤 0 | **決着（緑）** | 「3 回すべて緑」 |

#### 4.2.7 この結果が言うことと言わないこと

- **言うこと**——門を付ける前の 3 本は、負荷の無い機械で 1 本ずつ隔離して走らせる限り 3 回とも緑である。すなわち 3 本は壊れておらず、隔離は「常に赤いテストの握り潰し」ではない。
- **言わないこと**——間欠性が存在しないこと。今回の観測条件は §4.2.4 のとおり赤の機序（他プロセスの可視窓・負荷）を欠いており、3 回という上限は発生率を測れない。既知の赤の実測は本仕様の外（§3.3 の引受先 A-1・A-2）が持つ。
- ゆえに本節は §2 の裁定（系統 ⑵ ⑶ を明示実行の門へ隔離する）を**覆さない**。最終の裁定は §4.5 で下す（task 4.4）。

### 4.3 隔離後の走行（3 回・上限）

| 回 | 実施日時 | コマンド | 結果 | 赤くなったテスト |
|---|---|---|---|---|
| 1 | （未記入） | （未記入） | （未記入） | （未記入） |
| 2 | （未記入） | （未記入） | （未記入） | （未記入） |
| 3 | （未記入） | （未記入） | （未記入） | （未記入） |

### 4.4 門が効いていることの確認

門を付けたテストが**既定の走行で実際に走らなくなったこと**と、**環境変数を与えれば走ること**の両方を示す。片方だけでは門が効いた証拠にならない。

| 確認 | 期待 | 結果 |
|---|---|---|
| 既定の走行で 3 本が `ignored` に数えられる | （未記入） | （未記入） |
| 環境変数を与えた走行で 3 本が実行される | （未記入） | （未記入） |
| 隔離していない既存テストの本数が変わっていない | （未記入） | （未記入） |

### 4.5 裁定の結論

| 欄 | 値 |
|---|---|
| 隔離前に観測された間欠的な赤 | （未記入） |
| 隔離後に観測された間欠的な赤 | （未記入） |
| 決着したか | （未記入） |
| 最終の裁定（§2 の裁定を維持するか） | （未記入） |
| 完成判定（R10.7）へ渡せる状態か | （未記入） |
