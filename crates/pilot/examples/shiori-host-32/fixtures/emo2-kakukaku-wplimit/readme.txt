areka-P0-windowposition-limit 実機サインオフ用の検証バルーン
============================================================

`fixtures/emo2/emo2-kakukaku` の複製である（実行に不要な Paint.NET 作業ファイル
`online.pdn` だけは落としてある）。差分は面別上書き層
（`balloons0s.txt` / `balloonk0s.txt`）の `windowposition` 3 行のみで、
既定層（`descript.txt`）とすべての画像は原本と同一。

  balloons0s.txt (scope 0 / sakura): windowposition.x,center  ← 中央上
  balloonk0s.txt (scope 1 / kero)  : windowposition.x,bottom  ← 中央下
  両者とも windowposition.limit,1（正典既定と同値だが明示して解決値ログで見えるようにした）
  windowposition.y は原本のまま（sakura −129 / kero −75）＝キーワード指定でも
  調整量が加算され続けることの確認を兼ねる（要件 4.4）。

原本を書き換えず別ディレクトリへ分けたのは、`fixtures/emo2` を読む既存テスト
（採寸・合成・起動）が原本の数値指定 x=266 / x=−190 を期待値に持つためである。
`fixtures/emo2` ツリーの外（兄弟ディレクトリ）に置いてあるので、ゴーストツリーの
列挙にも一切影響しない。

使い方は `.kiro/specs/areka-P0-windowposition-limit/signoff-procedure.md` を参照。
起動時に argv[2]（バルーンルート）としてこのディレクトリの絶対パスを渡す。
