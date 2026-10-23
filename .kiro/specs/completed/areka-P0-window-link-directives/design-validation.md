# 設計の検証: areka-P0-window-link-directives

- 検証日: 2026-10-22
- 対象: `design.md`（基準 `5f0cf0ae`）・`requirements.md`・`research.md`・steering
- やり方: `kiro-validate-design` の手順（分析 → 重大な問題 → 良い点 → 判定）。対話なしで判定まで出した。設計が言っているコードの事実は、下の「コードで確かめたこと」のとおり実物で確かめた。

## まとめ

設計は今ある部品（`MoveCueSink` の 1 本の線・`Anchored`／`project_anchor`／`resize_window_to`・単一の書き手 `enqueue_window_set_pos`）にそのまま乗っていて、新しい経路の語も新しい線も足さない。層の向き（`emo2_boot → placement`）、1,000 行の目安、兄弟のテストファイルの決まりも守っている。要件 1〜10 はすべて部品と口に結びついている。ただし、ドラッグでの連れ動きに 2 か所、ぱすた（2 つの窓がどちらも `free`）でそのまま表に出る穴がある。どちらも直す範囲が小さく、設計の討議で決めれば足りるので、判定は GO（下の 2 点を討議で決めることが条件）とする。

## コードで確かめたこと

| 設計の言い分 | 結果 | どこで |
|---|---|---|
| wintf は `DragConfig.move_window` をドラッグの始めにだけ読み、同じ回に `OnDragStart` を呼ぶ | 合っている。`Started` の腕で `move_window` を読んで控え、その後で `OnDragStart` を配る。ドラッグ中は控えた値（`DragStateSnapshot::Dragging.move_window`）で窓を動かす | `wintf/src/ecs/drag/dispatch.rs:135-208`・`window_proc/mouse_move.rs:258-313` |
| `DraggingState.initial_inset` はドラッグを始めたときの窓の位置 | 合っている（名前は古いままで、中身は始めの窓の位置）。今の `policy_mapped_position` も同じ意味で使っている | `dispatch.rs:155-160`・`drag_follow.rs:247-272` |
| `resize_window_to(…, MoveCue)` は、今の経路の検査で止められたり鳴ったりしない | 合っている。`MoveCue` は可視の見張りの外・既定の位置の追従の外（`is_system_reanchor` が偽）・整合待ちの監視の外（`deferral_covers_route` が偽）。`resize_window_to` の中には経路で断る所が無い | `window_move.rs:180-365`・`:507-557`・`:640-700`・`diag.rs:296-318` |
| `close_windows_for_restart` で資源を外せば、ゴーストの切り替え・読み直しで帳簿が消える | 合っている。今も窓と一緒に `GhostWindows`・重なりの連鎖の資源などを外している。2 行足すだけで済む | `app_exit.rs:188-202` |
| kanade の SHIORI リソースの許可表は 13 名 | 合っている（`switchmovetodefaultpositionbutton.caption` が最後）。1 名足して 14 名 | `areka-kanade/src/schedule/resources.rs:38-68` |
| `drag_follow.rs` は 1,000 行の目安の内に収まる | 今 937 行。足すのは呼び出し数行（950 行ほどの見込み）で、目安の内に収まる。ほかに手を入れるファイル（`placement/mod.rs` 915 行・`ghost_session.rs` 910 行）も足すのは数行 | `wc -l` |
| `seriko.sticky-window` の生の値を運ぶ欄 | もう在る（`PlacementConfig.sticky_window_raw`）。設計の「`mod.rs` に足す」は、`PreparedPlacement` まで運ぶ所だけで足りる | `placement/config.rs:106-135` |
| 正しい形の `set`／`reset` の荷物は `ZOrderCueSink` の警告に当たらない | 合っている（`_` の腕で `debug!`）。警告は開けない形の荷物にだけ出る | `emo2_boot/zorder_cue.rs:96-140` |
| 近道の受け手が窓の手続きの中から `OnResetWindowPos` を出しても固まらない | 合っている。`raise_reset_window_pos` は送って答えの受け手を置くだけで、答えは次の tick で読む（待たない） | `menu/reset_pos_frame.rs:72-97` |

## 重大な問題

### 重大な問題 1: ドラッグの終わりに `DraggingState` が無いと、組の相手の位置が記憶に残らない

- **問題**: 設計は、ドラッグした窓の始めの位置を毎回 `DraggingState.initial_inset` から読み、読めないときは「相手を動かさず `warn!`」とする（System Flows「ドラッグでの連れ動き」・`finish_char_drag`）。ところが今のコードの注記は、ドラッグの終わりに `DraggingState` がもう無いことが「多窓のとき実機で起きた」と書き、そのためにドラッグした窓の保存を今の `WindowPos` で行う逃げ道を持っている（`drag_follow.rs:155-176`）。設計どおりだと、その場面ではドラッグした窓だけが記憶に書かれ、相手の最終位置は書かれない。
- **影響**: ぱすたは 2 つの窓の組なので、まさに多窓の場面に当たる。相手の記憶が書かれないと、次の起動で窓の枠とぱすたが離れて出る（要件 4.5・9.7 の ④ に反する）。
- **直し方**: `begin_char_drag` でドラッグした窓自身の始めの位置も `CharDragSession` に控え、途中と終わりの量は控えから取る（`DraggingState` に頼らない）。始めの位置は `OnDragStart` の時点の `WindowPos.position`（wintf が `initial_inset` に写したのと同じ値）でよい。`DraggingState` が無い場面のテストを 1 本足す。
- **要件**: 4.1・4.5・9.7
- **設計の場所**: 「ドラッグでの連れ動き」の注（`DraggingState.initial_inset` の段落）・`sticky_follow` の `follow_partners_on_drag`／`finish_char_drag`・`CharDragSession` の形

### 重大な問題 2: 2 つとも `free` の組では、ドラッグ中に相手が 1 フレーム以上遅れて付いてくる

- **問題**: `free` の窓は、wintf が `WM_MOUSEMOVE` の中で窓をすぐ動かす（`move_window=true`）。一方、組の相手は次のフレームの `OnDrag` で `enqueue_window_set_pos` の書き込みの列に積まれ、その後で動く。ぱすたは 2 つの窓がどちらも `free`（研究 1.7 の調べ 5）なので、ドラッグ中はぱすたが先に動き、窓の枠が後から追う。今の `free` の窓のバルーンも同じ仕組みで遅れているが、ぴったり重ねた窓の枠とぱすたでは、ずれがはっきり見える。設計の「危険」の節にも実機の確かめの項目にも、この点が無い。
- **影響**: 要件 4.1 の「組の窓どうしの位置の関係を保つ」が、ドラッグの途中で崩れて見える。v0.1.0 のゴール「ぱすたが SSP と同じレベルで動く」の見た目に直に効く。
- **直し方**（討議で選ぶ）: 推奨は案 A。組に入っている窓は、向きに関わらず `DragConfig.move_window=false` にし、ドラッグした窓も相手も areka が同じフレームで書く（今の吸着した窓と同じ道。`free` は `project_anchor` が恒等なので `policy_mapped_position` がそのまま使える）。`on_char_drag` の分かれ目は、ドラッグの始めに控えた「wintf が動かすか」で決める。組を作る・解く・descript の組を取り替えるたびに、`move_window` を付け直す。案 B は、遅れを受け入れて「危険」の節と COMPAT §8 に書き、実機の確かめに「ドラッグ中のずれ」の項目を足す。
- **要件**: 4.1・9.7
- **設計の場所**: Architecture「吸着」・「ドラッグでの連れ動き」・`sticky_follow` のドラッグの段・Risks・Testing Strategy「実機」

## 良い点

- **既にある道に乗り、新しい道を作らない。** 受け口は今の 1 本の線に相乗りする。これで同じ台本の `\![set,sticky-window]` → `\![move]` の並びが画面の側でも保たれる。書き込みの経路の語は `MoveCue` と「経路なし」だけで、`window_move.rs`・`diag.rs`・`chain_realign.rs`・`work_area_sync.rs` に触れない。経路の語ごとに見張りと監視の掛かり方をコードで確かめたうえで選んでいて（`AnchorChange` を避けた理由が `debug_assert` の実物にある）、思い込みの設計になっていない。
- **寿命と記憶の形を、窓に付く部品で構造から満たしている。** `AnchorSource { descript, by_script }` は窓と一緒に消えるので、「ゴースト終了まで有効」「シェルの切り替えでは台本の分を保つ」「`default` で descript へ戻す」が別々の場合分けを要らずに成り立つ。記憶へ書く形を descript の向きに揃える規則（`saved_form_anchor`）で、台本で向きを変えた窓が次の起動で半分の幅ずれる穴（要件 4.8）も、ドラッグの終わり・組の相手・並べ終えた時点の 3 か所で同じに塞いでいる。

## 判定

- **判定: GO**（重大な問題 1・2 を設計の討議で決めて design.md に入れることが条件）
- **理由**: 層の向き・経路の語・1,000 行の目安・寿命の扱いに食い違いは無く、設計の言うコードの事実は実物と合っていた。2 つの穴はどちらもドラッグでの連れ動きの中に閉じていて、直す範囲が `sticky_follow.rs` と `drag_follow.rs` の数行で済む。
- **次にすること**:
  1. 設計の討議（`/kiro-design-discussion areka-P0-window-link-directives`）で、問題 1（始めの位置を控えに持つ）と問題 2（案 A／案 B）を決め、design.md に入れる。
  2. その後でタスクを作る（`/kiro-spec-tasks areka-P0-window-link-directives`）。問題 1 のテスト（`DraggingState` が無いドラッグの終わり）と、選んだ案の確かめ（案 A なら組の窓の `move_window` の付け直し・案 B なら実機の項目）をタスクに入れる。

## 気になったが重大ではない点（参考）

- 設計の「`placement/mod.rs` に `sticky_window_raw` を足す」は、欄がもう `PlacementConfig` に在るので、`PreparedPlacement` まで運ぶ所だけで足りる。
- ドラッグ中に台本が向きを変えた場合、離したときの記憶は当て直しの前の位置で書かれる。次の起動は descript の向きで読み戻すので害は無いが、COMPAT §8 の行に一言あると後の spec が迷わない。
- 拡大率の変化での詰め直しが重ねた 2 つの窓を並べ直すこと（裁定 7）は、`areka-P0-dpi-realign-remembered-chain` が来るまで残る。実機の確かめでは拡大率を変えないこと。
