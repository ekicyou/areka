# 設計レビュー: areka-P0-restart-chain-finalize-stall

- 実施日: 2026-10-03（ワークツリー `claude/areka-p0-restart-chain-stall-b7f2ab`・HEAD `3b5646f3`）
- 入力: `design.md`・`requirements.md`・`research.md`（§6.1.1〜6.1.3・§7）・steering（`logging.md` ほか）
- 設計の主張は文書だけでなく実際のコードを読んで照合した（照合した定義は下の「確かめたこと」に列挙）。

## レビュー要約

変える本番ソースは `finalize_chain_once_with` の `Err(reason)` の腕 1 か所で、見送りの理由を「ゴースト待ち（数えない）」と「areka 自身の待ち（今日どおり数える）」に `match` で仕分ける設計である。仕分けの表・テストの形・直す前の赤・境界（触るのは 3 ファイルだけ）のいずれもコードの実物と一致しており、実装へ進める状態にある。

## 確かめたこと（コードとの照合）

| 設計の主張 | 照合先 | 結果 |
|---|---|---|
| (a) `match` が `ChainDeferReason` の全列挙子を網羅する | `placement/chain_finalize.rs` の `enum ChainDeferReason`（`NoGhostWindows`・`NoScopes`・`NoCharWindow`・`NotShownYet`・`UnusableShownSize`・`NoWindowPos`・`IncompleteWindowPos`・`ResnapNotLanded`・`DpiSyncHeld` の 9 種） | 一致。設計の 3＋6 で 9 種すべてを書いている。`DpiSyncHeld` は `collect_chain_states` からは返らず `chain_realign.rs` の `realign_chain_once_with` だけが作るが、網羅のために腕を置くのは妥当 |
| (b) ゴースト待ちの巡は `ChainFinalizeStall` を作り直さず、areka 自身の待ちは今日どおり作る | `drain_resnap.rs` の `defer_chain_finalize`（先頭の `world.init_resource::<ChainFinalizeStall>()` がこの経路で資源を作る唯一の場所）／`app_exit.rs` の `close_windows_for_restart`（`remove_resource::<ChainFinalizeStall>()`） | 一致。ゴースト待ちの腕で `defer_chain_finalize` を呼ばずに戻れば、閉じた後の資源は作り直されない（要件 1.1・1.4）。areka 自身の待ちは関数の本文に 1 bit も触れない |
| (c) 要件 3.5 のテスト（T2）が既存の偽物で組める | `frame_test_support.rs` の `spawn_resnap_windows`（`resnap_placements` の寸 434x687／278x357 を `WindowPos.size` に転記・`spawn.rs` の `window_pos`）・`PerTargetSizes`・`capture_logs`・`count_level`／既存 `next_window_set_finalizes_once_like_first_boot`（閉じて同じ World に次の一式を生やす形） | 組める。`(1, None)` の巡は scope 0 が全条件を通った後に scope 1 で `NotShownYet { scope: 1 }`＝数えない。`PerTargetSizes::new([(0, Some((500, 687))), (1, Some(SPAWN_SIZE_1))])` は scope 0 で `size.width 434 != 500` に当たり毎巡 `ResnapNotLanded { scope: 0 }` を返す（確定の処理は再スナップを呼ばないので何巡でも解けない）。表示が揃ってから 600 回目でちょうど 1 件になる |
| (d) `trace!` を `capture_logs` で決定論的に捕らえられる（T3 の「TRACE 1200 件」） | `log-capture-kit/src/capture.rs` の `CaptureSubscriber::enabled`（常に真＝TRACE を含む全 level）・`frame_chain_finalize_tests.rs` の `capture_logs`／`lines_of_level`（`level=TRACE` の行で数えられる）・workspace の `Cargo.toml` に `max_level_*` の feature 指定は無い | 捕らえられる。`NotShownYet` の巡では `collect_chain_states` が記録を出さないので、1200 巡の捕捉は本 spec の trace 行だけになる（WARN・INFO・DEBUG 0 件の判定も成り立つ） |
| (e) 境界: 触るのは 3 ファイルだけ | `drain_resnap.rs`・`frame_chain_finalize_tests.rs`・`frame_chain_finalize_restart_tests.rs`（`frame.rs` の `#[path]` で両テストとも既に取り込み済み・`count_level` は `test_support` から `pub(super)` で引ける） | 一致。`chain_finalize.rs`・`chain_realign.rs`・`app_exit.rs`・`frame_test_support.rs` に触る必要は無い |
| 直す前の赤（T1〜T3） | 今日の `finalize_chain_once_with` は理由を区別せず `defer_chain_finalize` へ渡す | T1: 窓なし 600 巡目に WARN・資源 `{600, true}` が残る。T2: 窓なしの 600 巡目で WARN が出て「599 巡目まで 0 件」が破れる。T3: 600 巡目に WARN（既存 `stalled_finalize_reports_the_reason_exactly_once` と同じ形）。いずれも赤になる |
| 置き換える既存テスト（要件 3.8） | `stalled_finalize_reports_the_reason_exactly_once`・`finalize_within_the_bounded_wait_emits_no_diagnostic`（どちらも `(1, None)` で停滞を作る） | 前者は直した後に赤（`NotShownYet` が数えられなくなる）・後者は WARN だけを数えるので緑のままだが、設計どおり `ResnapNotLanded` の形へ置き換えるのが筋。WARN 本文の判定 `scope 0`・「再アンカーが未 landing」は `Display` の実装（`"scope {scope}: 再アンカーが未 landing（実表示寸 …）"`）と一致 |
| 表示の経路が失敗をその場で記録している（§設計判断 3） | `frame/attach.rs`（窓あり資産なし `warn!`・装着失敗 `error!`・`text_slot_view` が `None` のとき `warn!`）・`emo2_boot/adapter.rs`（写像不能 `warn!`・受信端 drop `debug!`）・`areka-emo-present/src/presenter.rs` の冒頭説明（全失敗分岐が `error!`／`warn!`） | 一致。「`\s` は出たのに表示が着地しない」が無音になる経路は見つからなかった |
| 遷移後の解き直しの数え方は変わらない（要件 2.4） | `chain_realign.rs` の `defer_chain_realign`（自分の `init_resource`＋`note_chain_deferral`） | 一致。走査（`collect_chain_states`）は共有するが数える関数は別で、確定側の腕だけを変えても解き直し側は 1 行も変わらない |

## Critical Issues

なし（実装を止める問題は見つからなかった）。

以下は止める理由にはならないが、実装とレビューで意識しておく点である。

- **走査は昇順で最初に躓いた理由を返す**（`collect_chain_states`）。scope 0 が areka 自身の理由（例: 再アンカーが未 landing）で、scope 1 がまだ表示されていない巡は、scope 0 の理由が先に返るので数えられる。設計は System Flows でこの読みを明記し、要件 2.1 の「全スコープが表示済み」と矛盾しないと整理している。本番では再スナップが確定の前に毎巡走るので scope 0 の未 landing は普通 1 巡で解け、実機で空鳴りする筋は薄い。`finalize_chain_once_with` の説明文に、この「先に躓いた理由で決まる」ことを一言残しておくと読み手が迷わない。
- **T3 の「TRACE ちょうど 1200 件」** は、`NotShownYet` の巡に本 spec 以外の記録が無い今日の形に依っている。将来この経路に別の `trace!` が足されたら件数で赤になるが、それは「増えた記録に気付く」正しい赤なので、そのままでよい。

## Design Strengths

1. **変える場所が 1 つで、数える側に触らない。** 「数えるか」の判断を確定の本体の字面に置き、`defer_chain_finalize`・`ChainFinalizeStall`・しきい値・WARN の本文は不変のまま。要件 1.1 の「資源を作り直さない」が、関数を呼ばないという構造で満たされる。
2. **網羅する `match`（`_` を書かない）で、理由が増えたときの仕分けを足した側に強制する。** Revalidation Triggers に書いた前提（`close_windows_for_restart` が資源を外す・表示の経路が自分で記録する）も、それぞれコードの実物で裏が取れている。

## Final Assessment

- **判定: GO**
- 理由: 設計の全主張がコードの実物と一致し、新しい型・資源・依存を増やさず、直す前の赤と直した後の緑がいずれも既存の道具だけで組める。触るファイルは要件の境界どおり 3 つに収まる。
- 次の手: `/kiro-spec-tasks areka-P0-restart-chain-finalize-stall` でタスクを生成する。タスクの並びは設計の Testing Strategy のとおり「T1〜T3 を先に書いて赤を見る → 本番の腕を直す → T4・T5 へ置き換える → 実機で 1 回確かめる」の順にする。
