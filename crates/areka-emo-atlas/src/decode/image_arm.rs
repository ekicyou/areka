//! `image` クレートで動く絵（APNG・WebP）の全コマ／動きの 1 枚目を読み、乗算済み BGRA へ直す
//! （spec: areka-P0-animated-image-decode 要件 2.1〜2.5・2.7・2.8・3.2・3.4・6.4・7.5）。
//! 本番のソースで `image` の型を綴る唯一のファイル。
