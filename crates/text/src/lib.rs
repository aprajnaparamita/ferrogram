//! Text layout for shape text blocks (ADR 0024).
//!
//! This crate lays out plain runs with formatting in a box: wrapping,
//! alignment, line spacing, bullets, tabs, vertical alignment, margins and
//! rotation. It cannot see `ferrogram-model` (ADR 0002): the renderer
//! passes plain data. Output is positioned glyphs and em-unit outlines for
//! the backends. Fonts are bundled data (Carlito and Selawik with their OFL
//! licences, AGENTS.md section 4) resolved through a [`FontStack`].
//!
//! Part of Ferrogram. Alpha.

#![forbid(unsafe_code)]

mod error;
mod font;
mod layout;

pub use error::Error;
pub use font::{CARLITO, Font, FontSource, FontStack, GlyphId, SELAWIK, default_fonts};
pub use layout::{
    Align, BULLET_RUN, Block, Bullet, Format, GlyphPos, Insets, Layout, Line, Paragraph, Run,
    VAlign, Warning, layout,
};
