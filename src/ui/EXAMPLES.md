# Baro UI System Examples

Complete, practical examples demonstrating the UI system.

## Example 1: Simple Vertical Menu

A basic menu with a title and buttons:

```rust
use crate::ui::*;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

fn create_menu(bounds: Rectangle) -> Container<8> {
    let mut menu = Container::<8>::new(bounds, Direction::Vertical)
        .with_alignment(Alignment::Stretch)
        .with_gap(10);

    // Title
    menu.add_child(
        Element::text_auto("Main Menu", TextSize::Large),
        SizeConstraint::Fixed(30)
    ).ok();

    // Spacer
    menu.add_child(
        Element::spacer(Rectangle::zero()),
        SizeConstraint::Fixed(10)
    ).ok();

    // Buttons (equal height)
    menu.add_child(
        Element::button_auto("Start", Action::Custom(1)),
        SizeConstraint::Fixed(50)
    ).ok();

    menu.add_child(
        Element::button_auto("Settings", Action::NavigateToPage(PageId::Settings)),
        SizeConstraint::Fixed(50)
    ).ok();

    menu.add_child(
        Element::button_auto("Exit", Action::Custom(99)),
        SizeConstraint::Fixed(50)
    ).ok();

    menu
}
```

## Example 2: Nested Header and Body

Two-section layout with a fixed header and flexible body:

```rust
use crate::ui::*;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

pub struct AppPage {
    root: Container<2>,
}

impl AppPage {
    pub fn new(bounds: Rectangle) -> Self {
        let mut root = Container::<2>::new(bounds, Direction::Vertical)
            .with_alignment(Alignment::Stretch);

        // Header: Fixed 80px
        let mut header = Container::<MAX_CONTAINER_CHILDREN>::new(
            Rectangle::new(Point::zero(), Size::new(bounds.size.width, 80)),
            Direction::Vertical
        )
        .with_alignment(Alignment::Center)
        .with_main_axis_alignment(MainAxisAlignment::Center);

        header.add_child(
            Element::text_auto("Baro Metrics", TextSize::Large),
            SizeConstraint::Fit
        ).ok();

        header.add_child(
            Element::text_auto("Real-time Monitoring", TextSize::Small),
            SizeConstraint::Fit
        ).ok();

        // Body: Flexible
        let mut body = Container::<MAX_CONTAINER_CHILDREN>::new(
            Rectangle::zero(),
            Direction::Vertical
        )
        .with_alignment(Alignment::Stretch)
        .with_gap(15);

        body.add_child(
            Element::text_auto("Temperature: 22.5°C", TextSize::Medium),
            SizeConstraint::Fit
        ).ok();

        body.add_child(
            Element::text_auto("Humidity: 45%", TextSize::Medium),
            SizeConstraint::Fit
        ).ok();

        // Combine header and body
        root.add_child(Element::container(header), SizeConstraint::Fixed(80)).ok();
        root.add_child(Element::container(body), SizeConstraint::Grow(1)).ok();

        Self { root }
    }
}
```

## Example 3: Horizontal Button Row

Even spacing with flex-grow:

```rust
use crate::ui::*;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

fn create_button_row(bounds: Rectangle) -> Result<Container<3>, &'static str> {
    Ok(Container::<3>::new(bounds, Direction::Horizontal)
        .with_alignment(Alignment::Center)
        .with_main_axis_alignment(MainAxisAlignment::SpaceEvenly)
        .with_gap(10)
        .with_child(
            Element::button_auto("Back", Action::GoBack),
            SizeConstraint::Fit
        )?
        .with_child(
            Element::button_auto("Home", Action::NavigateToPage(PageId::Home)),
            SizeConstraint::Fit
        )?
        .with_child(
            Element::button_auto("Next", Action::Custom(1)),
            SizeConstraint::Fit
        )?)
}
```

## Example 4: Grid-like Layout with Nested Containers

Create a 2x2 grid using nested horizontal and vertical containers:

```rust
use crate::ui::*;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

fn create_dashboard(bounds: Rectangle) -> Container<2> {
    let mut root = Container::<2>::new(bounds, Direction::Vertical)
        .with_alignment(Alignment::Stretch)
        .with_gap(10);

    // Top row
    let mut top_row = Container::<MAX_CONTAINER_CHILDREN>::new(
        Rectangle::zero(),
        Direction::Horizontal
    )
    .with_alignment(Alignment::Stretch)
    .with_gap(10);

    top_row.add_child(
        Element::text_auto("Temp\n22.5°C", TextSize::Medium),
        SizeConstraint::Grow(1)
    ).ok();

    top_row.add_child(
        Element::text_auto("Humidity\n45%", TextSize::Medium),
        SizeConstraint::Grow(1)
    ).ok();

    // Bottom row
    let mut bottom_row = Container::<MAX_CONTAINER_CHILDREN>::new(
        Rectangle::zero(),
        Direction::Horizontal
    )
    .with_alignment(Alignment::Stretch)
    .with_gap(10);

    bottom_row.add_child(
        Element::text_auto("CO₂\n425ppm", TextSize::Medium),
        SizeConstraint::Grow(1)
    ).ok();

    bottom_row.add_child(
        Element::text_auto("Pressure\n1013hPa", TextSize::Medium),
        SizeConstraint::Grow(1)
    ).ok();

    // Combine rows
    root.add_child(Element::container(top_row), SizeConstraint::Grow(1)).ok();
    root.add_child(Element::container(bottom_row), SizeConstraint::Grow(1)).ok();

    root
}
```

## Example 5: Mixed Sizing Strategies

Combine fixed, fit, and grow constraints:

```rust
use crate::ui::*;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

fn create_mixed_layout(bounds: Rectangle) -> Container<5> {
    let mut layout = Container::<5>::new(bounds, Direction::Vertical)
        .with_alignment(Alignment::Stretch)
        .with_gap(5);

    // Fixed-height header
    layout.add_child(
        Element::text_auto("Fixed Header (40px)", TextSize::Large),
        SizeConstraint::Fixed(40)
    ).ok();

    // Auto-sized text
    layout.add_child(
        Element::text_auto("Auto-sized label", TextSize::Small),
        SizeConstraint::Fit
    ).ok();

    // Growing content area
    layout.add_child(
        Element::text_auto("This grows: weight=2", TextSize::Medium),
        SizeConstraint::Grow(2)
    ).ok();

    layout.add_child(
        Element::text_auto("This grows: weight=1", TextSize::Medium),
        SizeConstraint::Grow(1)
    ).ok();

    // Fixed-height footer
    layout.add_child(
        Element::text_auto("Fixed Footer (30px)", TextSize::Small),
        SizeConstraint::Fixed(30)
    ).ok();

    layout
}
```

## Example 6: Styled Containers

Add styling to containers for visual structure:

```rust
use crate::ui::*;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::pixelcolor::Rgb565;

fn create_styled_card(bounds: Rectangle) -> Container<3> {
    let mut card = Container::<3>::new(bounds, Direction::Vertical)
        .with_alignment(Alignment::Center)
        .with_gap(8)
        .with_padding(Padding::all(12))
        .with_corner_radius(8)
        .with_style(
            Style::default()
                .with_background_color(Some(Rgb565::new(20, 40, 60)))
                .with_border_color(Some(Rgb565::CYAN))
        );

    card.add_child(
        Element::text_auto("Sensor Card", TextSize::Large),
        SizeConstraint::Fit
    ).ok();

    card.add_child(
        Element::text_auto("Temperature: 22.5°C", TextSize::Medium),
        SizeConstraint::Fit
    ).ok();

    card.add_child(
        Element::button_auto("Details", Action::Custom(1)),
        SizeConstraint::Fixed(40)
    ).ok();

    card
}
```

## Example 7: Full Page with Multiple Sections

A complete page demonstrating composition:

```rust
use crate::ui::*;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

pub struct DashboardPage {
    root: Container<3>,
}

impl DashboardPage {
    pub fn new(bounds: Rectangle) -> Self {
        let mut root = Container::<3>::new(bounds, Direction::Vertical)
            .with_alignment(Alignment::Stretch)
            .with_gap(10);

        // Header
        let mut header = Container::<MAX_CONTAINER_CHILDREN>::new(
            Rectangle::zero(),
            Direction::Horizontal
        )
        .with_alignment(Alignment::Center)
        .with_main_axis_alignment(MainAxisAlignment::SpaceBetween);

        header.add_child(
            Element::text_auto("Dashboard", TextSize::Large),
            SizeConstraint::Fit
        ).ok();

        header.add_child(
            Element::button_auto("⚙", Action::NavigateToPage(PageId::Settings)),
            SizeConstraint::Fixed(40)
        ).ok();

        // Content area
        let mut content = Container::<MAX_CONTAINER_CHILDREN>::new(
            Rectangle::zero(),
            Direction::Vertical
        )
        .with_alignment(Alignment::Stretch)
        .with_gap(8);

        content.add_child(
            Element::text_auto("Current Readings:", TextSize::Medium),
            SizeConstraint::Fit
        ).ok();

        content.add_child(
            Element::text_auto("🌡️ Temperature: 22.5°C", TextSize::Small),
            SizeConstraint::Fit
        ).ok();

        content.add_child(
            Element::text_auto("💧 Humidity: 45%", TextSize::Small),
            SizeConstraint::Fit
        ).ok();

        // Footer actions
        let mut footer = Container::<MAX_CONTAINER_CHILDREN>::new(
            Rectangle::zero(),
            Direction::Horizontal
        )
        .with_alignment(Alignment::Center)
        .with_gap(10);

        footer.add_child(
            Element::button_auto("Refresh", Action::RefreshData),
            SizeConstraint::Grow(1)
        ).ok();

        footer.add_child(
            Element::button_auto("History", Action::NavigateToPage(PageId::TrendPage)),
            SizeConstraint::Grow(1)
        ).ok();

        // Combine sections
        root.add_child(Element::container(header), SizeConstraint::Fixed(50)).ok();
        root.add_child(Element::container(content), SizeConstraint::Grow(1)).ok();
        root.add_child(Element::container(footer), SizeConstraint::Fixed(60)).ok();

        Self { root }
    }
}
```

## Tips for Real-World Use

### 1. Use Meaningful Container Sizes

Choose `N` based on actual child count + some headroom:
- `Container<2>` for header/body splits
- `Container<8>` for typical menus
- `Container<MAX_CONTAINER_CHILDREN>` for flexible/nested containers

### 2. Automatic Sizing is Your Friend

Prefer `_auto()` constructors and `SizeConstraint::Fit` for labels and indicators:

```rust
Element::text_auto("Status: OK", TextSize::Small)
```

### 3. Fixed Sizes for Interactive Elements

Buttons and touch targets should have explicit minimum sizes:

```rust
Element::button_auto("Action", action), SizeConstraint::Fixed(50)
```

### 4. Grow for Flexible Content

Use `Grow()` for content that should fill available space:

```rust
// Scrollable content area
container.add_child(
    Element::container(scrollable_content),
    SizeConstraint::Grow(1)
).ok();
```

### 5. Test on Target Hardware

Layout calculations are deterministic, but visual appearance should be verified on the actual display.

## See Also

- `README.md` — UI system overview and concepts
- `src/ui/layouts/container.rs` — Complete Container API
- `src/pages/home.rs` — Real implementation example
