# 調べもの: areka-P0-window-link-directives

## 1. ギャップ分析（2026-10-22・基準 `13924489`）

要件（`requirements.md`・未承認）と今のコードの差を、コードを読んで確かめた。要件の作者が挙げた手がかり（吸着の受け手が本番で動いていない・シェルの切り替えで吸着が上書きされる・作業領域の変化で当て直すのは下だけ・キーボードの近道が無い・重なりの台本の組がシェルの切り替えで捨てられる）は、どれもコードで裏が取れた。加えて、要件に効く点が 5 つ見つかった（1.2 の ★ 印）。

### 1.1 要約

- **吸着（要件 1・2）は部品がほぼ揃っている。** 窓ごとの吸着の向き `Anchored`・辺へ沿わせる計算 `project_anchor`・吸着の向きでのドラッグの縛り・`\![move]` の吸着への従い・大きさが変わったときの当て直しは、どれも本番で動いている。足りないのは台本の指令の受け口と、「台本で変えた」ことの印（シェルの切り替え・`default` のため）だけ。
- **窓の組（要件 3〜5）は新しく作る。** 型の手本は重なりの組の帳簿 `ZOrderGroupLedger`（descript の組は 1 つまで・台本の組は複数・重なった ID の拒否・拒否を値で返す）だが、シェルの切り替えでの扱いが逆（重なりは台本の組も捨てる）で、窓ではなくスコープの単位なので、そのまま使い回すことはできない。連れ動きを差し込む所はドラッグの 2 つの受け手と `apply_move_directive` の 3 か所で、`resetwindowpos`・当て直し・大きさの変化には差し込まないので、要件 4.7・5.6 は差し込む場所の選び方だけで満たせる。
- **バルーン位置初期化（要件 7）は計算の材料がそろっていない。** 標準の位置は配置の解決で出せるが、並べ直しの材料 `DefaultPlacementBasis` は、起動のときとシェルの切り替えの後で中身が違う（完了 `move-canon` の申し送りどおり）。そのため要件 7.4 を満たすには先に材料をそろえる必要がある。記憶から消す仕組みは無く、書くことしかできない（暫定の裁定 8 の「書く」は今の仕組みのまま実現できる）。
- **キーボードの近道（要件 8）は土台が無い。** wintf はキーの押下を Esc でのドラッグ取り消しにしか使っておらず、areka へ知らせる口が無い。`WM_SYSKEYDOWN` は受けていない。F10 の扱い・入力先の当たり方は調べが要る。
- **規模は L（1〜2 週間）・危険度は中。** brief の見立て 10〜14 タスクに、キーボードの土台（wintf）と材料の統一で 2〜4 タスク増える見込み。それでも 20 には届かない。

### 1.2 今あるもの（調べた結果）

#### 吸着

| もの | 場所 | 今の状態 |
|---|---|---|
| 吸着の向き（5 値: 上・下・左・右・なし） | `placement/resolver.rs:415` `Anchor` | descript の値からの写し `Anchor::from_alignment`（`:438`）。読めない値は下（`warn!`）。 |
| 窓ごとの向き | `placement/follow/anchor.rs:181` `Anchored` | 作るときに付ける（`placement/spawn.rs:488`）。ドラッグ・大きさの変化・`\![move]`・記憶の読み戻しが読む。 |
| 辺へ沿わせる計算 | `anchor.rs` `project_anchor` | 上・下・左・右・なしの全部が実装済み・純関数。 |
| ドラッグの縛り | `placement/follow/drag_follow.rs` `on_char_drag`／`on_char_drag_end` | 吸着している窓は wintf に動かさせず（`DragConfig.move_window=false`）、areka が `project_anchor` を通した位置を書く。なしの窓は wintf が動かす。**要件 2.1〜2.3 は今すでに満たしている。** |
| ドラッグの縛りの切り替え | `DragConfig.move_window` | wintf はドラッグを始めたときにこの値を控える（`wintf/src/ecs/drag/state/mod.rs:89`）ので、途中で変えても次のドラッグから効く＝要件 2.4 の形に合う。 |
| `\![move]` の吸着への従い | `emo2_boot/move_cue_place.rs:147-167` | 動かす窓の `Anchored` で射影してから書く。向きを変えれば自動で従う（要件 1.5 の `\![move]` 分は手が要らない）。 |
| 向きが変わったときの当て直し | `placement/follow/window_move.rs:445` `anchor_changed_system` | **定義だけで本番の予定表に載っていない**（`#[allow(dead_code)]`・テストだけが呼ぶ）。 |
| ★ 同じ働きの本番の道 | `placement/reseed.rs:177` `reanchor_char_windows` | シェルの切り替えの後に、今の大きさのまま `resize_window_to(…, PlacementRoute::AnchorChange)` を呼んで当て直している。つまり経路の語 `AnchorChange` は**本番で使われている**。`window_move.rs:533` の注記「本番では未結線…現に到達しない」は古い。 |
| 作業領域の変化での当て直し | `emo2_boot/frame/work_area_sync.rs:207` | 下の吸着だけ（要件の範囲外の行と合う）。 |

#### 台本の指令の受け口

- 窓の位置の受け口 `MoveCueSink`（`emo2_boot/move_cue.rs`）が `move` と `(execute, resetwindowpos)` を名前で選び、`PositionCue`（今は `Move`・`ResetWindowPos` の 2 種）を 1 本の線で画面の側へ送る。画面の側は毎フレーム `run_move_drain_phase`（`emo2_boot/frame.rs:376`）が届いた順に適用する。
- 重なりの受け口 `ZOrderCueSink`（`emo2_boot/zorder_cue.rs`）は `(set, zorder)`・`(reset, zorder)` を選び、別の線で送る。実行したスコープ（`cue.actor`）を読まない。開けない荷物の宛名が `set`／`reset` なら自分宛として警告を出す決まりがあり（`zorder_cue.rs:69`）、`set`／`reset` の担当が増えたら見直すよう注記されている。
- 受け手の表 `emo2_boot/consumer_ledger.rs`（472 行）。`(set, alignmenttodesktop)`・`(set, alignmentondesktop)`・`(set, sticky-window)`・`(reset, sticky-window)`・`(execute, resetballoonpos)` は未登記。

#### 組の手本（重なりの組の帳簿）

- `placement/zorder_group_ledger.rs`（657 行）: descript の組は 1 つまで・台本の組は追加順・既に組にいるスコープを含む指令は全体を拒否・拒否は値で返して記録は呼び手・保存しない。
- 帳簿の置き場は結線の状態 `Emo2Wiring`（`emo2_boot/frame/wiring.rs:98`）。これはゴーストを起こすたびに新しく作られる。SHIORI だけの読み直し（`ghost_session_runtime.rs:85` `rebuild_runtime_in_place`）は窓と結線を残して実行系だけを組み直すので、帳簿は残る。ゴーストの読み直しと切り替えは窓を閉じて（`app_exit.rs:188` `close_windows_for_restart`）起こし直すので、帳簿は新しくなる。
- シェルの切り替えでは `apply_descript_rebase`（`emo2_boot/frame/zorder_descript.rs:97`）が**台本の組も捨てて** descript の組だけに戻す（暫定の裁定 2 と逆）。
- ★ 窓の組の帳簿を `Emo2Wiring` に置くと、ドラッグの受け手（`placement` の中・`&mut World` だけを受ける）から読めない。`placement` から `emo2_boot` へ向かう依存は今の層の向きに反する。組の帳簿は World に置く資源（`GhostWindows` と同じ扱い）にするのが自然で、そうすると要件 9.2 の「他の機能が確かめられる」（`chain_realign.rs` からの読み）もそのまま満たせる。

#### ドラッグ

- 連れ動きに要る「ドラッグを始めたときの相手の位置」を控える所が無い。キャラクターの窓にはドラッグの開始の受け手（`OnDragStart`）を付けていない（`placement/spawn.rs`）。動かした窓の開始の位置は wintf の `DraggingState.initial_inset` にある。
- なしの窓は wintf が先に動かし、`on_char_drag` は動いた後の `WindowPos.position` を読むだけ。1 回ごとの差分を「前の位置との差」で取るなら、wintf が `WindowPos` をいつ書き換えるかを確かめる必要がある（1.7 の調べ 3）。
- ドラッグの終わり（`on_char_drag_end`）で、その窓の位置を記憶へ書く。組の相手の分は書いていない（要件 4.5 の足し分）。
- ★ 記憶へ書く位置は向きで形が変わる（`placement/persist.rs:61` `char_pos_to_origin_x`: 下の吸着だけ x を窓の下端の中央へ直し、ほかは左上のまま）。読み戻すときは次の起動の descript の向きで解く。台本で向きを変えた窓をドラッグすると、書いた形と読み戻す形が食い違い、次の起動で窓の幅の半分だけ横にずれる（要件 6.1 の「次の起動は descript から」と組み合わさって起きる）。

#### 寿命

- シェルの切り替えの後始末 `finish_shell`（`emo2_boot/frame/switch.rs:432`）→ `apply_shell_descript`（`placement/reseed.rs:139-146`）が、新しいシェルの descript の向きで `Anchored` と `DragConfig.move_window` を**無条件に上書きする**。続けて `reanchor_char_windows` が当て直し、`reseed_zorder_descript_base` が重なりを descript に戻す。要件 6.4・6.5 の「台本で変えた分は保つ」には、スコープごとの「台本で変えた」印と、ここでの読み分けが要る。
- `close_windows_for_restart` は窓の一式に属する資源（`GhostWindows`・重なりの鎖ほか）を外す。組の帳簿を World に置くなら、ここに 1 行足せばゴーストの切り替え・読み直しで descript の値から始まる（要件 6.2）。窓ごとの `Anchored` は窓と一緒に消えるので、吸着の側には手が要らない。

#### 並べ直しとバルーン

- 並べ直し `placement/reset_pos.rs` `reset_window_positions` は、`DefaultPlacementBasis` と今の窓の大きさで配置を解き、キャラクターの窓だけを書く。解いた結果にはバルーンの既定のずらし（`ScopePlacement.balloon_offset_base`）も含まれるが、今は使っていない。
- `DefaultPlacementBasis` の中身: 起動では（`ghost_session.rs:240`）バルーンのずらしの拡大と `windowposition` を合わせた後の設定、シェルの切り替えでは（`switch.rs:466`）descript から組んだままの設定。バルーンの位置をここから出すと、起動の直後とシェルの切り替えの後で答えが変わる（要件 7.4 に反する）。
- シェルの切り替えの `apply_shell_descript` は、スコープごとに「その窓の DPI でずらしを拡大 → バルーンの `windowposition` を合わせる → 配置を解く → 記憶を重ねる」を通している。バルーン位置初期化に要る計算（記憶を重ねない版）はここと同じ流れになる。ただし材料（descript の値・バルーンの `windowposition` と作者の DPI）はシェルの切り替えのときに裏で読んだもので、普段は World に残っていない。
- バルーンのずらし `BalloonFollow`（`placement/follow/offset_space.rs:94`）は値を直接書けず、新しく組む（`BalloonFollow::new`）か焼き直す（`reestablish`）しかない。`apply_shell_descript` は組み直す側を使っている。
- 記憶の書き込み口は `persist_put` だけで、消す口は無い（`areka-sylphya/src/actor.rs:493`）。

#### メニュー

- 枠は `menu/mod.rs:57` の `Frame`（11 個・並びは `ORDER`）。項目名の表は `menu/captions.rs:30`、区切りの群は `menu/plan.rs:78`。「シェル位置初期化」は `menu/reset_pos_frame.rs`（139 行）が受け持ち、`OnResetWindowPos` を送って答えで並べ直すかを決める。
- 枠を 1 つ足すと、`ORDER` の長さ・項目名の表・群・`captions_tests.rs` の数（move-canon の申し送り 4.1 の「引く 9 名」）・枠の登記（`ghost_session_runtime.rs:50` の並び）に手が入る。
- ★ 項目名を引く SHIORI リソースの許可表（`areka-kanade/src/schedule/resources.rs:66`）に `resetballoonpositionbutton.caption` が無い。語彙の表（`areka-sylphya/src/vocab/shiori_resource.rs:170`）には載っている。許可表に足さないと、ゴーストが名前を返しても送られない（要件 7.6）。

#### キーボード

- wintf のキーの受け手は `wintf/src/ecs/window_proc/keyboard.rs` の `WM_KEYDOWN` だけで、Esc でドラッグを取り消すことにしか使っていない。areka へ知らせる出来事（マウスの `OnPointer…` に当たるもの）は無い。`WM_SYSKEYDOWN` は振り分け表（`window_proc/mod.rs:84`）に無い。
- メッセージの回し方はライブラリへ移してあり（`wintf/src/runtime/message_loop.rs`）、`TranslateAccelerator` を差し込む所は無い。
- areka の窓は `WS_EX_NOACTIVATE` を付けていないので、クリックすれば入力先になる見込み（確かめは 1.7 の調べ 1）。

#### 記録と台帳

- 網羅台帳 `doc/ukadoc-coverage/ledger/sakura-script.toml`: `reset,sticky-window`（2821 行）・`set,alignmentondesktop`（2924）・`set,alignmenttodesktop,方向`（2939）・`,free`（2955）・`set,sticky-window`（3273）・`execute,resetballoonpos`（1450）。`shiori.toml` の `resetballoonpositionbutton.caption`（14202）・`assets.toml` の `seriko.sticky-window`（5915）と `seriko.alignmenttodesktop` 系（2676 ほか）・`property.toml` の `currentghost.seriko.sticky-window`（949）。
- 互換の対応表 `doc/COMPAT_ARCHITECTURE.md` §8（122 行目から）。`resetwindowpos` とメニューの行（141・142 行目）がバルーン位置初期化の行の手本になる。
- 検体「窓際のぱすたさん」は `sample-ghost-kit` に `pasta-in-windows`（URL の行）として登記済み。この作業ツリーの `target\` にはまだ取り寄せていない。

### 1.3 要件ごとの対応表

印: **済**＝今のまま満たす／**足す**＝足りない（Missing）／**縛り**＝今の決まりと当たる（Constraint）／**不明**＝調べが要る（Unknown）

| 要件 | 使えるもの | 差 |
|---|---|---|
| 1.1〜1.4（向きの指令・綴り・`free`・`default`） | `Anchored`・`project_anchor`・`resize_window_to`・`reanchor_char_windows` の形 | **足す**: 受け口・指令の型・適用の相。`default` には descript の向き（`DefaultPlacementBasis` の `alignment` か、窓に控える）が要る。 |
| 1.5（以後の扱いが descript と同じ） | 大きさの変化・`\![move]` は `Anchored` を読むだけ | **済**（作業領域の変化は下だけ＝範囲外の行のとおり）。 |
| 1.6（バルーンは付いて動き、記憶に書かない） | `resize_window_to` がバルーンも動かし、記憶には書かない | **済**。ただし経路の語を何にするかで既定の位置の扱いが変わる（1.6 の議題 2）。 |
| 1.7・1.8（読めない値・窓が無い） | `move_skipped` 型の記録 | **足す**: 読めない値は「変えない」（descript の「下に倒す」と違う規則）。 |
| 2.1〜2.3 | ドラッグの縛り | **済**。 |
| 2.4（次のドラッグから） | `DragConfig.move_window` の切り替え（`reseed.rs` と同じ） | **足す**（1 行）。 |
| 3.1〜3.5（組を作る・拒否） | `ZOrderGroupLedger` の型・`parse_scope_id` の流儀 | **足す**: スコープ単位の新しい帳簿と読み手。 |
| 3.6（窓の無いスコープ） | `GhostWindows::char_window` が `None` を返す | **足す**（連れ動きの側で外して記録）。 |
| 3.7（`reset`） | `reset_to_descript` の型 | **足す**。 |
| 3.8・3.9（descript の組） | `PlacementConfig.sticky_window_raw`（`config.rs:107`）・重なりの起動での据え方 | **足す**: 起動とシェルの切り替えの 2 か所で据える。 |
| 4.1〜4.4（ドラッグの連れ動き） | `on_char_drag`・`project_anchor`・`enqueue_window_set_pos`・`follow_balloon` | **足す**: 開始の位置の控え・相手の書き込み。`drag_follow.rs` は 937 行＝中身は兄弟のファイルへ。**不明**: 差分の取り方（1.7 の調べ 3）。 |
| 4.5（記憶に書く） | `char_pos_entries`・`persist_entries` | **足す**。**縛り**: 書く形が向きで変わる（1.2「ドラッグ」の ★）。 |
| 4.6（バルーンのドラッグ） | 今の振る舞い | **済**。 |
| 4.7（当て直し・大きさの変化では動かさない） | 差し込む所がドラッグと `\![move]` だけ | **済**（差し込む場所の選び方で満たす）。 |
| 5.1〜5.4（`\![move]` の連れ動き） | `apply_move_directive`・`move_window_to` | **足す**: 実際に動いた量（書いた位置 − 前の位置）で相手を動かす。`ignore_sticky_window` を読む。 |
| 5.5（時間つき） | 今は即時に置く | **済**（相手も同じ道を通れば同じ）。 |
| 5.6（`resetwindowpos`） | `reset_window_positions` は連れ動きを通らない | **済**。 |
| 5.7 | — | **済**（組が無ければ何もしない）。 |
| 6.1・6.2（終わり・切り替えで descript から） | 窓と一緒に `Anchored` が消える・`close_windows_for_restart` | **足す**: 組の帳簿を外す 1 行（World に置く場合）。 |
| 6.3（SHIORI だけの読み直しで保つ） | `rebuild_runtime_in_place` は窓と結線を残す | **済**（どちらに置いても残る）。 |
| 6.4・6.5（シェルの切り替え） | `apply_shell_descript`・`finish_shell` | **縛り**: `reseed.rs:139-146` が無条件に上書きする。**足す**: 台本で変えた印・descript の組だけの取り替え・重なった descript の組の記録。 |
| 7.1〜7.4（バルーン位置初期化） | 配置の解決・`reseed` の流れ・`BalloonFollow::new`・`balloon_offset_entries` | **縛り**: `DefaultPlacementBasis` が起動とシェルの切り替えで違う。**足す**: 材料の統一・初期化の関数・記憶への書き込み。 |
| 7.5〜7.7（メニューの項目） | `reset_pos_frame.rs` の型・`Frame` | **足す**: 新しい枠・項目名の表・許可表に 1 名。 |
| 8.1〜8.4（近道） | wintf の `WM_KEYDOWN` の受け手 | **足す**: キーを areka へ知らせる口。**不明**: F10 の届き方・入力先の当たり方。 |
| 9.1〜9.5（記録） | `move_applied` などの `info!`・`warn!` の型・`logging.md` | **足す**。 |
| 9.6（決定論のテスト） | 帳簿・読み・射影は純関数にできる。ドラッグには `follow_test_support.rs`・wintf の `wndproc_bridge_drag_test_support.rs` がある | **足す**。 |
| 9.7・9.8（実機） | `real-run-kit`・検体 `pasta-in-windows`・排他テストの机 | **足す**（検体の取り寄せから）。 |
| 10.1〜10.5（記録の文書） | 上の行番号 | **足す**（文書だけ）。 |

### 1.4 実装の案

#### 案 A: 今の部品を広げる（受け口も帳簿も今の物に相乗り）

- 受け口は `MoveCueSink` に 5 組（`set,alignmenttodesktop`・`set,alignmentondesktop`・`set,sticky-window`・`reset,sticky-window`・`execute,resetballoonpos`）を足し、`PositionCue` に種類を足して同じ線で送る。
- 組の帳簿は `ZOrderGroupLedger` を広げて両方に使う（窓の単位とスコープの単位を切り替える）。
- ✅ 新しい線も新しい受け口も要らない。台本の中の並び（組を作ってから `\![move]`）が 1 本の線の中で保たれる。
- ❌ 重なりの帳簿は「シェルの切り替えで台本の組も捨てる」「窓の単位」「`Emo2Wiring` の中」の 3 点で窓の組と食い違う。広げると重なりの側の決まりと試験に手が入り、657 行のファイルが膨らむ。

#### 案 B: 新しい部品で作る（受け口・線・帳簿をそれぞれ新しく）

- `StickyCueSink`（新）と `AlignmentCueSink`（新）、それぞれの線と `Emo2Wiring` の受け端。帳簿は `placement/sticky_group_ledger.rs`（新）。
- ✅ 責任の境目がはっきりする。
- ❌ 線が増えると、同じ台本の `\![set,sticky-window]` と `\![move]` の届く順が画面の側で入れ替わりうる（線ごとに取り出す相が違う）。`GhostBootOptions.sinks` の数と結線の試験にも手が入る。

#### 案 C: 混ぜる（受け口は今の線へ相乗り・帳簿と連れ動きは新しいファイル）

- 受け口: `MoveCueSink` の名前選びに 5 組を足し、`PositionCue` に `SetAlignment{scope, 向き}`・`SetSticky{tokens}`・`ResetSticky`・`ResetBalloonPos` を足す（読みは兄弟のファイルへ）。受け手の表 `consumer_ledger.rs` に 5 行。
- 帳簿: `placement/sticky_group_ledger.rs`（新）。スコープの集まりの列・出どころ（台本／descript）・拒否を値で返す。World の資源にし、`close_windows_for_restart` で外す。重なりの帳簿から型と流儀を写す（共有はしない）。
- 吸着: 適用の相で `Anchored` を書き換え、`DragConfig.move_window` を切り替え、`resize_window_to` で当て直す（`reanchor_char_windows` と同じ形）。`anchor_changed_system` は結線しない。台本で変えた印は窓の部品（新）か帳簿に持つ。
- 連れ動き: `placement/follow/sticky_follow.rs`（新）に「相手の一覧を引き、量を足して、相手の向きで射影して書く」を置き、`on_char_drag`・`on_char_drag_end`・`apply_move_directive` から数行で呼ぶ。
- バルーン位置初期化: `reseed.rs` の「スコープごとに拡大して合わせて解く」を共有の関数へ出し、シェルの切り替えと初期化の両方が使う。材料は `DefaultPlacementBasis` を「descript の値＋バルーンの材料」に改めてそろえる。メニューは `menu/reset_balloon_pos_frame.rs`（新）。
- 近道: wintf に「キーの押下を areka へ知らせる口」を足し、areka の側で 2 つの組み合わせだけを拾ってメニューと同じ関数を呼ぶ。
- ✅ 台本の中の並びが保たれ、大きなファイル（`drag_follow.rs` 937・`window_move.rs` 1,221）にはほとんど足さない。組の帳簿をドラッグと詰め直しの両方から読める。
- ❌ 新しいファイルが 4〜5 本。`reseed.rs` の関数の切り出しは、シェルの切り替えの試験（`reseed_tests.rs`）を通し直す必要がある。

### 1.5 暫定の裁定ごとの見立て

| # | 暫定の答え | コードが示すこと | 案ごとの手間 |
|---|---|---|---|
| 1 | ゴーストの読み直しで保たない・SHIORI だけの読み直しで保つ | ゴーストの読み直しは窓を閉じて起こし直し、SHIORI だけの読み直しは窓と結線を残す。重なりの組も同じ寿命になっている。 | **暫定どおり**: 吸着は手間なし（窓と一緒に消える）。組は World に置くなら外す 1 行、`Emo2Wiring` に置くなら手間なし。**覆して保つ**: 読み直しの前に控えて起こし直した後に戻す仕組みが要る（`shell_carry.rs` の面の持ち越しと同じ型）。2 タスクほど増える。 |
| 2 | シェルの切り替えで保つ（descript に従っていた分だけ取り替え） | `reseed.rs:139-146` は向きを無条件に上書きし、重なりの組は台本の分も捨てている。 | **暫定どおり**: スコープごとの「台本で変えた」印と、`apply_shell_descript` の読み分け（印のあるスコープは `Anchored` と `DragConfig` を書かない）・descript の組だけの取り替え・重なった組の記録。1〜2 タスク。重なりの組とは寿命が食い違ったまま残る（COMPAT §8 に書く）。**覆して捨てる（重なりとそろえる）**: 印が要らず、`apply_shell_descript` は今のまま・組は descript で据え直すだけ。手間が最も小さい。 |
| 3 | 読めない向きは変えずに記録 | descript の側は読めない値を下へ倒している（`resolver.rs:447`）。 | どちらでも手間は同じ（読みの分岐 1 つ）。暫定どおりなら台本の読みを descript の写しと別にする。 |
| 4 | 拒否は組を作らず記録・トークは続ける | 重なりの帳簿が同じ捨て方をしている（`ZOrderReject`）。 | 暫定どおりなら重なりの拒否の型と同じ形で書ける。手間は小さい。 |
| 5 | ドラッグで連れ動いた相手も記憶に書く | 記憶へ書く口はドラッグの終わりにある。書く形は向きで変わる（1.2 の ★）。 | **暫定どおり**: 相手の分の書き込み（数行）＋向きの食い違いの手当て（1.6 の議題 6）。**覆して書かない**: 手間なし。ただし次の起動で組が離れる（要件 9.7 の確かめが成り立たない）。 |
| 6 | 当て直し・大きさの変化・作業領域の変化・位置の初期化では相手を動かさない | 連れ動きを差し込むのはドラッグと `\![move]` だけで済む。 | **暫定どおり**: 手間なし。**覆して動かす**: `resize_window_to`（`window_move.rs`）や `work_area_sync.rs`・`reset_pos.rs` にも差し込む要があり、`window_move.rs`（1,221 行・行数の番人の例外）に触れる。吸着の当て直しの連鎖（相手がまた当て直す）の扱いも決める必要がある。 |
| 7 | どの窓がどの組かを見られるようにするだけ（詰め直しは向こう） | `chain_realign.rs` は `placement` の中。組の帳簿を World の資源に置けば、向こうは資源を読むだけで済む。 | **暫定どおり**: 読み口を 1 つ公開するだけ。`Emo2Wiring` に置くと読めない（層の向きに反する）ので、帳簿の置き場はこの裁定から決まる。 |
| 8 | 初期化したバルーンの位置を記憶に書く | 記憶を消す口が無い（`persist_put` だけ）。シェル位置初期化も書いている。 | **暫定どおり**: `balloon_offset_entries`＋`persist_entries` で書ける。**覆して記憶から消す**: sylphya に消す口を足す（別のクレート・保存の形式に触れる）。 |
| 9 | ゴーストの窓が入力先のときだけ | 窓の手続きに届くキーは、その窓が入力先のときだけ。全体の近道（`RegisterHotKey`）は使っていない。 | **暫定どおり**: 窓の手続きで受ければ自然にこの形になる。**覆してどこでも効かせる**: `RegisterHotKey` で全体に登録する要があり、ほかのアプリの同じキーを横取りする。 |

### 1.6 設計で決めること（議題の候補）

1. **組の帳簿の置き場**: World の資源（ドラッグ・詰め直しから読める・ゴーストの切り替えで外す 1 行が要る）か `Emo2Wiring`（寿命は自動・ドラッグから読めない）か。暫定の裁定 7 を保つなら前者。
2. **台本の吸着の指令が窓を書くときの経路の語**: 今ある `AnchorChange`（基盤の置き直しの扱い＝既定の位置が付いて動き、可視の見張りが掛かり、拡大率の整合待ちの窓へは届いてはならない側）を使うか、台本の明示の操作として新しい語を足すか。新しい語を足すと `placement/diag.rs` の一覧と、`window_move.rs:543` 付近の全部の語を並べた判定に手が入る（brief の「`window_move.rs` は触らずに済ませたい」と当たる）。既定の位置が付いて動くかは、詰め直しの対象から外れるかどうかに効く。
3. **連れ動きの書き込みの経路の語**: ドラッグの相手は「経路なし（ドラッグと同じ）」、`\![move]` の相手は `MoveCue` を使い回すか、連れ動き専用の語を足すか（議題 2 と同じく `window_move.rs` に触れるかが分かれ目）。
4. **ドラッグの差分の取り方**: 開始のときの相手の位置を控えて「開始からの量」で置くか、1 回ごとに「前の位置との差」を足すか。控えるなら控えの置き場（窓の部品か資源）と、最初の `OnDrag` で控えるか開始の受け手を新しく付けるか。
5. **台本で変えた吸着の印と `default` の戻り先**: 印を窓の部品に持つか帳簿に持つか。`default` の向きを `DefaultPlacementBasis` から引き直すか、作るとき・シェルの切り替えのときに窓へ控えるか。
6. **台本で変えた向きのまま書いた記憶の形**: ドラッグの終わりに記憶へ書く位置の形（下の吸着だけ中央）を、今の向きで決めるか descript の向きで決めるか（後者なら次の起動で窓がずれない）。組の相手の分も同じ規則に乗る。
7. **バルーン位置初期化の材料をそろえる方法**: `DefaultPlacementBasis` を「descript の値＋バルーンの材料（作者の DPI・`windowposition`）」に改め、初期化のたびに今の窓の DPI で拡大して解く（`reseed.rs` の流れを共有）か、作るとき・シェルの切り替えのときに各スコープの既定のずらしを控えておき、拡大率の追従の仕組みに任せるか。キーワードでずらしを決めるバルーン（`BalloonKeywordBase`）の扱いもここで決まる。
8. **受け口の分け方**: `MoveCueSink` に相乗り（台本の中の並びが保たれる）か、新しい受け口と線か。相乗りなら `zorder_cue.rs` の「`set`／`reset` の開けない荷物は自分宛」の決まりを見直すか（同じ荷物に警告が 2 行出ないように）。
9. **キーボードの近道の土台**: wintf に「キーの押下を知らせる出来事」を一般の形で足すか、近道の 2 つの組み合わせだけを拾う狭い口を足すか。F10 が `WM_SYSKEYDOWN` で届くなら、その振り分けも足す。Shift＋F10（OS が右クリックのメニューの代わりに使う組み合わせ）と重ならないか。
10. **メニューの枠**: 「バルーン位置初期化」を新しい枠として「シェル位置初期化」の隣に置くか（`ORDER`・群・テストの数に手が入る）。選べる場面は「窓の組が在る」だけでよいか（シェル位置初期化は答え待ちの間は灰色＝バルーンの側に答え待ちは無い）。
11. **ID の上限**: 正典は 3 人目以降のスコープ ID も書ける。窓の無い ID を受け付けて記録する（要件 3.6）とき、数として読める限りいくつでも受けるか、上限を置くか。

### 1.7 調べが要ること

1. **キーボードの入力先**: キャラクターの窓・バルーンの窓をクリックした後に、その窓が本当にキーの入力先になるか（透過の切り替え `WS_EX_TRANSPARENT`・重ねた窓・メニューを閉じた後）。実機で `WM_KEYDOWN` の届き方を確かめる。
2. **Ctrl＋Shift＋F10 の届き方**: F10 は単独だと `WM_SYSKEYDOWN` で届く。Ctrl・Shift と一緒のときに `WM_KEYDOWN` か `WM_SYSKEYDOWN` か、`DefWindowProc` が何かをするか（Shift＋F10 は `WM_CONTEXTMENU` に化けうる）。
3. **なしの窓のドラッグでの位置の書き換えの時機**: wintf が窓を動かした後、`on_char_drag` が呼ばれる時点で `WindowPos.position` が新しい値か古い値か（議題 4 の差分の取り方を決める）。
4. **SSP で測る点**（要件の暫定の裁定に「SSP で測る」とある 1・2・3、brief の「相手のバルーンも一緒に動くか」）: 開発者の機械の SSP の MCP で、読み直し・シェルの切り替えの後に台本の吸着と組が残るか、読めない向きで何が起きるか、組の相手のバルーンがドラッグで付いてくるか。
5. **検体の中身**: `pasta-in-windows` を `target\` の下へ取り寄せて、シェルの descript（`seriko.sticky-window,0,1`・`seriko.alignmenttodesktop,free`）と辞書の「窓あわせ」の `\![move]` の引数を確かめる。記憶から 2 つの窓を戻すとき、作業領域へ寄せ直す処理（`apply_restored_placements`）が 2 つを別々に寄せて離さないか。
6. **ぱすたの起動の並べ直し**: 起動の連鎖の確定（`chain_finalize`）が、記憶の無い 2 つの窓を横に並べた後に辞書の `\![move]` で重ねる流れになるか（重ねた後に何かが並べ直さないか）。

### 1.8 規模と危険度

- **規模: L（1〜2 週間）**。吸着の指令（2〜3 タスク）・組の帳簿と descript（2〜3）・ドラッグと `\![move]` の連れ動き（2〜3）・寿命とシェルの切り替え（1〜2）・バルーン位置初期化と材料の統一（2〜3）・メニュー（1）・近道と wintf の口（1〜2）・記録と台帳（1）・実機（1）＝13〜19 タスク。20 を超えそうなら brief の分け方（「吸着の指令」と「組」）より、「近道」を後ろへ回す方が切り口がきれい（ぱすたに要るのは組だけ）。
- **危険度: 中**。今の部品と型に乗る部分が大半で低いが、キーボード（wintf に新しい口・F10 の届き方が不明）と、ドラッグの差分の時機（不明）と、記憶の形の食い違い（見落とすと次の起動でずれる）が中に押し上げる。

### 1.9 設計への申し送り

- 推す形は案 C（受け口は今の線へ相乗り・帳簿と連れ動きは新しいファイル・World の資源）。暫定の裁定 7 を保つ限り、帳簿は World に置くことになる。
- `window_move.rs` に触れるかどうかは議題 2・3（経路の語）だけで決まる。使い回すなら触れずに済む。
- 古い注記の直し（`window_move.rs:533` の「本番では未結線」と、`anchor.rs:174` の「seriko が書き換える」）は、`AnchorChange` の扱いを決めるときに一緒に直す。
- 要件 7.4 を満たすには、バルーン位置初期化より先に `DefaultPlacementBasis` の中身をそろえる（move-canon の申し送り 2.2）。
- 同じウェーブに置けない相手（brief の 2026-10-22 の再測定）: `descript-window-keys`（`config.rs`・`drag_follow.rs`）・`dpi-realign-remembered-chain`（働きでもこちらが先）・受け手の表に行を足す v0.1.0 の各 spec。

### 1.10 SSP での測り（2026-10-22・要件ディスカッションの前）

開発者の機械の SSP（MCP）で、ゴースト「悪役令嬢クローディア」（シェルの descript は `sakura.seriko.alignmenttodesktop,free`・`kero.…,free`）に台本を送り、窓の矩形を `currentghost.scope(0).rect` で読んだ。読み直し・シェルの切り替え・設定の変更はしていない。途中で SSP のゴーストが別の作業で「窓際のぱすたさん」へ切り替わったため、窓の組の測りの結果は読めず、そこで止めた。

| 送った台本（`\0` で） | 窓の上下（top–bottom） | 読み取り |
|---|---|---|
| 測る前 | 1204–1704 | 1704 が作業領域の下端 |
| `\![set,alignmenttodesktop,bottom]`（上へずらした後） | 1104–1604 → 1204–1704 | 下を指定するとすぐ下端へ沿う（要件 1.1 と合う） |
| `bottom` の後に `\![move,--X=0,--Y=-100,--base=me,--time=0]` | 1204–1704 のまま | 下の吸着は `\![move]` の縦を受けない |
| `bottom` の後に `\![set,alignmenttodesktop,foo]` → 同じ `\![move]` | 1104–1604 へ動いた | **読めない語は吸着を外す**（なし、または descript の向き。この検体は descript がなしなので見分けられない）。窓はその場に留まる |
| `bottom` の後に `\![set,alignmenttodesktop]`（値なし）→ 同じ `\![move]` | 1204–1704 のまま | **値が欠けた指令は何も変えない** |
| `\![set,alignmentondesktop,free]` → 同じ `\![move]` | 1104–1604 へ動いた | 古い綴りも効く（要件 1.4 と合う） |
| `\![set,alignmenttodesktop,default]`（descript がなし） | 動かない | descript の向きがなしなら沿わせない（要件 1.3 と合う） |

- どの台本でも、strict のエラーの記録は 0 件（読めない語も値なしも、エラーとしては扱われない）。
- 測れなかった点: 窓の組（`\![set,sticky-window]` の連れ動き・重なった ID の扱い）、ゴーストの読み直し・シェルの切り替えでの寿命、組の相手のバルーンがドラッグで付いてくるか。読み直しとシェルの切り替えは開発者の許可が要る。

## 2. 設計の段の調べ（2026-10-22・基準 `c2bc9727`）

### 2.1 調べの種類と範囲

- 種類: **軽い調べ（既存の仕組みの拡張）**。新しい外部のクレート・外部の API は無い。調べはコードを読む 3 本（ドラッグと `\![move]` の道・受け口と寿命とメニュー・wintf のキー入力）を並べて行った。
- 使った手引き: `kiro-spec-design` の決まり（調べ・まとめ・設計の関門）と設計の雛形だけ。画面の作り・利用しやすさの手引きは、この機能（窓の位置の振る舞い）に当てはまる項目が無いので使っていない。

### 2.2 1.7 の調べへの答え

| # | 調べ | 答え | どこで |
|---|---|---|---|
| 1 | キーボードの入力先 | 窓は `WS_EX_NOACTIVATE` を持たず、`WM_MOUSEACTIVATE` も受けないので、不透明な所をクリックすると窓が入力先になる（透過の所は `WS_EX_TRANSPARENT` で抜ける）。メニューも `SetForegroundWindow` でキャラクターの窓を前に出してから開く。実機の確かめは実装の段の実機の項目に回す | `placement/spawn.rs:588`・`wintf/src/ecs/clickthrough/win_style.rs:353`・`menu/win32.rs:36` |
| 2 | Ctrl＋Shift＋F10 の届き方 | コードには手がかりが無い。Windows の決まりでは F10 単独は `WM_SYSKEYDOWN`、Ctrl と一緒なら `WM_KEYDOWN` の見込み（確かさは中）。**設計は両方で受けて消費する**ので、どちらで届いても効き、`SC_KEYMENU`・Shift＋F10 の `WM_CONTEXTMENU` も起きない。実機の確かめは実装の段 | `wintf/src/ecs/window_proc/mod.rs:34-91`（`WM_SYSKEYDOWN` は今は受けていない） |
| 3 | なしの窓のドラッグでの位置の書き換えの時機 | wintf は `WM_MOUSEMOVE` の中で窓を動かし、`WM_WINDOWPOSCHANGED` で `WindowPos` を先に書く。`OnDrag` はその後のフレームの `Input` の予定表で、何回分かの動きをまとめて 1 回届く。**`on_char_drag` が呼ばれるとき `WindowPos` はもう新しい**。ドラッグの事件は絶対の位置（始めと今）を運び、差は運ばない | `wintf/src/ecs/window_proc/mouse_move.rs:258-313`・`window_pos.rs:143-150`・`ecs/drag/dispatch.rs:340-381` |
| 4 | SSP で測る点 | 本設計では測っていない（SSP と開発者の機械の状態には触らない）。答えは要件の段の裁定に既にあり、設計はそれに従う | — |
| 5・6 | 検体の中身・起動の連鎖と記憶の寄せ直し | 本設計では取り寄せていない。実機の確かめ（要件 9.7）の段で見る項目として設計の Testing Strategy に置いた | — |

### 2.3 新しく分かったこと（設計に効いたもの）

- `DragConfig.move_window` は wintf がドラッグの始め（`dispatch_drag_events` の `Started`）にだけ読む。同じ回に `OnDragStart` が呼ばれるので、areka はそこで「始めの向き」を控えられる。今の `on_char_drag` は毎回 `Anchored` を読むので、ドラッグ中に向きが変わると、wintf の書き手と areka の書き手がずれる（シェルの切り替えでも同じ穴がある）。設計は控えた向きで縛ることで要件 2.4 を満たす。
- `DraggingState.initial_inset` はドラッグを始めたときの窓の位置（窓の座標）。ドラッグした窓の「始めからの量」はこれで取れる。
- 経路の語: `MoveCue` は可視の見張り・既定の位置の追従・拡大率の整合待ちの監視のどれも掛からない。`AnchorChange` は 3 つとも掛かり、整合待ちの窓へ書くと `debug_assert` が鳴る（`window_move.rs:507-557`・`677-692`）。ドラッグは経路なし（`None`）で、こちらも整合待ちの監視の外。
- `enqueue_window_set_pos` は `follow` の中だけで使える（`pub(super)`）。組の相手をドラッグと同じ経路なしで書くには、連れ動きのファイルを `placement/follow/` に置く必要がある。
- `resolve_placement` のキャラクターの窓の位置（P1〜P4）はバルーンの欄（ずらし・`balloon_limit`・`balloon_x_mode`）を読まない（バルーンの欄は P5 だけ）。だから `DefaultPlacementBasis` を「descript の値そのまま」に揃えても、並べ直しの結果は変わらない。
- 起動の準備（`prepare_stages_for_shell`）は `DescriptSource`・バルーンの `windowposition`・作者の DPI を全部読んでいる。シェルの切り替えも同じ 3 つを背景で読んで UI へ運ぶ。これを材料として置けば、起動とシェルの切り替えで材料が同じになる。
- `persist_unremembered_char_positions` も今の `Anchored` で記憶の形を決めている。台本で `free` にしてその場に留まった窓は既定の位置のままなので、ここで書かれて次の起動でずれる。ドラッグの終わりと同じ規則（descript の向き）にする必要がある。
- 受け手の表の突き合わせの試験は、表の名前×最初の引数の掛け合わせ（例 `reset,alignmenttodesktop`）がどこにも届かないことも確かめる。`MoveCueSink` の選び方は組ごとに完全一致で書く。
- `ZOrderCueSink` の「自分宛の警告」は、開けない形の `set`／`reset` の荷物にだけ出る。正しい形の `set,alignmenttodesktop` などは届かない（`_` の腕で `debug!`）。`MoveCueSink` が新しい組を受けても警告は二重にならない。
- wintf にはキーの押下をアプリへ知らせる口が無いが、窓の手続きの中で呼ぶコールバックの部品の型（`OnFilesDropped`）があり、areka はそれをゴーストのすべての窓に付けている（`input_events/file_drop.rs`）。キーも同じ型で足せる。修飾キーは `WM_KEYDOWN` の引数に入らないので、窓の手続きの入口で読み、判定は値を受ける純関数に置く（決定論のテストのため）。

### 2.4 まとめ（一般化・作るか使うか・簡素化）

- **一般化**: 「組の相手を、ある量だけ、相手の向きで射影して動かす」はドラッグと `\![move]` に共通なので、純関数 `partner_target` を 1 つにした。書き込みの経路だけが呼び手で違う（ドラッグは経路なし・`\![move]` は `MoveCue`）。バルーン位置初期化とシェルの切り替えは「記憶を重ねるか」だけが違う同じ解決なので、`resolve_standard_scopes` を共有した。台本・メニュー・近道の 3 つの入口は同じ `reset_balloon_positions` を呼ぶ。
- **作るか使うか**: 新しい外部のクレートは不要。窓の組の帳簿は重なりの帳簿 `ZOrderGroupLedger` と寿命・単位・置き場が違う（研究 1.1）ので型の流儀だけを写し、ID の読み `parse_scope_id` は `pub(crate)` にして使い回す（写さない）。キーは Windows の全体の近道（`RegisterHotKey`）を使わず、窓の手続きで受ける（裁定 9 がそのまま形になる）。
- **簡素化**: 新しい経路の語・新しい受け口・新しい線・`anchor_changed_system` の結線は足さない。帳簿の World 側の関数は帳簿のファイルに置き、ドラッグの控えは連れ動きのファイルに置いて、新しいファイルを 6 本（テストを除く）に抑えた。`window_move.rs`・`diag.rs`・`chain_realign.rs`・`work_area_sync.rs` には触れない。

### 2.5 設計の決め（1.6 の 11 議題の答え）

設計の「設計の決め」の表が正本。要点:

1. 帳簿は World の資源（`close_windows_for_restart` で外す）。
2. 吸着の指令の当て直しは `resize_window_to(…, MoveCue)`。
3. ドラッグの相手は経路なし、`\![move]` の相手は `MoveCue`。
4. 「始めからの量」。始めの位置と始めの向きは `OnDragStart` で控える（資源 `CharDragSession`）。
5. 窓の部品 `AnchorSource { descript, by_script }`。
6. 記憶へ書く形は descript の向き（3 か所）。
7. `DefaultPlacementBasis` を「descript の値＋バルーンの材料」に揃え、シェルの切り替えと同じ解決を記憶なしで通す。
8. `MoveCueSink` に相乗り。`zorder_cue.rs` は注記だけ。
9. wintf に一般の `OnKeyDown`（`WM_KEYDOWN`・`WM_SYSKEYDOWN`）。判定は areka の純関数。
10. 新しい枠を「シェル位置初期化」のすぐ後へ。
11. ID の上限は置かない。

### 2.6 危険と手当て

| 危険 | 手当て |
|---|---|
| ドラッグ中に台本が向きを変えると、離した後に窓が辺へ飛ぶ | 正典が黙る点として COMPAT §8 に書く。テストで固定する |
| F10 の届き方・入力先の当たり方が実機で違う | 両方のメッセージで受けて消費する。実機の確かめの項目に近道を入れる |
| `drag_follow.rs`（937 行）が 1,000 行に近い | 呼び出し数行だけを足し、中身は `sticky_follow.rs` へ |
| `DefaultPlacementBasis` の形の変更が並べ直しのテストに触れる | P1〜P4 がバルーンの欄を読まないことを確かめた。並べ直しのテストを通し直す |
| 起動の連鎖・記憶の寄せ直しが重ねた 2 つの窓を離す | 実機の確かめで見る（研究 1.7 の調べ 5・6）。食い違いはこの spec で直すか持ち主を確かめて起票 |

### 2.7 境目と後の spec への影響

- 読む口 `StickyGroupLedger::group_of`／`partners_of` は `dpi-realign-remembered-chain` が詰め直しから組の窓を外すときに使う。形を変えたら向こうが読み直す。
- `AnchorSource` と記憶の形の規則は `window-snap`（吸着の相手を広げる）の土台になる。
- `PositionCue` の種類と受け手の表の 5 行は、`move_cue*.rs`・`consumer_ledger.rs` を触る後の spec と同じウェーブに置けない。

### 設計ディスカッション（2026-10-22）

- 設計の検証の問題 1（ドラッグの終わりに wintf の DraggingState が無いと相手の記憶が残らない）は自明な修正として、ドラッグした窓の始めの位置も CharDragSession に控える形にした。
- 問題 2（2 つとも吸着なしの組のドラッグ中のずれ）は議題 1 として出し、開発者が推奨案 A を承認した: 組に入った窓は吸着なしでも DragConfig.move_window=false にし、ドラッグした窓も相手も areka が同じ回に書く（設計の決め 12）。根拠は記憶「1 フレーム遅らせる解は取らない・状態の持ち方を変えて 0 フレームで解け」。`on_char_drag` の今の分かれ目は `Anchored` が Free かで決めているので、ドラッグの始めに控えた「wintf が動かすか」で分けるように改める。
