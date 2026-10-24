# 設計の検証: dpi-realign-remembered-chain

- 対象: `design.md`（2026-10-23 生成）・`requirements.md`（確定）・`research.md`
- 判定: **GO**（下の 3 点は設計の話し合いで直す。どれも形の作り直しは要らない）

## まとめ

設計は、詰め直しの対象を「変わる直前に隣り合っていたか」で決め直し、判定と解きを純関数 1 つずつ（`judge_adjacency`・`resolve_chain`）にまとめて、拡大率の後・起動・記憶の 3 か所で使い回す形になっている。今の「武装 → 1 度だけ解く」の往復・呼び順・窓の位置の書き手 1 つ・記憶の書き手 `persist_entries` をそのまま保つので、今の造りによく合う。気になるのは、ファイルの一覧から 1 つ漏れていてそのままでは組み上がらないこと、要件 3.1 の「台本の移動」のうち 1 つが記録に入らないこと、判定の分かれ道のテストが一部書かれていないことの 3 点。

## コードで確かめたこと

| 確かめたこと | 結果 | 根拠 |
|---|---|---|
| bevy 0.19 で、同じ部品を既に持つ窓へ `ScriptPlaced` を差し込み直したときも `on_insert` が走るか | 走る | `bevy_ecs-0.19.1/src/lifecycle.rs` 20 行目「Triggered when a component is added to an entity, regardless of whether it already had it」。`move_cue_place.rs` 178 行目は動かすたびに `insert(ScriptPlaced)` する |
| `enqueue_window_set_pos` が `WindowPos` をその場で書き換え、`persist_entries` の時点で画面の並びが読めるか | 読める | `placement/follow/window_move.rs` 717 行目付近で `WindowPos.position`（大きさを渡すときは `size` も）を書く。ドラッグの確定は `drag_follow.rs` 189 行目で窓を書いてから 249 行目で記憶へ送る。組の相手も `sticky_follow.rs` で動かしてから送る |
| ドラッグの始めの値の置き直しが、次の位置の計算より前に効くか（1 フレーム遅れないか） | 効く | 大きさを書く同じ呼び出しの中で `DraggingState.initial_inset` を置き直す設計。ECS 側の位置の式 `policy_mapped_position`（`drag_follow.rs` 276 行目）は毎回 `DraggingState` を読む。wintf の wndproc 側（`mouse_move.rs` 302 行目）は今はスレッドの控えを読むので、設計どおり `DraggingState` を読む形へ直せば同じ値になる。`DraggingState` と控えは `dispatch.rs` 157 行目で同じ値から入る |
| 並走ウェーブの約束（`run_move_drain_phase`・`PositionCue`／`MoveCueSink`・`transition_judge.rs`・`follow/{window_move,drag_follow}.rs`・`placement/{config,measure}.rs`・`emo2_boot` の結線の列に触らない） | 守っている | Boundary Commitments と File Structure Plan に挙がっていない。`drain_resnap.rs` で触るのは `finalize_chain_once_with` だけ |
| 1,000 行に近いファイルに足さないか | 足さない | `main.rs` は 978 行で、集める処理を `persist.rs`（673 行）へ移すとしている。新しいテストはどれも兄弟の新しいファイル |
| ログの無い失敗の道を作らないか | おおむね作らない | 控えに無い窓・読めない矩形・知らない語はどれも `warn!` が決まっている |

## 大事な問題（3 点）

### 問題 1: `PersistKey` に種類を足すと `alert.rs` が組み上がらない（ファイルの一覧の漏れ）

- **何が問題か**: `crates/areka/src/alert.rs` の `remembered_text`（110〜118 行目）は `PersistKey` の全部の種類を並べて分けている（「それ以外」の腕が無い）。`PersistKey::WindowAdjacent` を足すと、ここがコンパイルの誤りになる。設計の File Structure Plan と Modified Files に `alert.rs` が無い。
- **なぜ困るか**: 書けなかったときに利用者へ出す語（「窓の位置」など）を新しい鍵に決めないまま実装に入ると、タスクの段で境界の外のファイルを急に触ることになる。あわせて、`areka-sylphya/src/persist/mod.rs` はウェーブ C11 の表では `readonly-mode` だけの持ち物（「約束: なし」）で、`mcp-scope-rect-property` もそれを理由に触らない約束をしている。設計は「後に着地する側が取り込む」と書いているが、向こうのセッションはこの重なりを知らない。
- **直し方**: Modified Files に `crates/areka/src/alert.rs`（`WindowAdjacent` を「窓の位置」の腕へ入れる）と、そのテストを足す。`readonly-mode` のセッションへ「`persist/mod.rs` の `PersistKey`・`to_canonical_key`・`apply_entry`・`doc_to_entries` に 1 種足す（向こうが触る `save_scope` の差し替えの口とは別の所）」と伝えることを、設計の「並走ウェーブとの約束」の節に書く。新しい鍵は位置と同じ束で送るので、`write_failure_sites_tests.rs` の 13 種の表は変わらないことも書き添える。
- **要件**: 6.2・7.2（ログ無しの失敗を作らない）
- **設計の箇所**: File Structure Plan／Modified Files／Optional Sections「並走ウェーブ（C11）との約束」

### 問題 2: 台本の `\![execute,resetwindowpos]` が「武装の後に台本が置いた窓」の記録に入らない

- **何が問題か**: 武装の後に台本が置いた窓の記録は、`ScriptPlaced` の差し込みの知らせだけで取る設計になっている。ところが `\![execute,resetwindowpos]`（`PositionCue::ResetWindowPos` → `placement/reset_pos.rs`）は台本の指示で全部の窓を既定の位置へ並べ直すのに、`ScriptPlaced` を差し込まず、逆に外す（`reset_pos.rs` 118 行目）。武装から解決までのあいだにこの指示が来ると、変わる直前の控えで「右に隣り合う」だった組を、台本が並べた位置から動かしてしまう。
- **なぜ困るか**: 要件 3.1 は「台本の移動の指示（`\![move]` など）」で動かした窓を上書きしないとしている。起きる幅は数フレームと狭いが、判定の分かれ道に穴が残り、テストでも固められない。同じ形で、武装から解決までのあいだに利用者がドラッグを始めて落とし終えた窓も記録に入らない（設計は「起きにくい」として手当てしないと書いている）。
- **直し方**: どちらかに決めて設計に書く。(a) `chain_realign` に「台本が置いた窓を記録する」小さな関数を公開し、`reset_pos.rs` の並べ直しで動かした窓ごとに呼ぶ（`reset_pos.rs` はこのウェーブの誰の持ち物でもない）。(b) 並べ直しの指示は 3.1 の対象の外と決め、理由と一緒に要件の解釈として残す。ドラッグを始めて落とし終えた窓の扱いも、手当てしない理由を 7.1 のログで見分けられること（組の行の語）と合わせて書く。
- **要件**: 3.1・2.4・8.2
- **設計の箇所**: ChainRealign「組 (n−1, n) の決め」の表・Implementation Notes の「危うさ」

### 問題 3: 判定の分かれ道のうち、テストの計画に無いものがある

- **何が問題か**: 決定論のテストで全部の判定の分かれ道を固める決まりに対して、Testing Strategy に次の道が挙がっていない。
  1. `rescale_drag_grab` の「前の大きさの縦か横が 0 以下なら置き直さない」の道。
  2. `persist_entries` で印を決めるとき「矩形が読めない窓の組は印を書かず `warn!` を出し、位置は書く」の道。
  3. 同じ束に同じ印の鍵が 2 度出たときに 1 つにまとめる道。
  4. 武装していないときの `ScriptPlaced` の差し込み（資源が無い）で何も起きないこと（記録のテストの「武装の前」に含まれるかを明記していない）。
  5. 拡大率の相のバルーン窓の腕で、ドラッグ中でも掴んだ点を置き直さないこと。
  6. 控えを取るとき、位置か大きさの欠けた窓を控えに入れない道（解決の側の `Unreadable` のテストはあるが、控えを取る側の分け方が無い）。
- **なぜ困るか**: タスクの段で抜けると、組み上がっても判定の 1 つが檻の外に残る（「正しく動くことと固定されていることは別」）。
- **直し方**: 上の 6 つをそれぞれ置き場（`frame_dpi_drag_grab_tests.rs`・`persist_adjacency_tests.rs`・`chain_realign_script_moved_tests.rs`・`frame_chain_realign_exclusion_tests.rs`）付きで Testing Strategy に足す。
- **要件**: 8.2・8.5・8.6
- **設計の箇所**: Testing Strategy

## よい所

1. **判定の核を 1 つにまとめている**: 隣り合いの判定と詰めの解きを純関数にし、拡大率の後・起動の並べ直し・記憶の印の 3 か所が同じものを使う。今の `finalize_chain` も約束を変えずにその上へ載せ直すので、今のテスト（`chain_finalize_tests.rs`）がそのまま檻になる。「起動と実行中で決め方がずれる」ことが起きにくい形。
2. **今の造りを壊さずに足している**: 武装と解決の 1 往復・待ちの扱い・呼び順・窓の位置の書き手 1 つ・記憶の書き手 1 つ・記憶の版をどれも変えず、控え・印・置き直しをどれも同じフレームの中で効く形にしている。並走ウェーブの約束（`run_move_drain_phase` などに触らない）も守り、`ScriptPlaced` の知らせを印の定義の側に付けて `move_cue_place.rs`・`sticky_follow.rs` に触らずに済ませている。

## 判定: GO

- **理由**: 3 点はどれも設計の文の書き足し（ファイルの一覧・1 つの記録の口・テストの行）で直り、判定の核・状態の持ち方・データの形を作り直す必要は無い。bevy の差し込みの知らせ・`WindowPos` の書き換えの時機・ドラッグの始めの値の置き直しの時機という、設計が頼っている前提はコードで確かめて成り立っていた。
- **次にすること**:
  1. 設計の話し合い（`/kiro-design-discussion dpi-realign-remembered-chain`）で上の 3 点を議題にし、設計に書き足す。問題 2 は (a) か (b) のどちらにするかを決める。
  2. 書き足した後にタスクを作る（`/kiro-spec-tasks dpi-realign-remembered-chain`）。
  3. `readonly-mode` のセッションへ `persist/mod.rs` の重なりを伝える（問題 1）。
