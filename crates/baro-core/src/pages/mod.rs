pub mod constants;
pub mod home;
pub mod monitor;
pub mod page;
pub mod page_manager;
pub mod settings;
pub mod trend;
pub mod wifi_status;

pub use home::grid::HomeGridPage;
pub use home::outdoor::HomePage;
pub use monitor::MonitorPage;
pub use page::{Page, PageWrapper};
pub use page_manager::PageManager;
pub use settings::{DisplaySettingsPage, SettingsPage};
pub use trend::TrendPage;
pub use wifi_status::{WifiState, WifiStatusPage};
