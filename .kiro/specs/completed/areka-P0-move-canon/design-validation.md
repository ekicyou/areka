# 設計の検証: areka-P0-move-canon

- 対象: `design.md`（2026-10-17 生成）・`requirements.md`・`research.md`
- 方法: 設計の主張を今のコードで突き合わせた（下の「コードで確かめたこと」）。SSP と areka は動かしていない。

## 総評

設計は、読み（純粋）→ 計算（純粋）→ UI での適用 → フレームの順 → メニュー、と依存の向きがはっきりしており、窓の書込口・吸い付きの射影・配置の規則・記憶の書き方を「呼ぶだけ」にとどめていて、今の作りとよく噛み合う。要件 1〜10 はすべて部品とテストに割り当てられている。ただし、並べ直しの位置が emo2 で起動の位置とずれる点（要件 8.6）と、実機の確かめの手順が今の areka で実行できない点（要件 9.4）の 2 つは、タスクを作る前に設計の討議で決めておく必要がある。

## 重大な指摘

### 指摘 1: 「シェル位置初期化」の位置が、emo2 では起動の位置から横にずれる

- **問題**: 並べ直し（D7）は配置の規則を「今の窓の大きさ」で 1 度だけ解く。起動は「採寸した面 0 の幅で解く → 表示の面の幅で下端の中央を保って置き直す → 連鎖を直す（先頭は動かさない）」の 3 段で決まる。表示の面の幅が採寸の幅と違うと、先頭の窓は起動では右端から「幅の差の半分」だけ左に立つが、並べ直しでは右端にぴったり付く。設計自身も「Open Questions」でこれを認めている。
- **影響**: これは起こりにくい場合の話ではない。`placement/chain_finalize.rs` の冒頭の注記にある実測は、まさに emo2・拡大率 200% で「採寸の幅 868・表示の幅 764・差の半分 52px」である。本 spec の実機の確かめ（emo2・100% と 200%）で、並べ直した窓が起動の位置から 52px 右（相方も同じだけ）にずれ、要件 8.6「起動で並べるのと同じ既定の位置」を満たさない見込みが高い。並べ直しの位置はそのまま記憶へ書かれるので（要件 8.8）、次の起動からもずれた位置に出る。
- **提案**: 討議でどちらかを決める。(a) 起動と同じ 3 段を再現する: `DefaultPlacementBasis` に配置の設定と一緒に採寸の大きさ（`ScopeInput` の列）と採寸の DPI を持たせ、`resolve_placement` → 下端の中央を保って今の幅へ置き直す → `finalize_chain`（どれも今ある純粋な関数）の順で解く（拡大率が起動時から変わっていれば採寸の大きさを今の DPI へ換算する）。(b) ずれを受け入れる: 要件 8.6 の読みを「起動と同じ規則を今の窓の大きさで解いた位置」へ改め、COMPAT §8 に行を足す。areka の作りに合うのは (a)（起動と並べ直しで規則の持ち主が 1 つのまま）。
- **対応する要件**: 8.6・8.8
- **設計の該当箇所**: 「ResetPos（`placement/reset_pos.rs`）」の手順 2、「Open Questions / Risks」の「並べ直しの位置（D7）」、research.md「Decision: D7」

### 指摘 2: 実機の確かめの手順が、今の areka の MCP では実行できない

- **問題**: 「Testing Strategy > 実機」は、「エモの位置調整」の選択を areka の MCP の `raise_event` で起こし、`get_log` で `move_applied` の行を読む、としている。ところが areka の `raise_event` はまだ中身が無く、`NG:not implemented yet` を返すだけである（`crates/areka/src/mcp/raise_event.rs`。中身を作るのは未着手の `mcp-kanade-tools`・roadmap にも「今も not implemented yet を返すのは `sakurascript`・`raise_event`・`reload`」とある）。また `get_log` が返すのは error・script・network・update・status の 5 種の記録だけで、`tracing` の `info!(event = "move_applied")` の行がそこへ入る作りは設計に無い。
- **影響**: 要件 9.4（拡大率 100% と 200% の実機の証跡）と要件 6.1・6.2 の判定が、実装の段の最後で手順ごと止まる。v0.0.3 の利用者がまず踏む不具合を直す spec なので、最後の確かめで行き詰まると着地が遅れる。
- **提案**: 手順を今ある道具だけで閉じる形に書き直す。たとえば、(1) 実際の右クリックのメニューとバルーンの選択肢を、開発者の許可のうえで SendInput で選ぶ（areka の窓はクリックを透過するので、computer-use の右クリックは届かない。これは baseware-root-list の実機で使った手と同じ）、(2) 判定は `target\` の下の根で `RUST_LOG` を `move_applied`・`reset_window_pos`・`reset_pos_menu_outcome` が出る水準まで開けて取ったログファイルの行で行う、(3) `\0` の窓の矩形も同じログ（`move_applied` の `from_*`／`to_*` と基準の矩形）から読めるよう、`move_applied` に基準の矩形の欄を足す。`raise_event` を使いたいなら `mcp-kanade-tools` の着地を先にする依存を roadmap に書く必要があるが、v0.0.3 の順番を変えることになるので勧めない。
- **対応する要件**: 9.4・9.5・6.1・6.2
- **設計の該当箇所**: 「Testing Strategy > 実機（9.4・9.5）」、「MovePlace」の手順 6（`move_applied` の欄）

## 補足（重大ではないが、タスクの段で拾うとよいこと）

- **相の順（D4）が守れるのは「同じフレームに両方が届いた」ときだけ**: 面の切り替えの指令と `\![move]` は別々の線で届く。台本で `\s[…]` の直後に `\![move]` があり、フレームの drain（面）と move drain の間に 2 つが割って届くと、移動が古い大きさで計算される。今の作りにもある隙間で、起こる幅はごく狭い。`frame_move_order_tests.rs` の注記に「この隙間は扱わない」と書いておけば足りる。
- **`move_window_to` の注記が古くなる**: 「唯一の消費者は `\![move]`」と書いてあるが、連鎖の確定がすでに呼んでおり、本 spec で並べ直しも呼ぶ。本 spec は `window_move.rs` に触らない約束なので、直すかどうかは討議で決める（直すなら注記 1 行だけ・1,000 行の例外表のファイルなので行数は増やさない）。

## 設計の良いところ

1. **SSP 2.9.18 の実測を、計算の式とテストの入力にまで落としている**: 参照点の省略が `left.top` であること・吸い付いた軸は台本の値を使わないこと・使えない組み合わせの置き換えなど、正典が黙っている点をすべて実測の番号（M1〜M10）で裏付け、`move_cue_geometry_tests.rs` の入力にそのまま使う。emo2 の位置調整の検算（`x1 = x0 − w1`・`y1 = y0 + h0 − h1`、拡大率に依らない）と、旧い emo2 の着地が今の式と同じであることの検算も書いてあり、後戻りしにくい。
2. **今の部品を作り変えずに、正しい場所で呼ぶ形になっている**: 吸い付きは `project_anchor`（ドラッグと大きさの変化が使うのと同じ射影）を適用の側で 1 度通すだけ、並べ直しは連鎖の確定と同じ「`move_window_to` → `set_default_char_pos` → `persist_unremembered_char_positions`」の 3 手、メニューは葉の枠と `try_recv` の前例どおり。`window_move.rs`（1,221 行・例外表）と `main.rs`（1,000 行ちょうど）に触らず、683 行の `move_cue.rs` は兄弟のファイルへ出して痩せさせる、と行数の決まりも守っている。

## 判定

**GO（条件つき）**

- **理由**: 層の分け方・依存の向き・既存の部品との接し方に根本の食い違いは無く、要件はすべて部品とテストへ割り当てられている。指摘 1・2 はどちらも設計の討議で決めて design.md の該当の節を直せば片づき、作りの組み替えは要らない。
- **次の手順**:
  1. 設計の討議（`/kiro-design-discussion areka-P0-move-canon`）で指摘 1（並べ直しの位置の決め方）と指摘 2（実機の確かめの手順）を決め、design.md を改める。
  2. その後に `/kiro-spec-tasks areka-P0-move-canon` でタスクを作る。

## コードで確かめたこと（2026-10-17・この作業ツリー）

| 設計の主張 | 確かめた結果 |
|---|---|
| `project_anchor(anchor, raw, size, snapshot)` と `Anchored(pub Anchor)` が `placement/follow/anchor.rs` にある・`Free` は素通し・`Bottom` は `BottomSnapPolicy` へ委ねる | 合っている（anchor.rs:104・:181） |
| `move_window_to(world, window, x, y) -> bool` は経路 `MoveCue` で書き、既定の位置を自分では運ばない | 合っている（window_move.rs:45。既定の位置を運ぶのはシステム由来の経路だけ＝`diag.rs` の `is_system_reanchor`） |
| 連鎖の確定が `move_window_to` → `set_default_char_pos` の前例 | 合っている（`frame/drain_resnap.rs:371` 付近・`chain_realign.rs` の注記） |
| `primary_monitor` は今は私有で、`pub(crate)` にする | 合っている（placement/mod.rs:847 は `fn`） |
| `work_area_for_window` は中心で決め、外れたら最近傍 | 合っている（follow/work_area.rs:78 とそのテスト） |
| フレームの相の順は reconcile → move drain → zorder drain → resnap → finalize | 合っている（frame.rs:350〜374）。resnap は presenter の面の大きさを同じフレームで読む |
| 消費者の台帳は 24 件・`execute` は今 `install` だけ | 合っている（consumer_ledger.rs:731・:402） |
| kanade の許可表はイベント 53・リソース 12 | 合っている（events_change_tests.rs:41・resources.rs:168） |
| `RaiseOutcome` は `NotAllowed`・`NotSteady`・`Script`・`NoReply`・`Failed` の 5 つ | 合っている（areka-kanade/src/change.rs:111） |
| メニューの枠は 10 種・組 0 は Ghost・Shell・Balloon | 合っている（menu/mod.rs:72・menu/plan.rs:76） |
| 行数: `move_cue.rs` 683・`window_move.rs` 1,221・`main.rs` 1,000・`consumer_ledger.rs` 943・`placement/mod.rs` 902・`ghost_session.rs` 878 | 合っている。足す行は数行ずつで 1,000 を超えない |
| `MoveDirective` を外で使うのは線の型とテストだけ | 合っている（`frame/wiring.rs`・`frame.rs`・`mod.rs` と各テストの `mpsc::channel::<MoveDirective>()`） |
| 実機の確かめで areka の MCP の `raise_event` を使う | **使えない**（mcp/raise_event.rs は `NG:not implemented yet` を返す）→ 指摘 2 |
| 並べ直しは起動と同じ位置 | **emo2 ではずれる**（chain_finalize.rs:10〜13 の実測 868 と 764）→ 指摘 1 |
