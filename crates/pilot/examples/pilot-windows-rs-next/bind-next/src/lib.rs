// build.rs が OUT_DIR へ書いた既定の形の束縛。
#![allow(warnings)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
