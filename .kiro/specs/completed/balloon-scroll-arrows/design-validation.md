# 設計の検証: balloon-scroll-arrows

- 検証日: 2026-10-30
- 対象: `design.md`（2026-10-30 生成・コミット `99b5a4c4c`）・`requirements.md`（承認済み）・`research.md`
- 基準: `.claude/skills/kiro-validate-design/rules/design-review.md`（分析 → 重大な問題 → 良い点 → GO/NO-GO）
- 照合したコード: `main` `484263230` を基にしたこのワークツリー

## 設計の検証のまとめ

設計は今の作りによく乗っている。見える範囲を決める所を 1 か所（`present_actor` の `visible_window` の呼び出し）で差し替え、送りの描き直し（`ScrollPlanner` の blit）と選択肢の当たり（`committed`）は今の仕組みのまま合わせる。今の定義を書く所も 1 つ（`select_balloon_face`）に集める。要件 1〜11 は全部、部品と口へ対応が付いている。ただし、印の面の重なりの順（z 順）の書き方が wintf の `Children` の並びの決まりと食い違っていて、箱の面を挿す位置の計算も壊す。ここは実装の前に直す必要がある。重大な問題は 3 件で、どれも設計の形を変えずに直せる。

### コードとの照合（設計の要の主張）

| 主張 | 結果 | 根拠 |
|------|------|------|
| 見える範囲を決めるのは `present_actor` の 1 か所 | ○ | 本番の呼び出しは `crates/areka-emo-text/src/actor_present.rs:275` だけ（他は examples・tests・test_support） |
| 選択肢の当たりは `ScrollState.committed` に従う | ○ | `actor_present.rs:328` で `render.executor.scroll_state().committed` を `to_window_physical` へ渡す。箱の字の矩形（`glyph_cells`）も同じ値を使う |
| 送りの描き直しは向きに依らない | ○ | `viewbox_diff.rs` の `exposure_band` は blit の符号で上端・下端のどちらが空くかを決める。古い側へ戻す blit もそのまま扱える |
| `balloon_models` の書き手を `select_balloon_face` の 1 つにできる | ○ | 今の書き手は `frame/attach.rs:442` と `frame/switch.rs:544`（`finish_balloon`）の 2 か所で、設計はどちらも `select_balloon_face` を通す形に替える |
| 相の順は drain → 文字層スケール → 文字 | ○ | `emo2_boot/frame.rs` の 336 行（`run_drain_phase`）→ 402 行（`run_text_scale_phase`）→ 403 行（`run_text_phase`）。`refresh_actor_binding` はモデルが違えば作り直す（`actor_attach.rs:324` 以降）。1 フレームの遅れは生じない |
| 族の表 `ARROW_FAMILY` で要件 2.1 の順になる | ○ | `areka-emo-present/src/balloon.rs` の `prefix_chain` と `SeriesFamily`（`scope0_legacy` の長さは可変）。`face_digits_of` は `arrows0` を `arrow` の面と取り違えない |
| 普通のバルーンの窓へ子の面を「文字の面の直後」に挿すと「文字より手前」になる | **×** | 重大な問題 1 |
| 普通のバルーンの窓のホイールは誰も読んでいない | ○ | `char_watch.rs` は `CharWindowMarker` の窓だけを拾う。wintf の `WM_MOUSEWHEEL` は窓の entity へ回転を積む（`mouse_dblclick_wheel.rs:228`）ので、同じ読み方で拾える |

## 重大な問題

🔴 **重大な問題 1**: 印の面の重なりの順が、wintf の `Children` の並びの決まりと逆で、箱の面を挿す位置の計算も壊す

**懸念**: wintf では `Children` の**先頭が一番手前**になる（`wintf/src/ecs/graphics/systems/visual_sync.rs:122-130` の `InsertAtBottom`、`areka-emo-present/src/mount.rs:163-186` の「text-layer slot を先に足す＝最上」）。普通のバルーンの窓の子は `[文字の差し込み口, バルーンの絵]` の順なので、設計の言う「予約スロットの直後」（添字 1）に挿すと、矢印は**文字より奥**・バルーンの絵より手前になる。MarkerSurface の節が書く「バルーンの絵より手前・文字より手前」と食い違う。箱はもっと危うい。`ActorBox::box_child_index`（`actor_box.rs:465-479`）は、差し込み口の後ろには箱の文字の面しか無いとして `slot_at + 1 + (element番号の大きい箱の数)` を数える。箱ごとの印の面を窓の子に足すと、この数がずれる。すると、後から挿す箱の文字の面が、ほかの箱の文字の面と印の面のあいだに割り込む。
**影響**: 矢印が文字の下に隠れる。または、箱どうしの重なりの順が要件 3.7・3.9（完了 spec の箱の重なりの順）から外れる。どちらも決定論のテストでは見えにくく、実機で初めて出る。
**提案**: (a) 普通のバルーンの印の面を、差し込み口の**前**（添字 = 差し込み口の位置）へ挿して「文字より手前」にするか、文字より奥でよいと決めて、表の言い方を直す。SSP の測り ⑸ の絵で、矢印と文字のどちらが手前かも見て決める。(b) 箱は、`box_child_index` が印の面も数える形に直す。たとえば「箱 1 つ＝文字の面＋印の面の 2 枚」と数え、印の面は自分の箱の文字の面のすぐ前か後ろへ置く。これを設計の Modified Files（`actor_box.rs`）に足す。(c) 普通のバルーン・箱のどちらも、子の並びを装着の檻（`marker_surface_tests.rs`）で固定する。危うさの節の「並びは装着の檻で固定する」を箱にも広げる。
**追跡**: 要件 3.5・7.2・3.6（`dump_balloon` の重ねの順も同じ決めに従う）
**根拠**: design.md「MarkerSurface（GPU）」の「並び」・「危うさ」・「Modified Files」（`actor_box.rs` の行に `box_child_index` が無い）

🔴 **重大な問題 2**: 「新しい文字で手動を捨てる」と見える範囲を決める順が書かれておらず、1 フレーム遅れになりうる

**懸念**: `present_actor` では、`arrange_lines`（見える字が決まる）→ `visible_window`（見える範囲）の順に呼ぶ（`actor_present.rs:259-275`）。設計は「`visible_window` を `window_for` に替える 1 か所」と「字の数の観測の 1 行」を別々に書くだけで、どちらを先に呼ぶかを決めていない。`window_for` の口も、字の数を受け取らない。観測を `window_for` の後に置くと、新しい字が出たコマはまだ手動の位置で描かれ（新しい字は画面の外）、最新の行へ戻るのは次のコマになる。
**影響**: 開発者の原則「1 フレーム遅らせる解は取らない」に反する。要件 8.1 の「見える字が増えた時に戻る」も、1 コマずれる。話している最中は、毎コマこのずれが起きうる。
**提案**: `observe_visible(count)` を `window_for` より先に、同じコマの中で呼ぶと決める。そのためには、`window_for` の口に字の数を渡して中で先に観測するか、`present_actor` での呼ぶ順を契約として書く。`scroll_marker_tests.rs`（または `actor_scroll_tests.rs`）に「字が増えたそのコマの `VisibleWindow` が自動の位置に等しい」ことを確かめる檻を 1 本足す。
**追跡**: 要件 8.1・3.8・11.1
**根拠**: design.md「Modified Files」の `actor_present.rs` の行・「ScrollMarker（純粋）」の「新しい文字」と `window_for` の署名・「Testing Strategy」の「純粋な判断」

🔴 **重大な問題 3**: 面の切り替えでバルーンの窓を動かす（FaceWindowPosition）は、SSP で測らずに配置の振る舞いを足すことになる

**懸念**: 要件 1.7 の「すべての鍵」を `windowposition.*` にも当て、`\b[ID]` で面が替わるたびに、動かしていないバルーンを新しい面の標準の位置へ置き直す。これは配置の新しい振る舞いで、`placement` に新しいファイルと規則（「今のずらしが前の材料の標準のずらしと等しいスコープだけ動かす」）を足す。ところが、SSP がこう動くかは要件 10.1 の 6 点に入っていない。設計自身も Open Questions 1・2 に挙げ、バルーンの切り替え（`finish_balloon`）でも今は入れていない値が配置に入ると書いている。
**影響**: 正典が黙る所を測らずに決めることになり、開発者の方針「ukadoc で読めない点は SSP の MCP で測る」と合わない。台本が面を替えるたびに窓が動く副作用は、利用者に見える。並走・待機の spec（`placement-measure-bake-once`・`placement-crate-split` ほか）とも同じ材料（`DefaultPlacementBasis`）を取り合う。
**提案**: 設計の討議で次のどちらかに決める。(a) 測りの点に ⑺「`\b[ID]` で面ごとの `windowposition` が違うとき、SSP がバルーンを動かすか」を足し、答えが出るまでは配置の差し替えを入れない（文字の層と矢印だけを面ごとにする）。(b) 設計どおり進め、測りの答えで残すか外すかを決める。どちらにしても、`finish_balloon` の新しい `windowposition` の扱い（Open Question 2）は同じ決めに従わせ、§8 に登記する。
**追跡**: 要件 1.7・10.1・10.3・9.5
**根拠**: design.md「FaceWindowPosition（`placement/face_windowposition.rs`）」・「Open Questions」1・2・「危うさ」

## 設計の良い点

1. **差し込む所が 1 か所ずつで、今の仕組みをそのまま使う**: 手動の位置は `VisibleWindow` を替えるだけで、`choice.rs`・`ScrollPlanner`・時間切れの数え方に触らない。面の切り替えは「一次の情報源（presenter の `current_surface_id`）を drain の後に 1 回読む」プッシュの形で、今の唯一の権威 `refresh_actor_binding` に作り直しを任せる。第 2 の再結線の道を作らず、同じコマで効く（照合済み）。
2. **印の土台が下流の spec の乗り口として出来ている**: `SeriesFamily` の可変長の旧名・`select_faces` と同じ番号ごとの選び・`MarkerCoord`（`--数値`）・`MarkerPair` を emo-present に置き、依存の向き（emo-present は emo-text を知らない）を保つ。クリック待ち（`CLICKWAIT_FAMILY`）の順も決定論のテストで示す。当たりは α ≥ 0.5 の画素だけで、記録も読めない値・欠けた絵・壊れた絵の全部に `warn!`／`debug!` が決まっていて、ログ無しの失敗経路が無い。

## 最終の判断

- **判断: GO**
- **理由**: 要件はすべて対応が付き、層の境界と依存の向きは今の作りに合っている。重大な問題 3 件は、重なりの順の決め・呼ぶ順の契約・配置の振る舞いの範囲の決めで、設計の形（部品の分け方と口）を変えずに直せる。
- **次の一歩**:
  1. 設計の討議（`/kiro-design-discussion balloon-scroll-arrows`）で、重大な問題 1〜3 と Open Questions 1〜4 を決め、design.md を直す。
  2. 問題 1 は、普通のバルーンと箱のそれぞれで印の面の添字の決め方を設計に書き、`actor_box.rs`（`box_child_index`）を Modified Files に足す。
  3. 問題 2 は、`observe_visible` → `window_for` の順を口か契約で固定し、同じコマで戻る檻を Testing Strategy に足す。
  4. 直した後に `/kiro-spec-tasks balloon-scroll-arrows` へ進む。

### 小さな気付き（重大ではない）

- MarkerSurface の「当たり」の行が「（議題 8）」を根拠に挙げているが、要件の議題 8 は面ごとの設定ファイルの話。根拠の番号を直す。
- `PlaceScroll` の限りが、State の節では `0 ≤ line ≤ auto_first`、状態の図の注では `0 ≤ line < auto_first` と書かれている。「`auto_first` に達したら自動へ戻る」という同じ意味なので、書き方をそろえる。
