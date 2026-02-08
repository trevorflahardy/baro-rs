pub mod constants;
pub mod home;
pub mod page;
pub mod page_manager;
pub mod settings;
pub mod trend;
pub mod wifi_error;

pub use home::HomePage;
pub use page::{Page, PageWrapper};
pub use page_manager::PageManager;
pub use settings::SettingsPage;
pub use trend::TrendPage;
pub use wifi_error::WifiErrorPage;
