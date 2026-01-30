# Baro UI System - Practical Examples

This document provides cookbook-style examples for common UI patterns in the Baro UI system.

## Table of Contents

- [Basic Layouts](#basic-layouts)
- [Cards and Panels](#cards-and-panels)
- [Lists and Grids](#lists-and-grids)
- [Forms and Input](#forms-and-input)
- [Navigation Patterns](#navigation-patterns)
- [Data Display](#data-display)
- [Complete Page Examples](#complete-page-examples)

---

## Basic Layouts

### Simple Vertical Stack

Stack elements vertically with spacing:

```rust
use crate::ui::{Container, Direction, Alignment, SizeConstraint, Padding, Style};
use embedded_graphics::pixelcolor::Rgb565;

let mut container = Container::<3>::new(
    Rectangle::new(Point::new(10, 10), Size::new(300, 200)),
    Direction::Vertical,
)
.with_spacing(8)
.with_padding(Padding::all(12))
.with_style(Style::new().with_background(Rgb565::BLACK));

// Add three children - header (fixed), content (expands), footer (fixed)
container.add_child(Size::new(276, 40), SizeConstraint::Fixed(40))?;  // Header
container.add_child(Size::new(276, 0), SizeConstraint::Expand)?;     // Content expands
container.add_child(Size::new(276, 30), SizeConstraint::Fixed(30))?; // Footer
```

### Horizontal Button Group

Create a row of buttons:

```rust
use crate::ui::{Container, Direction, Alignment, SizeConstraint};

let mut button_row = Container::<3>::new(
    Rectangle::new(Point::new(10, 180), Size::new(300, 50)),
    Direction::Horizontal,
)
.with_spacing(10)
.with_alignment(Alignment::Center);

// Three equal-width buttons
button_row.add_child(Size::new(0, 50), SizeConstraint::Expand)?;
button_row.add_child(Size::new(0, 50), SizeConstraint::Expand)?;
button_row.add_child(Size::new(0, 50), SizeConstraint::Expand)?;
```

### Centered Content

Center a card in the middle of the screen:

```rust
use crate::ui::{Container, Direction, Alignment, SizeConstraint};

// Outer container fills screen, centers children
let mut centering_container = Container::<1>::new(
    Rectangle::new(Point::zero(), Size::new(320, 240)),
    Direction::Vertical,
)
.with_alignment(Alignment::Center);

// Add centered card (280x180)
centering_container.add_child(Size::new(280, 180), SizeConstraint::Fit)?;

// To horizontally center as well, nest another container
let mut h_center = Container::<1>::new(
    container.child_bounds(0).unwrap(),
    Direction::Horizontal,
)
.with_alignment(Alignment::Center);

h_center.add_child(Size::new(200, 180), SizeConstraint::Fit)?;
```

---

## Cards and Panels

### Simple Card

A rounded card with padding:

```rust
use crate::ui::{Container, Direction, Padding, Style};
use crate::ui::styling::{COLOR_FOREGROUND, WHITE};

let card = Container::<3>::new(
    Rectangle::new(Point::new(20, 20), Size::new(280, 120)),
    Direction::Vertical,
)
.with_padding(Padding::all(16))
.with_spacing(8)
.with_corner_radius(12)
.with_style(Style::new().with_background(COLOR_FOREGROUND));

// Add card content: title, body text, button
// Each child should be added with appropriate constraints
```

### Card with Header

Card with a colored header section:

```rust
use crate::ui::{Container, Direction, Alignment, SizeConstraint, Padding, Style};
use crate::ui::styling::{COLOR_FOREGROUND, COLOR_EXCELLENT_FOREGROUND, WHITE};

// Main card container
let mut card = Container::<2>::new(
    Rectangle::new(Point::new(20, 20), Size::new(280, 150)),
    Direction::Vertical,
)
.with_corner_radius(12)
.with_style(Style::new().with_background(COLOR_FOREGROUND));

// Header (fixed height, colored background)
card.add_child(Size::new(280, 50), SizeConstraint::Fixed(50))?;
let header_bounds = card.child_bounds(0).unwrap();

let mut header = Container::<1>::new(header_bounds, Direction::Vertical)
    .with_padding(Padding::all(12))
    .with_style(Style::new().with_background(COLOR_EXCELLENT_FOREGROUND));

// Body (expands to fill remaining space)
card.add_child(Size::new(280, 0), SizeConstraint::Expand)?;
let body_bounds = card.child_bounds(1).unwrap();

let body = Container::<2>::new(body_bounds, Direction::Vertical)
    .with_padding(Padding::all(12))
    .with_spacing(8);
```

### Info Card with Icon Area

A card with left icon area and right content:

```rust
use crate::ui::{Container, Direction, Alignment, SizeConstraint, Padding, Style};
use crate::ui::styling::{COLOR_FOREGROUND, COLOR_EXCELLENT_FOREGROUND};

let mut card = Container::<2>::new(
    Rectangle::new(Point::new(20, 20), Size::new(280, 80)),
    Direction::Horizontal,
)
.with_padding(Padding::all(12))
.with_spacing(12)
.with_corner_radius(8)
.with_alignment(Alignment::Center)
.with_style(Style::new().with_background(COLOR_FOREGROUND));

// Icon area (fixed width)
card.add_child(Size::new(60, 60), SizeConstraint::Fixed(60))?;

// Content area (expands to fill)
card.add_child(Size::new(0, 60), SizeConstraint::Expand)?;
```

---

## Lists and Grids

### Vertical List

Create a scrollable list of items:

```rust
use crate::ui::{Container, ScrollableContainer, ScrollDirection};
use crate::ui::{Direction, SizeConstraint, Padding, Style};
use crate::ui::styling::COLOR_FOREGROUND;

// Viewport for the list
let viewport = Rectangle::new(Point::new(10, 40), Size::new(300, 180));

// Content larger than viewport
let content_size = Size::new(300, 600);

let mut scrollable = ScrollableContainer::new(
    viewport,
    content_size,
    ScrollDirection::Vertical,
);

// Create list container inside scrollable content
let mut list = Container::<10>::new(
    Rectangle::new(Point::zero(), content_size),
    Direction::Vertical,
)
.with_spacing(8)
.with_padding(Padding::all(8));

// Add list items (each 80px high)
for _ in 0..10 {
    list.add_child(Size::new(284, 80), SizeConstraint::Fixed(80))?;
}

// Each list item can be a card:
if let Some(item_bounds) = list.child_bounds(0) {
    let item = Container::<2>::new(item_bounds, Direction::Horizontal)
        .with_padding(Padding::all(12))
        .with_corner_radius(8)
        .with_style(Style::new().with_background(COLOR_FOREGROUND));
}
```

### Grid Layout (2 Columns)

Create a 2-column grid:

```rust
use crate::ui::{Container, Direction, SizeConstraint, Padding};

// Main container (vertical for rows)
let mut grid = Container::<4>::new(
    Rectangle::new(Point::new(10, 40), Size::new(300, 180)),
    Direction::Vertical,
)
.with_spacing(8)
.with_padding(Padding::all(8));

// Add rows (each row is a horizontal container)
for _ in 0..4 {
    grid.add_child(Size::new(284, 40), SizeConstraint::Fixed(40))?;
}

// Each row is a horizontal container with 2 cells
let row_bounds = grid.child_bounds(0).unwrap();
let mut row = Container::<2>::new(row_bounds, Direction::Horizontal)
    .with_spacing(8);

// Two equal-width cells
row.add_child(Size::new(0, 40), SizeConstraint::Expand)?;
row.add_child(Size::new(0, 40), SizeConstraint::Expand)?;
```

---

## Forms and Input

### Form Layout

Vertical form with labels and input areas:

```rust
use crate::ui::{Container, Direction, SizeConstraint, Padding, Style};
use crate::ui::styling::{COLOR_BACKGROUND, COLOR_FOREGROUND};

let mut form = Container::<6>::new(
    Rectangle::new(Point::new(20, 20), Size::new(280, 200)),
    Direction::Vertical,
)
.with_spacing(12)
.with_padding(Padding::all(16))
.with_corner_radius(12)
.with_style(Style::new().with_background(COLOR_FOREGROUND));

// Title
form.add_child(Size::new(248, 30), SizeConstraint::Fixed(30))?;

// Field 1 (label + input)
form.add_child(Size::new(248, 50), SizeConstraint::Fixed(50))?;

// Field 2 (label + input)
form.add_child(Size::new(248, 50), SizeConstraint::Fixed(50))?;

// Spacer
form.add_child(Size::new(248, 0), SizeConstraint::Expand)?;

// Buttons row
form.add_child(Size::new(248, 40), SizeConstraint::Fixed(40))?;
```

### Input Field with Label

Label above input area:

```rust
use crate::ui::{Container, Direction, SizeConstraint, Padding, Style};
use crate::ui::styling::{COLOR_BACKGROUND, DARK_GRAY};

let mut field = Container::<2>::new(
    Rectangle::new(Point::new(0, 0), Size::new(240, 50)),
    Direction::Vertical,
)
.with_spacing(4);

// Label (small text, fixed height)
field.add_child(Size::new(240, 15), SizeConstraint::Fixed(15))?;

// Input area (larger, with border)
field.add_child(Size::new(240, 30), SizeConstraint::Fixed(30))?;

let input_bounds = field.child_bounds(1).unwrap();
let input = Container::<1>::new(input_bounds, Direction::Horizontal)
    .with_padding(Padding::symmetric(6, 8))
    .with_corner_radius(4)
    .with_style(Style::new()
        .with_background(COLOR_BACKGROUND)
        .with_border(DARK_GRAY, 1));
```

---

## Navigation Patterns

### Tab Bar

Bottom tab navigation:

```rust
use crate::ui::{Container, Direction, Alignment, SizeConstraint};

let tab_bar = Container::<4>::new(
    Rectangle::new(Point::new(0, 190), Size::new(320, 50)),
    Direction::Horizontal,
)
.with_alignment(Alignment::Center);

// Each tab button gets equal space
// Add 4 tabs (will be evenly distributed)
```

### Navigation Header

Top navigation with title and buttons:

```rust
use crate::ui::{Container, Direction, Alignment, SizeConstraint, Padding, Style};
use crate::ui::styling::COLOR_EXCELLENT_FOREGROUND;

let mut header = Container::<3>::new(
    Rectangle::new(Point::zero(), Size::new(320, 50)),
    Direction::Horizontal,
)
.with_padding(Padding::symmetric(8, 12))
.with_alignment(Alignment::Center)
.with_style(Style::new().with_background(COLOR_EXCELLENT_FOREGROUND));

// Back button (fixed width)
header.add_child(Size::new(40, 34), SizeConstraint::Fixed(40))?;

// Title (expands to fill)
header.add_child(Size::new(0, 34), SizeConstraint::Expand)?;

// Action button (fixed width)
header.add_child(Size::new(40, 34), SizeConstraint::Fixed(40))?;
```

---

## Data Display

### Sensor Reading Card

Display sensor data with label and large value:

```rust
use crate::ui::{Container, Direction, Alignment, SizeConstraint, Padding, Style};
use crate::ui::styling::{COLOR_FOREGROUND, COLOR_EXCELLENT_FOREGROUND};

let mut sensor_card = Container::<3>::new(
    Rectangle::new(Point::new(20, 60), Size::new(280, 100)),
    Direction::Vertical,
)
.with_padding(Padding::all(16))
.with_spacing(4)
.with_corner_radius(12)
.with_alignment(Alignment::Start)
.with_style(Style::new().with_background(COLOR_FOREGROUND));

// Sensor name (small)
sensor_card.add_child(Size::new(248, 15), SizeConstraint::Fixed(15))?;

// Large value display
sensor_card.add_child(Size::new(248, 40), SizeConstraint::Fixed(40))?;

// Unit and timestamp (small)
sensor_card.add_child(Size::new(248, 15), SizeConstraint::Fixed(15))?;
```

### Status Indicator

Small status card with color indicator:

```rust
use crate::ui::{Container, Direction, Alignment, SizeConstraint, Padding, Style};
use crate::ui::styling::{COLOR_FOREGROUND, COLOR_EXCELLENT_FOREGROUND};

let mut status_card = Container::<2>::new(
    Rectangle::new(Point::new(20, 20), Size::new(135, 60)),
    Direction::Horizontal,
)
.with_padding(Padding::all(10))
.with_spacing(10)
.with_corner_radius(8)
.with_alignment(Alignment::Center)
.with_style(Style::new().with_background(COLOR_FOREGROUND));

// Status indicator dot (fixed)
status_card.add_child(Size::new(12, 12), SizeConstraint::Fixed(12))?;
let dot_bounds = status_card.child_bounds(0).unwrap();

let dot = Container::<0>::new(dot_bounds, Direction::Vertical)
    .with_corner_radius(6)
    .with_style(Style::new().with_background(COLOR_EXCELLENT_FOREGROUND));

// Status text (expands)
status_card.add_child(Size::new(0, 40), SizeConstraint::Expand)?;
```

### Multi-Metric Dashboard

Grid of metric cards:

```rust
use crate::ui::{Container, Direction, SizeConstraint, Padding};

// Main dashboard
let mut dashboard = Container::<3>::new(
    Rectangle::new(Point::new(10, 60), Size::new(300, 170)),
    Direction::Vertical,
)
.with_spacing(10)
.with_padding(Padding::all(10));

// Row 1: Two cards side-by-side
dashboard.add_child(Size::new(280, 50), SizeConstraint::Fixed(50))?;

let row1_bounds = dashboard.child_bounds(0).unwrap();
let mut row1 = Container::<2>::new(row1_bounds, Direction::Horizontal)
    .with_spacing(10);
row1.add_child(Size::new(0, 50), SizeConstraint::Expand)?;
row1.add_child(Size::new(0, 50), SizeConstraint::Expand)?;

// Row 2: Single wide card
dashboard.add_child(Size::new(280, 50), SizeConstraint::Fixed(50))?;

// Row 3: Three small cards
dashboard.add_child(Size::new(280, 50), SizeConstraint::Fixed(50))?;

let row3_bounds = dashboard.child_bounds(2).unwrap();
let mut row3 = Container::<3>::new(row3_bounds, Direction::Horizontal)
    .with_spacing(5);
row3.add_child(Size::new(0, 50), SizeConstraint::Expand)?;
row3.add_child(Size::new(0, 50), SizeConstraint::Expand)?;
row3.add_child(Size::new(0, 50), SizeConstraint::Expand)?;
```

---

## Complete Page Examples

### Login Screen

Complete login page with logo, form, and button:

```rust
use crate::ui::{Container, Direction, Alignment, SizeConstraint, Padding, Style};
use crate::ui::{Button, TextComponent, TextSize, ButtonVariant};
use crate::ui::styling::{COLOR_BACKGROUND, COLOR_FOREGROUND, WHITE};
use embedded_graphics::prelude::*;

pub struct LoginPage {
    bounds: Rectangle,
    main_container: Container<3>,
    form_container: Container<4>,
    login_button: Button,
}

impl LoginPage {
    pub fn new(bounds: Rectangle) -> Self {
        // Main container (centers content vertically)
        let mut main = Container::<3>::new(bounds, Direction::Vertical)
            .with_alignment(Alignment::Center)
            .with_padding(Padding::all(20))
            .with_style(Style::new().with_background(COLOR_BACKGROUND));

        // Logo area
        main.add_child(Size::new(280, 60), SizeConstraint::Fixed(60)).ok();

        // Form card
        main.add_child(Size::new(280, 180), SizeConstraint::Fixed(180)).ok();

        // Spacer
        main.add_child(Size::new(280, 20), SizeConstraint::Fixed(20)).ok();

        // Form container
        let form_bounds = main.child_bounds(1).unwrap();
        let mut form = Container::<4>::new(form_bounds, Direction::Vertical)
            .with_padding(Padding::all(20))
            .with_spacing(16)
            .with_corner_radius(12)
            .with_style(Style::new().with_background(COLOR_FOREGROUND));

        form.add_child(Size::new(240, 30), SizeConstraint::Fixed(30)).ok(); // Title
        form.add_child(Size::new(240, 40), SizeConstraint::Fixed(40)).ok(); // Username
        form.add_child(Size::new(240, 40), SizeConstraint::Fixed(40)).ok(); // Password
        form.add_child(Size::new(240, 45), SizeConstraint::Fixed(45)).ok(); // Button

        let button_bounds = form.child_bounds(3).unwrap();
        let login_button = Button::new(
            button_bounds,
            "Login",
            Action::Custom(1),
        )
        .with_variant(ButtonVariant::Primary)
        .with_border_radius(8);

        Self {
            bounds,
            main_container: main,
            form_container: form,
            login_button,
        }
    }
}
```

### Dashboard with Header and Cards

Complete dashboard page:

```rust
use crate::ui::{Container, Direction, Alignment, SizeConstraint, Padding, Style};
use crate::ui::{TextComponent, TextSize};
use crate::ui::styling::{COLOR_BACKGROUND, COLOR_FOREGROUND, COLOR_EXCELLENT_FOREGROUND, WHITE};

pub struct DashboardPage {
    bounds: Rectangle,
    layout: Container<3>,
    sensor_cards: Container<4>,
    temp_text: TextComponent,
    humidity_text: TextComponent,
}

impl DashboardPage {
    pub fn new(bounds: Rectangle) -> Self {
        // Main layout: header + content + footer
        let mut layout = Container::<3>::new(bounds, Direction::Vertical)
            .with_style(Style::new().with_background(COLOR_BACKGROUND));

        layout.add_child(Size::new(320, 60), SizeConstraint::Fixed(60)).ok(); // Header
        layout.add_child(Size::new(320, 0), SizeConstraint::Expand).ok();    // Content
        layout.add_child(Size::new(320, 50), SizeConstraint::Fixed(50)).ok(); // Footer

        // Header
        let header_bounds = layout.child_bounds(0).unwrap();
        let header = Container::<1>::new(header_bounds, Direction::Horizontal)
            .with_padding(Padding::symmetric(16, 20))
            .with_alignment(Alignment::Center)
            .with_style(Style::new().with_background(COLOR_EXCELLENT_FOREGROUND));

        // Content area with sensor cards
        let content_bounds = layout.child_bounds(1).unwrap();
        let mut cards = Container::<4>::new(content_bounds, Direction::Vertical)
            .with_padding(Padding::all(16))
            .with_spacing(12);

        // Add 4 sensor cards
        for _ in 0..4 {
            cards.add_child(Size::new(288, 70), SizeConstraint::Fixed(70)).ok();
        }

        // Temperature text component
        let temp_card_bounds = cards.child_bounds(0).unwrap();
        let temp_text = TextComponent::new(
            Rectangle::new(
                temp_card_bounds.top_left + Point::new(16, 30),
                Size::new(256, 20)
            ),
            "-- °C",
            TextSize::Large,
        );

        // Humidity text component
        let humidity_card_bounds = cards.child_bounds(1).unwrap();
        let humidity_text = TextComponent::new(
            Rectangle::new(
                humidity_card_bounds.top_left + Point::new(16, 30),
                Size::new(256, 20)
            ),
            "-- %",
            TextSize::Large,
        );

        Self {
            bounds,
            layout,
            sensor_cards: cards,
            temp_text,
            humidity_text,
        }
    }
}
```

---

## Tips and Best Practices

### Layout Planning

1. **Sketch first**: Draw your UI on paper before coding
2. **Think in containers**: Break complex layouts into nested containers
3. **Use constraints wisely**:
   - `Fixed` for headers, footers, buttons with specific sizes
   - `Expand` for content areas that should fill space
   - `Fit` for content-sized elements

### Performance

1. **Reuse containers**: Don't recreate layouts every frame
2. **Mark dirty appropriately**: Only redraw what changed
3. **Limit nesting depth**: Deeply nested containers can be slower

### Styling

1. **Create style constants**: Define common styles once
2. **Use the theme system**: Consistent colors and spacing
3. **Corner radius guidelines**:
   - 4-6px: Subtle rounding
   - 8-12px: Standard cards and buttons
   - 16+px: Prominent rounding

### Touch Handling

1. **Size targets appropriately**: Minimum 40x40px for touch targets
2. **Provide visual feedback**: Change appearance on press
3. **Test on device**: Touch behavior can differ from mouse clicks

---

## Common Patterns Quick Reference

| Pattern | Direction | Alignment | Key Constraints |
|---------|-----------|-----------|----------------|
| Vertical stack | Vertical | Start | Mix Fixed/Expand |
| Button row | Horizontal | Center | Equal Expand |
| Card grid | Vertical rows of Horizontal | Center/Start | Fixed heights, Expand widths |
| Form | Vertical | Start | Fixed heights |
| Centered content | Vertical/Both | Center | Fit for content |
| Split view | Horizontal | Stretch | Expand both |
| Header | Horizontal | Center | Fixed sides, Expand middle |

---

For more information, see the main [UI README](README.md).
