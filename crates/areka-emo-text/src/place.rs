//! # place — 文字の場所の鍵（純粋層）
//!
//! 文字（`ActorTextState`）を持つ単位は「スコープ」と「普通のバルーン／箱の名前」の組
//! （[`PlaceKey`]）である（要件 4.6）。表の鍵は構造のある型で持ち、文字列の連結を鍵にしない。
//! 普通のバルーンは [`TextPlace::Balloon`]、シェル内バルーンの箱は `balloon.*`ブレスの名前
//! （[`BoxName`]）で指す。

use areka_emo_compose::BoxName;
use areka_sakura::contract::ActorKey;

/// スコープの中の文字の場所。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextPlace {
    /// バルーン専用の窓に出る普通のバルーン。
    Balloon,
    /// element定義の描画メソッド `balloon` で置かれた箱。
    Box(BoxName),
}

/// スコープと文字の場所の組。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlaceKey {
    pub actor: ActorKey,
    pub place: TextPlace,
}

impl PlaceKey {
    /// スコープの普通のバルーンの場所。
    pub fn balloon(actor: &ActorKey) -> PlaceKey {
        PlaceKey {
            actor: actor.clone(),
            place: TextPlace::Balloon,
        }
    }
}
