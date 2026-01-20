use embedded_sdmmc::{Mode, SdCard, SdCardError, TimeSource, VolumeIdx, VolumeManager};

use crate::storage::Rollup;

const ROLLUP_FILE_5M: &str = "rollup_5m.bin";
const ROLLUP_FILE_1H: &str = "rollup_1h.bin";
const ROLLUP_FILE_DAILY: &str = "rollup_daily.bin";
const ROLLUP_FILE_LIFETIME: &str = "lifetime.bin";

/// For NOW, these SD card operations are blocking (as are also the display operations on the same SPI bus),
/// BUT we're going to raw dog it and see if it works okay in practice.
///
/// In the future, we may want to implement async SD card operations, but to do this, the dual mode pin
/// would need to be async, and the embedded_sdmmc would need to be rewritten to be async-compatible.
pub struct SdCardStorage<S, D, T>
where
    S: embedded_hal::spi::SpiDevice<u8>,
    D: embedded_hal::delay::DelayNs,
    T: TimeSource,
{
    volume_mgr: VolumeManager<SdCard<S, D>, T, 4, 4, 1>,
}

impl<S, D, T> SdCardStorage<S, D, T>
where
    S: embedded_hal::spi::SpiDevice<u8>,
    D: embedded_hal::delay::DelayNs,
    T: TimeSource,
{
    /// Create a new SD card storage manager
    pub fn new(sd_card: SdCard<S, D>, ts: T) -> Self {
        let volume_mgr = VolumeManager::new(sd_card, ts);

        Self { volume_mgr }
    }

    /// Appends to a rollup file the data provided
    pub fn append_rollup_data(
        &self,
        file_name: &str,
        data: &Rollup,
    ) -> Result<(), embedded_sdmmc::Error<SdCardError>> {
        // Open volume
        let volume0 = self.volume_mgr.open_volume(VolumeIdx(0))?;

        // Open root directory
        let root_dir = volume0.open_root_dir()?;

        // Open file for appending
        let file = root_dir.open_file_in_dir(file_name, Mode::ReadWriteCreateOrAppend)?;

        // Write data to file
        file.write(data.as_ref())?;

        // Resources are automatically closed when dropped (RAII)
        // Explicitly close them to handle errors
        file.close()?;
        root_dir.close()?;
        volume0.close()?;

        Ok(())
    }

    pub fn read_rollup_data(
        &self,
        file_name: &str,
        buffer: &mut [Rollup],
        within_window: (u32, u32),
    ) -> Result<usize, embedded_sdmmc::Error<SdCardError>> {
        // Open volume
        let volume0 = self.volume_mgr.open_volume(VolumeIdx(0))?;

        // Open root directory
        let root_dir = volume0.open_root_dir()?;

        // Open file for reading
        let file = root_dir.open_file_in_dir(file_name, Mode::ReadOnly)?;

        let mut count = 0;
        let mut temp_rollup = Rollup::default();

        // Read rollups into buffer
        while count < buffer.len() {
            match file.read(temp_rollup.as_mut()) {
                Ok(bytes_read) => {
                    if bytes_read == 0 {
                        break; // EOF
                    }

                    // Check if within time window
                    let timestamp = temp_rollup.start_ts;
                    if timestamp >= within_window.0 && timestamp <= within_window.1 {
                        buffer[count] = temp_rollup;
                        count += 1;
                    }
                }
                Err(e) => {
                    // Handle read error
                    return Err(e);
                }
            }
        }

        // Explicitly close resources
        file.close()?;
        root_dir.close()?;
        volume0.close()?;

        Ok(count)
    }
}
