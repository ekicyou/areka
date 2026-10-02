# Brief: areka-P0-shell-balloon

> 2026-10-01 `/kiro-discovery` で起票（開発者指示「areka では設計上、シェルとバルーンの描画区別を付けないように留意してもらっていたと思う。実際に、シェル内にバルーン領域を持つゴーストを設計したい。実装時期は α 後なるべく早い時期」）。roadmap「シェル内バルーン」節の土台の spec。**完全に areka 独自の仕様＝差別化のための機能**（開発者）。
> 本文の file:line は起票時（main `35209987`）の実測＝**着手時に引き直すこと**。参考の絵と書き味は ghost_dev リポジトリの「窓際のぱすたさん」`doc/assets/text_layout/縦書きキャッチコピー.html`（白い窓の絵の中に、縦書きの台詞を右 2 列・左 3 列で置く）と同 `doc/画面設計_縦書き要件定義.md`（§3.1 の寸法）。

## Problem

- **ゴーストの作者**: 立ち絵の中（例: 窓の絵の白い面）に台詞を直接書く演出が、伺かでは作れない。正典には「シェルの中に文字の場所を持つ」仕組みが無い（ukadoc の調査で 0 件）。透明なバルーンを負の位置ずらしでシェルに重ねる手はあるが、窓が 2 枚のままで、ドラッグや拡大率の追従がずれ、1 人に 2 か所の文字の場所を持てない。
- areka の設計原則は「描画エンジンはシェルとバルーンを区別しない・バルーンはサーフェス上の文字の層」（roadmap「アーキテクチャ横断原則」）だが、それを利用者に見える形で使う機能がまだ無い。

## Current State（起票時の実測）

- **一番下の層（emo-present）は原則どおり**: 窓ごとの組み立て `VisualMount::attach`（`crates/areka-emo-present/src/mount.rs` の attach）は、シェルの窓にもバルーンの窓にも文字の層の差し込み口（`emo-text-layer-slot`）を作る。`text_slot_view(target)` はどの窓にも効く。拡大率 k もシェルの窓の値がそのまま流れる。
- **その上の層は区別している**:
  - 文字の描き手はバルーンの窓にしか結線されない（`crates/areka/src/emo2_boot/frame/attach.rs` の `run_attach_phase` → `connect_balloon_text`）。拡大率の追い直し（`frame/scale_text.rs`）もバルーンの側から組み直している見込み（未確認）。
  - 文字の設定の入力は `BalloonModel` だけ（`crates/areka-emo-text/src/actor.rs` の `register_actor_view`・`ResolvedBalloonText::resolve`）。
  - 1 つのスコープに文字の場所は 1 つ（`ActorKey`＝スコープ・`TextRegion` は 1 つ）。
  - バルーンの窓は文字の数で自動で現れる（`crates/areka/src/emo2_boot/balloon_visibility_phase.rs`）。
- **surfaces.txt の読み手**（`crates/areka-parsers/src/shell/decode.rs` の `dispatch_block`）は `descript`→`kero.surface.alias`→`surface.append*`→`surfaceNNN` の順に振り分け、知らないブレスは黙って捨てる。element定義の描画メソッドに `balloon` は無い。
- **1,000 行の上限**: `areka-emo-text/src/actor.rs` 975・`region.rs` 977・`layout.rs` 977 行＝**本 spec の変更は先に分割が要る**（番人 `crates/log-capture-kit/tests/file_length_guard_test.rs` の例外表には触れない）。

## Desired Outcome

シェルの surfaces.txt だけで、立ち絵の中に台詞を書く場所（**シェル内バルーン**）をいくつでも置け、台本からは普通のバルーンと同じ書き方で書ける。

```
balloon.台詞
{
  size,96,498
  vertical,1
  font.name,ZenKurenaido-Regular.ttf
  font.height,27
}

surface1000
{
  element0,overlay,pasta/xyz.png,0,0
  element2,balloon,台詞,364,62
  element3,balloon,独白,14,62
}
```

## Approach（2026-10-01 discovery の開発者確定事項）

1. **新しいブレス `balloon.名前`**（ukadoc の「surface*ブレス」と同じ用語の型）。中身は**バルーンの descript.txt と同じキー**＋大きさのキー（仮称 `size,幅,高さ`・サーフェス画像の px）。大きさはブレスが持ち、サーフェスが替わっても変わらない。
2. **置くのは `surface*`ブレスの element定義**: `elementN,balloon,名前,X,Y`。描画メソッド `balloon` のときだけ、ファイル名の欄を `balloon.名前`ブレスの名前として読む（画素を重ねるのではなく文字を描くので、専用の描画メソッドにした）。位置はサーフェスごとに変えてよい。数に上限は無い。`surface.append*` でも置ける。
   - 将来「`surface1000` を element定義で置く」を足すときは、描画メソッドはそのまま、ファイル名の欄でサーフェスを指す見込み（正典の pattern定義 `animation*.pattern*,overlay,100,…` の前例）。本 spec の決め方はそれと矛盾しない。
3. **バルーンの中身はすべて互換**: シェル内バルーンは**同じバルーンの実装**で、文字を描く先がシェルの窓の箱に替わるだけ。普通のバルーンで動くキーはすべてシェル内バルーンでも動き、未実装のキーは普通のバルーンで実装された時点で両方で動く。当てはまらないのは `windowposition.*`（位置は element定義が決める）と `use_self_alpha`・`use_input_alpha`（背景の絵を持たない＝背景はシェルの絵）だけ＝ログに 1 行残して読み捨てる。**背景の絵は持たせない**（開発者確定）。
4. 箱の中の座標: `origin.*`・`validrect.*`・`wordwrappoint.*` は箱の左上を (0,0) として読む（負の値は反対側の端から＝今の areka と同じ）。書かなければ `validrect` は箱いっぱい・`origin` は書き出しの角（縦書きなら右上）。
5. **文字の行き先は「スコープ × ブレスの名前」**: \0 の立ち絵の `台詞` と \1 の立ち絵の `台詞` は別の文字を持つ。既定の行き先は今のサーフェスの一番若い element番号のシェル内バルーン。`\b[名前]` で切り替える（`\b[数字]` は従来どおり普通のバルーン）。今のサーフェスに無い名前を指したら警告をログに残して切り替えない。
6. **普通のバルーンと併用しない**: シェル内バルーンを 1 つでも置いたサーフェスを出しているあいだ、そのスコープの普通のバルーンの窓は出さない。
7. **サーフェスの切り替え**（文字はサーフェスでなく「スコープ × 名前」に属する）:
   - 新しいサーフェスにも同じ名前がある → 文字ごと新しい位置へ移り、書きかけの続きも書ける。
   - 書いていた名前が無い（他のシェル内バルーンはある）→ 文字は持ったまま表示だけ消し、以後は新しいサーフェスの既定へ書く。元のサーフェスに戻れば残っていた文字がまた見える。
   - シェル内バルーンが 1 つも無い → 普通のバルーンの窓に切り替わる。シェル内バルーンの文字は持ったまま。
   - `\s[-1]` は「書いていた名前が無い」と同じ扱い。新しい台詞の頭では今までどおり全部消す。
8. **`\c` は今の行き先のシェル内バルーンだけを消す**（正典の「今のバルーンを消す」の読み替え）。
9. **選択肢（`\q`）もシェル内バルーンに出す**（`cursor.*` を生かす・選択肢のクリックも箱の中で受ける）。
10. **フォントファイルを探す順番はシェルのフォルダ → ゴーストのフォルダ**（読み込み自体は `balloon-font-file`。本 spec はシェル内バルーンの探し場所を渡す口まで）。
11. **重ね順（最初の版の縮め）**: 文字の層は今、画像の合成とは別の層で常に一番上に描かれる。最初の版では「`balloon` の element定義は画像の element定義より後ろに書く（＝一番上）」とし、違反は警告を出して一番上に描く。element定義の並び順どおりに挟み込むのは `balloon-element-order`（追跡 spec）。
12. **箱のクリック**: 箱の中のクリックは、話していないときは従来どおりシェルのクリック（当たり判定）として扱う。話している最中のクリックでの早送りは `talk-fast-forward`。

## Scope

- **In**:
  - surfaces.txt の `balloon.*`ブレスの読み取り（バルーンの descript.txt の読み手を再利用して `BalloonModel` を作る＋`size`）と、element定義の描画メソッド `balloon`（`surface*`ブレスと `surface.append*`ブレスの両方）。不正な定義（無いブレスの名前・`size` 欠落など）はログに残して読み捨て。
  - シェルの窓の文字の層の差し込み口への結線。1 スコープに複数の文字の場所（`ActorKey` の形の見直し）。拡大率の追従・窓のドラッグへの追従（シェルの窓にいるので追従は不要になるはず＝確かめる）。
  - 行き先の決め方（既定・`\b[名前]`）、併用しない規則、サーフェスの切り替えの 3 場面、`\c`、選択肢、箱のクリックの当たり判定。
  - `actor.rs`・`region.rs` の分割（振る舞いを変えない先行の括り出し）。
  - 決定論テストと、シェル内バルーンを持つ試験用シェル（リポジトリ内の検体）。実機の確かめは「窓際のぱすたさん」の参考の寸法で行う（ゴーストはリポジトリの外）。
- **Out**: フォントファイルの読み込み（`balloon-font-file`）・禁則と縦中横（`text-typesetting`）・ルビと列の間隔（`text-ruby`）・早送りと `\x`（`talk-fast-forward`）・矢印などの印（`balloon-markers`）・フェード（`balloon-scroll-fade`）・重ね順の挟み込み（`balloon-element-order`）。

## Boundary Candidates

- パーサ（`areka-parsers` の shell）＝ブレスと element定義の転記だけ。`BalloonModel` への写しは下流（parser は転記層）。
- 文字の層（`areka-emo-text`）＝1 スコープ複数の文字の場所。
- 結線と表示の判断（`crates/areka/src/emo2_boot/`）＝どの窓の差し込み口へつなぐか・普通のバルーンを出すか。

## Out of Boundary

- 正典のバルーン（`balloons*.png` の系列・`\b[数字]`）の振る舞いは変えない。
- SSP での見え方（SSP は `balloon.*`ブレスを知らない＝普通のバルーンで出る）。

## Upstream / Downstream

- **Upstream**: α の完成宣言（`alpha-release-signoff`）。
- **Downstream**: `talk-fast-forward`・`balloon-markers`・`balloon-element-order`（本 spec の結線と箱の上に乗る）。`balloon-font-file`・`text-typesetting`・`text-ruby` は普通のバルーンにも効く機能で、本 spec とは独立に進められる。

## Existing Spec Touchpoints

- **Extends**: なし（新しい境界）。
- **Adjacent**: `balloon-canon-residue`（`\b[数字]` の面の偶奇・系列）・`balloon-lifecycle-events`（表示の寿命）・`currentghost-property-tree`（`seriko.*` のプロパティに将来シェル内バルーンが現れうる）。
- **2026-10-01 追記（`/kiro-discovery`「動く画像」・roadmap「動く画像」節）**: Approach 2 の「将来 `surface1000` を element定義で置く」は `surface-element-nesting` として起票した。書き方は **ファイル名の欄が数字だけならサーフェスの番号**＝element の中身の読み分けは 3 通り（描画メソッドが `balloon` なら名前／数字だけならサーフェス／それ以外は画像）。**同じ element の型を触る＝同時に走らせない**。後から着地した方が 3 通りを 1 つの列挙に揃え、**入れ子の内側のサーフェスに置かれた `balloon` の element定義は警告をログに残して無視する**（開発者確定・最初の版。同じ子を 2 か所に置くと同名の箱が 2 つでき、「スコープ × 名前」の行き先が決まらないため）。

## Constraints

- 1 ファイル 1,000 行（番人の例外表に触れない）。
- 決定論テスト網羅は必達。ログ無しの失敗の経路を作らない。
- 意味論は ukadoc から輸入するが、本 spec の語（`balloon.*`ブレス・描画メソッド `balloon`・`\b[名前]`）は areka 独自。網羅台帳（`doc/ukadoc-coverage/`）の外の語として `doc/COMPAT_ARCHITECTURE.md` §8 に登記する。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- **ウェーブ C2（`emo-text-file-split` の直後）・Fable 推奨**。規模 L（**分割を先に済ませて 15〜19 タスク**＝上限の内。切り分けはしない）。
- 合っていた点: どの窓にも文字の層の差し込み口がある（`VisualMount::attach`）・`text_slot_view` は相手を問わない（定義は `presenter/read.rs`）・**拡大率の追い直しは「未確認」でなく確定**＝`emo2_boot/frame/scale_text.rs` は `balloon_target(scope)` しか組み直さない。
- **brief の誤りと抜け（要件で必ず扱う）**:
  1. **差し込み口は 1 つの窓に 1 つ**で、`TextSurface::attach` はその entity に直に文字の面を挿す（`surface.rs`）＝1 つの窓に文字の面は 1 枚しか載らない。箱を 2 つ以上置くには差し込み口を箱の数だけ作るか子の entity を作る（触る先は emo-present の `mount.rs`・`presenter/read.rs` か emo-text の `surface.rs`）。
  2. **`ActorKey` は dola の型**（`crates/dola/src/cue/command.rs`）で、テスト以外の 71 ファイルが使う。「`ActorKey` の形の見直し」をそのまま行うと dola・sakura・seriko・ghost まで波及する。**`ActorKey` は残し、emo-text の中（`TextLayerRuntime` の各表と `state.rs`）に箱の名前の副キーを足す**形が現実的。
  3. **`region.rs` は分割も変更も要らない見込み**。`TextRegion::resolve` は validrect を書かなければ画像いっぱい・origin を書かなければ書き出しの角を返す＝画像の大きさに箱の `size` を渡せば Approach はそのまま成り立つ。
  4. **シェルのパーサは `overlay` の element しか拾わない**（`shell/decode.rs` の `decode_elements`）。`Element` の型に描画メソッドの欄は無い。欄を足すか `Shell` に別の表を持たせるかで、約 20 ファイルへの波及が変わる（`surface-element-nesting`・`balloon-element-order` が触るのと同じ型）。
  5. **`\b[名前]` を今受けているのは seriko**（`crates/areka-seriko/src/actor.rs` の `BalloonResolve::NameForm` で warn して読み飛ばす）。直さないと `\b[台詞]` のたびに warn が出る。
  6. **「箱のクリックは従来どおりシェルのクリック」**: areka は今 `OnMouseClick` を送っていない（送るのは `OnMouseMove`／`OnMouseDoubleClick`）。表示中の差し込み口は当たり判定を持つので、見えている箱はサーフェスの当たり判定より手前でポインタを取る＝素通しにするかを決める。
- **触るファイル**: `crates/areka-parsers/src/{shell/{decode,model,mod}.rs, balloon/{parse,mod}.rs}`・`crates/areka-seriko/src/actor.rs`・`crates/areka-emo-text/src/{actor 系, actor_decoration.rs, state.rs, surface.rs, lib.rs}` か `crates/areka-emo-present/src/{mount.rs, presenter/read.rs}`・`crates/areka/src/emo2_boot/{frame/attach.rs, frame/scale_text.rs, assets.rs, balloon_visibility 系, frame/switch.rs か shell_balloon_switch.rs}`・`crates/areka/src/input_events/`・検体・`doc/COMPAT_ARCHITECTURE.md` §8。`main.rs`・`Cargo` の類・wintf・`placement/`・`areka-nar` には触らない見込み。
- **議題**: 「スコープ × 名前」の鍵を emo-text の中だけで持つか／箱を差し込み口を増やして載せるか子の entity にするか／element の型を広げるか別の表か／表示中の箱が取るポインタ判定／選択肢のクリックを受ける場所。
- **引き取り**: `emo-text-canon-residue` の最後の 1 件（項目 14＝折り返しの警告にバルーンの名前を出す・`region.rs` の `BALLOON_NAME_PLACEHOLDER`）を本 spec が引き取る（名前を持つバルーンを入れるのが本 spec＝棚卸⑳で `emo-text-canon-residue` を取り下げた）。wintf の「兄弟の重なり順」の裁定も、箱を子として複数作るなら本 spec の要件に乗せる。
- **並べ方（厳しい目）**: 文字まわりの spec（`balloon-font-file`・`text-typesetting`・`talk-fast-forward`・`text-reveal-fade`・`balloon-markers` ほか）とは**同時に走らせない**。roadmap の旧 S1「`balloon-font-file` と並走」は外れ＝`actor` 系・`actor_decoration.rs`・`frame/attach.rs` を共有する。
