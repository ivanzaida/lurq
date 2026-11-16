//! Size budgets for the values that render functions build and move by value.
//!
//! An unoptimized build gives every temporary its own stack slot, so the size
//! of these types multiplies into the stack depth of every render, rebuild and
//! layout recursion. 0.21 grew the theme role enums from 1 to 24 bytes and
//! `Element` from 2816 to 3616 bytes, and applications that fit the 1 MiB
//! Windows main-thread stack in debug builds on 0.20 overflowed on 0.22
//! (lurq#25, lurq#26). These checks fail the test build when a type outgrows
//! its budget; raise a budget only together with a stack measurement
//! (`scripts/stack-depth-probe.py`).
//!
//! Measured on x86_64 with every feature enabled: role enums 8, `Element` 8,
//! `Node` 1160 (what DevTools and MCP show of a node is boxed), `WindowChrome` 872,
//! `ChromeTitleBar` 760. The sizes depend on
//! the features that are on, and no two of the check's test runs turn all of
//! them on, so `scripts/check.ps1` also builds the tests once with every one.

use std::mem::size_of;

use crate::{
  app::theme::{BorderSize, RadiusSize, RoleName, ShadowStyle, SpacingSize, TypographyStyle},
  components::{ChromeTitleBar, WindowChrome},
  node::{Element, Node, SpacingValue, padding::Padding},
};

const _: () = assert!(size_of::<RoleName>() <= 4);
const _: () = assert!(size_of::<SpacingSize>() <= 8);
const _: () = assert!(size_of::<RadiusSize>() <= 8);
const _: () = assert!(size_of::<BorderSize>() <= 8);
const _: () = assert!(size_of::<TypographyStyle>() <= 8);
const _: () = assert!(size_of::<ShadowStyle>() <= 8);
const _: () = assert!(size_of::<SpacingValue>() <= 12);
const _: () = assert!(size_of::<Padding>() <= 48);

// An element is a pointer to its heap-allocated root node.
const _: () = assert!(size_of::<Element>() <= size_of::<usize>());
const _: () = assert!(size_of::<Node>() <= 1176);
// What every node keeps of its layout: the frame's subtree flags are packed in it.
const _: () = assert!(size_of::<crate::node::layout_cache::LayoutCache>() <= 120);
const _: () = assert!(size_of::<WindowChrome>() <= 1024);
const _: () = assert!(size_of::<ChromeTitleBar>() <= 1024);

