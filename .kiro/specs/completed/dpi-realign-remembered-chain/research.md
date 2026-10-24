# ギャップ分析: dpi-realign-remembered-chain

- 調べた日: 2026-10-23（ワークツリー `claude/kiro-start-dpi-realign-remembered-chain-efe2ac`・main `15d8b0f3` の上・requirements.md は `8ae287b2`）
- 読んだもの: requirements.md・brief.md・steering（product / tech / structure・roadmap の C11 の ⑥ の行と約束）・下の「今の姿」に挙げたコード
- この文書は判断の材料を並べるもので、決めはしない（決めは要件の話し合いと設計の段）

## 分析の要約

- **詰め直しの判定は作り直しになる**。今の詰め直し（`placement/chain_realign.rs`）は起動の並べ直しの判定 `finalize_chain`（`placement/chain_finalize.rs`）をそのまま使い、「既定の位置のままか」で対象を決める。この判定は ⑴ 変わる**直前**の位置と幅を持たない（拡大率が変わった**後**の位置しか見ない）・⑵ 隣を左にしか置けない（`new_x = 前の x − 自分の幅`）・⑶ 外した窓の今の位置を次の基準に使う（要件 2.4「ドラッグ中の窓へ寄せない」と当たる）。要件 1〜3 は、新しい純粋な判定関数と、変わる直前の窓の位置と大きさの控えを要する。控えは武装の資源 `ChainRealignPending` の中に持てば、窓の作り直しで消す所（`app_exit.rs` 196〜198 行目）に手を入れずに済む。
- **件 9 の直す所は `mouse_move.rs` だけでは足りない**。ぱすた（組に入った窓）は `move_window: false` で、ドラッグ中と落とした時の窓の位置を決めるのは wintf の `mouse_move.rs:302` ではなく、areka の `placement/follow/drag_follow.rs` の `policy_mapped_position`（276〜292 行目・`DraggingState` の `initial_inset + (カーソル − drag_start_pos)`）である。`drag_follow.rs` は ① `descript-window-keys` の持ち物で触れない。触らずに直す道は「窓の大きさが変わったときに `DraggingState`（公開の欄）の始めの位置を掴んだ点の比で置き直す」で、置き直す所は ⑥ の `emo2_boot/frame/dpi.rs`（キャラ窓の大きさを書く所）に置ける。`move_window: true` の窓のために `mouse_move.rs` が同じ置き直した値を読むようにすれば、wintf 側も ⑥ の 2 ファイルの内に収まる見込み。
- **要件 6 の案 A は、記憶の形を足すかぎり sylphya の `persist/mod.rs` に必ず手が入る**（記憶の項目は閉じた型 `PersistKey` と TOML の写し `FormatDoc` で、自由な鍵は無い）。加えて `persist/format.rs`（③ は名指ししていない）も要る。一方 **`drag_follow.rs` は要らない**: ドラッグの確定・組の相手の保存・並べ終えた時点の保存・並べ直しの保存は、どれも ⑥ の `placement/persist.rs` の `persist_entries` を通るので、そこで幅（または印）を足せる。起動・読み直し・切り替えで記憶から戻す側は、3 つとも `ghost_session::open_ghost_windows` → `main.rs` の `restore_merged_placements` の 1 本の道を通るので、`main.rs` と `ghost_session.rs`（⑥ の一覧の外・C11 のほかの持ち主は名指ししていない）に数行ずつ要る。
- **記憶の形を足さない案 A（推定）だけが ⑥ の一覧の内に収まる**が、拡大率が決まった段（100%・125%…）しか取らないことに頼る推定で、表情の違いや偶然の一致で外れる。案 B と案 C には、要件 2.1・2.2 と組み合わさって「一度起動し直すと、その組は二度と詰め直されなくなる（離れた組か重なった組として固まる）」という副作用がある（3.6 節）。
- 規模は M（要件 1〜5・7・8 で 9〜12 タスク・案 A の記憶の分で +3〜5）。危険は中（判定と控えは既存の型の上で決定論のテストで固められるが、ドラッグ中の拡大率の変化の順序は wintf と areka の 2 段にまたがり、実機の確かめが要る）。

## 1. 今の姿（コードの読み）

### 1.1 拡大率が変わった後の詰め直しの流れ

| 段 | 場所 | 中身 |
|---|---|---|
| 武装 | `emo2_boot/frame/dpi.rs` 350〜370 行目 | キャラ窓の大きさを書いた（`reconcile_window_size`）とき、書いた・キャラ窓・大きさが変わった、の 3 つが揃えば `chain_realign::arm_chain_realign` を呼ぶ。`before`（書く前の大きさ）はここで読める。**書く前の位置は控えていない** |
| 武装の資源 | `placement/chain_realign.rs` 72〜76 行目 | `ChainRealignPending { armed_frame }` だけ。`ChainFinalized` が無ければ武装しない・2 度目の武装は最初の値を保つ |
| 解決 | `chain_realign::realign_chain_once_with`（136〜189 行目） | 待ちの札 `DpiSyncHold` が無く、全スコープの窓の大きさが実際に描いた大きさに揃った最初のフレームで、走査 `collect_chain_states`（`drain_resnap.rs` 544〜605 行目）の結果を `finalize_chain` へ渡し、動かす窓を `move_window_with_route(.., PlacementRoute::ChainRealign)` で動かして武装を解く |
| 呼び順 | `emo2_boot/frame.rs` 375〜387 行目 | `run_move_drain_phase`（台本の `\![move]`）→ `finalize_chain_once`（起動の並べ直し）→ `realign_chain_once` の順に毎フレーム |
| 作り直しで消す | `app_exit.rs` 196〜198 行目 | `ChainFinalized`・`ChainFinalizeStall`・`ChainRealignPending` を外す（読み直し・切り替えで窓を作り直すとき） |

### 1.2 判定 `finalize_chain`（`chain_finalize.rs` 123〜155 行目）

- 入力は `ScopeChainState { scope, current_x, width, default_x }`（今の位置と今の幅だけ）。
- スコープ番号の小さい順に、先頭は動かさず、2 番目からは `default_x == Some(current_x)` のときだけ `new_x = 前の結果の x − 自分の幅`（隣は**左**に置く形しか無い）。外した窓は今の位置を次の基準にする。
- `default_x` は窓の一式 `GhostWindows` の `default_char_pos`。記憶から戻したスコープは `ghost_session.rs` 285〜291 行目で `None` にされ、**必ず外れる**（後退の原因）。
- 既定の位置は、システムが置き直したとき（拡大率の置き直しなど）に `follow/window_move.rs` の `track_default_char_pos`（816〜870 行目）が一緒に運ぶ。台本の移動・ドラッグでは運ばない。`None` は `None` のまま。

### 1.3 台本で置いた窓の印と、窓の組の帳簿

- 印 `ScriptPlaced`（`chain_finalize.rs` 65〜66 行目）: 付けるのは `emo2_boot/move_cue_place.rs` 178 行目（`\![move]` の対象）と `placement/follow/sticky_follow.rs` 368 行目（連れ動いた組の相手）。読むのは起動の並べ直し（`drain_resnap.rs` 447〜451 行目）と並べ終えた時点の保存（`persist.rs` 604 行目）だけで、**拡大率の後の詰め直しは読まない**。印は時刻を持たず、一度付くと並べ直し（`reset_pos`）まで残る＝「拡大率が変わった**後**に台本が置き直したか」（要件 3.1）は印の有無だけでは見分けられない（要件 3.3 は変わる前に台本で置かれた窓を対象に**入れる**）。
- 組の帳簿 `StickyGroupLedger`（`placement/sticky_group_ledger.rs`）: `group_of(scope)`（126 行目）・`partners_of(scope)`（133 行目）。descript の `seriko.sticky-window` と台本の組の両方を持つ。要件 2.3 の「一緒に動く組の窓」はこれで引ける（descript の組の窓は印 `ScriptPlaced` を持たないことがあるので、印では足りない）。

### 1.4 ドラッグの位置の決め方（件 9）

| 窓の種類 | 書き手 | 位置の式 | 始めの値を控える所 |
|---|---|---|---|
| 吸着なし・組なし（`move_window: true`） | wintf の wndproc | `mouse_move.rs` 302〜303 行目 `initial_window_pos + (カーソル − start_pos)` | スレッドの持ち物 `DragState::Dragging.initial_window_pos`（`wintf/src/ecs/drag/state/mod.rs`）。`dispatch.rs` 147〜152 行目で `WindowDragContextResource` に入れたものが写される |
| 吸着した窓・**組に入った窓**（ぱすた・`move_window: false`） | areka | `drag_follow.rs` の `policy_mapped_position`（276〜292 行目）`initial_inset + (カーソル − drag_start_pos)` → `project_anchor` | 部品 `DraggingState { drag_start_pos, initial_inset }`（`wintf/src/ecs/drag/mod.rs` 76〜81 行目・欄は公開）。`dispatch.rs` 155〜160 行目で入れる |
| 落とした時（両方） | areka | `drag_follow.rs` の `on_char_drag_end` が同じ `policy_mapped_position` で最後の位置を決めて記憶に書く（248〜249 行目） | 同上 |

- brief の根拠のログ `char DragEnd 保存 scope=0 char_x=-1725 char_y=194` は `drag_follow.rs` 236 行目の行で、ぱすたの件 9 は**2 段目の式**で起きている。`mouse_move.rs` だけを直しても、ぱすたの落とした位置は変わらない。
- 組の相手は `sticky_follow.rs` 288 行目で「ドラッグした窓の始めの位置からの差」だけ動く。掴んだ点を縮めると差も変わるので、相手はドラッグした窓と同じ左上の差を保つ（ぱすたは同じ左上に重ねているので崩れない・要件 4.5）。
- 拡大率の違う画面へ入ると wintf は `WM_DPICHANGED`（`window_proc/window_pos.rs`）で勧められた矩形へ窓を置き、続いて areka の拡大率の相（`dpi.rs`）が窓の大きさを書き直す。どの順でカーソルの移動と重なるかは調べが要る（6 節）。

### 1.5 記憶の形・書く所・戻す所

- 形: `crates/areka-sylphya/src/persist/mod.rs` の閉じた型 `PersistKey`（9 種）と `persist/format.rs` の `FormatDoc`（`[window."0"]` に x・y の 2 つの文字列）。モジュールの説明に「自由 key の汎用永続は将来シーム」とあり、**足すには型と TOML の写しの両方を直すしかない**。`format-version` を上げると古い areka は全部を「無し」として読む（`format.rs` 233〜244 行目）ので、足すなら版を上げずに任意の表として足す形になる。読み込んだ値は鏡像（`actor.rs` 605〜606 行目）にも入る＝新しい鍵は `get_property` などから読める値になる。
- 窓の位置の中身: キャラ窓ごとの「足元の中央の x と上端の y」（`persist.rs` の `char_pos_to_origin_x`・65〜73 行目）。幅も拡大率も残らない。
- 書く所（4 つ）。**どれも `persist.rs` の `persist_entries`（518〜535 行目）を通る**:
  - ドラッグの確定: `drag_follow.rs` 248〜249 行目（`char_pos_entries` → `persist_entries`）
  - 組の相手の確定: `sticky_follow.rs` 299 行目（同じ形）
  - 起動の最後に並べ終えた時点: `drain_resnap.rs` 501 行目 → `persist.rs` の `persist_unremembered_char_positions`（556〜643 行目）
  - 既定の位置への並べ直し: `reset_pos.rs` 130 行目（同じ関数）
  - 拡大率の詰め直しは書かない（今も・要件 6.1 はそのまま満たされている）
- 戻す所: 起動（`main.rs` 292 行目）・読み直しと切り替え（`ghost_session.rs` 351 行目の `reopen_ghost_windows`）は、どれも `ghost_session::open_ghost_windows` → `main.rs` 763〜795 行目の `restore_merged_placements`（読み込みはここ 1 か所・`load_restored_state`）を通る。戻したスコープの集まりを返し、`ghost_session.rs` が既定の位置を `None` にする。読み込んだ値そのものは World に残らない。

## 2. 要件と今ある物の対応

タグ: **欠** = 今無い・**未** = 調べが要る・**縛** = 既存の決まりや約束による縛り・**有** = 今あるもので足りる

| 要件 | 今ある物 | 足りない所 |
|---|---|---|
| 1.1 隣り合っていた組を記憶の有無にかかわらず詰める | 武装と一度きりの解決（`chain_realign.rs`） | **欠**: 変わる直前の窓の位置と大きさの控え。**欠**: 隣り合いで決める判定（`finalize_chain` は既定の位置で決める） |
| 1.2 隣り合いの定義（縦が重なり・重ならず・隙間が許す幅以内） | なし | **欠**: 判定。**縛**: 丸めで −1px の重なりが出うる（下端中央を保つ置き直しは幅の半分を整数で割る）。「重ならずに」を字のとおり読むと 1px 重なった組が外れる（7 節の論点 2） |
| 1.3 本体に近い側を止め、もう一方を横にだけ・左右を保つ | `finalize_chain` は左置きだけ | **欠**: 右側に置かれた組の式 `new_x = 前の x + 前の幅` |
| 1.4 縦を変えない | `realign_chain_once_with` は y を据え置く（170 行目） | **有** |
| 1.5 3 体以上・詰めた後の位置を次の基準 | `finalize_chain` は前の結果を次の基準にする | **未**: 「組」を番号の続く 2 つ（n−1 と n）に限るか、n の隣にいる番号の小さい窓とするか（7 節の論点 4） |
| 1.6 1 回の変化につき 1 回 | `ChainRealignPending` の 1 往復・`frame_chain_realign_tests.rs` の `one_transition_realigns_exactly_once` | **有**（控えを足しても往復は変えない） |
| 1.7 続けて変わる | 武装は最初の値を保つ | **未**: 解決の前に 2 度目の変化が来たとき、控えは最初の変化の直前のままでよいか（窓は途中の大きさで止まらないのでそれで足りる見込み） |
| 1.8 記憶から戻した窓だけで詰める | `None` で必ず外れる | **欠**（判定の作り直しで解ける） |
| 2.1 離れていた組 | `finalize_chain` は既定の位置で見る | **欠**（新しい判定の「隙間が許す幅より広い」） |
| 2.2 重なっていた組 | なし | **欠**（新しい判定。ぱすたは窓の枠と本体が同じ左上） |
| 2.3 一緒に動く組の窓 | `StickyGroupLedger::group_of` | **欠**: 詰め直しから帳簿を引く所（読むだけ） |
| 2.4 ドラッグ中の窓・その窓へ寄せない | `DraggingState`（キャラ窓の entity に付く）・`WindowDragging` | **欠**: 判定で読む所。**縛**: `finalize_chain` は外した窓の今の位置を次の基準にする＝「寄せない」に反する。新しい判定では外した窓の次の組を「基準が外れた」として動かさない形が要る |
| 2.5 表情の差し替えでは詰めない | 武装は拡大率の相だけ（`dpi.rs` 367 行目）・`a_surface_swap_during_a_conversation_never_arms_the_realign` | **有** |
| 3.1 変わった後・詰める前に台本が置き直した窓 | 印 `ScriptPlaced`（時刻なし） | **欠**: 「武装の後に動かされたか」の見分け（3.3 節） |
| 3.2 詰めた後の台本の移動を上書きしない | 武装を解けば以後は駆動しない | **有** |
| 3.3 変わる前に台本で置かれた窓は対象 | 起動の並べ直しは印で外すが、詰め直しは印を読まない | **有**（詰め直しで印を「対象から外す理由」にしなければよい）。ただし起動の並べ直しで外す働きは変えない |
| 3.4 ぱすたの並びを保つ | 組の帳簿・重なり | **欠**: 2.2・2.3 の判定で満たす。**未**: ドラッグの終わり（`OnMouseDragEnd`）でぱすたが出し直す `\![move]` と詰め直しの解決の順（6 節） |
| 4.1・4.2 掴んだ点を縮める | 1.4 節の 2 つの式 | **欠**: 窓の大きさが変わったときの始めの値の置き直し。**縛**: `drag_follow.rs` は ① の持ち物（3.4 節） |
| 4.3 同じ拡大率のドラッグは変えない | — | 置き直しは大きさが変わったときだけ＝**有**にできる |
| 4.4 窓の大きさは変える | DPI 追従の基本設計 | **有** |
| 4.5 組の相手が同じだけ動く | `sticky_follow.rs` の差の式 | **有**（差は始めの位置からの量なので、置き直した後もそのまま） |
| 4.6 確定の位置を記憶へ書く | `on_char_drag_end` | **有**（`policy_mapped_position` が置き直した値を読めば、落とした点に合った位置が書かれる） |
| 5.1・5.2 読み直し・切り替えでも同じ | 窓を作り直すと `ChainRealignPending` が消え、武装はやり直し。戻す道は 3 つとも 1 本 | **有**（判定がどの道で窓を作ったかを見なければよい） |
| 6.1 詰めた位置を書かない | 詰め直しは書かない | **有** |
| 6.2 書くときに幅（または印）を足す | 書く所は全部 `persist_entries` を通る | **欠**: 記憶の項目（sylphya）・足す所（`persist.rs`） |
| 6.3 戻すときに隣り合わせる | 起動の並べ直しは記憶から戻したスコープを外す | **欠**: 読み込んだ幅（印）を窓の一式へ運ぶ道（`main.rs`・`ghost_session.rs`・`spawn.rs`）と、並べ直しで使う判定 |
| 6.4 覚えたときに離れていた・重なっていた・組だった | — | **欠**（6.3 と同じ判定の外す側） |
| 6.5 古い記憶に情報だけ書き足す | `persist_unremembered_char_positions` は記憶があるスコープを書かない | **欠**: 「位置はあるが幅が無い」スコープに幅だけを書く枝 |
| 6.6 隣り合わせた位置を書かない・作業領域へ寄せる規則を変えない | 並べ終えた時点の保存は既定の位置が `None` のスコープを書かない | **有**（並べ直しで動かしても既定の位置は `None` のまま）。**縛**: `window-work-area-clamp` が後で同じ所に寄せを足す |
| 7.1 組ごとの判断の記録 | 動かした窓だけ `info!` | **欠**: 組ごとの判断・隙間・動かした量の行 |
| 7.2 情報が取り出せない窓 | 走査は 1 つ欠けると全体を見送る（`collect_chain_states`） | **縛**: 今は「全部揃うまで待つ」。要件 7.2 は「その窓だけ動かさず続ける」。待つ理由（大きさが揃っていない）と取り出せない理由（窓が無い）を分ける要がある |
| 7.3・7.4 §8 の記録・説明の書き直し | `chain_realign.rs` 27〜30 行目・`chain_finalize.rs` 22〜30 行目の説明 | **欠**（文の書き直し） |
| 8.1〜8.6 決定論のテスト | `FrameHarness`（`frame_test_support.rs` 902 行）・`frame_chain_realign_tests.rs`（593 行）・`placement/chain_finalize_tests.rs` | **縛**: 土台は上限の近く＝新しいファイルへ。今あるテスト `a_dragged_scope_stays_out_of_the_realign`（443 行目）は、新しい決め方では「ドラッグしたから」でなく「離れたから」（`DRAG_DX = 120`）で通る＝意図が変わるので書き直しが要る |
| 8.7 実機 | 排他テストの机・拡大率の違う 2 画面 | **有**（机の手順はある） |

## 3. 実装の選び

### 3.1 判定の作り方

| 案 | 中身 | 良い所 | 悪い所 |
|---|---|---|---|
| **J1 新しい純粋関数を足す**（例 `chain_realign` の中か兄弟の新しいファイル） | 入力＝スコープごとの「変わる直前の矩形」「今の x と幅」「外す理由（組・ドラッグ中・武装の後に動かされた・取り出せない）」。出力＝組ごとの判断と動かす量（記録にもそのまま使う） | `finalize_chain` を起動の並べ直しのまま残せる（起動の決まりを変えない＝要件の Out of scope と合う）。組ごとの判断を値で返せるので要件 7.1 の記録とテストが素直 | 判定が 2 つになる（起動用と拡大率用） |
| J2 `finalize_chain` を広げる | `ScopeChainState` に直前の矩形と外す理由を足し、1 つの関数で両方 | 判定が 1 つ | 起動の並べ直しの意味（既定の位置で決める）と拡大率の意味（隣り合いで決める）が 1 つの関数に混ざる。起動の側のテスト（505 行）にも影響 |

案 A（要件 6.3）の起動での隣り合わせは、J1 の関数を「覚えたときの矩形」で呼べば同じ規則を使い回せる（要件 6.3 の「要件 1.2〜1.5 と同じ決め方」）。

### 3.2 変わる直前の控えの取り方

| 案 | 中身 | 良い所 | 悪い所 |
|---|---|---|---|
| **S1 武装のときに全キャラ窓の矩形を控える** | `dpi.rs` の第 2 巡の前（どの窓もまだ書いていない時点）で全キャラ窓の位置と大きさを読み、武装が起きたら `ChainRealignPending` に入れる | 「直前」がそのまま取れる。作り直しで消す所（`app_exit.rs`）に手が要らない | `ChainRealignPending` が `Copy` でなくなる（`Vec` を持つ）。待ち札で窓ごとにフレームが割れる遷移でも、最初の武装の時点の控えでよいかの確かめ |
| S2 書く前の大きさだけ控え、位置は今の下端中央から戻す | 下端中央が保たれる前提で、直前の左端 = 今の中央 − 直前の幅 ÷ 2 | 控えが小さい | 「下端中央を保つ」が画面をまたぐときにも成り立つかに頼る。ドラッグで画面を移った窓は成り立たない |

### 3.3 「武装の後に台本が置き直した」の見分け（要件 3.1）

| 案 | 中身 | 触る所 | 注意 |
|---|---|---|---|
| **T1 位置で見る** | 拡大率の置き直しが書いた位置を控え（`dpi.rs` で書いた直後）、解決のときに今の位置と違えば「武装の後に動かされた」とする | `dpi.rs`・`chain_realign.rs`（⑥ の内） | 作業領域の寄せ直しなど、ほかのシステムの置き直しでも「動かされた」になる（動かさない側へ倒れるので安全）。ドラッグ中の窓もここで外れる |
| T2 印の変わった時刻で見る | `ScriptPlaced` は付け直すと bevy の変化の時刻が進む。武装のときの時刻と比べる | ⑥ の内（読むだけ） | World の変化の時刻がフレームの中で進むかに頼る（調べが要る）。ドラッグでは印が付かないので、ドラッグ中の窓は別に `DraggingState` を見る |
| T3 武装の後に動かした印を新しく付ける | `move_cue_place.rs`・`sticky_follow.rs` に 1 行ずつ | ⑥ の一覧の外（ほかの持ち主の名指しは無い） | 印が 2 種類になる |

### 3.4 件 9（掴んだ点を縮める）

| 案 | 中身 | 触る所 | 注意 |
|---|---|---|---|
| **D1 始めの値を置き直す** | ドラッグ中の窓の大きさが変わったら、掴んだ点の窓の中の位置 `g = drag_start_pos − initial_inset` を `g' = g × 新しい大きさ ÷ 始めの大きさ`（縦横別・丸め）にし、`initial_inset = drag_start_pos − g'` に置き直す。areka の式（`drag_follow.rs`）は置き直した値をそのまま読む。wintf の式（`mouse_move.rs`）も `DraggingState` の値（または同じに置き直した `WindowDragContext`）を読むよう直す | 置き直す所: ⑥ の `emo2_boot/frame/dpi.rs`（キャラ窓の大きさを書く所）または wintf の `drag/dispatch.rs`。読む所: `mouse_move.rs`。始めの大きさの控え: `dispatch.rs` の中に小さな部品を置けば `drag/mod.rs`・`context.rs`・`state/mod.rs` に触れずに済む見込み | `drag_follow.rs` に触れない。置き直しが `WM_DPICHANGED` の勧めの矩形・areka の大きさの書き直し・次のカーソルの移動のどの順で起きるかを確かめる要がある（6 節）。実測の形: (225,806) × 0.75 = (168.75, 604.5) → (169, 605)、窓の左上 (−1669, 395)（要件 8.5 と合う） |
| D2 式の所で縮める | 2 つの式（`mouse_move.rs` と `drag_follow.rs`）で、今の大きさと始めの大きさの比を掛ける | `drag_follow.rs`（① の持ち物）・`mouse_move.rs`・始めの大きさの控え（`drag/mod.rs` の `DraggingState` に欄を足す＝areka のテストの組み立てにも影響） | ① との順の調整か、`drag_follow.rs` の分を次のウェーブへ回すことになる |

### 3.5 要件 6（記憶）の案と、それぞれが触る所

| 案 | 記憶の形 | 書く所 | 戻す所 | ⑥ の一覧の外 | 主な危うさ |
|---|---|---|---|---|---|
| **A1 幅を覚える** | 新しい鍵（例 `[window-width."0"]` か `[window."0"]` に `w`）。sylphya の `persist/mod.rs`（型・正準の鍵・TOML への写しと戻し）と `persist/format.rs`（表の読み書き・`is_all_absent`）とそのテスト | `persist.rs` の `persist_entries` で、窓の位置の鍵が来たらそのスコープのキャラ窓の今の大きさを足す（4 つの書く所を全部覆う。`drag_follow.rs`・`sticky_follow.rs` に触れない）。古い記憶への書き足し（6.5）は `persist_unremembered_char_positions` に枝 | `main.rs` の `restore_merged_placements` が幅も返し、`ghost_session.rs` が窓の一式（`spawn.rs` に欄）へ入れ、起動の並べ直し（`drain_resnap.rs`）が J1 の関数で覚えたときの矩形を見て隣り合わせる | sylphya `persist/mod.rs`（③ と同じファイル）・`persist/format.rs`・`main.rs`・`ghost_session.rs` | **書いた時機が違う 2 つの記録の食い違い**: 実行中に 200%→100% で詰めた後（記憶は書かない）、100% で本体だけをドラッグして相方の横に付けると、本体の記録は 100% の幅、相方の記録は 200% の位置と幅のまま。起動で覚えたときの矩形どうしを比べると、利用者が見ていた並び（詰めた後の相方）と違い、隣り合いと読めない |
| A2 隣り合っていた印を覚える | 新しい鍵（例 スコープ n に「n−1 の左に隣り合っていた／右に／いいえ」）。触る所は A1 と同じ | `persist_entries` で、書く時点の画面の並び（表示の位置と大きさ）から印を決めて足す。片方をドラッグしたら相手の側の印も同じ束で書き直す（書く時機は足さず、中身を足す） | A1 と同じ道で印を運び、起動の並べ直しで印のある組を今の幅で隣り合わせる | A1 と同じ | 印は表示の並びで決まるので A1 の食い違いが起きにくい。代わりに「書く時点の画面の並び」を読む所が `persist_entries` に入る。相手の記録を書き直すのが「書く時機を足さない」（Out of scope）に当たるかの読み |
| A3 推定（記憶の形を足さない） | なし | なし | 起動の並べ直しで、記憶から戻した組の下端中央の距離 d と今の幅の和の半分 s を比べ、拡大率の段（100%〜500% の 25% 刻み）の比 ρ で `d ≈ s × ρ` となる ρ が 1 つだけあれば「覚えたときに隣り合っていた」と推定して隣り合わせる | **無し**（⑥ の `drain_resnap.rs`・`chain_*.rs` だけ） | 推定である。覚えたときと起動のときで表情（面の幅）が違えば外れる。離して置いた組がたまたま比に合えば寄せてしまう（要件 2.1 の破り）。シェルの切り替えの後も外れる |
| A4 別のファイルに置く | sylphya を通さず `placement/` が自分のファイルを持つ | `persist.rs` | 同左 | 新しい保存の口 | 統一プロパティシステム（sylphya）を迂回する二本目の記憶。書けなかったときの知らせ（`write-failure-notice`）・`readonly-mode` の「書かない」から漏れる。採る理由が薄い |
| **B 詰めた位置を書く** | 足さない | `chain_realign.rs` で動かした窓だけ `char_pos_entries` → `persist_entries`（動かさなかった本体の下端中央は置き直しで保たれているので書かなくてよい） | 変えない | 無し | `position-persist` 1.9・5.4・`char-position-save-on-exit` 3.2 を改める。**副作用**: 100% で書いた後に 200% で起動すると、絵の幅だけが倍になり組が重なる。新しい決め方（要件 2.2）では重なった組は詰めないので、**一度重なると二度と詰め直されない** |
| **C 書かない・起動の隙間は範囲外** | 足さない | 無し | 変えない | 無し | 拡大率を変えた後に起動し直すと隙間（359px）が開き、新しい決め方（要件 2.1）ではその組は「離れていた」になる。**一度起動し直すと、利用者がドラッグで付け直すまで二度と詰め直されない**＝後退が「同じ起動の中だけ直る」に縮むだけ |

## 4. 案 A の最小のファイルの組（依頼の問いへの答え）

- **sylphya `crates/areka-sylphya/src/persist/mod.rs` は要る**（A1・A2 とも）。記憶の項目は閉じた型 `PersistKey` で、TOML の写し（`apply_entry`・`doc_to_entries`）と正準の鍵（`to_canonical_key`）がこのファイルにある。今ある鍵の値の文字列に幅を詰め込む形は、`get_property` で読む値と `parse_px` を壊すので取れない。加えて **`persist/format.rs` も要る**（`FormatDoc` の表）。③ `readonly-mode` が `persist/mod.rs` で触るのは「差し替えの口」（書かない形への入れ替え・`io.rs` の側）で、型の一覧とは別の所＝**意味の上では当たらず、同じファイルの文字の当たり（後に着地する側の取り込み）だけ**。足した鍵も同じ書く道を通るので、`readonly-mode` の「書かない」に自然に覆われる。`format-version` は上げない（上げると古い areka が全部を捨てる）。
- **`placement/follow/drag_follow.rs` は要らない**。ドラッグの確定（248〜249 行目）は `persist.rs` の `char_pos_entries` → `persist_entries` を呼ぶだけなので、幅（または印）は `persist_entries` の中で窓の位置の鍵を見て足せる。組の相手（`sticky_follow.rs` 299 行目）・並べ終えた時点・並べ直し（`reset_pos.rs`）も同じ口を通る。代わりに `char_pos_entries` の引数を増やす形にすると `drag_follow.rs` と `sticky_follow.rs` を直すことになるので、取らない方が約束の内に収まる（`persist_entries` が黙って鍵を足す形になることは、設計で説明に残す）。
- **戻す側に ⑥ の一覧の外の 2 ファイルが要る**: `main.rs`（`restore_merged_placements` が幅か印も返す・数行）と `ghost_session.rs`（窓の一式へ入れる・数行）。C11 のほかの spec でこの 2 つを名指しする所は無く、④ は「`main.rs` に触らない」と約束している。
- まとめると、案 A1／A2 の最小の組は「⑥ の一覧（`persist.rs`・`spawn.rs`・`drain_resnap.rs`・`chain_*.rs`）＋ sylphya の `persist/mod.rs`・`persist/format.rs` とそのテスト＋ `main.rs`・`ghost_session.rs`」。**C11 の約束をまたぐのは sylphya の `persist/mod.rs`（③ のファイル）1 つだけで、`descript-window-keys` のファイルはまたがない**。requirements.md の Boundary Context の「`drag_follow.rs` に手が入る」は、`persist_entries` の口を使えば当たらない。
- 記憶の形を足さずに ⑥ の内で済むのは A3（推定）だけ。

## 5. 規模と危険

- **規模: M**（9〜12 タスク）。内訳の見立て: 判定の関数とテスト 2・直前の控え 1・外す理由（組・ドラッグ中・武装の後に動かされた・取り出せない）1〜2・記録 1・件 9 の置き直しと wintf の読み 2・§8 と説明の書き直し 1・決定論のテストの新しいファイル 1〜2・実機 1。案 A1／A2 なら +3〜5（記憶の型と TOML・書き足し・戻す道・起動の隣り合わせ・古い記憶の書き足し）で **M〜L**。brief の上限 20 の内。
- **危険: 中**。判定と控えは既存の型と決定論の土台（`FrameHarness`）の上に乗るので低いが、⑴ ドラッグ中の拡大率の変化は wintf の `WM_DPICHANGED`・areka の大きさの書き直し・カーソルの移動の 3 つの順に頼る ⑵ 案 A は sylphya の記憶の形を足す（③ と同じファイル・ほかのゴーストの記憶ファイルの読み戻しとの両立）⑶ ぱすたの出し直し（台本の `\![move]`）と解決の順は実機でしか確かめにくい。

## 6. 設計へ持ち越す調べ（Research Needed）

1. **ドラッグ中に画面をまたいだときの順**: `WM_DPICHANGED`（`window_proc/window_pos.rs` が勧めの矩形へ置く）→ areka の拡大率の相（`dpi.rs` が下端中央を保って大きさを書く）→ 次の `WM_MOUSEMOVE`（`mouse_move.rs`）・`on_char_drag`（`drag_follow.rs`）の並び。始めの値の置き直しをどこに置けば 1 フレーム遅れずに済むか（「1 フレーム遅らせて解く形は取らない」）。
2. **ドラッグ中の窓に拡大率の置き直しが当たるか**: 置き直し（下端中央を保つ）とドラッグの書き込みが同じフレームで競るか。ぱすたは areka が書く側なので、置き直しの直後の `on_char_drag` で上書きされる見込み。
3. **待ち札で窓ごとにフレームが割れる遷移**での控えの正しさ（S1 の「最初の武装の時点」で全部の窓がまだ書かれていないか）。画面をまたぐドラッグでは、動いた窓だけ拡大率が変わり、ほかの窓は変わらない。
4. **T2 を取るなら**: World の変化の時刻が `emo2_frame_system` の中で武装と `\![move]` の間に進むか（bevy_ecs 0.19 の `ComponentTicks`）。
5. **許す幅の値**: 拡大率の置き直しの丸め（幅の半分の整数割り）で出る ±1px と、利用者がドラッグで付けたときのずれ。実機で 200%↔100%↔150% を測る。
6. **3 体以上の「組」の定義**（番号の続く 2 つか、隣にいる番号の小さい窓か）と `extra-character-windows` の想定。
7. **案 A を取るなら**: 新しい鍵の名前と、鏡像（`get_property`）から読めるようになることの扱い（areka の名前空間なので正典とは当たらない見込み）。古い areka が新しい表を読み飛ばすこと（`read_axis_map` は知っている表だけを読む）の確かめ。A1 の「書いた時機が違う 2 つの記録」の食い違いを、相手の記録も同じ束で書く形で消せるか。
8. **`window-work-area-clamp` との順**: 記憶から戻した位置の作業領域への寄せ（`project_restore`）の後に起動の隣り合わせを当てると、寄せた位置から隣り合わせることになる（要件 6.6 は寄せの規則を変えない）。

## 7. 要件の話し合いへの論点（設計の決め）

1. **要件 6 の案**（A1 幅／A2 印／A3 推定／B／C）。4 節のとおり、A1・A2 で C11 の約束をまたぐのは sylphya の `persist/mod.rs`（③ のファイル・意味は当たらない）だけで、`drag_follow.rs` はまたがない。代わりに `main.rs`・`ghost_session.rs` が要る。B と C は「一度起動し直すと組が重なった／離れたで固まる」副作用を持つ（3.5 節）。
2. **要件 1.2 の「重ならずに」**: 丸めで 1px 重なる組や、利用者がドラッグで数 px 重ねて付けた組が「重なっていた」として外れる。「隙間または重なりの幅が許す幅以内なら隣り合い・それを超える重なりが要件 2.2 の重なり」と読み替えるか。
3. **件 9 の直し方**（D1 置き直し／D2 式で縮める）。ぱすたの位置は `drag_follow.rs` の式で決まるので、`mouse_move.rs` だけでは件 9 が直らない。D1 なら ⑥ の内（`dpi.rs` か `dispatch.rs`＋`mouse_move.rs`）で済む見込み。requirements.md の Boundary Context の「件 9＝`mouse_move.rs`・`dispatch.rs`」は、D1 を取るなら `emo2_boot/frame/dpi.rs` の置き直しを含めて読む。
4. **3 体以上の組の定義**（要件 1.5）。
5. **判定を分けるか**（J1 新しい関数／J2 `finalize_chain` を広げる）。J1 なら起動の並べ直しの決まりを変えずに済む。
6. **「武装の後に台本が置き直した」の見分け**（T1 位置／T2 印の時刻／T3 新しい印）。T1・T2 は ⑥ の内。
7. **要件 7.2 と今の走査の「1 つ欠けたら全体を見送る」の違い**: 大きさが揃うのを待つ理由（待つ）と、窓が無いなど取り出せない理由（その窓だけ外して続ける）を分けるか。
8. **今あるテストの意図の書き直し**: `a_dragged_scope_stays_out_of_the_realign` は新しい決め方では「離れていた」で通る。ドラッグで隣に付け直した組は詰めることになるので、テストの名前と主張を改める（陳腐化したテストの扱い）。

---

# 設計の段の調べと決め（2026-10-23・`/kiro-spec-design -y`）

## Summary

- **Feature**: `dpi-realign-remembered-chain`
- **Discovery Scope**: Extension（既存の「武装 → 一度きりの解決」の詰め直しと、記憶の形の拡張）。軽い調べ（`design-discovery-light.md`）を当て、外の文書の調べは要らなかった（新しい依存なし）。調べの下請け（サブエージェント）は使わず、主の文脈でコードを読んだ。
- **使った決まりと手引き**: スキル `kiro-spec-design` の `design-principles.md`・`design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`、型 `.kiro/settings/templates/specs/design.md`・`research.md`、steering の `structure.md`（新しいテストは兄弟のファイル・1 ファイル 1,000 行以下）・`logging.md`（ログの水準と構造化の欄）。
- **Key Findings**:
  - ゴースト窓は `DpiSuggestedRectPolicy::ExternalAuthority` を持つ（`placement/spawn.rs` 616 行目）ので、`WM_DPICHANGED` の勧めの矩形は窓に書かれない。拡大率の相が書く前の `WindowPos` は変わる直前の位置と大きさで、控え（S1）を相の中で取れる。
  - 位置の書き手 `enqueue_window_set_pos` は `WindowPos` をその場で書き換える（`follow_window_move_tests.rs` の `enqueue_window_set_pos_none_updates_position_leaves_size`）。`persist_entries` が呼ばれた時点の `WindowPos` は画面の並びなので、隣り合いの印を書く時点の並びから決められる（A2 が成り立つ）。
  - wndproc のドラッグの式（`mouse_move.rs`）が読むスレッドの控えの `Dragging` には `entity` があり、`world_borrow.world()` も読める。ドラッグ対象の `DraggingState` を読む形にすれば、areka が `DraggingState` を置き直すだけで 2 つの式（wintf と `drag_follow.rs`）が揃う（`dispatch.rs` は直さずに済む）。
  - bevy_ecs 0.19 の部品の差し込みの口（`#[component(on_insert = …)]`）は付け直しでも呼ばれ、同じ排他 system の中の変化の時刻の比べ（T2）と違ってフレームの中の順に頼らない。リポジトリには `spawn.rs` の `on_remove` ほか同じ書き方の先例がある。

## Research Log

### 変わる直前の矩形の取り方（6 節の 1・3）
- **Context**: 要件 1.1 は変わる直前の隣り合いで決める。武装は最初のキャラ窓を書いた**後**に起きる。
- **Findings**: 拡大率の相の第 2 巡で、最初のキャラ窓を処理する直前に全キャラ窓の矩形を控えれば、その巡で書かれる窓も含めて変わる前の矩形が揃う。待ちの札で窓ごとにフレームが割れても、まだ書かれていない窓は書かれるまで変わる前の位置にいる（`ExternalAuthority` で勧めの矩形も書かれない）。2 度目の武装は最初の控えを保てばよい。
- **Implications**: 控えは `ChainRealignPending.before` に持つ。窓の一式を閉じるときに資源ごと外れるので `app_exit.rs` に手が要らない。

### 「武装の後に台本が置いた」の見分け（6 節の 4）
- **Context**: 要件 3.1。印 `ScriptPlaced` は時刻を持たない。
- **Findings**: 武装（拡大率の相）と `\![move]`（台本の移動の相）は同じ排他 system の中の同じ巡で走りうるので、World の変化の時刻（T2）では前後を分けられない。位置で見る形（T1）は、作業領域の変化で下端が動く主な場面（主の画面の拡大率を変えるとタスクバーの物理の高さが変わる）で「動かされた」に倒れ、詰め直しが空振りする。新しい印を移動の側に足す形（T3）は `move_cue_place.rs`・`sticky_follow.rs` に手が要る。
- **Implications**: `ScriptPlaced` の定義に差し込みの口を付け、武装中なら窓を `ChainRealignPending.script_moved` に積む（T3 の働きを、触る所を増やさずに得る）。

### ドラッグ中に画面をまたいだときの順（6 節の 1・2）
- **Findings**: 窓の大きさは拡大率の相の `reconcile_window_size`（と報告の回収の経路）で書かれる。その呼び出しの中で `DraggingState.initial_inset` を置き直せば、次の `WM_MOUSEMOVE`（wintf の式）と `on_char_drag`／`on_char_drag_end`（`drag_follow.rs` の式）がどちらも新しい掴んだ点を使う。大きさの書き込みと同じ巡で位置まで書き直すと、拡大率の相のキャラ窓の書き込みを 1 巡 1 回とする約束を破るので取らない。
- **Implications**: 件 9 は `dpi.rs` と `mouse_move.rs` の 2 か所で直る。`drag_follow.rs`・`dispatch.rs` は触らない。

### 記憶の印の鍵（6 節の 7）
- **Findings**: 正準の鍵の逆読み（正準の文字列から `PersistKey` へ戻す所）は無く、`areka.window.scope(N).*` の子を数で読む読み手も無い。`AxisPair` は `balloon-offset` と共有なので、印は別の表 `[window-adjacent]` に置くのが他を動かさない。前の areka は知らない表を読み飛ばし、自分が書くときに落とす（`format-version` を上げないので位置は失われない）。
- **Implications**: 正準 `areka.window.scope(ID).adjacent`・TOML `[window-adjacent]`・値 `left`／`right`／`none`。

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| J1 改（採る） | 判定 `judge_adjacency` と解き `resolve_chain` を純関数 1 組にし、3 か所の呼び手（拡大率の後・起動・記憶）が組ごとの決めを作って渡す。`finalize_chain` は約束を変えずに解きの上へ載せ直す | 起動の並べ直しの決まりを変えない。同じ規則を 3 か所で使い回せる（要件 6.3 の「同じ決め方」）。組ごとの決めが値で出るのでログとテストが素直 | `chain_finalize.rs` が 330 → 500 行ほどに増える | J1（新しい関数）と J2（`finalize_chain` を広げる）の間。`ScopeChainState` の欄は足さない（505 行のテストの字面を壊さない） |
| J2 | `finalize_chain` に欄を足して 1 関数で両方 | 関数が 1 つ | 起動と拡大率の意味が混ざり、`ScopeChainState` の字面の変更が 505 行のテストに及ぶ | 採らない |

## Design Decisions

### Decision: 判定の作り方（7 節の論点 5）
- **Selected**: J1 改（上の表）。新しいモジュールは作らず `chain_finalize.rs` に置く（`placement/mod.rs` に手を入れない＝C11 の約束の内）。

### Decision: 控えの取り方（3.2 節）
- **Alternatives**: S1 武装のときに全キャラ窓の矩形を控える／S2 前の大きさだけ控えて今の下端中央から戻す。
- **Selected**: S1（拡大率の相の第 2 巡の最初のキャラ窓の前に 1 度）。
- **Rationale**: S2 は下端中央が保たれる前提に頼り、ドラッグで画面を移った窓で成り立たない。

### Decision: 「武装の後に台本が置き直した」の見分け（3.3 節・論点 6）
- **Alternatives**: T1 位置／T2 印の変化の時刻／T3 新しい印。
- **Selected**: `ScriptPlaced` の差し込みの口で武装中の差し込みを記録する（T3 の働き・触る所は `chain_finalize.rs`・`chain_realign.rs` だけ）。
- **Trade-offs**: 印の定義（`chain_finalize`）が受け口（`chain_realign`）を 1 つ指す上向きの参照ができる。判定の純関数はこれに依らない。

### Decision: 件 9 の直し方（3.4 節・論点 3）
- **Selected**: D1。置き直しは `emo2_boot/frame/dpi.rs` の `reconcile_window_size`（キャラ窓の腕）。wintf は `mouse_move.rs` だけを直し、`DraggingState` を正本として読む。`dispatch.rs` は直さない（要件の Boundary Context の触る所より狭い）。
- **Rationale**: D2 は `drag_follow.rs`（`descript-window-keys` の持ち物）に手が要る。

### Decision: 要件 6 の記憶の形（3.5 節・論点 1）
- **Alternatives**: A1 覚えたときの幅／A2 隣り合っていた印。
- **Selected**: A2。スコープ n の鍵に組 (n−1, n) の印 `left`／`right`／`none` を、位置を書く束へ足す（触れる組 k と k+1）。
- **Rationale**: A1 は書いた時機が違う 2 つの記録を比べる。200% で隣り合って覚え、100% で詰め直した後（記憶は書かない）に本体を相方の隣へドラッグし直すと、本体の記録は 100% の幅、相方の記録は 200% の位置と幅のままで、覚えたときの矩形どうしは隣り合わず（例: 191 px 離れて読める）、次の起動で隙間が開く。A2 は書く時点の画面の並びで決めるので、利用者が最後に見た並びを表す。
- **Trade-offs**: スコープ k を書くと k+1 の印も書く（書く時機は同じ・位置の値は書かない＝要件の「書く時機は足さない」の内）。

### Decision: 許す幅（6 節の 5・論点 2）
- **Selected**: `ADJACENT_TOLERANCE_PX = 2`（物理 px）。隙間と重なりの両側に効く（丸めの 1 px の重なりを隣り合いに含める＝要件 1.2）。
- **Rationale**: 下端中央を保つ置き直しで幅の半分の整数割りが窓ごとに最大 1 px ずれ、2 つの窓の分で 2 px。控え・印は詰め直しの後の並び（隙間 0）から取ることが多いので、実際の誤差はもっと小さい。実機（8.7）で測り直す。

### Decision: 3 体以上の組（6 節の 6・論点 4）
- **Selected**: 番号の続く 2 つ（n−1 と n）。本体から昇順に、n の基準は n−1 の解いた後の x。
- **Rationale**: 要件 1.5 の「前の窓を詰め直した後の位置を次の組の基準にする」と、起動の並べ直しの鎖（`finalize_chain`）と同じ形。星形の並びは詰めない（`extra-character-windows` の既定の並べは鎖で当たらない）。

### Decision: 要件 7.2 と待ちの扱い（論点 7）
- **Selected**: 「取り出せない」は控え（変わる直前の位置と大きさ）に無い窓で、その窓に触れる組だけを外して `warn!` し、ほかを続ける。今の走査の「揃うまで全体を待つ」（窓寸の未着・待ちの札・まだ表示されていない）は待ちであって取り出せないではないので、起動と共有の `collect_chain_states` の意味を変えない。

### Decision: 外す理由の及ぶ範囲
- **Selected**: 組・台本は「その窓を動かさない」だけ（n−1 がその窓でも、隣り合っていた n は n−1 の今の位置へ寄せる＝要件 1.1）。ドラッグは要件 2.4 が寄せることも禁じるので両側の組を外す。控えに無い窓は判定できないので両側の組を外す。

### Decision: 今あるテストの書き直し（論点 8）
- **Selected**: `a_dragged_scope_stays_out_of_the_realign` を「変わる前にドラッグで離して置いた窓は動かない」に改め、陽性の対に「ドラッグで隣に付け直した窓は詰まる」を足す。ドラッグの最中の除外は新しいテストのファイルが持つ。並べ終えた時点の保存の「x・y だけ」の主張は印が加わるので改める。

## Synthesis Outcomes

- **一般化**: 拡大率の後の詰め直し（要件 1〜3）・起動の隣り合わせ（要件 6.3〜6.5）・記憶に書く印（要件 6.2）は、どれも「組 (n−1, n) の並びを決め、本体から順に詰める」の変種。判定と解きを 1 組にし、違いは組ごとの決めを作る呼び手の糊に閉じた。今の `finalize_chain` もその上の 1 つの決め方（既定の位置のまま＝左に詰める）として載せ直した。
- **作るか使うか**: 新しい依存なし。部品の差し込みの口は bevy_ecs の既存の機能で、リポジトリに先例がある。記憶は sylphya の既存の鍵族の形（閉じた型＋TOML の写し）に沿う。
- **削ったもの**: 新しいモジュール（`placement/mod.rs` の変更）・`dispatch.rs` の変更・`drag/mod.rs` の欄の追加・新しい移動の印・ドラッグの最中の位置の上書き・控えの別資源（`ChainRealignPending` に持たせた）。

## Risks & Mitigations

- 武装から解決までの数フレームでドラッグを始めて終えた窓は寄せ戻されうる — 現実には起きにくいので手当てせず、実機で見る（design の ChainRealign の注記）。
- 画面をまたいだ直後、待ちの札で大きさの書き込みが見送られているあいだに落とすと、掴んだ点が落とした点から外れる — 待ちは数フレーム。実機の確かめ（8.7 ⑵）で見る。
- `readonly-mode` と `persist/mod.rs` の文字の当たり — 意味は当たらない（向こうは書かない形への差し替えの口）。後に着地する側が取り込む。`format-version` は上げない。
- 前の areka で開くと印の表が落ちる — 次にこの areka で起動したとき古い記憶の道（6.5）で印が書き足される。

## 設計の話し合いの決め（2026-10-23・開発者「承認。全設計議題を推奨案で確定して」）

- **議題 1（台本の「窓の位置を戻す」）**: `\![execute,resetwindowpos]` は `reset_pos.rs` で印 `ScriptPlaced` を外すので、差し込みの口では「変わった後に台本が置いた窓」として拾えない。`chain_realign::note_script_moved` を足して `reset_pos.rs` から 1 行で呼び、要件 3.1 を塞ぐ（範囲外と書き残す案は採らない）。`reset_pos.rs` は C11 のどの spec も名指ししていない。
- **議題 2（起きにくい外れ 3 つ）**: 武装から解決までのあいだに始めて終えたドラッグ・画面をまたいだ直後の次のカーソルの移動まで・待ちの札のあいだに落とした時、の 3 つは受け入れ、実機の確かめ（要件 8.7）で見る。直すには `drag_follow.rs`（触らない約束）か wintf にドラッグの始まりを知らせる口を足すか、位置の書き手の「1 巡 1 回」の約束を破ることになる。目に見えて起きたら `/kiro-discovery` で起票する。
- **検証の指摘の自明な直し**: `alert.rs` の `remembered_text`（`PersistKey` の全種を名指しする `match`）に印の鍵を「窓の位置」として足す・テストの分かれ道 6 つを足す・約束の節に `readonly-mode` への知らせと `vocab/dotted.rs` に触れないことを書く。
