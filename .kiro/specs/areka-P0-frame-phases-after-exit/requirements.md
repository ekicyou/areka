# Requirements Document

> 本文の実測は **2026-09-28・本ブランチ**（main `f233f720`＝棚卸⑲のコミット。ソースは `10a8d724`〔`session-mark-residue` の完了〕から不変）のもの。コードは「何の定義か」（関数名・型名＋ファイルパス）で指し、行番号では指さない。brief の行番号は起票時の目安である。
> 議題は 0 件（brief の「Fable の要否: −」）。止める位置・ログの水準・採らない案は brief で確定済みで、本書はそれを要件の形に写す。

## Project Description (Input)

**誰の何が困っているか**: α の実機確認でログの ERROR の件数を判定に使う人（開発者・`areka-P0-alpha-release-signoff`）。メニューの「終了」で areka を終えると、停止そのものは正常（記憶の書き出し・`session_mark_cleared`・終了コード 0）で利用者の画面にも何も起きないのに、ログに ERROR が 1 件と WARN が 7 件残ることがある。「正常に終えたのに ERROR が出る」ため、本物の異常と見分けにくい。

**今の状態**: 終了の相（`crates/areka/src/emo2_boot/frame.rs` の `run_ghost_quit_phase`）と毎フレームの相（同 `emo2_frame_system`）は同じ巡で続けて走る（`crates/areka/src/emo2_boot/mod.rs` の `wire_kanade_stop` が `ghost_quit_system.before(emo2_frame_system)` で登録）。終了の相は `quit_app`（`crates/areka/src/app_exit.rs`）で全ゴースト窓をその場で消してから終了を指示するが、直後の `emo2_frame_system` は終了が指示済みかを見ずに全相を回す。受信端に残っていた表示の指令（SERIKO のループのコマなど）が消えた窓へ適用され、`crates/areka-emo-present/src/scale.rs` の `derive_scale` の「窓 DPI を取得できない」`error!` 1 件と、`crates/areka-emo-present/src/mount.rs` の「装着が未完了」WARN が出る。初出は完了 `areka-P0-ghost-change-name-resolution` の `signoff.md`「2026-09-27 追記」の manual2。

**何を変えるか**: アプリの終了が指示された後の巡では、毎フレームの処理は相を 1 つも回さず、理由の分かる `debug!` を 1 行残して戻る。`quit_app` の出所（7 種類）はどれも「窓を消してから終了を指示する」ので、この 1 か所の判定で全出所・全相が同時に塞がる。決定論テストと実機の確認 1 回で固定する。

> 起票: 2026-09-28 棚卸⑲（roadmap の「登記だけの行」の格上げ）。段は **バグ**。規模 XS。

## Introduction

### 誰が困っているか

- **α の実機確認を判定する人**: メニューの終了のたびに「正常な終了なのに ERROR 1 件」が混ざり得る（emo2 は SERIKO のループ animation_id=1400 を常に回すので、終了の巡に指令が残る回がある）。ERROR 0 件を合格の条件に使えない。
- **利用者**: 画面上の害は無い。本件は利用者の見た目を変えない。

### いま何が起きているか（2026-09-28 静的に確認）

1. 終了の相 `run_ghost_quit_phase` → `quit_as_today` → `quit_app` → 私有部品 `despawn_app_windows` が全ゴースト窓（子の surface entity・文字層の枠も連鎖で）を消し、`wintf::AppExit::request_exit` を呼ぶ。
2. 毎フレームの結線状態 `Emo2Wiring`（表示の出し手と指令の受信端）は World に残る。直後の `emo2_frame_system` は `Emo2Wiring` を取り出して全相を回し、`crates/areka/src/emo2_boot/frame/drain_resnap.rs` の `run_drain_phase` が残っていた `ShowSurface` を `crates/areka-emo-present/src/presenter/show.rs` の `apply_show` へ渡す。
3. `apply_show` は消えた窓の DPI を読めず `derive_scale` へ「無し」を渡し、`error!` 1 件。続く表示・配置・可視の設定が消えた entity を叩いて WARN。
4. `run_ghost_quit_phase` の説明文は「終了が決まったフレームで他の相を走らせても、これから閉じる窓のために描き直すだけだからである」と書くが、他の相を止める判定はどこにも無い。

### 同じ穴を持つ経路と、持たない経路（呼び手の grep）

- **持つ**（`quit_app` を通る終了）: メニューの終了・別れの台詞のあと、切替の後に迎え入れたゴーストを終える（`emo2_boot/ghost_switch.rs` の `on_ghost_stopped`）、既定ゴーストへ戻せない致命（同 `fatal`）、強制退避（`input_events/mod.rs` の `ExitOrigin::Escape`）、smoke の自動終了（`main.rs`）、OS のセッションの終了（`session_end.rs`）、kanade 未結線の OS の閉鎖要求（`app_exit.rs` の `on_ghost_os_close`）。
- **持たない**: ゴースト切替（`ghost_switch.rs` の `switch_to`・`switch_to_default`）。同じ system 呼び出しの中で `Emo2Wiring` が新品へ差し替わり、古い受信端は残っていた指令ごと捨てられる。同じ実行体の manual3（切替あり・自動終了）は ERROR 0 件。

### 前例

- 完了 `areka-P0-dpi-window-vanish` の要件「終了処理でゴースト窓が破棄された後に窓寸の再導出が走ったとき、既に存在しない窓に対する警告以上の水準のログを出力しない」。本件は同じ方針を毎フレームの相全体へ広げる。
- 終了処理の正常系の読み飛ばしは `debug!`（`quit_as_today` の `ghost_quit_no_windows`・`despawn_app_windows` の `DESPAWNED_SKIP_TAG`）。

## Boundary Context

- **In scope**:
  - アプリの終了が指示された後の巡で、毎フレームの処理の全相を止める判定 1 つと、読み飛ばしの記録 1 行
  - 終了の相の説明文のうち「他の相を走らせても……」の一文を、判定の実際の場所に合わせて書き直すこと（挙動は変えない）
  - 決定論テスト（新しい兄弟のテストファイル 1 本）と実機の確認 1 回（emo2・debug 版・メニューの終了）
- **Out of scope**:
  - `derive_scale` の「窓 DPI を取得できない」の水準（`error!` のまま据え置く。生きた窓の DPI 欠落は本物の異常＝完了 `areka-P0-emo-dpi-scaling` 要件 1.4）
  - 表示の出し手（`crates/areka-emo-present/**`）の変更（「窓が無ければ読み飛ばす」案は採らない）
  - `quit_app`・`close_windows_for_restart` の変更、終了の相で `Emo2Wiring` を取り除く案
  - ゴースト切替の経路（`ghost_switch.rs`・`ghost_session.rs`）。穴が無い
  - 終了の後始末（`main.rs` の終了統括）の順序の変更
  - `quit_app` が外さない資源（重なりの鎖の計画など）が終了の巡の後段で消えた窓を読むかどうか。今回のログに症状が無い。見つかれば別件で登記する
- **Adjacent expectations**:
  - `quit_app` の不変条件「窓を消してから終了を指示する」（完了 `areka-P0-app-lifetime-separation`・`wintf::AppExit` の型の説明の契約）が保たれていること。本件の判定は「終了指示済み＝ゴースト窓はもう無い」に依る
  - 終了の受け口 `wintf::AppExit`（`crates/wintf/src/runtime/message_loop.rs`）の既存の公開の問い合わせ「指示済みか」を読むだけで、wintf は変えない
  - 下流の `areka-P0-alpha-release-signoff` は、本件の完了後「正常な終了の ERROR 0 件」を判定に使える

## Requirements

### Requirement 1: 終了が指示された後の巡では毎フレームの処理を止める

**Objective:** As a α の実機確認を判定する人, I want アプリの終了が指示された後は消えた窓へ何も適用されないこと, so that 正常な終了のログに ERROR が混ざらず、ERROR 0 件を合格の条件に使える

#### Acceptance Criteria

1. When アプリの終了が指示された後に毎フレームの処理が呼ばれたとき, the areka shall 表示の指令を受信端から 1 件も取り出さず、表示の適用・バルーンの可視性・窓寸の反映・`\![move]` の適用・重なりの鎖の確定と再解決・文字層の描画のいずれも行わない。
2. When アプリの終了が指示された後に毎フレームの処理を読み飛ばしたとき, the areka shall 読み飛ばした理由の分かる記録 `frame_phases_skipped_after_exit` を debug の水準で、読み飛ばした巡ごとに 1 行残す。
3. When アプリの終了が指示された後に毎フレームの処理を読み飛ばしたとき, the areka shall その巡で毎フレームの処理に由来する ERROR を 0 件、WARN を 0 件とする。
4. When アプリの終了が指示された後に毎フレームの処理を読み飛ばしたとき, the areka shall 毎フレームの処理の結線（表示の出し手と指令の受信端）を取り除かず壊さずに残す。
5. The areka shall 終了の出所（メニューの終了・切替の後の終了・既定ゴーストへ戻せない致命・強制退避・smoke の自動終了・OS のセッションの終了・kanade 未結線の OS の閉鎖要求）を問わず、1.1〜1.4 を同じに適用する。
6. While アプリの終了が指示されていない, the areka shall 毎フレームの処理を今日どおり全相回し、読み飛ばしの記録を 0 行とする。
7. If 終了の受け口がその構成に存在しない, then the areka shall 終了は指示されていないものとして 1.6 と同じに振る舞う。

### Requirement 2: 変えないものを変えない

**Objective:** As a 開発者, I want 本件の判定が他の経路と既存の異常の知らせを動かさないこと, so that 本物の異常の ERROR を消さず、終了と切替のふるまいが今日のまま保たれる

#### Acceptance Criteria

1. The areka shall 生きた窓で DPI が得られないときの「窓 DPI を取得できない」を今日どおり ERROR の水準で記録する。
2. The areka shall 終了の後始末（SERIKO のループの停止・記憶の書き出し・起動中の印の消去・終了コード）の内容と順序を今日のまま保つ。
3. When ゴースト切替が行われたとき, the areka shall 今日どおりの手順で切り替え、切替の途中で読み飛ばしの記録を 0 行とする。
4. The areka shall 終了の相の説明文を、他の相を止める判定が毎フレームの処理の入口にあることと一致させる（挙動の変更を伴わない）。

### Requirement 3: 決定論テストで固定する

**Objective:** As a 開発者, I want 終了指示の有無による分岐を GPU なしの決定論テストで固定すること, so that 判定が外れたり常に真になったりする退行を機械で検出できる

#### Acceptance Criteria

1. When 終了が指示済みの World で、受信端に表示の指令が 1 件ある状態で毎フレームの処理を 1 回回したとき, the テスト shall ERROR 0 件・WARN 0 件・読み飛ばしの記録 1 件・結線が World に残ること・受信端に指令が 1 件残ることを確かめる。
2. When 終了が指示されていない同じ組み立ての World で毎フレームの処理を 1 回回したとき, the テスト shall 指令が取り出されて適用されたこと（未登録の対象への適用の ERROR 1 件で観測）・読み飛ばしの記録 0 件・受信端が空であることを確かめる。
3. If 終了指示の判定を外した実装で 3.1 を回したとき, then the テスト shall 赤になる（実装時に一度確かめて記録する）。
4. The テスト shall 実装と同じディレクトリの兄弟ファイルに置き、1 ファイル 1,000 行の上限を守り、既存のテスト補助のファイルに補助の関数を足さない。
5. When 本件のテストを含む crate 単位のテストを回すとき, the 開発者 shall ファイル行数の上限の検査（`-p log-capture-kit`）も併せて回し、失敗 0 件を確かめる。

### Requirement 4: 実機で確かめる

**Objective:** As a α の実機確認を判定する人, I want emo2 でメニューから終えた実機のログで ERROR 0 件を確かめること, so that 本件の直りを実物で裏付け、下流の判定に使える

#### Acceptance Criteria

1. When emo2（debug 版・`RUST_LOG` に `areka=debug` を含む）を起動し、SERIKO のループが回るのを待ってメニューの「終了」で終えたとき, the areka shall ログの ERROR を 0 件とする。
2. When 4.1 の手順で終えたとき, the areka shall `event="app_exit"` の記録より後に、消えた窓に対する「装着が未完了」の WARN を 0 件とする。
3. When 4.1 の手順で終えたとき, the areka shall `event="app_exit"` の `origin=KanadeStopped(Quit)` を 1 件、`session_mark_cleared` を 1 件記録し、終了コード 0 で終わる。
4. When 4.1 の手順で終えたとき, the 開発者 shall 読み飛ばしの記録 `frame_phases_skipped_after_exit` の有無を記録する。記録は終了の巡に指令が残っていた回にだけ出るので、出なくても不合格としない（このことを起動の前に告げる）。
