# ギャップ分析: areka-P0-mouse-drag-events

> 2026-10-04 `/kiro-validate-gap`。調べた実物は作業ブランチ `claude/mouse-drag-events-456cbe`（main `e2a373b5` の上の `f26b5256`）。ソースは「何の定義か」で指す（行番号は使わない）。

## 1. 要約

- **既にあるもの**: ドラッグの開始と終了の知らせは wintf に揃っている（累積器 → 配り手 → `OnDragStart`／`OnDragEnd` の部品）。kanade 側は `OnMouseDoubleClick` の経路（知らせの型 `MouseEventKind`・組み立て `on_mouse_double_click`・振り分け `steady::on_mouse`・送ってよい表 `ALLOWED_EVENT_IDS`）がそのまま型紙になる。areka 側も座標から当たり判定を引く `resolve_hit_owned` とスコープを引く `char_scope` がある。
- **足りないもの**: `MouseEventKind` の 2 つの腕・組み立ての 2 関数・表の 2 行・`on_mouse` の 2 つの腕・areka のキャラクター窓の `OnDragStart`（どこにも付いていない）と、`OnDragEnd` を「位置の保存 → 知らせ」の包みに差し替えること・網羅の台帳の 2 行とそれに連動する文書。
- **確かめたかった点の答え**: ⑴ 取り消しの `DragEndEvent` が運ぶ位置は**押した位置（画面の物理 px）**であって、取り消された時点の位置ではない（要件 4.2 の字面と食い違う＝議題 1）。⑵ 本番で `start_dragging` を呼ぶのは WM_MOUSEMOVE の閾値の枝だけで、そこでは必ず直後に開始の知らせを積む（`start_dragging` 自身は積まない＝テストで直に呼ぶと開始は出ない）。⑶ `steady.rs`（929 行）・`schedule/mod.rs`（937 行）は本 spec の変更では 1,000 行を超えない見込み。⑷ 台帳の 2 行を「担当は本 spec」にすると、網羅の検査が `roadmap-draft.md` への行の追加と `briefing.md` の数の書き換えまで求める（要件 7.2 の範囲より広い＝議題 4）。
- **推す道筋**: brief の推奨どおり、既存の経路を伸ばす案 A（kanade は既存ファイルへ足す・areka は新しい `input_events/drag.rs` に包みを置く）。規模は S（6〜9 タスク）、危険度は低〜中（取り消しの位置の読み方と、実機でドラッグ中に面が替わるときの配置の振る舞いが残る不確かさ）。

## 2. 今の姿

### 2.1 wintf（ドラッグの仕組み・本 spec では触らない）

- **流れ**: 窓の手続き（WM_LBUTTONDOWN）が `DragConfig` を持つ祖先を探して「準備中」にする → WM_MOUSEMOVE が閾値（既定 5 px・距離の二乗で比べる）を越えたら `start_dragging` で状態を進め、**同じ枝で**累積器へ開始の知らせ（`DragTransition::Started`・押した位置）を積む → 毎フレームの `dispatch_drag_events` が累積器を空にして、積んだ順に開始・終了を配る。
- **開始の知らせ**: `DragStartEvent.position` は**押した位置**（画面の物理 px）。配る前に `DraggingState`（開始時の窓位置を持つ）を挿す。配った後で「ドラッグ中」へ進める。
- **終了の知らせ**: 離したとき（WM_LBUTTONUP の 2 つの枝）は離した位置。取り消し（ESC・WM_CANCELMODE・WM_ACTIVATE の非活性化・WM_CAPTURECHANGED の 4 か所）は **`end_pos: start_pos`＝押した位置**で `cancelled: true`。
- **開始を伴わない終了は来ない**: 累積器の `set_transition` が「ドラッグ中の対象」が無い終了を捨てる（完了 spec `drag-click-without-move`）。ただし World を借りられなかったときに取り消しの終了を積み損ねる穴が残っており（`drag-cancel-borrow-miss`・C3-② で並走）、そのときは「開始 → 開始」と終了の無い開始が 2 つ続きうる。
- **配り方**: `dispatch_event_for_handler` が同じ知らせを Tunnel（根 → 対象）と Bubble（対象 → 根）の 2 相で呼ぶ。部品は 1 つの窓に 1 つだけ（同じ型の部品を `insert` すると置き換わる）。ドラッグの対象は `DragConfig` を持つキャラクター窓の entity 自身なので、`ev.target == entity` が実の流れでは常に成り立つ。
- **1 フレームに開始と終了が両方積まれる**こともある（速いドラッグ）。そのときも同じ `dispatch_drag_events` の中で開始 → 終了の順に配られる。
- **右ボタン**: `DragConfig` の既定は左ボタンだけ（キャラクター窓もバルーン窓も既定のまま）。右ボタンでは準備に入らない（要件 3.4 は今の仕組みで満たされる）。
- **バルーン窓**: 自分の `DragConfig` と `OnDragEnd(on_balloon_drag_end)` を持つ。キャラクター窓とは別の entity なので、キャラクター窓にだけ付けた受け手には届かない（要件 3.3 は付け先で満たされる）。

### 2.2 areka（受ける側）

- **窓を作る経路**: 本番は `ghost_session.rs` の `prepare_ghost_windows` が組む閉包 1 つ。`spawn_ghost_windows`（`placement::spawn`）の直後に `input_events::attach_char_pointer_handlers` が呼ばれる。起こし直しも同じ閉包を通る。
- **位置の保存**: `placement::spawn` がキャラクター窓に `OnDragEnd(on_char_drag_end)` を付ける。定義は `placement/follow/drag_follow.rs`（`pub(crate)`・`placement::follow` から再輸出）。中で窓の最終位置を決め（非 Free は `project_anchor` で辺へ寄せ直す・Free は窓の手続きが動かした位置）、`enqueue_window_set_pos` で `WindowPos` を書き、`persist_entries` で記憶へ書く。
- **取り消しでは窓が元の位置へ戻る**: 取り消しの終了は押した位置を運ぶので、`on_char_drag_end` の「開始時の窓位置＋（カーソル − 押した位置）」が開始時の窓位置になり、窓はそこへ戻って保存される（今の振る舞い・要件 6 で変えない）。
- **層の決まり**: `placement` は `crate::` を引けない（example が `#[path]` で私有 include する）。よって kanade へ送る包みは `input_events` 側に置き、`on_char_drag_end` を呼んでから送る形になる（brief の結論どおり）。
- **マウスの配線**: `input_events/mod.rs` の `MouseWiring`（NonSend・kanade への `Sender`・当たり判定の供給源 `RegionSource`〔本番 `Presenter`／テスト `Mock`〕・間引き状態）。`resolve_hit_owned` は窓のクライアントの物理 px を受け、当たり判定名と配信空間の座標（縮約後のサーフェス px）を返す。`char_scope` は `CharWindowMarker.scope` を引き、**`scope <= 1` を `debug_assert` する**。
- **画面 px とクライアント px**: WM_MOUSEMOVE は「クライアント px＋`WindowPos.position`」で画面 px を作っている。だから知らせの位置から `WindowPos.position` を引けばクライアント px に戻る（同じ空間の逆算）。
- **シェルの中の箱**（`shell-balloon`）: 早期 return はポインタの移動・押下の受け手の先頭にあるだけで、ドラッグの準備は窓の手続きが別に始める。箱の上から始めたドラッグも窓のドラッグとして配られる（要件 1.3 は特別な手当て無しで満たされる見込み・実機で確かめる）。

### 2.3 kanade（送る側）

- **知らせの型**: `msg.rs` の `MouseInput { scope, x, y, region, kind }`・`MouseEventKind { Move, DoubleClick { button } }`。`KanadeMsg::Mouse` で受け、`actor.rs` が `Input::Mouse` へ写す。
- **振り分け**: `schedule/mod.rs` の横断の腕が Steady のときだけ `steady::on_mouse` へ渡し、他の相では `trace!`（`mouse_input_ignored`・相の名前だけ。**どの入力かは残さない**）で捨てる。`on_mouse` は終了の握手の待ち（`pending_close`）なら `trace!`（`mouse_close_pending`・入力を `?input` で残す）で捨て、それ以外は `state.snapshot()` から `Status` を導いて GET を 1 件出す。
- **往復は一度に 1 つ**: kanade は 1 アクター 1 受信箱で、`drive` の中で SHIORI と同期往復する。受信箱が順番待ちそのもので、マウスのための別の待ち行列は無い（要件 5.5 は同じ経路に載せるだけで満たされる）。
- **送ってよい表**: `events.rs` の `ALLOWED_EVENT_IDS`（46 語・各行に ukadoc の URL 1 行）。`actor.rs` は送る直前に照合し、表に無い名前は `error!`（`event_id_not_allowed`）で内部の失敗にする＝表へ足し忘れるとそのゴーストが故障扱いになる（要件 9.1 ⑼ の赤は自然に立つ）。
- **送る記録**: 送出の直前の `trace!`（`shiori_request`・Method・ID・Reference・Status）。要件 8.1 はこれで満たされる（実機の照合には `kanade=trace` が要る）。
- **応答の扱い**: `value_replaces_active_talk` は `OnSecondChange` 以外の応答で再生中の会話を置き換える。2 つのイベントの応答（クローディアの開始は `\0\s[29]\e`）も、会話の途中なら会話を置き換える（`OnMouseDoubleClick` と同じ）。応答の台詞は `translate-pipeline` の出口で `OnTranslate` を通る（特別な手当て不要）。
- **パッシブモード**: `status.rs` に語があるだけで入る経路が無い（`SEAM(Req6.1/6.3)`）。

### 2.4 網羅の台帳と検査（`crates/ukadoc-survey`）

- 台帳 `doc/ukadoc-coverage/ledger/shiori.toml` の `OnMouseDragStart:1`・`OnMouseDragEnd:1` は `absent`・`owner = ""`・優先度 A12・価値「触れ合い」。実装済みの型紙は `OnMouseDoubleClick:1`（`owner = ""` のまま `implemented`・備考は「壊れ方・ログ・根拠の場所・構築関数・無いと失うもの」）。
- **実装済みには証拠が要る**: ソースの定義箇所に `// ukadoc: <URL>` の 1 行（許可表の要素・分岐の腕・語彙表の行のどれか）。無ければ `ImplementedWithoutEvidence`。表の 2 行に URL を付ければ足りる。
- 報告 `report/shiori.md`・`report/summary.md` は `cargo run -p ukadoc-survey -- report`／`report-summary` で作り直す生成物（状態の件数が 69→71 などへ動く）。
- **台帳の外で数が連動する場所**（`cargo test -p ukadoc-survey` の検査が見る）:
  - `briefing.md` の `[[barrier]]`（ページ `list_shiori_event` の状態の分布・腕 f）: `implemented = 46 → 48`・`absent = 238 → 236`。前例 `translate-pipeline` も同じ書き換えをした。
  - `owner` を本 spec にするなら、`roadmap-draft.md` の `[[spec]]` に本 spec の行（`stage`・`bundle = "撫で"`・`owner_count = 2`・`wave`）が要る（腕 f「宛先は `[[spec]]` か `[[owner_completed]]` のどれか」・腕 c「`owner_count` は数え直しと合う」）。あわせて `[briefs].count`（今 41）・行数の地の文・段階 A の表の「撫で」の行（依存する既存 spec の欄）を直すのが前例（`file-drop`・`shell-balloon-switch`・`sakura-time-critical` の追加の段落）。
  - 完了時には `/kiro-complete` が `briefing.md` の `[[owner_completed]]` へ移す流れになる（腕 e は `completed/` に在ることを見る）。
  - `briefing.md` 7-7 節（段階 A の一般化で壊れる項目の数とヤヤ製テンプレートの表）にも 2 つが「未対応」で載っているが、検査の腕は見ていない（手書きの数）。直すかどうかは前例が無い。

### 2.5 行数（1,000 行の番人は「1,000 を超えたら赤」）

| ファイル | 今 | 見込みの増分 | 備考 |
|---|---:|---:|---|
| `areka-kanade/src/schedule/steady.rs` | 929 | +15〜25（2 つの腕・doc の追記・テストの接続宣言を置くなら +3） | 上限の内。同じ列の後続（`sakura-time-critical` など）の余白は細る |
| `areka-kanade/src/schedule/mod.rs` | 937 | 0〜+3（触らないで済む見込み・テストの接続をここに置くなら +3） | |
| `areka-kanade/src/schedule/events.rs` | 672 | +60〜70（組み立て 2 関数と doc・表 2 行と URL・冒頭の表の 2 行） | |
| `areka-kanade/src/msg.rs` | 902 | +10〜15 | |
| `areka/src/input_events/mod.rs` | 558 | +5〜10（`mod drag;`・部品の挿入・doc） | |
| `areka/src/placement/follow/drag_follow.rs` | 936 | 0 | 触らない |
| `areka-kanade/src/schedule/steady_flow_tests.rs` | 925 | 0 | ここへは足せない＝新しい兄弟へ |
| `areka/src/input_events/input_events_tests.rs` | 890 | 0 | 同上 |
| `areka-kanade/src/schedule/events_tests.rs` | 787 | +5 前後（全語の列挙のテストの 2 語と名前 `..._forty_six_...` の改名） | |

先に分割するタスクは要らない見込み。ただし `steady.rs` に兄弟のテストの接続宣言を置くかどうかで数行動く。

## 3. 要件ごとの対応表

| 要件 | 今ある資産 | ギャップ | 種別 |
|---|---|---|---|
| 1.1 開始を送る | wintf の `DragStartEvent`・`OnDragStart` 部品 | キャラクター窓に `OnDragStart` が無い・`MouseEventKind` の腕・組み立て・表・`on_mouse` の腕が無い | Missing |
| 1.2 1 回だけ | 配りは Tunnel／Bubble の 2 相 | 片方の相でだけ送る規律。`drag-cancel-borrow-miss` の穴で「開始 → 開始」が来たときの扱い（議題 3） | Missing／Unknown |
| 1.3 箱・当たり判定に関わらず | 準備は窓の手続きが始める | 特になし（実機で確かめる） | Constraint |
| 1.4／2.5 間引きしない | 間引きは `plan_and_send_move` だけ | 別の送出口（`send_double_click` と同じ形）を作るだけ | — |
| 2.1 終了を送る | `DragEndEvent`・`on_char_drag_end` | 包み（保存 → 送出）と部品の差し替え | Missing |
| 2.2 取り消しも同じ並び | `cancelled` は区別できる | 位置は押した位置しか来ない（議題 1） | Constraint |
| 2.3 1 回だけ | 同上 | 同上（議題 3） | Missing |
| 2.4 開始より後 | 配り手は積んだ順・`Sender` は FIFO・kanade は 1 受信箱 | 特になし | — |
| 3.1／3.2 動かさないクリック・ダブルクリック | 累積器が開始の無い終了を捨てる | 特になし（2 回目の押下のまま閾値を越えればドラッグになる＝議題 7） | Constraint |
| 3.3 バルーン | 別 entity・別の受け手 | キャラクター窓にだけ付ける | — |
| 3.4 右ボタン | `DragConfig` の既定 | 特になし | — |
| 4.1〜4.7 Reference | `on_mouse_double_click` の 7 個の並び・定数 `REF2_WHEEL_M1`・`REF6_DEVICE_MOUSE` | 組み立て 2 関数（`OnMouseDoubleClick` の左と同じ値） | Missing |
| 4.2 座標の空間 | `resolve_hit_owned`（クライアント px → サーフェス px と当たり判定） | 画面 px からクライアント px へ直す一手（`WindowPos.position` を引く）。終了は保存の後の窓位置で引くか前で引くか（議題 2） | Missing／Unknown |
| 4.4 3 体目以降のスコープ | `char_scope` | `scope <= 1` の `debug_assert`（今は 3 体目を作る経路が無い・議題 6） | Constraint |
| 4.8 見出し | `state.snapshot()` → `Status` | 特になし | — |
| 5.1〜5.5 いつ送るか | `mod.rs` の横断の腕・`on_mouse` の先頭の防御 | 特になし。記録の水準と中身（議題 5） | Constraint |
| 6.1〜6.3 位置の保存 | `on_char_drag_end` | 包みが先に必ず呼ぶ・送り先が無くても呼ぶ。保存が同じことを見るテストの置き場（議題 8） | Missing |
| 7.1 表と URL | 表の各行の URL | 2 行 | Missing |
| 7.2 台帳と報告 | 生成器・検査 | `briefing.md`・`roadmap-draft.md` まで動く（議題 4） | Constraint |
| 7.3 パッシブモードの印 | `status.rs` の SEAM | 送るかを決める場所の印（`on_mouse` の腕の近く） | Missing |
| 8.1 送る記録 | `shiori_request`（trace） | 特になし | — |
| 8.2 捨てる経路の記録 | `mouse_input_ignored`（入力の中身なし）・`mouse_close_pending`・`mouse_*_no_wiring`・`mouse_send_failed` | 非 Steady の捨てで「どのイベントか」が分からない・重なりを捨てる経路は新設なら記録も新設（議題 3・5） | Missing |
| 9.1〜9.2 決定論のテスト | kanade は `step` を直に叩く檻・areka は `follow_drag_end_gate_tests.rs` の Rig（累積器へ積む → `dispatch_drag_events`） | 新しい兄弟のテスト 2 本前後。Rig を共有するか（議題 8） | Missing |
| 9.3 実機 | 検体 `claudia`（`sample-ghost-kit` に登記済み） | 面が替わったときに配置の再計算がドラッグとぶつからないか（Research） | Unknown |
| 9.4 全体テスト | `tools/test-all.ps1` | 特になし | — |

## 4. 実装の道筋

### 案 A: 既存の経路を伸ばす（brief の推奨）

- **kanade**: `msg.rs` の `MouseEventKind` に `DragStart`・`DragEnd` の 2 つ（ボタンは持たない＝左だけ。持たせるなら議題 7 の関連）。`events.rs` に `on_mouse_drag_start`・`on_mouse_drag_end`（`on_mouse_double_click` の左と同じ 7 個）・表の末尾に 2 行と URL・冒頭の表に 2 行。`steady::on_mouse` の `match` に 2 つの腕とパッシブモードの印。`lib.rs` の `pub use` に 2 関数。`events_change_tests.rs` の個数 46→48・`events_tests.rs` の全語の列挙。
- **areka**: 新規 `input_events/drag.rs` に `on_char_drag_start`（Bubble の相だけ・`MouseWiring` が無ければ記録して何もしない・画面 px − `WindowPos.position` → `resolve_hit_owned` → `KanadeMsg::Mouse`）と `on_char_drag_end_and_notify`（Tunnel では何もしない・Bubble で先に `placement::follow::on_char_drag_end` を呼び、その戻り値をそのまま返す・その後で送る）。`attach_char_pointer_handlers` で `OnDragStart` を足し、`OnDragEnd` を包みで置き換える。
- **利点**: 終了の握手の待ち・定常でないときの捨て・`Status`・順番待ちの決まりを 1 つも複製しない。後で `sakura-time-critical` が `on_mouse` の先頭に置く防御にも自動で掛かる。触るファイルは roadmap の C3-④ の一覧と一致する。
- **欠点**: 位置の保存の部品を `placement` が付けた後で `input_events` が上書きする「後から置き換え」の関係が生まれる（付け直しの順を取り違えると保存が消える）。example（`window-placement.rs` など）は包みを通らないが、送り先が無いので困らない。

### 案 B: 汎用の入口（`KanadeMsg::RaiseEvent`）で送る

- areka 側だけで済み、kanade の型を増やさない。
- ただし `on_mouse` の「終了の握手の待ちは送らない」防御を通らず（要件 5.2 を別の場所で作り直すことになる）、`Status` の導き方も別、`sakura-time-critical` の抑えにも掛からない。brief の再測定で退けた案。要件 5.5 と 2.3 の決まりに例外を作る形になるので勧めない。

### 案 C: 位置の保存の受け手（`on_char_drag_end`）の中から送る

- 部品の置き換えが要らない。
- `placement` が `crate::input_events`・kanade へ依存し、層の決まり（example の私有 include）を破る。brief で退けた案。

### 案 A の中の選び分け（混成の余地）

- **包みの置き場**: 新しい `input_events/drag.rs`（推す・`mod.rs` を太らせない・兄弟のテストを置ける）か、`mod.rs` に直接。
- **重なりの見張り**（議題 3）: areka の `MouseWiring` にスコープごとの「開始を送った」印を持たせて、印の無い終了・印のある開始を記録して捨てる案と、wintf の累積器の約束（開始の無い終了は来ない）と `drag-cancel-borrow-miss` の修正に任せて何も持たない案。

## 5. 規模と危険度

- **規模: S**（6〜9 タスク）。既存の型紙が揃っており、新しい仕組みは無い。テストの置き場と台帳まわりの連動の書き換えが手数の大半。
- **危険度: 低〜中**。コードは型紙どおりで低い。中にしているのは、取り消しの位置の読み方（要件の字面と wintf の値の食い違い）と、実機でドラッグ中に面が替わったときの配置の振る舞いという、テストで先に潰しきれない 2 点があるため。

## 6. 議題（要件討議へ渡すもの）

1. **取り消しの終了で Reference0／1 に載せる位置**。wintf は取り消しの終了に押した位置しか載せず、位置の保存はそれを使って窓を開始の位置へ戻している。要件 4.2 は「取り消された時点の位置」と書く。選択肢: ⑴ 押した位置をそのまま使う（窓が戻るので、戻った窓から見た押した場所＝開始と同じ値になる。要件 4.2 の括弧書きを「取り消しのときは押した位置」へ直す）／⑵ 送る時点でカーソルの実位置を OS から読む（areka の入力層に Win32 の読みが入り、テストには差し替えの口が要る）／⑶ wintf の取り消しの終了に実位置を載せる（wintf は本 spec の境界の外・同じウェーブの `drag-cancel-borrow-miss` が同じ `keyboard.rs` を触るので文字の衝突も起きる）。正典は取り消しについて何も書いていない。
2. **終了の座標をどの窓位置から引くか**。⑴ 保存の受け手を呼んだ**後**の `WindowPos.position`（保存の受け手が最終位置を書き込む＝最後の移動の知らせが欠ける穴も埋まった後の位置）から引く／⑵ 呼ぶ前の位置から引く（最後の移動の知らせが来ないことがあるので、離した時点の窓とずれうる）。Free の窓では ⑴ だと押した場所と同じ値になる（窓が指に付いて動くため）。⑴ が自然に見えるが、「保存の後に読む」という順序の決まりを包みに持たせることになる。
3. **重なって届いた知らせの見張りを areka 側に持つか**。今の実の流れで 2 相の呼び出しは Bubble だけ扱えば済み、開始の無い終了は累積器が捨てる。残る穴は `drag-cancel-borrow-miss`（World を借りられないと取り消しの終了が積まれない → 終了の無い開始が 2 つ続く）。⑴ スコープごとに「開始を送った」印を持ち、重なりを記録して捨てる（要件 8.2 の「重ねて届いた知らせを捨てる」経路が実在するようになる）／⑵ 持たない（並走の修正に任せる。要件 8.2 の該当の経路は「無い」と明記する）。要件 1.2・2.3 の「複数の段を通って」が 2 相のことだけを指すなら ⑵ で足りる。
4. **台帳の 2 行の担当（`owner`）を本 spec にするか空にするか**。要件 7.2 は「担当は本 spec」。そうすると網羅の検査が `roadmap-draft.md` に本 spec の `[[spec]]` 行（と `[briefs].count`・地の文・段階 A の表の「撫で」の行）を求め、完了時には `briefing.md` の `[[owner_completed]]` への移しも要る。前例は 2 つある: `OnMouseDoubleClick` は担当を空のまま実装済み、`file-drop`・`translate-pipeline` などは担当を書いて `roadmap-draft.md` に行を足した。どちらにしても `briefing.md` の `[[barrier]]` の 2 つの数（46→48・238→236）は直す必要がある。要件 7.2 の「台帳から作る報告」に `briefing.md`・`roadmap-draft.md` を含めて読むか、要件の文に足すかも決めたい。同じウェーブの `install-companion-reading` も台帳を触るので、`roadmap-draft.md` に行を足すなら `[briefs].count` で文字の衝突が起きうる。あわせて、検査が見ない `briefing.md` 7-7 節の手書きの数（段階 A の一般化で壊れる項目）を直すかどうか。
5. **捨てたときの記録の水準と中身**。今の捨ての記録は `trace!` で、定常でないときの `mouse_input_ignored` は**どの入力かを残さない**。マウスの移動（高頻度）と同じ腕を通るので、2 つのイベントだけ水準を上げる（例: `debug!` か `info!`）なら腕の中で種類を見て分ける必要がある。要件 5.2・5.3・8.2 の「記録に 1 件残す」を trace のままで満たすとみなすか、実機で追えるように種類を足すか（水準は変えずに `kind` を足すだけでも、どのイベントを捨てたかは分かる）。
6. **3 体目以降のスコープ**。要件 4.4 は「3 体目以降はその番号」。今の `char_scope` は `scope <= 1` を `debug_assert` し、3 体目を作る経路もまだ無い。`char_scope` をそのまま使えば今の他のマウスのイベントと同じ扱い（3 体目ができた日に一緒に直す）。要件の文を「今は 0 と 1 だけ」に寄せるか、そのままにして印だけ残すか。
7. **ダブルクリックの 2 回目の押下のまま動かしたとき**。2 回目の押下で閾値を越えると、`OnMouseDoubleClick`（押下で送る）の後にドラッグが始まり `OnMouseDragStart` が出る。要件 3.2 の「ダブルクリックの各押下について送らない」は、閾値を越えない押下のことと読めば今の仕組みのまま。越えた場合も送らないとするなら、押下とドラッグを突き合わせる新しい仕組みが要る（SSP の振る舞いは ukadoc に書かれていない）。
8. **位置の保存が前と同じであることを見るテストの置き場**。`placement/follow_drag_end_gate_tests.rs` に「累積器へ積む → `dispatch_drag_events` → 記憶への書き込みを偽の入出力で拾う」檻（Rig）が既にあるが、そのファイルの私有の型。`input_events` のテストで同じ檻を使うには、`structure.md` の決まり（共有の手助けは `<stem>_test_support.rs` へ・複製しない）に従って切り出すことになる。⑴ Rig を `placement` のテスト用の手助けへ移して共有する（`placement` のテストファイルを触る＝触るファイルが C3-④ の一覧より増える）／⑵ 包みの側は「保存の受け手を呼んだこと」だけを見て、保存の中身の同一性は既存の Rig のテストに任せる（要件 9.1 ⑺ の「前と同じ」を何で示すかが弱くなる）。
9. **COMPAT の裁量の記録**。正典が沈黙している点（取り消しの扱い・取り消しの位置・Reference2 を 0 にすること・座標の空間をサーフェス px にすること）を `doc/COMPAT_ARCHITECTURE.md` §8 の表に足すか。要件には書かれていない。座標の空間は既存の他のマウスのイベントの裁定（collision-dpi-hittest）をそのまま使う。

## 7. 設計で調べること（Research Needed）

- **ドラッグ中に面が替わったときの配置**: クローディアは開始の応答で面 29（相方 19）へ替える。絵の大きさは同じ（333×500）だが、面の切り替えが配置の再計算（下端への寄せ直し・連鎖の並べ直し）を起こし、ドラッグ中の単一の書き手（`on_char_drag`）とぶつからないか。`emo2_boot` の面の切り替え → 窓の寸法・位置の流れを設計で確かめ、実機の確認項目に入れる。
- **開始の時点の `WindowPos.position`**: Free の窓で、閾値を越えてから開始が配られるまでの間に窓の手続きが窓を動かしていないか（「開始を配る前」の状態では動かさない作りに見えるが、設計で確かめる）。動かしていれば開始の座標がずれる。
- **会話の置き換え**: 開始の応答 `\0\s[29]\e` が再生中の会話・選択肢の待ちを置き換える（`OnMouseDoubleClick` と同じ）。ドラッグのたびに会話が切れることを良しとするか、実機で見え方を確かめる（SSP でも新しい応答は会話を切る）。
- **実機の記録の照合**: 送出の記録 `shiori_request` は trace なので、実機は `kanade=trace` まで開ける。areka 側の入口にも記録を置くか（置くなら名前を決める）。
- **`roadmap-draft.md` の `wave` の書き方**: 足すなら roadmap の C3-④ を写すことになるが、前例は `W15`・`B6`・`C4 の候補` とまちまち。

## 8. 設計への申し送り

- 推す道筋は案 A。kanade は既存ファイルへ足すだけ、areka は新しい `input_events/drag.rs` と `attach_char_pointer_handlers` の 2 行。分割のタスクは要らない見込み（`steady.rs` は 950 行前後に収まる）。
- 包みの約束: Tunnel では何もしない・Bubble で必ず先に `on_char_drag_end` を呼び、その戻り値を返す・送り先（`MouseWiring`）が無くても保存は行う・送るのはその後で 1 回。
- 決める必要があるのは議題 1〜9。中でも 1（取り消しの位置）と 4（台帳の担当と連動する文書）は要件の文の直しに繋がりうる。
- 並走: `sakura-time-critical` とは `steady::on_mouse` の同じ関数（向こうは先頭の防御・こちらは `match` の腕）。`balloon-lifecycle-events` とは表の末尾・個数のテスト・`lib.rs` の `pub use`。どちらも同じウェーブには居ない。同じウェーブの `drag-cancel-borrow-miss` とは触るファイルが重ならない（こちらは wintf を触らない）。
