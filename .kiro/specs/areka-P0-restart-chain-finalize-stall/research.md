# ギャップ分析: areka-P0-restart-chain-finalize-stall

- 実施日: 2026-10-03（ワークツリー `claude/areka-p0-restart-chain-stall-b7f2ab`・HEAD `f5503095`）
- 入力: `requirements.md`（確定）・`brief.md`・steering（`product.md`・`tech.md`・`structure.md`・`logging.md`）
- コードの引用は「何の定義か」（関数名・型名）で行う。行番号は使わない。

## 1. 要約

- 直す場所は 1 か所に絞れる。初期配置の確定の本体 `finalize_chain_once_with`（`crates/areka/src/emo2_boot/frame/drain_resnap.rs`）が、走査 `collect_chain_states` の返す見送りの理由を `defer_chain_finalize` へ渡して数えている。理由が `ChainDeferReason::NoGhostWindows` のときに数えずに戻れば、要件 1〜3 は満たせる。決定論テストは既存の兄弟テストの道具（`resnap_world`・`spawn_resnap_windows`・`PerTargetSizes`・`capture_logs`・本番と同じ `close_windows_for_restart`）だけで組め、OS の窓も GPU も要らない。
- brief の静的な追跡のうち 1〜4（窓が無いと `NoGhostWindows` になる・`init_resource` が外した数の記録を作り直す・新しい窓が出ても誰も数を戻さない）はコードで裏が取れた。
- **ただし、「窓の無い巡が数に入って 600 に届く」という見立て（brief の 5）は、コードと過去の実機の記録から見ると当たっていない可能性が高い。** 起こし直し（`ghost_switch::switch_to`）は 1 回の呼び出しの中で同期的に「降ろす → 窓を閉じる → 展開する → 起こす」まで済ませるので、その間は毎フレームの処理が 1 巡も回らない。窓の無い巡は、起こし直しが終わってから新しい窓が生えるまでの 1〜数巡だけと見込まれる。一方、WARN は起こし直しの約 5 秒後に出ている。この機械では 1 秒に約 120 巡回っている（過去の実機の記録から逆算）ので、600 巡はほぼすべて **新しい窓が在って相方（scope 1）の初回表示を待っている間** に積まれたと読める。
- したがって要件どおりに直すと、決定論テスト（要件 3）は通るが、実機の確かめ（要件 4.1 の「WARN 0 件」）は満たせない見込みが高い。要件 4.4 は「そのとき止めて原因を取り直す」と定めているので手順としては破綻しないが、要件の議論でこの見込みを先に扱うのがよい（下の議題 1）。
- 規模は XS（変更 10 行前後＋テスト 2〜3 本）・リスクは「直し自体は低い／実機で目的を果たせない見込みは高い」。

## 2. 現状の調査

### 2.1 関係するファイルと役割

| ファイル | 中身 | この spec での扱い |
|---|---|---|
| `crates/areka/src/emo2_boot/frame/drain_resnap.rs` | `finalize_chain_once`（本番の入口・本体へ委ねるだけ）／`finalize_chain_once_with`（確定の本体）／`collect_chain_states`（走査・見送りの理由を返す）／`defer_chain_finalize`（見送りを数え、600 巡目で WARN を 1 回） | 触る（唯一の本番ソース） |
| `crates/areka/src/placement/chain_finalize.rs` | `ChainFinalized`（確定の印）／`ChainFinalizeStall`（見送りの数と「報告済み」）／`note_chain_deferral`（数えて、しきい値ちょうどで `true`）／`CHAIN_FINALIZE_STALL_FRAMES = 600`／`ChainDeferReason`（理由 9 種と本文） | 触らない（境界の外） |
| `crates/areka/src/placement/chain_realign.rs` | 拡大率の遷移の後の解き直し。`arm_chain_realign` が武装のたびに `ChainFinalizeStall::reset`、`defer_chain_realign` が同じ資源で数える | 触らない（要件 2.4） |
| `crates/areka/src/app_exit.rs` | `close_windows_for_restart` が窓を消し、`GhostWindows`・`ChainFinalized`・`ChainFinalizeStall`・`ChainRealignPending` ほかを外す | 触らない（前提） |
| `crates/areka/src/emo2_boot/frame.rs` | `emo2_frame_system`：毎フレーム、終了の指示が無く `Emo2Wiring` が在れば、各相の後に `finalize_chain_once` → `realign_chain_once` を呼ぶ | 触らない |
| `crates/areka/src/emo2_boot/ghost_switch.rs` | `switch_to`／`switch_to_default`／`boot_into`：起こし直しの手順 | 触らない（読むだけ） |
| `crates/areka/src/emo2_boot/frame_chain_finalize_restart_tests.rs` | 既存の兄弟テスト 2 本（下記） | テストを足す |
| `crates/areka/src/emo2_boot/frame_chain_finalize_tests.rs` | 確定と停滞の WARN の既存テスト（`stalled_finalize_reports_the_reason_exactly_once`・`finalize_within_the_bounded_wait_emits_no_diagnostic` ほか） | 触らない（後退が無いことの確かめに使う） |
| `crates/areka/src/emo2_boot/frame_test_support.rs` | `resnap_world`・`spawn_resnap_windows`・`settled_sizes`・`PerTargetSizes`・`SPAWN_SIZE_0/1`・`capture_logs`・`pos_of` | 触らない（そのまま使える） |

### 2.2 brief の静的な追跡の検証（1 段ずつ）

1. **起こし直しが資源を外す** — 確認済み。`close_windows_for_restart` は `despawn_app_windows` の後に `GhostWindows`・`ChainFinalized`・`ChainFinalizeStall`・`ChainRealignPending`・`ZOrderChainPlan`・`ZOrderAbsentReports` を `remove_resource` する。`Emo2Wiring` は外さない。
2. **窓の無い区間も毎フレームの処理が回り、確定へ入る** — 半分だけ正しい。
   - 正しい点: `emo2_frame_system` は `GhostWindows` の有無を見ずに `finalize_chain_once` を呼ぶ。`finalize_chain_once_with` の最初の判定は `ChainFinalized` の有無だけで、外された直後は無いので走査へ進む。
   - 訂正が要る点: 「古い結線のまま回る」は切替の経路では起きない。`switch_to` は `run_ghost_quit_phase`（`ghost_quit_system`・`emo2_frame_system` より前に走る排他の系）の中から同期的に呼ばれ、`take_down` → `close_windows_for_restart` → `install::desk::run_overwrite_between` → `boot_into`（中で `boot_ghost_strict` → 起動の結線が新しい `Emo2Wiring` を `insert_non_send`）→ `commit_ghost_windows` までを 1 回の呼び出しで終える。次に `emo2_frame_system` が回るときには、結線はすでに新しいもの（未装着）に替わっている。窓は `commit_ghost_windows` が作業プールへ渡した閉包が次の `Input` 段で World に適用されて初めて生えるので、**窓の無い巡は「起こし直しを終えたその巡」から「閉包が着く巡」までの 1〜数巡**である（巡数は推定・未測定）。
3. **`NoGhostWindows` が数える道へ届き、`init_resource` が資源を作り直す** — 確認済み。`collect_chain_states` は冒頭で `GhostWindows` が無ければ `Err(ChainDeferReason::NoGhostWindows)` を返し、`finalize_chain_once_with` はどの理由でも区別なく `defer_chain_finalize` を呼ぶ。`defer_chain_finalize` の先頭は `world.init_resource::<ChainFinalizeStall>()` なので、外したばかりの資源が既定値（0・未報告）で作り直され、`note_chain_deferral` が 1 を足す。
4. **誰も数を戻さない** — 確認済み。`ChainFinalizeStall` に触る本番コードは `app_exit.rs`（外す）・`drain_resnap.rs`（作り直して数える）・`chain_realign.rs`（武装時の `reset` と数える）の 3 つだけ。`arm_chain_realign` は `ChainFinalized` が無ければ何もしないので、確定前の新しい一式で数が戻ることは無い。
5. **窓の無い巡が数に入って 600 に届く** — 当たっていない可能性が高い（次節）。

### 2.3 「600 に届く巡」はどこで積まれたか（過去の記録からの読み）

- 毎フレームの処理は画面の書き換えに合わせて回る（`wintf` の `runtime::tick_bridge` の `tick_one_frame`・`tick_gate` の説明に「120Hz なら」とある）。`CHAIN_FINALIZE_STALL_FRAMES` の説明は「60Hz で約 10 秒」を前提にしているが、120Hz の画面では約 5 秒になる。
- 完了 `areka-P0-ghost-shell-balloon-switch` の `signoff.md`（走行 `run11c`）には、切替先の窓が在るまま初回表示が来ない形で `ghost_switch_booted` 04:28:08.942 → WARN `deferrals=600` 04:28:13.991 という記録がある。窓が在る区間だけで **600 巡 ≒ 5.05 秒（約 119 巡/秒）** だったことになる。
- 完了 `areka-P0-ghost-install` の `signoff.md`（項目 3）の時刻: `windows_closed_for_restart` 00:56:08.42 → `install_overwrite_done` 08.70 → `ghost_switch_booted` 09.00 → `ghost_switch_done` 10.83 → `OnInstallComplete` 10.89 → WARN 00:56:14（秒まで）。閉じてから起こし終えるまでの 0.58 秒は 2.2 の 2 のとおり 1 回の同期呼び出しの中なので巡は回らない。WARN は起こし終えてから約 5 秒後で、上の「600 巡 ≒ 5 秒」とそのまま合う。
- 以上から、600 巡のほぼすべては **新しい窓が在り、scope 1 のシェルの初回表示（最初の `\s` の到着）を待っていた巡**（理由 `NotShownYet { scope: 1 }`）と読める。WARN の本文の理由も `scope 1: 実表示寸が未確定（初回表示が未成立）` で、窓が在るときにしか出ない理由である（窓が無ければ理由は `NoGhostWindows` になる）。
- scope 1 の表示が遅れる筋の候補（未確認）: シェルは台本の最初の `\s` まで表示しない作り（`frame.rs` の説明の「defect #5」）なので、起こし直しの直後の台本が相方（`\1`）へ届くまでに 5 秒以上かかると、窓が在るまま 600 巡に届く。emo2 の辞書（`vendors/sample_ghost/emo2.nar` の `dic/boot.pasta`・`dic/install.pasta`）では、同じゴーストへの切替の `OnGhostChanged` は 204 を返し、起動・インストール完了の台詞はどれも 1 行目がむらさき（`\0`）で始まる形が多い。起動の台詞の途中で `OnInstallComplete` の台詞に置き換わると、相方が出るのはさらに 1 行あと、という順になり得る。初回の起動・里々・メニューからの普通の切替で出ないこととも矛盾しない。**これは静的な読みで、実機では確かめていない。**

### 2.4 窓が無い以外の理由が「窓の無い区間」に紛れ込むか

`collect_chain_states` が返し得る理由ごとに、起こし直しの間に起こり得るかを洗った。

| 理由 | 起きる場面 | 窓が無い区間で起こるか |
|---|---|---|
| `NoGhostWindows` | `GhostWindows` が World に無い（起動で窓が生える前・`close_windows_for_restart` の後） | **起こる（この spec の対象）** |
| `NoScopes` | `GhostWindows` は在るがスコープが 0 | `spawn_ghost_windows` は配置の数だけ窓を作るので、配置が 0 件のときだけ起こり得る（配置の準備が 0 件を弾くかは未確認）。起こし直しの区間に固有ではない。`collect_chain_states` の説明の文は「窓がまだ生えていない」と書かれており、意味としては「窓が無い」側に近い（議題 4） |
| `NoCharWindow` | 台帳のスコープにキャラ窓が無い | `spawn_ghost_windows` はキャラ窓とバルーン窓を対で作るので通常は起きない |
| `NotShownYet` | 表示側が未装着、または初回 `ShowSurface` が来ていない | 窓が在る区間で起こる（新しい一式の装着前・最初の `\s` 前）。**600 に届いた本体はこれと見られる** |
| `UnusableShownSize` | 実表示寸が 0 か大きすぎる | 窓が在る区間のみ |
| `NoWindowPos`・`IncompleteWindowPos` | 窓の entity に `WindowPos` が無い・未確定 | 窓が在る区間。終了の後に窓だけ消えて台帳が残る形でも起こるが、終了が指示された巡は `emo2_frame_system` が入口で戻るので確定へ届かない |
| `ResnapNotLanded` | 窓寸が実表示寸に追いついていない | 窓が在る区間のみ |
| `DpiSyncHeld` | 解き直し専用（`chain_realign`） | 起動時の確定では起きない |

結論: 「窓の一式が無い」に当たるのは本番では `NoGhostWindows` だけで、要件 1 の範囲はこの 1 つで足りる。窓が在るのに数えるべきでない巡（新しい一式の装着前・最初の `\s` 前）は `NotShownYet` に含まれており、要件 1 の直しでは数え続ける（要件 2 で「本物の停滞」と同じ扱い）。

### 2.5 決定論テストの道具

- `finalize_chain_once_with` は寸の引き口（`PhysicalSizeSource`）を外から渡せるので、偽の寸（`PerTargetSizes`・`settled_sizes`）で OS の窓も GPU も無しに何巡でも回せる。既存テストがすでに 600 巡・1200 巡を回している（`stalled_finalize_reports_the_reason_exactly_once`）。
- 窓の一式は `resnap_world`（本物の `spawn_ghost_windows`・2 スコープ・偽の窓ハンドル）で生やし、`close_windows_for_restart` で閉じ、`spawn_resnap_windows` で同じ World に次の一式を生やせる（既存の `next_window_set_finalizes_once_like_first_boot` と同じ形）。
- 記録の捕捉は `capture_logs`（呼んだスレッドだけを捕まえる）。WARN の件数は `level=WARN` の行で数える既存の書き方がある。
- 要件 3.2（直す前は赤）: 今のコードでは「閉じた後に窓なしで 600 巡」は 600 巡目に WARN が出て `ChainFinalizeStall { deferrals: 600, reported: true }` が残る＝赤になる。要件 3.4（窓なしの巡を挟んだ後の新しい一式の 600 巡目）も、今のコードでは窓なしの巡数ぶん早く WARN が出るので赤になる（窓なしの巡を 1 以上挟めば判定できる）。
- 要件 3.3（窓が在る対照）は既存の `stalled_finalize_reports_the_reason_exactly_once` とほぼ同じ形。兄弟テストに置くかどうかは設計で決める。

### 2.6 判定の分岐の記録（今日の姿）

- 見送りの各巡は無音（記録なし）。600 巡目に `warn!`（本文「chain_finalize: 初期配置の確定が続けて見送られている…」・項目 `deferrals`・`scope`・`reason`）が 1 回だけ。
- 確定したときは、動かしたスコープごとに `info!`（「実表示寸で連鎖を再解決」・`scope`・`from_x`・`to_x`）、最後に `debug!`（「初期配置を確定」・`scopes`・`moved`）。
- 見送りの理由を巡ごとに残す記録は、確定の側には無い（解き直しの側には既定で切ってある観測の記録 `transition_diag::log_chain` がある）。
- 記録の target は `areka::emo2_boot::frame::drain_resnap`。roadmap の覚え書きは再測定に `RUST_LOG=…,areka::emo2_boot::frame=debug` を挙げている。
- 表示側は初回表示の成立を `info!`（「apply(ShowSurface): 表示・マスクを更新」・target つき）で残しているので、既定に近い level でも「scope 1 のシェル（target 2）がいつ初めて表示されたか」は読める。2.3 の見立ての確かめに使える。
- steering `logging.md` は「フレームごとの処理」を `trace!` に割り当てている。

## 3. 要件と既存資産の対応

| 要件 | 使える既存資産 | 足りないもの | 区分 |
|---|---|---|---|
| 1.1 窓が無い巡を数えない | `collect_chain_states` が `NoGhostWindows` を区別して返す | `finalize_chain_once_with`（または `defer_chain_finalize`）での分岐 1 つ | 不足（小） |
| 1.2 窓が無い間は WARN を出さない | 1.1 で数が作られないので自動的に満たす | なし | — |
| 1.3 新しい一式は 0 から | `close_windows_for_restart` が資源を外す＋1.1 | なし | — |
| 1.4 新しい一式で確定を 1 回 | 既存の `next_window_set_finalizes_once_like_first_boot` が確かめ済み | 窓なしの巡を挟んだ形でも 1 回であることの確かめ | 不足（テスト） |
| 1.5 数えなかったことを開けた level でだけ残す | `tracing` の `debug!`／`trace!` | 記録の位置・level・頻度（毎巡か区間の始まりだけか） | 議題 3 |
| 2.1〜2.3 本物の停滞は今日と同じ | `note_chain_deferral`・`defer_chain_finalize` の WARN | なし（触らない） | 制約 |
| 2.4 規則・しきい値・解き直しの数え方を変えない | 境界の外のファイルに触らないことで守れる | なし | 制約 |
| 3.1〜3.5 決定論テスト | 2.5 の道具一式 | テスト 2〜3 本 | 不足（テスト） |
| 4.1 実機で WARN 0 件 | — | **要件 1 の直しだけで届くかが不確か**（2.3） | 要調査（議題 1） |
| 4.2 判定の分岐が見える level で実機 | 2.6 の記録 | 1.5 の記録の level に合わせた `RUST_LOG` | 1.5 に従う |
| 4.3 実機の根は `target\` の下 | 既存の実機の手順 | なし | 制約 |
| 4.4 WARN が出たら止める | — | なし（手順） | 制約 |

## 4. 実装の選択肢

### 案 A: 確定の本体で「窓が無い」を数えずに戻る（brief の案）

- 変える所: `finalize_chain_once_with` の `Err(reason)` の腕で、`reason` が `ChainDeferReason::NoGhostWindows` なら記録（要件 1.5）だけして戻り、それ以外は今日どおり `defer_chain_finalize` へ渡す。
- 良い点: 1 ファイル・数行。判断が「確定の本体」に見える形で 1 か所に残る。`defer_chain_finalize` は「数える」だけの役目のまま。
- 気になる点: 要件 4.1 を満たす保証にはならない（2.3）。

### 案 A': `defer_chain_finalize` の先頭で「窓が無い」なら数えずに戻る

- 案 A と効き目は同じ。違いは判断の置き場所で、`defer_chain_finalize` の名前（見送りを記録する）と「記録しない」判断が同居する。
- 良い点: 呼び手の形を変えない。
- 気になる点: 「数えない」が数える関数の内側に隠れ、読み手が確定の本体だけを見て気付きにくい。

### 案 A'': 走査の前に `GhostWindows` の有無を見て戻る

- `finalize_chain_once_with` の冒頭（`ChainFinalized` の判定の直後）で `GhostWindows` が無ければ戻る。
- 良い点: 走査に入らない分、意図が字面に出る（「窓が無ければ確定の対象が無い」）。
- 気になる点: `collect_chain_states` の `NoGhostWindows` の腕が起動時の確定からは届かなくなる（解き直しの側からは引き続き届く）。判定が 2 か所に分かれる。

### 案 B: 新しい窓の一式が出たときに数を戻す

- 例: `ChainFinalizeStall` に窓を閉じた回数（`WindowsEpoch`）を持たせ、変わっていたら 0 に戻す。
- 気になる点: `chain_finalize.rs`（境界の外）に触る。窓が在る間に積まれる巡（2.3 の本体）には効かない。brief が採らない案に挙げた「起こし直しの終わりに数を戻す」とも近い。
- この spec の境界では採りにくい。

### 案 C: 窓が在る間の正常な待ちも数えない（2.3 の見立てに効く案・参考）

- 例: 新しい一式の表示側が装着されて最初の `\s` が来るまで（`NotShownYet` のうち「まだ一度も表示していない」間）は数えない／時間で測るしきい値に替える／起こし直しの直後は待ちを長く見る。
- 気になる点: どれも要件 2（今日と同じしきい値・同じ WARN）と境界（`chain_finalize.rs` に触らない）にぶつかる。また「最初の `\s` が永久に来ない」は本物の異常（例: 完了 `shell-implicit-surface` の brief の症状）でもあるので、数えないと本物を見逃す。
- 要件の議論で、実機の測り直しの結果を見てから別の spec として扱うかを決める材料として挙げる。

### 比較

| 案 | 触るファイル | 要件 1〜3 | 要件 4.1 | 境界 |
|---|---|---|---|---|
| A | `drain_resnap.rs` | 満たす | 不確か | 内 |
| A' | `drain_resnap.rs` | 満たす | 不確か | 内 |
| A'' | `drain_resnap.rs` | 満たす | 不確か | 内 |
| B | `chain_finalize.rs` ほか | 一部 | 効かない見込み | 外 |
| C | `chain_finalize.rs` ほか | 要件 2 と衝突 | 効く見込み | 外 |

## 5. 規模とリスク

- 規模: **XS**（本番 10 行前後＋テスト 2〜3 本・既存の道具で足りる）。
- リスク:
  - 直しそのもの: **低**。既存の分岐に 1 つ腕を足すだけで、窓が在る巡の扱い（要件 2）は 1 行も変わらない。既存テスト `frame_chain_finalize_tests.rs` の停滞の 2 本がそのまま後退の確かめになる。
  - 目的（実機で WARN 0 件）を果たせない見込み: **高**。2.3 のとおり、600 巡の本体は窓が在る区間で積まれていると読める。

## 6. 設計への申し送り

### 6.1 議題（要件の議論で扱う）

1. **見立ての食い違いをどう扱うか。** 起こし直しは 1 回の同期呼び出しで済み、窓の無い巡は 1〜数巡しか無い。WARN は窓が在る区間の約 5 秒（約 120 巡/秒で 600 巡）で出たと読める。このまま要件どおり直すと、決定論テストは通るが実機の 4.1 は満たせない見込みが高い。選択肢: (a) このまま進め、実機で WARN が出たら要件 4.4 に従って止める／(b) 設計の前に実機で 1 回測り直す（`RUST_LOG` で `areka::emo2_boot::frame=debug` を開け、`ghost_switch_booted` と、表示側の `apply(ShowSurface)` の target 2 の最初の行と、WARN の時刻を突き合わせる）／(c) 窓が在る間の正常な待ちまで扱うよう範囲を広げる（境界と要件 2 の見直しが要る・別の spec に分ける手もある）。
2. **しきい値の前提（60Hz で約 10 秒）が 120Hz の画面では約 5 秒になる。** 要件 2.4 でしきい値は変えないと決まっているので、この spec では触らない。ただし起こし直しの後の正常な待ちが 5 秒を超えるゴーストでは今後も WARN が出る。別の spec として登記するかを決める。
3. **要件 1.5 の記録の形。** 窓の無い巡は毎フレーム起こるので、毎巡記録するなら steering `logging.md` の割り当てでは `trace!`。区間の始まりだけ `debug!` で 1 回残すには「前の巡も窓が無かったか」を覚える状態が要り、外された資源を作り直さないという要件 1.1 と両立する置き場所（資源でない場所）を考える必要がある。実機の確かめ（要件 4.2）の `RUST_LOG` もこの決定に合わせる。
4. **`NoScopes`（台帳は在るがスコープ 0）を「窓が無い」に含めるか。** 本番ではほぼ起きないが、説明の文は「窓がまだ生えていない」と書いている。要件の文言（窓の一式が無い）に含めるかどうかで分岐の書き方が変わる。
5. **判断の置き場所（案 A／A'／A''）。** どれも 1 ファイルで効き目は同じ。読み手に「窓が無ければ確定の対象が無い」が伝わる場所を選ぶ。
6. **対照のテスト（要件 3.3）を兄弟テストに新しく書くか、既存の `stalled_finalize_reports_the_reason_exactly_once` を対照として指すか。** 境界は兄弟テスト 1 ファイルなので、新しく書くなら `frame_chain_finalize_restart_tests.rs` に置く。

### 6.1.1 議題 1 の裁定と実機の測り直し（2026-10-03）

議題 1 は開発者の裁定で (b)「設計の前に実機で 1 回測り直す」。

- 実行体: `tools/package-alpha.ps1` で HEAD `7393c30` から組んだ `areka-alpha-x64-20261003-7393c30.zip`（全段 緑）。根はワークツリーの `target\rt\<走行名>\`、検体は `target\rt\nar\emo2.nar`（`vendors/sample_ghost/emo2.nar` の写し）
- 環境: 画面 120Hz（Intel Arc・2880 幅・DPI 192）・`RUST_LOG=info,areka=debug,kanade=trace`・`AREKA_NO_ALERT=1`・`AREKA_APP_SMOKE_EXIT_MS` で有界の自動終了（どの走行も終了コード 0）
- 入れ直しのきっかけ: 画面を操作せず、根の emo2 の `dic/boot.pasta` の台詞に `\![execute,install,path,<検体>]` を書き足した（入れ直しで元の emo2 に戻るので、起こし直しの後の台本は素の emo2）

| 走行 | 形 | 起こした → 窓が生えた | 窓 → 相方（target 2）の初回表示 | 起こし直しの後に話したもの | `deferrals=600` |
|---|---|---|---|---|---|
| r1〜r4 | 初回の起動（`OnFirstBoot`）の 1 行目に差し込み | 0.03〜0.04 秒 | 0.5〜0.9 秒 | `OnFirstBoot`（1 行目がエモ `\1`）→ `OnInstallComplete` | 0 件（4 回とも） |
| **r5** | 1 回目の起動で初回を済ませ、2 回目の `OnBoot`（起動朝/昼/夜/深夜の 12 場面）に差し込み＝**観測と同じ形** | **0.15 秒** | **5.76 秒** | `OnGhostChanged` → `OnBoot` → `OnInstallComplete`（むらさき始まり） | 0 件 |

読み取れたこと:

1. **窓の無い巡はごくわずか**（起こしてから窓が生えるまで 0.03〜0.15 秒）。brief の見立て「窓の無い巡が積もって 600 に届く」は**外れ**。2.2 の 2 の静的な読み（起こし直しは同期の 1 回の呼び出しで済む）と合う。
2. **600 巡に近づく本体は、窓が在って相方の初回表示を待つ巡**。観測と同じ形（r5）では、相方の最初の `\s` が窓から 5.76 秒後だった。理由は台本の順（むらさきの台詞が先で、エモはその後の行で初めて話す）で、ゴーストとしては正常な振る舞い。
3. 600 巡の見張りは「60Hz で約 10 秒」を前提に決めた数だが、120Hz の画面では満速なら約 5 秒（`ghost-shell-balloon-switch` の `run11c` の実測 600 巡 ≒ 5.05 秒）。r5 の 5.76 秒の待ちで鳴らなかったのは、この区間の巡が満速より遅かったためと読める（巡の数の記録は無い）。**観測の 2 件と今回の 0 件の差は、しきい値のきわでの速さの揺れ**と見るのが筋。
4. よって、要件 1（窓が無い巡を数えない）だけでは観測の WARN は消えない。根は「巡の数で測るしきい値が画面の書き換えの速さに引きずられ、120Hz では設計の半分の時間で鳴る」こと。

### 6.1.2 議題 2 の裁定: 見送りを「ゴースト待ち」と「areka 自身の待ち」に分ける（2026-10-03）

- 開発者の問い「ゴーストによって秒数が変わるのでは」を受けて検討した。シェルは最初の `\s` まで表示しない作り（`attach.rs` の説明・SSP 互換の既定は表示なし）なので、相方の初回表示の時刻はゴーストの台本しだいで決まる。巡でも秒でも、決まった数で区切る限りどこかのゴーストで空鳴りする。しきい値を時間に直す案は、きわを動かすだけなので採らない。
- 裁定: **ゴースト待ち**（`NoGhostWindows`・`NoScopes`・`NotShownYet`）は数えない。**areka 自身の待ち**（表示された後の理由）は今日どおり数え、同じしきい値で WARN を出す。しきい値・本文・`chain_finalize.rs` は変えない。触るのは `drain_resnap.rs` の判定 1 か所とテストだけ。
- 要件 1〜4 をこの形に書き直した（requirements.md）。brief の見立ての誤りは requirements.md の Project Description に記した。

### 6.1.3 設計へ送る判断（カテゴリ B）

1. **数えなかったことの記録の形（要件 1.6・4.2）**: 毎巡の `trace!`（steering `logging.md` の割り当て）か、区間の始まりだけ `debug!` か。後者は「前の巡もゴースト待ちだったか」を覚える置き場所が要り、外した資源を作り直さない要件 1.1 と両立させる必要がある。
2. **`NotShownYet` の中身の仕分け**: 実表示寸が引けない理由には「ゴーストがまだ `\s` を出していない」のほかに「表示側がまだ装着されていない」も含まれる（`NotShownYet` の説明）。後者は areka 自身の待ちだが、普段は数巡で終わり、装着の失敗は装着の側が `error!` を残す。同じ扱い（数えない）でよいかを確かめる。
3. **表示の経路の失敗の記録**: 「`\s` は出たのに表示が着地しない」はこの WARN では拾えなくなる。表示の経路（seriko → PresentBridge → drain → `apply(ShowSurface)`）がその場で失敗を記録しているかを確かめる。記録の無い失敗の経路があれば、それは別の spec として登記する（本 spec の境界の外）。
4. **`NoScopes` を「窓の一式が無い」に含めるか**（6.1 の議題 4）: ゴースト待ちの側に入れる案で要件を書いた。配置が 0 件のときだけ起こり得る理由なので、設計で扱いを確かめる。
5. **判断の置き場所**（6.1 の議題 5・案 A／A'／A''）。
6. **既存テストの更新（要件 3.8）**: `frame_chain_finalize_tests.rs` の `stalled_finalize_reports_the_reason_exactly_once` は相方のシェルを表示しない形（`PerTargetSizes` の `(1, None)`）で停滞の WARN を確かめているので、新しい振る舞いでは赤になる。areka 自身の理由（例: 窓の大きさの追従が着地しない）で停滞させる形へ置き換える。`finalize_within_the_bounded_wait_emits_no_diagnostic` ほか、同じ形を使うテストも洗う。
7. **対照のテスト（要件 3.4・3.5）の置き場所**（6.1 の議題 6）。

### 6.2 要調査（設計で確かめる）

- 起こし直しを終えた巡から新しい窓の閉包が着く巡までの実際の巡数（作業プール → `Input` 段の着き方）。決定論テストで「窓なしの巡」を何巡回すかの根拠になる（テストは 600 巡回すので結論は変わらない）。
- emo2 の入れ直しの後、scope 1 のシェルが初めて表示されるまでの時間と、その間の台詞の順（`OnGhostChanged` の 204 の後の扱い・`OnInstallComplete` の置き換え）。2.3 の見立ての確かめ。
- 過去の実機の記録（`target\` の下の `run.log`）が残っていれば、`apply(ShowSurface)` の時刻から上の時間を測れる。

## 7. 設計フェーズの記録（2026-10-03・`design.md` 生成時）

### 7.1 調査の範囲と要点

- 区分: **既存の仕組みの拡張（軽い調査）**。新しい依存・外部 API・新しい型は無く、変える本番ソースは 1 ファイルの `match` の腕 1 つ。サブエージェントは使わず、関係するコードを直接読んだ。
- 読んだもの: `drain_resnap.rs`（`finalize_chain_once_with`・`collect_chain_states`・`defer_chain_finalize`）・`placement/chain_finalize.rs`（`ChainDeferReason` 9 種・`note_chain_deferral`・`CHAIN_FINALIZE_STALL_FRAMES`）・`placement/chain_realign.rs`（`realign_chain_once_with` が同じ走査の閉包を受け、自分の `defer_chain_realign` で数える）・`frame.rs`（`emo2_frame_system` の呼び順）・`frame/attach.rs`（シェルは最初の `\s` まで `ShowSurface` を出さない・装着の失敗は `warn!`／`error!`）・`emo2_boot/adapter.rs`（`PresentBridge` の失敗の記録）・`areka-emo-present/src/presenter.rs` の冒頭（全失敗分岐が `error!`／`warn!`）・兄弟テスト 2 ファイルと `frame_test_support.rs`（`PerTargetSizes`・`settled_sizes`・`capture_logs`・`count_level`・`spawn_resnap_windows`）・`log-capture-kit`（TRACE を含む全 level を捕捉）。
- 要点:
  1. 「数えるか」の判断は `finalize_chain_once_with` の `Err(reason)` の腕に無く、理由を区別せず `defer_chain_finalize` へ渡している。ここに仕分けを足せば、`init_resource` を含む数える側は 1 行も変えずに済む。
  2. 遷移後の解き直し（`chain_realign.rs`）は同じ走査を使うが数える関数は別なので、確定側の腕だけを変えれば要件 2.4 は自動的に守られる。
  3. 「areka 自身の待ち」を決定論で作る fake は既存の `finalize_defers_until_resnap_has_landed` が使う `PerTargetSizes::new([(0, Some((500, 687))), (1, Some(SPAWN_SIZE_1))])`（`ResnapNotLanded { scope: 0 }`）で足りる。確定の処理は再スナップを呼ばないので、この食い違いは何巡でも解けない。

### 7.2 設計の判断（§6.1.3 の 7 項目の決着）

| 項目 | 決めたこと | 理由 |
|---|---|---|
| 1 記録の形 | 毎巡 `trace!` 1 行（target `areka::emo2_boot::frame::drain_resnap`・項目 `scope`・`reason` は WARN と同じ） | 区間の始まりだけ `debug!` は状態が要り、1.1（資源を作り直さない）と置き場所がぶつかる。steering の割り当て（フレームごとの処理は `trace`）と合う。`drain_resnap.rs` に今日 `trace!` は無いので、この target を開けても増えるのは本 spec の行だけ |
| 2 `NotShownYet` の中身 | 「`\s` 未到着」も「表示側が未装着」も同じく数えない | 走査からは見分けられず、見分けるには表示側の装着状態を問う口が要る（境界の外）。装着は数巡で終わり、失敗は `attach.rs` が自分で `warn!`／`error!` を出す |
| 3 表示の経路の失敗の記録 | 確かめた・別 spec の登記は不要 | `PresentBridge`: 写像不能は `warn!`・配送先の drop（終了中）は `debug!`。`EmoPresenter::apply`: 全失敗分岐が `error!`／`warn!`（`presenter_display_failure_tests.rs` が固定）。装着: 項目 2 のとおり |
| 4 `NoScopes` | ゴースト待ち（数えない） | 配置 0 件のときだけ起こり、説明文も「窓がまだ生えていない」。要件 1.1 の「窓の一式が無い」に含める |
| 5 置き場所 | 案 A（`finalize_chain_once_with` の `Err` の腕・列挙子を網羅する `match`） | 「数えない」が確定の本体の字面に見える。`_` を書かないので、理由が増えたら足した側がコンパイルエラーで仕分けを決める |
| 6 既存テストの更新 | `stalled_finalize_reports_the_reason_exactly_once`・`finalize_within_the_bounded_wait_emits_no_diagnostic` を `ResnapNotLanded` の形へ置き換え（本文の判定は `scope 0`・「再アンカーが未 landing」） | `(1, None)` は新しい振る舞いではゴースト待ち＝数えない。`finalize_defers_while_any_scope_has_not_shown_yet` は数えることを前提にしないので据え置く |
| 7 対照の置き場所 | 3.4 は `frame_chain_finalize_tests.rs` の置き換え後の T4、3.5 は `frame_chain_finalize_restart_tests.rs` の T2（閉じる → 窓なし → 新しい一式 → 未表示 → 揃ってから areka 自身の待ち） | 既存の `next_window_set_finalizes_once_like_first_boot` が窓なしを挟まない形を固定済みなので、T2 は停滞の対照に専念させ、確定 1 回（1.5・3.6）は T3 の末尾で見る |

### 7.3 まとめ方の検討（一般化・既製の採用・簡素化）

- 一般化: 要件 1.1〜1.3 はすべて「見送りの理由を 2 種に仕分ける」1 つの判断に畳める。仕分けの表は `ChainDeferReason` の列挙子に対する `match` 1 つで、新しい型も関数も要らない。
- 既製の採用: 記録は `tracing` のまま。見送りの数と一発フラグは既存の `ChainFinalizeStall`／`note_chain_deferral` をそのまま使う。
- 簡素化: 「前の巡もゴースト待ちだったか」を覚える状態、`ChainFinalizeStall` に理由を持たせる案、仕分けを別関数（`is_ghost_wait`）へ出す案は、どれも本 spec の要件に対して増えるだけなので採らない。`match` の腕にそのまま書く。

### 7.4 リスクと備え

- 「`\s` は出たのに表示が着地しない」はこの WARN では拾えなくなる — 7.2 の項目 3 で、表示の経路がその場で記録していることを確かめた。
- しきい値の前提（60Hz で約 10 秒）は 120Hz では約 5 秒のまま — 本 spec の境界の外（§6.1 議題 2）。areka 自身の待ちが 5 秒を超える場面では今日どおり鳴る。
- 直す前の赤（3.3）を見ずに本番を先に直すと、テストが「初めから緑」で判定の力を失う — 実装の順序は T1〜T3 を先に書いて赤を見てから本番を直す（`design.md` Testing Strategy）。
