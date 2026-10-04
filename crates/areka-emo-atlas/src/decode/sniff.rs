//! 動く絵の見分け: ファイルの見出し（PNG のチャンク・WebP の RIFF チャンク）だけを読み、
//! コマが 2 枚以上の APNG・WebP かどうかとコマの枚数・寸法を答える
//! （spec: areka-P0-animated-image-decode 要件 1.1〜1.5・6.3）。画素は解かない。
