# 設計の検証: audit-emo-text-state-actor

- 日付: 2026-11-06
- 読んだもの: `spec.json`（言語 ja）・`requirements.md`（終わりの「要件の討議で確定した事項」を含む）・`design.md`・`research.md`・`.kiro/steering/`（`roadmap.md` のウェーブ C18 の約束を含む）
- コードで確かめた所: `crates/areka-emo-text/src/` の `actor_present.rs`・`actor_decoration.rs`・`actor_box.rs`・`actor.rs`・`state.rs`・`state_decoration.rs`・`state_route.rs`・`lookahead.rs`・`scroll_marker.rs`・`marker_surface.rs`・`viewbox_draw_render.rs`、`crates/areka/src/emo2_boot/frame/scale_text.rs`・`crates/areka/src/mcp/dump_balloon_gpu_tests.rs`・`crates/areka/src/emo2_boot/balloon_visibility*.rs`、`crates/wintf/src/ecs/graphics/systems/init.rs`（どれも読んだだけで、書き換えていない）

## まとめ

設計は 30 件の指摘のそれぞれに、直す場所・直し方・壊れたら落ちるテスト・範囲の外への波及を書いており、要件の 1〜8 との対応表も抜けが無い。`crates/areka` が使う公開の形を変えないという約束は、コードで確かめた範囲では守られている。ただ、新しく足す 2 つの「覚え」（作れなかった面の覚えと、前のフレームの配置の覚え）は、どちらも「いつ覚えを捨てるか」の書き方に穴が 1 つずつある。どちらも設計の中の数行で直せるので、判定は GO（下の 2 点を設計の討議で直すことが条件）。

## 大事な問題（2 件）

### 問題 1: 作れなかった面の覚えが、装置と関係の無い一時の失敗まで覚えてしまい、文字が戻らなくなることがある

- **何が問題か**: 設計は「面を作るのが `Err` なら覚えて、入力（物理寸と位置）が変わるか、`GraphicsCore` が作り直されたら消す」と書く（「面の上限と作れない面の覚え」・System Flows の `Build -->|Err| Note2`）。ところが今のコードでは、面を作る前に `WucGraphicsResource` の Compositor が無い場合（`actor_present.rs:273-285`）と、`GraphicsCore` が無いか無効な場合（`:286-298`）にも `Err` を返す。`wintf` の `init_wuc_resource`（`crates/wintf/src/ecs/graphics/systems/init.rs:151-188`）は、`GraphicsCore` とは別に WUC の資源を作り直し、作れなかったら次の回にもう一度試す。このとき `GraphicsCore` の変化の印は立たない。
- **困る場面**: 装置を作り直した直後のフレームで、`GraphicsCore` はもう新しいのに WUC の資源がまだ無い（または作るのに 1 度失敗した）とする。文字の層は覚えを消して作り直しを試し、Compositor が無いので失敗し、また覚える。次の回に WUC の資源ができても、`GraphicsCore` の印はもう動かないので覚えは消えない。その場所の文字は、拡大率か位置が変わるまで出なくなる。今のコードは毎フレーム試し直すので、この場面でも文字は戻る。つまり今より悪くなる。
- **直し方の案**: 覚えるのは、入力で答えが決まる失敗だけにする。具体的には、物理寸が上限を越えたとき（面を作らない）と、`TextSurface::attach`／`attach_window_child`／印の面の `attach` が、その寸で作るのに失敗したときだけ。Compositor が無い・`GraphicsCore` が無いか無効、のような面を作る前の資源の確かめは覚えず、今どおり毎回試す（記録は今どおり）。あわせて、`WucGraphicsResource` の変化の印が動いたときも覚えを消すか、そうしない理由を書く。印の面の覚え（`markers_failed`）にも同じ決めを当てる。テストを 1 本足す: Compositor の無い World で 1 フレーム回して失敗させ、資源を入れた次のフレームで面ができること。
- **要件**: 2.1（指摘 3 を直す）・6.4（記録の無い失敗を作らない）・4.5（壊れたら落ちるテスト）
- **設計の中の場所**: 「面の上限と作れない面の覚え（指摘 3）」の「作れなかった場所の覚え」の段・System Flows・Error Handling の表の 2〜3 行目

### 問題 2: 配置の覚えの鍵に、テストだけの関数（`TalkLookahead::number`）を使っている

- **何が問題か**: 鍵の表の「全文の先読み」の行は、`lookahead_rev` と `lookahead.number(place)` を鍵にすると書く。しかし `lookahead.rs:221-225` の `number` は `#[cfg(test)]` で、本番のビルドには無い。`lookahead.rs` は `audit-emo-text-layout-draw` の持ち物で、設計自身も「`lookahead.rs`（向こうの持ち物）は変えない」と書いている。このままでは実装の段で、向こうのファイルを直すか、鍵を変えるかを決め直すことになる。また、`lookahead_rev` を進める所を「4 か所」と書くが、今の呼び出しは `preview_talk` の `install`（`actor.rs:433`）、`rehearse_again` の `reinstall`（`actor.rs:609`・`set_surface_resolver` からも呼ばれる）、`set_box_layout` の `forget_boxes`（`actor_box.rs:79`）の 3 か所だけ。
- **困る場面**: 鍵の欄が 1 つ欠けると、古い字を見せる（設計自身がそう書いている）。実装係が「`number` が使えないので外す」とだけして、外してよい理由を確かめないと、ここが穴になる。
- **直し方の案**: `lookahead.number(place)` を鍵から外し、`TextLayerState::clears_at(&PlaceKey)` と `lookahead_rev` で足りる理由を `FrameMemo` の説明に書く。理由は次のとおり。区間の番号は、先読みを入れ直した後の全消しと `\c` の数なので、同じ `lookahead_rev` の間は、その場所の消した回数と同じ時に動く。描き手（`ActorRender`）を持つ場所は、全消しのときに必ず `actors` に載っていて回数が進む（指摘 11 の「作りたての場所を外す」も、面を持つ場所は外さない）。箱の場所の回数を捨てる `set_box_index` と、先読みの `forget_boxes` と、箱の描き手を捨てる所は、どれも `set_box_layout` の中で一緒に起きる。「4 か所」は呼び出しの実数に合わせる。鍵の欄ごとのテスト（`actor_frame_memo_tests.rs`）には、「先読みを入れ直したフレームで作り直す」と「`\c` の後に同じ数の字を書き直すと作り直す」を残す。
- **要件**: 5.4（向こうの持ち物の形は申し合わせてから変える）・2.1（指摘 8 を直す）・4.6（決定論のテスト）
- **設計の中の場所**: 「配置の覚え（指摘 8・23）」の鍵の表の「全文の先読み」の行・Data Models の `FrameMemo`

## 大事ではないが直すとよい所（議題にしなくてよい）

- Data Models の表に、`scroll_marker.rs` の `ResolvedMarkerPlacement` の欄 `up`／`down` が `(i64, i64)` になることと、`default_marker_position` の形が変わることが無い（本文の「矢印の広がり」には書いてある）。呼び手は `actor_decoration.rs`・`actor_scroll.rs`・このクレートのテストだけで、`crates/areka` は使っていない（grep で確かめた）。
- 1,000 行の見込みは `surface.rs` だけに書いてある。`actor.rs`（841 行）と `actor_present.rs`（680 行）にも欄と覚えの処理が入るので、タスクを作るときに直した後の行数の見込みを書いておくと安全。
- 指摘 11 で作りたての場所を外すと、外したスコープは次の全消しで `clear_count` が進まなくなる（減りはしない）。`crates/areka` の `balloon_visibility` が見るのは登録したスコープで、登録したスコープは見た目の 2 層を持つので外れない。この理由を説明に 1 行書いておくと、後で読む人が迷わない。

## よい所

1. **公開の形を変えない約束を、コードの事実で裏付けている。** 消す口（`sink::handle_text_msg`・`TextLayerRuntime::config`・`MarkerSurface::read_back`／`size`／`window_child`）は、`crates/areka` からの呼び出しが 0 件だった（`crates/areka` が使う印の面は `marker_surface()` が返す `TextSurface` の `size`／`read_back` で、残る）。`PlacedMarker`・`ResolvedMarkerPlacement` の形の変化も、呼び手はこのクレートの中だけだった。`scale_text.rs:302` の記録が 1 回で止まるという主張も、`present_frame` が最初の `Err` だけを返す今の作りと合っている。
2. **怖い直しに、壊れたら落ちるテストと「変わらないこと」のテストの両方を置いている。** 指摘 3 の右と下だけを切る決めは、描き手が面の大きさ（`viewbox_draw_render.rs:70` の `surface.size()`）から計画し、見える行の窓は validrect から決まる（`window_for`）という今のコードと合っていて、見える画素は変わらない。指摘 5 の「組み立てを全部通してから面を付ける」も、`build_actor_render` の 4 つの手順がどれも面を要らないことをコードで確かめた。指摘 8 の鍵は、項目が全消しまで増えるだけ（項目を減らすのは `clear_content` だけ）という不変条件に立っていて、問題 2 を除けば足りている。鍵の欄ごとに「変えたら作り直す」テストを置く計画も、古い字を見せる危うさによく効く。

## 判定

**GO**

- **理由**: 大きな作りの食い違いは無く、要件はすべて実現する場所が決まっていて、実装の道筋もはっきりしている。2 件の問題は、どちらも新しく足す覚えの消し方の書き方の穴で、設計の数行を直せば閉じる。
- **次にすること**:
  1. 設計の討議（`/kiro-design-discussion audit-emo-text-state-actor`）で問題 1 と問題 2 を議題にし、設計の該当の段を直す。
  2. そのあと `/kiro-spec-tasks audit-emo-text-state-actor` でタスクを作る。問題 1 の追加テスト（資源が後から入ると面ができる）と、`actor.rs`・`actor_present.rs` の直した後の行数の見込みを、タスクに含める。
