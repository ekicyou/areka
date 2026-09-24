# Brief: areka-P0-shiori-fault-notice

> 2026-09-24 `/kiro-discovery` 再入で起票（台帳 #55）。`areka-P0-shiori-loadu` の実機確認（同 spec の `research.md` §11・§12）で見えた壊れ方のうち、**引受先がどこにも無かった**もの。
> 本文のコードは「何の定義か」（関数名・型名＋ファイルパス）で指す。事実は**起票時の実測**（2026-09-24・ブランチ `claude/kiro-start-areka-p0-shiori-d1a08b` の `216893b9`）。着手時に必ず引き直すこと。

## 2026-09-24 棚卸⑯の再測定（main `0b01f654`＝#12 の着地後）

**本仕様は次のウェーブ（B1）の先頭＝バグとして最初に着手する。** #12 は着地済みなので「実装は #12 を待つ」は解けた。想定 **5〜7 タスク**・分割不要。#13 とは `emo2_boot/frame.rs` の**同じ関数** `run_ghost_quit_phase` を触る（#13 は切替のときに `quit_app` を呼ばない分岐、本仕様は Fault の告知）ので**並走不可のまま**。#58 `ghost-restart-unit` とも `main.rs`・`app_exit.rs` を共有する＝直列（本仕様 → #58 → #13）。

**崩れた／変わった前提**

1. **告知の部品はそのままでは使い回せない。** `crates/areka/src/alert.rs` の公開口（すべて `pub(crate)`）は `alert_text(&AlertScene) -> (String, String)`・`suppressed_from(Option<&str>)`・`suppressed()`・`raise(&AlertScene, suppressed)` の 4 つ。`AlertScene` は `RootMissing`・`GhostMissing`・`BalloonMissing`・`StartupWindow` の 4 値で、題名は定数 `TITLE`＝「areka を起動できません」で共通、本文は 3 行の固定構成。**SHIORI の失敗を載せるには場面を足したうえで題名を場面ごとに分ける**（会話中の失敗に「起動できません」は合わない）。`raise` は親窓なしの `MessageBoxW(MB_OK|MB_ICONERROR)` で、必ず `error!(event="alert")` を 1 件残す。#15 `ghost-install` も同じ部品を使う（`terms.txt` の受諾／拒否には押されたボタンを返す形が要る）＝**本仕様が先に一般化し、#15 はそれに乗る**。
2. **抑止は環境変数 `AREKA_NO_ALERT`**（未設定・空・空白だけ・`"0"` なら出す、それ以外の値なら抑える）。`smoke_boot_loop_exit.rs` の `run_smoke` は 3 方向すべてで `AREKA_NO_ALERT=1` と `NO_COLOR=1` を渡し、「終了コード＋ログの目印」で判定する。Desired Outcome 4 は新しい仕組みなしで満たせる。
3. **Fault の原因は `main` まで届いていない。** `emo2_boot/frame.rs` の `run_ghost_quit_phase` が全原因を同じ 1 本で `quit_app(world, ExitOrigin::KanadeStopped(stopped.cause))` へ渡す（Fault 専用の腕は無い）。`quit_app(&mut World, ExitOrigin) -> usize`（`app_exit.rs`）は原因を `info!` に残して `AppExit::request_exit()` を呼ぶだけ。`AppExit`（`wintf/src/runtime/message_loop.rs`）は bool と通知しか持たず、`app.run()` は `Result<()>`。**`main` は `app.world()` を持っているので、`quit_app` が World に最後の `ExitOrigin` を置けば `run()` の後に読める＝wintf は無改変で済む**（推し）。
4. **終了コードの現状**（`fn main` は `Result<()>`・`Err` なら 1）: 非 0 は起動解決の失敗（`resolve_boot` の `Err`）・`open_startup_window`・`runtime.shutdown`・`seriko.join` の失敗だけ。`app.run()` が正常に戻れば理由を問わず 0（Fault もここ）。
5. **失敗の種類は kanade の中で失われている。** `KanadeStopCause`（`msg.rs`）は `Quit`・`Forced`・`CloseSilent`・`DeadlineExceeded`・`Fault` の 5 値で中身を持たない `Copy` 型。`schedule/mod.rs` の `to_unloading_fault` も `TermCause::Fault`（中身なし）へ倒すだけ。一方、内側には `ShioriFailure`（`Handshake`・`Timeout`・`Ipc`・`Shiori`・`Internal` の 5 種）と `Input::ShioriDown{reason: String}` が残っている。**Desired Outcome 1「失敗の種類が平易な言葉で載る」を満たすには `msg.rs`・`actor.rs`（`stop_cause_of`・`notify_stop`）・`schedule/mod.rs`（`TermCause`・`to_unloading_fault`）を触る**＝brief が範囲外とした「各腕の判断」とは別に**原因を運ぶ変更**が要る（議題 2）。
6. **「起動時」と「会話中」は `main` からは見分けがつかない。** 接続の失敗は `shiori/real.rs` の `spawn_shiori_actor` でアクターのスレッド上で非同期に起き、`connect_failed` → `KanadeMsg::ShioriDown` → `shiori_down` の腕で Fault に入る。`areka_ghost::boot_with_kanade_stop` 自体は成功を返すので、窓が出てから消える形。brief が挙げた actor の「期限切れの腕」（`round_trip` の `shiori_reply_timeout`）は無限に待つ `recv()` の防御用で実際には到達しない——本当の期限切れは境界で写す `ShioriFailure::Timeout`（`real.rs` の `map_error`）から `shiori_failed` の腕へ入る経路。`notify_stop` は原因不明のときも Fault として通知する（`stop_cause_unknown`）。
7. **新しい穴（静的に確認・実走はしていない）**: `wire_emo2_boot` が不成立で LogSink 側の起動に倒れた場合、`main` は `areka_ghost::boot`（＝`boot_with_kanade_stop(options, None)`）を使い、停止通知の受け口が無い。Fault が起きても**窓が残ったまま kanade だけが止まる**と読める＝「黙って消える」ではなく「黙って固まる」（議題 5）。
8. **要件 3.8 と #12 の裁定は衝突していない。** 完了 `app-lifetime-separation` 要件 3.8 の原文は「The areka shall 終了操作 7 種のどれでも、終了の指示のあとの後始末の順序と終了コード（0）を今日と同じに保つ。」。3.1 の「kanade の終了系列の完了通知」は原因を区別しないので Fault は構造上 3.8（0）に含まれる。#12 の非 0（要件 1.4・4.7・4.8・5.8・6.3・6.4）は起動前の失敗だけで Fault に触れていない。**Fault に効いているのは 3.8（0）**＝非 0 にするなら本仕様で Fault に限って上書きし `doc/COMPAT_ARCHITECTURE.md` §8 に記す。

**用意済みの部品**: x64 で SHIORI を決定論的に失敗させる口＝`areka_ghost::ShioriWiring::Custom(connect closure)`（`Err(String)` で `connect_failed`）・kanade の中で `Input::ShioriDown`／`ShioriOutcome::Failed(..)` を直接投入・`frame_ghost_quit_tests.rs`（`wiring_with_stop` に偽の `KanadeStopped` を流す。今は Quit／Forced だけ＝Fault の case を足すだけ）。実機用の失敗検体＝`crates/shiori-host32-testdll-loadu`（`shiori_loadu.dll`・`HOST32_TESTDLL_LOADU_FAIL=1` で `loadu` が 0 を返し、helper の `ProxyError::LoadReturnedFalse` は `load` へ落ちないので確実に失敗する）。ほかに `shiori-host32-testdll`・`shiori4-testdll`。文面を純関数で固める型は `alert_tests.rs` を写せる。

**触るファイル**: 高＝`alert.rs`（＋`alert_tests.rs`）・`app_exit.rs`（＋tests）・`emo2_boot/frame.rs`・`main.rs`（`app.run()` の後の終了コード）・`tests/smoke_boot_loop_exit.rs`（Fault 方向を 1 本足すなら）・§8。中（失敗の種類を載せる場合）＝kanade `msg.rs`・`actor.rs`・`schedule/mod.rs`。低（会話中の 500 を Fault から外す場合）＝`schedule/steady.rs`・`schedule/mod.rs` の `on_shiori_reply`。

**相乗り候補（要件で決める）＝台帳 #57「起動後の途中終了で記憶の確定が飛ぶ」（XS）**: `fn main` は `app.run()?` の**後**で `GhostRuntime::shutdown` を呼ぶので、`app.run()` が `Err` を返すと（main スレッドの panic も同様）shutdown を通らず、boot 直後に投函した記憶（`[last]`）や窓位置の永続化が sylphya のアクターに処理される前にプロセスごと消えうる（実害は未観測・`GhostRuntime` は `Drop` を実装しない）。**本仕様は `fn main` の `app.run()` の後ろ＝同じ場所を書き換える**ので、`Err` の腕でも終了順序を通す形を同じ手で入れれば 1〜2 タスクで閉じる（`Drop` での自動 shutdown は終了理由を持てないので採らない）。相乗り 1 件＝`runtime.rs` の `GhostBootError` の doc が退役済みの「ダミー窓」を今の挙動として書いている。**ただし #58 が終了順序を関数に括り出すので、#57 の直しはその関数の中に置くのが筋**＝本仕様で採るか #58 へ送るかを要件で決める（#58 へ送るなら本仕様は #57 に触らない）。

**要件段階の議題（Fable）**: ⑴ Fault の終了コードを 0 以外にするか（要件 3.8 の上書き。**smoke の 500 ms の自動終了より先に Fault が起きる場合＝i686 helper の成果物が無いときに smoke ①② が赤になりうる競合は未測定**）。⑵ 告知に失敗の種類まで載せるか（載せるなら kanade 3 本に手が入り #13 との共有が 2 本 → 5 本）。⑶ 会話中のエラー応答（500 など）で終了し続けるか、choice と同じく 204 扱いで会話を続けるか。⑷ 告知を出す場所——`run()` の後の `main`（推し）か、frame の相（ECS の system の中で模態の `MessageBoxW` を回す＝入れ子のメッセージループの危険）か。⑸ 前提 7（LogSink 側の起動で Fault が起きると窓が残って固まる）を本仕様で拾うか別に起票するか。

## Problem

**誰の何が困っているか**: α の利用者（第三者）。自分で入れたゴーストの SHIORI が動かないとき、**アプリが一瞬で消え、何が起きたのか画面では何も分からない**。

`areka-P0-shiori-loadu` のタスク 1 で、今日のコードのまま既定コードページに無い字を含むフォルダから検体を起動した（`research.md` §11）。

- YAYA（`konnoyayame`）: 初期化は通るが、最初の問い合わせが 500 で返る → `kanade` が終了の流れ（Fault）へ入る → 起動から **0.47 秒**でアプリが終わる。
- pasta（`emo2`）: 初期化が偽を返す（`LoadReturnedFalse`）→ 接続失敗 → Fault → **1.7 秒**で終わる。
- どちらも**終了コードは 0**。ログには `ERROR` が残るが、利用者向けの表示は 0 件。

`loadu` の実装でこの 2 体は直ったが、**「SHIORI が動かないとアプリが黙って消える」という形そのものは残っている**。第三者が持ち込むゴーストでは次のどれでも同じことが起きる: 32bit でない DLL・依存 DLL の欠落・壊れた辞書・SHIORI の内部エラー・応答の期限切れ。

今日の経路（すべてログは出る＝ログ無しの失敗ではない。欠けているのは**利用者への告知**と、**失敗で終わったことを外から区別する手段**）:

- SHIORI の失敗はどれも Fault へ入る: `crates/areka-kanade/src/schedule/mod.rs` の SHIORI 呼出失敗の腕（`event="shiori_failed"`）と死活報告の腕（`event="shiori_down"`）、`crates/areka-kanade/src/actor.rs` の送出失敗・応答の切断・期限切れの 3 腕。
- Fault は `crates/areka/src/emo2_boot/frame.rs` で `quit_app(world, ExitOrigin::KanadeStopped(cause))`（`crates/areka/src/app_exit.rs`）へ渡り、全窓を閉じてアプリを終える。
- `fn main`（`crates/areka/src/main.rs`）は終了の理由によらず終了コード 0 で戻る。完了 `areka-P0-app-lifetime-separation` の要件 3.8 は「終了操作 7 種のどれでも終了コード 0 を今日と同じに保つ」と書いたが、**SHIORI の失敗で終わる場合をどう見せるかは決めていない**（「今日と同じ」を保っただけ）。

**引受先が無いことの確認（2026-09-24）**: main の brief と roadmap を `Fault`・`connect_failed`・`SHIORI.*失敗` で検索した。

- `baseware-root-layout`（#12）が持つのは「ゴーストの根が**無い**」ときの告知（`MessageBoxW`）と非 0 終了だけで、根は在るが SHIORI が動かない場合は範囲外。
- `ghost-shell-balloon-switch`（#13）が持つのは「**切替先**のゴーストが起動できない」ときの扱いだけで、起動時と会話中の失敗は範囲外。
- `popup-menu-residue`（#44）の 6 番は、照会の往復の失敗の記録の**文言**だけを扱う。

## Desired Outcome

完了時に次が真になっている。

1. **起動時**に SHIORI が確立できない（DLL が読めない・入口が無い・初期化が偽・最初の問い合わせの失敗）とき、利用者に**何が起きたかを告げる**表示が出て、そのうえでアプリが終わる。表示には、どのゴーストか（名前か置き場所）と、失敗の種類が平易な言葉で載る。
2. **会話中**に SHIORI が失敗した（応答の期限切れ・helper の異常終了・エラー応答）ときも、黙って消えない。告知して終わるか、ゴーストを降ろして告知するかは要件で決める。**そもそもエラー応答（`500` など）でアプリを終えるべきかも要件の議題**——正典 SHIORI/3.0 の「ステータスコード」は `500` を「SHIORI内部でなにかしらのエラーが起き、レスポンスを返せなかった」と定義するだけで、ベースウェアが終わるべきとは書いていない。今日の areka はエラー応答を一律に Fault へ入れている（選択肢の問い合わせだけは `choice_shiori_failed_as_204` で 204 と同じに扱い、会話を続けている＝`crates/areka-kanade/src/schedule/steady.rs`）。
3. SHIORI の失敗で終わったことが、ログの外からも区別できる（終了コードを非 0 にするか。完了 `app-lifetime-separation` 要件 3.8 との関係は要件で決める）。
4. 常設の smoke テスト（`crates/areka/tests/smoke_boot_loop_exit.rs`）と、有界の自動終了（`AREKA_APP_SMOKE_EXIT_MS`）での無人の走行が、**告知の窓で止まらない**。
5. 判断の分岐（どの失敗が告知の対象か・どの終わり方か）は決定論テストで固定されている。窓そのものは実機で一度見る。

## Approach

`kanade` の Fault の原因と `ExitOrigin::KanadeStopped(Fault)` の経路はすでに 1 本にまとまっているので、**告知はその 1 か所（アプリが Fault で終わる場所）に置く**のが第一候補。SHIORI の各失敗の腕には手を入れない。

告知の手段は `baseware-root-layout`（#12）が「ゴーストが無い」のために作る利用者向け告知と**同じもの**を使う（2 系統にしない）。#12 の brief は「`MessageBoxW` の告知が常設の smoke テストを止める」問題を実測で見つけて、その解き方を持つ。本 spec は同じ解き方に乗る。

## Scope

- **In**: 起動時と会話中の SHIORI の失敗に対する利用者向けの告知／SHIORI の失敗で終わったことを外から区別する手段／告知が無人の走行を止めないこと／判断の分岐の決定論テスト／実機での一度の目視。
- **Out**:
  - SHIORI の失敗そのものを減らすこと（原因ごとの修正は各 spec）。
  - 切替先のゴーストが起動できない場合（#13 が持つ）。
  - ゴーストの根が無い場合（#12 が持つ）。
  - 照会の往復の失敗の記録の文言（#44 の 6 番）。
  - 失敗したゴーストを別のゴーストで起こし直すこと（α 後・切替の仕組みが要る）。

## Boundary Candidates

- アプリが Fault で終わる場所の告知（`crates/areka/src/emo2_boot/frame.rs` の終了の腕と `crates/areka/src/app_exit.rs`）
- 告知の文面の組み立て（失敗の種類 → 平易な文。純関数で固定できる）
- 終了コードの決め方（`fn main` の戻り）

## Out of Boundary

- `areka-kanade` の Fault へ入る各腕（`schedule/mod.rs`・`actor.rs`）の判断
- `shiori-host32-helper`／`shiori-host32-host` の失敗の種類と記録
- 告知の仕組みそのものの作り方（#12 が作る）

## Upstream / Downstream

- **Upstream**: `baseware-root-layout`（#12）の利用者向け告知の仕組み（**実装は #12 の着地を待つ**。要件と設計は先行できる）・完了 `areka-P0-app-lifetime-separation`（`quit_app`・`ExitOrigin`）・`areka-P0-shiori-loadu`（本 spec の出どころ・着地済みであること）。
- **Downstream**: `areka-P0-alpha-release-signoff`（#17）の既知の制限の一覧から「SHIORI が動かないとアプリが黙って消える」を外せる。`ghost-install`（#15）は、入れた直後に起動したゴーストが動かない場合に同じ告知へ落ちる。

## Existing Spec Touchpoints

- **Extends**: なし（新しい振る舞い）。完了 `app-lifetime-separation` 要件 3.8（終了コード 0）を上書きするかは要件で決め、上書きするなら `doc/COMPAT_ARCHITECTURE.md` §8 に記す（完了 spec の文書は変えない）。
- **Adjacent**: #12（告知の仕組み・`crates/areka/src/main.rs`・`smoke_boot_loop_exit.rs` を共有）／#13（切替の失敗の扱い・`crates/areka/src/main.rs` の終了順序を触る）。**#12 と #13 とは並走させない**。

## Constraints

- 規模 **S**（タスク 4〜7 本）。
- 段は **バグ**（利用者から見える実害＝第三者のゴーストが動かないと理由が分からないまま消える。2026-09-24 に実機で観測）。
- 正典 ukadoc は、SHIORI が動かないときにベースウェアが何を見せるかに沈黙している（DLL 共通仕様は `load`／`loadu` の戻りを「初期化に成功した場合は TRUE を、失敗した場合は FALSE を返却する」とだけ書く）。見せ方は areka の裁量で、`doc/COMPAT_ARCHITECTURE.md` §8 に記す。SSP の挙動を実測して合わせることはしない（記憶 no-ssp-measurement-import-semantics-from-ukadoc）。
- 実機の確認は有界の自動終了で回し、`RUST_LOG` は判定の分岐の水準まで開ける（記憶 real-machine-signoff-needs-trace-level-for-the-deciding-branch）。再現には `areka-P0-shiori-loadu` の `research.md` §11 の手順が使える。ただし `loadu` で直った 2 体の代わりに、**SHIORI を確実に失敗させる検体が要る**。既存の偽 DLL の注入で作るか、要件で決める。
- 文書と報告では「SHIORI が動かないとき、何が起きたかを利用者に告げる」と平易に書く（記憶 no-project-jargon-in-user-facing-docs）。
