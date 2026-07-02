# Requirements Document

## Project Description (Input)
emo2 のバルーン設定（balloon `descript.txt` ＋ 画像別 `balloonsXXs.txt`/`balloonkXXs.txt`）を、下流の text-layer・surface-engine/render が消費できる「幾何＋フォント＋3段優先度解決済み」のバルーンモデルへ解析する parser が存在しない。`areka-P0-parser-foundation`（charset デコード＋素朴 KV マップ化・完了済）は KV マップまでしか担わず、バルーン固有のキー写像・座標符号解釈・3段参照優先度解決は非所有である。本 spec は `areka-parsers` クレートへ `balloon` モジュールを追加し、foundation 2 段（charset→KV）の出力を消費して、emo2-kakukaku fixture で pass するバルーンモデル生成源を確立する。正典は ukadoc（`descript_balloon`）、`doc/emo2-conformance-scope.md` §4、および emo2-kakukaku fixture であり、emo2 が実際に使う幾何＋フォント subset のみを実装する（過剰・予測実装は禁止）。

## Requirements

## Boundary Context

- **In scope**:
  - `areka_parsers::balloon` モジュールによる、バルーン `descript.txt`（既定層）と画像別 `balloonsXXs.txt`/`balloonkXXs.txt`（上書き層）の解析。
  - 幾何＋フォント subset のバルーンモデル定義: `windowposition`(x,y)・`origin`(x,y)・`wordwrappoint`(x,y)・`validrect`(top/bottom/left/right)・`font`(name/height/color rgb)。
  - 3段参照優先度解決（画像別 ＞ balloon descript ＞ 未指定）と、画像別・descript の 2 層ファイル間マージ。
  - バルーン座標の符号解釈（負値＝反対辺からのオフセット、`windowposition.y` の下方向＝正）。
  - emo2-kakukaku fixture ベースの単体テストによる観測（host 不要・純粋関数）。
- **Out of scope**:
  - charset デコード・KV マップ化（`areka-P0-parser-foundation` 領分）。
  - バルーンフォルダの所在解決・どのバルーンを使うかの選択（ghost/package 領分・baseware 共有）。
  - 文字描画・バルーン枠 surface 合成・文字レイアウト（`areka-P0-text-layer`/`areka-P0-surface-engine`/render 領分）。
  - choice/link/scroll 系キー（cursor・anchor・number・arrow・sstpmarker/sstpmessage・onlinemarker・communicatebox・marker）のモデル化・挙動（M1 未実装）。
  - さくらスクリプトのバルーン操作タグ（`\b`/`\_b`/`\q` 等）の解析（`areka-P0-sakura-parse` 領分）。
  - sakura/kero の左右配置を決める shell descript の `*.balloon.alignment`（shell parse／消費側の領分）。
- **Adjacent expectations**:
  - 上流 `areka-P0-parser-foundation` が charset デコード済み文字列または KV マップを提供する。本モジュールはその出力を入力として受け取り、ファイルのバイト列やパスからの前処理は行わない。
  - 下流 `areka-P0-text-layer` は `origin`/`wordwrappoint`/`validrect`/`font` を、`areka-P0-surface-engine`/render は `windowposition` を消費する。負値座標の最終ピクセル解決（反対辺の実寸に加算する計算）は、バルーン画像サイズを知る消費側が行う。本モジュールは符号付きの型付き幾何を提供するのみで、画像サイズには依存しない。

### Requirement 1: バルーン設定の入力受理と寛容パース
**Objective:** parser 利用者（下流エンジンの呼び出し側）として、デコード済みバルーン設定を渡すだけでバルーンモデルを得たい。そうすれば、ファイル所在解決やエラーハンドリングを気にせず幾何・フォント情報を消費できる。

#### Acceptance Criteria
1. When 呼び出し側が単一のバルーン設定ソース（デコード済み文字列または foundation の KV マップ）を渡したとき, the balloon parser shall そのソースを解析して 1 つのバルーンモデルを返す。
2. The balloon parser shall 解析結果を `Result` ではなく常にバルーンモデル値として返し, 解析失敗によるエラーを呼び出し側へ伝播しない（寛容パス）。
3. If 入力に未知のキーまたは解釈不能なトークンが含まれるとき, then the balloon parser shall それらを無視してモデル化対象キーの解析を継続する。
4. If モデル化対象キーの値が数値として解釈できないとき, then the balloon parser shall そのキーを未指定として扱い, 解析を継続する。
5. The balloon parser shall バルーンフォルダの所在解決・使用バルーンの選択・charset デコード・KV マップ化を行わず, これらを上流・呼び出し側の責務とする。

### Requirement 2: バルーンモデル（幾何＋フォント subset）の定義
**Objective:** 下流の text-layer・surface-engine/render 実装者として、emo2 描画に必要な幾何・フォント値を型付きで参照したい。そうすれば、キー文字列の再解釈なしにバルーン配置・文字領域・フォントを描画できる。

#### Acceptance Criteria
1. The balloon parser shall バルーン配置調整値 `windowposition`（x, y）をモデルに含める。
2. The balloon parser shall 文字描画原点 `origin`（x, y）をモデルに含める。
3. The balloon parser shall 自動折返し点 `wordwrappoint`（x, および存在すれば y）をモデルに含める。
4. The balloon parser shall テキスト描画有効矩形 `validrect`（top, bottom, left, right）をモデルに含める。
5. The balloon parser shall フォント設定 `font.name`・`font.height`・`font.color`（r, g, b）をモデルに含める。
6. Where モデル化対象キーが入力に存在しないとき, the balloon parser shall 当該値を「未指定」として表現し, 組込み既定値で埋めない。
7. The balloon parser shall choice/link/scroll 系キー（cursor・anchor・number・arrow・sstpmarker・sstpmessage・onlinemarker・communicatebox・marker）をモデル化せず, 幾何＋フォント subset に限定する。
8. The balloon parser shall モデルの各値へ読み取り専用のアクセス手段を提供し, 将来のキー追加に対する拡張の余地を型に残す。

### Requirement 3: 3段参照優先度解決とファイル間マージ
**Objective:** parser 利用者として、画像別設定と共通既定設定を渡すだけで優先度解決済みの単一モデルを得たい。そうすれば、複数ファイルの重ね合わせロジックを呼び出し側で再実装せずに済む。

#### Acceptance Criteria
1. When 呼び出し側がバルーン `descript.txt`（既定層）と画像別ファイル（`balloonsXXs.txt` または `balloonkXXs.txt`・上書き層）の両方を渡したとき, the balloon parser shall 両者を 1 つのバルーンモデルへマージする。
2. When 同一キーが画像別層と descript 層の双方に存在するとき, the balloon parser shall 画像別層の値を優先する。
3. When あるキーが画像別層に存在せず descript 層のみに存在するとき, the balloon parser shall descript 層の値を採用する。
4. When あるキーがどちらの層にも存在しないとき, the balloon parser shall 当該値を「未指定」として表現する。
5. Where 呼び出し側が descript 層のみ（画像別層なし）を渡したとき, the balloon parser shall descript 層の値のみからモデルを構築する。

### Requirement 4: バルーン座標の符号解釈
**Objective:** 下流の描画実装者として、負値座標が SSP 慣行どおりに解釈された型付き幾何を得たい。そうすれば、符号の意味を各所で再判断せずにピクセル位置へ解決できる。

#### Acceptance Criteria
1. The balloon parser shall `validrect` および `wordwrappoint` の負値を「反対辺からのオフセット」を表す符号付き値として保持する（例: `validrect.bottom,-56` は下端から内側 56、`wordwrappoint.x,-34` は右端から内側 34）。
2. The balloon parser shall `windowposition.x` について、シェル側方向を正・シェルから離れる方向を負とする符号を保持する。
3. The balloon parser shall `windowposition.y` について、下方向を正・上方向を負とする符号を保持する。
4. The balloon parser shall 負値座標を反対辺の実寸へ加算する最終ピクセル解決を行わず, バルーン画像サイズに依存しない符号付き値として消費側へ委ねる。
5. Where 入力の座標値が非負のとき, the balloon parser shall その値をそのまま型付き幾何として保持する。

### Requirement 5: emo2-kakukaku fixture 適合
**Objective:** M1 開発者として、emo2 実物 fixture で parser が正しくモデルを生成することを確認したい。そうすれば、emo2 が「そのまま動く」M1 ゴールに対する parser 側の適合を保証できる。

#### Acceptance Criteria
1. When emo2-kakukaku の balloon `descript.txt` を解析したとき, the balloon parser shall `origin`(0,0)・`wordwrappoint.x`(-34)・`validrect`(0,0,0,0)・`font.name`(Yu Gothic UI)・`font.height`(28)・`font.color`(0,0,0) を含むモデルを生成する。
2. When emo2-kakukaku の `descript.txt` と `balloons0s.txt` をマージ解析したとき, the balloon parser shall `windowposition`(266,-129)・`wordwrappoint.x`(-49)・`validrect`(46,-56,36,-44) を反映したモデルを生成する。
3. When emo2-kakukaku の `descript.txt` と `balloonk0s.txt` をマージ解析したとき, the balloon parser shall `windowposition`(-190,-75)・`validrect`(40,-70,24,-48) を反映したモデルを生成する。
4. The balloon parser shall 単体テスト（host 不要・純粋関数）のみで上記の適合を観測可能とする。
5. The balloon parser shall 2 例目の実物バルーンが要求するまで、emo2 が使用しない幾何・フォント以外の抽象を追加しない。
