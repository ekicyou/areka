# 設計の検証: wintf-d2d-composite-rules

- 日付: 2026-11-13
- 的: `design.md`（設計の段の生成の後・未承認）
- 読んだもの: `spec.json`・`requirements.md`・`design.md`・`research.md`（§10 の試しの結果・§11 の決め）・steering（`product.md`・`tech.md`・`structure.md`・`logging.md`）
- 突き合わせたコード（`crates/wintf`）: `ecs/graphics/systems/render.rs`（153 行）・`components.rs`（`SurfaceGraphics`・`set_surface`／`clear`）・`core.rs`（共有の DC・`note_draw_failure`・工場は `MULTI_THREADED`）・`visual/draw/{builder,emit,command}.rs`・`visual/world_visual.rs`（177〜183 行目の `warn!`・`subtree_world_aabb`）・`com/wuc.rs`（`begin_draw` は `ID2D1DeviceContext3`）・共有の DC の使い手（`shapes/rectangle.rs`・`text/draw_labels.rs`・`text/typewriter_draw.rs`・`bitmap_source/systems.rs`・`areka-emo-present` の `presenter/show.rs`）・`composite_mode` の使い手の全数

## 設計の検証のまとめ

既存の描画の木（深さで閉じる push／pop）と描画面の部品の寿命（`set_surface`・`clear`）にそのまま乗る形で、新しい依存も `crates/areka/` への接触も無く、要件 1〜8 の全項目が部品と流れに割り付けられている。最大の危険だった要件 3（図形の切り抜きの中の効き先）と 4.2（入れ物を通した画素のビット一致）は、設計の前の試しで WARP と実 GPU の両方の数で裏付けられている。残る問題は、読み戻しの並行の約束と、決まって失敗する入力のときの描き直しの回り続けの 2 つで、どちらも設計の中の約束を足せば閉じ、作りを変えない。

コードとの突き合わせで確かめた点（食い違い 0）:

- `render_surface` は今 `&SurfaceGraphics` を読み、失いでない `BeginDraw` の失敗だけ描き直しの印を命令で立て直し、WUC の `EndDraw` の失敗は記録だけ（設計の「今は記録だけ」と一致）。
- `SurfaceGraphics::set_surface`・`clear` が描画面の入れ替えと装置の失いの無効化（`window_pos.rs` の `invalidate_dependent_components` の `sg.clear()`）の唯一の口＝入れ物をこの部品の欄に持てば寿命が揃う（4.3・4.5）。
- `emit` の今の注記（14〜15 行目「層は透明から始まる・変えるのは本 spec」）と図形の層の `contentBounds ±f32::MAX`・`D2D1_LAYER_OPTIONS1_NONE` は設計の「変えるところ」と一致。
- `composite_mode` の使い手は `emit.rs`・`builder.rs`・`command.rs`・`draw_content.rs`・`emit_tests.rs` 125 行目・`builder_tests.rs` 406 行目だけで、ほかは `new`／`new_ex` 経由（Migration Strategy の列挙と一致）。emo は `VisualDrawContent` を使っていない。
- 1,000 行の決め: 今いちばん大きい触るファイルは `builder_tests.rs` 577 行・`emit_tests.rs` 498 行で、新しいテストは兄弟のファイルへ分ける計画（6.7 は守れる見込み）。
- steering の参照の行（`tech.md` 72・90・106 行目、`structure.md` 55 行目）は今の文書のその行を指している。

## 重大な問題（3 件まで）

### 🔴 重大な問題 1: 1 画素の読み戻しの「並んで走ってよい」と、共有の 1×1 の写し先の錠の範囲が合っていない

- **問題**: `SurfaceComposite` は 1×1 の写し先 `pixel_staging` を 1 枚だけ持ち、錠（`Mutex`）は控え `last_read` だけに掛かる形で書かれている。一方で `SurfaceReadback` は `&self` の読みだけで「`Res<GraphicsCore>` を読む係どうしと並んで走ってよい」とされ、`SurfaceReadback` を持つ係が 2 つあれば bevy は並べて走らせる。2 つの読みが同じ描画面で `CopyFromBitmap` → `Map` → `Unmap` を交互に行うと、片方が他方の座標の画素を読む（5.1 に反する・別の値に化ける）か、写像中の写し先への `Map` が失敗して `error!` と `Unreadable` になる（5.6 の「GPU の失敗」に化ける）。
- **影響**: 使い手が 2 つ目の係で読み戻しを使い始めた時点（`hit-id-pass`・`draw-methods-canon`）で、まれに座標の違う画素を返す。再現が並行の取り合わせに依るので、檻では見つけにくい。
- **提案**: 写し先と控えを 1 つの錠の中に入れ（例: `Mutex<ReadState { staging, last }>`）、`CopyFromBitmap`・`Map`・`Unmap`・控えの更新を錠を持ったまま行うことを Invariants に書く。`read_all` は呼び出しごとに写し先と DC を作るので今のままでよい。readback_tests に「2 つのスレッドから違う座標を交互に読み、それぞれの答えが自分の座標の値」の 1 本を足す。
- **追跡**: 5.1・5.2・5.6・7.8
- **根拠**: design.md「SurfaceComposite」の Contracts（`pixel_staging`・`last_read: Mutex<Option<LastRead>>`）、「SurfaceReadback」の末尾（「`Res<GraphicsCore>` を読む係どうしと並んで走ってよい」）

### 🔴 重大な問題 2: 決まって失敗する入力で、描画面が毎フレーム合成し直され `error!` が流れ続ける

- **問題**: 設計は混ぜ方の値を確かめずに D2D へ通す（「範囲の外の値は D2D の答えに任せる・`EndDraw` の失敗に出る」）。そして合成の失敗（失いでない）は `error!` を残して描き直しの印を命令で立て直す（4.6）。公開の欄 `mix_mode` に `MixMode::Blend(D2D1_BLEND_MODE(99))` のような値が入ると、合成の `EndDraw` が毎回同じ理由で失敗し、印の立て直し → 次の回にまた失敗、を止まらずに繰り返す（`tick_wake` で係も起こし続ける）。その間は WUC の描画面に触らないので、同じ描画面のほかの節の変化も画面に出ない。今まで記録だけだった WUC の `EndDraw` の失敗も同じ道に乗るので、決まった失敗の回り続けの入口が増える。
- **影響**: 描き手 1 人の誤った値 1 つで、その描画面全体の更新が止まり、ログが毎フレームの `error!` で埋まり、UI スレッドと GPU を回し続ける（「1 体の失敗はアプリの失敗ではない」の方針に照らしても、影響が描画面全体と CPU に広がる）。
- **提案**: 次の 2 つのどちらか（両方でもよい）を設計に足す。(a) `draw_method` で値の範囲を確かめ、ブレンドは 0〜25・重ね方は 0〜12 の外を `SOURCE_OVER` で描き `error!` を 1 度だけ残す（1.7 の道と同じ形・純粋なテストで固定できる）。(b) 失いでない合成の失敗の記録を、同じ描画面・同じ HRESULT の続きは 2 回目から `debug!` に落とす（`ReportedIssues` と emo の `failure_note` の形）。描き直しの印を立て直すこと自体は 4.6 のとおり残す。
- **追跡**: 1.2・4.6・6.6
- **根拠**: design.md「MixMode」の不変条件、System Flows「描画面 1 枚の描画」の 1 つ目の箇条（「WUC の `EndDraw` の失敗も同じ道に乗せる」）、「render_surface の変更」の失敗の行、Error Handling の表

## 設計のよいところ

1. **最大の危険を数で潰してから形を決めている**: 図形の切り抜きの層の旗を 5 通り試し、積む前に基本の混ぜ方を `COPY` にする 2 通りでだけ内側・外側とも食い違い 0 になること（設計はそのうち積んだ直後に `SOURCE_OVER` へ戻す形を採った）、降ろす直前だけ `COPY` にすると誤ることまで WARP と実 GPU で確かめた（research.md §10.2）。入れ物を通した写しのビット一致（§10.4）・`BlendImage` の変形と絵の外（§10.3）・DC6 の cast（§10.5）・読み戻しの待ち（§10.6）も測ってあり、5.10 の記録の要件を設計の段で満たしている。
2. **既存の継ぎ目にそのまま乗り、足すものが少ない**: 不透明度の層は切り抜きと同じ「深さで閉じる push／pop」に乗り（新しい仕組みを作らない）、入れ物の寿命は `SurfaceGraphics` の `set_surface`・`clear` に乗り、合成の手順は WUC と ECS を知らない `SurfaceComposite` に置いて本番の係と WARP の足場の両方から同じ手順を通す。D3D の写し・専用の DC・読み戻しの係・`com/wuc.rs` の変更を採らない簡素化の理由も表に残っている。

## 最終の判定

- **判定: GO**
- **理由**: 作りの食い違い・要件の欠け・依存の向きの誤りは無く、重大な問題 2 件はどちらも部品の中の約束（錠の範囲・値の範囲か記録の間引き）を足すだけで閉じ、部品の分け方と流れを変えない。
- **次の段**:
  1. 設計の議論（`/kiro-design-discussion wintf-d2d-composite-rules`）で、重大な問題 1・2 の提案を議題にし、決めた形を design.md の「SurfaceComposite」の Invariants と「MixMode」の不変条件（または Error Handling の表）へ書き足す。
  2. その後 `/kiro-spec-tasks wintf-d2d-composite-rules` でタスクを作る。
