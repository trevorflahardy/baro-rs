//! WiFi status page
//!
//! Displays a status screen for WiFi connection state — either "Connecting"
//! (with a spinner-like indicator) or "Error" (with a disconnected icon and
//! a non-functional "Connect" button placeholder).
//!
//! Layout is built using the [`Container`] system for automatic centering
//! and sizing. Icons (grid, wifi) are drawn as overlays since there is no
//! icon Element variant.
//!
//! ```text
//! ┌──────────────────────────────────────┐
//! │  ▫  AIR AROUND YOU         ≈ (icon)  │  ← header (Container)
//! ├──────────────────────────────────────┤
//! │                                      │
//! │           ( n o n )   or  ...        │  ← status text
//! │                                      │
//! │       No Wi-Fi Connection            │  ← title
//! │       Data cannot be updated.        │  ← subtitle
//! │                                      │
//! │       [ <-> CONNECT TO WI-FI ]       │  ← button (noop)
//! │                                      │
//! └──────────────────────────────────────┘
//! ```

use embedded_graphics::Drawable as EgDrawable;
use embedded_graphics::geometry::{Point, Size};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};

extern crate alloc;
use alloc::boxed::Box;

use crate::pages::page::Page;
use crate::ui::core::{Action, Drawable, PageId, TouchEvent};
use crate::ui::styling::{
    COLOR_BACKGROUND, COLOR_FOREGROUND, DISPLAY_HEIGHT_PX, DISPLAY_WIDTH_PX, WHITE,
};
use crate::ui::{
    Alignment as UiAlignment, Button, ButtonVariant, ColorPalette, Container, Direction, Element,
    MAX_CONTAINER_CHILDREN, MainAxisAlignment, Padding, SizeConstraint, Style, TextComponent,
    TextSize,
};

// ---------------------------------------------------------------------------
// Layout constants
// ---------------------------------------------------------------------------

/// Height of the top header bar in pixels.
const HEADER_HEIGHT_PX: u32 = 36;

/// Left padding inside the header (space for the grid icon).
const HEADER_LEFT_PADDING_PX: u32 = 36;

/// Right padding inside the header.
const HEADER_RIGHT_PADDING_PX: u32 = 12;

/// Gap between body content items (status text → title → subtitle).
const BODY_CONTENT_GAP_PX: u32 = 4;

/// Height of the button element.
const BUTTON_HEIGHT_PX: u32 = 34;

/// Grid icon square size in the header.
const GRID_ICON_SQUARE_PX: u32 = 6;

/// Grid icon gap between squares.
const GRID_ICON_GAP_PX: i32 = 2;

/// Grid icon left offset from header left edge.
const GRID_ICON_LEFT_PX: i32 = 12;

// ---------------------------------------------------------------------------
// Colors
// ---------------------------------------------------------------------------

/// Cyan accent used for the connecting state text.
const COLOR_ACCENT_CYAN: Rgb565 = Rgb565::new(0, 50, 31);

/// Muted gray for subtitle / secondary text.
const COLOR_TEXT_MUTED: Rgb565 = Rgb565::new(14, 28, 14);

/// Light grayish text for the header title.
const COLOR_HEADER_TEXT: Rgb565 = Rgb565::new(20, 40, 20);

// ---------------------------------------------------------------------------
// WiFi connection state
// ---------------------------------------------------------------------------

/// Describes the current WiFi connection status displayed by the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WifiState {
    /// WiFi is currently attempting to connect.
    Connecting,
    /// WiFi connection failed or is unavailable.
    Error,
}

impl WifiState {
    /// Large status text rendered in the centre of the page.
    fn status_text(self) -> &'static str {
        match self {
            Self::Connecting => ". . .",
            Self::Error => "( n o n )",
        }
    }

    /// Primary title line beneath the status text.
    fn title_text(self) -> &'static str {
        match self {
            Self::Connecting => "Connecting to Wi-Fi",
            Self::Error => "No Wi-Fi Connection",
        }
    }

    /// Secondary subtitle.
    fn subtitle(self) -> &'static str {
        match self {
            Self::Connecting => "Please wait...",
            Self::Error => "Data cannot be updated.",
        }
    }

    /// Accent color used for the status text.
    fn accent_color(self) -> Rgb565 {
        match self {
            Self::Connecting => COLOR_ACCENT_CYAN,
            Self::Error => COLOR_TEXT_MUTED,
        }
    }
}

// ---------------------------------------------------------------------------
// WifiStatusPage
// ---------------------------------------------------------------------------

/// Full-screen bounds for the page.
fn page_bounds() -> Rectangle {
    Rectangle::new(
        Point::zero(),
        Size::new(DISPLAY_WIDTH_PX as u32, DISPLAY_HEIGHT_PX as u32),
    )
}

/// A combined WiFi connecting / error page.
///
/// Uses the [`Container`] layout system for automatic positioning and
/// centering. Icons (grid, wifi) are drawn as overlays.
pub struct WifiStatusPage {
    state: WifiState,
    root: Container<2>,
    dirty: bool,
}

impl WifiStatusPage {
    /// Create the page in the given initial state.
    pub fn new(state: WifiState) -> Self {
        let mut page = Self {
            state,
            root: Container::new(page_bounds(), Direction::Vertical),
            dirty: true,
        };
        page.rebuild_layout();
        page
    }

    /// Update the displayed state, marking the page dirty if it changed.
    pub fn set_state(&mut self, state: WifiState) {
        if self.state != state {
            self.state = state;
            self.rebuild_layout();
            self.dirty = true;
        }
    }

    /// Current state.
    pub fn state(&self) -> WifiState {
        self.state
    }

    // -- layout construction -----------------------------------------------

    /// Rebuild the root container tree for the current state.
    fn rebuild_layout(&mut self) {
        let bounds = page_bounds();

        let mut root =
            Container::<2>::new(bounds, Direction::Vertical).with_alignment(UiAlignment::Stretch);

        // ── Header row ──────────────────────────────────────────────────
        let header_text = TextComponent::auto("AIR AROUND YOU", TextSize::Medium)
            .with_style(Style::new().with_foreground(COLOR_HEADER_TEXT));

        let header = Container::<MAX_CONTAINER_CHILDREN>::new(
            Rectangle::new(
                Point::zero(),
                Size::new(bounds.size.width, HEADER_HEIGHT_PX),
            ),
            Direction::Horizontal,
        )
        .with_alignment(UiAlignment::Center)
        .with_main_axis_alignment(MainAxisAlignment::Start)
        .with_style(Style::new().with_background(COLOR_FOREGROUND))
        .with_padding(Padding::new(
            0,
            HEADER_RIGHT_PADDING_PX,
            0,
            HEADER_LEFT_PADDING_PX,
        ))
        .with_child(Element::Text(Box::new(header_text)), SizeConstraint::Fit);

        let _ = root.add_child(
            Element::container(header),
            SizeConstraint::Fixed(HEADER_HEIGHT_PX),
        );

        // ── Body content (vertically centred in remaining space) ─────────
        // Use full page bounds (not Rectangle::zero()) so that intermediate
        // layout passes give children realistic widths.  The root container
        // will override these bounds with the actual remaining space, but
        // preferred_size() reads current bounds, so starting at zero would
        // corrupt child widths to 0 and break centering.
        let mut body = Container::<MAX_CONTAINER_CHILDREN>::new(bounds, Direction::Vertical)
            .with_alignment(UiAlignment::Center)
            .with_main_axis_alignment(MainAxisAlignment::Center)
            .with_gap(BODY_CONTENT_GAP_PX);

        // Status text
        let status = TextComponent::auto(self.state.status_text(), TextSize::Large)
            .with_style(Style::new().with_foreground(self.state.accent_color()));
        let _ = body.add_child(Element::Text(Box::new(status)), SizeConstraint::Fit);

        // Title
        let title = TextComponent::auto(self.state.title_text(), TextSize::Large)
            .with_style(Style::new().with_foreground(WHITE));
        let _ = body.add_child(Element::Text(Box::new(title)), SizeConstraint::Fit);

        // Subtitle
        let subtitle = TextComponent::auto(self.state.subtitle(), TextSize::Small)
            .with_style(Style::new().with_foreground(COLOR_TEXT_MUTED));
        let _ = body.add_child(Element::Text(Box::new(subtitle)), SizeConstraint::Fit);

        // Button (only in error state)
        if self.state == WifiState::Error {
            // Small spacer before button
            let _ = body.add_child(Element::spacer(Rectangle::zero()), SizeConstraint::Fixed(8));

            let palette = ColorPalette {
                surface: COLOR_FOREGROUND,
                text_primary: COLOR_ACCENT_CYAN,
                border: COLOR_TEXT_MUTED,
                ..ColorPalette::default()
            };

            let btn = Button::auto("CONNECT TO WI-FI", Action::Custom(0))
                .with_variant(ButtonVariant::Outline)
                .with_palette(palette);
            let _ = body.add_child(
                Element::Button(Box::new(btn)),
                SizeConstraint::Fixed(BUTTON_HEIGHT_PX),
            );
        }

        let _ = root.add_child(Element::container(body), SizeConstraint::Grow(1));

        self.root = root;
    }

    // -- icon overlays -----------------------------------------------------

    /// Draw the 2×2 grid icon in the top-left of the header.
    fn draw_grid_icon<D: DrawTarget<Color = Rgb565>>(
        &self,
        display: &mut D,
    ) -> Result<(), D::Error> {
        let sq_style = PrimitiveStyle::with_fill(COLOR_HEADER_TEXT);

        // Vertically centre the icon block within the header.
        let icon_block_height = GRID_ICON_SQUARE_PX * 2 + GRID_ICON_GAP_PX as u32;
        let icon_top = (HEADER_HEIGHT_PX.saturating_sub(icon_block_height) / 2) as i32;

        for row in 0..2i32 {
            for col in 0..2i32 {
                Rectangle::new(
                    Point::new(
                        GRID_ICON_LEFT_PX + col * (GRID_ICON_SQUARE_PX as i32 + GRID_ICON_GAP_PX),
                        icon_top + row * (GRID_ICON_SQUARE_PX as i32 + GRID_ICON_GAP_PX),
                    ),
                    Size::new(GRID_ICON_SQUARE_PX, GRID_ICON_SQUARE_PX),
                )
                .into_styled(sq_style)
                .draw(display)?;
            }
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Page trait
// ---------------------------------------------------------------------------

impl Page for WifiStatusPage {
    fn id(&self) -> PageId {
        PageId::WifiStatus
    }

    fn title(&self) -> &str {
        match self.state {
            WifiState::Connecting => "WiFi Connecting",
            WifiState::Error => "WiFi Error",
        }
    }

    fn on_activate(&mut self) {
        self.dirty = true;
    }

    fn handle_touch(&mut self, _event: TouchEvent) -> Option<Action> {
        // Button does nothing for now
        None
    }

    fn update(&mut self) {
        // No periodic updates needed
    }

    fn draw_page<D: DrawTarget<Color = Rgb565>>(
        &mut self,
        display: &mut D,
    ) -> Result<(), D::Error> {
        Drawable::draw(self, display)
    }

    fn bounds(&self) -> Rectangle {
        Drawable::bounds(self)
    }

    fn is_dirty(&self) -> bool {
        Drawable::is_dirty(self)
    }

    fn mark_clean(&mut self) {
        Drawable::mark_clean(self)
    }

    fn mark_dirty(&mut self) {
        Drawable::mark_dirty(self)
    }
}

// ---------------------------------------------------------------------------
// Drawable trait
// ---------------------------------------------------------------------------

impl Drawable for WifiStatusPage {
    fn draw<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        if !self.dirty {
            return Ok(());
        }

        // Full-screen dark background
        display.clear(COLOR_BACKGROUND)?;

        // Container draws the header background, "AIR AROUND YOU" text (vertically
        // centred), body content (centrally positioned), and button.
        self.root.draw(display)?;

        // Overlay: grid icon in header (not representable as an Element).
        self.draw_grid_icon(display)?;

        Ok(())
    }

    fn bounds(&self) -> Rectangle {
        page_bounds()
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn mark_clean(&mut self) {
        self.dirty = false;
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}
