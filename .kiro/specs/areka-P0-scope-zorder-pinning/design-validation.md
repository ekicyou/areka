# 設計検証レポート: areka-P0-scope-zorder-pinning

**実施**: 2026-08-27 ／ **入力**: design.md・requirements.md（確定・議題 3 件裁定済）・research.md（§9 再実測込み）・brief.md・`.kiro/steering/` ／ **言語**: ja（spec.json）
**方式**: 非対話（design-review.md の Analysis → Critical Issues → Strengths → GO/NO-GO）。設計中の file:line 主張は本ワークツリーの実コードで抜き取り検証した。

## Design Review Summary

案 C（グループの正本と解釈は areka・維持は wintf の新設 1 系統・既存 `zorder_pair*` 5 ファイルは無編集）は、要件 9.5（既存観測語彙の保存）と要件 6.4（既定＝非強制）を**構造で**満たしており、層の分界・依存方向・記録規律・檻の作法のいずれも既存の正典と整合する。要件 1〜13 の全 65 受入基準が Requirements Traceability 表に行を持ち、裁定 3 件（不可視＝Windows 基準／誤記訂正は COMPAT §8／追跡 spec＋sylphya 非接触）はいずれも設計本文に反映済みである。ただし**診断行の発行時点**と**頭打ちの粒度**に、実装がそのまま書くと実機サインオフの証跡と収束性を損なう欠陥が 1 件ずつあり、加えて先送り語彙檻の対象ファイルが 1 本漏れている。

## Critical Issues（3 件）

### 🔴 Critical Issue 1: `[zorder-group] fix` 行を「発行直後の実測」で組むと、証跡が書込前の値になる

**Concern**: 設計は group 維持系 ④ で連鎖発行し、group diag の項で `fix` 行を「発行直後の実測隣（pair `fix_line:131` と同型）」と規定している。しかし先例の実体は同型ではない——`fix_line` を実際に呼ぶのは `zorder_pair.rs:858` の `record_verification` であり、**次巡の検証で一致したときにだけ `fix`（debug）を、不一致なら `verify-failed`（error）を出す**。指令の書込は巡の後の flush で起きるため、発行と同じ巡で測れば必ず**書込前の重なり**が載る。
**Impact**: 要件 9.2（指令とその直後の実測を同一行）が字面だけ満たされ、要件 9.4 の実機サインオフが「`fix` 行が出た＝成立した」と読むと**偽の成立証跡**になる。過去に同型の誤診があったことが `zorder_pair_diag.rs:126-129` に明記されている。
**Suggestion**: group 維持系 ① の検証段で `fix`／`verify-failed` を出す形へ統一し（発行時は `applied`／計画のみ）、設計本文の「発行直後の実測隣」を「次巡の検証で採った実測隣」へ改める。サインオフ判定語（§Testing の ⑴）も検証行を読む形へ揃える。
**Traceability**: 9.1, 9.2, 9.4
**Evidence**: design.md「group 維持系（`zorder_group_maintain.rs`）」1 巡の処理 ①④／「group diag（`zorder_group_diag.rs`）」／「Testing Strategy > 実機サインオフ」

### 🔴 Critical Issue 2: `fail_streak` が全グループ共有＝1 グループの不成立が他グループの是正を止める

**Concern**: `ZOrderGroups` は `fail_streak: u8` を**Resource に 1 本**持ち、「`fail_streak >= 3` で warn を出し pending を降ろす」。一方、是正は「1 巡 1 グループ」であり、検証も発行したグループ単位で起きる。実現不能なグループ（例: 明示モードで片方の窓だけを並べ、OS の owner 制約に当たる形）が 1 つあると、その失敗 3 回で pending が全体的に降り、**同時に有効な他グループの是正が新しいトリガが来るまで止まる**。
**Impact**: 要件 1.1／1.3／7.1 の「グループが有効な間は指定順を保つ」が、無関係なグループの失敗によって静かに破れる。記録は 1 行出るので要件 8.3 は満たすが、原因の帰属が付かない。
**Suggestion**: `fail_streak` をグループ ID ごとに持ち（`HashMap<u32,u8>` か `ZOrderGroupSpec` の同居フィールド）、頭打ちは**そのグループだけ**を維持対象から外す形にする。pending は他グループが残っている限り降ろさない。併せて頭打ち warn に group_id と観測順を載せる。
**Traceability**: 1.1, 1.3, 7.1, 8.3
**Evidence**: design.md「ZOrderGroups Resource＋純判断」State 定義（`fail_streak: u8`）／「group 維持系」頭打ちの節

### 🔴 Critical Issue 3: 先送り語彙檻の対象に新設本番ファイルが 1 本漏れている

**Concern**: File Structure Plan の areka 新設**本番**ファイルは `placement/zorder_group_ledger.rs`・`emo2_boot/zorder_cue.rs`・`emo2_boot/frame/zorder_drain.rs` の 3 本だが、Modified Files は `spawn_zorder_pair_deferred_tests.rs` の `PRODUCTION_FILES` を **2→4**（ledger・zorder_cue のみ）とし、Traceability 10.4 も「新設 5 本（wintf 3・areka 2）」と数えている。実ファイル（`spawn_zorder_pair_deferred_tests.rs:48` = `[&str; 2]`）に対し正しくは **2→5**、総数は 6 本である。
**Impact**: 要件 10.4／11.4 は「新しい実装ファイルも検査対象に含める」ことを求めており、drain 相だけが先送り語彙（`topmost`／`stayontop`／`windowstate` ほか 9 語）の検査から外れる。檻は全緑のまま穴が残る形で、`test-cage-determinism` が踏んだ「対象選定がファイル単位で 1 件取りこぼす」と同型。
**Suggestion**: `PRODUCTION_FILES` を 2→5（`src/emo2_boot/frame/zorder_drain.rs` を追加）とし、件数定数と Traceability 10.4 の「5 本」を 6 本へ訂正する。wintf 側 5→8 は正しい。
**Traceability**: 10.4, 11.4
**Evidence**: design.md「File Structure Plan」新設ファイル一覧／「Modified Files」／Traceability 10.4 行

## Design Strengths

1. **既存語彙の保存を構造で担保している**: 既存ペア 5 ファイルを 1 行も編集せず、`[zorder-pair]` 6 タグ・`ZORDER` 起床旗・`SCHEDULE_NAMES` を不変に保つ設計は、要件 9.5 と干渉台帳（zsp⇄pwc／zsp⇄bod）の義務を「守る努力」ではなく「触らない形」で満たしている。`enqueue_window_set_pos` 非接触も明示されており、並走 spec との衝突面が最小。
2. **未知が実測で潰されたうえで設計に反映されている**: R1（`DeferWindowPos` 一括投入の順序保存）は既存の実窓テスト `command_batch_tests.rs:633`（並べ替えに敏感な 2 連鎖で両経路一致）で緑であることを確認済みで、それに立って「1 巡 1 グループ・グループ内は自己参照連鎖で一括・先頭窓は動かさない」という収束論証が組まれている。判断はすべて純関数（`parse_zorder_tokens`／`decide_group_fix`）に切り出され、要件 10.2 の 9 分岐が実機不要で檻に入る。

## Final Assessment

**Decision: GO**

**Rationale**: 層の分界・依存方向・記録規律・既定＝非強制の保存に構造的な不整合はなく、要件 1〜13 の全受入基準に実現要素が対応し、裁定 3 件も本文へ正しく反映されている。指摘 3 件はいずれも**設計文の局所訂正で閉じる**（診断行の発行時点／頭打ちの粒度／檻の対象 1 本）であり、アーキテクチャの選択（案 C）を揺るがさない。

**Next Steps**:
1. 設計ディスカッションで指摘 1〜3 を反映（1 は診断の発行時点、2 は `fail_streak` の粒度、3 は `PRODUCTION_FILES` 2→5・総数 6）。
2. その後 `/kiro-spec-tasks areka-P0-scope-zorder-pinning` でタスク生成。
3. タスク化時の注意（合否外の申し送り）:
   - 明示モードで同一スコープの片方の窓だけを並べたグループ（例 `\![set,zorder,b1,b0]`）は連鎖が owner 一組の間への挿入を要求し得る。検証が**相対順のみ**であることが救いになっている設計なので、その理由を実装の doc に残すと後続が誤って「直後隣接」を検証条件に格上げしない。
   - `consumer_ledger` は本番に読み手が 0 件（`consumer_of`／`try_register` の呼出はファイル内のみ）。要件 11.3 の一意性は台帳＋テストによる記録上の保証であり、実行時の分配点ではないことを設計・タスクの記述で取り違えない。
   - 設計の `maintain.rs:368-372`（起床旗の作法）は実測では `:363-372` が doc、`mark` 呼出は `:374`。実害はないが実装時に参照するなら再確認のこと。
