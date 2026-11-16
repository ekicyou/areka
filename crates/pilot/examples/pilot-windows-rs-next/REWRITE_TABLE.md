# 書き換えの対応表

<!-- 見出しだけ。行の形: | 項目 | 書き換え前（0.62） | 書き換え後（0.101） | 形（既定／.minimal()／.sys()） | 通した所（置き場のファイル）か「文書から」 | 注 | -->

## 戻り値の無い COM メソッドの HRESULT
## スライスの引数（ポインタ＋長さ）
## HWND（既定・.minimal()・.sys() の 3 行）と包みとの行き来
## 列挙の定数（i32／u32）
## windows-numerics のフィールド名
## #[implement] の _Impl の戻り値
## HSTRING の Display
## 束縛のフィルターへの API の足し方
## COM の型の ==（same_root・GraphicsCommandList の導出の PartialEq）
## == が番地だけの比べに変わる形（IUnknown へ cast して比べる書き方）
