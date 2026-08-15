# ギャップ分析（validate-gap）: areka-P0-dpi-transition-atomicity

- 作成日: 2026-08-15
- 対象: `requirements.md`（生成済み・未承認）と現行ツリー（`main` に W6.5 の 3 本＝exact／wpl／budget が全て取り込まれた状態）
- 方針: 本書は**情報と選択肢**を出す。最終決定は要件ディスカッション／設計へ委ねる。引用する `file:line` はすべて 2026-08-15 の現行ツリーで開いて確認した（読み違いを避けるため、行番号は本文の該当行を指す）。
- 用語: 「決定論テスト」＝実機・GPU 無しで毎回同じ結果になる自動テスト。「一括 flush」＝フレーム末尾でキューに積まれた `SetWindowPos` をまとめて実行すること。

---

## 0. 要約（3〜5 点）

1. **書込経路は 2 本で確定**。経路 A（`WM_DPICHANGED` 受理時の同期書込・`crates/wintf/src/ecs/window_proc/window_pos.rs:303-443`）はゴースト窓では `ExternalAuthority` 政策により**書込 0 回が期待値**（:372-374 で採否・:407-434 で採用時のみ書く）。経路 B（フレーム末尾の一括 flush・`crates/wintf/src/ecs/window/command.rs:155-167` で積み、`crates/wintf/src/runtime/tick_bridge.rs:200` で `flush_window_pos_commands()`）が実質唯一の書き手。**1 tick の内側では「サーフェス更新（中盤・`FrameFinalize` の `emo2_frame_system`）→ 窓書込（末尾 flush）」の順序が構造で固定されている**が、フレーム番号つきの観測は存在しない。
2. **観測基盤は「流用＋不足分の増設」が現実解**。`presenter/timing.rs` の段階別計時（`Stage`＝CacheLookup/Compose/Resample/MaskGen/Upload・`show.rs:414` で発行）はフレーム番号・窓（scope/種別）・「バッファ寸変更が起きたか」を持たない。窓書込側も `guarded_set_window_pos` の debug 行（`command.rs:97-102`）はフレーム番号と「同期か flush か」を持たない。モニタ表更新（`monitor_systems.rs:280-316`）もフレーム番号を持たない。**フレーム番号の供給源は wintf の `FrameCount` Resource（`schedule_labels.rs:8`・`world/mod.rs:503-504` で毎 tick +1）で、World を借用できる点では読めるが、flush 点は World 借用外**にある——ここが設計上の穴。
3. **+36px（作業領域非追随）は静的構造で確定済み**。`MonitorSnapshot` は起動時に 1 度だけ構築（`main.rs:530-538`・:573-574）され、以後どこも更新しない（`work_area.rs:12-24`）。一方 wintf のモニタ表は `WM_DISPLAYCHANGE`（`lifecycle.rs:122-134`）→ `Update` スケジュールの `detect_display_change_system`（`world/mod.rs:209-225`・`monitor_systems.rs:195-239`）で更新され、同じ関数が窓 DPI を再導出する（:380）。接地点は `resize_window_to` が `MonitorSnapshot` から導く（`window_move.rs:288-289` → `anchor.rs:105-157`）。**既存の決定論テストは `MonitorSnapshot` を手で差し替えて DPI 変化を模している**（`frame_dpi_reproject_tests.rs:148,170,297,450`）ため、本番に「差し替える者がいない」ことをテストが見逃す構造になっている。
4. **一度きり確定の解き直しには隠れた前提がある**。`finalize_chain` は「現在位置 ≠ 既定位置なら動かさない」（`chain_finalize.rs:108`）で、既定位置は spawn 時の値（`spawn.rs:304-320`）を持ち、resnap／DPI 再射影で中央保存により X が動いても既定位置は更新されない。**DPI 遷移では全キャラ窓の X が動くので、`ChainFinalized` を外すだけでは全スコープが「動かされた」扱いになり解き直しが空振りする**。解き直しを採るなら「既定位置の追跡規則」を併せて裁定する必要がある（Requirement 6 の設計判断項目として追加）。
5. **決定論テストの土台は揃っている**。areka 側: `frame_test_support.rs`（偽 `WindowHandle`・`Arrangement` を書込の証人にする方式・`s2_snapshot(dpi)`・`s2_ground_point`・`FakeReports`）、ログ捕捉ハーネス（`placement/test_support.rs`）。wintf 側: `capture_under_filter`（`ecs/test_support.rs:96-115`・crate 内限定）と `SingleThreaded` 実行器での捕捉（`monitor_systems_tests.rs:199`）。不足は「フレーム番号を持つ World での多フレーム駆動」と「flush 点の観測」の 2 点。

---

## 1. 現状調査（Current State）

### 1.1 遷移 1 回の現行データフロー（構造事実・file:line つき）

```
[OS] 拡大率変更
 ├─(a) WM_DPICHANGED（窓ごと・wndproc・tick の外）
 │      window_pos.rs:303 WM_DPICHANGED
 │        :345-365 DPI component を新値へ（Changed<DPI> 発火・無条件）
 │        :372-374 dpi_suggested_position_decision → ExternalAuthority なら None
 │        :407-434 Some のときだけ guarded_set_window_pos（位置のみ・SWP_NOSIZE）＝経路 A
 │        :442 Some(LRESULT(0)) を返し DefWindowProc の既定適用を止める
 └─(b) WM_DISPLAYCHANGE（lifecycle.rs:122-134 → App::mark_display_change）
        次 tick の Update: detect_display_change_system（world/mod.rs:213）
          monitor_systems.rs:218 enumerate_monitors → :224 apply_monitor_snapshot
            :279 値の変化で Monitor entity 更新（:280-316 debug 行）
            :380 redrive_window_dpi_for_updated_monitors → 窓 DPI 再導出（Changed<DPI>）

[tick N] try_tick_world（world/mod.rs:481-536）
   :503-504 FrameCount +1
   :518-530 Input … Update … FrameFinalize の 13 本
   FrameFinalize: emo2_frame_system（frame.rs:155・登録 emo2_boot/mod.rs:465）
     :168 run_dpi_phase → dpi.rs:232 dpi_phase_with
        :242-252 Changed<DPI> の窓を collect
        :287 refresh_scale_report → refresh.rs:52 refresh_scale
             refresh.rs:91 apply_show（show.rs:43）
               show.rs:56  段階計時 start
               show.rs:281-282 chain/mount 生成（初回のみ）
               show.rs:297 chain.upload → chain.rs:173
                   chain.rs:178-194 外形変化なら ResizeBuffers（バッファ寸変更）
                   chain.rs:224 Present(0)（★ここで供給面の新寸が DWM へ提示される）
               show.rs:306 size = chain.size()
               show.rs:313 マスク差替 / :322 可視化 / :328 set_bounds（mount.rs:242-264: Arrangement＋SpriteVisual::SetSize）
               show.rs:348-357 pending_resize（物理寸が変わったときだけ）
               show.rs:388-405 info 行 / :413-422 perf 行（timing.rs:167-208）
             refresh.rs:116 take_pending_resize
        :290 Some → reconcile_window_size（dpi.rs:124）
              Char → resize_window_to（window_move.rs:158）
                 :274-283 下端中央の保存 / :288-289 MonitorSnapshot から接地点再導出
                 :298-306 可視性ガード / :310-318 べき等 skip
                 :323-332 enqueue_window_set_pos（★書込 1 本）
                 :342-350 手順 5a キーワード基本位置の一度きり再導出
                 :366-374 follow_balloon（★随伴バルーンの書込 1 本）
              Balloon → resize_window_keep_position（:902-965）（★寸のみ書込 1 本）
        :295-301 None → Char は reproject_char_window_at_current_size（:335-359）
     :187 reconcile_reported_sizes（refresh が消費済みなら何も出ない）
     :195 resnap_shell_targets（drain_resnap.rs:167 → :205 resnap_with → :121 resnap_from_sizes・:143 同寸 skip）
     :199 finalize_chain_once（drain_resnap.rs:269 → :294 finalize_chain_once_with・:299-301 一度きり）
   （enqueue はすべて command.rs:155-167 の thread_local キューへ）
[tick N の後] tick_bridge.rs:200 flush_window_pos_commands（command.rs:173-206・enqueue 順に guarded_set_window_pos）
```

**確定できる事実**:

- サーフェス寸の変化（`ResizeBuffers`＋`Present`）はフレーム**中盤**、窓寸の変化はフレーム**末尾**（World 借用解放後）で、同一 tick 内であれば「1 vblank 未満の差」に収まる。ただし **tick 全体の所要が 1 vblank を超えると、DWM の合成 1 回ぶんだけ「新寸のサーフェス × 旧矩形の窓」が見え得る**（構造上の可能性・実測は Requirement 1）。budget 着地後の 1 コマ適用は release p50 5.7ms・p95 9.1ms（`completed/areka-P0-recompose-budget/remeasure-2026-08-15.md` §2）で、4 窓を同一 tick で回すと 20〜36ms になり得る。
- 窓ごとの書込回数の現行期待値（同一 tick に全窓の `Changed<DPI>` が揃った場合）: **キャラ窓 1 回**（`DpiReproject`・位置＋寸）、**バルーン窓 2 回**（`KeepPositionResize` の寸＋`BalloonFollow` の位置・順序は Query の走査順に依存）。経路 A は 0 回が期待値。これが Requirement 4.5 の「回数」の設計入力になる。
- 随伴の同一フレーム性は「キャラ窓とバルーン窓の `Changed<DPI>` が同一 tick に揃うか」に依存する。経路 (b) の再導出は同一 system 内で全窓に及ぶ（`monitor_systems.rs:380`）ので揃う。経路 (a) はメッセージが tick を跨いで届き得るため揃わない可能性がある（旧観測の「窓ごとに 60〜90ms」がこの形と整合するかは再観測で確定）。
- 経路 A と経路 (b) の両方が同じ遷移で発火すると `Changed<DPI>` が 2 度発火するが、`refresh_scale` の k 不変ゲート（`refresh.rs:70-73`）と `resize_window_to` のべき等 skip（:310-318）で 2 度目は書込ゼロに吸収される（churn なし）。
- 一括 flush キューへ積む本番の書き手は 3 箇所: areka の単一ライター（`window_move.rs:559`）、wintf の Z 維持系（`zorder_pair_maintain.rs:475`・Z のみ）、wintf の `apply_window_pos_changes`（`graphics/systems/window_pos.rs:89-98`・`Changed<WindowPos>` 駆動。ゴースト窓は bypass ミラー＝`window_move.rs:569-577` ゆえ発火しない）。

### 1.2 既存の観測点（流用候補）と足りないフィールド

| 観測点 | 場所 | 水準／target | 持っている | 持っていない（Requirement 2 が要る） |
|---|---|---|---|---|
| 窓書込の実施 | `command.rs:97-102`（`guarded_set_window_pos`） | debug／`wintf::ecs::window` | hwnd・x/y/cx/cy・flags | フレーム番号・同期か flush か・要求経路語彙・scope/種別 |
| 窓書込の要求（areka） | `window_move.rs:606-611` → `diag.rs:355-374`（`[diag.window_move]`） | debug／`areka::placement::diag`（`diag.rs:69`） | route・entity・kind・scope・x/y/w/h・dpi | フレーム番号（World 内なので `FrameCount` を読める） |
| 提案位置の採否（経路 A） | `window_pos.rs:392-403` | debug／`wintf::ecs::window_proc` | entity・hwnd・policy・applied | フレーム番号 |
| 段階別計時 | `timing.rs:191-207`（perf 行・`show.rs:414` 発行） | debug／`areka_emo_present::presenter::timing` | target_id・段別 µs・alloc・key_hash | フレーム番号・scope/種別（`TargetId` は `target_map` の規則 shell=2·scope／balloon=2·scope+1 で機械的に対応づけ可能）・「バッファ寸変更が起きたか」 |
| 表示成立点 | `show.rs:388-405` | **info**／同上 | scaled_w/h・size_changed・k | フレーム番号・段階（バッファ寸変更／アップロード／可視化） |
| バッファ寸変更 | `chain.rs:178-194` | ログ無し | — | 全部 |
| モニタ表更新 | `monitor_systems.rs:280-316` | debug／`wintf::ecs::layout::systems::monitor_systems` | 新旧 bounds/work_area/dpi | フレーム番号 |
| モニタスナップショット | `diag.rs:383-392`（起動時 1 回） | debug／`areka::placement::diag` | 全モニタ | 更新時には出ない（更新経路が無い） |

`FrameCount` は `world/mod.rs:503-504` で `try_tick_world` の冒頭に +1 され、13 本のスケジュール全体で同じ値を読める。**flush 点（`tick_one_frame` の :200）は World 借用を解いた後**なので、フレーム番号を運ぶには (i) enqueue 時に `SetWindowPosCommand` へ焼き込む、(ii) `try_tick_world` が thread_local の「現在フレーム番号」ミラーを更新して flush が読む、(iii) `tick_one_frame` が flush へ番号を渡す、のいずれかが要る。

### 1.3 作業領域源の実態（Requirement 5）

- `MonitorSnapshot`（`work_area.rs:20-24`・`RectPx` の列だけの純データ）は `main.rs:573-574` で `enumerate_monitors()` から 1 度だけ作られ Resource 挿入される。doc に「セッション内固定＝M1 受容（`WM_DISPLAYCHANGE` 追随は後続・DD15）」（:17）と明記された**意図的な先送り**である。
- 消費者は多い: `resize_window_to`（:288）・ドラッグ（`drag_follow.rs:290,472,871`）・可視性ガード（`visibility.rs`）・limit 関門（`window_move.rs:806-808`）・永続復元（`persist.rs:164,324,336`）・`balloon_limit.rs:171`。**「起動時固定の値を読む前提」に依存している判断は無い**（どれも「いま挿入されている snapshot」を都度読む）ため、Resource を差し替えても呼出側の変更は要らない。
- wintf 側のモニタ表は `Monitor` component（entity 単位）で、`apply_monitor_snapshot`（`monitor_systems.rs:253-381`）が値変化・追加・削除を扱う。areka はこの entity を直接読んでいない。
- 接地点の導出は `project_anchor`（`anchor.rs:105-157`）→ Bottom は `BottomSnapPolicy`（:118-120）。work area は「生位置に置いた矩形の中心が属するモニタ」から live に引く（:131-141）。snapshot が新しければ、次に `resize_window_to` が呼ばれた時点で自然に新下端へ着地する。
- **再スナップの契機**として現存するのは (1) `Changed<DPI>`（`dpi_phase_with`）、(2) シェル面寸変化（`resnap_from_sizes`・寸が変わったときのみ）、(3) `Changed<Anchored>`。作業領域変化を契機にするものは無い。

### 1.4 一度きり確定の実態（Requirement 6）

- `ChainFinalized`（`chain_finalize.rs:48`）は Resource の存在で判定（`drain_resnap.rs:299-301`）。挿入は `:338`。解除経路は無い。
- 駆動条件（`collect_chain_states` `:353-409`）: 全スコープで実表示寸が引け・`WindowPos.size` と一致（:388-394）。**同一 tick 内**で `apply_show` → `enqueue`（bypass ミラーで `WindowPos.size` を即時更新・`window_move.rs:569-577`）が済むため、遷移した窓は同じ tick の末尾で「landing 済み」と判定される。したがって遷移が複数 tick に分かれると、**まだ遷移していないスコープが旧寸で「一致」しているだけで全体が landing 済みと見なされ、途中フレームで解き直しが走り得る**（Requirement 6.2 の「中間フレームでは行わない」は現行ガードだけでは満たせない）。
- `finalize_chain`（`:99-131`）の 7.3 判定 `:108`: `default_x != Some(current_x)` なら動かさない。既定位置は spawn 時値（`spawn.rs:304-306`）で、更新は確定時のみ（`drain_resnap.rs:333`）。**`resize_window_to` の中央保存（`window_move.rs:274-283`）で X が動いても既定位置は据え置き**なので、DPI 遷移後は全スコープが「動かされた」判定になる。
- 停滞診断: `ChainFinalizeStall`（`chain_finalize.rs:160-165`）・`note_chain_deferral`（:249-259）・600 フレーム（:153）・一度きり警告（`drain_resnap.rs:415-429`）。解除時にこれも初期化しないと 2 度目の待ちは無音（brief 追記(63)のとおり）。
- キーワード基本位置: `BalloonKeywordBase` を `rederive_keyword_balloon_offset`（`keyword_base.rs:59-163`）が「寸が変わった最初の書込」で消費・除去（:79-81・:148）。DPI 遷移では消費済みなので再導出は起きない。再導出するなら素材（mode/adjust）を保持し直す仕組みが要る（`world.entity_mut().remove::<BalloonKeywordBase>()` を「消費済み標識つきで残す」等）。ただしキーワード式は `(char_w − balloon_w)/2` の**両寸の物理量**から出るため、DPI で両寸が同じ k で伸びれば結果は k 倍——`balloon-offset-dpi` の「offset を k 倍するか」と同じ問いに帰着する。

### 1.5 テスト資産（Requirement 7）

- areka: `frame_test_support.rs:216-241` `resnap_world`（実 `spawn_ghost_windows`＋偽 `WindowHandle`＋`MonitorSnapshot`）、`:328-342` `dpi_world`（`DPI` 96 と書込証人 `Arrangement`）、`:406-414` `s2_work_area_for_dpi(dpi)`（タスクバー物理高を dpi/96 倍する合成 work area）、`:431-435` `s2_snapshot`、`:439-443` `s2_ground_point`、`:351-391` `FakeReports`（`ScaleReportSource` の決定論版）、`:256-282` `FakeSizes`（`PhysicalSizeSource`）。ログ捕捉は `placement/test_support.rs`（interest キャッシュの問題を回避したハーネス）。
- wintf: `capture_under_filter`（`ecs/test_support.rs:96-115`・`pub(crate)`）、モニタ表更新の決定論テスト（`monitor_systems_tests.rs:199` で `SingleThreaded`）。
- 不足: (1) `FrameCount` を持つ World で「複数フレームにまたがる遷移」を駆動するハーネス（`emo2_frame_system` を複数回回し、間に DPI／snapshot を差し替える）、(2) 一括 flush キューの内容を検査する口（現在は `WINDOW_POS_COMMANDS` が private・`flush` は実 `SetWindowPos` を呼ぶ＝偽 hwnd では失敗して warn）、(3) 判定語の純関数（レコード組立）と正例／負例の対。
- 記憶事項の反映: 常時テストは x64・偽境界（`prefer-x64-fake-boundary-tests-not-x86`）。既定スケジュールは多スレッドでログ捕捉が空になる（`areka-log-cage-harness-blindspots`）ため、`Update` を回すテストは `SingleThreaded` を明示。

---

## 2. 要件→資産マップ（gap タグ: Missing／Unknown／Constraint）

| 要件 | 既存資産 | ギャップ |
|---|---|---|
| R1 再観測 | `dpi-window-vanish` 診断手順書セッション②（`diagnosis-procedure.md` §6.2・`RUST_LOG` 例 :150・`AREKA_APP_SMOKE_EXIT_MS`）／budget の `judge-perf.py` と perf 行 | **Unknown**: 現行ログでは「同期か flush か」「フレーム番号」が読めないため、旧手順のままでは R1.3 の「順序・起点からの遅れ」がフレーム単位で出ない（時刻での近似になる）。**Constraint**: 既定 OFF の観測点を点灯せずに 0 回と言わない（手順書 §1 の教訓）。 |
| R2 観測基盤 | `placement::diag`（純関数レコード・専用 target・既定 OFF）／`timing.rs`／`FrameCount` | **Missing**: フレーム番号の配管（特に flush 点）・書込種別（同期／flush）・バッファ寸変更の記録・モニタ表更新のフレーム番号・レコード純関数と語彙固定テスト。**Constraint**: wintf→areka の import 禁止＝target 定数と scope 語彙は wintf に置くか、wintf 側は entity/hwnd を結合キーにして areka 側レコードと 2 段 grep で結ぶ（zorder-pair の前例）。 |
| R3 台帳 | `dpi-window-vanish/diagnosis-report.md` の様式 | 資料作成のみ。**Unknown**: DWM 合成の 1 回ぶんの中間状態は tick 単位ログでは見えない（§4 研究項目）。 |
| R4 原子性 | 単一 tick 内順序（構造）・べき等 skip・flush 一括 | **Unknown**: 中間状態が残るか（R1 待ち）。**Missing**: 有界フレーム数・書込回数の回帰テスト。多窓の 1 バッチ化（`DeferWindowPos`）や tick 跨ぎのバリアは未実装。 |
| R5 作業領域追随 | `MonitorSnapshot`・`project_anchor`・`reproject_char_window_at_current_size` | **Missing**: snapshot の更新経路と、更新を契機とする再スナップ。テストは snapshot 差し替えで模しているので土台はある。 |
| R6 寿命裁定 | `ChainFinalized`／`ChainFinalizeStall`／`BalloonKeywordBase` | **Missing**: 解除条件・停滞初期化・既定位置の追跡規則（§1.4）。**Constraint**: 途中フレームで解かない。 |
| R7 回帰テスト | §1.5 | **Missing**: 多フレーム駆動ハーネス・flush キュー検査口・語彙固定テスト。 |
| R8 サインオフ | 手順書テンプレ | 手順書の改訂（新 target・判定語・遷移回数の数え方）。 |
| R9 縮退判定 | — | 実測待ち。判定に使う 3 条件は §5 の観測量へ写像できる。 |
| R10 回帰境界 | 各着地物 | **Constraint**: budget の定常アロケーション 0（`show.rs` の観測増設は確保を伴わない形で）／zorder の Z 指令の順序（flush を改造するなら enqueue 順を保つ）／wpl・scg の一度きり。 |

---

## 3. 論点別の詳細（依頼された (a)〜(f)）

### (a) 2 本の書込経路と「同一フレームコミット」に要るもの

**現状**: 経路 A は同期・即時（World 借用の外・`window_pos.rs:420-430`）、経路 B はキュー→末尾 flush。ゴースト窓では A が 0 回のはずなので、実質「1 tick に 1 バッチ」は既に成立している。足りないのは次の 3 点。

1. **tick を跨ぐ分裂の防止**（複数窓の `Changed<DPI>` が別 tick に落ちる場合）。選択肢:
   - A-1 何もしない（再観測で分裂が無ければ不要）。
   - A-2 「遷移バリア」: dpi 相で「同一モニタの全ゴースト窓の DPI が同じ値に揃うまで表示更新を保留」——**保留中に来ない可能性**（不可視窓・別モニタ）を有界で打ち切る規定が要り、tick 構造へ介入する。要件は「機序未確定のまま先取りしない」と明記（R4 の裁定）。
   - A-3 経路 (b) を優先させる: 経路 (a) の `Changed<DPI>` 発火を「モニタ表の更新と同じ tick」へ寄せる（例: `WM_DPICHANGED` では DPI component を書かず flag だけ立て、tick 冒頭で一括反映）。`WM_DPICHANGED` 受理の意味を変えるため `dpi-window-vanish` R4 の規約と突合が要る。
2. **1 tick 内の DWM 合成境界**（サーフェス Present と窓書込の間に vblank が入る可能性）。選択肢:
   - B-1 現状維持＋観測で「tick 所要 > 1 vblank」の頻度を測る（perf 行の合計と `FrameTime` から導出可能）。
   - B-2 flush を `BeginDeferWindowPos`/`DeferWindowPos`/`EndDeferWindowPos` の 1 バッチにする（`command.rs:173-206` の局所改造）。複数窓の位置・寸を OS 側で 1 回の再配置にまとめる。Z のみの指令（`hwnd_insert_after` あり）も同 API で扱える。**Present との同時性は保証しない**（別パイプ）。研究項目。
   - B-3 サーフェスの可視化（`Present`／`set_visible`／`set_bounds`）を窓書込の直前まで遅らせる 2 相化。`apply_show` の分割（`show.rs:297-330`）＝budget の関心域に隣接、`test-cage-determinism` ④の観測点（:297-301）が動く。**最も重い**。
3. **観測**（(b) 参照）。

### (b) フレーム番号つき統合観測の建て方

- フレーム番号: `FrameCount`（wintf）を単一系列にする。areka の `emo2_frame_system`・presenter の `apply_show`（`world` を持つ）・`monitor_systems` はいずれも World から読める。flush 点だけ World 外——選択肢は §1.2 末尾の (i)〜(iii)。**(i) コマンドへ焼き込む**が最も局所的（`SetWindowPosCommand` に `frame: u32` と `origin`（要求経路の短い語）を足す＝3 つの enqueue 点の変更）。(ii) は `try_tick_world` に thread_local を 1 つ足すだけで、wndproc の同期書込にも同じ番号が使える（tick 外なら「直近 tick の番号」）。
- 「同期か flush か」: `guarded_set_window_pos` の中では区別できない。flush ループ（`command.rs:183-204`）と `WM_DPICHANGED`（`window_pos.rs:420`）の**呼出側**で別レコードを出す。
- 専用 target: `tracing` の `target:` は任意リテラルなので単一文字列（例 `"areka::transition"`）を wintf・emo-present・areka の 3 crate で共有できるが、依存方向の規律上、定数は wintf に置く（emo-present／areka から参照）か、crate ごとに専用 target を持ち `RUST_LOG` の directive を 3 語にする。要件 2.5 は「専用 target の指定でのみ有効化」なので後者でも満たせるが、手順書の 1 語化を優先するなら前者。
- レコード様式: `placement::diag` の流儀（純関数がレコード行を組み、ログはそれを出すだけ・`diag.rs:22-29`）を踏襲。フィールドは「frame・kind（monitor／surface／window）・stage（resize-buffers／upload／visualize／enqueue／flush-sync／flush-batch）・scope・window-kind・rect／size・route／origin・hwnd／entity」。値が無い経路でも番兵 `-` でフィールドを落とさない（`diag.rs:141-145` の方針）。
- 段階別計時との突合（R2.8）: perf 行に `frame` を足すのが最短（`EmitContext` に 1 フィールド追加・`show.rs:414-422`）。perf 行の判定スクリプト `judge-perf.py` はフィールド追加に対して後方互換か要確認（欠落検出は「段フィールドの欠落」なので追加は無害の見込み・要確認）。
- 「バッファ寸変更」の記録: `chain.rs:178-194` はログを持たない。`upload` の戻りに「リサイズした」を載せて `show.rs` 側で 1 行出すか、`chain.rs` で直接出す。前者なら presenter の中で `frame`／target が揃う。
- 既定 OFF の担保: `diag_tests.rs` の実濾過テストと同型のテストを新 target に対して書く。

### (c) 作業領域追随（+36px）

- 是正主体の選択肢:
  - C-1 **snapshot の更新**: wintf `Monitor` の変化（`Changed<Monitor>`／`Added`／削除）を areka の system が拾い `MonitorSnapshot::from_monitors` で作り直す。置き場は `Update`（`update_monitor_layout_system` の後）か `FrameFinalize` の `emo2_frame_system` の前。**同一 tick で dpi 相が新 snapshot を読める**ので、経路 (b) 由来の遷移では追加の書込なしに +36 が消える（既存テストの差し替えと同じ形）。経路 (a) が先行して旧 snapshot で着地した窓のために、`snapshot 変化 → 全キャラ窓を現寸で再射影`（`reproject_char_window_at_current_size` の流用・べき等 skip で変化無しは書込 0＝R5.4）を足す。更新のたびに `log_monitor_snapshot` を別 context で出せば R2.3／R5.3 を兼ねる。
  - C-2 placement が wintf の `Monitor` entity を直接読む: 消費者が多く（§1.3）、`MonitorSnapshot` の「純データ・合成注入」というテスト戦略を壊す。不利。
  - C-3 dpi 相だけ新 work area を wintf 側から引き直す: 二重の権威になり `dpi-window-vanish` D12（構築点を正典）と矛盾。不利。
- 注意点: (1) 経路 (a) と (b) の到着順で「旧下端で 1 度・新下端で 1 度」の 2 段書込になると、それ自体が Y の跳ね（36px）になる——**snapshot 更新は dpi 相より前の同一 tick に置く**か、経路 (a) の反映を tick 冒頭へ寄せる（A-3）ことで 1 度書きに保つ。(2) `Free` アンカーは動かさない契約（`anchor.rs:111-114`）。(3) 拡大率をまたぐ保存位置は追従しない（R5.7）——snapshot 更新はランタイムの判断だけを変え、保存経路（`persist.rs`）へは効かせない設計にする。(4) DD15 の「セッション内固定」doc（`work_area.rs:17`）と `main.rs:569-570` の注記を書き換える（doc 主張の file:line 裏取り規律）。

### (d) 一度きり確定のリセットが触る範囲

- `ChainFinalized` の解除: `world.remove_resource::<ChainFinalized>()` 1 行で足りるが、**いつ**が問題（§1.4）。候補: (i) snapshot 更新／DPI 遷移の**開始**で解除し、`collect_chain_states` の landing 条件を「全スコープの `DPI` が帰属モニタの DPI と一致」まで強める、(ii) 「同一 tick に `Changed<DPI>` があるあいだは見送る」を追加、(iii) 遷移の完了を観測基盤の判定（有界フレーム）と同じ語で定義して解除。
- 既定位置の追跡: 解き直しを採るなら `GhostWindows.default_char_pos` を「system 由来の再アンカー（`DpiReproject`／`Resnap`／`ReportedSizeReconcile`）では追随更新し、明示操作（`MoveCue`／ドラッグ／`Restore`）でだけ乖離させる」規則が要る。`enqueue_window_set_pos` は route を持っている（`window_move.rs:519-520`）ので、そこで既定位置を更新するのが 1 箇所で済む。ただし GhostWindows の意味（「spawn 時の既定」）が変わるため scg の 7.3 記述と突合する。
- 停滞計数: `ChainFinalizeStall` を解除時に `remove_resource` または `Default` へ戻す。
- キーワード基本位置: 素材を消費除去せず「消費済み標識」で残せば再導出可能。**offset を k 倍で済ませる**（`balloon-offset-dpi` の裁定）なら再導出は不要で、両者は排他になる可能性が高い——R6.5 の「矛盾しない形」はこの二者択一の裁定そのもの。
- テスト: 「遷移 1 回につき解き直しは一度だけ」「表情差替では解き直さない」（R6.6）は `frame_chain_finalize_tests.rs` の形で追加できる。

### (e) 決定論的検証の可否

- DPI 注入: `DPI::from_dpi(120,120)`／`(192,192)` を component に入れるだけで `Changed<DPI>` が立つ（`frame_dpi_reproject_tests.rs:299`）。複数モニタは `s2_snapshot` の合成 2 台構成が既にある。
- 多フレーム駆動: `emo2_frame_system(&mut world)` を直接呼ぶ形（`frame_visibility_integration_tests.rs` は実 GPU だが、`dpi_phase_with`＋`FakeReports` は GPU 不要）。`FrameCount` を World へ挿入して回せばフレーム番号つきレコードが決定論で出る。
- flush 点: 実 `SetWindowPos` は偽 hwnd で失敗する。選択肢: (i) キューの内容を検査する `#[cfg(test)]` の覗き口を command.rs に足す、(ii) レコードを enqueue 時に出して flush 側は「実行した」だけを出す（enqueue レコードで順序・回数を検証・flush の記録は実機サインオフの担当）、(iii) `flush` を関数注入にする（実行子を差し替え）。(ii) が最小。
- ログ捕捉: `Update` を含む wintf 側の検証は `SingleThreaded` を明示（R7.6）。areka の `emo2_frame_system` は排他 system なので呼出スレッドで走り捕捉できる。
- 状態汚染（R7.7）: `WINDOW_POS_COMMANDS` は thread_local（`command.rs:127-130`）でテストスレッドごとに独立。`FrameCount` は World 単位。`SELF_INITIATED_DEPTH` はプロセス大域 static だが RAII で戻る。thread_local ミラー（(ii) 案）を足すならテスト冒頭で初期化する口を用意する。

### (f) 縮退／全経路の判断入力（Requirement 9.1 の 3 条件を観測量へ）

| 条件 | 観測量（新観測基盤で機械判定） | 補助 |
|---|---|---|
| ⑴ 窓ごとの逐次適用が無い | 1 遷移内の全窓書込レコードの `frame` の最大−最小（0 なら同一 tick） | perf 行の `t_total_us` 合計と vblank 周期の比較（1 tick が 1 vblank を超えたか） |
| ⑵ 有界フレーム内で完了 | モニタ表更新レコードの `frame` から最終窓書込の `frame` までの差 | 経路 A の同期書込レコード件数（0 が期待値） |
| ⑶ 目視 | 開発者所見 | 中間フレーム判定（surface レコードの寸 ≠ 同フレーム末の窓寸） |

- 縮退した場合に残る実装: C-1（snapshot 更新＋再射影）・R6 の裁定・観測基盤は「最小」でも `frame`＋窓書込＋モニタ表更新の 3 種は残す（再発検知に要る）。
- 残存した場合の分割候補: (a) 観測基盤＋台帳／(b) 原子性是正／(c) 作業領域追随。(c) は (a) 無しでも静的構造で確定済みなので独立可能。(b) は (a) の結果に依存。

---

## 4. 研究項目（Research Needed・設計フェーズへ持ち越し）

1. **DWM の合成境界と 2 つのパイプ**: DXGI `Present(0)`（`chain.rs:224`）と `SetWindowPos`（flush）を同一 tick で行ったとき、DWM が 1 回の合成で両方を反映する保証があるか。WUC の暗黙コミットのタイミング（DispatcherQueue 処理点）との関係。tick 単位ログでは観測不能＝目視／画面キャプチャで補う。
2. **`DeferWindowPos` 群の適用可否**: 位置＋寸＋Z を混ぜた複数窓バッチ、`WM_WINDOWPOSCHANGED` の同期発火順、`hwnd_insert_after=None` の扱い、失敗時の部分適用の有無。
3. **`WM_DPICHANGED` の到着形**: 拡大率変更（同一モニタ）で全ゴースト窓に届くか、1 メッセージループ反復で全窓ぶん届くか、tick と交互になるか。`dpi-window-vanish` の 2 セッションで所見が割れている（S4 で「届かない」→ PMv2 設定後の 08-01 では 24 件受理）。再観測で確定。
4. **経路 (a) と (b) の同一遷移内の順序**: `WM_DPICHANGED` と `WM_DISPLAYCHANGE` のどちらが先に届くか（+36 是正の 1 度書きに直結）。
5. **`judge-perf.py` の後方互換**: perf 行へ `frame` を足しても判定が壊れないか。
6. **`FrameCount` の周回**: u32・約 828 日で周回（`world/mod.rs:493-502`）。観測の突合には差分だけを使う。

---

## 5. 実装アプローチの選択肢

### Option A: 既存コンポーネントの拡張（最小増設）
- 内容: `SetWindowPosCommand` へ `frame`／`origin` を追加し flush 側でレコード出力（wintf）／`window_move.rs` のレコードへ `frame` 追加／perf 行へ `frame`／`chain.upload` の寸変更を `show.rs` で 1 行／`monitor_systems` の更新行へ `frame`／areka に snapshot 更新 system＋再射影／`ChainFinalized` 解除の裁定実装。
- 長所: 触る場所が既知で局所。budget の予算域と重ならない（観測は確保を伴わない）。
- 短所: レコード語彙が wintf と areka に散る（2 段 grep が要る）。tick 跨ぎ分裂には効かない。

### Option B: 新規コンポーネント（観測モジュールの新設）
- 内容: wintf に `ecs::window::transition_diag`（target 定数・レコード純関数・フレーム番号ミラー）を新設し、emo-present／areka がそれを呼ぶ。areka 側は `placement::diag` に `[diag.frame_*]` レコードを追加。作業領域追随は `placement/follow/work_area.rs` に「更新」の口を、frame 側に snapshot 同期の相を新設。
- 長所: 判定語が 1 箇所に集まり、以後の spec（cage／e2e）が再利用しやすい。既定 OFF の担保テストも 1 箇所。
- 短所: wintf の公開面が増える。areka の scope 語彙は wintf に置けないので完全な 1 レコード化はできない（entity/hwnd 結合は残る）。

### Option C: ハイブリッド（段階実装）
- 段 1（要件フェーズの research）: 既存 target を点灯した再観測＋最小の `frame` 付与（Option A の観測部分だけ）で R1 を実施。
- 段 2: R1 の結果で (b) 原子性の形を決め、必要なら `DeferWindowPos` バッチ／2 相化を追加。
- 段 3: R5（C-1）と R6 の裁定実装、回帰テスト、サインオフ。
- 長所: 機序未確定のまま tick 構造へ介入しない（要件の裁定と一致）。縮退した場合は段 2 を飛ばせる。
- 短所: 段 1 の観測増設が「再観測前にコードへ触る」ことになる——ただし観測増設は R3.4 で「変更」に数えないと明記済み。

---

## 6. 工数と危険度

| 関心 | 工数 | 危険度 | 根拠 |
|---|---|---|---|
| 観測基盤（R2） | M | Medium | 3 crate に跨る・flush 点の番号配管・語彙固定テスト。既存の流儀（`placement::diag`・`timing.rs`）に倣える。 |
| 再観測＋台帳（R1・R3） | S〜M | Low | 手順書と実機は既存。判定は新レコード次第。 |
| 作業領域追随（R5） | S〜M | Medium | 実装は小さいが「1 度書き」の順序と DD15 撤回の記録が要る。 |
| 原子性是正（R4） | S（縮退）〜L（バリア／2 相化） | Medium〜High | tick 構造への介入は再観測結果に依存。`DeferWindowPos` は局所だが OS 挙動の研究が要る。 |
| 寿命裁定（R6） | M | Medium | 既定位置の追跡規則が scg の意味に触れる。 |
| 回帰テスト（R7） | M | Low | 土台あり。多フレーム駆動ハーネスと flush 覗き口が新規。 |
| サインオフ（R8） | S | Low | 手順書改訂。 |

---

## 7. 設計フェーズへの推奨と設計判断項目

**推奨**: Option C。段 1 は Option A の観測部分（`frame` 付与＋書込種別＋バッファ寸変更＋モニタ表更新）を最小で入れて再観測し、Requirement 9 の判定を出す。作業領域追随は C-1（snapshot 更新＋更新契機の再射影・dpi 相より前の同一 tick）を第一候補にする。

**設計判断項目（要件ディスカッションへ）**:

1. フレーム番号の配管方式——(i) コマンドへ焼き込み／(ii) tick が更新する thread_local ミラー／(iii) flush へ引数渡し。
2. 観測 target の置き方——単一 target 定数を wintf に置き 3 crate で共有するか、crate ごとに専用 target で directive を複数語にするか。
3. 書込レコードの一本化——「要求（areka・scope 付き）」と「実施（wintf・hwnd 付き）」を 2 段 grep で結ぶ現行流儀を踏襲するか、コマンドに要求語彙を載せて実施側 1 行で済ませるか。
4. R2.2 の記録点——`chain.rs`（バッファ寸変更の当事者）か `show.rs`（frame と target が揃う）か。`test-cage-determinism` ④の観測点（`show.rs:297-301`）を動かさないこと。
5. 作業領域源の更新主体——C-1 を採るか。採る場合の system の置き場（`Update` 後段／`FrameFinalize` の dpi 相の前）と、経路 (a) 先行時の再射影の扱い（1 度書きの保証）。DD15「セッション内固定」の撤回を要件・設計・doc の 3 箇所で揃える。
6. tick 跨ぎ分裂への構え——A-1（何もしない・観測で確認）／A-2（遷移バリア）／A-3（経路 (a) の反映を tick 冒頭へ寄せる）。再観測結果を見てから決める前提で、要件文書には「選ばない」ことを明記するかどうか。
7. 1 tick 内の DWM 境界——B-1（観測のみ）／B-2（`DeferWindowPos` バッチ）／B-3（可視化の 2 相化）。B-3 は budget 予算域と cage④ の観測点に隣接するので採るなら申し送り。
8. `ChainFinalized` の解除条件と「遷移完了」の定義（§1.4）。解除に伴う `ChainFinalizeStall` の初期化。
9. 既定位置（`default_char_pos`）の追跡規則——system 由来の再アンカーで追随更新するか（scg 7.3 の意味との整合）。
10. キーワード基本位置と `balloon-offset-dpi` の関係——offset を k 倍で済ませるか、素材を残して再導出するか（排他の見込み）。
11. 回帰テストの flush 検査口——enqueue レコードで代替するか、`#[cfg(test)]` の覗き口／実行子注入を足すか。
12. perf 行への `frame` 追加と `judge-perf.py` の互換確認を本 spec の DoD に含めるか。
13. Requirement 4.5 の「回数」の設計値——現行期待値（キャラ 1・バルーン 2・経路 A 0）を上限に採るか、C-1 導入後の追加書込（べき等 skip で 0 の見込み）を含めるか。
14. `FrameCount`（u32）の周回——観測レコードの突合と有界フレーム数の判定は差分のみで行い、絶対値比較を判定語に使わない（要件ディスカッションで追加・§4 研究項目 6）。
15. Requirement 5.8「一度の窓書込」の実現形——C-1 の system 配置（dpi 相より前の同一 tick）で足りるか、経路 (a) が `WM_DISPLAYCHANGE` より先に届く場合に A-3（経路 (a) の反映を tick 冒頭へ寄せる）が要るか（要件ディスカッションで追加・§3(c) 注意点 (1)）。
16. Requirement 6.2 の除外判定——「明示操作のみを根拠」とするための既定位置の追跡規則（項目 9 と同じ問い。要件ディスカッションで R6.2 に義務として明記済み）。

---

# 設計フェーズの追記（2026-08-15・kiro-spec-design）

- **Discovery Scope**: Extension（既存 tick 構造・観測流儀・配置機構の拡張）。研究の主眼は (i) 現行ツリーでの全アンカー再検証、(ii) 先行 5 spec の設計慣行、(iii) Win32／DWM の文書調査、(iv) 上記 §7 の設計判断項目 16 件の裁定。
- **Key Findings**:
  1. **依存方向は wintf ← areka-emo-present ← areka**（`crates/areka-emo-present/Cargo.toml:17`・`crates/areka/Cargo.toml:19,51`）。ゆえに観測 target 定数・フレーム番号ミラー・共通レコード接頭語は wintf に置いて 3 crate で共有でき、`RUST_LOG` の directive は 1 語で済む（設計判断 2 の決着）。
  2. **全窓の enqueue は既に同一 tick 内**（`emo2_frame_system` の 1 回の排他 system で全 `Changed<DPI>` を処理）。第 1 段の「逐次適用」は tick 跨ぎではなく**一括 flush の内側の実時間**（`SetWindowPos` 1 回 16〜78ms・初回最重・当該窓の最初の `SetWindowPos` の内側で `WM_DPICHANGED` を同期受理 24/24）。したがって tick 単位のフレーム差は 0 が期待値で、実時間の内訳（未特定）を名指しするには **tick 開始基準の µs・各書込の所要 µs・flush 中に受理したメッセージ**を同じレコード系列へ載せる必要がある（設計 C1／C2）。
  3. **`SetWindowPosCommand` は hwnd・矩形・flags・insert_after のみ**（`command.rs:116-125`）で entity も要求語彙も持たない。`new()` は 7 引数（呼出 3 箇所＝areka `window_move.rs:559`・zorder `:199-205`・graphics `:89-97`）。ビルダ `with_tag` で追加すれば呼出側は 1 行の変更で済む。
  4. **`MonitorSnapshot` の構築リテラルは 18 ファイル 51 箇所**（テスト含む）→ フィールド追加は不採用。DPI 表は別 Resource `MonitorDpiTable` として同じ `Monitor` 列から同時構築する。
  5. **`judge-perf.py`（`tools/perf/judge-perf.py`）は `名前=値` 辞書化＋必須フィールド存在チェックのみ**→ perf 行末尾への `frame=` 追加は互換（DoD で `--selftest`）。
  6. **`chain_finalize` の既定位置判定は「現在位置＝既定位置」**（`chain_finalize.rs:108`）。DPI 再射影の中央保存で X が動くと全スコープが除外されるため、解き直しには「システム由来の書込で、書込前に既定位置と一致していた場合のみ追随」の追跡規則が要る。
  7. **Win32 文書調査**（詳細は §8.3）: `DeferWindowPos` 群は per-window の flags／`hwndInsertAfter`・同一 parent（top-level は NULL）・`EndDeferWindowPos` が各窓へ `WM_WINDOWPOSCHANGING/CHANGED` を送る、まで文書化。**原子性・部分適用・所要**は非文書化。`WM_DPICHANGED` の送達形式（同期／post）と `WM_DISPLAYCHANGE` との順序も非文書化。HWND 矩形と swap chain `Present` を同一 DWM フレームへ揃える API は無い（Electron／Avalonia の同旨報告）。

## 8. Research Log（設計フェーズ）

### 8.1 現行ツリーでのアンカー再検証（2026-08-15）
- **Context**: brief 棚卸⑨の rebase 点と要件・研究の file:line を、設計が引用する前に全て開いて確認する（doc 主張は書く前に裏取り）。
- **Findings**（訂正のあったもの）: `show.rs` 予算域は :96-170（:95 ではない）／`resize_window_to` の limit クランプは :822（:806-808 は作業領域の取得）／`reproject_char_window_at_current_size` は `dpi.rs:335`（`window_move.rs` ではない）／`enqueue_window_set_pos` の route 引数は :520・署名 :514-521／`MonitorSnapshot` の構築は `main.rs:573-574`・挿入は :611・seam は :530-538・注記は :569／`BalloonKeywordBase` は `spawn.rs:237-243`／`ScopeWindows.default_char_pos` は `spawn.rs:266`・spawn 時設定は :514／`Stage` は `timing.rs:59-71`／`chain.upload` は `Result<(), PresentError>` でリサイズの有無を返さない／`redrive_window_dpi_for_updated_monitors` の定義は `monitor_systems.rs:438-480`（private）／`WM_DISPLAYCHANGE` は `lifecycle.rs:122-138`・戻り `None`／`FrameCount(pub u32)`（`schedule_labels.rs:8`）／`command.rs` に `#[cfg(test)]` の覗き口は無い／`capture_under_filter` は `pub(crate)`。
- **Implications**: 設計の File Structure Plan と Modified Files 表の全 file:line はこの再検証値。

### 8.2 先行 spec の設計慣行
- **Sources**: `completed/areka-P0-dpi-window-vanish/design.md`（専用 target・既定 OFF・純関数レコード・番兵 `-`・確定台帳 2 証跡クラス・File Structure Plan の突合台帳義務・サインオフ §6.2）／`completed/areka-P0-recompose-budget/design.md`（`FrameTiming`／`EmitContext`／perf 行契約・定常アロケーション 0 の判定式）／`completed/areka-P0-scope-chain-gap/design.md`（`ChainFinalized`・landing guard・`ChainFinalizeStall`・R7.3 の既定位置判定・DPI 遷移は atom へ申し送りと明記）／`completed/areka-P0-windowposition-limit/design.md`＋`tasks.md:233`（キーワード基本位置の一度きり・「拡大率をまたぐ保存追従はしない」裁定）／`completed/areka-P0-surface-resize-resnap/design.md`（`enqueue_window_set_pos` の size 契約・bypass ミラー・同一 tick 直接呼出）／`areka-P0-balloon-offset-dpi/brief.md`（offset 単位空間・atom 従属）。
- **Implications**: 設計は同じ節構成（Boundary Commitments → File Structure Plan → Traceability → Components）と、`*_with(source, world)` の依存注入・偽装境界・語彙固定テストの流儀を踏襲した。

### 8.3 Win32／DWM 文書調査（研究項目 §4-1・§4-2・§4-3・§4-4）
- **Sources**: Microsoft Learn — [DeferWindowPos](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-deferwindowpos)・[BeginDeferWindowPos](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-begindeferwindowpos)・[EndDeferWindowPos](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-enddeferwindowpos)・[Window Features](https://learn.microsoft.com/en-us/windows/win32/winmsg/window-features)・[SetWindowPos](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowpos)・[WM_DPICHANGED](https://learn.microsoft.com/en-us/windows/win32/hidpi/wm-dpichanged)・[WM_GETDPISCALEDSIZE](https://learn.microsoft.com/en-us/windows/win32/hidpi/wm-getdpiscaledsize)・[High DPI development](https://learn.microsoft.com/en-us/windows/win32/hidpi/high-dpi-desktop-application-development-on-windows)・[WM_DISPLAYCHANGE](https://learn.microsoft.com/en-us/windows/win32/gdi/wm-displaychange)・[WM_SYNCPAINT](https://learn.microsoft.com/en-us/windows/win32/gdi/wm-syncpaint)・[DirectComposition basic concepts](https://learn.microsoft.com/en-us/windows/win32/directcomp/basic-concepts)・[DwmFlush](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/nf-dwmapi-dwmflush)／[Electron: window resize behavior](https://www.electronjs.org/blog/tech-talk-window-resize-behavior)／[Avalonia #9103](https://github.com/AvaloniaUI/Avalonia/issues/9103)／ReactOS `winpos.c`（参考・保証ではない）。
- **Findings**:
  - `DeferWindowPos` は per-window に `hWndInsertAfter`・flags を取り、Z 専用（`SWP_NOMOVE|SWP_NOSIZE`）と位置＋寸を 1 バッチに混在できる。「同一 parent」制約は top-level（NULL）で満たす。`EndDeferWindowPos` は「single screen-refreshing cycle で同時更新」と記すが、メッセージ送達の一括化や部分適用の扱いは非文書化（ReactOS 実装は逐次適用・先頭失敗で打ち切り）。
  - `WM_DPICHANGED` は PMv2 で OS が窓を動かさない（アプリの責任）。送達形式（同期／post）と `WM_DISPLAYCHANGE` との順序は非文書化。`lParam` が RECT へのポインタゆえ同期送達（`SendMessage` 系）と推定。第 1 段の実測「当該窓の最初の `SetWindowPos` の内側で受理」と整合するが、断定はしない。
  - `SetWindowPos` の内側で同期に送られるメッセージ: `WM_GETMINMAXINFO`・`WM_NCCALCSIZE`（寸変化時）・`WM_WINDOWPOSCHANGING/CHANGED`・`WM_SYNCPAINT`（他スレッド）。`SWP_ASYNCWINDOWPOS` は他スレッド所有窓にだけ効く（同一スレッドでは無効）。DWM／`WS_EX_NOREDIRECTIONBITMAP` 固有の所要は非文書化。
  - HWND の移動・寸変更と swap chain `Present`／WUC commit を同一 DWM フレームへ揃える文書化 API は無い。DirectComposition は「1 commit 内の変更は 1 フレームへ」までで、HWND 幾何とは別パイプ。
- **Implications**: B-2b（`DeferWindowPos` 一括）は API 上は適用可能だが効果は測るまで不明→段階裁定。B-3（2 相化）でも同一 DWM フレームの保証は得られない→最後の手段。「窓内下端中央補償」（B-4）は同一フレーム保証に依存しない代替として候補に載せた。

### 8.4 逐次 `SetWindowPos` の内訳を名指しするための観測要件
- **Context**: 第 1 段の未特定 §8-1（1 回 16〜78ms の内訳）・§8-2（enqueue→flush の 20〜80ms）。
- **Findings**: 遷移 1 の間隔は 78／35／21／16／16ms と初回が最重、位置のみ書込（flags=21）は 2ms（+216→+218）。`WM_DPICHANGED` は各窓の最初の書込の内側で受理。ゆえに (i) 各書込の所要 µs、(ii) flush 開始の tick 開始基準 µs、(iii) flush 中に受理したメッセージ（`in_swp`・flush 開始基準 µs）、(iv) サーフェス可視化の tick 開始基準 µs、を同じフレーム番号系列に載せれば「OS 同期処理／自前 handler／flush 前の残り system」のどこに消えたかを区別できる。
- **Implications**: 設計 C1 の `Stamp { frame, t_us }`・`WriteRecord.call_us`・`FlushRecord.since_tick_us`・`MsgRecord.since_flush_us`。

## 9. Design Decisions（§7 の 16 項目の裁定）

| # | 項目 | 裁定 | 代替案と却下理由 |
|---|---|---|---|
| 1 | フレーム番号の配管 | **(ii) スレッド局所ミラー**（`try_tick_world` が `FrameCount` 増分直後に `begin_tick`） | (i) コマンド焼込みは enqueue と flush が同一 tick ゆえ冗長・wndproc 経路に届かない／(iii) 引数渡しは flush だけにしか届かない |
| 2 | 観測 target | **単一定数 `wintf::transition` を wintf に置き 3 crate で共有** | crate ごとの target は directive が 3 語になり手順書と判定が散る。依存方向 wintf←emo-present←areka で共有可能と確認 |
| 3 | 書込レコードの一本化 | **コマンドに `WriteTag { origin, scope, kind }` を載せ flush 側 1 行**。`[diag.window_move]` は不変 | 2 段 grep はサインオフの手間と判定の二重化を招く。タグは `&'static str`＋数値のみで wintf は areka の型を知らない |
| 4 | 2.2 の記録点 | **`show.rs`**（frame と target が揃う）。`chain.upload` は `UploadOutcome { resized }` を返す。:297-301 の字面不変 | `chain.rs` 直接発行は frame と target_id を持たない |
| 5 | 作業領域源の更新主体 | **C-1**（`Monitor` 表から `MonitorSnapshot`／`MonitorDpiTable` を作り直す）。置き場＝`emo2_frame_system` 先頭（dpi 相の前・同一 tick）。変化時のみ再構築・`WorkAreaResnap` で現寸再射影・DD15 撤回 | C-2（消費者が `Monitor` 直読）は偽装境界を壊す／C-3（dpi 相だけ別源）は二重権威。`Update` 後段への system 追加は排他 system 内の 1 呼出より順序が読みにくい |
| 6 | tick 跨ぎ分裂 | **A-1＋窓ごとの整合ゲート `DpiSyncHold`** | A-2（遷移バリア）は tick 構造への介入で要件の裁定に反する／A-3 は `dpi-window-vanish` の受理契約を変える上に (a) 先行時の旧 snapshot 読みを解かない |
| 7 | 1 tick 内の DWM 境界 | **B-2a 合流は確定、B-2b／B-4／B-3 は段階裁定**（採用条件を台帳の数量で固定） | 内訳未特定のまま選ぶと `dpi-window-vanish` 07-18 の偽陰性を再生産する |
| 8 | `ChainFinalized` の解除条件と遷移完了 | **解除しない。別機構 `ChainRealign`（武装＝k 変化のある `DpiReproject`・解決＝landing かつ hold なし・停滞は `ChainFinalizeStall` を武装時に初期化して再利用）** | 解除方式は起動時一度きりの意味と `finalize_chain_once_with` の一度きりガードを崩し、scg のテストへ波及する |
| 9 | 既定位置の追跡 | **キャラ窓・システム由来 route・書込前に既定位置と一致していた場合のみ追随。`None` は `None`** | 無条件追随はドラッグ後の DPI 再射影で明示配置を連鎖に取り込んでしまう。追跡なしは全スコープ除外で解き直しが空振り |
| 10 | キーワード基本位置と bod | **再導出しない。offset を k 倍（bod が実装）** | 再導出は素材保持と 2 度目の書込を要し、k 倍と ≤1px の差しか生まない |
| 11 | flush 検査口 | **`drain_window_pos_commands()`（実行せず取り出す）＋合流純関数** | enqueue レコードだけでは合流結果を検証できない／実行子注入は flush の形を変えすぎる |
| 12 | perf 行 `frame=` と互換 | **末尾追加・`judge-perf.py --selftest` を DoD に含める** | 解析規則は `名前=値` 辞書化＋必須存在チェックのみ（確認済み） |
| 13 | 4.5 の回数 | **キャラ 1・バルーン 1・経路 A 0（合流後）** | 合流前（バルーン 2）を上限にすると R6 同一 tick 解決で 3 になり得る |
| 14 | `FrameCount` 周回 | **差分のみ（`wrapping_sub`）** | — |
| 15 | 5.8 一度書き | **D5 配置で経路 (b) を保証、経路 (a) 先行は `DpiSyncHold`（上限 30 フレーム・超過は warn の上で続行）** | 上限なしは表更新が来ない異常で寸法追従が止まる／保持なしは旧下端の中間矩形が可視化される。静的証跡: zorder の Z 書込→当該窓 `WM_DPICHANGED`（SWP 内で受理・実機 24/24）→`window_pos.rs:352-363`→`dpi.rs:242-252`→`window_move.rs:288` の旧 snapshot 読み |
| 16 | 6.2 の除外判定 | 項目 9 と同一 | — |

### 設計討議での改訂（2026-08-15・design.md が正本）
- 項目 1（フレーム番号）: **World を持つ観測点は `Res<FrameCount>`＋新 Resource `TickStart` から刻印を組む。スレッド局所ミラーは World 外（flush・wndproc）専用**。`Update` の `detect_display_change_system` は多スレッド実行器上で走り得るため、ミラー統一だと `monitor` レコードの frame だけが壊れる（検証レポート Critical 1）。
- 項目 4（2.2 の記録点）: `UploadOutcome` は撤回。**upload 直前の `chain.size()` と :306 の `size` の比較で `resized` を得る**（`chain.rs` 無改変・:297-301 字面不変と両立。検証 付録 B-1）。
- 項目 5（作業領域源）: 資源差替は dpi 相の前のまま、**再射影 `WorkAreaResnap` は dpi 相の後**（経路 (b) の二重 enqueue／二重レコード回避。付録 B-2）。
- 項目 6／15（`DpiSyncHold`）: 保証範囲の裁定は設計討議 議題 1（後述・design.md「設計討議の裁定」節）。
- C7 判定量: 決定論量に加えて**実機サインオフ専用の `visualize_to_write_us`／`flush_total_us`（`Bounds::signoff`）**を追加（検証 Critical 2）。C8 の Q2／Q3 はこの量で分岐する。
- 帰属規則の共有（付録 B-3）・証跡クラスの分離表記（B-4）・`drain_window_pos_commands` の `#[doc(hidden)]`（B-5）・C8 表の対テスト列（B-6）を反映。

### 合成の記録（design-synthesis）
- **一般化**: 「窓書込・メッセージ・モニタ表・サーフェス・配置判断」を**単一の観測チャネル**（共通刻印 `frame`＋`t_us`・`kind` 判別）へ一般化し、判定は 1 つの純関数へ集約（決定論テストと実機サインオフで同一実装）。
- **Build vs Adopt**: tracing の target 濾過（adopt）／`DeferWindowPos`（adopt 候補・段階裁定）／DWM 同一フレーム同期は文書化 API が無く build もしない（B-4 は同期に依存しない補償）。Python の判定スクリプトは作らず Rust 純関数＋`#[ignore]` ランナーで一本化。
- **簡素化**: 遷移バリア・2 相コミット・`ChainFinalized` の解除機構・`MonitorSnapshot` の型変更（51 箇所）・コマンドへの frame 焼込み・enqueue／flush の 2 段 grep を採らない。

## 10. Risks & Mitigations（設計フェーズ）
- 合流が実時間を減らさない → 回数の下限化と R6 同一 tick 解決に要るため採用維持。時間は R3 で測り、B-2b 以降で扱う。
- 整合ゲートが縁の配置で最大 30 フレーム待つ → warn で可視化。表に既在の dpi へ移す別モニタドラッグは即通過（10.7）。
- 不可視のキャラ窓が旧幅で landing を通る → scg R7.4 と同じ受容。emo2 実機では発生しない。
- 観測行の追加が定常アロケーションを増やす → 表示成立点の記録は寸変化時のみ・`tracing::enabled!` で守る・perf 行は末尾追加のみ。既存 `presenter_budget_steady_state_tests` を維持。
- レコード語彙の散逸 → `kind`／`stage`／フィールド名は wintf・emo-present・areka の各レコード純関数の `pub const` に一元化し、C7 が同じ定数で解析。
