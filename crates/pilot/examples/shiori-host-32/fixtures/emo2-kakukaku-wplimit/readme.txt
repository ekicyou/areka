areka-P0-windowposition-limit 実機サインオフ用の検証バルーン
============================================================

`fixtures/emo2/emo2-kakukaku` の複製である（実行に不要な Paint.NET 作業ファイル
`online.pdn` だけは落としてある）。原本との差分は次の 3 点で、既定層
（`descript.txt`）とすべての画像は原本と 1 バイトも違わない。

  (a) 面別上書き層（`balloons0s.txt` / `balloonk0s.txt`）の `windowposition` 各 3 行
  (b) `install.txt` の `name` と `directory`（複製であることを名前で判るようにした）
  (c) この `readme.txt`（新規追加）

面別上書き層の中身:

  balloons0s.txt (scope 0 / sakura): windowposition.x,center  ← 中央上
  balloonk0s.txt (scope 1 / kero)  : windowposition.x,bottom  ← 中央下
  両者とも windowposition.limit,1（正典既定と同値だが明示して解決値ログで見えるようにした）
  両者とも windowposition.y,0（原本は sakura −129 / kero −75。検証の途中で 0 へ変更した）

`windowposition.y` を 0 にした理由: 原本の y は「数値指定用に作られた値」で、数値指定の
基本位置（バルーン上端＝キャラ上端）を前提にしている。キーワードの基本位置は
（中央上なら）バルーン下端がキャラ上端に接する位置なので、同じ y を流用すると
バルーン高さぶん余計に浮き、「基本位置が意図どおりか」の目視（手順書 §5 の 6）が
交絡して判定しにくくなる。0 にすると基本位置が素で見える。

したがって手順書 §5 の 6 を現行ファイルのまま実施する場合、「ずれ無し」（キャラ画像の
真上・真下の水平中央）が合格の見え方である。y を非ゼロへ戻した場合はその値ぶん
上へずれているのが正しい。なお「キーワード指定でも調整量の加算が続く」（要件 4.4）の
実機証跡は、y が原本のままだった 200% の走行が既に持っている（手順書 §7.1）。

原本を書き換えず別ディレクトリへ分けたのは、`fixtures/emo2` を読む既存テスト
（採寸・合成・起動）が原本の数値指定 x=266 / x=−190 を期待値に持つためである。
`fixtures/emo2` ツリーの外（兄弟ディレクトリ）に置いてあるので、ゴーストツリーの
列挙にも一切影響しない。

使い方は `.kiro/specs/areka-P0-windowposition-limit/signoff-procedure.md` を参照。
起動時に argv[2]（バルーンルート）としてこのディレクトリの絶対パスを渡す。
