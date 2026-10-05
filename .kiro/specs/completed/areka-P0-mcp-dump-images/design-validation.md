# 設計レビュー: areka-P0-mcp-dump-images

> 2026-10-04・本ブランチ（`claude/areka-p0-mcp-dump-images-41fbca`）のコードを読んで書いた。対話なしで判定した。
> 対象: [design.md](design.md)・[requirements.md](requirements.md)（要件 4.4 は欠番）・[research.md](research.md)・`.kiro/steering/`。
> コードは「何の定義か」＋ファイルパスで指す。確かめていないものは「未確認」と書く。

## 判定

**GO（条件つき）**。設計の骨組み（薄い配線＋純粋な関数 3 つ＋表示の層の読むだけの口）は、コードと突き合わせて成り立つ。下の重要な指摘 3 件は、どれも設計を作り直す話ではなく、**設計ディスカッションで決めてからタスクへ進む**点である。

## 1. レビューの要約

- 設計が頼っている名前・場所・見え方は、抜き取りで確かめた範囲ですべて実在し、書かれたとおりに振る舞う（下の「コードと突き合わせた結果」）。依存の向きも合法で、`Cargo.toml` を触らずに組める。
- 要件は 1.1〜7.7 のすべてが部品とテストに対応づいている（4.4 は欠番として表に明記）。
- 残る弱点は 3 つ: ⑴ 無圧縮 PNG の大きさを、受け取る側で確かめないまま最後の実機確認へ送っている、⑵ バルーンの文字の位置合わせ（要件 3.3）を、実際の描画を通るテストが固定していない、⑶ UI スレッドでの符号化の所要時間に合否の線が無い。

## 2. コードと突き合わせた結果

| 設計の主張 | 確かめた定義 | 結果 |
|---|---|---|
| `super::later` を呼ぶだけで後から答えられる。`mcp/mod.rs` は触らない | 関数 `later`・`poll_later`・`drain`（`crates/areka/src/mcp/mod.rs`） | 合っている。`later` は `pub(crate)`。覗く関数の形は `FnMut(&mut World) -> Option<ToolOutcome> + 'static`。預けた直後の同じ `drain` の中でも 1 度覗かれる（害なし）。待つ側が居なくなった組は `poll_later` が `is_abandoned` で捨てる |
| 待ちの上限は 10 秒で相手の持ち物 | 定数 `REPLY_WAIT`（`crates/areka-mcp/src/tools/mod.rs`） | 合っている |
| 子モジュールを `#[path]` で親のファイルに宣言すれば `mcp/mod.rs` を触らない | 既存の `#[path = "dump_surface_tests.rs"] mod …`（`crates/areka/src/mcp/dump_surface.rs`） | 合っている（同じフォルダの兄弟ファイルを指せる）。ただし下の軽い指摘 a |
| `handle` の署名は今のまま | 関数 `handle`（`crates/areka/src/mcp/dump_surface.rs`）・マクロ `resolved!`（`mcp/mod.rs`） | 合っている |
| 結線状態に欄 `attached` が在り、読み口を 1 本足せば済む | 構造体 `Emo2Wiring` の欄 `attached`（`pub(super)`）・読み口 `presenter()`・`runtime()`（`crates/areka/src/emo2_boot/frame/wiring.rs`）、関数 `run_attach_phase`（`frame/attach.rs`） | 合っている。`attached` は装着の相を 1 度通ると真になる（資産が消費済みという異常の枝でも真） |
| `read_back_target` の `#[cfg(test)]` は外さない | 同ファイルの `read_back_target` | テスト専用のまま。`presenter()` 経由で足りる |
| 結線状態が World から外れるのは `Update` の段の中だけ | 関数 `emo2_frame_system`（`crates/areka/src/emo2_boot/frame.rs`）の `remove_non_send`〜`insert_non_send`、`drain` は `Input` の段 | 合っている |
| 隠しても `last_show` と合成メモは残り、`current_surface_id` だけ消える | 関数 `apply_hide`（`crates/areka-emo-present/src/presenter/hub.rs`）、欄 `last_show` の注記（`presenter/target.rs`） | 合っている |
| 既存の `read_back` は未表示で `error!` を出すので使えない | 関数 `EmoPresenter::read_back`（`presenter/read.rs`） | 合っている（3 つの失敗の枝すべてで `error!`） |
| `snapshot.rs` から非公開の欄に届く | `PresentTarget` の欄は `pub(super)`（`presenter/target.rs`）、`presenter.rs` の `mod` の並び | 合っている（`presenter` の子モジュールなら届く）。`ComposeCache::get`・`CacheEntry.composed` は公開 |
| 新しい `Composer` で `&self` のまま合成できる | `Composer::new`・`Composer::compose`（`crates/areka-emo-compose/src/lib.rs`）、`EmoWorld::surface`（`world.rs`） | 合っている。`compose` は結果を保持しない |
| 普通のバルーンの文字の面だけを読む（箱は含めない） | `TextLayerRuntime::surface`＝`PlaceKey::balloon(actor)` を引く（`crates/areka-emo-text/src/actor.rs`）、`TextSurface::read_back`・`size`（`surface.rs`） | 合っている。`read_back` は `&self` |
| 文字の面は「物理寸・窓の原点からの物理 px の offset」で付く | 関数 `physical_arrangement`・`TextSurface::attach` の注記（`crates/areka-emo-text/src/surface.rs`） | 合っている |
| `areka` は既に `areka-nar` に依存し、`crc32` は公開で PNG と同じ変種 | `crates/areka/Cargo.toml` の `areka-nar` の行、関数 `crc32`（`crates/areka-nar/src/crc32.rs`・`lib.rs` で再公開・注記に「zip・gzip・PNG と同じ」） | 合っている。表引きの実装 |
| WIC の符号化は今の `windows` の機能では呼べない | 根の `Cargo.toml` の機能の並びに `Win32_System_Com_StructuredStorage` が無い | 無いことは確認。`windows` 側の条件付けそのものは未確認（research.md §10.1 の記述を信じた） |
| GPU つきの土台の前例 | 関数 `lap_rig_of`・`spawn_windows`（`crates/areka/src/shell_balloon_switch_session_lap_tests.rs`）、`ghost_switch_test_support.rs` | 在る。`GraphicsCore::new()` と `DPI::from_dpi` を使っている |
| テストから `ToolRequest::new` で呼べる | `ToolRequest::new`（`crates/areka-mcp/src/tools/bridge.rs`）、既存の `dump_surface_tests.rs` | 合っている |

「装着の前は `later` に預ける」と 10 秒の上限の関係は健全と判断した: 覗く関数は毎フレーム判断を最初からやり直すので、装着が済んだフレームで普通の答えになり、済まなければ入口が 10 秒で答え、待つ側が去った組は次の `poll_later` で落ちる。終了の途中は `close` が置き場ごと落とすので "shutting down" になる。要件 6.1・6.3 に反しない。

## 3. 重要な指摘（最大 3 件）

### 指摘 1: 無圧縮 PNG の大きさを、受け取る側で確かめる前に実装を終える段取りになっている

- **何が問題か**: 設計は「受け取る側が 1〜数 MB の画像を扱えるかは未確認。実機確認（要件 7.7 ⑴）で確かめる」としている。実機確認は実装の最後なので、そこで駄目と分かると `Cargo.toml` を触る判断（要件の境界の外）へ戻ることになる。
- **大きさの見立て**: base64 の長さは およそ 幅×高さ×4×1.37。434×687 で約 1.6 MB（SSP の 10〜20 倍）。要件 5.5 は上限を設けないので、絵が大きいシェルではそのまま膨らむ。レビュー者の知識では、Anthropic の API が受ける画像は 1 枚 5 MB までで、無圧縮だと約 91 万画素（例 800×1140）でこれを超える（**本レビューでは未実測**。Claude Code が手前で縮めて渡すかも未確認）。加えて、Claude Code は会話の記録に結果を残すので、1 回 1.6 MB は記録の肥大にも効く。画像のトークン数は画素数で決まるので、そこには差が出ない。
- **research.md が見落としている道**: `miniz_oxide` は既に `Cargo.lock` に在り、`areka-nar` が伸長に使っている（`crates/areka-nar/Cargo.toml`）。`crates/areka/Cargo.toml` に 1 行足せば、新しいクレートを 1 つも取り込まずに zlib の圧縮が 1 回の呼び出しで書ける。WIC の案（機能を 1 語足す）と比べて、COM もストリームも `unsafe` も要らず、決定論テストもそのまま使え、設計の「本番コードに `unsafe` を足さない」とも両立する。どちらの案も `Cargo.toml` を触るので、開発者の裁定が要る点は同じ。
- **求めること**（設計ディスカッションで 1 つ選ぶ）:
  - (ア) 無圧縮のまま進める。その場合は**最初のタスク**で、1.6 MB 級と 5 MB 超の画像を実際に Claude Code へ返して受け取れるかを確かめ、駄目なら止めて報告する、と段取りを改める。
  - (イ) `crates/areka/Cargo.toml` に `miniz_oxide` の 1 行を足すことを認め、`png` 関数の `IDAT` の中身だけを圧縮に替える（推奨。替わるのは 1 関数で、設計のほかの部分は動かない）。
  - (ウ) WIC（機能 1 語）にする。
- **影響する要件**: 5.1・5.5・6.1・7.7 ⑴。

### 指摘 2: バルーンの文字の位置合わせ（要件 3.3）を、実際の描画を通るテストが固定していない

- **何が問題か**: 重ね合わせの純粋な関数のテスト（Testing Strategy の 6）は式そのものを固定するが、**本番の配線が正しい値を渡しているか**（offset を差し込み口の `Arrangement` から取る・比を「物理寸÷原寸」で取る）は、統合テスト 5 の「文字の領域に背景と違う画素を含む」「大きさが原寸」でしか見ていない。offset を渡し忘れても、縦横を取り違えても、比を逆にしても、どこかに字が載っていれば緑になる。要件 3.3 の「背景と文字の位置がずれない」が檻に入っていない。
- **縮め方そのものについて**: 面積で重み付けした平均は妥当。拡大率 1・整数の offset で写しと同じ値になるという主張も式の上で成り立つ。「物理寸÷原寸」を使うことによる端のずれ（最大 0.5 物理 px）は原寸で 1 画素に満たず、受け入れられる。ただし設計は**丸めの規則と計算の精度**（重みの和で割った後の丸め方・`f32` か `f64` か・重ねるときの `÷255` の丸め）を書いていない。ここが決まっていないと、テスト 6 の「一致」が実装ごとに 1 段階ぶれる。
- **求めること**:
  - 統合テスト 5 を強める。拡大率 1 では、返った絵が「背景の合成」に「文字の面の読み戻し」を**バルーンの定義の領域の原点**（配線が使う `Arrangement` とは別の出どころ）へ重ねたものと画素で一致すること。拡大率 1.5 では、背景と違う画素の外接の矩形が、拡大率 1 のときの矩形と上下左右 1 画素以内で一致すること。
  - 最初に、この土台で文字の面が実際に透明でない画素を持つことを確かめる（持たないなら比較は空振りになる）。
  - `overlay_text` の丸めの規則と精度を設計に 1 行で書く。
- **影響する要件**: 3.1・3.3・7.4 ⑶。

### 指摘 3: UI スレッドでの符号化に、合否の線と退避の道が書かれていない

- **何が問題か**: 設計は符号化を UI スレッドでその場で行い、所要時間は「未測定・実機確認で記録する」としている。要件 6.1 は「描画が引っかからない」ことを求めるが、何ミリ秒なら合格かが無いので、実機確認 ⑺ は**測って書くだけで判定しない**ことになる。1 回の呼び出しは、乗算を戻す・CRC・Adler-32・base64 と、絵の全バイトを 4 回なめる（バルーンは GPU からの読み戻しと重ね合わせが加わる）。最適化の無いビルドや大きなシェルでは 1 フレーム（約 16 ms）を超えうる。指摘 1 で圧縮を選ぶと、さらに重くなる。
- **求めること**: 合否の線を設計に置く（例: 配布用のビルド・434×687 で 1 フレーム以内。大きさに比例する旨も書く）。超えたときの道も 1 行で決めておく: 絵の写しを取った後の符号化を別スレッドへ渡し、結果を `mcp::later` で受ける（置き場は既に在り、ツールの 2 ファイルの中で閉じる）。今は作らず、線を超えたら作る、でよい。
- **影響する要件**: 6.1・7.7 ⑺。

## 4. 軽い指摘（判定には影響しない・タスクで拾えば足りる）

- **a. 見え方の指定がそのままでは通らない**: 設計は `judge`・`image` の中の項目を `pub(super)` と書いているが、これは親の `dump_surface` の中でしか見えない。兄弟の `dump_balloon` から `super::dump_surface::{judge, image}` の中身（`judge_balloon`・`balloon_text`・文言の定数・`png_base64`）を使うには、`pub(in crate::mcp)`（または `pub(crate)`）が要る。`PresenterFacts` と `fail` は `dump_surface.rs` 直下の `pub(super)` で足りる。
- **b. ゴーストの切り替えの途中**: 覗く関数は毎フレーム World の結線状態を読み直すので、預けている間にゴーストが替わると、解決したゴーストと別のゴーストの絵を返しうる。結線状態が一時的に無い間は `NG:This ghost has no window` になる。切り替えの間に結線状態が World から外れる時間が在るかは**未確認**。差の一覧か注記に 1 行あれば足りる。
- **c. `later` の `#[allow(dead_code)]`**: 呼び手ができても `mcp/mod.rs` を触らない約束なので、属性と「生えるまで」の注記が残る。害は無い。
- **d. 表示の層の口のテスト**: `snapshot.rs` の 3 本は x64 の GPU つきテストでしか固定されない。research.md §11.4 が設計ディスカッションへ出すとした「`presenter/snapshot_tests.rs` を足してよいか」は、認めれば GPU なしで `has_surface`・`compose_alone` を固定できる。

## 5. 設計の良い点

- **判断を事実の口（`ShellFacts`）へ寄せた**: 要件 4.9 の順と 4 つの文言が、窓も GPU も要らない 1 つの関数に収まり、要件 7.2 の 12 通りを手書きの表で固定できる。`read_back` の `error!` を避けるために別の読み口を足した判断も、コードの振る舞いと合っている。
- **「装着の前」を既存の `later` で受けた**: 新しい文言も新しい仕組みも足さずに、在るはずのスコープ 0 へ「無い」と答える誤りを避けている。10 秒の上限・終了の途中の答えも相手のまま保たれる。

## 6. 次の一手

1. 設計ディスカッションで指摘 1（PNG の圧縮の扱い）を裁定する。
2. 指摘 2・3 を設計へ反映する（テストの強化・丸めの規則・所要時間の線）。
3. その後 `/kiro-spec-tasks areka-P0-mcp-dump-images` へ進む。
