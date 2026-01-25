pub mod home;
pub mod page_manager;
pub mod settings;
pub mod wifi_error;

pub use home::HomePage;
pub use page_manager::{Page, PageManager, PageWrapper};
pub use settings::SettingsPage;
pub use wifi_error::WifiErrorPage;
