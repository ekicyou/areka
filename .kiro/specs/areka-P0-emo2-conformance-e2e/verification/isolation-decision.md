# 間欠的な赤の隔離裁定の記録

> **本書は隔離裁定の記録である。** §1（着手前の確認）は着手時点で埋め切った節であり、実測とその再現手順を持つ。§2〜§4 は隔離の作業（相 2）で埋める節で、**task 4.4 の時点ですべての欄が埋まった**（裁定表・失う被覆と引受先・隔離前後の走行・門が効いていることの確認・裁定の結論）。最終の裁定は §4.5 に在る。
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

design.md D11 の裁定表を本記録の正本として写す。加えて、**実装中に実測された 2 系統（⑸ ⑹）を行として足した**——R9.1 は「既知の 3 系統について裁定する」と書くが、これは下限であって上限ではない。裁定を書かずに閉じると、間欠的な赤が引受先のないまま完成判定へ流れ込む（R8.5・R10.7）。

| 系統 | 実体 | 裁定 | 理由 | 適用したコミット | 走行結果 |
|---|---|---|---|---|---|
| ⑴ 記録が非空になるのを待つだけで、直後の呼出数を待たずに数える | `crates/areka-ghost/tests/ghost/spine_e2e_test_s3_helper_liveness_detected.rs:174-185`（是正前の番号） | **更新する**（隔離しない） | 同じ確認を「条件が満たされるまで待つ」形で書いた手本が既に在る（`crates/areka/src/emo2_boot/spine_boot_smoke_tests.rs:32-36`）。移植は機械的で、**被覆を 1 つも失わない** | `3e7414fc`（task 4.1） | 根因を `crates/areka-kanade/src/schedule/boot.rs:241` に特定（`Action::StartTalk` を `:279` の basewareversion 要求より先に積むため、cue の発火が 5 呼出の完了を含意しない）。自然発生は 348 走行でも再現せず、`notify("basewareversion")` を 200ms 遅らせる一時ラッパで決定論的な赤（`left: 4 / right: 5`＝2026-08-27 の署名と一致）を作って是正を証明した |
| ⑵ 実窓の重なり順が他プロセスの可視窓に割り込まれる | `crates/wintf/src/ecs/window/zorder_pair_maintain_always_on_top_tests.rs:369`（`pair_fix_commands_keep_a_pair_inside_the_band_it_already_shares`）・同 `:740`（`the_top_of_normal_band_fix_keeps_a_real_owned_pair_adjacent_and_out_of_the_band`）。**門の付与後は `:407`・`:781`** | **明示実行の門へ隔離する** | 実窓の位置関係を直に測るため、環境の窓に割り込まれると崩れる。根治は判定の作り直しであり本仕様の範囲外 | `1c76f2f5`（task 4.3） | 環境変数 `AREKA_WINTF_REAL_WINDOW_ZORDER` を与えた明示実行で **A1 3 回・A2 3 回とも緑**（§4.3）。既定の走行では `ignored` に落ちる（§4.4） |
| ⑶ 画面同期の通知の壁時計期限が負荷で飢える | `crates/wintf/src/runtime/tick_bridge.rs:346`（`vblank_notifies_listener_then_joins_on_drop`・期限は門の付与前 `:353-358`）。**門の付与後は `:383`・期限は `:393-398`** | **明示実行の門へ隔離する** | 期限そのものが判定に効く形であり、待つ形へ書き換えると判定の意味が変わる（＝根治にあたる） | `1c76f2f5`（task 4.3） | 環境変数 `AREKA_WINTF_VBLANK_DEADLINE` を与えた明示実行で **B1 3 回とも緑**（§4.3）。既定の走行では `ignored` に落ちる（§4.4） |
| ⑷ `emo2_boot` 側の同形 2 本 | `crates/areka/src/emo2_boot/spine_boot_smoke_tests.rs:46-49`・`spine_talk_close_tests.rs:306-309` | **触らない** | いずれも「条件が揃うまで待つ」形で既に書かれており、期限は打ち切りの上限にすぎない | 該当なし（コード無改変） | 該当なし |
| ⑸ 系統 ⑴ と同形の兄弟（`areka-ghost` の S1） | `crates/areka-ghost/tests/ghost/spine_e2e_test_s1_boot_success.rs:145-156`（`spin_pumping_ticks` が `!surface_records…is_empty()` だけを待つ）→ `:185-196`（直後に 5 要素の起動系列を等値照合） | **触らない**（本仕様では更新しない） | 機構も競走も系統 ⑴ と同一だが、**担当範囲の外**である——R12.1 の事前登記済み例外は S3 の 1 ファイルに限られ、design D11 ⑴ も S3 しか挙げない。根治が難しいからではなく、境界の外だから触らない（詳細は §2.1） | 該当なし（コード無改変） | 本タスクでは走らせていない（境界外） |
| ⑹ 本仕様が新設した檻が高負荷で期限切れになる | `crates/areka/src/emo2_boot/spine_conformance_support_tests.rs:556-557`（`kanade_probe_raises_no_shiori_call_and_observes_the_close`・作成コミット `b6323ffe`・2026-09-04・task 2.3） | **触らない（隔離しない）。ただし完成判定へ留保として渡す** | 門を付ければ本仕様が今回作った被覆を既定で失う。赤の中身は判定の誤りではなく機械の飢餓で、同じ飢餓は兄弟の常設テスト群にも同条件で現れる（詳細は §2.1） | 該当なし（コード無改変） | task 3.1 の負荷走行で **18 回中 4 回**、task 3.4 で **12 回中 1 回** 赤（`Timeout { stage: "終了指示", … }`）。本タスクでは追加の走行を採っていない（R9.6 の 3 回は ⑵ ⑶ の妥当性確認に費やした） |

**門の形（R9.3）**——既存の書き方をそのまま使う。理由付きの `#[ignore]` と環境変数の併用であり、手本は `crates/areka/src/placement/transition_signoff_tests.rs:58-61`。**判定ロジックは 1 行も変えない**（R9.5・R12.2）。

**行番号の逐語確認（2026-09-03・本ブランチ `84b84acb`）**——上表の 3 本はいずれも `#[test]` 属性の行が裁定表の行番号と一致することを確認した（例外 A の `:369`・`:740`、例外 B の `:346`）。門を付ける作業（task 4.3）の前に、同じ確認をもう一度行うこと。行番号は併走の rebase でずれ得る。**門を付けた後は 3 本とも下方向へずれている**（対応表は §3.2）。

### 2.1 ⑸ ⑹ の裁定の詳細（R9.1・R9.4）

表のセルに収まらない分をここへ置く。両件とも**根治は 1 行も行っていない**（R9.5）。

**⑸ `spine_e2e_test_s1_boot_success.rs`**

| 欄 | 値 |
|---|---|
| 実測の根拠 | 逐語で確認した（2026-09-04・本ブランチ `1c76f2f5`）。`:145-156` の `spin_pumping_ticks` は完了条件に `!surface_records.lock()…is_empty()` しか置いておらず、`:185-196` がその直後に 5 要素の起動系列（`OnInitialize` → username 先取り → `OnFirstBoot` → `OnBoot` → basewareversion）を `assert_eq!` で等値照合する |
| 機序 | 系統 ⑴ と同一。task 4.1 が特定した根因（`crates/areka-kanade/src/schedule/boot.rs:241` の `Action::StartTalk` が `:279` の basewareversion 要求より先に積まれる）は S1 にもそのまま当たる——cue の発火は 5 呼出の完了を含意しない |
| 裁定 | **触らない** |
| 理由 | R12.1 の事前登記済み例外は S3 の 1 ファイルに限られ、design D11 ⑴ も S3 しか挙げない。触れば境界違反になる。**「直せない」ではなく「本仕様の担当ではない」** |
| 失う被覆 | **無し**。門を付けないので既定の走行で走り続ける |
| 残る危険 | 同じ競走で間欠的に赤くなりうること。失敗の署名は Vec の不一致であり、S3 の `left: 4 / right: 5` とは別の形（要素数も内容も S1 固有） |
| 引受先 | **無し**（§3.3.1 の走査で確定）。新規起票が要る |

**⑹ `spine_conformance_support_tests.rs`（本仕様が新設した檻）**

| 欄 | 値 |
|---|---|
| 実測の赤 | task 3.1 の負荷走行で 18 回中 4 回、task 3.4 で 12 回中 1 回。いずれも `Timeout { stage: "終了指示", … }` |
| 待ちの形（本タスクで逐語確認） | 壁時計 30 秒（`SPIN_WAIT`）の上限つきで、1 周ごとに 200µs 眠る——`crates/areka/src/emo2_boot/spine_conformance_support.rs:516`（`let deadline = clock() + SPIN_WAIT;`）・`:590`（`if clock() >= deadline`）・`:654`（`std::thread::sleep(Duration::from_micros(200))`）。**上限は壁時計であって反復回数ではない**。tasks.md の Implementation Notes は「待ちの予算が反復回数」と書くが、実体は壁時計期限であり、赤の正体は「別スレッドの前進が 30 秒以内に届かない」＝期限そのものの飢餓である（この点だけ申し送りを訂正する） |
| 裁定 | **触らない（隔離しない）。ただし完成判定へ留保として渡す** |
| 理由 ⑴ | 門を付けると、本仕様が task 2.3／3.x で積み上げた被覆（駆動器が探りで終了を捉えること・握手が 1 度ずつであること）が既定の走行から消える。**自分が作った檻を自分で既定から外すのは、被覆を得た作業そのものを打ち消す** |
| 理由 ⑵ | 赤の中身は判定の誤りではなく機械の飢餓である。同じ飢餓は兄弟の常設テスト群にも同条件で現れており（task 3.1 の負荷走行で `talk_close_tests`・`text_scale_tests`、別走行で `seriko_loop_tests` 2 種・`display_tests`）、**1 本だけ門を付けるのは無差別隔離の裏返しの恣意になる**（R9.7 の趣旨＝隔離は無差別に行わない） |
| 理由 ⑶ | 待ちの形は design D11 ⑷ が「触らない」と裁定した 2 本と同じ「条件が揃うまで待ち、期限は打ち切りの上限」である。同じ裁定が素直に当たる |
| 理由 ⑷ | 飢餓に強い待ち方へ作り直すのは根治であり R9.5 が禁じている |
| 失う被覆 | **無し**（走り続ける） |
| 残る危険 | R10.1 の「ワークスペース全体のテストが成功で終わること」を直接汚す。高負荷の走行では低い確率で赤が出る |
| 完成判定への渡し方 | **留保つきで渡す**（§4.5・R10.6 の形＝未達・理由・引受先を明記したうえで宣言し、判定結果そのものを書き換えない） |
| 引受先 | **無し**（§3.3.1 の走査で確定）。新規起票が要る |

---

## 3. 失う被覆と引受先（R9.4）

### 3.1 要旨（design.md D11 より）

- `zorder_pair_maintain_always_on_top_tests.rs:369` を止めると、3 つの挿入位置がいずれも常時最前面の帯の所属を変えないことを実 OS に対して主張しなくなる。対照 2 本（印の読み取りが常に真ではないこと・挿入位置が帯を跨ぐと OS が引き込むこと）も同時に止まり、残る単体判定が反証不能になる。
- 同 `:740` を止めると、実の所有リンクが実の持ち上げ後も隣接を保つことを測る唯一のテストが既定で走らなくなる。所有リンク無しの対照も止まる。
- `tick_bridge.rs:346` を止めると、実 DWM の垂直同期通知が待機側へ届くことと、同期スレッドが清く畳まれることの証跡が既定で止まる。隣の登録・命名だけを見るテストは代替にならない。

**3 つに共通する形**——止まるのは主張だけではない。**対照が同時に止まる**ので、残った判定が生きているかどうかを確かめる手立ても一緒に失われる。これが隔離の本当の代償である。

### 3.2 失う被覆の表（research.md §10.1 の写し）

research.md §10.1 の 3 行をそのまま写し、**門の付与でずれた現在の行番号を併記した**（研究時の番号は門を付ける前のものである）。

| 止めるテスト | 失う主張 | 同時に止まる対照 | 残る判定で代替できるか | 引受先 |
|---|---|---|---|---|
| `zorder_pair_maintain_always_on_top_tests.rs:369`（本体は研究時 `:370-444`・**現在は `#[test]` `:407`／本体 `:409-470`**） | 実の最上位窓 4 枚を作り、3 つの挿入位置指令を実 `SetWindowPos` で流す。**常設走行が「3 つの挿入位置はいずれも帯の所属を変えない」ことを実 OS に対して主張しなくなる** | 対照①＝印の読み取りが常に真を返していないこと（研究時 `:411-414`／**現在 `:452-455`**）・対照②＝帯の内側の窓の直前へ挿すと OS が帯へ引き込むこと（研究時 `:416-424`／**現在 `:457-465`**） | **できない**。対照 2 本が同時に止まるため、残る単体判定は反証不能になる | `.kiro/specs/areka-P0-zorder-chain-residue/brief.md` **A-1**（`:15`） |
| 同 `:740`（本体は研究時 `:741-794`・**現在は `#[test]` `:781`／本体 `:783-838`**） | 実の所有リンク（`set_window_owner`）を張った実窓 2 枚に `TopOfNormalBand` を適用し、「バルーンがキャラのすぐ手前」を測る（研究時 `:767`／**現在 `:811`**）。**実の所有リンクが実の持ち上げ後も隣接を保つことを測る唯一のテスト**が既定で走らなくなる | 所有リンク無しの対照（`assert_ne!`・研究時 `:775-788`／**現在 `:819-832`**）＝隣接が「所有リンクのおかげ」か「指令の副作用」かを分ける対照 | **できない**（唯一のテストであり、切り分けの対照も同時に止まる） | 同 **A-1**（`:15`） |
| `tick_bridge.rs:346`（本体は研究時 `:346-362`・**現在は `#[test]` `:383`／本体 `:385-402`**） | 500ms の壁時計期限（研究時 `:353-354`／**現在 `:393-394`**）を置いて通知の到達を主張し（研究時 `:355-358`／**現在 `:395-398`**）、`drop` が停止→join を兼ねる（研究時 `:360-361`／**現在 `:400-401`**）。**実 DWM の垂直同期通知が待機側へ届くことと、`wintf-vsync` スレッドが清く畳まれることの証跡**が既定で止まる | 同一テスト内の対照は無い。隣の `vsync_thread_registers_itself_with_the_vblank_role`（研究時 `#[test]` `:325`／**現在 `:362`**）は登録・命名しか見ないので**代替にならない** | **できない**。実 DWM の通知が届くことも、同期スレッドが清く畳まれることも、既定で主張する相手が居なくなる | 同 **A-2**（`:16`） |

**研究時の行番号のうち 1 件は当時から実体とずれていた**——research §10.1 は A1 の本体を `:370-444` と書くが、実測すると本体の閉じ括弧は門の付与後で `:470`＝門の付与前に直すと `:429` である（終端が 15 行過大）。始端（`:370`）と、対照①②の行域（`:411-414`・`:416-424`）は実測と一致する。**主張の中身は変わらない**ので、表には研究時の記載と実測の両方を残した。

### 3.3 引受先の実在確認（R7.6 と同じ規律）

| 引受先 | 項目 | 実在確認（2026-09-04・逐語） |
|---|---|---|
| `.kiro/specs/areka-P0-zorder-chain-residue/brief.md` | **A-1**（`:15`）＝既存ペア機構の実窓の檻が 3 プロセス同時 regime で稀に赤。実体として `zorder_pair_maintain_always_on_top_tests.rs:767`・同 `:411` を挙げている | 実在。`:15` を逐語で確認した |
| 同上 | **A-2**（`:16`）＝壁時計期限の飢餓。実体として `tick_bridge.rs:355`・`spine_boot_smoke_tests.rs:46`・`spine_talk_close_tests.rs:306` を挙げている | 実在。`:16` を逐語で確認した |
| 同上 `:34` | 分担の明記＝「A-1／A-2 は、**e2e が先に踏んだ場合は e2e が隔離裁定（除外 or 更新）を行い根治は本 spec**」 | 実在。`:34` を逐語で確認した |

**本仕様は根治を行わない**（R9.5）。判定ロジックには触れない。

**引受先の brief が引く行番号は門の付与でずれた（引き直さず申し送りとして渡す）**——task 4.3 の全域走査が確定させたとおり、`zorder_pair_maintain_always_on_top_tests.rs:411`→`:452`・同 `:767`→`:811`・`tick_bridge.rs:355`→**`:395`**。`brief.md:15-16` の引用は門を付ける前の番号のままである。同 brief は**本仕様の編集集合の外**なので書き換えない。なお tasks.md の Implementation Notes は `tick_bridge.rs:355`→`:396` と書くが、本タスクの実測では `:395` が `assert!(` の行で、`:396` はその 1 行下の `got.is_some(),` である（1 行ぶん過大）。ずれの実測値は本節を正とすること。

#### 3.3.1 ⑸ ⑹ には引受先が無い（R9.4・R8.5）

上の A-1／A-2 は ⑸ ⑹ の受け皿に**ならない**。名指しの範囲と機序の両方で外れる。

| 件 | A-1 に入るか | A-2 に入るか | 判定 |
|---|---|---|---|
| ⑸ `spine_e2e_test_s1_boot_success.rs` | **入らない**。A-1 は wintf の重なり順専用（名指しは `zorder_pair_maintain_always_on_top_tests.rs:767`／`:411` の 2 件のみ） | **入らない**。A-2 は「壁時計期限の飢餓」＝**待つ形で書かれているのに有界内に届かない**形であり、名指しは `tick_bridge.rs:355`・`spine_boot_smoke_tests.rs:46`・`spine_talk_close_tests.rs:306` の 3 件。⑸ は `areka-ghost` の S1 であり、機序も「待たずに数える」競走であって飢餓ではない | **引受先なし** |
| ⑹ `spine_conformance_support_tests.rs` | **入らない**（wintf ではない） | **入らない**。機序は A-2 と同類（飢餓）だが、⑹ のファイルは brief の起票（2026-09-02）より後に**本仕様が作った**（作成コミット `b6323ffe`・2026-09-04）ため、名指しに含まれ得ない。台帳 spec の項目は zsp research §13.8 の転記で固定されており、**本仕様が別 spec の brief に項目を足すことはできない**（編集集合の外） | **引受先なし** |

**他 spec の走査（較正つき）**——`.kiro/specs/**/*.md` を `spine_e2e_test_s1`／`spine_conformance_support_tests`／`SPIN_WAIT` で走査した。一致は**完了置き場（`completed/`）の 3 spec のみ**（`file-slimming`・`test-cage-determinism`・`kero-balloon`）で、進行中の spec には 1 件も無い。**完了 spec は申し送りを消化できない**（記憶 `deferral-requires-verified-owner`）。走査が生きていることは、この一致そのものが示している（「0 件」だけを根拠にしていない）。加えて `間欠` で brief を走査すると、進行中の spec で一致するのは本仕様と `areka-P0-zorder-chain-residue` の 2 本だけである。

**⇒ 開発者の裁定が要る。** 選べる形は 2 つある。どちらも本仕様の編集集合の外なので、本タスクでは実行していない。

1. **台帳 spec へ項目を足す**——`.kiro/specs/areka-P0-zorder-chain-residue/brief.md` に A-3（⑸＝「待たずに数える」競走の兄弟）・A-4（⑹＝適合の駆動器の檻の飢餓）を足す。⑹ の機序は A-2 と同類なので収まりは良い。ただし同 brief の Current State は zsp research §13.8 の転記であり、転記でない項目を足すなら台帳の性格を一段広げる判断になる。
2. **新規に起票する**——「常設テストの飢餓耐性」を主題にした spec を 1 本立て、⑸ ⑹ と task 3.1 が観測した兄弟群（`talk_close_tests`・`text_scale_tests`・`seriko_loop_tests`・`display_tests`）をまとめて持たせる。⑹ の実測（18 回中 4 回）が示すとおり、これは 1 本の欠陥ではなく**待ち方の類型**の問題である。

新しい引受先が持つべき中身は次の 3 点である——⑴ ⑸ の競走を S3 と同じ「待つ形」へ揃えること（根因は `boot.rs:241` で特定済み）、⑵ 高負荷で 30 秒の壁時計期限が飢える待ち方の類型を、飢餓に強い形へ作り直すこと、⑶ その作り直しが既存の被覆を減らさないことを示すこと。

---

## 4. 妥当性の確認（R9.6）

> 走行は task 4.2（隔離前・§4.2）と task 4.4（隔離後・§4.3）で行い、結果をここへ書いた。**前後とも赤の機序が不在の静かな機械で採られている**ため、裁定の主証跡は §4.4（門の機械的なふるまい）に置く。

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

> **この 3 回 × 3 本は「隔離の効果」を測っていない。** 隔離前の 9 回（§4.2）は `cargo`／`rustc` が 0 個の静かな機械で採られ、design D11 が挙げる赤の機序——⑵ 他プロセスの可視窓の割り込み・⑶ 負荷による壁時計期限の飢餓——が**どちらも不在**だった。隔離後も同じ条件で採っている（下記「機械の状態」）。同一条件で前後を並べて測れるのは**条件の同一性**であって、隔離の効果ではない。**ゆえに裁定の主証跡は本節ではなく §4.4（門の機械的なふるまい）に置く。** 本節が示すのは「門を付けた後も、明示実行すれば 3 本は従来どおり緑で走る」——すなわち門の付与が判定を壊していないこと、それだけである。

#### 4.3.1 走行の形

1 回の走行は **1 本だけ**を対象にする（§4.2.3 と同じ形）。門を付けた後は `--ignored` と**対応する環境変数**が要る。

```
cargo test -p wintf --lib -- --exact <完全名> --ignored --nocapture --test-threads=1
```

| 記号 | 完全名 | 与えた環境変数 |
|---|---|---|
| A1 | `ecs::window::zorder_pair_maintain::always_on_top_tests::pair_fix_commands_keep_a_pair_inside_the_band_it_already_shares` | `AREKA_WINTF_REAL_WINDOW_ZORDER=1` |
| A2 | `ecs::window::zorder_pair_maintain::always_on_top_tests::the_top_of_normal_band_fix_keeps_a_real_owned_pair_adjacent_and_out_of_the_band` | 同上 |
| B1 | `runtime::tick_bridge::tests::vblank_notifies_listener_then_joins_on_drop` | `AREKA_WINTF_VBLANK_DEADLINE=1` |

**環境変数を与えないまま無差別に `--ignored` を回すと 3 本は赤になる**（task 4.3 の設計どおり——素通りの緑が「実窓で測った」という記録に化けるのを防ぐため、門が閉じたままの呼出は失敗させる）。ゆえに 1 本ごとに対応する変数を与えている。

事前に `cargo test -p wintf --lib --no-run` を実行した（0.3 秒＝再構築なし）。実行体は `target\debug\deps\wintf-991cffaa4b12d45b.exe` で、**§4.2.3 と同一のファイル名である**。この構築は走行回数に数えない。

#### 4.3.2 機械の状態

| 時点 | `cargo` | `rustc` | `claude` | `node` |
|---|---|---|---|---|
| 9 回の直前（20:08:28） | 0 | 0 | 18 | 6 |
| 9 回の直後（20:08:31） | 0 | 0 | 18 | 6 |

§4.2.4 と同じ静かな条件である。**負荷を意図的に足しても避けてもいない**——素の状態をそのまま測った。前段の警告のとおり、この条件には赤の機序が不在である。

#### 4.3.3 走行結果（生の値）

| 回 | 実施日時 | 対象 | 終了コード | 集計 | 壁時計 | 結果 | 赤くなったテスト |
|---|---|---|---|---|---|---|---|
| A1-1 | 2026-09-04 20:08:28 | A1 | 0 | `1 passed; 0 failed; 0 ignored; 0 measured; 950 filtered out; finished in 0.01s` | 0.3s | **緑** | 無し |
| A1-2 | 2026-09-04 20:08:28 | A1 | 0 | `1 passed; 0 failed; 0 ignored; 0 measured; 950 filtered out; finished in 0.01s` | 0.3s | **緑** | 無し |
| A1-3 | 2026-09-04 20:08:28 | A1 | 0 | `1 passed; 0 failed; 0 ignored; 0 measured; 950 filtered out; finished in 0.01s` | 0.4s | **緑** | 無し |
| A2-1 | 2026-09-04 20:08:29 | A2 | 0 | `1 passed; 0 failed; 0 ignored; 0 measured; 950 filtered out; finished in 0.01s` | 0.3s | **緑** | 無し |
| A2-2 | 2026-09-04 20:08:29 | A2 | 0 | `1 passed; 0 failed; 0 ignored; 0 measured; 950 filtered out; finished in 0.01s` | 0.3s | **緑** | 無し |
| A2-3 | 2026-09-04 20:08:29 | A2 | 0 | `1 passed; 0 failed; 0 ignored; 0 measured; 950 filtered out; finished in 0.01s` | 0.3s | **緑** | 無し |
| B1-1 | 2026-09-04 20:08:30 | B1 | 0 | `1 passed; 0 failed; 0 ignored; 0 measured; 950 filtered out; finished in 0.01s` | 0.3s | **緑** | 無し |
| B1-2 | 2026-09-04 20:08:30 | B1 | 0 | `1 passed; 0 failed; 0 ignored; 0 measured; 950 filtered out; finished in 0.00s` | 0.3s | **緑** | 無し |
| B1-3 | 2026-09-04 20:08:30 | B1 | 0 | `1 passed; 0 failed; 0 ignored; 0 measured; 950 filtered out; finished in 0.01s` | 0.3s | **緑** | 無し |

各回の出力に `test <完全名> ... ok` の行が在ることを逐語で確認した（例：B1-1 は `test runtime::tick_bridge::tests::vblank_notifies_listener_then_joins_on_drop ... ok`）。**「1 passed」であって「0 passed」ではない**——名前の綴り違いで 1 本も走らないまま緑になる形ではないことを、この 1 行が排除している（較正は §4.4 の 4 行目）。

**「0 ignored」の読み方**——`--ignored` を付けた走行では、門を付けたテストは *ignored* ではなく *passed* に数えられる。集計行の `0 ignored` は「無視されたものが無い」＝**指定した 1 本が実際に実行された**ことを意味する。

**赤の逐語出力は 1 件も無い。** 9 回すべてが `0 failed` であったため、「赤くなったテスト」欄の「無し」は**事象が発生しなかった**ことを意味する（記録漏れではない）。失敗したテスト名を取り逃がさないよう、9 回とも出力をファイルへ退避してから集計行と `... ok` 行を抜き出している（task 2.3 で名前を取り損ねた 1 回の教訓）。

走行は **3 本 × 3 回 ＝ 9 回で打ち切った**。R9.6 の上限どおりであり、追加の走行は行っていない。

#### 4.3.4 分類（§4.2.1 の基準の適用）

| 記号 | 緑／赤の内訳 | 分類 | 適用した基準 |
|---|---|---|---|
| A1 | 緑 3・赤 0 | **決着（緑）** | 「3 回すべて緑」 |
| A2 | 緑 3・赤 0 | **決着（緑）** | 「3 回すべて緑」 |
| B1 | 緑 3・赤 0 | **決着（緑）** | 「3 回すべて緑」 |

**隔離前（§4.2.6）と同じ分類である。** 前後で分類が変わらないこと自体が、上の警告——この観測条件では隔離の効果が測れない——の裏づけになっている。判定が壊れていないことは言えるが、間欠性が減ったとは言えない。

### 4.4 門が効いていることの確認

門を付けたテストが**既定の走行で実際に走らなくなったこと**と、**環境変数を与えれば走ること**の両方を示す。片方だけでは門が効いた証拠にならない。**§4.3 の走行回数ではなく、本節が裁定の主証跡である。**

| 確認 | 期待 | 結果 |
|---|---|---|
| 既定の走行で 3 本が `ignored` に数えられる | `cargo test -p wintf --lib` が `3 ignored` を報告し、**その 3 本の名前が逐語で出る** | **一致**。2026-09-04 20:07:46・終了コード 0・1.64s・`test result: ok. 948 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out`。`... ignored` 行の名前は A1・A2・B1 の 3 本で、いずれも門の理由文（`AREKA_WINTF_REAL_WINDOW_ZORDER を与えて明示実行する` ／ `AREKA_WINTF_VBLANK_DEADLINE を与えて明示実行する`）を伴う |
| 環境変数を与えた走行で 3 本が実行される | 1 本ごとに `1 passed` と `test <完全名> ... ok` | **一致**。9 回すべてで両方を確認した（§4.3.3） |
| 隔離していない既存テストの本数が変わっていない | 列挙総数は 951 のまま・成功が**ちょうど 3 本だけ**減る（951 → 948）・新たな赤は 0 | **一致**。付与後は `running 951 tests` から `948 passed; 0 failed; 3 ignored`。列挙総数は §4.2.2 の `--list`（951 本）と同じで、差はちょうど 3、`0 failed` も保たれている。**門の付与前が `0 ignored` だったことは構造で確かめられる**——`git grep -c '#\[ignore' HEAD~1 -- 'crates/wintf/src/**'` の一致は 2 ファイルのみで、どちらもドキュメントコメント中の言及であり、`#[ignore]` **属性**は 0 件だった（対象 2 ファイル単体でも 0 件）。ゆえに 951 本すべてが走っていた |
| 終了コードだけでは判定に使えない（較正） | 綴りを 1 字違えた `--exact` でも終了コードは 0 になるはず | **そのとおりだった**。`…it_already_sharesXX` を与えると `test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 951 filtered out` で**終了コード 0**。ゆえに §4.3 の判定に使ったのは「1 passed」と `... ok` の行であって、終了コードではない |
| 本タスクが門を足しても外してもいない | `#[ignore]` 属性の実数が 3 のまま・`crates/` に差分 0 | **一致**。`crates/wintf/src` の `#[ignore` 一致は 5 件だが、うち 2 件はドキュメントコメント中の言及（`hit_test_shared_mask_tests.rs:12`・`alpha_mask_regenerate_tests.rs:15` の「`#[ignore]` なし（要件 6.3）」）で、**属性は 3 件**（`zorder_pair_maintain_always_on_top_tests.rs:408`・同 `:782`・`tick_bridge.rs:384`）。`git diff --numstat -- crates/` は空であり、同じ pathspec が `HEAD~1`（task 4.3）に対しては 2 ファイル 84 行の追加を返す＝**pathspec が実在を解決していることを先に証明したうえでの「差分なし」**である |

### 4.5 裁定の結論

| 欄 | 値 |
|---|---|
| 隔離前に観測された間欠的な赤 | **0 件**（3 本 × 3 回＝9 回すべて緑）。**ただしこの 0 件は、赤の機序が不在の観測条件で採られている**——`cargo`／`rustc` が 0 個の静かな機械であり、design D11 が挙げる 2 つの機序（他プロセスの可視窓の割り込み・負荷による壁時計期限の飢餓）がどちらも無かった（§4.2.4）。**ゆえに「0 件」は隔離が不要であることの証拠にはならない。** 既知の赤の実測は本仕様の外（§3.3 の A-1・A-2）が持つ |
| 隔離後に観測された間欠的な赤 | **0 件**（同じく 9 回すべて緑）。観測条件は隔離前と同じで、上と同じ限界がそのまま当てはまる（§4.3 冒頭） |
| 決着したか | §4.2.1 の基準では 3 本とも「決着（緑）」。**決着したのは「この観測条件では赤を引かなかった」ことだけ**であり、間欠性の有無は決着していない。R9.6 の上限どおり前後 3 回ずつで打ち切り、決着の付かない反復は行っていない |
| 最終の裁定（§2 の裁定を維持するか） | **維持する**。⑴ 更新する（`3e7414fc`）／⑵ ⑶ 明示実行の門へ隔離する（`1c76f2f5`）／⑷ 触らない／⑸ 触らない（境界外）／⑹ 触らない＋完成判定へ留保。**裁定を支えているのは走行回数ではなく §4.4 の門の機械的なふるまい**——既定の走行から 3 本がちょうど落ち（951 → 948 passed・3 ignored）、環境変数を与えれば従来どおり走る（1 passed ＋ `... ok`）。走行 9 回は「門の付与が判定を壊していない」ことしか言わない |
| 失う被覆 | §3.2 のとおり。3 本とも**対照が同時に止まる**ため、残る判定の生死を確かめる手立ても一緒に失う。引受先は A-1（重なり順 2 本）・A-2（画面同期 1 本） |
| 根治を行ったか | **行っていない**（R9.5）。判定ロジックは 1 行も変えていない。本タスク（4.4）は `crates/**` を 1 行も編集しておらず、門を足しても外してもいない（§4.4 の最終行） |
| 完成判定（R10.7）へ渡せる状態か | **条件付きで渡せる。** ⑴〜⑷ は裁定済みで、間欠的な赤は既定の走行から外れた。⑸ ⑹ も裁定済みだが**引受先が無い**（§3.3.1）。とくに ⑹ は本仕様が新設した檻で、高負荷での赤が実測されている（18 回中 4 回・12 回中 1 回）——R10.1 の「ワークスペース全体のテストが成功で終わること」を直接汚しうる |

#### 4.5.1 完成判定（task 7.2）への申し送り

R10.7 は「隔離裁定の後に完成判定を行い、間欠的な赤が残ったまま『全通過』と記録しない」と定める。⑹ は本記録で裁定済み（触らない）であるから R10.7 の順序は満たすが、**赤が消えたわけではない**。ゆえに次の形を推奨する。

- **推奨する形**——task 7.2 は R10.6 の形で宣言する。すなわち ⑴ 全体テストの結果はそのまま記録し（判定結果を書き換えない）、⑵ 留保として「⑹ は高負荷で低確率に期限切れになる実測がある。裁定は §2 の ⑹、引受先は未定（§3.3.1）」を併記する。緑が出るまで回し直さない（design D10 の手順 2 は同一コミットでの再走を **1 回だけ**に限っている）。
- **推奨しない形 ⑴**——⑹ に門を付けて既定から外す。R9.3 が門を付ける対象として挙げるのは ⑵ ⑶ の 3 本だけであり、⑹ への門は**裁定の無い隔離**になる。加えて本仕様が作った被覆を本仕様が消すことになり、R9.7 の「隔離は無差別に行わない」の趣旨にも反する。
- **推奨しない形 ⑵**——留保を書かずに「全通過」と記録する。R10.7 に正面から反する。
- **開発者へ**——⑸ ⑹ の引受先が無い（§3.3.1）。台帳 spec へ項目を足すか、新規に起票するかは開発者の裁定であり、本仕様では実行していない（どちらも編集集合の外）。
