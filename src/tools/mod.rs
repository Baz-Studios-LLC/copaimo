//! What the maker's tools are built from.
//!
//! The terrain tool and the workbench are different jobs — one moves a hillside,
//! the other places a fence rail — but they are the same KIND of thing, and they
//! should look and behave like it. A maker who has learned one panel should not
//! have to learn the second.
//!
//! So the visual language and the widgets live here, once, rather than inside
//! whichever tool grew them first. That is not tidiness: the workbench got a wall
//! of text at the top of the screen instead of a panel precisely because the panel
//! was locked inside the terrain tool, and the choice at the time was between
//! copying it and lifting it out.
//!
//! Compiled only into a maker's build. See the `tools` feature.

pub mod theme;
pub mod widget;
