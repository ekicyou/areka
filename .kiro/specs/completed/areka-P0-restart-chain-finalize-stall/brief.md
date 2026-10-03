# Brief: areka-P0-restart-chain-finalize-stall

> 2026-10-02 棚卸⑳で `.kiro/steering/roadmap.md` の覚え書き「emo2 を上書きで起こし直した後に初期配置の確定の見送りの WARN が 1 件出た」（2026-09-29 登記・09-30 追記）から格上げ。file:line は起票時値（main `03e8d7d6`）。行番号は目安で、正本は「何の定義行か」の方。

## Problem

ゴーストを同じプロセスの中で起こし直すと（起動中のゴーストへ上書きインストールした後・ネットワーク更新の読み直しの後）、起こし直した約 5 秒後に次の WARN が 1 件出ることがある。

```
WARN chain_finalize: 初期配置の確定が続けて見送られている deferrals=600 scope=Some(1) reason=scope 1: 実表示寸が未確定（初回表示が未成立）
```

画面は正常で、利用者から見える害は無い。しかしこの WARN は本来「初期配置がいつまでも決まらない」という本物の異常を知らせるためのもので、正常な起こし直しのたびに鳴ると、本物と見分けられなくなる。

観測 2 件: 完了 `ghost-install` の実機の項目 3（emo2 へ `emo2.nar` を入れ直す）・完了 `network-update` の実機の 2 回目（あやめを更新して読み直す）。里々（R_POST）の上書きと、メニューからの普通の切替では 0 件。

## Current State

静的に追った結果（棚卸⑳）:

1. 起こし直しは `close_windows_for_restart`（`crates/areka/src/app_exit.rs`）で窓と初期配置の資源（`GhostWindows`・`ChainFinalized`・`ChainFinalizeStall` ほか）を外す。毎フレームの結線 `Emo2Wiring` は外さない（次の起動が差し替える）。
2. そのため窓の無い区間（SHIORI を降ろす・書庫を展開する・読み直す）も、毎フレームの処理（`crates/areka/src/emo2_boot/frame.rs` の `emo2_frame_system`）は古い結線のまま回り、初期配置の確定（`finalize_chain_once`）へ入る。
3. 窓が無いので `collect_chain_states` は「窓が無い」（`ChainDeferReason::NoGhostWindows`）を返す。見送りを数える `defer_chain_finalize`（`crates/areka/src/emo2_boot/frame/drain_resnap.rs`）は `init_resource::<ChainFinalizeStall>()` で**外したばかりの資源を作り直し、数え続ける**。
4. 新しい窓が開いても、この数を戻す者が居ない（`ChainFinalizeStall` に触るのは `app_exit.rs`・`drain_resnap.rs`・`chain_realign.rs` だけで、`chain_realign.rs` が戻すのは自分の待ちを仕掛けるときだけ）。
5. 新しい一式で相方（scope 1）の初回表示を待つ間も数は増え、600 に届いて WARN が出る。

**仮説の部分**: 5 の「窓の無い区間の巡が数に入って 600 に届く」は静的な読みで、実機では確かめていない。起動の重いゴースト（emo2＝pasta・あやめ＝YAYA の更新）で出て、里々で出ないことと合う。数えているのは時間でなく巡なので、「600 ＝約 10 秒」の前提もこの区間では成り立たない。

完了 `ghost-shell-balloon-switch` の 11.9 は「切替で起こしたゴーストの見送りの数を引き継がない」を直したが、直したのは資源を外す所までで、外した後に窓の無い区間で数え直される道は残っている。

## Desired Outcome

1. 窓が無い巡は、初期配置の確定の見送りに数えない。
2. 起こし直しの後、新しい窓の見送りは 0 から数え始める。
3. 本物の異常（窓が在るのに初期配置がいつまでも決まらない）では、今日と同じ WARN が出る。
4. 決定論テスト: 窓を外した後に確定の処理を窓なしで 600 回回しても WARN が出ず、数が 0 のままであること（直す前は赤）。対照として、窓が在って見送りが続く場合は今日と同じ回数で WARN が出ること。
5. 実機: emo2 へ `emo2.nar` を入れ直して、起こし直しの後に `deferrals=600` の WARN が 0 件。

## Approach

`drain_resnap.rs` の確定の処理（`finalize_chain_once_with`）で、見送りの理由が「窓が無い」のときは数えずに戻る。数行の変更で、根本（窓が無い区間は初期配置の対象が存在しない）に 1 か所で効く。

- 採らない案: 起こし直しの終わりに数を戻す（`ghost_session.rs`・`app_exit.rs` の側）＝窓の無い区間が長いと、戻す前に WARN が出る。WARN のしきい値を上げる＝本物の異常の検出が遅れる。
- 実機で仮説が外れた場合（直しても WARN が出る）は、そこで止めて原因を取り直す。直しを重ねない。

## Scope

- **In**: 窓が無い巡を見送りに数えない修正・決定論テスト・実機 1 回の確かめ。
- **Out**: 窓の無い区間で毎フレームの処理そのものを止めること（`Emo2Wiring` の持ち方の変更）／初期配置の確定の規則／WARN のしきい値。

## Boundary Candidates

- 見送りを数える 1 か所の判定

## Out of Boundary

- 起こし直しの手順（`close_windows_for_restart`・`GhostSession`）
- 配置の計算（`crates/areka/src/placement/`）

## Upstream / Downstream

- **Upstream**: 完了 `areka-P0-ghost-shell-balloon-switch`（起こし直しの形と 11.9）・完了 `areka-P0-ghost-install`・完了 `areka-P0-network-update`（観測の出どころ）。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `areka-P0-install-live-target-hazards`（同じ「起動中のゴーストへ入れる」場面を実機で扱う。こちらは `drain_resnap.rs` だけ・あちらは `install/` と終了の受け手＝共有 0）。

## Constraints

- 触るソースは `crates/areka/src/emo2_boot/frame/drain_resnap.rs` と、兄弟テスト `crates/areka/src/emo2_boot/frame_chain_finalize_restart_tests.rs` だけ。`app_exit.rs`・`ghost_session.rs`・`crates/areka/src/placement/`・`crates/areka/src/install/` には触らない（同じウェーブの他の spec が触る）。
- 実機の根・検体はワークツリーの `target\` の下に置く。
- 実機の確かめは、判定の分岐の記録が見える level まで `RUST_LOG` を開ける。

## 想定

- 規模 XS（2〜3 タスク）。議題 0 件。Opus で足りる。
