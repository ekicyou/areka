# Requirements Document

## Project Description (Input)

- **困っている人**: 利用者とゴーストの作者。element定義の描画メソッド `base`（`element0,base,<絵>,X,Y`）を使うサーフェスが正しく描かれない。クローディア（`sample-ghost-kit` の `claudia`）の surface6・11・26 は `element0,base,surface0.png,0,0` の後に `element1,overlay,<顔の部品>` を重ねる形で、areka では上乗せの部品の大きさ（surface26＝100×56・surface11＝71×42）だけで描かれ、キャラクターが消えたように見える。
- **今の状況**: areka は element定義のうち描画メソッドが `overlay` の行だけを読み、ほかの描画メソッドの行を黙って捨てる。記録は何も出ない。起動直後の台詞（`\s[26]`）でも起きる。`element0,base,surfaceN.png` のように自分の番号の画像を指すサーフェス（surface19・29）は偶然正しく見え、別の画像を指すサーフェスだけが壊れる。spec `areka-P0-mouse-drag-events` の実機確認は、根の写しの `surfaces.txt` だけ `base` を `overlay` に書き替えて進めた。
- **変えたいこと**: element定義の `base` の行を読み、ukadoc の `base` の意味で描く。クローディアの surface6・11・26 が 333×500 で、土台の絵と顔の部品が重なって出る。`base` 以外の未対応の描画メソッドの行は、黙って捨てずに記録を 1 件残す。確定事項の詳細は同じフォルダの `brief.md`（2026-10-05 起票）にある。

## Introduction

本 spec は、シェルの surfaces.txt の element定義で、描画メソッド `base` の行を描けるようにする。段はバグ（キャラクターが消える・記録なし）である。

```
surface26
{
  element0,base,surface0.png,0,0
  element1,overlay,face26.png,120,80
}
```

この書き方は「surface0.png を土台にして、その上に顔の部品を重ねる」という意味である。今の areka は 1 行目を捨てるので、顔の部品だけが小さく描かれる。

正典との関係は次のとおり（いずれも `ukadoc:descript_shell_surfaces`）。

- element定義の項: 「サーフェスに対応するsurface*.pngという画像がある場合、element0が定義されていると元のsurface*.pngの内容が破棄されて、element0で置き換えられる」「element1以降は、その上に順次合成されていく」。
- 描画メソッド `base` の項: 「ベースサーフェスを新規レイヤで完全に置き換える」「着せ替えおよびelementでは、baseメソッドは最初（element0、pattern0）にしか用いることができない。それ以外はoverlayに読み変えられる」。
- この 2 つから、element定義の中の `base` は次のように決まる。`element0` では、置き換えられる側のベースサーフェスは破棄された後で何も無いので、`base` で置いた絵は、同じ絵を `overlay` で置いたときと同じ絵になる。`element1` 以降では、正典が `overlay` に読み替えると書いている。つまり **element定義の `base` は、どの番号でも、同じ行を `overlay` と書いたときと同じ見え方になる**。本 spec はこれを要件にする。
- `base` の項の「XY座標は無視される」は「この描画メソッドが指定されたpattern定義では」と限って書かれており、element定義の `base` の X,Y をどう扱うかを正典は書いていない。本 spec は上の読み方に従い、`overlay` の element定義と同じ扱いにして、areka の裁量として記録する（要件 4.4）。

## Boundary Context

- **In scope**:
  - `surface*`ブレス・`surface.append*`ブレスの element定義で、描画メソッドが `base` の行を読み、描くこと。
  - areka が element定義で描けない描画メソッドの行を、描かないまま、サーフェスと行が分かる記録を残すこと。
  - DLL を使わない決定論的なテストと、クローディア（無改変）での実機の確認。
  - 網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` の 2 行（element定義・`base`）と `doc/COMPAT_ARCHITECTURE.md` §8 の更新。
- **Out of scope**:
  - `base` 以外の描画メソッド（`overlayfast`／`overlay-fast`・`replace`・`interpolate`・`asis`・`reduce`・`blend-*` など）を element定義で描くこと。本 spec では記録を残すところまで。残りを描く仕事は、本 spec の完了時に `/kiro-discovery` で追跡用の spec を起票して持ち主を決める（2026-10-05 要件の討議・開発者の決定）。向きは「合成の方法は Direct2D が支える全部に対応できるよう、wintf も含めて広げる」（開発者の方針）で、起票する spec はこの向きに沿って範囲を決める。
  - pattern定義（`animation*.pattern*`）の描画メソッド `base`。今の振る舞い（描かずに警告を残す）のままにする。
  - element定義のオプション（`--clipping` ほか＝`element-clipping-option`）。
  - 動く絵の再生（`animated-image-playback`）。
  - サーフェスの外形の計算の規則（`extent-element-offset`）。
- **Adjacent expectations**:
  - 描画メソッドが `overlay` の element定義について今ある振る舞い——画像を置く書き方、ファイル名の欄が数字だけの書き方（areka 独自・`surface-element-nesting`）、areka 独自の描画メソッド `balloon`（`shell-balloon`）、サーフェスの番号の画像（`surface*.png`）を土台に敷く規則——はそのまま使い、変えない。
  - `base` の element定義の位置と外形は、同じ行を `overlay` と書いたときのものに従う。外形の規則そのものを直すのは `extent-element-offset` で、その修正が入れば `base` の行も同じ規則に乗る。
  - 描けない描画メソッドの `element0` を持つサーフェスで `surface*.png` が土台に残ること（正典の字面では破棄）は、今ある既知のずれで、`self-alpha-declaration` の議題が持つ（roadmap の「`shell-implicit-surface` の着地で残したもの」⑹）。本 spec は直さず、今の見え方を保つ（要件 2.5）。
  - `extent-element-offset`・`element-clipping-option`・`animated-image-playback` は同じ場所を触るので、本 spec と同時には走らせない。

## Requirements

### Requirement 1: element定義の描画メソッド `base` を描く

**Objective:** As a ゴーストの利用者とシェルの作者, I want `element0,base,<絵>,X,Y` で土台の絵を置いたサーフェスが正典どおりに描かれること, so that 土台の絵に顔の部品を重ねる書き方のシェルで、キャラクターが消えずに表示される

#### Acceptance Criteria

1. When `surface*`ブレスの `element0` の描画メソッドが `base` である場合, the areka shall その行を、描画メソッドを `overlay` と書いた同じ行（同じファイル名・同じ X,Y）と同じ絵・同じ大きさで描く
2. When `element0` が `base` で、その後ろに `overlay` の element定義が続く場合, the areka shall `element0` の絵を土台として、後ろの element定義を element番号の順（番号の大きいものが手前）にその上へ重ね、サーフェスの大きさを土台の絵を含む大きさにする
3. When サーフェスの番号に対応する画像（`surface*.png`）があり、そのサーフェスの `element0` が `base` である場合, the areka shall `surface*.png` を使わず、`element0` が指す絵を土台にする
4. When `element1` 以降の element定義の描画メソッドが `base` である場合, the areka shall その行を `overlay` と書いた同じ行と同じ絵で重ねる（ukadoc の `base` の項の読み替え）
5. When `surface.append*`ブレスの element定義の描画メソッドが `base` である場合, the areka shall 対象のサーフェスに、`surface*`ブレスに書いた場合と同じ規則で描く
6. When `base` の element定義のファイル名の欄が半角の数字だけで書かれている場合, the areka shall その行を、`overlay` で同じ番号のサーフェスを置いたときと同じ絵で描く
7. If `base` の element定義が指す画像を読み込めない場合, the areka shall `overlay` の element定義で画像を読み込めないときと同じ記録を残し、同じサーフェスの他の element定義は描く

### Requirement 2: 描けない描画メソッドの element定義を記録に残す

**Objective:** As a シェルの作者と areka の開発者, I want areka が描けない描画メソッドの element定義がどのサーフェスのどの行かを記録で知ること, so that 絵が欠けたときに、原因が未対応の描画メソッドだとすぐ分かる

#### Acceptance Criteria

1. If element定義の描画メソッドが、areka が element定義で描けるもの（`overlay`・`base`・areka 独自の `balloon`）のどれでもない場合, the areka shall ブレスの見出しに書かれたサーフェスの番号（複数の番号を並べた見出しではそのすべて）・element番号・書かれていた描画メソッドの語を含む警告を、その行について 1 件ログに残す
2. If element定義の描画メソッドの欄が空である、または ukadoc に無い語である場合, the areka shall 要件 2.1 と同じ形の警告を 1 件ログに残す
3. The areka shall 要件 2.1・2.2 の警告を、シェルを 1 回読み込むごとに該当する行 1 行につき 1 件だけ出し（見出しが複数の番号を並べていても 1 行は 1 件）、描画やアニメーションのたびに繰り返さない
4. If 描けない描画メソッドの element定義がサーフェスにある場合, the areka shall その行だけを描かず、同じサーフェスの他の element定義は描く
5. The areka shall 描けない描画メソッドの element定義を持つサーフェスの見え方（絵と大きさ）を、本 spec の前と同じに保つ（増えるのは警告の記録だけ）
6. When シェルのどの element定義も areka が描ける描画メソッドで書かれている場合, the areka shall 要件 2.1・2.2 の警告を 1 件も出さない

### Requirement 3: 今ある振る舞いを変えない

**Objective:** As a 既存のゴーストの利用者, I want この修正で、今正しく見えているサーフェスの見え方が変わらないこと, so that `base` を使わないシェルや、たまたま正しく見えていたサーフェスが壊れない

#### Acceptance Criteria

1. The areka shall 描画メソッドが `overlay` の element定義だけで書かれたサーフェスの絵と大きさを、本 spec の前と同じに保つ
2. When `element0` が `base` で自分のサーフェスの番号の画像（`element0,base,surfaceN.png,0,0`）を指している場合, the areka shall そのサーフェスを本 spec の前と同じ絵・同じ大きさで描く（縛るのは絵と大きさ。`surface*.png` を「使った」「`element0` が在るので使わなかった」の記録の内訳は変わってよい）
3. The areka shall pattern定義（`animation*.pattern*`）の描画メソッド `base` の振る舞い（描かずに警告を残す）を、本 spec の前と同じに保つ
4. The areka shall `element0` を持たないサーフェスで、サーフェスの番号の画像（`surface*.png`）を土台に敷く振る舞いを、本 spec の前と同じに保つ
5. The areka shall サーフェスの外形を決める規則を変えず、`base` の element定義を、外形の計算で `overlay` の同じ行と同じに扱う

### Requirement 4: 検証と記録の更新

**Objective:** As a areka の開発者, I want `base` の描画と未対応の描画メソッドの記録がテストと実機で固定され、台帳と設計文書が実際の状態を表すこと, so that 後の変更で同じ欠陥が黙って戻らず、対応の範囲を文書から正しく読める

#### Acceptance Criteria

1. The areka の決定論的なテスト shall DLL を使わずに、`element0,base,<サーフェスの番号と別の絵>` に `overlay` の element定義を重ねたサーフェスの大きさ（土台の絵の大きさ）と画素（土台の絵の画素と、重ねた部品の画素の両方）を判定する
2. The areka の決定論的なテスト shall 描けない描画メソッドの element定義について、警告がサーフェスの番号・element番号・描画メソッドの語を含んで 1 件出ること、および描ける描画メソッドだけのシェルで 0 件であることを判定する
3. When 無改変のクローディア（`sample-ghost-kit` の `claudia`）を実機で起動し `\s[6]`・`\s[11]`・`\s[26]` を表示した場合, the areka shall キャラクターを 333×500 の大きさで、土台の絵と顔の部品が重なった形で表示する
4. The `doc/COMPAT_ARCHITECTURE.md` §8 shall 「element定義の `base` の X,Y は、ukadoc が pattern定義についてだけ無視すると書いており element定義については書いていないため、areka は `overlay` の element定義と同じに扱う」ことを、ukadoc の該当の項を指して記録する
5. The 網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` shall element定義の行と `base` の行（`ukadoc:descript_shell_surfaces:base:1`）に、本 spec の後の実際の状態（element定義の `base` は描ける・pattern定義の `base` は未対応のまま・`overlay` と `base` 以外の描画メソッドの element定義は描かずに警告を残す）と担当の spec を書く。残りの描画メソッドを描く仕事の担当には、完了時に起票する追跡用の spec の名前を書く
