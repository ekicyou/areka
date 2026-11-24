pub(crate) mod desk;
mod edit;
pub(crate) mod pick;

#[cfg(test)]
mod roots_test_support;

pub(crate) use desk::install;
pub(crate) use edit::{add_root, move_to_front, remove_root};
