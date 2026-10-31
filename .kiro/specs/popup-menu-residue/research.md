# ギャップ分析: popup-menu-residue

- 調べた日: 2026-10-31（ブランチ `claude/kiro-start-popup-menu-residue-c40ea1`・main `da020c10` の上の `e529a6c3`）
- 入力: `requirements.md`（要件 1〜10）・`brief.md`・steering の `logging.md`（記録の型の決まり。`product.md`・`tech.md`・`structure.md` は今回読み直していない）・`roadmap.md` のウェーブ C14 の ⑦ の行
- C14 の約束: 件 11・12（実機でメニューを開いて撮る）と件 13（SSP との見比べ）はこのウェーブでは取らない。触ってよいのは `crates/areka/src/menu/{trigger,win32,captions}.rs`・`menu/trigger_show_tests.rs`・`emo2_boot/readme_cue.rs`・kanade の `actor.rs`（文だけ）・`actor_resources.rs`。

## 1. まとめ

- 済みとされた 4 件（件 1・2・7・10）は、今の main で 4 件とも直っていて、決定論テストもある（件 7 はコメントだけ）。要件 2.5 の「直っていなければ戻す」は起きない。
- 直す 6 件（件 3〜6・8・9）は、どれも今ある形（ログの捕まえ方・偽の SHIORI・偽の OS・メニューの表示の関数を引数で受ける形）の延長で書ける。新しい依存も新しい仕組みも要らない。
- 一番大きな見落としは件 9: 網羅台帳の `owner` は、`doc/ukadoc-coverage/roadmap-draft.md` の `[[spec]]` の名前か `briefing.md` の `[[owner_completed]]` の名前でなければならない（`crates/ukadoc-survey/tests/consistency/spec_checks.rs` の腕 f）。`extra-character-windows` はどちらにも無いので、要件 8.1 どおりに `owner` を書き換えると `ukadoc-survey` のテストが赤になる。直すには `roadmap-draft.md`（C14 の触るファイルに無い）と `owner_count` の手書きの数を触る必要がある。
- 件 4 は、表示の関数の戻り値が「組み立ての失敗」と「表示の失敗」を区別していないのが根。戻り値の型で分けるのが一番小さい。
- 件 3 は、`boot_runtime` を偽の SHIORI で起こす既存の通しのテスト（`emo2_boot/script_security_e2e_tests.rs`）が型紙になる。ただし開く処理の入口（`opener::submit`）はゴーストの席（`GhostSlot`）を要るので、そこをどう据えるかが設計の論点。

## 2. 済みの 4 件の確かめ（要件 2）

| 件 | 今の main での場所 | 確かめるテスト | 直した spec |
|---|---|---|---|
| 1 | `crates/areka/src/menu/trigger.rs` 353〜366 行: 預かりの窓番号（`d.scope`）が要求の窓番号と違えば `decide` の前に捨て、`trace!`（`event = "menu_deferred_double_click_scope_mismatch"`・`request_scope`・`deferred_scope`）を 1 行残す | `menu/trigger_flow_tests.rs` 677 行 `a_suppressed_tick_drops_a_deferred_double_click_of_another_window`（送らない・預かりが残らない・TRACE がちょうど 1 行で両方の番号を持つ） | `areka-P0-wintf-drag-state-rest-contract`（#172・2026-09-24） |
| 2 | `crates/areka/src/readme.rs` 163〜170 行: `open_from_world` はファイルが無ければ `opener::submit` へ渡さず `warn!`（`event = "readme_open_skipped_missing"`）を 1 行残して戻る | `readme_tests.rs` 350 行 `open_from_world_skips_a_missing_file_with_a_warning_and_no_submit`（渡さない・1 行・`level=WARN`・パスを含む） | 同上（#172） |
| 7 | `crates/areka/src/emo2_boot/spine.rs` 752 行のコメントは「現在は 13 本＝`runtime_boot.rs` の `runtime_sinks`」。`runtime_boot.rs` 106〜128 行の `vec![…]` の `Box::new` を数えると 13 本で合っている | テストなし（コメントだけ・要件 2.3 も記録だけを求める） | — |
| 10 | 開く処理は専用のスレッド `open-external`（`crates/areka/src/readme/opener.rs` 148〜161 行 `Opener::spawn`）で `ShellExecuteExW` を呼ぶ。UI スレッドの入口 `opener::submit`（同 174 行）は送るだけ。メニューの「説明書」（`menu/mod.rs` 350〜352 行 `open_readme`）と台本の `\![open,readme]`（`readme.rs` 202〜209 行 `drain_readme_requests`）の両方が `open_from_world` → `submit` を通る | `readme_tests.rs` 327 行（絶対パスを開く処理へ渡す）・`readme/opener_*_tests.rs`・`readme_tests.rs` 383 行（`ShellExecute` を綴るのは OS の境界のファイルだけ） | `areka-P0-open-external-tags`（#251・2026-10-06） |

補足（件 10）: UI スレッドに残る OS の呼び出しは `wiring.path.exists()` と `std::path::absolute` の 2 つ（ファイルの有無を見るだけ）。描画を止める重さではないが、記録には「OS の起動は専用のスレッド・UI スレッドに残るのはファイルの有無の確認だけ」と書くと正確になる。

## 3. 直す件の今の場所と足りないもの

### 件 3: 台本の入口から説明書を開くまでの通しのテスト（要件 3）

- 今あるもの（部品ごと）: `emo2_boot/readme_cue_tests.rs`（受け口が `ReadmeRequest::Readme` を 1 件送る）・`readme_tests.rs` 261 行（取り出しが届いた順に渡す）・327 行（絶対パスを渡す）・`readme/opener_execute_tests.rs`（偽の OS `FakeOs` に呼び出しが届く）。
- 足りないもの: 台本の文字列 → 解析 → 汎用キャリア → 本番の受け口の並び（`runtime_sinks` の 6 本目 `ReadmeCueSink`）→ `drain_readme_requests` → `open_from_world` → `submit` → 開く処理 → 偽の OS、を 1 本で通すテスト。
- 型紙: `emo2_boot/script_security_e2e_tests.rs`。emo2 の検体（`acquire_emo2()`・使い捨ての複製）と偽の SHIORI（`standard_script(ON_BOOT)`）で `boot_runtime` を起こし、`OnBoot` の台本を dispatcher へ Tick を入れて進める。`boot_runtime` は `wire_readme` まで済ませる（`runtime_boot.rs` 220〜222 行）。
- 論点（設計で決める）:
  1. `opener::submit` は `GhostSlot` が無いと `error!`（`no_ghost`）で捨てる。`boot_runtime` は `GhostSlot` を据えない。`GhostSession::for_test` を据えるか、`ghost_session::boot_ghost`（席まで据える本番の道・`ghost_session_reload_tests.rs` などが使う）から起こすか。
  2. 偽の OS へ届けるには、`Opener::from_sender` で受信端を取り、テストのスレッドで `opener::serve(rx, &mut FakeOs)`（または `execute`）を回すのが決定論的。テストのビルドの `Opener::spawn` は `RefusingOs` を使い、受けた呼び出しをプロセス共有の表（`opener_test_support.rs` の `REFUSED`）に積むので、並走するテストと混ざりうる。
  3. 「ファイルが無い」の場合は、使い捨ての複製から `readme.txt` を消せばよい（emo2.nar の根に `readme.txt` が在ることを確かめた）。
  4. 要件 3.2 の「`warn` をちょうど 1 行」は、起動の全体で数えると他の警告が混ざりうる。`[readme]` の行（または `event=readme_open_skipped_missing`）に絞って数える形にするか、要件の読み方を決める。

### 件 4: メニューの組み立ての失敗の文（要件 4）

- 場所: `menu/win32.rs` 28〜29 行 `show` が `build(&plan.entries)?` で組み立ての失敗（`CreatePopupMenu` 92 行・`AppendMenuW` 132 行）も `TrackPopupMenuEx` の失敗（66 行）も同じ `Err` で返す。`menu/trigger.rs` 397 行 `type ShowResult = windows::core::Result<Option<u32>>`、481〜490 行 `finish` が `Err` を全部 `event = "menu_display_failed"`・`"[menu] TrackPopupMenuEx failed"` で記録する（brief の「487 行」と一致）。
- テスト: `menu/trigger_show_tests.rs` 256〜272 行（263 行に文）と 427〜445 行（439 行に文）。偽の表示の関数は 153 行 `access_denied()` と 161 行 `exits()` の表。
- 足りないもの: 戻り値に「どの段で失敗したか」が無い。
- 組み立ての失敗を本物の OS で決定論的に起こす手は見当たらない（`build` は自分でメニューを作るので無効なハンドルを渡せない）。テストは今の形どおり、表示の関数を差し替えて失敗の種類を返させるのが現実的。

### 件 5: 説明書の受け口の記録の型（要件 5）

- 場所: `emo2_boot/readme_cue.rs` の「`ReadmeCueSink:`」7 か所＝78・83・87・101・114・123・133 行。`event=` を持つのは 120 行の `open_external_rejected` だけ。残り 6 行は `event=` 無し。文は日本語（`readme.rs` の `[readme]` の行は英語）。
- 隣の受け口の型: 新しい受け口（`change_cue.rs`・`install_cue.rs`・`reload_cue.rs`・`switch_cue.rs`・`update_cue.rs`）は角括弧の接頭辞＋`event=`。ただし接頭辞は `[ChangeCueSink]` のように受け口の型の名前。古い受け口（`move_cue.rs`・`user_break_cue.rs`・`window_state_cue.rs`・`zorder_cue.rs`）は今の `ReadmeCueSink:` と同じ「名前＋コロン」。
- `readme_cue_tests.rs` は水準（`level=WARN`）と `open_external_rejected` を見ていて、`ReadmeCueSink:` の綴りには頼っていない（書き換えで赤にならない）。
- kanade の側（要件 5.4・直さない）: `actor_resources.rs` は `target: "areka_kanade::resource"` と `event=` で揃い、角括弧の接頭辞は持たない。kanade の他の記録も `target: "kanade"`＋`event=` の形。steering `logging.md` 69 行「同一モジュール内で両形式を混在させるより、ファイル単位で一貫させる」とも合う。

### 件 6: SHIORI との往復の失敗の文（要件 6）

- 場所: `crates/areka-kanade/src/actor.rs` の `round_trip`（741 行〜）。750 行 `shiori_send_failed`・760 行 `shiori_reply_dropped`・769 行 `shiori_reply_timeout` の文が「——終了系列（Fault）へ」で終わる（brief の行番号と一致）。
- この関数は会話・終了・照会（`actor_resources.rs` 87 行が `round_trip_request` 経由で呼ぶ）・翻訳（`round_trip_raw`）の全部が通る。終了へ進むかは呼んだ側が決める（照会は `Failed` を `ResourceOutcome::Failed` にして返すだけ）。
- 同じ誤りが説明（doc コメント）にもある: 739〜740 行「再投入された Failed は状態機械を Unloading{Fault} へ倒す」は会話の道にだけ当たる。要件は記録の文だけだが、同じ関数の説明なので一緒に直すかを設計で決める。
- 触らない所: `schedule/mod.rs` 171 行（`shiori_down`）・550 行（`shiori_failed`）の「終了系列（Fault）へ」は状態機械の中で実際に終了へ進むので正しい。
- テスト: `actor_tests.rs` 327〜378 行が `event` と水準だけを見ている（文は見ない）。照会の道で文を見るテストは無い。`actor_resources_tests.rs` に、`Steady` の状態で受信端を落とした SHIORI の口へ `answer` を呼び、`error!` の文に「終了」が無いことを見る 1 本を足せる（`actor_tests.rs` 327 行と同じ作り）。

### 件 8: 相方の窓からの終了を kanade の端から端まで（要件 7）

- 今あるもの:
  - areka の側: `menu/mod.rs` 377 行が `CloseReason::User { scope: ctx.scope }` を送る。`menu/trigger_wired_tests.rs` 144 行が、スコープ 1 の窓のメニューから `KanadeMsg::CloseRequest { reason: User { scope: 1 } }` が出ることを確かめている。
  - kanade の単体: `schedule/events.rs` 482〜491 行 `on_close` が Ref1・Ref2 にスコープ番号を載せる。`schedule/events_tests.rs` 148 行がそれを見る。
  - kanade の通し: `tests/kanade/close_test_handshake_tests.rs` は `CloseRequest` を入口に入れて偽の SHIORI の記録列に `OnClose` が現れるのを見るが、**スコープ 0 だけ**（51・177・333・424 行）。しかも期待値を `events::on_close` から導いている（`close_test_test_support.rs`）ので、`on_close` 自身が壊れても気付かない。
- 足りないもの: スコープ 1（と 0）で、記録列の `OnClose` の References が書き下した `["user","1","1"]`（`["user","0","0"]`）であることを見る通しのテスト。置き場は `tests/kanade/close_test_*` の兄弟（要件 10.3 の「今在るテストの束」に当たる）。

### 件 9: 語彙のみの 4 つの名前（要件 8）

- 台帳: `doc/ukadoc-coverage/ledger/shiori.toml` 12468 行（`char*.popupmenu.type`）・12493 行（`char*.popupmenu.visible`）・13359 行（`kero.popupmenu.type`）・14434 行（`sakura.popupmenu.type`）。4 行とも `status = "vocabulary-only"`・`owner = "areka-P0-popup-menu-minimal"`・備考の引受先は「先に起こした側が本欄を引き取る」のまま。
- 表: `menu/captions.rs` 75〜92 行（`UNQUERIED_POPUPMENU_RESOURCES` は 87 行）。説明は「α に無い」「正典が定めていない」の 2 点で、引受先は書いていない。`captions_tests.rs` 172〜195 行が表の 4 名と、運行の許可表に入っていないことを確かめている（要件 8.6 の後ろ半分は既に檻がある）。
- 正典: ukadoc の `sakura.popupmenu.type` は「本体側のオーナードローメニューの変更。0で本体側メニュー、1で省略メニュー。」だけで、省略メニューの中身は書いていない（<https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#sakura.popupmenu.type>）。要件 8.2 の理由と合う。ただし「オーナードローメニューの」とある点は、`popup-menu-shell-look` がオーナードローを作るなら見直す条件の候補になる。
- **台帳の検査（見落とし）**: `crates/ukadoc-survey/tests/consistency/spec_checks.rs` の判定 ⑸ は、
  - 腕 f: 台帳の空でない `owner` は `roadmap-draft.md` の `[[spec]]` の名前か `briefing.md` の `[[owner_completed]]` の名前に限る。
  - 腕 c: `[[spec]].owner_count` は台帳 4 本でその名前を持つ行の数と同じ。
  - 腕 a: `[briefs].count`（今 66・`roadmap-draft.md` 611 行）は `[[spec]]` の行数と同じ。
  - `extra-character-windows` も `popup-menu-residue` も `[[spec]]` に無い。`areka-P0-popup-menu-minimal` は 705 行で `owner_count = 10`（shiori 8・assets 1・sakura-script 1）。
  - したがって要件 8.1 のとおり `owner` を `extra-character-windows` にすると腕 f が赤。`[[spec]]` の行を足せば腕 a・c の手書きの数（`[briefs].count`・`owner_count`）も動かす。`roadmap-draft.md` は C14 の触るファイルに無く、他のセッションと同じ手書きの数を触る（両方の枝が同じ誤った数を書くと git は黙って通す）。
  - 先例: `assets.toml` 2699 行の備考は「担当 spec は extra-character-windows…宛先の欄は空にした。…表へ足すのは棚卸の仕事である」として、`owner` を空にし備考に引受先を書いている。
  - 触る crate が `ukadoc-survey` に広がる（要件 10.4 は `areka`・`areka-kanade` だけを挙げる）。
- 他にも: `captions.rs` 120 行 `visible_resource_for` の `debug_assert!(scope <= 1, …)` は、3 人目以降の窓が出たら引っかかる。これは `extra-character-windows` の brief（14 行）が自分の仕事として挙げている。

### 件 11・12・13 の渡し先（要件 9）

- 件 11・12 → `popup-menu-shell-look`: 同 brief 61 行の Adjacent と 113 行の「どちらを先に直すかは未決」が件 11・12 を名指ししている。写し先として成り立つ。件 12 の `menu_dismissed`（`trigger.rs` 492 行・`debug!`）は今も DEBUG のまま（要件 9.4 でコードは触らない）。
- 件 13 → 写し先が決まっていない: `popup-menu-shell-look` の brief 40 行の Out は「メニューの項目そのものの増減（`popup-menu-residue`・各 spec）」と書き、項目を足す仕事を本 spec へ向けている。つまり今の生きている spec で件 13 ⑵（項目を足す）を受ける所は無い。件 13 ⑴（古い名前 `updatebuttoncaption` も問う）は `crates/areka-kanade/src/schedule/resources.rs` 49 行の問う名前の表を触るので、要件 9.4 で本 spec では触れない。
- メニューに関わる他の生きている spec: `menu-thumbnail-preview`・`tasktray-*`・`audit-areka-input-menu`（`menu*` の全ファイルを受け持つ見直しの spec・同じウェーブには居ない）。どれも項目の増減を範囲に持っていない。

## 4. 要件と今ある物の対応

| 要件 | 今ある物 | 足りない物 | 種類 |
|---|---|---|---|
| 1 仕分けの記録 | 要件の表 | 本 spec の文書に件ごとの記録（tasks.md か別の記録） | 足りない |
| 2 済みの 4 件 | 上の 2 節 | 記録に写すだけ | 無し |
| 3 件 3 | 部品ごとのテスト・`script_security_e2e_tests.rs` の型紙・`FakeOs` | 通しのテスト 1〜2 本・`GhostSlot` の据え方 | 足りない／要調査 |
| 4 件 4 | `run_show` が表示の関数を引数で受ける形・`exits()` の表 | 失敗の段を区別する戻り値・新しい記録の文・テスト | 足りない |
| 5 件 5 | `[readme]` の型（`readme.rs`）・新しい受け口の型 | 6 行の `event=` と接頭辞 | 足りない |
| 6 件 6 | `actor_tests.rs` の往復の失敗のテスト | 3 行の文・照会の道の文のテスト | 足りない |
| 7 件 8 | 両端の単体のテスト・スコープ 0 の通し | スコープ 1 の通し・書き下した期待値 | 足りない |
| 8 件 9 | 台帳 4 行・表・許可表の檻 | 備考の書き換え・`owner` の扱い・`roadmap-draft.md` の検査 | 制約（検査） |
| 9 件 11〜13 | 写し先の brief（件 11・12） | 件 13 の写し先 | 未決 |
| 10 動きを変えない | 既存のメニューと説明書のテスト | — | 制約 |

## 5. やり方の案

### 件 4（失敗の段の区別）

- **案 A（戻り値の型で分ける）**: `win32::show` の失敗を `Build(Error)` と `Track(Error)` の 2 つに分けた型で返す（`build(...).map_err(...)?` と最後の `Err` の 2 か所）。`trigger.rs` の `ShowResult` と `finish` の `Err` の腕を 2 つにし、組み立ての失敗は新しい文（例: `[menu] building the menu failed`）で記録する。テストは `exits()` の表に組み立ての失敗を 1 つ足す。
  - 良い所: 触るのは 3 ファイルだけ・表示の関数を引数で受ける形のまま・記録は呼び手が出す（`win32.rs` の冒頭の説明）を守る。
  - 悪い所: `trigger_show_tests.rs` の偽の関数の型が変わる（`access_denied` など数か所）。
- **案 B（関数を 2 つに割る）**: `win32::build` と `win32::track` を別々に公開し、`run_show` が 2 つの関数を受ける。分けは明快だが、引数と偽の関数が倍になり、差分が大きい。
- **案 C（`win32.rs` の中で記録する）**: 組み立ての失敗を `build` の中で `error!` にする。呼び手の `finish` も `Err` を記録するので 2 行になるか、`finish` 側を抑える細工が要る。記録は呼び手が出すという今の取り決めに反する。
- 決めること: 組み立ての失敗の `event=` の名前（新しく `menu_build_failed` を立てるか、`menu_display_failed` に段の欄 `stage=` を足すか。要件 4.2 は表示の失敗の名前と文を変えないことだけを求める）。どの API が失敗したか（`CreatePopupMenu`／`AppendMenuW`）を欄に載せるか。

### 件 3（通しのテスト）

- **案 A（`boot_runtime`＋席を据える）**: `script_security_e2e_tests.rs` と同じ組み方で `OnBoot` に `\0\![open,readme]\e` を返し、`GhostSlot`（`GhostSession::for_test`）と `Opener::from_sender` を World に据え、`drain_readme_requests` を回し、受けた仕事をテストのスレッドで `FakeOs` に通す。台本 → 本番の受け口の並び → 取り出し → 開く処理 → 偽の OS は全部本物。据えるのは文脈（席と送り先）だけ。
- **案 B（`boot_ghost` から起こす）**: 席の据え付けと取り出しの登録（`ghost_session::register_systems`）まで本番の道を通る。差し替えはさらに少ないが、起こす物が増えて重く、窓や GPU が要るかは要調査。
- **案 C（台本の解析から受け口を直に呼ぶ）**: 台本を解析して cue にし、`ReadmeCueSink` へ直に渡す。kanade・dispatcher・本番の受け口の並びを通らないので、要件 3.3 の「途中の段を差し替えない」には弱い。

### 件 9（台帳の持ち主）

- **案 A（要件 8.1 のとおり `owner` を書き換える）**: `char*.popupmenu.visible` の `owner` を `extra-character-windows` に、`*.popupmenu.type` 3 行を `popup-menu-residue`（本 spec の裁定）にする。`roadmap-draft.md` に `[[spec]]` を 2 行足し（`none = true`・`reason`・`owner_count`・`wave`）、`[briefs].count` を 66 → 68、`areka-P0-popup-menu-minimal` の `owner_count` を 10 → 6 にする。`ukadoc-survey` のテストを回す。C14 の触るファイルの外で、手書きの数を他の枝と取り合う。
- **案 B（`owner` は空・備考に引受先）**: 4 行の `owner` を空にし、備考に「引受先: `extra-character-windows`」「本 spec の裁定で作り分けない」を書く（`assets.toml` 2699 行の先例）。`areka-P0-popup-menu-minimal` の `owner_count` だけ 10 → 6 に直す（これも `roadmap-draft.md`・腕 c）。`[[spec]]` は足さない。要件 8.1 の「持ち主…を `extra-character-windows` にする」の文言を改める必要がある。
- **案 C（`owner` は閉じた spec のまま・備考だけ）**: `owner` を動かさず備考だけ書き換える。検査は全部緑のまま・`roadmap-draft.md` も触らない。要件 8 の Objective（閉じた spec でなく生きた spec か裁定を指す）は備考でだけ満たす。要件 8.1 の文言を改める必要がある。

## 6. 大きさと危うさ

- 大きさ: S（タスク 4〜6・brief と roadmap の見立てどおり）。件 9 で案 A を取ると `roadmap-draft.md` と `ukadoc-survey` の確かめが 1 つ増える。
- 危うさ: 低い。今ある形の延長で、利用者に見える動きは変えない。中くらいに上がりうるのは 2 点＝件 3 の通しのテストの組み方（席と開く処理の据え方・起動の重さ）と、件 9 の台帳の検査（他の枝と手書きの数を取り合う）。

## 7. 設計へ持ち越す調べもの

1. 件 3: `boot_runtime` の後に `GhostSlot` を据えて `opener::submit` が通るか。`boot_ghost` を使う場合に窓・GPU・COM が要るか。台本の時計を進めて受け口へ届くまでの待ち方（`wait_until` の上限）。
2. 件 3: テストのビルドの `Opener::spawn`（`RefusingOs`・プロセス共有の表）を避け、`Opener::from_sender`＋テストのスレッドの `serve` で決定論にできるか。
3. 件 4: `AppendMenuW`／`CreatePopupMenu` を本物の OS で決定論的に失敗させる手が無いことの確認（無ければ、表示の関数の差し替えでテストする）。
4. 件 9: 案 A を取る場合、同じウェーブの兄弟が `roadmap-draft.md` の `[briefs].count`・`owner_count` を触るか（触るなら着地の順に合わせて数え直す）。
5. 件 8: `close_test_handshake_tests.rs` の偽の SHIORI と記録列で、書き下した References を比べる形（`expected_call` を使わない形）にできるか。
6. 件 4: 組み立ての失敗の `event=` の名前（新しく立てるか・`stage=` の欄を足すか）と、失敗した API の名前を欄に載せるか（要件の討議で設計へ回した）。
7. 件 5: 6 行に付ける `event=` の名前と、文を日本語のままにするか `readme.rs` に合わせて英語にするか（検索は接頭辞と `event=` で引くので、文の言葉は要件に効かない・要件の討議で設計へ回した）。

## 8. 要件の討議へ渡す論点

1. **件 9 の `owner` の書き方**（上の 5 節の案 A／B／C）。要件 8.1 は `owner` の書き換えを求めるが、そのままでは `ukadoc-survey` の検査が赤になり、C14 の触るファイルの外（`roadmap-draft.md`）と手書きの数を触る。
2. **件 9 のうち `char*.popupmenu.visible` を本当に `extra-character-windows` へ渡せるか**。同 brief の 84 行の議題 3 は「本 spec で引き取るか」がまだ開いていて、77 行「20 を超えそうならメニューの n≧2 を `popup-menu-residue` へ戻す」とも書く。要件 1.3 の「渡し先が生きている spec」は満たすが、向こうがまだ受けると決めていない。向こうのセッションへ先に相談するか（「他のセッションの持ち場の事実で詰まったら、そのセッションへ相談」）。
3. **件 9 の `*.popupmenu.type` の「作り分けない」裁定の見直す条件**。正典の文は「オーナードローメニューの変更」と書く。`popup-menu-shell-look` がオーナードローのメニューを作ると決めたら見直す、を条件に加えるか。
4. **件 13 の写し先**（要件 9.2 は spec を名指ししていない）。`popup-menu-shell-look` の Out は項目の増減を本 spec へ向けているので、今の生きている spec に受け手が居ない。候補: ⑴ `/kiro-discovery` で新しい spec を起こす（例: 古い名前を問う ⑴ と項目を足す ⑵ をまとめた 1 本、あるいは ⑴ と ⑵ を分ける） ⑵ `popup-menu-shell-look` へ写して向こうの Out を広げてもらう ⑶ ⑴ だけ別の spec（問う名前の表 `schedule/resources.rs` を触るもの）へ、⑵ は新しい spec へ。件 15（おすすめの資源）との分け方が未決であることも一緒に写す（要件 9.2）。
5. **件 4 の組み立ての失敗の記録の名前**（新しい `event=` を立てるか・段の欄を足すか）と、どの API が失敗したかを記録に載せるか。
6. **件 5 の接頭辞と文の言葉**。要件 5.1 は `[readme]` と決めている（`readme.rs` と同じ）。新しい受け口は `[ChangeCueSink]` のような型の名前の接頭辞を使うので、受け口の間では揃わなくなる。文を日本語のままにするか、`readme.rs` に合わせて英語にするか。6 行に付ける `event=` の名前。
7. **件 6 の説明（doc コメント）も直すか**。`actor.rs` 739〜740 行の「Unloading{Fault} へ倒す」は記録の文と同じ誤りを持つ。要件は「文だけ」と書くが、記録の文か説明の文かは書いていない。
8. **件 3 の「`warn` をちょうど 1 行」の数え方**（起動の全体か・説明書の道の行だけか）と、`GhostSlot` を据えることが「途中の段を差し替えない」に反しないか。
9. **件 8 の期待値**。今の通しのテストは `events::on_close` から期待値を作るので、`on_close` 自身の誤りに気付かない。新しいテストは References を書き下すか。
10. **触る crate が広がる場合の要件 10.4**。件 9 で `roadmap-draft.md` を触るなら `ukadoc-survey` のテストも通すことを要件に足すか。

## 9. 次の段

- 要件の討議（`/kiro-requirements-discussion popup-menu-residue`）で 8 節の論点を決め、要件 8・9 の文言を確定させる。
- その後 `/kiro-design popup-menu-residue`。

## 10. 要件の討議の結論（2026-10-31・全議題を推奨案で承認）

- 論点 1（件 9 の `owner`）: 案 B＝4 行の `owner` を空にし備考に引受先・`roadmap-draft.md` は `areka-P0-popup-menu-minimal` の `owner_count` の 1 行だけ直す・要件 10.4 に `ukadoc-survey` を足した（要件 8.1・8.4・10.4）。
- 論点 2（`char*.popupmenu.visible`）: `extra-character-windows` へ渡す。引き取らないと決めたらそのときに新しく起票（要件 8.2）。
- 論点 3（`*.popupmenu.type` の見直す条件）: `popup-menu-shell-look` がオーナードローのメニューを入れたときを足し、向こうの brief へも写す（要件 8.3・8.7）。
- 論点 4（件 13）: 本 spec の完了時の棚卸で `/kiro-discovery` により新しい spec 1 本（⑴⑵ まとめ）として起票（要件 9.2）。
- 論点 5〜7 は設計へ（7 節 6・7）・論点 8〜10 は要件 3.2・7.3・10.4 へ反映済み。
